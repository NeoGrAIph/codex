use std::path::Path;

use codex_app_server_protocol::AgentRoleToolSelectionCatalogEntry;
use codex_app_server_protocol::AgentRoleToolSelectionCatalogExposure;
use codex_app_server_protocol::AgentRoleToolSelectionCatalogReadResponse;
use url::Url;

use super::*;
use crate::bottom_pane::MultiSelectItem;
use crate::bottom_pane::MultiSelectPicker;
use crate::legacy_core::agent_role_templates::AgentRoleTemplateEntry;
use crate::legacy_core::agent_role_templates::AgentRoleTemplateSource;
use crate::legacy_core::agent_role_templates::create_user_agent_role_template_from_draft;
use crate::legacy_core::agent_role_templates::list_agent_role_templates;
use crate::legacy_core::agent_role_templates::starter_agent_role_template_draft;
use crate::legacy_core::agent_role_templates::starter_agent_role_template_draft_with_allowed_tools;
use crate::legacy_core::agent_role_templates::update_user_agent_role_template_from_draft;
use crate::legacy_core::agent_role_templates::user_agent_role_template_draft_with_allowed_tools;
use crate::multi_agents::AgentRoleRuntimeUsage;
use std::collections::BTreeMap;

const AGENT_ROLE_TEMPLATES_TITLE: &str = "Agent Role Templates";
const NATIVE_PROFILE_FIELDS: &str = "Native profile fields: name, description, nickname_candidates, developer_instructions, model, model_provider, model_reasoning_effort, service_tier";
const RUNTIME_CATALOG_PREVIEW_LIMIT: usize = 8;

#[derive(Clone)]
pub(crate) enum AgentRoleRuntimeCatalogState {
    NotRequested,
    Loaded(AgentRoleToolSelectionCatalogReadResponse),
    Unavailable(String),
}

impl ChatWidget {
    pub(crate) fn open_agent_role_templates_popup(&mut self) {
        self.open_agent_role_templates_popup_with_runtime_catalog(None);
    }

    pub(crate) fn open_agent_role_templates_popup_with_runtime_catalog(
        &mut self,
        runtime_catalog: Option<Result<AgentRoleToolSelectionCatalogReadResponse, String>>,
    ) {
        self.open_agent_role_templates_popup_with_runtime_context(runtime_catalog, BTreeMap::new());
    }

    pub(crate) fn open_agent_role_templates_popup_with_runtime_context(
        &mut self,
        runtime_catalog: Option<Result<AgentRoleToolSelectionCatalogReadResponse, String>>,
        runtime_usage: BTreeMap<String, AgentRoleRuntimeUsage>,
    ) {
        let runtime_catalog = match runtime_catalog {
            Some(Ok(catalog)) => AgentRoleRuntimeCatalogState::Loaded(catalog),
            Some(Err(err)) => AgentRoleRuntimeCatalogState::Unavailable(err),
            None => AgentRoleRuntimeCatalogState::NotRequested,
        };
        let entries = list_agent_role_templates(&self.config);
        let mut items = Vec::with_capacity((entries.len() * 2) + 2);
        items.push(create_role_template_item());
        if let AgentRoleRuntimeCatalogState::Loaded(catalog) = &runtime_catalog {
            items.push(create_role_template_from_catalog_item(catalog));
        }
        for entry in entries {
            if let Some(item) = role_template_tool_selection_item(&entry, &runtime_catalog) {
                items.push(role_template_item(entry, &runtime_catalog, &runtime_usage));
                items.push(item);
            } else {
                items.push(role_template_item(entry, &runtime_catalog, &runtime_usage));
            }
        }

        let mut header = ColumnRenderable::new();
        header.push(Line::from(AGENT_ROLE_TEMPLATES_TITLE.bold()));
        header.push(Line::from(
            "Create and inspect role profiles used by spawn_agent.agent_type.".dim(),
        ));
        header.push(Line::from(
            "User templates live in $CODEX_HOME/agents/*.toml.".dim(),
        ));
        header.push(Line::from(runtime_catalog_header(&runtime_catalog).dim()));

        self.show_selection_view(SelectionViewParams {
            header: Box::new(header),
            footer_hint: Some(standard_popup_hint_line()),
            items,
            initial_selected_idx: Some(0),
            col_width_mode: ColumnWidthMode::AutoAllRows,
            ..Default::default()
        });
    }

