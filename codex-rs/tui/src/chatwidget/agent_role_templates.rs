use std::path::Path;

use codex_app_server_protocol::AgentRoleToolSelectionCatalogEntry;
use codex_app_server_protocol::AgentRoleToolSelectionCatalogExposure;
use codex_app_server_protocol::AgentRoleToolSelectionCatalogReadResponse;
use url::Url;

use super::*;
use crate::bottom_pane::MultiSelectItem;
use crate::bottom_pane::MultiSelectPicker;
use crate::legacy_core::agent_role_templates::AgentRoleTemplateEntry;
use crate::legacy_core::agent_role_templates::AgentRoleTemplateModelDefaults;
use crate::legacy_core::agent_role_templates::AgentRoleTemplateSource;
use crate::legacy_core::agent_role_templates::create_user_agent_role_template_from_draft;
use crate::legacy_core::agent_role_templates::list_agent_role_templates;
use crate::legacy_core::agent_role_templates::starter_agent_role_template_draft;
use crate::legacy_core::agent_role_templates::starter_agent_role_template_draft_from_current_model;
use crate::legacy_core::agent_role_templates::starter_agent_role_template_draft_with_allowed_tools;
use crate::legacy_core::agent_role_templates::starter_agent_role_template_draft_with_model_defaults;
use crate::legacy_core::agent_role_templates::update_user_agent_role_template_from_draft;
use crate::legacy_core::agent_role_templates::user_agent_role_template_draft_from_current_model;
use crate::legacy_core::agent_role_templates::user_agent_role_template_draft_from_current_model_provider;
use crate::legacy_core::agent_role_templates::user_agent_role_template_draft_from_current_reasoning;
use crate::legacy_core::agent_role_templates::user_agent_role_template_draft_from_file;
use crate::legacy_core::agent_role_templates::user_agent_role_template_draft_with_allowed_tools;
use crate::legacy_core::agent_role_templates::user_agent_role_template_draft_with_denied_tools;
use crate::legacy_core::agent_role_templates::user_agent_role_template_draft_with_model_defaults;
use crate::legacy_core::agent_role_templates::user_agent_role_template_draft_without_model_defaults;
use crate::multi_agents::AgentRoleRuntimeUsage;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

const AGENT_ROLE_TEMPLATES_TITLE: &str = "Agent Role Templates";
const NATIVE_PROFILE_FIELDS: &str = "Native profile fields: name, description, nickname_candidates, developer_instructions, model, model_provider, model_reasoning_effort, service_tier";
const RUNTIME_CATALOG_PREVIEW_LIMIT: usize = 8;

#[derive(Clone, Copy)]
enum RoleToolSelectionEditKind {
    Allowed,
    Denied,
}

#[derive(Clone)]
enum RoleModelDefaultsTarget {
    NewRole,
    ExistingRole {
        role_name: String,
        role_path: std::path::PathBuf,
    },
}

