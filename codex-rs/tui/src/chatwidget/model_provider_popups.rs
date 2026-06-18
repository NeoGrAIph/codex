use super::*;
use crate::app_event::SensitiveString;
use crate::bottom_pane::SelectionToggle;
use codex_app_server_protocol::ModelProvider;
use codex_app_server_protocol::ModelProviderAuthStatus;
use codex_model_provider_info::OPENAI_PROVIDER_ID;

const MODEL_PROVIDERS_TITLE: &str = "Model Providers";

impl ChatWidget {
    pub(crate) fn open_model_providers_popup(&mut self, providers: Vec<ModelProvider>) {
        let providers = providers
            .into_iter()
            .filter(|provider| provider.id != OPENAI_PROVIDER_ID)
            .collect::<Vec<_>>();
        if providers.is_empty() {
            self.add_info_message(
                "No configurable model providers are available.".to_string(),
                Some("OpenAI is native and always available.".to_string()),
            );
            return;
        }

        let mut header = ColumnRenderable::new();
        header.push(Line::from(MODEL_PROVIDERS_TITLE.bold()));
        header.push(Line::from(
            "Space toggles /model visibility. Enter opens details. OpenAI is always available."
                .dim(),
        ));

        let current_index = providers
            .iter()
            .position(|provider| provider.active)
            .or_else(|| {
                providers
                    .iter()
                    .position(|provider| provider.enabled_in_picker)
            });
        self.show_selection_view(SelectionViewParams {
            header: Box::new(header),
            footer_hint: Some(Line::from("space toggle · enter details · esc close".dim())),
            items: providers.into_iter().map(provider_item).collect(),
            initial_selected_idx: current_index,
            is_searchable: true,
            search_placeholder: Some("Search providers".to_string()),
            col_width_mode: ColumnWidthMode::AutoAllRows,
            ..Default::default()
        });
    }

    pub(crate) fn open_model_provider_detail_popup(&mut self, provider: ModelProvider) {
        let mut header = ColumnRenderable::new();
        header.push(Line::from(provider.name.clone().bold()));
        header.push(Line::from(format!("Provider ID: {}", provider.id).dim()));

        self.show_selection_view(SelectionViewParams {
            header: Box::new(header),
            footer_hint: Some(Line::from("enter action · esc close".dim())),
            items: provider_detail_items(provider),
            initial_selected_idx: Some(0),
            is_searchable: false,
            col_width_mode: ColumnWidthMode::AutoAllRows,
            ..Default::default()
        });
    }

    pub(crate) fn open_model_provider_api_key_prompt(&mut self, provider_id: String) {
        let tx = self.app_event_tx.clone();
        let title = format!("Set API key for {provider_id}");
        let view = CustomPromptView::new_sensitive(
            title,
            "Paste API key and press Enter".to_string(),
            String::new(),
            Some("Stored in encrypted Codex managed secrets.".to_string()),
            Box::new(move |api_key: String| {
                tx.send(AppEvent::SaveModelProviderApiKey {
                    provider_id: provider_id.clone(),
                    api_key: SensitiveString::new(api_key),
                });
            }),
        );
        self.bottom_pane.show_view(Box::new(view));
    }
}

fn provider_item(provider: ModelProvider) -> SelectionItem {
    let id = provider.id.clone();
    let active = provider.active;
    let status = provider_auth_status_label(&provider.auth_status);
    let name = provider.name.clone();
    let description = Some(format!(
        "{} · {status} · {} models",
        provider.id, provider.model_count
    ));
    let selected_description = Some(provider_list_selected_description(&provider));
    let can_toggle = !active;
    let toggle = can_toggle.then(|| SelectionToggle {
        is_on: provider.enabled_in_picker,
        action: Box::new(move |enabled, tx| {
            tx.send(AppEvent::SetModelProviderEnabled {
                provider_id: id.clone(),
                enabled,
            });
        }),
    });
    let detail_provider = provider.clone();
    let actions: Vec<SelectionAction> = vec![Box::new(move |tx| {
        tx.send(AppEvent::OpenModelProviderDetail {
            provider: detail_provider.clone(),
        });
    })];

    SelectionItem {
        name,
        toggle,
        toggle_placeholder: (!can_toggle).then_some("[-] "),
        description,
        selected_description,
        actions,
        dismiss_on_select: false,
        dismiss_parent_on_child_accept: false,
        search_value: Some(format!("{} {}", provider.id, provider.name)),
        ..Default::default()
    }
}

