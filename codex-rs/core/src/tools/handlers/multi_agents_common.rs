use crate::agent::AgentStatus;
use crate::agent::role::DEFAULT_ROLE_NAME;
use crate::agent::role::RoleApplication;
use crate::agent::role::apply_role_to_config;
use crate::agent::role::resolve_role_config;
use crate::agent::role_templates::LoadedRoleTemplates;
use crate::agent::role_templates::RoleTemplateSettings;
use crate::config::Config;
use crate::config::DEFAULT_MULTI_AGENT_V2_MIN_WAIT_TIMEOUT_MS;
use crate::config::MAX_MULTI_AGENT_V2_WAIT_TIMEOUT_MS;
use crate::function_tool::FunctionCallError;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolOutput;
use crate::tools::context::ToolPayload;
use codex_features::Feature;
use codex_models_manager::manager::RefreshStrategy;
use codex_protocol::AgentPath;
use codex_protocol::ThreadId;
use codex_protocol::error::CodexErr;
use codex_protocol::models::BaseInstructions;
use codex_protocol::models::PermissionProfile;
use codex_protocol::models::ResponseInputItem;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::openai_models::ReasoningEffortPreset;
use codex_protocol::protocol::CollabAgentRef;
use codex_protocol::protocol::CollabAgentStatusEntry;
use codex_protocol::protocol::Op;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use codex_protocol::user_input::UserInput;
use codex_utils_absolute_path::AbsolutePathBuf;
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;

/// Minimum wait timeout to prevent tight polling loops from burning CPU.
pub(crate) const MIN_WAIT_TIMEOUT_MS: i64 = DEFAULT_MULTI_AGENT_V2_MIN_WAIT_TIMEOUT_MS;
pub(crate) const DEFAULT_WAIT_TIMEOUT_MS: i64 = 600 * 1000;
pub(crate) const MAX_WAIT_TIMEOUT_MS: i64 = MAX_MULTI_AGENT_V2_WAIT_TIMEOUT_MS;

pub(crate) fn function_arguments(payload: ToolPayload) -> Result<String, FunctionCallError> {
    match payload {
        ToolPayload::Function { arguments } => Ok(arguments),
        _ => Err(FunctionCallError::RespondToModel(
            "collab handler received unsupported payload".to_string(),
        )),
    }
}

pub(crate) fn tool_output_json_text<T>(value: &T, tool_name: &str) -> String
where
    T: Serialize,
{
    serde_json::to_string(value).unwrap_or_else(|err| {
        JsonValue::String(format!("failed to serialize {tool_name} result: {err}")).to_string()
    })
}

pub(crate) fn tool_output_response_item<T>(
    call_id: &str,
    payload: &ToolPayload,
    value: &T,
    success: Option<bool>,
    tool_name: &str,
) -> ResponseInputItem
where
    T: Serialize,
{
    FunctionToolOutput::from_text(tool_output_json_text(value, tool_name), success)
        .to_response_item(call_id, payload)
}

pub(crate) fn tool_output_code_mode_result<T>(value: &T, tool_name: &str) -> JsonValue
where
    T: Serialize,
{
    serde_json::to_value(value).unwrap_or_else(|err| {
        JsonValue::String(format!("failed to serialize {tool_name} result: {err}"))
    })
}

pub(crate) fn build_wait_agent_statuses(
    statuses: &HashMap<ThreadId, AgentStatus>,
    receiver_agents: &[CollabAgentRef],
) -> Vec<CollabAgentStatusEntry> {
    if statuses.is_empty() {
        return Vec::new();
    }

    let mut entries = Vec::with_capacity(statuses.len());
    let mut seen = HashMap::with_capacity(receiver_agents.len());
    for receiver_agent in receiver_agents {
        seen.insert(receiver_agent.thread_id, ());
        if let Some(status) = statuses.get(&receiver_agent.thread_id) {
            entries.push(CollabAgentStatusEntry {
                thread_id: receiver_agent.thread_id,
                agent_nickname: receiver_agent.agent_nickname.clone(),
                agent_role: receiver_agent.agent_role.clone(),
                status: status.clone(),
            });
        }
    }

    let mut extras = statuses
        .iter()
        .filter(|(thread_id, _)| !seen.contains_key(thread_id))
        .map(|(thread_id, status)| CollabAgentStatusEntry {
            thread_id: *thread_id,
            agent_nickname: None,
            agent_role: None,
            status: status.clone(),
        })
        .collect::<Vec<_>>();
    extras.sort_by(|left, right| left.thread_id.to_string().cmp(&right.thread_id.to_string()));
    entries.extend(extras);
    entries
}

