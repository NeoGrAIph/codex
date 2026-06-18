use super::*;
use crate::models::model_from_preset;
use codex_app_server_protocol::Model;
use codex_config::config_toml::ConfigToml;
use codex_model_provider_info::ModelProviderInfo;
use codex_model_provider_info::OPENAI_PROVIDER_ID;
use codex_models_manager::manager::RefreshStrategy;
use codex_secrets::SecretName;
use codex_secrets::SecretScope;
use codex_secrets::SecretsBackendKind;
use codex_secrets::SecretsManager;
use futures::StreamExt;

#[derive(Clone)]
pub(crate) struct CatalogRequestProcessor {
    pub(super) outgoing: Arc<OutgoingMessageSender>,
    pub(super) skills_watcher: Arc<SkillsWatcher>,
    pub(super) auth_manager: Arc<AuthManager>,
    pub(super) thread_manager: Arc<ThreadManager>,
    pub(super) config: Arc<Config>,
    pub(super) config_manager: ConfigManager,
    pub(super) workspace_settings_cache: Arc<workspace_settings::WorkspaceSettingsCache>,
}

const SKILLS_LIST_CWD_CONCURRENCY: usize = 5;

fn skills_to_info(
    skills: &[codex_core::skills::SkillMetadata],
    disabled_paths: &HashSet<AbsolutePathBuf>,
) -> Vec<codex_app_server_protocol::SkillMetadata> {
    skills
        .iter()
        .map(|skill| {
            let enabled = !disabled_paths.contains(&skill.path_to_skills_md);
            codex_app_server_protocol::SkillMetadata {
                name: skill.name.clone(),
                description: skill.description.clone(),
                short_description: skill.short_description.clone(),
                interface: skill.interface.clone().map(|interface| {
                    codex_app_server_protocol::SkillInterface {
                        display_name: interface.display_name,
                        short_description: interface.short_description,
                        icon_small: interface.icon_small,
                        icon_large: interface.icon_large,
                        brand_color: interface.brand_color,
                        default_prompt: interface.default_prompt,
                    }
                }),
                dependencies: skill.dependencies.clone().map(|dependencies| {
                    codex_app_server_protocol::SkillDependencies {
                        tools: dependencies
                            .tools
                            .into_iter()
                            .map(|tool| codex_app_server_protocol::SkillToolDependency {
                                r#type: tool.r#type,
                                value: tool.value,
                                description: tool.description,
                                transport: tool.transport,
                                command: tool.command,
                                url: tool.url,
                            })
                            .collect(),
                    }
                }),
                path: skill.path_to_skills_md.clone(),
                scope: skill.scope.into(),
                enabled,
            }
        })
        .collect()
}

fn hooks_to_info(hooks: &[codex_hooks::HookListEntry]) -> Vec<HookMetadata> {
    hooks
        .iter()
        .map(|hook| HookMetadata {
            key: hook.key.clone(),
            event_name: hook.event_name.into(),
            handler_type: hook.handler_type.into(),
            matcher: hook.matcher.clone(),
            command: hook.command.clone(),
            timeout_sec: hook.timeout_sec,
            status_message: hook.status_message.clone(),
            source_path: hook.source_path.clone(),
            source: hook.source.into(),
            plugin_id: hook.plugin_id.clone(),
            display_order: hook.display_order,
            enabled: hook.enabled,
            is_managed: hook.is_managed,
            current_hash: hook.current_hash.clone(),
            trust_status: hook.trust_status.into(),
        })
        .collect()
}

fn errors_to_info(
    errors: &[codex_core::skills::SkillError],
) -> Vec<codex_app_server_protocol::SkillErrorInfo> {
    errors
        .iter()
        .map(|err| codex_app_server_protocol::SkillErrorInfo {
            path: err.path.to_path_buf(),
            message: err.message.clone(),
        })
        .collect()
}

impl CatalogRequestProcessor {
    pub(crate) fn new(
        outgoing: Arc<OutgoingMessageSender>,
        skills_watcher: Arc<SkillsWatcher>,
        auth_manager: Arc<AuthManager>,
        thread_manager: Arc<ThreadManager>,
        config: Arc<Config>,
        config_manager: ConfigManager,
        workspace_settings_cache: Arc<workspace_settings::WorkspaceSettingsCache>,
    ) -> Self {
        Self {
            outgoing,
            skills_watcher,
            auth_manager,
            thread_manager,
            config,
            config_manager,
            workspace_settings_cache,
        }
    }

    pub(crate) async fn skills_list(
        &self,
        params: SkillsListParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.skills_list_response(params)
            .await
            .map(|response| Some(response.into()))
    }

    pub(crate) async fn hooks_list(
        &self,
        params: HooksListParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.hooks_list_response(params)
            .await
            .map(|response| Some(response.into()))
    }