    pub(crate) fn open_agent_role_template_create_prompt(&mut self) {
        self.open_agent_role_template_create_prompt_with_draft(starter_agent_role_template_draft());
    }

    pub(crate) fn open_agent_role_template_create_prompt_with_allowed_tools(
        &mut self,
        allowed_tools: Vec<String>,
    ) {
        self.open_agent_role_template_create_prompt_with_draft(
            starter_agent_role_template_draft_with_allowed_tools(&allowed_tools),
        );
    }

    fn open_agent_role_template_create_prompt_with_draft(&mut self, draft: String) {
        let tx = self.app_event_tx.clone();
        let view = CustomPromptView::new(
            "Create role template".to_string(),
            "Edit native role TOML and press Enter".to_string(),
            draft,
            Some("$CODEX_HOME/agents/<role>.toml".to_string()),
            Box::new(move |draft: String| {
                tx.send(AppEvent::CreateAgentRoleTemplateFromDraft { draft });
            }),
        );
        self.bottom_pane.show_view(Box::new(view));
    }

    pub(crate) fn open_agent_role_template_tool_selection_picker(
        &mut self,
        catalog_entries: Vec<AgentRoleToolSelectionCatalogEntry>,
    ) {
        self.open_agent_role_template_tool_selection_picker_for_new_role(catalog_entries);
    }

    fn open_agent_role_template_tool_selection_picker_for_new_role(
        &mut self,
        catalog_entries: Vec<AgentRoleToolSelectionCatalogEntry>,
    ) {
        let tx = self.app_event_tx.clone();
        let mut entries = catalog_entries;
        entries.sort_by(|left, right| {
            exposure_bucket_order(&left.exposure)
                .cmp(&exposure_bucket_order(&right.exposure))
                .then_with(|| left.name.cmp(&right.name))
        });
        let items = entries
            .iter()
            .enumerate()
            .map(|(idx, entry)| {
                let next_exposure = entries.get(idx + 1).map(|entry| &entry.exposure);
                MultiSelectItem {
                    id: entry.name.clone(),
                    name: entry.name.clone(),
                    description: Some(format!("{} tool id", exposure_label(&entry.exposure))),
                    enabled: false,
                    orderable: false,
                    section_break_after: next_exposure
                        .is_some_and(|next_exposure| next_exposure != &entry.exposure),
                }
            })
            .collect::<Vec<_>>();
        let view = MultiSelectPicker::builder(
            "Select role tools".to_string(),
            Some(
                "Choose current-thread tool ids for a native [tool_selection] allowed_tools draft."
                    .to_string(),
            ),
            tx,
        )
        .items(items)
        .on_preview(|items| {
            let selected_count = items.iter().filter(|item| item.enabled).count();
            Some(Line::from(format!(
                "{selected_count} selected; Enter opens editable TOML draft"
            )))
        })
        .on_confirm(|allowed_tools, tx| {
            tx.send(
                AppEvent::OpenAgentRoleTemplateCreatePromptWithAllowedTools {
                    allowed_tools: allowed_tools.to_vec(),
                },
            );
        })
        .build();
        self.bottom_pane.show_view(Box::new(view));
    }