pub(crate) fn collab_spawn_error(err: CodexErr) -> FunctionCallError {
    match err {
        CodexErr::UnsupportedOperation(message) if message == "thread manager dropped" => {
            FunctionCallError::RespondToModel("collab manager unavailable".to_string())
        }
        CodexErr::UnsupportedOperation(message) => FunctionCallError::RespondToModel(message),
        err => FunctionCallError::RespondToModel(format!("collab spawn failed: {err}")),
    }
}

pub(crate) fn collab_agent_error(agent_id: ThreadId, err: CodexErr) -> FunctionCallError {
    match err {
        CodexErr::ThreadNotFound(id) => {
            FunctionCallError::RespondToModel(format!("agent with id {id} not found"))
        }
        CodexErr::InternalAgentDied => {
            FunctionCallError::RespondToModel(format!("agent with id {agent_id} is closed"))
        }
        CodexErr::UnsupportedOperation(_) => {
            FunctionCallError::RespondToModel("collab manager unavailable".to_string())
        }
        err => FunctionCallError::RespondToModel(format!("collab tool failed: {err}")),
    }
}

#[derive(Default)]
pub(crate) struct ThreadSpawnSourceOptions<'a> {
    pub(crate) agent_role: Option<&'a str>,
    pub(crate) agent_persona: Option<String>,
    pub(crate) task_name: Option<String>,
}

pub(crate) fn thread_spawn_source(
    parent_thread_id: ThreadId,
    parent_session_source: &SessionSource,
    depth: i32,
    options: ThreadSpawnSourceOptions<'_>,
) -> Result<SessionSource, FunctionCallError> {
    let agent_path = options
        .task_name
        .as_deref()
        .map(|task_name| {
            parent_session_source
                .get_agent_path()
                .unwrap_or_else(AgentPath::root)
                .join(task_name)
                .map_err(FunctionCallError::RespondToModel)
        })
        .transpose()?;
    Ok(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
        parent_thread_id,
        depth,
        agent_path,
        agent_nickname: None,
        agent_role: options.agent_role.map(str::to_string),
        agent_persona: options.agent_persona,
    }))
}

pub(crate) fn parse_collab_input(
    message: Option<String>,
    items: Option<Vec<UserInput>>,
) -> Result<Op, FunctionCallError> {
    match (message, items) {
        (Some(_), Some(_)) => Err(FunctionCallError::RespondToModel(
            "Provide either message or items, but not both".to_string(),
        )),
        (None, None) => Err(FunctionCallError::RespondToModel(
            "Provide one of: message or items".to_string(),
        )),
        (Some(message), None) => {
            if message.trim().is_empty() {
                return Err(FunctionCallError::RespondToModel(
                    "Empty message can't be sent to an agent".to_string(),
                ));
            }
            Ok(vec![UserInput::Text {
                text: message,
                text_elements: Vec::new(),
            }]
            .into())
        }
        (None, Some(items)) => {
            if items.is_empty() {
                return Err(FunctionCallError::RespondToModel(
                    "Items can't be empty".to_string(),
                ));
            }
            Ok(items.into())
        }
    }
}

/// Builds the base config snapshot for a newly spawned sub-agent.
///
/// The returned config starts from the parent's effective config and then refreshes the
/// runtime-owned fields carried on `turn`, including model selection, reasoning settings,
/// approval policy, sandbox, and cwd. Role-specific overrides are layered after this step;
/// skipping this helper and cloning stale config state directly can send the child agent out with
/// the wrong provider or runtime policy.
pub(crate) fn build_agent_spawn_config(
    base_instructions: &BaseInstructions,
    turn: &TurnContext,
) -> Result<Config, FunctionCallError> {
    let mut config = build_agent_shared_config(turn)?;
    config.base_instructions = Some(base_instructions.text.clone());
    Ok(config)
}