    pub(crate) async fn skills_config_write(
        &self,
        params: SkillsConfigWriteParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.skills_config_write_response_inner(params)
            .await
            .map(|response| Some(response.into()))
    }

    pub(crate) async fn skills_extra_roots_set(
        &self,
        params: SkillsExtraRootsSetParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.skills_extra_roots_set_response(params)
            .await
            .map(|response| Some(response.into()))
    }

    pub(crate) async fn model_list(
        &self,
        params: ModelListParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.list_models(params)
            .await
            .map(|response| Some(response.into()))
    }

    pub(crate) async fn model_provider_list(
        &self,
        params: ModelProviderListParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.list_model_providers(params)
            .await
            .map(|response| Some(response.into()))
    }

    pub(crate) async fn agent_role_tool_selection_catalog_read(
        &self,
        params: AgentRoleToolSelectionCatalogReadParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.read_agent_role_tool_selection_catalog(params)
            .await
            .map(|response| Some(response.into()))
    }

    pub(crate) async fn model_provider_config_write(
        &self,
        params: ModelProviderConfigWriteParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.write_model_provider_config(params)
            .await
            .map(|response| Some(response.into()))
    }

    pub(crate) async fn model_provider_auth_write(
        &self,
        params: ModelProviderAuthWriteParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.write_model_provider_auth(params)
            .await
            .map(|response| Some(response.into()))
    }

    pub(crate) async fn model_provider_auth_remove(
        &self,
        params: ModelProviderAuthRemoveParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.remove_model_provider_auth(params)
            .await
            .map(|response| Some(response.into()))
    }

    pub(crate) async fn experimental_feature_list(
        &self,
        params: ExperimentalFeatureListParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.experimental_feature_list_response(params)
            .await
            .map(|response| Some(response.into()))
    }

    pub(crate) async fn permission_profile_list(
        &self,
        params: PermissionProfileListParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.permission_profile_list_response(params)
            .await
            .map(|response| Some(response.into()))
    }

    pub(crate) async fn collaboration_mode_list(
        &self,
        params: CollaborationModeListParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        Self::list_collaboration_modes(self.thread_manager.clone(), params)
            .await
            .map(|response| Some(response.into()))
    }

    pub(crate) async fn mock_experimental_method(
        &self,
        params: MockExperimentalMethodParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.mock_experimental_method_inner(params)
            .await
            .map(|response| Some(response.into()))
    }

    async fn resolve_cwd_config(
        &self,
        cwd: &Path,
    ) -> Result<(AbsolutePathBuf, ConfigLayerStack), String> {
        let cwd_abs =
            AbsolutePathBuf::relative_to_current_dir(cwd).map_err(|err| err.to_string())?;
        let config_layer_stack = self
            .config_manager
            .load_config_layers_for_cwd(cwd_abs.clone())
            .await
            .map_err(|err| err.to_string())?;

        Ok((cwd_abs, config_layer_stack))
    }

    async fn load_latest_config(
        &self,
        fallback_cwd: Option<PathBuf>,
    ) -> Result<Config, JSONRPCErrorError> {
        self.config_manager
            .load_latest_config(fallback_cwd)
            .await
            .map_err(|err| internal_error(format!("failed to reload config: {err}")))
    }

    async fn workspace_codex_plugins_enabled(
        &self,
        config: &Config,
        auth: Option<&CodexAuth>,
    ) -> bool {
        match workspace_settings::codex_plugins_enabled_for_workspace(
            config,
            auth,
            Some(&self.workspace_settings_cache),
        )
        .await
        {
            Ok(enabled) => enabled,
            Err(err) => {
                warn!(
                    "failed to fetch workspace Codex plugins setting; allowing Codex plugins: {err:#}"
                );
                true
            }
        }
    }

    async fn list_models(
        &self,
        params: ModelListParams,
    ) -> Result<ModelListResponse, JSONRPCErrorError> {
        let ModelListParams {
            limit,
            cursor,
            include_hidden,
            include_configured_providers,
        } = params;
        let include_hidden = include_hidden.unwrap_or(false);
        let models = if include_configured_providers.unwrap_or(false) {
            self.supported_models_from_configured_providers(include_hidden)
                .await?
        } else {
            supported_models(self.thread_manager.clone(), include_hidden).await
        };
        Self::paginate_models(models, limit, cursor)
    }