impl RoleToolSelectionEditKind {
    fn label(self) -> &'static str {
        match self {
            RoleToolSelectionEditKind::Allowed => "allowed",
            RoleToolSelectionEditKind::Denied => "denied",
        }
    }

    fn picker_description(self) -> &'static str {
        match self {
            RoleToolSelectionEditKind::Allowed => {
                "Choose current-thread tool ids for this role's native [tool_selection] allowed_tools."
            }
            RoleToolSelectionEditKind::Denied => {
                "Choose current-thread tool ids for this role's native [tool_selection] denied_tools."
            }
        }
    }
}

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
        let mut items = Vec::with_capacity((entries.len() * 2) + 4);
        items.push(create_role_template_item());
        items.push(create_role_template_from_current_model_item());
        items.push(create_role_template_from_model_catalog_item());
        if let AgentRoleRuntimeCatalogState::Loaded(catalog) = &runtime_catalog {
            items.push(create_role_template_from_catalog_item(catalog));
        }
        for entry in entries {
            let edit_item = role_template_edit_item(&entry);
            let current_model_item = role_template_current_model_item(&entry);
            let model_catalog_item = role_template_model_catalog_item(&entry);
            let current_model_provider_item = role_template_current_model_provider_item(&entry);
            let current_reasoning_item = role_template_current_reasoning_item(&entry);
            let clear_model_item = role_template_clear_model_item(&entry);
            let allowed_tool_item = role_template_tool_selection_item(&entry, &runtime_catalog);
            let denied_tool_item =
                role_template_denied_tool_selection_item(&entry, &runtime_catalog);
            items.push(role_template_item(entry, &runtime_catalog, &runtime_usage));
            if let Some(item) = edit_item {
                items.push(item);
            }
            if let Some(item) = current_model_item {
                items.push(item);
            }
            if let Some(item) = model_catalog_item {
                items.push(item);
            }
            if let Some(item) = current_model_provider_item {
                items.push(item);
            }
            if let Some(item) = current_reasoning_item {
                items.push(item);
            }
            if let Some(item) = clear_model_item {
                items.push(item);
            }
            if let Some(item) = allowed_tool_item {
                items.push(item);
            }
            if let Some(item) = denied_tool_item {
                items.push(item);
            }
        }
        items.push(import_markdown_agent_roles_item());

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

    pub(crate) fn open_agent_role_template_create_prompt_from_current_model(&mut self) {
        self.open_agent_role_template_create_prompt_with_draft(
            starter_agent_role_template_draft_from_current_model(&self.config),
        );
    }

    pub(crate) fn open_agent_role_template_create_model_picker(&mut self) {
        self.open_agent_role_template_model_picker(RoleModelDefaultsTarget::NewRole);
    }

    pub(crate) fn open_agent_role_template_edit_model_picker(
        &mut self,
        role_name: String,
        role_path: std::path::PathBuf,
    ) {
        self.open_agent_role_template_model_picker(RoleModelDefaultsTarget::ExistingRole {
            role_name,
            role_path,
        });
    }

    fn open_agent_role_template_model_picker(&mut self, target: RoleModelDefaultsTarget) {
        let presets: Vec<ModelPreset> = match self.model_catalog.try_list_models() {
            Ok(models) => models
                .into_iter()
                .filter(|preset| preset.show_in_picker)
                .collect(),
            Err(_) => {
                self.add_info_message(
                    "Models are being updated; please try /agent-roles again in a moment."
                        .to_string(),
                    /*hint*/ None,
                );
                return;
            }
        };
        if presets.is_empty() {
            self.add_info_message(
                "No role model defaults are available right now.".to_string(),
                /*hint*/ None,
            );
            return;
        }

        let mut model_slug_counts = BTreeMap::new();
        for preset in &presets {
            *model_slug_counts
                .entry(preset.model.clone())
                .or_insert(0usize) += 1;
        }

        let mut items = Vec::new();
        for preset in presets {
            let duplicate_slug = model_slug_counts
                .get(preset.model.as_str())
                .is_some_and(|count| *count > 1);
            let name = if duplicate_slug {
                format!("{} ({})", preset.model, preset.model_provider)
            } else {
                preset.model.clone()
            };
            let single_supported_effort = preset.supported_reasoning_efforts.len() == 1;
            let description = (!preset.description.is_empty()).then(|| {
                format!(
                    "{} · provider {}",
                    preset.description, preset.model_provider
                )
            });
            let target_for_action = target.clone();
            let preset_for_action = preset.clone();
            let actions: Vec<SelectionAction> = vec![Box::new(move |tx| {
                if single_supported_effort {
                    send_role_model_defaults_event(
                        tx,
                        target_for_action.clone(),
                        &preset_for_action,
                        Some(preset_for_action.default_reasoning_effort.clone()),
                    );
                    return;
                }

                match target_for_action.clone() {
                    RoleModelDefaultsTarget::NewRole => {
                        tx.send(AppEvent::OpenAgentRoleTemplateCreateReasoningPicker {
                            model: preset_for_action.clone(),
                        });
                    }
                    RoleModelDefaultsTarget::ExistingRole {
                        role_name,
                        role_path,
                    } => {
                        tx.send(AppEvent::OpenAgentRoleTemplateEditReasoningPicker {
                            role_name,
                            role_path,
                            model: preset_for_action.clone(),
                        });
                    }
                }
            })];
            items.push(SelectionItem {
                name,
                description,
                is_default: preset.is_default,
                actions,
                dismiss_on_select: single_supported_effort,
                dismiss_parent_on_child_accept: !single_supported_effort,
                ..Default::default()
            });
        }

        let title = match &target {
            RoleModelDefaultsTarget::NewRole => "Select role model defaults".to_string(),
            RoleModelDefaultsTarget::ExistingRole { role_name, .. } => {
                format!("Select model defaults for {role_name}")
            }
        };
        let mut header = ColumnRenderable::new();
        header.push(Line::from(title.bold()));
        header.push(Line::from(
            "Writes model, model_provider, model_reasoning_effort and service_tier into native role TOML only.".dim(),
        ));

        self.bottom_pane.show_selection_view(SelectionViewParams {
            header: Box::new(header),
            footer_hint: Some(standard_popup_hint_line()),
            items,
            col_width_mode: ColumnWidthMode::AutoAllRows,
            ..Default::default()
        });
    }

    pub(crate) fn open_agent_role_template_create_reasoning_picker(&mut self, model: ModelPreset) {
        self.open_agent_role_template_reasoning_picker(RoleModelDefaultsTarget::NewRole, model);
    }

    pub(crate) fn open_agent_role_template_edit_reasoning_picker(
        &mut self,
        role_name: String,
        role_path: std::path::PathBuf,
        model: ModelPreset,
    ) {
        self.open_agent_role_template_reasoning_picker(
            RoleModelDefaultsTarget::ExistingRole {
                role_name,
                role_path,
            },
            model,
        );
    }

    fn open_agent_role_template_reasoning_picker(
        &mut self,
        target: RoleModelDefaultsTarget,
        preset: ModelPreset,
    ) {
        let mut choices: Vec<ReasoningEffortConfig> = preset
            .supported_reasoning_efforts
            .iter()
            .map(|option| option.effort.clone())
            .collect();
        if choices.is_empty() {
            choices.push(preset.default_reasoning_effort.clone());
        }
        let default_choice = choices
            .contains(&preset.default_reasoning_effort)
            .then(|| preset.default_reasoning_effort.clone())
            .or_else(|| choices.first().cloned());

        let mut items = Vec::new();
        for choice in choices {
            let is_default = default_choice.as_ref() == Some(&choice);
            let name = Self::reasoning_effort_label(&choice);
            let description = preset
                .supported_reasoning_efforts
                .iter()
                .find(|option| option.effort == choice)
                .map(|option| option.description.to_string())
                .filter(|text| !text.is_empty());
            let preset_for_action = preset.clone();
            let target_for_action = target.clone();
            let effort_for_action = Some(choice);
            let actions: Vec<SelectionAction> = vec![Box::new(move |tx| {
                send_role_model_defaults_event(
                    tx,
                    target_for_action.clone(),
                    &preset_for_action,
                    effort_for_action.clone(),
                );
            })];
            items.push(SelectionItem {
                name,
                description,
                is_default,
                actions,
                dismiss_on_select: true,
                ..Default::default()
            });
        }

        let mut header = ColumnRenderable::new();
        header.push(Line::from(
            format!("Select role reasoning for {}", preset.model).bold(),
        ));
        header.push(Line::from(
            format!("Provider: {}", preset.model_provider).dim(),
        ));

        self.bottom_pane.show_selection_view(SelectionViewParams {
            header: Box::new(header),
            footer_hint: Some(standard_popup_hint_line()),
            items,
            ..Default::default()
        });
    }

    pub(crate) fn open_agent_role_template_create_prompt_with_model_defaults(
        &mut self,
        model_provider: String,
        model: String,
        reasoning_effort: Option<ReasoningEffortConfig>,
        service_tier: Option<String>,
    ) {
        let defaults = role_model_defaults(model_provider, model, reasoning_effort, service_tier);
        self.open_agent_role_template_create_prompt_with_draft(
            starter_agent_role_template_draft_with_model_defaults(&defaults),
        );
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
        self.open_agent_role_template_tool_selection_picker_for_role_kind(
            role_name,
            role_path,
            selected_tools,
            catalog_entries,
            RoleToolSelectionEditKind::Allowed,
        );
    }

    pub(crate) fn open_agent_role_template_denied_tool_selection_picker_for_role(
        &mut self,
        role_name: String,
        role_path: std::path::PathBuf,
        selected_tools: Vec<String>,
        catalog_entries: Vec<AgentRoleToolSelectionCatalogEntry>,
    ) {
        self.open_agent_role_template_tool_selection_picker_for_role_kind(
            role_name,
            role_path,
            selected_tools,
            catalog_entries,
            RoleToolSelectionEditKind::Denied,
        );
    }

    fn open_agent_role_template_tool_selection_picker_for_role_kind(
        &mut self,
        role_name: String,
        role_path: std::path::PathBuf,
        selected_tools: Vec<String>,
        catalog_entries: Vec<AgentRoleToolSelectionCatalogEntry>,
        edit_kind: RoleToolSelectionEditKind,
    ) {
        let tx = self.app_event_tx.clone();
        let selected_tools = selected_tools.into_iter().collect::<BTreeSet<_>>();
        let mut entries = catalog_entries;
        entries.sort_by(|left, right| {
            exposure_bucket_order(&left.exposure)
                .cmp(&exposure_bucket_order(&right.exposure))
                .then_with(|| left.name.cmp(&right.name))
        });
        let items = agent_role_tool_selection_items(&entries, &selected_tools, edit_kind);
        let role_name_for_preview = role_name.clone();
        let role_name_for_confirm = role_name.clone();
        let role_path_for_confirm = role_path;
        let view = MultiSelectPicker::builder(
            format!("Edit {} tools for {role_name}", edit_kind.label()),
            Some(edit_kind.picker_description().to_string()),
            tx,
        )
        .items(items)
        .on_preview(move |items| {
            let selected_count = items.iter().filter(|item| item.enabled).count();
            Some(Line::from(format!(
                "{selected_count} {} tools selected for {role_name_for_preview}; Enter opens editable TOML draft",
                edit_kind.label()
            )))
        })
        .on_confirm(move |selected_tools, tx| match edit_kind {
            RoleToolSelectionEditKind::Allowed => {
                tx.send(AppEvent::OpenAgentRoleTemplateEditPromptWithAllowedTools {
                    role_name: role_name_for_confirm.clone(),
                    role_path: role_path_for_confirm.clone(),
                    allowed_tools: selected_tools.to_vec(),
                });
            }
            RoleToolSelectionEditKind::Denied => {
                tx.send(AppEvent::OpenAgentRoleTemplateEditPromptWithDeniedTools {
                    role_name: role_name_for_confirm.clone(),
                    role_path: role_path_for_confirm.clone(),
                    denied_tools: selected_tools.to_vec(),
                });
            }
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

    pub(crate) fn open_agent_role_template_edit_prompt(
        &mut self,
        role_name: String,
        role_path: std::path::PathBuf,
    ) {
        match user_agent_role_template_draft_from_file(&self.config, &role_name, &role_path) {
            Ok(draft) => {
                let tx = self.app_event_tx.clone();
                let role_name_for_event = role_name.clone();
                let role_path_for_event = role_path.clone();
                let view = CustomPromptView::new(
                    format!("Edit {role_name} native TOML"),
                    "Edit native role TOML and press Enter".to_string(),
                    draft,
                    Some(role_path.display().to_string()),
                    Box::new(move |draft: String| {
                        tx.send(AppEvent::UpdateAgentRoleTemplateFromDraft {
                            role_name: role_name_for_event.clone(),
                            role_path: role_path_for_event.clone(),
                            draft,
                        });
                    }),
                );
                self.bottom_pane.show_view(Box::new(view));
            }
            Err(err) => {
                self.add_error_message(format!("Failed to prepare role template draft: {err}"));
                self.open_agent_role_templates_popup();
            }
        }
    }

    pub(crate) fn open_agent_role_template_edit_prompt_from_current_model(
        &mut self,
        role_name: String,
        role_path: std::path::PathBuf,
    ) {
        match user_agent_role_template_draft_from_current_model(
            &self.config,
            &role_name,
            &role_path,
        ) {
            Ok(draft) => {
                let tx = self.app_event_tx.clone();
                let role_name_for_event = role_name.clone();
                let role_path_for_event = role_path.clone();
                let view = CustomPromptView::new(
                    format!("Edit {role_name} model defaults"),
                    "Review native role TOML and press Enter".to_string(),
                    draft,
                    Some(role_path.display().to_string()),
                    Box::new(move |draft: String| {
                        tx.send(AppEvent::UpdateAgentRoleTemplateFromDraft {
                            role_name: role_name_for_event.clone(),
                            role_path: role_path_for_event.clone(),
                            draft,
                        });
                    }),
                );
                self.bottom_pane.show_view(Box::new(view));
            }
            Err(err) => {
                self.add_error_message(format!("Failed to prepare role template draft: {err}"));
                self.open_agent_role_templates_popup();
            }
        }
    }

    pub(crate) fn open_agent_role_template_edit_prompt_from_current_model_provider(
        &mut self,
        role_name: String,
        role_path: std::path::PathBuf,
    ) {
        match user_agent_role_template_draft_from_current_model_provider(
            &self.config,
            &role_name,
            &role_path,
        ) {
            Ok(draft) => {
                let tx = self.app_event_tx.clone();
                let role_name_for_event = role_name.clone();
                let role_path_for_event = role_path.clone();
                let view = CustomPromptView::new(
                    format!("Edit {role_name} model/provider defaults"),
                    "Review native role TOML and press Enter".to_string(),
                    draft,
                    Some(role_path.display().to_string()),
                    Box::new(move |draft: String| {
                        tx.send(AppEvent::UpdateAgentRoleTemplateFromDraft {
                            role_name: role_name_for_event.clone(),
                            role_path: role_path_for_event.clone(),
                            draft,
                        });
                    }),
                );
                self.bottom_pane.show_view(Box::new(view));
            }
            Err(err) => {
                self.add_error_message(format!("Failed to prepare role template draft: {err}"));
                self.open_agent_role_templates_popup();
            }
        }
    }

    pub(crate) fn open_agent_role_template_edit_prompt_from_current_reasoning(
        &mut self,
        role_name: String,
        role_path: std::path::PathBuf,
    ) {
        match user_agent_role_template_draft_from_current_reasoning(
            &self.config,
            &role_name,
            &role_path,
        ) {
            Ok(draft) => {
                let tx = self.app_event_tx.clone();
                let role_name_for_event = role_name.clone();
                let role_path_for_event = role_path.clone();
                let view = CustomPromptView::new(
                    format!("Edit {role_name} reasoning defaults"),
                    "Review native role TOML and press Enter".to_string(),
                    draft,
                    Some(role_path.display().to_string()),
                    Box::new(move |draft: String| {
                        tx.send(AppEvent::UpdateAgentRoleTemplateFromDraft {
                            role_name: role_name_for_event.clone(),
                            role_path: role_path_for_event.clone(),
                            draft,
                        });
                    }),
                );
                self.bottom_pane.show_view(Box::new(view));
            }
            Err(err) => {
                self.add_error_message(format!("Failed to prepare role template draft: {err}"));
                self.open_agent_role_templates_popup();
            }
        }
    }

    pub(crate) fn open_agent_role_template_edit_prompt_without_model_defaults(
        &mut self,
        role_name: String,
        role_path: std::path::PathBuf,
    ) {
        match user_agent_role_template_draft_without_model_defaults(
            &self.config,
            &role_name,
            &role_path,
        ) {
            Ok(draft) => {
                let tx = self.app_event_tx.clone();
                let role_name_for_event = role_name.clone();
                let role_path_for_event = role_path.clone();
                let view = CustomPromptView::new(
                    format!("Clear {role_name} model defaults"),
                    "Review native role TOML and press Enter".to_string(),
                    draft,
                    Some(role_path.display().to_string()),
                    Box::new(move |draft: String| {
                        tx.send(AppEvent::UpdateAgentRoleTemplateFromDraft {
                            role_name: role_name_for_event.clone(),
                            role_path: role_path_for_event.clone(),
                            draft,
                        });
                    }),
                );
                self.bottom_pane.show_view(Box::new(view));
            }
            Err(err) => {
                self.add_error_message(format!("Failed to prepare role template draft: {err}"));
                self.open_agent_role_templates_popup();
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

    pub(crate) fn open_agent_role_template_edit_prompt_with_denied_tools(
        &mut self,
        role_name: String,
        role_path: std::path::PathBuf,
        denied_tools: Vec<String>,
    ) {
        match user_agent_role_template_draft_with_denied_tools(
            &self.config,
            &role_name,
            &role_path,
            &denied_tools,
        ) {
            Ok(draft) => {
                let tx = self.app_event_tx.clone();
                let role_name_for_event = role_name.clone();
                let role_path_for_event = role_path.clone();
                let view = CustomPromptView::new(
                    format!("Edit {role_name} denied tools"),
                    "Review native role TOML and press Enter".to_string(),
                    draft,
                    Some(role_path.display().to_string()),
                    Box::new(move |draft: String| {
                        tx.send(AppEvent::UpdateAgentRoleTemplateFromDraft {
                            role_name: role_name_for_event.clone(),
                            role_path: role_path_for_event.clone(),
                            draft,
                        });
                    }),
                );
                self.bottom_pane.show_view(Box::new(view));
            }
            Err(err) => {
                self.add_error_message(format!("Failed to prepare role template draft: {err}"));
                self.open_agent_role_templates_popup();
            }
        }
    }

    pub(crate) fn open_agent_role_template_edit_prompt_with_model_defaults(
        &mut self,
        role_name: String,
        role_path: std::path::PathBuf,
        model_provider: String,
        model: String,
        reasoning_effort: Option<ReasoningEffortConfig>,
        service_tier: Option<String>,
    ) {
        let defaults = role_model_defaults(model_provider, model, reasoning_effort, service_tier);
        match user_agent_role_template_draft_with_model_defaults(
            &self.config,
            &role_name,
            &role_path,
            &defaults,
        ) {
            Ok(draft) => {
                let tx = self.app_event_tx.clone();
                let role_name_for_event = role_name.clone();
                let role_path_for_event = role_path.clone();
                let view = CustomPromptView::new(
                    format!("Edit {role_name} model defaults"),
                    "Review native role TOML and press Enter".to_string(),
                    draft,
                    Some(role_path.display().to_string()),
                    Box::new(move |draft: String| {
                        tx.send(AppEvent::UpdateAgentRoleTemplateFromDraft {
                            role_name: role_name_for_event.clone(),
                            role_path: role_path_for_event.clone(),
                            draft,
                        });
                    }),
                );
                self.bottom_pane.show_view(Box::new(view));
            }
            Err(err) => {
                self.add_error_message(format!("Failed to prepare role template draft: {err}"));
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

fn role_template_edit_item(entry: &AgentRoleTemplateEntry) -> Option<SelectionItem> {
    if !user_agents_dir_role_is_editable(entry) {
        return None;
    }
    let role_path = entry.config_file.clone()?;
    let role_name = entry.name.clone();
    let role_name_for_action = role_name.clone();
    Some(SelectionItem {
        name: format!("Edit native TOML for {role_name}"),
        name_prefix_spans: vec!["[edit] ".dim()],
        description: Some("Open this user role file in the inline TOML editor".to_string()),
        selected_description: Some(
            "Reads the existing $CODEX_HOME/agents TOML file as text, validates it with the native role parser, and saves through the same native update/reload path. Comments and field order are preserved unless edited.".to_string(),
        ),
        actions: vec![Box::new(move |tx| {
            tx.send(AppEvent::OpenAgentRoleTemplateEditPrompt {
                role_name: role_name_for_action.clone(),
                role_path: role_path.clone(),
            });
        })],
        dismiss_on_select: true,
        search_value: Some(format!("edit native toml {role_name}")),
        ..Default::default()
    })
}

fn role_template_current_model_item(entry: &AgentRoleTemplateEntry) -> Option<SelectionItem> {
    if !user_agents_dir_role_is_editable(entry) {
        return None;
    }
    let role_path = entry.config_file.clone()?;
    let role_name = entry.name.clone();
    let role_name_for_action = role_name.clone();
    Some(SelectionItem {
        name: "Use current model defaults".to_string(),
        name_prefix_spans: vec!["[model] ".dim()],
        description: Some(
            format!("Copy current model/provider/reasoning defaults into {role_name}")
        ),
        selected_description: Some(
            "Opens an editable native TOML draft for the existing $CODEX_HOME/agents role file. This updates only model, model_provider, model_reasoning_effort and service_tier; it does not switch the current session model or create a role-local provider registry.".to_string(),
        ),
        actions: vec![Box::new(move |tx| {
            tx.send(AppEvent::OpenAgentRoleTemplateEditPromptFromCurrentModel {
                role_name: role_name_for_action.clone(),
                role_path: role_path.clone(),
            });
        })],
        dismiss_on_select: true,
        search_value: Some(format!("use current model defaults {role_name}")),
        ..Default::default()
    })
}

fn role_template_model_catalog_item(entry: &AgentRoleTemplateEntry) -> Option<SelectionItem> {
    if !user_agents_dir_role_is_editable(entry) {
        return None;
    }
    let role_path = entry.config_file.clone()?;
    let role_name = entry.name.clone();
    let role_name_for_action = role_name.clone();
    Some(SelectionItem {
        name: format!("Choose model defaults for {role_name}"),
        name_prefix_spans: vec!["[model] ".dim()],
        description: Some("Pick model/provider/reasoning from native catalog".to_string()),
        selected_description: Some(
            "Opens the native model catalog and then an editable TOML draft for this $CODEX_HOME/agents role file. The selected defaults update only model, model_provider, model_reasoning_effort and service_tier; no role-local provider registry or current-session model switch is created.".to_string(),
        ),
        actions: vec![Box::new(move |tx| {
            tx.send(AppEvent::OpenAgentRoleTemplateEditModelPicker {
                role_name: role_name_for_action.clone(),
                role_path: role_path.clone(),
            });
        })],
        dismiss_on_select: true,
        search_value: Some(format!("choose model catalog defaults {role_name}")),
        ..Default::default()
    })
}

fn role_template_current_model_provider_item(
    entry: &AgentRoleTemplateEntry,
) -> Option<SelectionItem> {
    if !user_agents_dir_role_is_editable(entry) {
        return None;
    }
    let role_path = entry.config_file.clone()?;
    let role_name = entry.name.clone();
    let role_name_for_action = role_name.clone();
    Some(SelectionItem {
        name: "Use current model/provider".to_string(),
        name_prefix_spans: vec!["[model] ".dim()],
        description: Some(format!("Copy current model/provider defaults into {role_name}")),
        selected_description: Some(
            "Opens an editable native TOML draft for the existing $CODEX_HOME/agents role file. This updates only model and model_provider; it preserves role-local model_reasoning_effort, service_tier, tool selection, action policy and other native tables.".to_string(),
        ),
        actions: vec![Box::new(move |tx| {
            tx.send(
                AppEvent::OpenAgentRoleTemplateEditPromptFromCurrentModelProvider {
                    role_name: role_name_for_action.clone(),
                    role_path: role_path.clone(),
                },
            );
        })],
        dismiss_on_select: true,
        search_value: Some(format!("use current model provider {role_name}")),
        ..Default::default()
    })
}

fn role_template_current_reasoning_item(entry: &AgentRoleTemplateEntry) -> Option<SelectionItem> {
    if !user_agents_dir_role_is_editable(entry) {
        return None;
    }
    let role_path = entry.config_file.clone()?;
    let role_name = entry.name.clone();
    let role_name_for_action = role_name.clone();
    Some(SelectionItem {
        name: "Use current reasoning".to_string(),
        name_prefix_spans: vec!["[reasoning] ".dim()],
        description: Some(format!(
            "Copy current reasoning/service-tier defaults into {role_name}"
        )),
        selected_description: Some(
            "Opens an editable native TOML draft for the existing $CODEX_HOME/agents role file. This updates only model_reasoning_effort and service_tier; it preserves role-local model, model_provider, tool selection, action policy and other native tables.".to_string(),
        ),
        actions: vec![Box::new(move |tx| {
            tx.send(AppEvent::OpenAgentRoleTemplateEditPromptFromCurrentReasoning {
                role_name: role_name_for_action.clone(),
                role_path: role_path.clone(),
            });
        })],
        dismiss_on_select: true,
        search_value: Some(format!("use current reasoning {role_name}")),
        ..Default::default()
    })
}

fn role_template_clear_model_item(entry: &AgentRoleTemplateEntry) -> Option<SelectionItem> {
    if !user_agents_dir_role_is_editable(entry) || !role_has_model_defaults(entry) {
        return None;
    }
    let role_path = entry.config_file.clone()?;
    let role_name = entry.name.clone();
    let role_name_for_action = role_name.clone();
    Some(SelectionItem {
        name: "Clear model defaults".to_string(),
        name_prefix_spans: vec!["[model] ".dim()],
        description: Some(format!(
            "Remove role-local model/provider/reasoning defaults from {role_name}"
        )),
        selected_description: Some(
            "Opens an editable native TOML draft for the existing $CODEX_HOME/agents role file. This removes only model, model_provider, model_reasoning_effort and service_tier so future spawns inherit session defaults; it preserves tool/action policies and other native tables.".to_string(),
        ),
        actions: vec![Box::new(move |tx| {
            tx.send(AppEvent::OpenAgentRoleTemplateEditPromptWithoutModelDefaults {
                role_name: role_name_for_action.clone(),
                role_path: role_path.clone(),
            });
        })],
        dismiss_on_select: true,
        search_value: Some(format!("clear model defaults {role_name}")),
        ..Default::default()
    })
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

fn create_role_template_from_current_model_item() -> SelectionItem {
    SelectionItem {
        name: "Create template from current model".to_string(),
        description: Some("Seed a native TOML role draft with current model defaults".to_string()),
        selected_description: Some(
            "Copies the current Config model, model_provider, model_reasoning_effort and service_tier into an editable $CODEX_HOME/agents TOML draft. It does not change the current session model or create a role-local provider registry.".to_string(),
        ),
        actions: vec![Box::new(|tx| {
            tx.send(AppEvent::OpenAgentRoleTemplateCreatePromptFromCurrentModel);
        })],
        dismiss_on_select: true,
        ..Default::default()
    }
}

fn create_role_template_from_model_catalog_item() -> SelectionItem {
    SelectionItem {
        name: "Create template from model catalog".to_string(),
        description: Some("Choose model/provider/reasoning defaults from native catalog".to_string()),
        selected_description: Some(
            "Opens the native model catalog, then writes the selected model, model_provider, model_reasoning_effort and service_tier into an editable $CODEX_HOME/agents TOML draft. It does not change the current session model.".to_string(),
        ),
        actions: vec![Box::new(|tx| {
            tx.send(AppEvent::OpenAgentRoleTemplateCreateModelPicker);
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

fn import_markdown_agent_roles_item() -> SelectionItem {
    SelectionItem {
        name: "Import markdown/frontmatter agents".to_string(),
        description: Some("Compile external markdown agents into native TOML roles".to_string()),
        selected_description: Some(
            "Opens the existing /import flow. Markdown/frontmatter agents are compiled into native $CODEX_HOME/agents/*.toml files; Codex does not load markdown profiles at runtime.".to_string(),
        ),
        actions: vec![Box::new(|tx| {
            tx.send(AppEvent::OpenExternalAgentConfigMigration);
        })],
        dismiss_on_select: true,
        ..Default::default()
    }
}

fn role_template_tool_selection_item(
    entry: &AgentRoleTemplateEntry,
    runtime_catalog: &AgentRoleRuntimeCatalogState,
) -> Option<SelectionItem> {
    role_template_tool_selection_item_for_kind(
        entry,
        runtime_catalog,
        RoleToolSelectionEditKind::Allowed,
    )
}

fn role_template_denied_tool_selection_item(
    entry: &AgentRoleTemplateEntry,
    runtime_catalog: &AgentRoleRuntimeCatalogState,
) -> Option<SelectionItem> {
    role_template_tool_selection_item_for_kind(
        entry,
        runtime_catalog,
        RoleToolSelectionEditKind::Denied,
    )
}

fn role_template_tool_selection_item_for_kind(
    entry: &AgentRoleTemplateEntry,
    runtime_catalog: &AgentRoleRuntimeCatalogState,
    edit_kind: RoleToolSelectionEditKind,
) -> Option<SelectionItem> {
    if !user_agents_dir_role_is_editable(entry) {
        return None;
    }
    let AgentRoleRuntimeCatalogState::Loaded(catalog) = runtime_catalog else {
        return None;
    };
    let role_path = entry.config_file.clone()?;
    let role_name = entry.name.clone();
    let selected_tools = match edit_kind {
        RoleToolSelectionEditKind::Allowed => entry.locks.allowed_tool_names.clone(),
        RoleToolSelectionEditKind::Denied => entry.locks.denied_tool_names.clone(),
    };
    let catalog_entries = catalog.data.clone();
    let role_name_for_action = role_name.clone();
    Some(SelectionItem {
        name: format!("Edit {} tools for {role_name}", edit_kind.label()),
        name_prefix_spans: vec!["[user tools] ".dim()],
        description: Some(format!(
            "Update this native role TOML {}_tools list from current tools",
            edit_kind.label()
        )),
        selected_description: Some(format!(
            "Opens a searchable current-thread tool picker, then an editable native TOML draft for the existing $CODEX_HOME/agents role file. This updates [tool_selection].{}_tools without adding an app-server write API or a parallel role registry.",
            edit_kind.label()
        )),
        actions: vec![Box::new(move |tx| match edit_kind {
            RoleToolSelectionEditKind::Allowed => {
                tx.send(AppEvent::OpenAgentRoleTemplateToolSelectionPickerForRole {
                    role_name: role_name_for_action.clone(),
                    role_path: role_path.clone(),
                    selected_tools: selected_tools.clone(),
                    catalog_entries: catalog_entries.clone(),
                });
            }
            RoleToolSelectionEditKind::Denied => {
                tx.send(
                    AppEvent::OpenAgentRoleTemplateDeniedToolSelectionPickerForRole {
                        role_name: role_name_for_action.clone(),
                        role_path: role_path.clone(),
                        selected_tools: selected_tools.clone(),
                        catalog_entries: catalog_entries.clone(),
                    },
                );
            }
        })],
        dismiss_on_select: true,
        search_value: Some(format!("edit {} tools {role_name}", edit_kind.label())),
        ..Default::default()
    })
}

fn send_role_model_defaults_event(
    tx: &crate::app_event_sender::AppEventSender,
    target: RoleModelDefaultsTarget,
    preset: &ModelPreset,
    reasoning_effort: Option<ReasoningEffortConfig>,
) {
    match target {
        RoleModelDefaultsTarget::NewRole => {
            tx.send(
                AppEvent::OpenAgentRoleTemplateCreatePromptWithModelDefaults {
                    model_provider: preset.model_provider.clone(),
                    model: preset.model.clone(),
                    reasoning_effort,
                    service_tier: preset.default_service_tier.clone(),
                },
            );
        }
        RoleModelDefaultsTarget::ExistingRole {
            role_name,
            role_path,
        } => {
            tx.send(AppEvent::OpenAgentRoleTemplateEditPromptWithModelDefaults {
                role_name,
                role_path,
                model_provider: preset.model_provider.clone(),
                model: preset.model.clone(),
                reasoning_effort,
                service_tier: preset.default_service_tier.clone(),
            });
        }
    }
}

fn role_model_defaults(
    model_provider: String,
    model: String,
    reasoning_effort: Option<ReasoningEffortConfig>,
    service_tier: Option<String>,
) -> AgentRoleTemplateModelDefaults {
    AgentRoleTemplateModelDefaults {
        model: Some(model),
        model_provider: (!model_provider.trim().is_empty()).then_some(model_provider),
        model_reasoning_effort: reasoning_effort.map(|effort| effort.to_string()),
        service_tier: service_tier.filter(|service_tier| !service_tier.trim().is_empty()),
    }
}

fn user_agents_dir_role_is_editable(entry: &AgentRoleTemplateEntry) -> bool {
    entry.source == AgentRoleTemplateSource::User
        && entry.validation_error.is_none()
        && entry.config_file_origin
            == Some(
                crate::legacy_core::agent_role_templates::AgentRoleTemplateConfigFileOrigin::UserAgentsDir,
            )
}

fn role_has_model_defaults(entry: &AgentRoleTemplateEntry) -> bool {
    entry.locks.model.is_some()
        || entry.locks.model_provider.is_some()
        || entry.locks.reasoning_effort.is_some()
        || entry.locks.service_tier.is_some()
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
    if !entry.locks.allowed_tool_names.is_empty() || !entry.locks.denied_tool_names.is_empty() {
        lines.push("Tool selection state: native runtime allowlist/denylist; unmatched entries warn during tool planning and grant no access".to_string());
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
            "tool_selection.allowed={}",
            entry.locks.allowed_tool_names.join(", ")
        ));
    }
    if !entry.locks.denied_tool_names.is_empty() {
        locked.push(format!(
            "tool_selection.denied={}",
            entry.locks.denied_tool_names.join(", ")
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
                    "Built-in role; create a separate user template or edit native config to shadow it.".to_string(),
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
            if !entry.locks.denied_tool_names.is_empty() {
                let catalog_names = catalog
                    .data
                    .iter()
                    .map(|catalog_entry| catalog_entry.name.as_str())
                    .collect::<std::collections::BTreeSet<_>>();
                let present = entry
                    .locks
                    .denied_tool_names
                    .iter()
                    .filter(|name| catalog_names.contains(name.as_str()))
                    .count();
                let missing = entry.locks.denied_tool_names.len().saturating_sub(present);
                lines.push(format!(
                    "Template denylist vs current catalog: {present} present, {missing} absent"
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

fn agent_role_tool_selection_items(
    entries: &[AgentRoleToolSelectionCatalogEntry],
    selected_tools: &BTreeSet<String>,
    edit_kind: RoleToolSelectionEditKind,
) -> Vec<MultiSelectItem> {
    let catalog_names = entries
        .iter()
        .map(|entry| entry.name.as_str())
        .collect::<BTreeSet<_>>();
    let mut items = entries
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

    let missing_selected_tools = selected_tools
        .iter()
        .filter(|tool| !catalog_names.contains(tool.as_str()))
        .collect::<Vec<_>>();
    if !missing_selected_tools.is_empty()
        && let Some(last) = items.last_mut()
    {
        last.section_break_after = true;
    }
    for tool in missing_selected_tools {
        items.push(MultiSelectItem {
            id: tool.clone(),
            name: tool.clone(),
            description: Some(format!(
                "{} tool id not available in the current thread catalog",
                edit_kind.label()
            )),
            enabled: true,
            orderable: false,
            section_break_after: false,
        });
    }

    items
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::legacy_core::agent_role_templates::AgentRoleTemplateAvailability;
    use crate::legacy_core::agent_role_templates::AgentRoleTemplateLocks;
    use pretty_assertions::assert_eq;

    #[test]
    fn tool_selection_items_preserve_selected_tools_missing_from_catalog() {
        let entries = vec![AgentRoleToolSelectionCatalogEntry {
            name: "update_plan".to_string(),
            selected: false,
            exposure: AgentRoleToolSelectionCatalogExposure::Direct,
        }];
        let selected_tools = ["future_tool".to_string(), "update_plan".to_string()]
            .into_iter()
            .collect::<BTreeSet<_>>();

        let items = agent_role_tool_selection_items(
            &entries,
            &selected_tools,
            RoleToolSelectionEditKind::Allowed,
        );

        assert_eq!(
            items
                .iter()
                .map(|item| (item.id.as_str(), item.enabled, item.description.as_deref()))
                .collect::<Vec<_>>(),
            vec![
                ("update_plan", true, Some("direct tool id")),
                (
                    "future_tool",
                    true,
                    Some("allowed tool id not available in the current thread catalog"),
                ),
            ]
        );
    }

    #[test]
    fn role_template_detail_renders_allowlist_and_denylist_separately() {
        let entry = AgentRoleTemplateEntry {
            name: "reviewer".to_string(),
            description: Some("Review code changes".to_string()),
            source: AgentRoleTemplateSource::User,
            is_shadowed: false,
            shadows_built_in: false,
            availability: AgentRoleTemplateAvailability::NewSessionsOnly,
            config_file: None,
            config_file_origin: None,
            nickname_candidates: Vec::new(),
            locks: AgentRoleTemplateLocks {
                allowed_tool_names: vec!["update_plan".to_string()],
                denied_tool_names: vec!["apply_patch".to_string()],
                ..Default::default()
            },
            field_sources: Default::default(),
            validation_error: None,
        };

        let detail = role_template_detail(
            &entry,
            &AgentRoleRuntimeCatalogState::NotRequested,
            &BTreeMap::new(),
        );

        assert!(detail.contains("native runtime allowlist/denylist"));
        assert!(detail.contains("tool_selection.allowed=update_plan"));
        assert!(detail.contains("tool_selection.denied=apply_patch"));
    }
}