pub(crate) fn build_agent_spawn_config_for_cwd<'a>(
    base_instructions: &'a BaseInstructions,
    turn: &'a TurnContext,
    cwd: SpawnAgentCwd,
) -> Pin<Box<dyn Future<Output = Result<Config, FunctionCallError>> + Send + 'a>> {
    Box::pin(async move {
        match cwd {
            SpawnAgentCwd::Inherited => build_agent_spawn_config(base_instructions, turn),
            SpawnAgentCwd::Explicit(cwd) => {
                let refreshed_config =
                    Config::load_for_cwd(turn.config.codex_home.to_path_buf(), &cwd)
                        .await
                        .map_err(|err| {
                            FunctionCallError::RespondToModel(format!(
                                "cwd config rebuild failed for `{}`: {err}",
                                cwd.display()
                            ))
                        })?;
                let mut config = turn
                    .config
                    .rebuild_preserving_session_layers_for_cwd(&refreshed_config, cwd.clone())
                    .await
                    .map_err(|err| {
                        FunctionCallError::RespondToModel(format!(
                            "cwd config rebuild failed for `{}`: {err}",
                            cwd.display()
                        ))
                    })?;
                apply_spawn_agent_runtime_state(&mut config, turn);
                apply_spawn_agent_runtime_overrides_for_cwd(
                    &mut config,
                    turn,
                    SpawnAgentCwd::Explicit(cwd),
                )?;
                config.base_instructions = Some(base_instructions.text.clone());
                Ok(config)
            }
        }
    })
}

pub(crate) fn build_agent_resume_config(
    turn: &TurnContext,
    child_depth: i32,
) -> Result<Config, FunctionCallError> {
    let mut config = build_agent_shared_config(turn)?;
    apply_spawn_agent_overrides(&mut config, child_depth);
    // For resume, keep base instructions sourced from rollout/session metadata.
    config.base_instructions = None;
    Ok(config)
}

fn build_agent_shared_config(turn: &TurnContext) -> Result<Config, FunctionCallError> {
    let base_config = turn.config.clone();
    let mut config = (*base_config).clone();
    apply_spawn_agent_runtime_state(&mut config, turn);
    apply_spawn_agent_runtime_overrides(&mut config, turn)?;

    Ok(config)
}

fn apply_spawn_agent_runtime_state(config: &mut Config, turn: &TurnContext) {
    config.model = Some(turn.model_info.slug.clone());
    config.model_provider = turn.provider.info().clone();
    config.model_reasoning_effort = turn
        .reasoning_effort
        .or(turn.model_info.default_reasoning_level);
    config.model_reasoning_summary = Some(turn.reasoning_summary);
    config.developer_instructions = turn.developer_instructions.clone();
    config.compact_prompt = turn.compact_prompt.clone();
}

pub(crate) fn reject_full_fork_spawn_overrides(
    agent_type: Option<&str>,
    agent_persona: Option<&str>,
    model: Option<&str>,
    reasoning_effort: Option<ReasoningEffort>,
) -> Result<(), FunctionCallError> {
    if agent_type.is_some()
        || agent_persona.is_some()
        || model.is_some()
        || reasoning_effort.is_some()
    {
        return Err(FunctionCallError::RespondToModel(
            "Full-history forked agents inherit the parent agent type, persona, model, and reasoning effort; omit agent_type, agent_persona, model, and reasoning_effort, or spawn without a full-history fork.".to_string(),
        ));
    }
    Ok(())
}

pub(crate) fn resolve_spawn_agent_role_template(
    config: &Config,
    role_name: Option<&str>,
    agent_persona: Option<&str>,
) -> Result<Option<RoleTemplateSettings>, FunctionCallError> {
    let role_name = role_name.unwrap_or(crate::agent::role::DEFAULT_ROLE_NAME);
    LoadedRoleTemplates::load_for_config(config)
        .resolve_settings(role_name, agent_persona)
        .map_err(FunctionCallError::RespondToModel)
}

pub(crate) async fn apply_spawn_agent_native_role_or_template_only(
    config: &mut Config,
    role_name: Option<&str>,
) -> Result<RoleApplication, FunctionCallError> {
    let resolved_role_name = role_name.unwrap_or(DEFAULT_ROLE_NAME);
    if resolve_role_config(config, resolved_role_name).is_some() {
        return apply_role_to_config(config, role_name)
            .await
            .map_err(FunctionCallError::RespondToModel);
    }

    let templates = LoadedRoleTemplates::load_for_config(config);
    match templates.resolve_settings(resolved_role_name, /*agent_persona*/ None) {
        Ok(Some(_)) => Ok(RoleApplication::default()),
        Ok(None) => Err(FunctionCallError::RespondToModel(format!(
            "unknown agent_type '{resolved_role_name}'"
        ))),
        Err(err) => Err(FunctionCallError::RespondToModel(err)),
    }
}