    async fn supported_models_from_configured_providers(
        &self,
        include_hidden: bool,
    ) -> Result<Vec<Model>, JSONRPCErrorError> {
        let config = self.load_latest_config(/*fallback_cwd*/ None).await?;
        let mut provider_ids = config.model_providers.keys().cloned().collect::<Vec<_>>();
        provider_ids.sort();

        let mut models = Vec::new();
        for provider_id in provider_ids {
            let Some(provider_info) = config.model_providers.get(&provider_id).cloned() else {
                continue;
            };
            let is_active_provider = provider_id == config.model_provider_id;
            if provider_id != OPENAI_PROVIDER_ID
                && !is_active_provider
                && config.disabled_model_providers.contains(&provider_id)
            {
                continue;
            }
            if !is_active_provider && !has_authoritative_inactive_catalog(&provider_info) {
                continue;
            }
            let config_model_catalog = is_active_provider
                .then(|| config.model_catalog.clone())
                .flatten();
            let refresh_strategy = if is_active_provider {
                RefreshStrategy::OnlineIfUncached
            } else {
                RefreshStrategy::Offline
            };
            let provider = create_model_provider(provider_info, Some(self.auth_manager.clone()));
            let manager =
                provider.models_manager(config.codex_home.to_path_buf(), config_model_catalog);
            models.extend(
                manager
                    .list_models(refresh_strategy)
                    .await
                    .into_iter()
                    .filter(|preset| include_hidden || preset.show_in_picker)
                    .map(|mut preset| {
                        preset.model_provider = provider_id.clone();
                        model_from_preset(preset)
                    }),
            );
        }

        Ok(models)
    }

    async fn list_model_providers(
        &self,
        _params: ModelProviderListParams,
    ) -> Result<ModelProviderListResponse, JSONRPCErrorError> {
        let config = self.load_latest_config(/*fallback_cwd*/ None).await?;
        let mut provider_ids = config.model_providers.keys().cloned().collect::<Vec<_>>();
        provider_ids.sort();
        let mut data = Vec::with_capacity(provider_ids.len());
        for provider_id in provider_ids {
            if provider_id == OPENAI_PROVIDER_ID {
                continue;
            }
            let Some(provider_info) = config.model_providers.get(&provider_id).cloned() else {
                continue;
            };
            let config_model_catalog = (provider_id == config.model_provider_id)
                .then(|| config.model_catalog.clone())
                .flatten();
            let model_count =
                create_model_provider(provider_info.clone(), Some(self.auth_manager.clone()))
                    .models_manager(config.codex_home.to_path_buf(), config_model_catalog)
                    .list_models(RefreshStrategy::Offline)
                    .await
                    .into_iter()
                    .filter(|preset| preset.show_in_picker)
                    .count() as u32;
            data.push(codex_app_server_protocol::ModelProvider {
                id: provider_id.clone(),
                name: provider_info.name.clone(),
                active: provider_id == config.model_provider_id,
                enabled_in_picker: !config.disabled_model_providers.contains(&provider_id),
                auth_status: model_provider_auth_status(
                    &provider_id,
                    &provider_info,
                    config.codex_home.as_path(),
                ),
                env_key: provider_info.env_key.clone(),
                base_url: provider_info.base_url.clone(),
                wire_api: provider_info.wire_api.to_string(),
                model_count,
            });
        }

        Ok(ModelProviderListResponse { data })
    }

    async fn read_agent_role_tool_selection_catalog(
        &self,
        params: AgentRoleToolSelectionCatalogReadParams,
    ) -> Result<AgentRoleToolSelectionCatalogReadResponse, JSONRPCErrorError> {
        let AgentRoleToolSelectionCatalogReadParams { thread_id } = params;
        let thread_id = ThreadId::from_string(&thread_id)
            .map_err(|err| invalid_request(format!("invalid thread id: {err}")))?;
        let thread = self
            .thread_manager
            .get_thread(thread_id)
            .await
            .map_err(|_| invalid_request(format!("thread not found: {thread_id}")))?;
        let catalog = thread.tool_selection_catalog().await.map_err(|err| {
            internal_error(format!("failed to read tool selection catalog: {err}"))
        })?;

        Ok(AgentRoleToolSelectionCatalogReadResponse {
            data: catalog
                .entries
                .into_iter()
                .map(|entry| AgentRoleToolSelectionCatalogEntry {
                    name: entry.name,
                    selected: entry.selected,
                    exposure: match entry.exposure {
                        CoreToolSelectionCatalogExposure::Direct => {
                            AgentRoleToolSelectionCatalogExposure::Direct
                        }
                        CoreToolSelectionCatalogExposure::Deferred => {
                            AgentRoleToolSelectionCatalogExposure::Deferred
                        }
                        CoreToolSelectionCatalogExposure::DirectModelOnly => {
                            AgentRoleToolSelectionCatalogExposure::DirectModelOnly
                        }
                        CoreToolSelectionCatalogExposure::Hidden => {
                            AgentRoleToolSelectionCatalogExposure::Hidden
                        }
                        CoreToolSelectionCatalogExposure::Hosted => {
                            AgentRoleToolSelectionCatalogExposure::Hosted
                        }
                    },
                })
                .collect(),
            unmatched_allowed_tools: catalog.unmatched_allowed_tools,
        })
    }