    pub(crate) fn open_agent_role_template_tool_selection_picker_for_role(
        &mut self,
        role_name: String,
        role_path: std::path::PathBuf,
        selected_tools: Vec<String>,
        catalog_entries: Vec<AgentRoleToolSelectionCatalogEntry>,
    ) {
        let tx = self.app_event_tx.clone();
        let selected_tools = selected_tools
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        let mut entries = catalog_entries;
        entries.sort_by(|left, right| {
            exposure_bucket_order(&left.exposure)
                .cmp(&exposure_bucket_order(&right.exposure))
                .then_with(|| left.name.cmp(&right.name))
        });
        let items = entries
            .iter()
            .enumerate()
            .map(|(idx, entry)| {
                let next_exposure = entries.get(idx + 1).map(|entry| &entry.exposure);
                MultiSelectItem {
                    id: entry.name.clone(),
                    name: entry.name.clone(),
                    description: Some(format!("{} tool id", exposure_label(&entry.exposure))),
                    enabled: selected_tools.contains(&entry.name),
                    orderable: false,
                    section_break_after: next_exposure
                        .is_some_and(|next_exposure| next_exposure != &entry.exposure),
                }
            })
            .collect::<Vec<_>>();
        let role_name_for_preview = role_name.clone();
        let role_name_for_confirm = role_name.clone();
        let role_path_for_confirm = role_path;
        let view = MultiSelectPicker::builder(
            format!("Edit tools for {role_name}"),
            Some(
                "Choose current-thread tool ids for this existing native role TOML file."
                    .to_string(),
            ),
            tx,
        )
        .items(items)
        .on_preview(move |items| {
            let selected_count = items.iter().filter(|item| item.enabled).count();
            Some(Line::from(format!(
                "{selected_count} selected for {role_name_for_preview}; Enter opens editable TOML draft"
            )))
        })
        .on_confirm(move |allowed_tools, tx| {
            tx.send(
                AppEvent::OpenAgentRoleTemplateEditPromptWithAllowedTools {
                    role_name: role_name_for_confirm.clone(),
                    role_path: role_path_for_confirm.clone(),
                    allowed_tools: allowed_tools.to_vec(),
                },
            );
        })
        .build();
        self.bottom_pane.show_view(Box::new(view));
    }

    pub(crate) fn create_agent_role_template_from_draft(&mut self, draft: String) -> bool {
        match create_user_agent_role_template_from_draft(&self.config, &draft) {
            Ok(created) => {
                self.config
                    .agent_roles
                    .insert(created.name.clone(), created.config);
                self.add_info_message(
                    format!("Created agent role template `{}`", created.name),
                    Some(format!(
                        "{}\nReloading loaded threads through native config refresh.",
                        created.path.display()
                    )),
                );
                self.open_agent_role_templates_popup();
                true
            }
            Err(err) => {
                self.add_error_message(format!("Failed to create agent role template: {err}"));
                self.open_agent_role_templates_popup();
                false
            }
        }
    }

    pub(crate) fn open_agent_role_template_edit_prompt_with_allowed_tools(
        &mut self,
        role_name: String,
        role_path: std::path::PathBuf,
        allowed_tools: Vec<String>,
    ) {
        match user_agent_role_template_draft_with_allowed_tools(
            &self.config,
            &role_name,
            &role_path,
            &allowed_tools,
        ) {
            Ok(draft) => {
                let tx = self.app_event_tx.clone();
                let title = format!("Edit role template `{role_name}`");
                let view = CustomPromptView::new(
                    title,
                    "Review native role TOML and press Enter".to_string(),
                    draft,
                    Some(role_path.display().to_string()),
                    Box::new(move |draft: String| {
                        tx.send(AppEvent::UpdateAgentRoleTemplateFromDraft {
                            role_name: role_name.clone(),
                            role_path: role_path.clone(),
                            draft,
                        });
                    }),
                );
                self.bottom_pane.show_view(Box::new(view));
            }
            Err(err) => {
                self.add_error_message(format!("Failed to prepare role template update: {err}"));
                self.open_agent_role_templates_popup();
            }
        }
    }

    pub(crate) fn update_agent_role_template_from_draft(
        &mut self,
        role_name: String,
        role_path: std::path::PathBuf,
        draft: String,
    ) -> bool {
        match update_user_agent_role_template_from_draft(
            &self.config,
            &role_name,
            &role_path,
            &draft,
        ) {
            Ok(updated) => {
                self.config
                    .agent_roles
                    .insert(updated.name.clone(), updated.config);
                self.add_info_message(
                    format!("Updated agent role template `{}`", updated.name),
                    Some(format!(
                        "{}\nReloading loaded threads through native config refresh.",
                        updated.path.display()
                    )),
                );
                self.open_agent_role_templates_popup();
                true
            }
            Err(err) => {
                self.add_error_message(format!("Failed to update agent role template: {err}"));
                self.open_agent_role_templates_popup();
                false
            }
        }
    }
}

fn create_role_template_item() -> SelectionItem {
    SelectionItem {
        name: "Create new template".to_string(),
        description: Some("Edit a native TOML role draft for future sub-agent spawns".to_string()),
        selected_description: Some(
            "Opens a parser-validated TOML draft for $CODEX_HOME/agents/. No markdown/frontmatter profile is written. New sessions load user roles from that native config directory.".to_string(),
        ),
        actions: vec![Box::new(|tx| {
            tx.send(AppEvent::OpenAgentRoleTemplateCreatePrompt);
        })],
        dismiss_on_select: true,
        ..Default::default()
    }
}

