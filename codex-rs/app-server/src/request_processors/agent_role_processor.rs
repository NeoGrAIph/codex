use std::path::PathBuf;
use std::sync::Arc;

use crate::config_manager::ConfigManager;
use crate::error_code::internal_error;
use crate::error_code::invalid_request;
use codex_app_server_protocol::AgentRoleActionPolicySetParams;
use codex_app_server_protocol::AgentRoleActionPolicySetResponse;
use codex_app_server_protocol::AgentRoleToolSelectionSetParams;
use codex_app_server_protocol::AgentRoleToolSelectionSetResponse;
use codex_app_server_protocol::ClientResponsePayload;
use codex_app_server_protocol::JSONRPCErrorError;
use codex_core::ThreadManager;
use codex_core::agent_role_templates::AgentRoleTemplateConfigFileOrigin;
use codex_core::agent_role_templates::AgentRoleTemplateCreateError;
use codex_core::agent_role_templates::AgentRoleTemplateSource;
use codex_core::agent_role_templates::list_agent_role_templates;
use codex_core::agent_role_templates::update_user_agent_role_template_action_policy;
use codex_core::agent_role_templates::update_user_agent_role_template_tool_selection;
use codex_utils_absolute_path::AbsolutePathBuf;

#[derive(Clone)]
pub(crate) struct AgentRoleRequestProcessor {
    thread_manager: Arc<ThreadManager>,
    config_manager: ConfigManager,
}

impl AgentRoleRequestProcessor {
    pub(crate) fn new(thread_manager: Arc<ThreadManager>, config_manager: ConfigManager) -> Self {
        Self {
            thread_manager,
            config_manager,
        }
    }

    pub(crate) async fn tool_selection_set(
        &self,
        params: AgentRoleToolSelectionSetParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.set_tool_selection(params)
            .await
            .map(|response| Some(response.into()))
    }

    pub(crate) async fn action_policy_set(
        &self,
        params: AgentRoleActionPolicySetParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.set_action_policy(params)
            .await
            .map(|response| Some(response.into()))
    }

    async fn set_tool_selection(
        &self,
        params: AgentRoleToolSelectionSetParams,
    ) -> Result<AgentRoleToolSelectionSetResponse, JSONRPCErrorError> {
        let AgentRoleToolSelectionSetParams {
            role_name,
            allowed_tools,
            denied_tools,
        } = params;
        let config = self
            .config_manager
            .load_latest_config(/*fallback_cwd*/ None)
            .await
            .map_err(|err| internal_error(format!("failed to load current config: {err}")))?;
        let entry = list_agent_role_templates(&config)
            .into_iter()
            .find(|entry| entry.name == role_name)
            .ok_or_else(|| invalid_request(format!("agent role `{role_name}` not found")))?;

        if entry.source != AgentRoleTemplateSource::User
            || entry.config_file_origin != Some(AgentRoleTemplateConfigFileOrigin::UserAgentsDir)
        {
            return Err(invalid_request(format!(
                "agent role `{role_name}` is not a discovered user role under $CODEX_HOME/agents"
            )));
        }
        let path = entry.config_file.ok_or_else(|| {
            invalid_request(format!(
                "agent role `{role_name}` has no writable user role file"
            ))
        })?;

        let updated = update_user_agent_role_template_tool_selection(
            &config,
            &role_name,
            &path,
            allowed_tools.as_deref(),
            denied_tools.as_deref(),
        )
        .map_err(map_role_template_error)?;
        self.reload_loaded_threads().await?;
        Ok(AgentRoleToolSelectionSetResponse {
            role_name: updated.name,
            file_path: absolute_path(updated.path)?,
            allowed_tools,
            denied_tools,
        })
    }

    async fn set_action_policy(
        &self,
        params: AgentRoleActionPolicySetParams,
    ) -> Result<AgentRoleActionPolicySetResponse, JSONRPCErrorError> {
        let AgentRoleActionPolicySetParams {
            role_name,
            allowed_actions,
            denied_actions,
        } = params;
        let config = self
            .config_manager
            .load_latest_config(/*fallback_cwd*/ None)
            .await
            .map_err(|err| internal_error(format!("failed to load current config: {err}")))?;
        let entry = list_agent_role_templates(&config)
            .into_iter()
            .find(|entry| entry.name == role_name)
            .ok_or_else(|| invalid_request(format!("agent role `{role_name}` not found")))?;

        if entry.source != AgentRoleTemplateSource::User
            || entry.config_file_origin != Some(AgentRoleTemplateConfigFileOrigin::UserAgentsDir)
        {
            return Err(invalid_request(format!(
                "agent role `{role_name}` is not a discovered user role under $CODEX_HOME/agents"
            )));
        }
        let path = entry.config_file.ok_or_else(|| {
            invalid_request(format!(
                "agent role `{role_name}` has no writable user role file"
            ))
        })?;

        let updated = update_user_agent_role_template_action_policy(
            &config,
            &role_name,
            &path,
            allowed_actions.as_deref(),
            denied_actions.as_deref(),
        )
        .map_err(map_role_template_error)?;
        self.reload_loaded_threads().await?;
        Ok(AgentRoleActionPolicySetResponse {
            role_name: updated.name,
            file_path: absolute_path(updated.path)?,
            allowed_actions,
            denied_actions,
        })
    }

    async fn reload_loaded_threads(&self) -> Result<(), JSONRPCErrorError> {
        let next_config = self
            .config_manager
            .load_latest_config(/*fallback_cwd*/ None)
            .await
            .map_err(|err| {
                internal_error(format!(
                    "failed to reload config after agent role update: {err}"
                ))
            })?;
        let thread_ids = self.thread_manager.list_thread_ids().await;
        for thread_id in thread_ids {
            let Ok(thread) = self.thread_manager.get_thread(thread_id).await else {
                continue;
            };
            thread.refresh_runtime_config(next_config.clone()).await;
        }
        Ok(())
    }
}

fn absolute_path(path: PathBuf) -> Result<AbsolutePathBuf, JSONRPCErrorError> {
    AbsolutePathBuf::from_absolute_path(path).map_err(|err| {
        internal_error(format!(
            "updated agent role path is not absolute after native config resolution: {err}"
        ))
    })
}

fn map_role_template_error(err: AgentRoleTemplateCreateError) -> JSONRPCErrorError {
    match err {
        AgentRoleTemplateCreateError::EmptyName
        | AgentRoleTemplateCreateError::InvalidName
        | AgentRoleTemplateCreateError::AlreadyExists(_)
        | AgentRoleTemplateCreateError::FileExists(_)
        | AgentRoleTemplateCreateError::FileMissing(_)
        | AgentRoleTemplateCreateError::InvalidTemplate(_) => invalid_request(err.to_string()),
        AgentRoleTemplateCreateError::Write { .. } => {
            internal_error(format!("failed to update agent role template: {err}"))
        }
    }
}