    async fn write_model_provider_config(
        &self,
        params: ModelProviderConfigWriteParams,
    ) -> Result<ConfigWriteResponse, JSONRPCErrorError> {
        let config = self.load_latest_config(/*fallback_cwd*/ None).await?;
        let Some(provider_info) = config.model_providers.get(&params.provider_id).cloned() else {
            return Err(invalid_request(format!(
                "model provider `{}` not found",
                params.provider_id
            )));
        };
        if params.set_active && params.enabled_in_picker == Some(false) {
            return Err(invalid_request(
                "model provider cannot be made active and hidden from /model in the same request",
            ));
        }

        let mut edits = Vec::new();
        if let Some(enabled_in_picker) = params.enabled_in_picker {
            if params.provider_id == OPENAI_PROVIDER_ID && !enabled_in_picker {
                return Err(invalid_request(
                    "OpenAI is a native model provider and cannot be disabled",
                ));
            }
            if !enabled_in_picker && params.provider_id == config.model_provider_id {
                return Err(invalid_request(
                    "active model provider cannot be disabled; select another provider first",
                ));
            }
            let mut disabled = config
                .disabled_model_providers
                .iter()
                .cloned()
                .collect::<Vec<_>>();
            if enabled_in_picker {
                disabled.retain(|provider_id| provider_id != &params.provider_id);
            } else if !disabled.contains(&params.provider_id) {
                disabled.push(params.provider_id.clone());
            }
            disabled.sort();
            edits.push(codex_app_server_protocol::ConfigEdit {
                key_path: "disabled_model_providers".to_string(),
                value: serde_json::json!(disabled),
                merge_strategy: MergeStrategy::Replace,
            });
        }

        if params.set_active {
            let default_model = self
                .default_model_for_provider(&config, &params.provider_id, provider_info)
                .await?;
            edits.push(codex_app_server_protocol::ConfigEdit {
                key_path: "model_provider".to_string(),
                value: serde_json::json!(&params.provider_id),
                merge_strategy: MergeStrategy::Replace,
            });
            edits.push(codex_app_server_protocol::ConfigEdit {
                key_path: "model".to_string(),
                value: serde_json::json!(default_model),
                merge_strategy: MergeStrategy::Replace,
            });
        }

        let response = self
            .config_manager
            .batch_write(ConfigBatchWriteParams {
                edits,
                file_path: None,
                expected_version: None,
                reload_user_config: true,
            })
            .await
            .map_err(|err| internal_error(err.to_string()))?;
        self.handle_model_provider_config_mutation().await;
        Ok(response)
    }

    async fn default_model_for_provider(
        &self,
        config: &Config,
        provider_id: &str,
        provider_info: ModelProviderInfo,
    ) -> Result<String, JSONRPCErrorError> {
        let config_model_catalog = (provider_id == config.model_provider_id)
            .then(|| config.model_catalog.clone())
            .flatten();
        let models = create_model_provider(provider_info, Some(self.auth_manager.clone()))
            .models_manager(config.codex_home.to_path_buf(), config_model_catalog)
            .list_models(RefreshStrategy::Offline)
            .await
            .into_iter()
            .filter(|preset| preset.show_in_picker)
            .collect::<Vec<_>>();
        models
            .iter()
            .find(|preset| preset.is_default)
            .or_else(|| models.first())
            .map(|preset| preset.model.clone())
            .ok_or_else(|| {
                invalid_request(format!(
                    "model provider `{provider_id}` does not expose a picker model to set as default"
                ))
            })
    }

    async fn write_model_provider_auth(
        &self,
        params: ModelProviderAuthWriteParams,
    ) -> Result<ModelProviderAuthWriteResponse, JSONRPCErrorError> {
        let config = self.load_latest_config(/*fallback_cwd*/ None).await?;
        let Some(provider) = config.model_providers.get(&params.provider_id) else {
            return Err(invalid_request(format!(
                "model provider `{}` not found",
                params.provider_id
            )));
        };
        if params.provider_id == OPENAI_PROVIDER_ID || provider.requires_openai_auth {
            return Err(invalid_request(
                "OpenAI is a native model provider; use account login to manage its auth",
            ));
        }
        let secret_name = model_provider_secret_name(&params.provider_id, provider)?;
        SecretsManager::new(config.codex_home.to_path_buf(), SecretsBackendKind::Local)
            .set(&SecretScope::Global, &secret_name, &params.api_key)
            .map_err(|err| internal_error(format!("failed to save model provider key: {err:#}")))?;
        self.handle_model_provider_config_mutation().await;
        Ok(ModelProviderAuthWriteResponse {
            provider_id: params.provider_id,
            managed_key_present: true,
        })
    }