fn create_role_template_from_catalog_item(
    catalog: &AgentRoleToolSelectionCatalogReadResponse,
) -> SelectionItem {
    let catalog_entries = catalog.data.clone();
    SelectionItem {
        name: "Create template from current tools".to_string(),
        description: Some(format!(
            "Pick from {} current-thread runtime tool ids",
            catalog_entries.len()
        )),
        selected_description: Some(
            "Opens a searchable tool picker and then an editable native TOML draft. The current-thread catalog is used only as tool-id evidence; the saved source remains $CODEX_HOME/agents/*.toml.".to_string(),
        ),
        actions: vec![Box::new(move |tx| {
            tx.send(AppEvent::OpenAgentRoleTemplateToolSelectionPicker {
                catalog_entries: catalog_entries.clone(),
            });
        })],
        dismiss_on_select: true,
        ..Default::default()
    }
}

fn role_template_tool_selection_item(
    entry: &AgentRoleTemplateEntry,
    runtime_catalog: &AgentRoleRuntimeCatalogState,
) -> Option<SelectionItem> {
    if entry.source != AgentRoleTemplateSource::User
        || entry.validation_error.is_some()
        || entry.config_file_origin
            != Some(
                crate::legacy_core::agent_role_templates::AgentRoleTemplateConfigFileOrigin::UserAgentsDir,
            )
    {
        return None;
    }
    let AgentRoleRuntimeCatalogState::Loaded(catalog) = runtime_catalog else {
        return None;
    };
    let role_path = entry.config_file.clone()?;
    let role_name = entry.name.clone();
    let selected_tools = entry.locks.allowed_tool_names.clone();
    let catalog_entries = catalog.data.clone();
    let role_name_for_action = role_name.clone();
    Some(SelectionItem {
        name: format!("Edit tools for {role_name}"),
        name_prefix_spans: vec!["[user tools] ".dim()],
        description: Some("Update this native role TOML allowlist from current tools".to_string()),
        selected_description: Some(
            "Opens a searchable current-thread tool picker, then an editable native TOML draft for the existing $CODEX_HOME/agents role file. This does not add an app-server write API or a parallel role registry.".to_string(),
        ),
        actions: vec![Box::new(move |tx| {
            tx.send(
                AppEvent::OpenAgentRoleTemplateToolSelectionPickerForRole {
                    role_name: role_name_for_action.clone(),
                    role_path: role_path.clone(),
                    selected_tools: selected_tools.clone(),
                    catalog_entries: catalog_entries.clone(),
                },
            );
        })],
        dismiss_on_select: true,
        search_value: Some(format!("edit tools {role_name}")),
        ..Default::default()
    })
}

fn role_template_item(
    entry: AgentRoleTemplateEntry,
    runtime_catalog: &AgentRoleRuntimeCatalogState,
    runtime_usage: &BTreeMap<String, AgentRoleRuntimeUsage>,
) -> SelectionItem {
    let source_label = role_template_source_state_label(&entry);
    let name = entry.name.clone();
    let description = entry
        .description
        .as_deref()
        .map(short_role_template_description);
    let selected_description = Some(role_template_detail(&entry, runtime_catalog, runtime_usage));
    let mut actions: Vec<SelectionAction> = Vec::new();
    let mut dismiss_on_select = false;

    if entry.source == AgentRoleTemplateSource::User
        && let Some(config_file) = entry.config_file.clone()
    {
        actions.push(open_role_template_file_action(config_file));
        dismiss_on_select = true;
    }

    SelectionItem {
        name,
        name_prefix_spans: vec![format!("[{source_label}] ").dim()],
        description,
        selected_description,
        actions,
        dismiss_on_select,
        search_value: Some(format!("{source_label} {}", entry.name)),
        ..Default::default()
    }
}

fn role_template_source_state_label(entry: &AgentRoleTemplateEntry) -> &'static str {
    if entry.is_shadowed {
        return "built-in shadowed";
    }
    if entry.shadows_built_in {
        return "user active";
    }
    match entry.source {
        AgentRoleTemplateSource::User => "user active",
        AgentRoleTemplateSource::BuiltIn => "built-in active",
    }
}