pub(crate) fn apply_spawn_agent_role_template(
    config: &mut Config,
    turn: &TurnContext,
    settings: &RoleTemplateSettings,
    native_role_application: RoleApplication,
) -> Result<(), FunctionCallError> {
    if let Some(base_instructions) = &settings.base_instructions {
        config.base_instructions = Some(base_instructions.clone());
    }
    config.developer_instructions = Some(match config.developer_instructions.take() {
        Some(existing) if !existing.trim().is_empty() => {
            format!("{}\n\n{}", existing.trim_end(), settings.instructions)
        }
        _ => settings.instructions.clone(),
    });

    if !native_role_application.owns_model
        && config.model.as_deref() == Some(turn.model_info.slug.as_str())
        && let Some(model) = &settings.model
    {
        config.model = Some(model.clone());
    }
    let inherited_reasoning_effort = turn
        .reasoning_effort
        .or(turn.model_info.default_reasoning_level);
    if !native_role_application.owns_reasoning_effort
        && config.model_reasoning_effort == inherited_reasoning_effort
        && let Some(reasoning_effort) = settings.reasoning_effort
    {
        config.model_reasoning_effort = Some(reasoning_effort);
    }
    if settings.read_only {
        config
            .permissions
            .set_permission_profile(PermissionProfile::read_only())
            .map_err(|err| {
                FunctionCallError::RespondToModel(format!("permission_profile is invalid: {err}"))
            })?;
    }
    if settings.allow_list.is_some() || settings.deny_list.is_some() {
        let inherited = config.agent_tool_policy.take().map(Box::new);
        config.agent_tool_policy = Some(codex_tools::AgentToolPolicyConfig {
            allow_list: settings.allow_list.clone(),
            deny_list: settings.deny_list.clone(),
            inherited,
        });
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SpawnAgentCwd {
    Inherited,
    Explicit(AbsolutePathBuf),
}

impl SpawnAgentCwd {
    pub(crate) fn is_explicit(&self) -> bool {
        matches!(self, Self::Explicit(_))
    }
}

pub(crate) fn resolve_spawn_agent_cwd(
    parent_cwd: &AbsolutePathBuf,
    cwd: Option<&Path>,
) -> Result<SpawnAgentCwd, FunctionCallError> {
    let Some(cwd) = cwd else {
        return Ok(SpawnAgentCwd::Inherited);
    };
    if cwd.as_os_str().is_empty() {
        return Err(FunctionCallError::RespondToModel(
            "cwd is invalid: empty path".to_string(),
        ));
    }
    let resolved = if cwd.is_absolute() {
        AbsolutePathBuf::from_absolute_path(cwd)
    } else {
        Ok(parent_cwd.join(cwd))
    }
    .map_err(|err| {
        FunctionCallError::RespondToModel(format!("cwd is invalid: {}: {err}", cwd.display()))
    })?;
    match std::fs::metadata(resolved.as_path()) {
        Ok(metadata) if metadata.is_dir() => Ok(SpawnAgentCwd::Explicit(resolved)),
        Ok(_) => Err(FunctionCallError::RespondToModel(format!(
            "cwd is not a directory: {}",
            resolved.display()
        ))),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            Err(FunctionCallError::RespondToModel(format!(
                "cwd does not exist: {}",
                resolved.display()
            )))
        }
        Err(err) => Err(FunctionCallError::RespondToModel(format!(
            "cwd could not be accessed: {}: {err}",
            resolved.display()
        ))),
    }
}

/// Copies runtime-only turn state onto a child config before it is handed to `AgentControl`.
///
/// These values are chosen by the live turn rather than persisted config, so leaving them stale
/// can make a child agent disagree with its parent about approval policy, cwd, or sandboxing.
pub(crate) fn apply_spawn_agent_runtime_overrides(
    config: &mut Config,
    turn: &TurnContext,
) -> Result<(), FunctionCallError> {
    config
        .permissions
        .approval_policy
        .set(turn.approval_policy.value())
        .map_err(|err| {
            FunctionCallError::RespondToModel(format!("approval_policy is invalid: {err}"))
        })?;
    config.permissions.shell_environment_policy = turn.shell_environment_policy.clone();
    config.codex_linux_sandbox_exe = turn.codex_linux_sandbox_exe.clone();
    config.cwd = turn.cwd.clone();
    config
        .permissions
        .set_permission_profile(turn.permission_profile())
        .map_err(|err| {
            FunctionCallError::RespondToModel(format!("permission_profile is invalid: {err}"))
        })?;
    Ok(())
}

pub(crate) fn apply_spawn_agent_runtime_overrides_for_cwd(
    config: &mut Config,
    turn: &TurnContext,
    cwd: SpawnAgentCwd,
) -> Result<(), FunctionCallError> {
    match cwd {
        SpawnAgentCwd::Inherited => apply_spawn_agent_runtime_overrides(config, turn),
        SpawnAgentCwd::Explicit(cwd) => {
            config
                .permissions
                .approval_policy
                .set(turn.approval_policy.value())
                .map_err(|err| {
                    FunctionCallError::RespondToModel(format!("approval_policy is invalid: {err}"))
                })?;
            config.permissions.shell_environment_policy = turn.shell_environment_policy.clone();
            config.codex_linux_sandbox_exe = turn.codex_linux_sandbox_exe.clone();
            let permission_profile = rebase_spawn_agent_permission_profile(turn, cwd.as_path())?;
            config.cwd = cwd;
            config
                .permissions
                .set_permission_profile(permission_profile)
                .map_err(|err| {
                    FunctionCallError::RespondToModel(format!(
                        "permission_profile is invalid: {err}"
                    ))
                })?;
            Ok(())
        }
    }
}

fn rebase_spawn_agent_permission_profile(
    turn: &TurnContext,
    cwd: &Path,
) -> Result<PermissionProfile, FunctionCallError> {
    let sandbox_policy = turn
        .permission_profile()
        .to_legacy_sandbox_policy(turn.cwd.as_path())
        .map_err(|err| {
            FunctionCallError::RespondToModel(format!(
                "permission_profile cannot be applied to explicit cwd: {err}"
            ))
        })?;
    Ok(PermissionProfile::from_legacy_sandbox_policy_for_cwd(
        &sandbox_policy,
        cwd,
    ))
}

pub(crate) fn apply_spawn_agent_overrides(config: &mut Config, child_depth: i32) {
    if child_depth >= config.agent_max_depth && !config.features.enabled(Feature::MultiAgentV2) {
        let _ = config.features.disable(Feature::SpawnCsv);
        let _ = config.features.disable(Feature::Collab);
    }
}

pub(crate) async fn apply_requested_spawn_agent_model_overrides(
    session: &Session,
    turn: &TurnContext,
    config: &mut Config,
    requested_model: Option<&str>,
    requested_reasoning_effort: Option<ReasoningEffort>,
) -> Result<(), FunctionCallError> {
    if requested_model.is_none() && requested_reasoning_effort.is_none() {
        return Ok(());
    }

    if let Some(requested_model) = requested_model {
        let available_models = session
            .services
            .models_manager
            .list_models(RefreshStrategy::Offline)
            .await;
        let selected_model_name = find_spawn_agent_model_name(&available_models, requested_model)?;
        let selected_model_info = session
            .services
            .models_manager
            .get_model_info(&selected_model_name, &config.to_models_manager_config())
            .await;

        config.model = Some(selected_model_name.clone());
        if let Some(reasoning_effort) = requested_reasoning_effort {
            validate_spawn_agent_reasoning_effort(
                &selected_model_name,
                &selected_model_info.supported_reasoning_levels,
                reasoning_effort,
            )?;
            config.model_reasoning_effort = Some(reasoning_effort);
        } else {
            config.model_reasoning_effort = selected_model_info.default_reasoning_level;
        }

        return Ok(());
    }

    if let Some(reasoning_effort) = requested_reasoning_effort {
        validate_spawn_agent_reasoning_effort(
            &turn.model_info.slug,
            &turn.model_info.supported_reasoning_levels,
            reasoning_effort,
        )?;
        config.model_reasoning_effort = Some(reasoning_effort);
    }

    Ok(())
}

fn find_spawn_agent_model_name(
    available_models: &[codex_protocol::openai_models::ModelPreset],
    requested_model: &str,
) -> Result<String, FunctionCallError> {
    available_models
        .iter()
        .find(|model| model.model == requested_model)
        .map(|model| model.model.clone())
        .ok_or_else(|| {
            let available = available_models
                .iter()
                .map(|model| model.model.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            FunctionCallError::RespondToModel(format!(
                "Unknown model `{requested_model}` for spawn_agent. Available models: {available}"
            ))
        })
}

fn validate_spawn_agent_reasoning_effort(
    model: &str,
    supported_reasoning_levels: &[ReasoningEffortPreset],
    requested_reasoning_effort: ReasoningEffort,
) -> Result<(), FunctionCallError> {
    if supported_reasoning_levels
        .iter()
        .any(|preset| preset.effort == requested_reasoning_effort)
    {
        return Ok(());
    }

    let supported = supported_reasoning_levels
        .iter()
        .map(|preset| preset.effort.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Err(FunctionCallError::RespondToModel(format!(
        "Reasoning effort `{requested_reasoning_effort}` is not supported for model `{model}`. Supported reasoning efforts: {supported}"
    )))
}