    async fn remove_model_provider_auth(
        &self,
        params: ModelProviderAuthRemoveParams,
    ) -> Result<ModelProviderAuthRemoveResponse, JSONRPCErrorError> {
        let config = self.load_latest_config(/*fallback_cwd*/ None).await?;
        let Some(provider) = config.model_providers.get(&params.provider_id) else {
            return Err(invalid_request(format!(
                "model provider `{}` not found",
                params.provider_id
            )));
        };
        if params.provider_id == OPENAI_PROVIDER_ID || provider.requires_openai_auth {
            return Err(invalid_request(
                "OpenAI is a native model provider; use account login to manage its auth",
            ));
        }
        let secret_name = model_provider_secret_name(&params.provider_id, provider)?;
        SecretsManager::new(config.codex_home.to_path_buf(), SecretsBackendKind::Local)
            .delete(&SecretScope::Global, &secret_name)
            .map_err(|err| {
                internal_error(format!("failed to remove model provider key: {err:#}"))
            })?;
        self.handle_model_provider_config_mutation().await;
        Ok(ModelProviderAuthRemoveResponse {
            provider_id: params.provider_id,
            managed_key_present: false,
        })
    }

    async fn handle_model_provider_config_mutation(&self) {
        self.thread_manager.plugins_manager().clear_cache();
        self.thread_manager.skills_manager().clear_cache();
    }

    fn paginate_models(
        models: Vec<Model>,
        limit: Option<u32>,
        cursor: Option<String>,
    ) -> Result<ModelListResponse, JSONRPCErrorError> {
        let total = models.len();

        if total == 0 {
            return Ok(ModelListResponse {
                data: Vec::new(),
                next_cursor: None,
            });
        }

        let effective_limit = limit.unwrap_or(total as u32).max(1) as usize;
        let effective_limit = effective_limit.min(total);
        let start = match cursor {
            Some(cursor) => cursor
                .parse::<usize>()
                .map_err(|_| invalid_request(format!("invalid cursor: {cursor}")))?,
            None => 0,
        };

        if start > total {
            return Err(invalid_request(format!(
                "cursor {start} exceeds total models {total}"
            )));
        }

        let end = start.saturating_add(effective_limit).min(total);
        let items = models[start..end].to_vec();
        let next_cursor = if end < total {
            Some(end.to_string())
        } else {
            None
        };
        Ok(ModelListResponse {
            data: items,
            next_cursor,
        })
    }

    async fn list_collaboration_modes(
        thread_manager: Arc<ThreadManager>,
        params: CollaborationModeListParams,
    ) -> Result<CollaborationModeListResponse, JSONRPCErrorError> {
        let CollaborationModeListParams {} = params;
        let items = thread_manager
            .list_collaboration_modes()
            .into_iter()
            .map(Into::into)
            .collect();
        let response = CollaborationModeListResponse { data: items };
        Ok(response)
    }