fn open_role_template_file_action(path: impl Into<std::path::PathBuf>) -> SelectionAction {
    let path = path.into();
    Box::new(move |tx| match file_url(&path) {
        Some(url) => tx.send(AppEvent::OpenUrlInBrowser { url }),
        None => tx.send(AppEvent::InsertHistoryCell(Box::new(
            history_cell::new_error_event(format!(
                "Cannot open agent role template file: {}",
                path.display()
            )),
        ))),
    })
}

fn role_template_detail(
    entry: &AgentRoleTemplateEntry,
    runtime_catalog: &AgentRoleRuntimeCatalogState,
    runtime_usage: &BTreeMap<String, AgentRoleRuntimeUsage>,
) -> String {
    let mut lines = vec![
        format!("Role: {}", entry.name),
        format!("Source: {}", entry.source.label()),
        format!(
            "Status: {}",
            if entry.is_shadowed {
                "shadowed"
            } else {
                "active"
            }
        ),
    ];
    if entry.shadows_built_in {
        lines.push("Overrides built-in role with the same name.".to_string());
    }
    if entry.is_shadowed {
        lines.push("A user role with the same name is active for new spawns.".to_string());
    }

    if let Some(description) = &entry.description {
        lines.push(format!("Description: {description}"));
    }
    if let Some(path) = &entry.config_file {
        lines.push(format!("Config file: {}", path.display()));
        if let Some(origin) = entry.config_file_origin {
            lines.push(format!("Config origin: {}", origin.label()));
        }
    } else {
        lines.push("Config file: none".to_string());
    }
    lines.push(NATIVE_PROFILE_FIELDS.to_string());
    if !entry.field_sources.declaration_fields.is_empty() {
        lines.push(format!(
            "Declaration fields: {}",
            entry.field_sources.declaration_fields.join(", ")
        ));
    }
    if !entry.field_sources.declaration_origins.is_empty() {
        lines.push(format!(
            "Declaration origins: {}",
            entry.field_sources.declaration_origins.join(", ")
        ));
    }
    if !entry.field_sources.config_file_fields.is_empty() {
        lines.push(format!(
            "TOML fields: {}",
            entry.field_sources.config_file_fields.join(", ")
        ));
    }
    if !entry.field_sources.bounded_provenance.is_empty() {
        let sources = entry
            .field_sources
            .bounded_provenance
            .iter()
            .map(|source| {
                if let Some(detail) = &source.detail {
                    format!("{}={} ({detail})", source.field_name, source.source.label())
                } else {
                    format!("{}={}", source.field_name, source.source.label())
                }
            })
            .collect::<Vec<_>>();
        lines.push(format!("Field sources: {}", sources.join(", ")));
    }
    if entry.source == AgentRoleTemplateSource::User {
        if entry.config_file.is_some() {
            if let Some(error) = &entry.validation_error {
                lines.push(format!(
                    "Validation: invalid native TOML role file: {error}"
                ));
            } else {
                lines.push("Validation: valid native TOML role file".to_string());
            }
        } else {
            lines.push(
                "Validation: declared in config.toml; no role file to validate here".to_string(),
            );
        }
    } else {
        lines.push("Validation: bundled built-in role definition".to_string());
    }
    if !entry.nickname_candidates.is_empty() {
        lines.push(format!(
            "Nicknames: {}",
            entry.nickname_candidates.join(", ")
        ));
    }
    lines.push(role_runtime_usage_line(
        entry,
        runtime_usage.get(&entry.name),
    ));
    if !entry.locks.allowed_tool_names.is_empty() {
        lines.push("Tool selection state: runtime allowlist; unmatched entries warn during tool planning and grant no access".to_string());
    }
    lines.extend(runtime_catalog_detail_lines(entry, runtime_catalog));

    let mut locked = Vec::new();
    if let Some(model) = &entry.locks.model {
        locked.push(format!("model={model}"));
    }
    if let Some(model_provider) = &entry.locks.model_provider {
        locked.push(format!("model_provider={model_provider}"));
    }
    if let Some(reasoning) = &entry.locks.reasoning_effort {
        locked.push(format!("reasoning={reasoning}"));
    }
    if let Some(service_tier) = &entry.locks.service_tier {
        locked.push(format!("service_tier={service_tier}"));
    }
    if entry.locks.has_developer_instructions {
        locked.push("developer_instructions".to_string());
    }
    if !entry.locks.allowed_tool_names.is_empty() {
        locked.push(format!(
            "tool_selection={}",
            entry.locks.allowed_tool_names.join(", ")
        ));
    }
    if !locked.is_empty() {
        lines.push(format!("Template sets: {}", locked.join(", ")));
    }
    let mut security = Vec::new();
    if let Some(approval_policy) = &entry.locks.approval_policy {
        security.push(format!("approval_policy={approval_policy}"));
    }
    if let Some(sandbox_mode) = &entry.locks.sandbox_mode {
        security.push(format!("sandbox_mode={sandbox_mode}"));
    }
    if let Some(default_permissions) = &entry.locks.default_permissions {
        security.push(format!("default_permissions={default_permissions}"));
    }
    if entry.locks.has_sandbox_workspace_write {
        security.push("sandbox_workspace_write".to_string());
    }
    if entry.locks.permission_profile_count > 0 {
        security.push(format!(
            "permissions={} profiles",
            entry.locks.permission_profile_count
        ));
    }
    if !security.is_empty() {
        lines.push(format!("Security sets: {}", security.join(", ")));
    }
    let mut subsystems = Vec::new();
    if entry.locks.mcp_server_count > 0 {
        subsystems.push(format!("mcp_servers={}", entry.locks.mcp_server_count));
    }
    if entry.locks.hook_handler_count > 0 {
        subsystems.push(format!("hooks={} handlers", entry.locks.hook_handler_count));
    }
    if entry.locks.skill_config_count > 0 {
        subsystems.push(format!("skills={} config", entry.locks.skill_config_count));
    }
    if entry.locks.app_config_count > 0 {
        subsystems.push(format!("apps={} apps", entry.locks.app_config_count));
    }
    if entry.locks.has_apps_default_config {
        subsystems.push("apps_default".to_string());
    }
    if !subsystems.is_empty() {
        lines.push(format!("Subsystem sets: {}", subsystems.join(", ")));
    }
    lines.push(format!(
        "Runtime binding: spawn_agent.agent_type `{}`",
        entry.name
    ));

    match entry.source {
        AgentRoleTemplateSource::User => {
            lines.push("Enter: open template file for editing".to_string());
            lines.push(format!("Availability: {}.", entry.availability.label()));
        }
        AgentRoleTemplateSource::BuiltIn => {
            if entry.is_shadowed {
                lines.push(entry.availability.label().to_string());
            } else {
                lines.push(
                    "Built-in role; create a user template to customize behavior.".to_string(),
                );
                lines.push(format!("Availability: {}.", entry.availability.label()));
            }
        }
    }
    if !entry.is_shadowed {
        lines.push(format!(
            "Use: ask Codex to spawn a sub-agent with agent_type `{}`.",
            entry.name
        ));
    }

    lines.join("\n")
}