fn provider_list_selected_description(provider: &ModelProvider) -> String {
    let status = provider_auth_status_label(&provider.auth_status);
    let toggle_action = if provider.enabled_in_picker {
        "hide from"
    } else {
        "show in"
    };
    if provider.active {
        format!("{status}   Active default provider. Enter view details.")
    } else {
        format!("{status}   Space to {toggle_action} /model; Enter view details.")
    }
}

fn provider_detail_items(provider: ModelProvider) -> Vec<SelectionItem> {
    let mut items = Vec::new();
    let detail = Some(provider_detail(&provider));
    if !provider.active {
        let provider_id = provider.id.clone();
        items.push(SelectionItem {
            name: "Set as default provider".to_string(),
            description: Some("Use this provider for newly selected models.".to_string()),
            selected_description: detail.clone(),
            actions: vec![Box::new(move |tx| {
                tx.send(AppEvent::SetActiveModelProvider {
                    provider_id: provider_id.clone(),
                });
            })],
            dismiss_on_select: false,
            ..Default::default()
        });
    }

    let provider_id = provider.id.clone();
    items.push(SelectionItem {
        name: "Set managed API key".to_string(),
        description: Some("Store or replace the local encrypted provider key.".to_string()),
        selected_description: detail.clone(),
        actions: vec![Box::new(move |tx| {
            tx.send(AppEvent::OpenModelProviderApiKeyPrompt {
                provider_id: provider_id.clone(),
            });
        })],
        dismiss_on_select: false,
        ..Default::default()
    });

    let provider_id = provider.id;
    items.push(SelectionItem {
        name: "Clear managed API key".to_string(),
        description: Some("Remove the local encrypted provider key.".to_string()),
        selected_description: detail,
        actions: vec![Box::new(move |tx| {
            tx.send(AppEvent::ClearModelProviderApiKey {
                provider_id: provider_id.clone(),
            });
        })],
        dismiss_on_select: false,
        ..Default::default()
    });

    items
}

fn provider_detail(provider: &ModelProvider) -> String {
    let auth = provider_auth_status_label(&provider.auth_status);
    let env = provider.env_key.as_deref().unwrap_or("none");
    let base_url = provider.base_url.as_deref().unwrap_or("default");
    let enabled = if provider.enabled_in_picker {
        "shown in /model"
    } else {
        "hidden from /model"
    };
    let active = if provider.active { "yes" } else { "no" };
    format!(
        "Provider: {}\nActive default: {active}\nPicker: {enabled}\nAuth: {auth}\nEnv key: {env}\nWire API: {}\nBase URL: {base_url}\nModels in picker: {}",
        provider.name, provider.wire_api, provider.model_count
    )
}

fn provider_auth_status_label(status: &ModelProviderAuthStatus) -> &'static str {
    match status {
        ModelProviderAuthStatus::OpenAiAuth => "OpenAI auth",
        ModelProviderAuthStatus::EnvKeyPresent => "env key present",
        ModelProviderAuthStatus::EnvKeyMissing => "env key missing",
        ModelProviderAuthStatus::ManagedKeyPresent => "managed key present",
        ModelProviderAuthStatus::ManagedKeyMissing => "managed key missing",
        ModelProviderAuthStatus::Command => "command auth",
        ModelProviderAuthStatus::Aws => "AWS auth",
        ModelProviderAuthStatus::InlineBearer => "inline bearer",
        ModelProviderAuthStatus::None => "no auth",
    }
}