    async fn experimental_feature_list_response(
        &self,
        params: ExperimentalFeatureListParams,
    ) -> Result<ExperimentalFeatureListResponse, JSONRPCErrorError> {
        let ExperimentalFeatureListParams {
            cursor,
            limit,
            thread_id,
        } = params;
        let config = match thread_id.as_deref() {
            Some(thread_id) => {
                let thread_id = ThreadId::from_string(thread_id)
                    .map_err(|err| invalid_request(format!("invalid thread id: {err}")))?;
                let thread = self
                    .thread_manager
                    .get_thread(thread_id)
                    .await
                    .map_err(|_| invalid_request(format!("thread not found: {thread_id}")))?;
                let thread_config = thread.config().await;
                self.config_manager
                    .load_latest_config_for_thread(thread_config.as_ref())
                    .await
                    .map_err(|err| internal_error(format!("failed to reload config: {err}")))?
            }
            None => self.load_latest_config(/*fallback_cwd*/ None).await?,
        };
        let auth = self.auth_manager.auth().await;
        let workspace_codex_plugins_enabled = self
            .workspace_codex_plugins_enabled(&config, auth.as_ref())
            .await;

        let data = FEATURES
            .iter()
            .map(|spec| {
                let (stage, display_name, description, announcement) = match spec.stage {
                    Stage::Experimental {
                        name,
                        menu_description,
                        announcement,
                    } => (
                        ApiExperimentalFeatureStage::Beta,
                        Some(name.to_string()),
                        Some(menu_description.to_string()),
                        Some(announcement.to_string()),
                    ),
                    Stage::UnderDevelopment => (
                        ApiExperimentalFeatureStage::UnderDevelopment,
                        None,
                        None,
                        None,
                    ),
                    Stage::Stable => (ApiExperimentalFeatureStage::Stable, None, None, None),
                    Stage::Deprecated => {
                        (ApiExperimentalFeatureStage::Deprecated, None, None, None)
                    }
                    Stage::Removed => (ApiExperimentalFeatureStage::Removed, None, None, None),
                };

                ApiExperimentalFeature {
                    name: spec.key.to_string(),
                    stage,
                    display_name,
                    description,
                    announcement,
                    enabled: config.features.enabled(spec.id)
                        && (workspace_codex_plugins_enabled
                            || !matches!(spec.id, Feature::Apps | Feature::Plugins)),
                    default_enabled: spec.default_enabled,
                }
            })
            .collect::<Vec<_>>();

        let total = data.len();
        if total == 0 {
            return Ok(ExperimentalFeatureListResponse {
                data: Vec::new(),
                next_cursor: None,
            });
        }

        // Clamp to 1 so limit=0 cannot return a non-advancing page.
        let effective_limit = limit.unwrap_or(total as u32).max(1) as usize;
        let effective_limit = effective_limit.min(total);
        let start = match cursor {
            Some(cursor) => match cursor.parse::<usize>() {
                Ok(idx) => idx,
                Err(_) => return Err(invalid_request(format!("invalid cursor: {cursor}"))),
            },
            None => 0,
        };

        if start > total {
            return Err(invalid_request(format!(
                "cursor {start} exceeds total feature flags {total}"
            )));
        }

        let end = start.saturating_add(effective_limit).min(total);
        let data = data[start..end].to_vec();
        let next_cursor = if end < total {
            Some(end.to_string())
        } else {
            None
        };

        Ok(ExperimentalFeatureListResponse { data, next_cursor })
    }

    async fn permission_profile_list_response(
        &self,
        params: PermissionProfileListParams,
    ) -> Result<PermissionProfileListResponse, JSONRPCErrorError> {
        let PermissionProfileListParams { cursor, limit, cwd } = params;
        let config_layer_stack = match cwd {
            Some(cwd) => {
                let cwd = PathBuf::from(cwd);
                let (_, config_layer_stack) = self
                    .resolve_cwd_config(&cwd)
                    .await
                    .map_err(|err| internal_error(format!("failed to reload config: {err}")))?;
                config_layer_stack
            }
            None => self
                .config_manager
                .load_config_layers(/*cwd*/ None)
                .await
                .map_err(|err| internal_error(format!("failed to reload config: {err}")))?,
        };
        let effective_config: ConfigToml = config_layer_stack
            .effective_config()
            .try_into()
            .map_err(|err| internal_error(format!("failed to read effective config: {err}")))?;
        let mut profiles = vec![
            PermissionProfileSummary {
                id: BUILT_IN_PERMISSION_PROFILE_READ_ONLY.to_string(),
                description: None,
            },
            PermissionProfileSummary {
                id: BUILT_IN_PERMISSION_PROFILE_WORKSPACE.to_string(),
                description: None,
            },
            PermissionProfileSummary {
                id: BUILT_IN_PERMISSION_PROFILE_DANGER_FULL_ACCESS.to_string(),
                description: None,
            },
        ];
        let mut configured_profiles = effective_config
            .permissions
            .into_iter()
            .flat_map(|permissions| permissions.entries)
            .map(|(id, profile)| PermissionProfileSummary {
                id,
                description: profile.description,
            })
            .collect::<Vec<_>>();
        configured_profiles.sort_by(|left, right| left.id.cmp(&right.id));
        profiles.extend(configured_profiles);
        let total = profiles.len();
        let effective_limit = limit.unwrap_or(total as u32).max(1) as usize;
        let effective_limit = effective_limit.min(total);
        let start = match cursor {
            Some(cursor) => cursor
                .parse::<usize>()
                .map_err(|_| invalid_request(format!("invalid cursor: {cursor}")))?,
            None => 0,
        };

        if start > total {
            return Err(invalid_request(format!(
                "cursor {start} exceeds total permission profiles {total}"
            )));
        }

        let end = start.saturating_add(effective_limit).min(total);
        let data = profiles[start..end].to_vec();
        let next_cursor = (end < total).then_some(end.to_string());

        Ok(PermissionProfileListResponse { data, next_cursor })
    }

    async fn mock_experimental_method_inner(
        &self,
        params: MockExperimentalMethodParams,
    ) -> Result<MockExperimentalMethodResponse, JSONRPCErrorError> {
        let MockExperimentalMethodParams { value } = params;
        let response = MockExperimentalMethodResponse { echoed: value };
        Ok(response)
    }