fn role_runtime_usage_line(
    entry: &AgentRoleTemplateEntry,
    usage: Option<&AgentRoleRuntimeUsage>,
) -> String {
    let Some(usage) = usage else {
        return "Runtime use: no known threads with this agent_type in current TUI session"
            .to_string();
    };

    let mut parts = vec![format!(
        "{} known thread{}",
        usage.total,
        if usage.total == 1 { "" } else { "s" }
    )];
    if usage.running > 0 {
        parts.push(format!("{} running", usage.running));
    }
    if usage.waiting > 0 {
        parts.push(format!("{} waiting", usage.waiting));
    }
    if usage.errors > 0 {
        parts.push(format!("{} error", usage.errors));
    }
    if usage.closed > 0 {
        parts.push(format!("{} closed", usage.closed));
    }
    parts.push(format!(
        "current view: {}",
        if usage.current_view { "yes" } else { "no" }
    ));
    let caveat = if entry.is_shadowed || entry.shadows_built_in {
        "; thread metadata stores agent_type, not definition source"
    } else {
        ""
    };
    format!("Runtime use: {}{}", parts.join("; "), caveat)
}

fn runtime_catalog_header(runtime_catalog: &AgentRoleRuntimeCatalogState) -> String {
    match runtime_catalog {
        AgentRoleRuntimeCatalogState::Loaded(catalog) => {
            let selected_count = catalog.data.iter().filter(|entry| entry.selected).count();
            format!(
                "Runtime tool catalog: {} current-thread tools, {} selected by current policy.",
                catalog.data.len(),
                selected_count
            )
        }
        AgentRoleRuntimeCatalogState::Unavailable(err) => {
            format!("Runtime tool catalog unavailable: {err}")
        }
        AgentRoleRuntimeCatalogState::NotRequested => {
            "Runtime tool catalog: open from a loaded thread to inspect current native tool ids."
                .to_string()
        }
    }
}

