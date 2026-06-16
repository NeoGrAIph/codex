use std::path::Path;

use url::Url;

use super::*;
use crate::legacy_core::agent_role_templates::AgentRoleTemplateEntry;
use crate::legacy_core::agent_role_templates::AgentRoleTemplateSource;
use crate::legacy_core::agent_role_templates::create_user_agent_role_template;
use crate::legacy_core::agent_role_templates::list_agent_role_templates;

const AGENT_ROLE_TEMPLATES_TITLE: &str = "Agent Role Templates";

impl ChatWidget {
    pub(crate) fn open_agent_role_templates_popup(&mut self) {
        let entries = list_agent_role_templates(&self.config);
        let mut items = Vec::with_capacity(entries.len() + 1);
        items.push(create_role_template_item());
        items.extend(entries.into_iter().map(role_template_item));

        let mut header = ColumnRenderable::new();
        header.push(Line::from(AGENT_ROLE_TEMPLATES_TITLE.bold()));
        header.push(Line::from(
            "Create and inspect role profiles used by spawn_agent.agent_type.".dim(),
        ));
        header.push(Line::from(
            "User templates live in $CODEX_HOME/agents/*.toml.".dim(),
        ));

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
        let tx = self.app_event_tx.clone();
        let view = CustomPromptView::new(
            "Create role template".to_string(),
            "Role name, for example: code reviewer".to_string(),
            String::new(),
            Some("$CODEX_HOME/agents/<role>.toml".to_string()),
            Box::new(move |raw_name: String| {
                tx.send(AppEvent::CreateAgentRoleTemplate { raw_name });
            }),
        );
        self.bottom_pane.show_view(Box::new(view));
    }

    pub(crate) fn create_agent_role_template(&mut self, raw_name: String) {
        match create_user_agent_role_template(&self.config, &raw_name) {
            Ok(created) => {
                self.config
                    .agent_roles
                    .insert(created.name.clone(), created.config);
                self.add_info_message(
                    format!("Created agent role template `{}`", created.name),
                    Some(format!(
                        "{}\nNew sessions load this role from $CODEX_HOME/agents/.",
                        created.path.display()
                    )),
                );
                self.open_agent_role_templates_popup();
            }
            Err(err) => {
                self.add_error_message(format!("Failed to create agent role template: {err}"));
                self.open_agent_role_templates_popup();
            }
        }
    }
}

fn create_role_template_item() -> SelectionItem {
    SelectionItem {
        name: "Create new template".to_string(),
        description: Some("Add a user role TOML file for future sub-agent spawns".to_string()),
        selected_description: Some(
            "Creates a valid starter file under $CODEX_HOME/agents/. New sessions load user roles from that native config directory.".to_string(),
        ),
        actions: vec![Box::new(|tx| {
            tx.send(AppEvent::OpenAgentRoleTemplateCreatePrompt);
        })],
        dismiss_on_select: true,
        ..Default::default()
    }
}

fn role_template_item(entry: AgentRoleTemplateEntry) -> SelectionItem {
    let source_label = entry.source.label();
    let name = entry.name.clone();
    let description = entry
        .description
        .as_deref()
        .map(short_role_template_description);
    let selected_description = Some(role_template_detail(&entry));
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

fn role_template_detail(entry: &AgentRoleTemplateEntry) -> String {
    let mut lines = vec![
        format!("Role: {}", entry.name),
        format!("Source: {}", entry.source.label()),
    ];

    if let Some(description) = &entry.description {
        lines.push(format!("Description: {description}"));
    }
    if let Some(path) = &entry.config_file {
        lines.push(format!("Config file: {}", path.display()));
    } else {
        lines.push("Config file: none".to_string());
    }
    if !entry.nickname_candidates.is_empty() {
        lines.push(format!(
            "Nicknames: {}",
            entry.nickname_candidates.join(", ")
        ));
    }

    let mut locked = Vec::new();
    if let Some(model) = &entry.locks.model {
        locked.push(format!("model={model}"));
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
    if !locked.is_empty() {
        lines.push(format!("Template sets: {}", locked.join(", ")));
    }

    match entry.source {
        AgentRoleTemplateSource::User => {
            lines.push("Enter: open template file for editing".to_string());
        }
        AgentRoleTemplateSource::BuiltIn => {
            lines.push("Built-in role; create a user template to customize behavior.".to_string());
        }
    }
    lines.push(format!(
        "Use in new sessions: ask Codex to spawn a sub-agent with agent_type `{}`.",
        entry.name
    ));

    lines.join("\n")
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