    async fn skills_list_response(
        &self,
        params: SkillsListParams,
    ) -> Result<SkillsListResponse, JSONRPCErrorError> {
        let SkillsListParams { cwds, force_reload } = params;
        let cwds = if cwds.is_empty() {
            vec![self.config.cwd.to_path_buf()]
        } else {
            cwds
        };

        let config = self.load_latest_config(/*fallback_cwd*/ None).await?;
        let auth = self.auth_manager.auth().await;
        let workspace_codex_plugins_enabled = self
            .workspace_codex_plugins_enabled(&config, auth.as_ref())
            .await;
        let skills_manager = self.thread_manager.skills_manager();
        let plugins_manager = self.thread_manager.plugins_manager();
        let fs = self
            .thread_manager
            .environment_manager()
            .default_environment()
            .map(|environment| environment.get_filesystem());
        let mut data = futures::stream::iter(cwds.into_iter().enumerate())
            .map(|(index, cwd)| {
                let config = &config;
                let fs = fs.clone();
                let plugins_manager = &plugins_manager;
                let skills_manager = &skills_manager;
                async move {
                    let (cwd_abs, config_layer_stack) = match self.resolve_cwd_config(&cwd).await {
                        Ok(resolved) => resolved,
                        Err(message) => {
                            let error_path = cwd.clone();
                            return (
                                index,
                                codex_app_server_protocol::SkillsListEntry {
                                    cwd,
                                    skills: Vec::new(),
                                    errors: vec![codex_app_server_protocol::SkillErrorInfo {
                                        path: error_path,
                                        message,
                                    }],
                                },
                            );
                        }
                    };
                    let effective_skill_roots = if workspace_codex_plugins_enabled {
                        let plugins_input = config.plugins_config_input();
                        plugins_manager
                            .effective_skill_roots_for_layer_stack(
                                &config_layer_stack,
                                &plugins_input,
                            )
                            .await
                    } else {
                        Vec::new()
                    };
                    let skills_input = codex_core::skills::SkillsLoadInput::new(
                        cwd_abs.clone(),
                        effective_skill_roots,
                        config_layer_stack,
                        config.bundled_skills_enabled(),
                    );
                    let outcome = skills_manager
                        .skills_for_cwd(&skills_input, force_reload, fs)
                        .await;
                    let errors = errors_to_info(&outcome.errors);
                    let skills = skills_to_info(&outcome.skills, &outcome.disabled_paths);
                    (
                        index,
                        codex_app_server_protocol::SkillsListEntry {
                            cwd,
                            skills,
                            errors,
                        },
                    )
                }
            })
            .buffer_unordered(SKILLS_LIST_CWD_CONCURRENCY)
            .collect::<Vec<_>>()
            .await;
        data.sort_unstable_by_key(|(index, _)| *index);
        let data = data.into_iter().map(|(_, entry)| entry).collect();
        Ok(SkillsListResponse { data })
    }

    async fn skills_extra_roots_set_response(
        &self,
        params: SkillsExtraRootsSetParams,
    ) -> Result<SkillsExtraRootsSetResponse, JSONRPCErrorError> {
        let SkillsExtraRootsSetParams { extra_roots } = params;
        self.skills_watcher
            .register_runtime_extra_roots(&extra_roots);
        self.thread_manager
            .skills_manager()
            .set_extra_roots(extra_roots);
        self.outgoing
            .send_server_notification(ServerNotification::SkillsChanged(
                codex_app_server_protocol::SkillsChangedNotification {},
            ))
            .await;
        Ok(SkillsExtraRootsSetResponse {})
    }