fn runtime_catalog_detail_lines(
    entry: &AgentRoleTemplateEntry,
    runtime_catalog: &AgentRoleRuntimeCatalogState,
) -> Vec<String> {
    match runtime_catalog {
        AgentRoleRuntimeCatalogState::Loaded(catalog) => {
            let mut lines = vec![
                "Runtime catalog reference: current thread planner output; use as tool id evidence, not as a role-specific preview.".to_string(),
                format!(
                    "Runtime catalog totals: {} tools, {} unmatched current-policy entries.",
                    catalog.data.len(),
                    catalog.unmatched_allowed_tools.len()
                ),
            ];
            if !entry.locks.allowed_tool_names.is_empty() {
                let catalog_names = catalog
                    .data
                    .iter()
                    .map(|catalog_entry| catalog_entry.name.as_str())
                    .collect::<std::collections::BTreeSet<_>>();
                let present = entry
                    .locks
                    .allowed_tool_names
                    .iter()
                    .filter(|name| catalog_names.contains(name.as_str()))
                    .count();
                let missing = entry.locks.allowed_tool_names.len().saturating_sub(present);
                lines.push(format!(
                    "Template allowlist vs current catalog: {present} present, {missing} absent"
                ));
            }
            let preview = catalog
                .data
                .iter()
                .take(RUNTIME_CATALOG_PREVIEW_LIMIT)
                .map(|catalog_entry| {
                    format!(
                        "{}{} ({})",
                        if catalog_entry.selected { "*" } else { "" },
                        catalog_entry.name,
                        exposure_label(&catalog_entry.exposure)
                    )
                })
                .collect::<Vec<_>>();
            if !preview.is_empty() {
                lines.push(format!(
                    "Runtime catalog sample: {}{}",
                    preview.join(", "),
                    if catalog.data.len() > preview.len() {
                        format!(", +{} more", catalog.data.len() - preview.len())
                    } else {
                        String::new()
                    }
                ));
            }
            if !catalog.unmatched_allowed_tools.is_empty() {
                lines.push(format!(
                    "Current unmatched allowlist entries: {}",
                    catalog.unmatched_allowed_tools.join(", ")
                ));
            }
            lines
        }
        AgentRoleRuntimeCatalogState::Unavailable(err) => {
            vec![format!("Runtime catalog unavailable: {err}")]
        }
        AgentRoleRuntimeCatalogState::NotRequested => {
            vec![
                "Runtime catalog reference: unavailable because no loaded thread was available for app-server catalog read.".to_string(),
            ]
        }
    }
}

fn exposure_label(exposure: &AgentRoleToolSelectionCatalogExposure) -> &'static str {
    match exposure {
        AgentRoleToolSelectionCatalogExposure::Direct => "direct",
        AgentRoleToolSelectionCatalogExposure::Deferred => "deferred",
        AgentRoleToolSelectionCatalogExposure::DirectModelOnly => "direct model-only",
        AgentRoleToolSelectionCatalogExposure::Hidden => "hidden",
        AgentRoleToolSelectionCatalogExposure::Hosted => "hosted",
    }
}

fn exposure_bucket_order(exposure: &AgentRoleToolSelectionCatalogExposure) -> u8 {
    match exposure {
        AgentRoleToolSelectionCatalogExposure::Direct => 0,
        AgentRoleToolSelectionCatalogExposure::Deferred => 1,
        AgentRoleToolSelectionCatalogExposure::DirectModelOnly => 2,
        AgentRoleToolSelectionCatalogExposure::Hosted => 3,
        AgentRoleToolSelectionCatalogExposure::Hidden => 4,
    }
}

fn short_role_template_description(description: &str) -> String {
    truncate_text(
        description.lines().next().unwrap_or_default().trim(),
        /*max_graphemes*/ 96,
    )
}

fn file_url(path: &Path) -> Option<String> {
    Url::from_file_path(path).ok().map(Into::into)
}