    /// Handle `hooks/list` by resolving hooks for each requested cwd.
    async fn hooks_list_response(
        &self,
        params: HooksListParams,
    ) -> Result<HooksListResponse, JSONRPCErrorError> {
        let HooksListParams { cwds } = params;
        let cwds = if cwds.is_empty() {
            vec![self.config.cwd.to_path_buf()]
        } else {
            cwds
        };

        let auth = self.auth_manager.auth().await;
        let plugins_manager = self.thread_manager.plugins_manager();
        let mut data = Vec::new();
        for cwd in cwds {
            let config = match self
                .config_manager
                .load_for_cwd(
                    /*request_overrides*/ None,
                    ConfigOverrides::default(),
                    Some(cwd.clone()),
                )
                .await
            {
                Ok(config) => config,
                Err(err) => {
                    let error_path = cwd.clone();
                    data.push(codex_app_server_protocol::HooksListEntry {
                        cwd,
                        hooks: Vec::new(),
                        warnings: Vec::new(),
                        errors: vec![codex_app_server_protocol::HookErrorInfo {
                            path: error_path,
                            message: err.to_string(),
                        }],
                    });
                    continue;
                }
            };
            let workspace_codex_plugins_enabled = self
                .workspace_codex_plugins_enabled(&config, auth.as_ref())
                .await;
            let plugins_enabled =
                config.features.enabled(Feature::Plugins) && workspace_codex_plugins_enabled;
            let plugin_hooks = if plugins_enabled {
                let plugins_input = config.plugins_config_input();
                let plugin_outcome = plugins_manager.plugins_for_config(&plugins_input).await;
                codex_core_plugins::PluginHookLoadOutcome {
                    hook_sources: plugin_outcome.effective_plugin_hook_sources(),
                    hook_load_warnings: plugin_outcome.effective_plugin_hook_warnings(),
                }
            } else {
                codex_core_plugins::PluginHookLoadOutcome::default()
            };
            let hooks = codex_hooks::list_hooks(codex_hooks::HooksConfig {
                feature_enabled: config.features.enabled(Feature::CodexHooks),
                bypass_hook_trust: config.bypass_hook_trust,
                config_layer_stack: Some(config.config_layer_stack),
                plugin_hook_sources: plugin_hooks.hook_sources,
                plugin_hook_load_warnings: plugin_hooks.hook_load_warnings,
                ..Default::default()
            });
            data.push(codex_app_server_protocol::HooksListEntry {
                cwd,
                hooks: hooks_to_info(&hooks.hooks),
                warnings: hooks.warnings,
                errors: Vec::new(),
            });
        }
        Ok(HooksListResponse { data })
    }

    async fn skills_config_write_response_inner(
        &self,
        params: SkillsConfigWriteParams,
    ) -> Result<SkillsConfigWriteResponse, JSONRPCErrorError> {
        let SkillsConfigWriteParams {
            path,
            name,
            enabled,
        } = params;
        let edit = match (path, name) {
            (Some(path), None) => ConfigEdit::SetSkillConfig {
                path: path.into_path_buf(),
                enabled,
            },
            (None, Some(name)) if !name.trim().is_empty() => {
                ConfigEdit::SetSkillConfigByName { name, enabled }
            }
            _ => {
                return Err(invalid_params(
                    "skills/config/write requires exactly one of path or name",
                ));
            }
        };
        let edits = vec![edit];
        ConfigEditsBuilder::new(&self.config.codex_home)
            .with_edits(edits)
            .apply()
            .await
            .map(|()| {
                self.thread_manager.plugins_manager().clear_cache();
                self.thread_manager.skills_manager().clear_cache();
                SkillsConfigWriteResponse {
                    effective_enabled: enabled,
                }
            })
            .map_err(|err| internal_error(format!("failed to update skill settings: {err}")))
    }
}

fn has_authoritative_inactive_catalog(provider_info: &ModelProviderInfo) -> bool {
    provider_info.is_openai() || provider_info.is_amazon_bedrock() || provider_info.is_deepseek()
}

fn model_provider_auth_status(
    provider_id: &str,
    provider: &ModelProviderInfo,
    codex_home: &std::path::Path,
) -> ModelProviderAuthStatus {
    if provider.requires_openai_auth {
        return ModelProviderAuthStatus::OpenAiAuth;
    }
    if provider.auth.is_some() {
        return ModelProviderAuthStatus::Command;
    }
    if provider.aws.is_some() {
        return ModelProviderAuthStatus::Aws;
    }
    let env_key_present = provider
        .env_key
        .as_deref()
        .is_some_and(|env_key| std::env::var(env_key).is_ok_and(|value| !value.trim().is_empty()));
    if env_key_present {
        return ModelProviderAuthStatus::EnvKeyPresent;
    }

    let managed_key_present = model_provider_secret_name(provider_id, provider)
        .ok()
        .and_then(|secret_name| {
            SecretsManager::new(codex_home.to_path_buf(), SecretsBackendKind::Local)
                .get(&SecretScope::Global, &secret_name)
                .ok()
                .flatten()
        })
        .is_some_and(|value| !value.trim().is_empty());
    if managed_key_present {
        return ModelProviderAuthStatus::ManagedKeyPresent;
    }
    if provider.experimental_bearer_token.is_some() {
        return ModelProviderAuthStatus::InlineBearer;
    }

    if provider.env_key.is_some() {
        ModelProviderAuthStatus::EnvKeyMissing
    } else {
        ModelProviderAuthStatus::None
    }
}

fn model_provider_secret_name(
    provider_id: &str,
    provider: &ModelProviderInfo,
) -> Result<SecretName, JSONRPCErrorError> {
    if let Some(env_key) = provider.env_key.as_deref() {
        return SecretName::new(env_key)
            .map_err(|err| invalid_request(format!("invalid provider env key: {err}")));
    }

    let suffix = provider_id
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    SecretName::new(&format!("MODEL_PROVIDER_{suffix}_API_KEY"))
        .map_err(|err| invalid_request(format!("invalid model provider id: {err}")))
}
