use crate::agent::AgentStatus;
use crate::agent::exceeds_thread_spawn_depth_limit;
use crate::codex::Session;
use crate::codex::TurnContext;
use crate::config::Config;
use crate::error::CodexErr;
use crate::features::Feature;
use crate::function_tool::FunctionCallError;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolOutput;
use crate::tools::context::ToolPayload;
use crate::tools::handlers::parse_arguments;
use crate::tools::registry::ToolHandler;
use crate::tools::registry::ToolKind;
use async_trait::async_trait;
use codex_protocol::ThreadId;
use codex_protocol::models::BaseInstructions;
use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::protocol::CollabAgentInteractionBeginEvent;
use codex_protocol::protocol::CollabAgentInteractionEndEvent;
use codex_protocol::protocol::CollabAgentRef;
use codex_protocol::protocol::CollabAgentSpawnBeginEvent;
use codex_protocol::protocol::CollabAgentSpawnEndEvent;
use codex_protocol::protocol::CollabAgentStatusEntry;
use codex_protocol::protocol::CollabCloseBeginEvent;
use codex_protocol::protocol::CollabCloseEndEvent;
use codex_protocol::protocol::CollabResumeBeginEvent;
use codex_protocol::protocol::CollabResumeEndEvent;
use codex_protocol::protocol::CollabWaitingBeginEvent;
use codex_protocol::protocol::CollabWaitingEndEvent;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use codex_protocol::user_input::UserInput;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;

pub struct MultiAgentHandler;

/// Minimum wait timeout to prevent tight polling loops from burning CPU.
pub(crate) const MIN_WAIT_TIMEOUT_MS: i64 = 10_000;
pub(crate) const DEFAULT_WAIT_TIMEOUT_MS: i64 = 30_000;
pub(crate) const MAX_WAIT_TIMEOUT_MS: i64 = 3600 * 1000;

#[derive(Debug, Deserialize)]
struct CloseAgentArgs {
    id: String,
}

#[async_trait]
impl ToolHandler for MultiAgentHandler {
    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    fn matches_kind(&self, payload: &ToolPayload) -> bool {
        matches!(payload, ToolPayload::Function { .. })
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<ToolOutput, FunctionCallError> {
        let ToolInvocation {
            session,
            turn,
            tool_name,
            payload,
            call_id,
            ..
        } = invocation;

        let arguments = match payload {
            ToolPayload::Function { arguments } => arguments,
            _ => {
                return Err(FunctionCallError::RespondToModel(
                    "collab handler received unsupported payload".to_string(),
                ));
            }
        };

        match tool_name.as_str() {
            "spawn_agent" => spawn::handle(session, turn, call_id, arguments).await,
            "send_input" => send_input::handle(session, turn, call_id, arguments).await,
            "set_thread_note" => set_thread_note::handle(session, turn, call_id, arguments).await,
            "resume_agent" => resume_agent::handle(session, turn, call_id, arguments).await,
            "wait" => wait::handle(session, turn, call_id, arguments).await,
            "close_agent" => close_agent::handle(session, turn, call_id, arguments).await,
            other => Err(FunctionCallError::RespondToModel(format!(
                "unsupported collab tool {other}"
            ))),
        }
    }
}

mod spawn {
    use super::*;
    use crate::agent::role::DEFAULT_ROLE_NAME;
    use crate::agent::role::apply_role_to_config;
    use crate::agent::role::is_declared_role;
    use crate::agent::role_templates::DEFAULT_AGENT_NICKNAME;
    use crate::agent::role_templates::LoadedRoleTemplates;
    use crate::agent::role_templates::RoleTemplateModelSource;
    use crate::protocol::SandboxPolicy;
    use codex_protocol::openai_models::ModelPreset;
    use codex_protocol::openai_models::ReasoningEffort as ReasoningEffortConfig;

    use crate::agent::exceeds_thread_spawn_depth_limit;
    use crate::agent::next_thread_spawn_depth;
    use std::sync::Arc;

    #[derive(Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct SpawnAgentArgs {
        message: Option<String>,
        items: Option<Vec<UserInput>>,
        agent_type: Option<String>,
        agent_nickname: Option<String>,
        agent_name: Option<String>,
        model: Option<String>,
        reasoning_effort: Option<String>,
        thread_note: Option<String>,
    }

    #[derive(Debug, Serialize)]
    struct SpawnAgentResult {
        agent_id: String,
        nickname: Option<String>,
        thread_note: Option<String>,
        requested_model: Option<String>,
        model_source: String,
        model: Option<String>,
        reasoning_effort: Option<codex_protocol::openai_models::ReasoningEffort>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum SpawnModelSource {
        ExplicitArgument,
        TemplatePersona,
        TemplateRole,
        InheritedParent,
        CatalogDefault,
    }

    impl SpawnModelSource {
        const fn as_str(self) -> &'static str {
            match self {
                Self::ExplicitArgument => "explicit_argument",
                Self::TemplatePersona => "template_persona",
                Self::TemplateRole => "template_role",
                Self::InheritedParent => "inherited_parent",
                Self::CatalogDefault => "catalog_default",
            }
        }
    }

    pub async fn handle(
        session: Arc<Session>,
        turn: Arc<TurnContext>,
        call_id: String,
        arguments: String,
    ) -> Result<ToolOutput, FunctionCallError> {
        let args: SpawnAgentArgs = parse_arguments(&arguments)?;
        if args
            .agent_name
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
        {
            return Err(FunctionCallError::RespondToModel(
                "unsupported key `agent_name`; use `agent_nickname`".to_string(),
            ));
        }
        let role_name = args
            .agent_type
            .as_deref()
            .map(str::trim)
            .filter(|role| !role.is_empty());
        let requested_agent_nickname = args
            .agent_nickname
            .as_deref()
            .map(str::trim)
            .filter(|nickname| !nickname.is_empty());
        let model_override = args
            .model
            .as_deref()
            .map(str::trim)
            .filter(|model| !model.is_empty())
            .map(ToString::to_string);
        let requested_model = model_override.clone();
        let reasoning_effort_override = args
            .reasoning_effort
            .as_deref()
            .map(str::trim)
            .filter(|effort| !effort.is_empty())
            .map(ToString::to_string);
        let thread_note = crate::util::normalize_thread_note(args.thread_note.as_deref());
        let input_items = parse_collab_input(args.message, args.items)?;
        let prompt = input_preview(&input_items);
        let session_source = turn.session_source.clone();
        let child_depth = next_thread_spawn_depth(&session_source);
        let max_depth = turn.config.agent_max_depth;
        if exceeds_thread_spawn_depth_limit(child_depth, max_depth) {
            return Err(FunctionCallError::RespondToModel(
                "Agent depth limit reached. Solve the task yourself.".to_string(),
            ));
        }
        session
            .send_event(
                &turn,
                CollabAgentSpawnBeginEvent {
                    call_id: call_id.clone(),
                    sender_thread_id: session.conversation_id,
                    prompt: prompt.clone(),
                }
                .into(),
            )
            .await;
        let mut config =
            build_agent_spawn_config(&session.get_base_instructions().await, turn.as_ref())?;
        let requested_role_name = role_name.unwrap_or(DEFAULT_ROLE_NAME);
        let loaded_templates = LoadedRoleTemplates::load_for_context(
            Some(turn.cwd.as_path()),
            Some(turn.config.codex_home.as_path()),
        )
        .map_err(FunctionCallError::RespondToModel)?;
        let template_resolved_role = loaded_templates
            .resolve_role_name(requested_role_name)
            .map_err(FunctionCallError::RespondToModel)?;
        let declared_role_name = if is_declared_role(&config, requested_role_name) {
            Some(requested_role_name.to_string())
        } else if let Some(template_role_name) = template_resolved_role.as_ref() {
            if is_declared_role(&config, template_role_name) {
                Some(template_role_name.clone())
            } else {
                None
            }
        } else {
            None
        };
        if let Some(declared_role_name) = declared_role_name.as_deref() {
            apply_role_to_config(&mut config, Some(declared_role_name))
                .await
                .map_err(FunctionCallError::RespondToModel)?;
        } else if template_resolved_role.is_none() {
            return Err(FunctionCallError::RespondToModel(format!(
                "unknown agent_type '{requested_role_name}'"
            )));
        }
        apply_spawn_agent_runtime_overrides(&mut config, turn.as_ref())?;
        let mut model_source = SpawnModelSource::InheritedParent;

        let effective_role_name = template_resolved_role
            .as_deref()
            .or(declared_role_name.as_deref())
            .unwrap_or(requested_role_name);
        let template_settings = loaded_templates
            .resolve_spawn_settings(effective_role_name, requested_agent_nickname)
            .map_err(FunctionCallError::RespondToModel)?;
        if template_settings.is_none()
            && requested_agent_nickname
                .is_some_and(|nickname| !nickname.eq_ignore_ascii_case(DEFAULT_AGENT_NICKNAME))
        {
            return Err(FunctionCallError::RespondToModel(format!(
                "agent_type '{effective_role_name}' does not define agent_nickname '{nickname}'",
                nickname = requested_agent_nickname.unwrap_or(DEFAULT_AGENT_NICKNAME)
            )));
        }

        let (agent_persona, allow_list, deny_list) = match template_settings {
            Some(settings) => {
                config.base_instructions = Some(settings.instructions);
                if let Some(model) = settings.model {
                    config.model = Some(model);
                    model_source = match settings.model_source {
                        Some(RoleTemplateModelSource::Nickname) => {
                            SpawnModelSource::TemplatePersona
                        }
                        Some(RoleTemplateModelSource::Role) => SpawnModelSource::TemplateRole,
                        None => SpawnModelSource::TemplateRole,
                    };
                }
                if let Some(reasoning_effort) = settings.reasoning_effort {
                    config.model_reasoning_effort = Some(reasoning_effort);
                }
                if settings.read_only {
                    config
                        .permissions
                        .sandbox_policy
                        .set(SandboxPolicy::new_read_only_policy())
                        .map_err(|err| {
                            FunctionCallError::RespondToModel(format!(
                                "sandbox_policy is invalid: {err}"
                            ))
                        })?;
                }
                (
                    Some(settings.agent_persona),
                    settings.allow_list,
                    settings.deny_list,
                )
            }
            None => (None, None, None),
        };
        if let Some(model) = model_override {
            config.model = Some(model);
            model_source = SpawnModelSource::ExplicitArgument;
        }
        if let Some(reasoning_effort) = reasoning_effort_override {
            let presets = session
                .services
                .models_manager
                .try_list_models()
                .map_err(|_| {
                    FunctionCallError::RespondToModel(
                        "Models are being updated; try spawn_agent again in a moment.".to_string(),
                    )
                })?;
            let model = resolve_spawn_model(&mut config, &presets)?;
            let preset = presets
                .iter()
                .find(|preset| preset.model == model)
                .ok_or_else(|| {
                    let available = available_models_csv(&presets);
                    FunctionCallError::RespondToModel(format!(
                        "unknown model {model:?} for spawn_agent. Available models: {available}"
                    ))
                })?;
            let effort = parse_reasoning_effort_config(&reasoning_effort).ok_or_else(|| {
                let supported = supported_reasoning_efforts_csv(preset);
                FunctionCallError::RespondToModel(format!(
                    "reasoning_effort {reasoning_effort:?} is not supported for model {model:?}. Supported efforts: {supported}"
                ))
            })?;
            config.model_reasoning_effort = Some(effort);
        }
        let had_model_before_validation = config.model.is_some();
        validate_spawn_model_selection(session.as_ref(), &mut config)?;
        if !had_model_before_validation && config.model.is_some() {
            model_source = SpawnModelSource::CatalogDefault;
        }
        apply_spawn_agent_overrides(&mut config, child_depth);

        let result = session
            .services
            .agent_control
            .spawn_agent(
                config,
                input_items,
                Some(thread_spawn_source(
                    session.conversation_id,
                    child_depth,
                    agent_persona.clone(),
                    Some(effective_role_name),
                    thread_note.clone(),
                    allow_list,
                    deny_list,
                )),
                thread_note.clone(),
            )
            .await
            .map_err(collab_spawn_error);
        let (new_thread_id, status) = match &result {
            Ok(thread_id) => (
                Some(*thread_id),
                session.services.agent_control.get_status(*thread_id).await,
            ),
            Err(_) => (None, AgentStatus::NotFound),
        };
        let (new_agent_nickname, new_agent_persona, new_agent_role, new_agent_thread_note) =
            match new_thread_id {
                Some(thread_id) => session
                    .services
                    .agent_control
                    .get_agent_metadata(thread_id)
                    .await
                    .unwrap_or((None, None, None, None)),
                None => (None, None, None, None),
            };
        let (new_agent_model, new_agent_reasoning_effort) = match new_thread_id {
            Some(thread_id) => session
                .services
                .agent_control
                .get_agent_model_settings(thread_id)
                .await
                .unwrap_or((None, None)),
            None => (None, None),
        };
        let nickname = new_agent_nickname.clone();
        let result_thread_note = new_agent_thread_note.clone();
        session
            .send_event(
                &turn,
                CollabAgentSpawnEndEvent {
                    call_id,
                    sender_thread_id: session.conversation_id,
                    new_thread_id,
                    new_agent_nickname,
                    new_agent_persona,
                    new_agent_role,
                    new_agent_thread_note,
                    prompt,
                    status,
                }
                .into(),
            )
            .await;
        let new_thread_id = result?;
        turn.otel_manager.counter(
            "codex.multi_agent.spawn",
            1,
            &[("role", effective_role_name)],
        );

        let content = serde_json::to_string(&SpawnAgentResult {
            agent_id: new_thread_id.to_string(),
            nickname,
            thread_note: result_thread_note,
            requested_model,
            model_source: model_source.as_str().to_string(),
            model: new_agent_model,
            reasoning_effort: new_agent_reasoning_effort,
        })
        .map_err(|err| {
            FunctionCallError::Fatal(format!("failed to serialize spawn_agent result: {err}"))
        })?;

        Ok(ToolOutput::Function {
            body: FunctionCallOutputBody::Text(content),
            success: Some(true),
        })
    }

    fn validate_spawn_model_selection(
        session: &Session,
        config: &mut Config,
    ) -> Result<(), FunctionCallError> {
        let presets = session
            .services
            .models_manager
            .try_list_models()
            .map_err(|_| {
                FunctionCallError::RespondToModel(
                    "Models are being updated; try spawn_agent again in a moment.".to_string(),
                )
            })?;
        let model = resolve_spawn_model(config, &presets)?;
        let preset = presets
            .iter()
            .find(|preset| preset.model == model)
            .ok_or_else(|| {
                let available = available_models_csv(&presets);
                FunctionCallError::RespondToModel(format!(
                    "unknown model {model:?} for spawn_agent. Available models: {available}"
                ))
            })?;

        if let Some(effort) = config.model_reasoning_effort
            && !preset
                .supported_reasoning_efforts
                .iter()
                .any(|supported| supported.effort == effort)
        {
            let supported = supported_reasoning_efforts_csv(preset);
            return Err(FunctionCallError::RespondToModel(format!(
                "reasoning_effort {effort:?} is not supported for model {model:?}. Supported efforts: {supported}"
            )));
        }

        Ok(())
    }

    fn resolve_spawn_model(
        config: &mut Config,
        presets: &[ModelPreset],
    ) -> Result<String, FunctionCallError> {
        if let Some(model) = config.model.as_deref() {
            return Ok(model.to_string());
        }

        let default_model = presets
            .iter()
            .find(|preset| preset.is_default)
            .or_else(|| presets.first())
            .map(|preset| preset.model.clone())
            .ok_or_else(|| {
                FunctionCallError::RespondToModel(
                    "No models are available for spawn_agent.".to_string(),
                )
            })?;
        config.model = Some(default_model.clone());
        Ok(default_model)
    }

    fn available_models_csv(presets: &[ModelPreset]) -> String {
        let models = presets
            .iter()
            .map(|preset| preset.model.as_str())
            .collect::<Vec<_>>();
        if models.is_empty() {
            "<none>".to_string()
        } else {
            models.join(", ")
        }
    }

    fn supported_reasoning_efforts_csv(preset: &ModelPreset) -> String {
        let efforts = preset
            .supported_reasoning_efforts
            .iter()
            .map(|supported| supported.effort.to_string())
            .collect::<Vec<_>>();
        if efforts.is_empty() {
            "<none>".to_string()
        } else {
            efforts.join(", ")
        }
    }

    fn parse_reasoning_effort_config(effort: &str) -> Option<ReasoningEffortConfig> {
        match effort.trim().to_ascii_lowercase().as_str() {
            "none" => Some(ReasoningEffortConfig::None),
            "minimal" => Some(ReasoningEffortConfig::Minimal),
            "low" => Some(ReasoningEffortConfig::Low),
            "medium" => Some(ReasoningEffortConfig::Medium),
            "high" => Some(ReasoningEffortConfig::High),
            "xhigh" => Some(ReasoningEffortConfig::XHigh),
            _ => None,
        }
    }
}

mod send_input {
    use super::*;
    use std::sync::Arc;

    #[derive(Debug, Deserialize)]
    struct SendInputArgs {
        id: String,
        message: Option<String>,
        items: Option<Vec<UserInput>>,
        #[serde(default)]
        interrupt: bool,
    }

    #[derive(Debug, Serialize)]
    struct SendInputResult {
        submission_id: String,
    }

    pub async fn handle(
        session: Arc<Session>,
        turn: Arc<TurnContext>,
        call_id: String,
        arguments: String,
    ) -> Result<ToolOutput, FunctionCallError> {
        let args: SendInputArgs = parse_arguments(&arguments)?;
        let receiver_thread_id = agent_id(&args.id)?;
        let input_items = parse_collab_input(args.message, args.items)?;
        let prompt = input_preview(&input_items);
        let (receiver_agent_nickname, receiver_agent_persona, receiver_agent_role) = session
            .services
            .agent_control
            .get_agent_identity(receiver_thread_id)
            .await
            .unwrap_or((None, None, None));
        if args.interrupt {
            session
                .services
                .agent_control
                .interrupt_agent(receiver_thread_id)
                .await
                .map_err(|err| collab_agent_error(receiver_thread_id, err))?;
        }
        session
            .send_event(
                &turn,
                CollabAgentInteractionBeginEvent {
                    call_id: call_id.clone(),
                    sender_thread_id: session.conversation_id,
                    receiver_thread_id,
                    prompt: prompt.clone(),
                }
                .into(),
            )
            .await;
        let result = session
            .services
            .agent_control
            .send_input(receiver_thread_id, input_items)
            .await
            .map_err(|err| collab_agent_error(receiver_thread_id, err));
        let status = session
            .services
            .agent_control
            .get_status(receiver_thread_id)
            .await;
        session
            .send_event(
                &turn,
                CollabAgentInteractionEndEvent {
                    call_id,
                    sender_thread_id: session.conversation_id,
                    receiver_thread_id,
                    receiver_agent_nickname,
                    receiver_agent_persona,
                    receiver_agent_role,
                    prompt,
                    status,
                }
                .into(),
            )
            .await;
        let submission_id = result?;

        let content = serde_json::to_string(&SendInputResult { submission_id }).map_err(|err| {
            FunctionCallError::Fatal(format!("failed to serialize send_input result: {err}"))
        })?;

        Ok(ToolOutput::Function {
            body: FunctionCallOutputBody::Text(content),
            success: Some(true),
        })
    }
}

mod set_thread_note {
    use super::*;
    use std::sync::Arc;

    #[derive(Debug, Deserialize)]
    struct SetThreadNoteArgs {
        id: String,
        note: Option<String>,
    }

    #[derive(Debug, Serialize)]
    struct SetThreadNoteResult {
        thread_note: Option<String>,
    }

    pub async fn handle(
        session: Arc<Session>,
        _turn: Arc<TurnContext>,
        _call_id: String,
        arguments: String,
    ) -> Result<ToolOutput, FunctionCallError> {
        let args: SetThreadNoteArgs = parse_arguments(&arguments)?;
        let receiver_thread_id = agent_id(&args.id)?;
        let note = crate::util::normalize_thread_note(args.note.as_deref());

        session
            .services
            .agent_control
            .set_thread_note(receiver_thread_id, note.clone())
            .await
            .map_err(|err| collab_agent_error(receiver_thread_id, err))?;

        let content =
            serde_json::to_string(&SetThreadNoteResult { thread_note: note }).map_err(|err| {
                FunctionCallError::Fatal(format!(
                    "failed to serialize set_thread_note result: {err}"
                ))
            })?;

        Ok(ToolOutput::Function {
            body: FunctionCallOutputBody::Text(content),
            success: Some(true),
        })
    }
}

mod resume_agent {
    use super::*;
    use crate::agent::next_thread_spawn_depth;
    use std::sync::Arc;

    #[derive(Debug, Deserialize)]
    struct ResumeAgentArgs {
        id: String,
    }

    #[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
    pub(super) struct ResumeAgentResult {
        pub(super) status: AgentStatus,
    }

    pub async fn handle(
        session: Arc<Session>,
        turn: Arc<TurnContext>,
        call_id: String,
        arguments: String,
    ) -> Result<ToolOutput, FunctionCallError> {
        let args: ResumeAgentArgs = parse_arguments(&arguments)?;
        let receiver_thread_id = agent_id(&args.id)?;
        let (receiver_agent_nickname, receiver_agent_persona, receiver_agent_role) = session
            .services
            .agent_control
            .get_agent_identity(receiver_thread_id)
            .await
            .unwrap_or((None, None, None));
        let child_depth = next_thread_spawn_depth(&turn.session_source);
        let max_depth = turn.config.agent_max_depth;
        if exceeds_thread_spawn_depth_limit(child_depth, max_depth) {
            return Err(FunctionCallError::RespondToModel(
                "Agent depth limit reached. Solve the task yourself.".to_string(),
            ));
        }

        session
            .send_event(
                &turn,
                CollabResumeBeginEvent {
                    call_id: call_id.clone(),
                    sender_thread_id: session.conversation_id,
                    receiver_thread_id,
                    receiver_agent_nickname: receiver_agent_nickname.clone(),
                    receiver_agent_persona: receiver_agent_persona.clone(),
                    receiver_agent_role: receiver_agent_role.clone(),
                }
                .into(),
            )
            .await;

        let mut status = session
            .services
            .agent_control
            .get_status(receiver_thread_id)
            .await;
        let error = if matches!(status, AgentStatus::NotFound) {
            // If the thread is no longer active, attempt to restore it from rollout.
            match try_resume_closed_agent(&session, &turn, receiver_thread_id, child_depth).await {
                Ok(resumed_status) => {
                    status = resumed_status;
                    None
                }
                Err(err) => {
                    status = session
                        .services
                        .agent_control
                        .get_status(receiver_thread_id)
                        .await;
                    Some(err)
                }
            }
        } else {
            None
        };

        let (receiver_agent_nickname, receiver_agent_persona, receiver_agent_role) = session
            .services
            .agent_control
            .get_agent_identity(receiver_thread_id)
            .await
            .unwrap_or((
                receiver_agent_nickname,
                receiver_agent_persona,
                receiver_agent_role,
            ));
        session
            .send_event(
                &turn,
                CollabResumeEndEvent {
                    call_id,
                    sender_thread_id: session.conversation_id,
                    receiver_thread_id,
                    receiver_agent_nickname,
                    receiver_agent_persona,
                    receiver_agent_role,
                    status: status.clone(),
                }
                .into(),
            )
            .await;

        if let Some(err) = error {
            return Err(err);
        }
        turn.otel_manager
            .counter("codex.multi_agent.resume", 1, &[]);

        let content = serde_json::to_string(&ResumeAgentResult { status }).map_err(|err| {
            FunctionCallError::Fatal(format!("failed to serialize resume_agent result: {err}"))
        })?;

        Ok(ToolOutput::Function {
            body: FunctionCallOutputBody::Text(content),
            success: Some(true),
        })
    }

    async fn try_resume_closed_agent(
        session: &Arc<Session>,
        turn: &Arc<TurnContext>,
        receiver_thread_id: ThreadId,
        child_depth: i32,
    ) -> Result<AgentStatus, FunctionCallError> {
        let config = build_agent_resume_config(turn.as_ref(), child_depth)?;
        let resumed_thread_id = session
            .services
            .agent_control
            .resume_agent_from_rollout(
                config,
                receiver_thread_id,
                thread_spawn_source(
                    session.conversation_id,
                    child_depth,
                    None,
                    None,
                    None,
                    None,
                    None,
                ),
            )
            .await
            .map_err(|err| collab_agent_error(receiver_thread_id, err))?;

        Ok(session
            .services
            .agent_control
            .get_status(resumed_thread_id)
            .await)
    }
}

pub(crate) mod wait {
    use super::*;
    use crate::agent::status::is_final;
    use futures::FutureExt;
    use futures::StreamExt;
    use futures::stream::FuturesUnordered;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::watch::Receiver;
    use tokio::time::Instant;

    use tokio::time::timeout_at;

    #[derive(Debug, Deserialize)]
    struct WaitArgs {
        ids: Vec<String>,
        timeout_ms: Option<i64>,
    }

    #[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
    pub(crate) struct WaitResult {
        pub(crate) status: HashMap<ThreadId, AgentStatus>,
        pub(crate) timed_out: bool,
    }

    pub async fn handle(
        session: Arc<Session>,
        turn: Arc<TurnContext>,
        call_id: String,
        arguments: String,
    ) -> Result<ToolOutput, FunctionCallError> {
        let args: WaitArgs = parse_arguments(&arguments)?;
        if args.ids.is_empty() {
            return Err(FunctionCallError::RespondToModel(
                "ids must be non-empty".to_owned(),
            ));
        }
        let receiver_thread_ids = args
            .ids
            .iter()
            .map(|id| agent_id(id))
            .collect::<Result<Vec<_>, _>>()?;
        let mut receiver_agents = Vec::with_capacity(receiver_thread_ids.len());
        for receiver_thread_id in &receiver_thread_ids {
            let (agent_nickname, agent_persona, agent_role, thread_note) = session
                .services
                .agent_control
                .get_agent_metadata(*receiver_thread_id)
                .await
                .unwrap_or((None, None, None, None));
            receiver_agents.push(CollabAgentRef {
                thread_id: *receiver_thread_id,
                agent_nickname,
                agent_persona,
                agent_role,
                thread_note,
            });
        }

        // Validate timeout.
        // Very short timeouts encourage busy-polling loops in the orchestrator prompt and can
        // cause high CPU usage even with a single active worker, so clamp to a minimum.
        let timeout_ms = args.timeout_ms.unwrap_or(DEFAULT_WAIT_TIMEOUT_MS);
        let timeout_ms = match timeout_ms {
            ms if ms <= 0 => {
                return Err(FunctionCallError::RespondToModel(
                    "timeout_ms must be greater than zero".to_owned(),
                ));
            }
            ms => ms.clamp(MIN_WAIT_TIMEOUT_MS, MAX_WAIT_TIMEOUT_MS),
        };

        session
            .send_event(
                &turn,
                CollabWaitingBeginEvent {
                    sender_thread_id: session.conversation_id,
                    receiver_thread_ids: receiver_thread_ids.clone(),
                    receiver_agents: receiver_agents.clone(),
                    call_id: call_id.clone(),
                }
                .into(),
            )
            .await;

        let mut status_rxs = Vec::with_capacity(receiver_thread_ids.len());
        let mut initial_final_statuses = Vec::new();
        for id in &receiver_thread_ids {
            match session.services.agent_control.subscribe_status(*id).await {
                Ok(rx) => {
                    let status = rx.borrow().clone();
                    if is_final(&status) {
                        initial_final_statuses.push((*id, status));
                    }
                    status_rxs.push((*id, rx));
                }
                Err(CodexErr::ThreadNotFound(_)) => {
                    initial_final_statuses.push((*id, AgentStatus::NotFound));
                }
                Err(err) => {
                    let mut statuses = HashMap::with_capacity(1);
                    statuses.insert(*id, session.services.agent_control.get_status(*id).await);
                    session
                        .send_event(
                            &turn,
                            CollabWaitingEndEvent {
                                sender_thread_id: session.conversation_id,
                                call_id: call_id.clone(),
                                agent_statuses: build_wait_agent_statuses(
                                    &statuses,
                                    &receiver_agents,
                                ),
                                statuses,
                            }
                            .into(),
                        )
                        .await;
                    return Err(collab_agent_error(*id, err));
                }
            }
        }

        let statuses = if !initial_final_statuses.is_empty() {
            initial_final_statuses
        } else {
            // Wait for the first agent to reach a final status.
            let mut futures = FuturesUnordered::new();
            for (id, rx) in status_rxs.into_iter() {
                let session = session.clone();
                futures.push(wait_for_final_status(session, id, rx));
            }
            let mut results = Vec::new();
            let deadline = Instant::now() + Duration::from_millis(timeout_ms as u64);
            loop {
                match timeout_at(deadline, futures.next()).await {
                    Ok(Some(Some(result))) => {
                        results.push(result);
                        break;
                    }
                    Ok(Some(None)) => continue,
                    Ok(None) | Err(_) => break,
                }
            }
            if !results.is_empty() {
                // Drain the unlikely last elements to prevent race.
                loop {
                    match futures.next().now_or_never() {
                        Some(Some(Some(result))) => results.push(result),
                        Some(Some(None)) => continue,
                        Some(None) | None => break,
                    }
                }
            }
            results
        };

        // Convert payload.
        let statuses_map = statuses.clone().into_iter().collect::<HashMap<_, _>>();
        let agent_statuses = build_wait_agent_statuses(&statuses_map, &receiver_agents);
        let result = WaitResult {
            status: statuses_map.clone(),
            timed_out: statuses.is_empty(),
        };

        // Final event emission.
        session
            .send_event(
                &turn,
                CollabWaitingEndEvent {
                    sender_thread_id: session.conversation_id,
                    call_id,
                    agent_statuses,
                    statuses: statuses_map,
                }
                .into(),
            )
            .await;

        let content = serde_json::to_string(&result).map_err(|err| {
            FunctionCallError::Fatal(format!("failed to serialize wait result: {err}"))
        })?;

        Ok(ToolOutput::Function {
            body: FunctionCallOutputBody::Text(content),
            success: None,
        })
    }

    async fn wait_for_final_status(
        session: Arc<Session>,
        thread_id: ThreadId,
        mut status_rx: Receiver<AgentStatus>,
    ) -> Option<(ThreadId, AgentStatus)> {
        let mut status = status_rx.borrow().clone();
        if is_final(&status) {
            return Some((thread_id, status));
        }

        loop {
            if status_rx.changed().await.is_err() {
                let latest = session.services.agent_control.get_status(thread_id).await;
                return is_final(&latest).then_some((thread_id, latest));
            }
            status = status_rx.borrow().clone();
            if is_final(&status) {
                return Some((thread_id, status));
            }
        }
    }
}

pub mod close_agent {
    use super::*;
    use std::sync::Arc;

    #[derive(Debug, Deserialize, Serialize)]
    pub(super) struct CloseAgentResult {
        pub(super) status: AgentStatus,
    }

    pub async fn handle(
        session: Arc<Session>,
        turn: Arc<TurnContext>,
        call_id: String,
        arguments: String,
    ) -> Result<ToolOutput, FunctionCallError> {
        let args: CloseAgentArgs = parse_arguments(&arguments)?;
        let agent_id = agent_id(&args.id)?;
        let (receiver_agent_nickname, receiver_agent_persona, receiver_agent_role) = session
            .services
            .agent_control
            .get_agent_identity(agent_id)
            .await
            .unwrap_or((None, None, None));
        session
            .send_event(
                &turn,
                CollabCloseBeginEvent {
                    call_id: call_id.clone(),
                    sender_thread_id: session.conversation_id,
                    receiver_thread_id: agent_id,
                }
                .into(),
            )
            .await;
        if matches!(
            turn.session_source,
            SessionSource::SubAgent(SubAgentSource::ThreadSpawn { .. })
        ) && agent_id != session.conversation_id
            && !session
                .services
                .agent_control
                .is_descendant_of(session.conversation_id, agent_id)
                .await
        {
            let status = AgentStatus::Errored(
                "not permitted to close agents outside your subtree".to_string(),
            );
            session
                .send_event(
                    &turn,
                    CollabCloseEndEvent {
                        call_id: call_id.clone(),
                        sender_thread_id: session.conversation_id,
                        receiver_thread_id: agent_id,
                        receiver_agent_nickname: receiver_agent_nickname.clone(),
                        receiver_agent_persona: receiver_agent_persona.clone(),
                        receiver_agent_role: receiver_agent_role.clone(),
                        status,
                    }
                    .into(),
                )
                .await;
            return Err(FunctionCallError::RespondToModel(
                "Not permitted to close agents outside your subtree.".to_string(),
            ));
        }
        let status = match session
            .services
            .agent_control
            .subscribe_status(agent_id)
            .await
        {
            Ok(mut status_rx) => status_rx.borrow_and_update().clone(),
            Err(err) => {
                let status = session.services.agent_control.get_status(agent_id).await;
                session
                    .send_event(
                        &turn,
                        CollabCloseEndEvent {
                            call_id: call_id.clone(),
                            sender_thread_id: session.conversation_id,
                            receiver_thread_id: agent_id,
                            receiver_agent_nickname: receiver_agent_nickname.clone(),
                            receiver_agent_persona: receiver_agent_persona.clone(),
                            receiver_agent_role: receiver_agent_role.clone(),
                            status,
                        }
                        .into(),
                    )
                    .await;
                return Err(collab_agent_error(agent_id, err));
            }
        };
        let result = if !matches!(status, AgentStatus::Shutdown) {
            session
                .services
                .agent_control
                .shutdown_agent(agent_id)
                .await
                .map_err(|err| collab_agent_error(agent_id, err))
                .map(|_| ())
        } else {
            Ok(())
        };
        session
            .send_event(
                &turn,
                CollabCloseEndEvent {
                    call_id,
                    sender_thread_id: session.conversation_id,
                    receiver_thread_id: agent_id,
                    receiver_agent_nickname,
                    receiver_agent_persona,
                    receiver_agent_role,
                    status: status.clone(),
                }
                .into(),
            )
            .await;
        result?;

        let content = serde_json::to_string(&CloseAgentResult { status }).map_err(|err| {
            FunctionCallError::Fatal(format!("failed to serialize close_agent result: {err}"))
        })?;

        Ok(ToolOutput::Function {
            body: FunctionCallOutputBody::Text(content),
            success: Some(true),
        })
    }
}

fn agent_id(id: &str) -> Result<ThreadId, FunctionCallError> {
    ThreadId::from_string(id)
        .map_err(|e| FunctionCallError::RespondToModel(format!("invalid agent id {id}: {e:?}")))
}

fn build_wait_agent_statuses(
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
                agent_persona: receiver_agent.agent_persona.clone(),
                agent_role: receiver_agent.agent_role.clone(),
                thread_note: receiver_agent.thread_note.clone(),
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
            agent_persona: None,
            agent_role: None,
            thread_note: None,
            status: status.clone(),
        })
        .collect::<Vec<_>>();
    extras.sort_by(|left, right| left.thread_id.to_string().cmp(&right.thread_id.to_string()));
    entries.extend(extras);
    entries
}

fn collab_spawn_error(err: CodexErr) -> FunctionCallError {
    match err {
        CodexErr::UnsupportedOperation(_) => {
            FunctionCallError::RespondToModel("collab manager unavailable".to_string())
        }
        err => FunctionCallError::RespondToModel(format!("collab spawn failed: {err}")),
    }
}

fn collab_agent_error(agent_id: ThreadId, err: CodexErr) -> FunctionCallError {
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

fn thread_spawn_source(
    parent_thread_id: ThreadId,
    depth: i32,
    agent_persona: Option<String>,
    agent_role: Option<&str>,
    thread_note: Option<String>,
    allow_list: Option<Vec<String>>,
    deny_list: Option<Vec<String>>,
) -> SessionSource {
    SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
        parent_thread_id,
        depth,
        agent_nickname: None,
        agent_persona,
        agent_role: agent_role.map(str::to_string),
        thread_note,
        allow_list,
        deny_list,
    })
}

fn parse_collab_input(
    message: Option<String>,
    items: Option<Vec<UserInput>>,
) -> Result<Vec<UserInput>, FunctionCallError> {
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
            }])
        }
        (None, Some(items)) => {
            if items.is_empty() {
                return Err(FunctionCallError::RespondToModel(
                    "Items can't be empty".to_string(),
                ));
            }
            Ok(items)
        }
    }
}

fn input_preview(items: &[UserInput]) -> String {
    let parts: Vec<String> = items
        .iter()
        .map(|item| match item {
            UserInput::Text { text, .. } => text.clone(),
            UserInput::Image { .. } => "[image]".to_string(),
            UserInput::LocalImage { path } => format!("[local_image:{}]", path.display()),
            UserInput::Skill { name, path } => {
                format!("[skill:${name}]({})", path.display())
            }
            UserInput::Mention { name, path } => format!("[mention:${name}]({path})"),
            _ => "[input]".to_string(),
        })
        .collect();

    parts.join("\n")
}

pub(crate) fn build_agent_spawn_config(
    base_instructions: &BaseInstructions,
    turn: &TurnContext,
) -> Result<Config, FunctionCallError> {
    let mut config = build_agent_shared_config(turn)?;
    config.base_instructions = Some(base_instructions.text.clone());
    Ok(config)
}

fn build_agent_resume_config(
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
    config.model = Some(turn.model_info.slug.clone());
    config.model_provider = turn.provider.clone();
    config.model_reasoning_effort = turn.reasoning_effort;
    config.model_reasoning_summary = Some(turn.reasoning_summary);
    config.developer_instructions = turn.developer_instructions.clone();
    config.compact_prompt = turn.compact_prompt.clone();
    apply_spawn_agent_runtime_overrides(&mut config, turn)?;

    Ok(config)
}

fn apply_spawn_agent_runtime_overrides(
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
        .sandbox_policy
        .set(turn.sandbox_policy.get().clone())
        .map_err(|err| {
            FunctionCallError::RespondToModel(format!("sandbox_policy is invalid: {err}"))
        })?;
    Ok(())
}

fn apply_spawn_agent_overrides(config: &mut Config, child_depth: i32) {
    if child_depth >= config.agent_max_depth {
        config.features.disable(Feature::Collab);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AuthManager;
    use crate::CodexAuth;
    use crate::ThreadManager;
    use crate::built_in_model_providers;
    use crate::codex::make_session_and_context;
    use crate::config::DEFAULT_AGENT_MAX_DEPTH;
    use crate::config::types::ShellEnvironmentPolicy;
    use crate::function_tool::FunctionCallError;
    use crate::protocol::AskForApproval;
    use crate::protocol::Op;
    use crate::protocol::SandboxPolicy;
    use crate::protocol::SessionSource;
    use crate::protocol::SubAgentSource;
    use crate::turn_diff_tracker::TurnDiffTracker;
    use codex_protocol::ThreadId;
    use codex_protocol::models::ContentItem;
    use codex_protocol::models::ResponseItem;
    use codex_protocol::protocol::InitialHistory;
    use codex_protocol::protocol::RolloutItem;
    use pretty_assertions::assert_eq;
    use serde::Deserialize;
    use serde_json::json;
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;
    use tempfile::TempDir;
    use tokio::sync::Mutex;
    use tokio::time::timeout;

    fn invocation(
        session: Arc<crate::codex::Session>,
        turn: Arc<TurnContext>,
        tool_name: &str,
        payload: ToolPayload,
    ) -> ToolInvocation {
        ToolInvocation {
            session,
            turn,
            tracker: Arc::new(Mutex::new(TurnDiffTracker::default())),
            call_id: "call-1".to_string(),
            tool_name: tool_name.to_string(),
            payload,
        }
    }

    fn function_payload(args: serde_json::Value) -> ToolPayload {
        ToolPayload::Function {
            arguments: args.to_string(),
        }
    }

    fn thread_manager() -> ThreadManager {
        ThreadManager::with_models_provider_for_tests(
            CodexAuth::from_api_key("dummy"),
            built_in_model_providers()["openai"].clone(),
        )
    }

    fn write_role_template(temp_dir: &TempDir, role_name: &str, template_body: &str) -> PathBuf {
        let role_dir = temp_dir.path().join(".codex").join(".agents");
        fs::create_dir_all(&role_dir).expect("create role templates directory");
        let role_path = role_dir.join(format!("{role_name}.md"));
        fs::write(&role_path, template_body).expect("write role template");
        role_path
    }

    #[tokio::test]
    async fn handler_rejects_non_function_payloads() {
        let (session, turn) = make_session_and_context().await;
        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "spawn_agent",
            ToolPayload::Custom {
                input: "hello".to_string(),
            },
        );
        let Err(err) = MultiAgentHandler.handle(invocation).await else {
            panic!("payload should be rejected");
        };
        assert_eq!(
            err,
            FunctionCallError::RespondToModel(
                "collab handler received unsupported payload".to_string()
            )
        );
    }

    #[tokio::test]
    async fn handler_rejects_unknown_tool() {
        let (session, turn) = make_session_and_context().await;
        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "unknown_tool",
            function_payload(json!({})),
        );
        let Err(err) = MultiAgentHandler.handle(invocation).await else {
            panic!("tool should be rejected");
        };
        assert_eq!(
            err,
            FunctionCallError::RespondToModel("unsupported collab tool unknown_tool".to_string())
        );
    }

    #[tokio::test]
    async fn spawn_agent_rejects_empty_message() {
        let (session, turn) = make_session_and_context().await;
        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "spawn_agent",
            function_payload(json!({"message": "   "})),
        );
        let Err(err) = MultiAgentHandler.handle(invocation).await else {
            panic!("empty message should be rejected");
        };
        assert_eq!(
            err,
            FunctionCallError::RespondToModel(
                "Empty message can't be sent to an agent".to_string()
            )
        );
    }

    #[tokio::test]
    async fn spawn_agent_rejects_when_message_and_items_are_both_set() {
        let (session, turn) = make_session_and_context().await;
        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "spawn_agent",
            function_payload(json!({
                "message": "hello",
                "items": [{"type": "mention", "name": "drive", "path": "app://drive"}]
            })),
        );
        let Err(err) = MultiAgentHandler.handle(invocation).await else {
            panic!("message+items should be rejected");
        };
        assert_eq!(
            err,
            FunctionCallError::RespondToModel(
                "Provide either message or items, but not both".to_string()
            )
        );
    }

    #[tokio::test]
    async fn spawn_agent_uses_explorer_role_and_preserves_approval_policy() {
        #[derive(Debug, Deserialize)]
        struct SpawnAgentResult {
            agent_id: String,
            nickname: Option<String>,
            thread_note: Option<String>,
        }

        let (mut session, mut turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();
        let mut config = (*turn.config).clone();
        config
            .permissions
            .approval_policy
            .set(AskForApproval::OnRequest)
            .expect("approval policy should be set");
        turn.approval_policy
            .set(AskForApproval::OnRequest)
            .expect("approval policy should be set");
        turn.config = Arc::new(config);

        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "spawn_agent",
            function_payload(json!({
                "message": "inspect this repo",
                "agent_type": "explorer"
            })),
        );
        let output = MultiAgentHandler
            .handle(invocation)
            .await
            .expect("spawn_agent should succeed");
        let ToolOutput::Function {
            body: FunctionCallOutputBody::Text(content),
            ..
        } = output
        else {
            panic!("expected function output");
        };
        let result: SpawnAgentResult =
            serde_json::from_str(&content).expect("spawn_agent result should be json");
        let agent_id = agent_id(&result.agent_id).expect("agent_id should be valid");
        assert!(
            result
                .nickname
                .as_deref()
                .is_some_and(|nickname| !nickname.is_empty())
        );
        let snapshot = manager
            .get_thread(agent_id)
            .await
            .expect("spawned agent thread should exist")
            .config_snapshot()
            .await;
        assert_eq!(snapshot.approval_policy, AskForApproval::OnRequest);
        assert_eq!(result.thread_note, None);
    }

    #[tokio::test]
    async fn spawn_agent_propagates_thread_note_to_result_and_snapshot() {
        #[derive(Debug, Deserialize)]
        struct SpawnAgentResult {
            agent_id: String,
            thread_note: Option<String>,
        }

        let (mut session, turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();

        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "spawn_agent",
            function_payload(json!({
                "message": "inspect this repo",
                "thread_note": "  keep this task scoped to repo metadata  "
            })),
        );
        let output = MultiAgentHandler
            .handle(invocation)
            .await
            .expect("spawn_agent should succeed");
        let ToolOutput::Function {
            body: FunctionCallOutputBody::Text(content),
            ..
        } = output
        else {
            panic!("expected function output");
        };
        let result: SpawnAgentResult =
            serde_json::from_str(&content).expect("spawn_agent result should be json");
        let agent_id = agent_id(&result.agent_id).expect("agent_id should be valid");
        assert_eq!(
            result.thread_note,
            Some("keep this task scoped to repo metadata".to_string())
        );

        let snapshot = manager
            .get_thread(agent_id)
            .await
            .expect("spawned agent thread should exist")
            .config_snapshot()
            .await;
        assert_eq!(
            snapshot.session_source.get_thread_note(),
            Some("keep this task scoped to repo metadata".to_string())
        );
    }

    #[tokio::test]
    async fn spawn_agent_errors_when_manager_dropped() {
        let (session, turn) = make_session_and_context().await;
        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "spawn_agent",
            function_payload(json!({"message": "hello"})),
        );
        let Err(err) = MultiAgentHandler.handle(invocation).await else {
            panic!("spawn should fail without a manager");
        };
        assert_eq!(
            err,
            FunctionCallError::RespondToModel("collab manager unavailable".to_string())
        );
    }

    #[tokio::test]
    async fn spawn_agent_reapplies_runtime_sandbox_after_role_config() {
        fn pick_allowed_sandbox_policy(
            constraint: &crate::config::Constrained<SandboxPolicy>,
            base: SandboxPolicy,
        ) -> SandboxPolicy {
            let candidates = [
                SandboxPolicy::DangerFullAccess,
                SandboxPolicy::new_workspace_write_policy(),
                SandboxPolicy::new_read_only_policy(),
            ];
            candidates
                .into_iter()
                .find(|candidate| *candidate != base && constraint.can_set(candidate).is_ok())
                .unwrap_or(base)
        }

        #[derive(Debug, Deserialize)]
        struct SpawnAgentResult {
            agent_id: String,
            nickname: Option<String>,
        }

        let (mut session, mut turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();
        let expected_sandbox = pick_allowed_sandbox_policy(
            &turn.config.permissions.sandbox_policy,
            turn.config.permissions.sandbox_policy.get().clone(),
        );
        turn.approval_policy
            .set(AskForApproval::OnRequest)
            .expect("approval policy should be set");
        turn.sandbox_policy
            .set(expected_sandbox.clone())
            .expect("sandbox policy should be set");
        assert_ne!(
            expected_sandbox,
            turn.config.permissions.sandbox_policy.get().clone(),
            "test requires a runtime sandbox override that differs from base config"
        );

        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "spawn_agent",
            function_payload(json!({
                "message": "await this command",
                "agent_type": "awaiter"
            })),
        );
        let output = MultiAgentHandler
            .handle(invocation)
            .await
            .expect("spawn_agent should succeed");
        let ToolOutput::Function {
            body: FunctionCallOutputBody::Text(content),
            ..
        } = output
        else {
            panic!("expected function output");
        };
        let result: SpawnAgentResult =
            serde_json::from_str(&content).expect("spawn_agent result should be json");
        let agent_id = agent_id(&result.agent_id).expect("agent_id should be valid");
        assert!(
            result
                .nickname
                .as_deref()
                .is_some_and(|nickname| !nickname.is_empty())
        );

        let snapshot = manager
            .get_thread(agent_id)
            .await
            .expect("spawned agent thread should exist")
            .config_snapshot()
            .await;
        assert_eq!(snapshot.sandbox_policy, expected_sandbox);
        assert_eq!(snapshot.approval_policy, AskForApproval::OnRequest);
    }

    #[tokio::test]
    async fn spawn_agent_rejects_when_depth_limit_exceeded() {
        let (mut session, mut turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();

        let max_depth = turn.config.agent_max_depth;
        turn.session_source = SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id: session.conversation_id,
            depth: max_depth,
            agent_nickname: None,
            agent_role: None,
            agent_persona: None,
            thread_note: None,
            allow_list: None,
            deny_list: None,
        });

        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "spawn_agent",
            function_payload(json!({"message": "hello"})),
        );
        let Err(err) = MultiAgentHandler.handle(invocation).await else {
            panic!("spawn should fail when depth limit exceeded");
        };
        assert_eq!(
            err,
            FunctionCallError::RespondToModel(
                "Agent depth limit reached. Solve the task yourself.".to_string()
            )
        );
    }

    #[tokio::test]
    async fn spawn_agent_allows_depth_up_to_configured_max_depth() {
        #[derive(Debug, Deserialize)]
        struct SpawnAgentResult {
            agent_id: String,
            nickname: Option<String>,
        }

        let (mut session, mut turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();

        let mut config = (*turn.config).clone();
        config.agent_max_depth = DEFAULT_AGENT_MAX_DEPTH + 1;
        turn.config = Arc::new(config);
        turn.session_source = SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id: session.conversation_id,
            depth: DEFAULT_AGENT_MAX_DEPTH,
            agent_nickname: None,
            agent_role: None,
            agent_persona: None,
            thread_note: None,
            allow_list: None,
            deny_list: None,
        });

        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "spawn_agent",
            function_payload(json!({"message": "hello"})),
        );
        let output = MultiAgentHandler
            .handle(invocation)
            .await
            .expect("spawn should succeed within configured depth");
        let ToolOutput::Function {
            body: FunctionCallOutputBody::Text(content),
            success,
            ..
        } = output
        else {
            panic!("expected function output");
        };
        let result: SpawnAgentResult =
            serde_json::from_str(&content).expect("spawn_agent result should be json");
        assert!(!result.agent_id.is_empty());
        assert!(
            result
                .nickname
                .as_deref()
                .is_some_and(|nickname| !nickname.is_empty())
        );
        assert_eq!(success, Some(true));
    }

    #[tokio::test]
    async fn spawn_agent_rejects_unknown_template_nickname() {
        let (mut session, turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();

        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "spawn_agent",
            function_payload(json!({
                "message": "inspect this repo",
                "agent_type": "explorer",
                "agent_nickname": "runner"
            })),
        );
        let Err(err) = MultiAgentHandler.handle(invocation).await else {
            panic!("spawn should fail for unknown agent_nickname");
        };
        assert_eq!(
            err,
            FunctionCallError::RespondToModel(
                "unknown agent_nickname 'runner' for agent_type 'explorer'".to_string()
            )
        );
    }

    #[tokio::test]
    async fn spawn_agent_accepts_template_only_role_without_declared_config() {
        #[derive(Debug, Deserialize)]
        struct SpawnAgentResult {
            agent_id: String,
            nickname: Option<String>,
        }

        let temp_dir = TempDir::new().expect("create tempdir");
        write_role_template(
            &temp_dir,
            "orchestrator",
            r#"---
description: Orchestrator role from project templates
read_only: true
agent_names:
  - name: default
    description: default orchestrator persona
allow_list: ["wait", "send_input"]
deny_list: ["apply_patch"]
---
<!-- agent_nickname: default -->
Coordinate sub-agents and report concise status updates.
"#,
        );

        let (mut session, mut turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();
        turn.cwd = temp_dir.path().to_path_buf();

        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "spawn_agent",
            function_payload(json!({
                "message": "coordinate this task",
                "agent_type": "orchestrator"
            })),
        );
        let output = MultiAgentHandler
            .handle(invocation)
            .await
            .expect("spawn should succeed for template-only role");
        let ToolOutput::Function {
            body: FunctionCallOutputBody::Text(content),
            ..
        } = output
        else {
            panic!("expected function output");
        };
        let result: SpawnAgentResult =
            serde_json::from_str(&content).expect("spawn_agent result should be json");
        let agent_id = agent_id(&result.agent_id).expect("agent_id should be valid");
        assert!(
            result
                .nickname
                .as_deref()
                .is_some_and(|nickname| !nickname.is_empty())
        );

        let snapshot = manager
            .get_thread(agent_id)
            .await
            .expect("spawned agent thread should exist")
            .config_snapshot()
            .await;

        match snapshot.session_source {
            SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                agent_role,
                allow_list,
                deny_list,
                ..
            }) => {
                assert_eq!(agent_role, Some("orchestrator".to_string()));
                assert_eq!(
                    allow_list,
                    Some(vec!["send_input".to_string(), "wait".to_string()])
                );
                assert_eq!(deny_list, Some(vec!["apply_patch".to_string()]));
            }
            source => panic!("expected ThreadSpawn session source, got {source:?}"),
        }
        assert_eq!(
            snapshot.sandbox_policy,
            SandboxPolicy::new_read_only_policy()
        );
    }

    #[tokio::test]
    async fn spawn_agent_accepts_template_alias_and_emits_canonical_role() {
        #[derive(Debug, Deserialize)]
        struct SpawnAgentResult {
            agent_id: String,
            nickname: Option<String>,
        }

        let (mut session, turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();

        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "spawn_agent",
            function_payload(json!({
                "message": "inspect this repo",
                "agent_type": "Explorer"
            })),
        );
        let output = MultiAgentHandler
            .handle(invocation)
            .await
            .expect("spawn should succeed for template alias");
        let ToolOutput::Function {
            body: FunctionCallOutputBody::Text(content),
            ..
        } = output
        else {
            panic!("expected function output");
        };
        let result: SpawnAgentResult =
            serde_json::from_str(&content).expect("spawn_agent result should be json");
        let agent_id = agent_id(&result.agent_id).expect("agent_id should be valid");
        assert!(
            result
                .nickname
                .as_deref()
                .is_some_and(|nickname| !nickname.is_empty())
        );

        let snapshot = manager
            .get_thread(agent_id)
            .await
            .expect("spawned agent thread should exist")
            .config_snapshot()
            .await;

        match snapshot.session_source {
            SessionSource::SubAgent(SubAgentSource::ThreadSpawn { agent_role, .. }) => {
                assert_eq!(agent_role, Some("explorer".to_string()));
            }
            source => panic!("expected ThreadSpawn session source, got {source:?}"),
        }
    }

    #[tokio::test]
    async fn spawn_agent_applies_template_policy_and_runtime_overrides() {
        #[derive(Debug, Deserialize)]
        struct SpawnAgentResult {
            agent_id: String,
            nickname: Option<String>,
        }

        let temp_dir = TempDir::new().expect("create tempdir");
        write_role_template(
            &temp_dir,
            "worker",
            r#"---
description: Worker template from project
read_only: true
agent_names:
  - name: default
    description: default worker persona
allow_list: ["wait", "spawn_agent", "wait"]
deny_list: ["send_input"]
---
<!-- agent_nickname: default -->
Use this worker template prompt.
"#,
        );

        let (mut session, mut turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();
        turn.cwd = temp_dir.path().to_path_buf();

        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "spawn_agent",
            function_payload(json!({
                "message": "implement this task",
                "agent_type": "worker"
            })),
        );
        let output = MultiAgentHandler
            .handle(invocation)
            .await
            .expect("spawn should succeed");
        let ToolOutput::Function {
            body: FunctionCallOutputBody::Text(content),
            ..
        } = output
        else {
            panic!("expected function output");
        };
        let result: SpawnAgentResult =
            serde_json::from_str(&content).expect("spawn_agent result should be json");
        let agent_id = agent_id(&result.agent_id).expect("agent_id should be valid");
        assert!(
            result
                .nickname
                .as_deref()
                .is_some_and(|nickname| !nickname.is_empty())
        );

        let snapshot = manager
            .get_thread(agent_id)
            .await
            .expect("spawned agent thread should exist")
            .config_snapshot()
            .await;

        match snapshot.session_source {
            SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                agent_role,
                allow_list,
                deny_list,
                ..
            }) => {
                assert_eq!(agent_role, Some("worker".to_string()));
                assert_eq!(
                    allow_list,
                    Some(vec!["spawn_agent".to_string(), "wait".to_string()])
                );
                assert_eq!(deny_list, Some(vec!["send_input".to_string()]));
            }
            source => panic!("expected ThreadSpawn session source, got {source:?}"),
        }
        assert_eq!(
            snapshot.sandbox_policy,
            SandboxPolicy::new_read_only_policy()
        );
    }

    #[tokio::test]
    async fn spawn_agent_applies_template_model_and_reasoning_effort() {
        #[derive(Debug, Deserialize)]
        struct SpawnAgentResult {
            agent_id: String,
            requested_model: Option<String>,
            model_source: String,
            model: Option<String>,
            reasoning_effort: Option<codex_protocol::openai_models::ReasoningEffort>,
        }

        let temp_dir = TempDir::new().expect("create tempdir");
        write_role_template(
            &temp_dir,
            "worker",
            r#"---
description: Worker template from project
model: gpt-5.2-codex
reasoning_effort: low
agent_names:
  - name: default
    description: default worker persona
  - name: Runner
    description: run long commands
    model: gpt-5.3-codex
    reasoning_effort: high
---
<!-- agent_nickname: default -->
Use this worker template prompt.
<!-- agent_nickname: runner -->
Use this runner template prompt.
"#,
        );

        let (mut session, mut turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();
        turn.cwd = temp_dir.path().to_path_buf();

        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "spawn_agent",
            function_payload(json!({
                "message": "run cargo test",
                "agent_type": "worker",
                "agent_nickname": "runner"
            })),
        );
        let output = MultiAgentHandler
            .handle(invocation)
            .await
            .expect("spawn should succeed");
        let ToolOutput::Function {
            body: FunctionCallOutputBody::Text(content),
            ..
        } = output
        else {
            panic!("expected function output");
        };
        let result: SpawnAgentResult =
            serde_json::from_str(&content).expect("spawn_agent result should be json");
        let agent_id = agent_id(&result.agent_id).expect("agent_id should be valid");

        let snapshot = manager
            .get_thread(agent_id)
            .await
            .expect("spawned agent thread should exist")
            .config_snapshot()
            .await;
        assert_eq!(snapshot.model, "gpt-5.3-codex");
        assert_eq!(
            snapshot.reasoning_effort,
            Some(codex_protocol::openai_models::ReasoningEffort::High)
        );
        assert_eq!(result.model, Some("gpt-5.3-codex".to_string()));
        assert_eq!(
            result.reasoning_effort,
            Some(codex_protocol::openai_models::ReasoningEffort::High)
        );
        assert_eq!(result.requested_model, None);
        assert_eq!(result.model_source, "template_persona");

        match snapshot.session_source {
            SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                agent_persona,
                agent_role,
                ..
            }) => {
                assert_eq!(agent_persona, Some("Runner".to_string()));
                assert_eq!(agent_role, Some("worker".to_string()));
            }
            source => panic!("expected ThreadSpawn session source, got {source:?}"),
        }
    }

    #[tokio::test]
    async fn spawn_agent_explicit_model_and_reasoning_override_take_priority() {
        #[derive(Debug, Deserialize)]
        struct SpawnAgentResult {
            agent_id: String,
            requested_model: Option<String>,
            model_source: String,
            model: Option<String>,
            reasoning_effort: Option<codex_protocol::openai_models::ReasoningEffort>,
        }

        let temp_dir = TempDir::new().expect("create tempdir");
        write_role_template(
            &temp_dir,
            "worker",
            r#"---
description: Worker template from project
model: gpt-5.2-codex
reasoning_effort: low
agent_names:
  - name: default
    description: default worker persona
  - name: Runner
    description: run long commands
    model: gpt-5.3-codex-spark
    reasoning_effort: high
---
<!-- agent_nickname: default -->
Use this worker template prompt.
<!-- agent_nickname: runner -->
Use this runner template prompt.
"#,
        );

        let (mut session, mut turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();
        turn.cwd = temp_dir.path().to_path_buf();

        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "spawn_agent",
            function_payload(json!({
                "message": "run cargo test",
                "agent_type": "worker",
                "agent_nickname": "runner",
                "model": "gpt-5-codex",
                "reasoning_effort": "medium"
            })),
        );
        let output = MultiAgentHandler
            .handle(invocation)
            .await
            .expect("spawn should succeed");
        let ToolOutput::Function {
            body: FunctionCallOutputBody::Text(content),
            ..
        } = output
        else {
            panic!("expected function output");
        };
        let result: SpawnAgentResult =
            serde_json::from_str(&content).expect("spawn_agent result should be json");
        let agent_id = agent_id(&result.agent_id).expect("agent_id should be valid");

        let snapshot = manager
            .get_thread(agent_id)
            .await
            .expect("spawned agent thread should exist")
            .config_snapshot()
            .await;
        assert_eq!(snapshot.model, "gpt-5-codex");
        assert_eq!(
            snapshot.reasoning_effort,
            Some(codex_protocol::openai_models::ReasoningEffort::Medium)
        );
        assert_eq!(result.model, Some("gpt-5-codex".to_string()));
        assert_eq!(
            result.reasoning_effort,
            Some(codex_protocol::openai_models::ReasoningEffort::Medium)
        );
        assert_eq!(result.requested_model, Some("gpt-5-codex".to_string()));
        assert_eq!(result.model_source, "explicit_argument");
    }

    #[tokio::test]
    async fn spawn_agent_without_model_uses_parent_model_source() {
        #[derive(Debug, Deserialize)]
        struct SpawnAgentResult {
            agent_id: String,
            requested_model: Option<String>,
            model_source: String,
            model: Option<String>,
        }

        let temp_dir = TempDir::new().expect("create tempdir");
        write_role_template(
            &temp_dir,
            "worker",
            r#"---
description: Worker template from project
agent_names:
  - name: default
    description: default worker persona
---
<!-- agent_nickname: default -->
Use this worker template prompt.
"#,
        );

        let (mut session, mut turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();
        turn.cwd = temp_dir.path().to_path_buf();
        let expected_parent_model = turn.model_info.slug.clone();

        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "spawn_agent",
            function_payload(json!({
                "message": "run cargo test",
                "agent_type": "worker"
            })),
        );
        let output = MultiAgentHandler
            .handle(invocation)
            .await
            .expect("spawn should succeed");
        let ToolOutput::Function {
            body: FunctionCallOutputBody::Text(content),
            ..
        } = output
        else {
            panic!("expected function output");
        };
        let result: SpawnAgentResult =
            serde_json::from_str(&content).expect("spawn_agent result should be json");
        let agent_id = agent_id(&result.agent_id).expect("agent_id should be valid");
        assert_eq!(result.requested_model, None);
        assert_eq!(result.model_source, "inherited_parent");
        assert_eq!(result.model, Some(expected_parent_model.clone()));

        let snapshot = manager
            .get_thread(agent_id)
            .await
            .expect("spawned agent thread should exist")
            .config_snapshot()
            .await;
        assert_eq!(snapshot.model, expected_parent_model);
    }

    #[tokio::test]
    async fn spawn_agent_model_does_not_follow_parent_model_updates() {
        #[derive(Debug, Deserialize)]
        struct SpawnAgentResult {
            agent_id: String,
            model: Option<String>,
        }

        let (mut session, turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();

        let parent_model_before = turn.model_info.slug.clone();
        let new_parent_model = session
            .services
            .models_manager
            .try_list_models()
            .expect("model catalog should be available")
            .into_iter()
            .map(|preset| preset.model)
            .find(|model| model != &parent_model_before)
            .expect("catalog should include an alternate model");

        let session = Arc::new(session);
        let turn = Arc::new(turn);
        let invocation = invocation(
            Arc::clone(&session),
            Arc::clone(&turn),
            "spawn_agent",
            function_payload(json!({
                "message": "run cargo test",
                "agent_type": "worker"
            })),
        );
        let output = MultiAgentHandler
            .handle(invocation)
            .await
            .expect("spawn should succeed");
        let ToolOutput::Function {
            body: FunctionCallOutputBody::Text(content),
            ..
        } = output
        else {
            panic!("expected function output");
        };
        let result: SpawnAgentResult =
            serde_json::from_str(&content).expect("spawn_agent result should be json");
        let agent_id = agent_id(&result.agent_id).expect("agent_id should be valid");
        assert_eq!(result.model, Some(parent_model_before.clone()));

        let current_mode = session.collaboration_mode().await;
        let updated_mode = current_mode.with_updates(Some(new_parent_model.clone()), None, None);
        session
            .update_settings(crate::codex::SessionSettingsUpdate {
                collaboration_mode: Some(updated_mode),
                ..Default::default()
            })
            .await
            .expect("parent model override should apply");

        timeout(Duration::from_secs(5), async {
            loop {
                let current_turn = session.new_default_turn().await;
                if current_turn.model_info.slug == new_parent_model {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("parent model should update");

        let child_snapshot = manager
            .get_thread(agent_id)
            .await
            .expect("spawned agent thread should exist")
            .config_snapshot()
            .await;
        assert_eq!(child_snapshot.model, parent_model_before);
    }

    #[tokio::test]
    async fn spawn_agent_rejects_unknown_model_override() {
        let (mut session, turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();

        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "spawn_agent",
            function_payload(json!({
                "message": "hello",
                "model": "gpt-5.999-codex-invalid"
            })),
        );
        let Err(err) = MultiAgentHandler.handle(invocation).await else {
            panic!("spawn should fail");
        };
        let FunctionCallError::RespondToModel(message) = err else {
            panic!("expected respond-to-model error");
        };
        assert!(
            message.contains("unknown model"),
            "expected unknown model message, got: {message}"
        );
        assert!(
            message.contains("Available models:"),
            "expected available models list, got: {message}"
        );
    }

    #[tokio::test]
    async fn spawn_agent_rejects_unknown_reasoning_effort_with_model_specific_list() {
        let (mut session, turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();

        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "spawn_agent",
            function_payload(json!({
                "message": "hello",
                "model": "gpt-5.3-codex",
                "reasoning_effort": "ultra"
            })),
        );
        let Err(err) = MultiAgentHandler.handle(invocation).await else {
            panic!("spawn should fail");
        };
        let FunctionCallError::RespondToModel(message) = err else {
            panic!("expected respond-to-model error");
        };
        assert!(
            message.contains("\"gpt-5.3-codex\""),
            "expected model-specific error, got: {message}"
        );
        assert!(
            message.contains("Supported efforts: low, medium, high, xhigh"),
            "expected model-specific supported list, got: {message}"
        );
    }

    #[tokio::test]
    async fn spawn_agent_rejects_unsupported_reasoning_effort_for_model() {
        let (mut session, turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();

        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "spawn_agent",
            function_payload(json!({
                "message": "hello",
                "model": "gpt-5.1-codex-mini",
                "reasoning_effort": "xhigh"
            })),
        );
        let Err(err) = MultiAgentHandler.handle(invocation).await else {
            panic!("spawn should fail");
        };
        let FunctionCallError::RespondToModel(message) = err else {
            panic!("expected respond-to-model error");
        };
        assert!(
            message.contains("reasoning_effort"),
            "expected unsupported reasoning message, got: {message}"
        );
        assert!(
            message.contains("\"gpt-5.1-codex-mini\""),
            "expected model slug in error, got: {message}"
        );
    }

    #[tokio::test]
    async fn spawn_agent_rejects_none_reasoning_effort_when_model_does_not_support_it() {
        let (mut session, turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();

        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "spawn_agent",
            function_payload(json!({
                "message": "hello",
                "model": "gpt-5.1-codex-mini",
                "reasoning_effort": "none"
            })),
        );
        let Err(err) = MultiAgentHandler.handle(invocation).await else {
            panic!("spawn should fail");
        };
        let FunctionCallError::RespondToModel(message) = err else {
            panic!("expected respond-to-model error");
        };
        assert!(
            message.contains("reasoning_effort"),
            "expected unsupported reasoning message, got: {message}"
        );
        assert!(
            message.contains("Supported efforts: medium, high"),
            "expected model-supported list, got: {message}"
        );
    }

    #[tokio::test]
    async fn send_input_rejects_empty_message() {
        let (session, turn) = make_session_and_context().await;
        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "send_input",
            function_payload(json!({"id": ThreadId::new().to_string(), "message": ""})),
        );
        let Err(err) = MultiAgentHandler.handle(invocation).await else {
            panic!("empty message should be rejected");
        };
        assert_eq!(
            err,
            FunctionCallError::RespondToModel(
                "Empty message can't be sent to an agent".to_string()
            )
        );
    }

    #[tokio::test]
    async fn send_input_rejects_when_message_and_items_are_both_set() {
        let (session, turn) = make_session_and_context().await;
        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "send_input",
            function_payload(json!({
                "id": ThreadId::new().to_string(),
                "message": "hello",
                "items": [{"type": "mention", "name": "drive", "path": "app://drive"}]
            })),
        );
        let Err(err) = MultiAgentHandler.handle(invocation).await else {
            panic!("message+items should be rejected");
        };
        assert_eq!(
            err,
            FunctionCallError::RespondToModel(
                "Provide either message or items, but not both".to_string()
            )
        );
    }

    #[tokio::test]
    async fn send_input_rejects_invalid_id() {
        let (session, turn) = make_session_and_context().await;
        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "send_input",
            function_payload(json!({"id": "not-a-uuid", "message": "hi"})),
        );
        let Err(err) = MultiAgentHandler.handle(invocation).await else {
            panic!("invalid id should be rejected");
        };
        let FunctionCallError::RespondToModel(msg) = err else {
            panic!("expected respond-to-model error");
        };
        assert!(msg.starts_with("invalid agent id not-a-uuid:"));
    }

    #[tokio::test]
    async fn send_input_reports_missing_agent() {
        let (mut session, turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();
        let agent_id = ThreadId::new();
        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "send_input",
            function_payload(json!({"id": agent_id.to_string(), "message": "hi"})),
        );
        let Err(err) = MultiAgentHandler.handle(invocation).await else {
            panic!("missing agent should be reported");
        };
        assert_eq!(
            err,
            FunctionCallError::RespondToModel(format!("agent with id {agent_id} not found"))
        );
    }

    #[tokio::test]
    async fn send_input_interrupts_before_prompt() {
        let (mut session, turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();
        let config = turn.config.as_ref().clone();
        let thread = manager.start_thread(config).await.expect("start thread");
        let agent_id = thread.thread_id;
        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "send_input",
            function_payload(json!({
                "id": agent_id.to_string(),
                "message": "hi",
                "interrupt": true
            })),
        );
        MultiAgentHandler
            .handle(invocation)
            .await
            .expect("send_input should succeed");

        let ops = manager.captured_ops();
        let ops_for_agent: Vec<&Op> = ops
            .iter()
            .filter_map(|(id, op)| (*id == agent_id).then_some(op))
            .collect();
        assert_eq!(ops_for_agent.len(), 2);
        assert!(matches!(ops_for_agent[0], Op::Interrupt));
        assert!(matches!(ops_for_agent[1], Op::UserInput { .. }));

        let _ = thread
            .thread
            .submit(Op::Shutdown {})
            .await
            .expect("shutdown should submit");
    }

    #[tokio::test]
    async fn send_input_accepts_structured_items() {
        let (mut session, turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();
        let config = turn.config.as_ref().clone();
        let thread = manager.start_thread(config).await.expect("start thread");
        let agent_id = thread.thread_id;
        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "send_input",
            function_payload(json!({
                "id": agent_id.to_string(),
                "items": [
                    {"type": "mention", "name": "drive", "path": "app://google_drive"},
                    {"type": "text", "text": "read the folder"}
                ]
            })),
        );
        MultiAgentHandler
            .handle(invocation)
            .await
            .expect("send_input should succeed");

        let expected = Op::UserInput {
            items: vec![
                UserInput::Mention {
                    name: "drive".to_string(),
                    path: "app://google_drive".to_string(),
                },
                UserInput::Text {
                    text: "read the folder".to_string(),
                    text_elements: Vec::new(),
                },
            ],
            final_output_json_schema: None,
        };
        let captured = manager
            .captured_ops()
            .into_iter()
            .find(|(id, op)| *id == agent_id && *op == expected);
        assert_eq!(captured, Some((agent_id, expected)));

        let _ = thread
            .thread
            .submit(Op::Shutdown {})
            .await
            .expect("shutdown should submit");
    }

    #[tokio::test]
    async fn set_thread_note_rejects_invalid_id() {
        let (session, turn) = make_session_and_context().await;
        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "set_thread_note",
            function_payload(json!({"id": "not-a-uuid", "note": "worker"})),
        );
        let Err(err) = MultiAgentHandler.handle(invocation).await else {
            panic!("invalid id should be rejected");
        };
        let FunctionCallError::RespondToModel(msg) = err else {
            panic!("expected respond-to-model error");
        };
        assert!(msg.starts_with("invalid agent id not-a-uuid:"));
    }

    #[tokio::test]
    async fn set_thread_note_reports_missing_agent() {
        let (mut session, turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();
        let agent_id = ThreadId::new();
        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "set_thread_note",
            function_payload(json!({"id": agent_id.to_string(), "note": "worker"})),
        );
        let Err(err) = MultiAgentHandler.handle(invocation).await else {
            panic!("missing agent should be reported");
        };
        assert_eq!(
            err,
            FunctionCallError::RespondToModel(format!("agent with id {agent_id} not found"))
        );
    }

    #[tokio::test]
    async fn set_thread_note_submits_update_and_clear() {
        let (mut session, turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();
        let config = turn.config.as_ref().clone();
        let thread = manager.start_thread(config).await.expect("start thread");
        let agent_id = thread.thread_id;
        let session = Arc::new(session);
        let turn = Arc::new(turn);

        let set_invocation = invocation(
            session.clone(),
            turn.clone(),
            "set_thread_note",
            function_payload(json!({"id": agent_id.to_string(), "note": "  worker note  "})),
        );
        let output = MultiAgentHandler
            .handle(set_invocation)
            .await
            .expect("set_thread_note should succeed");
        let ToolOutput::Function {
            body: FunctionCallOutputBody::Text(content),
            success,
            ..
        } = output
        else {
            panic!("expected function output");
        };
        let result: serde_json::Value =
            serde_json::from_str(&content).expect("set_thread_note result should be json");
        assert_eq!(
            result.get("thread_note").and_then(|v| v.as_str()),
            Some("worker note")
        );
        assert_eq!(success, Some(true));

        let clear_invocation = invocation(
            session,
            turn,
            "set_thread_note",
            function_payload(json!({"id": agent_id.to_string(), "note": "   "})),
        );
        MultiAgentHandler
            .handle(clear_invocation)
            .await
            .expect("clear thread note should succeed");

        let ops = manager.captured_ops();
        let set_seen = ops.iter().any(|(id, op)| {
            *id == agent_id
                && *op
                    == Op::SetThreadNote {
                        note: Some("worker note".to_string()),
                    }
        });
        let clear_seen = ops
            .iter()
            .any(|(id, op)| *id == agent_id && *op == Op::SetThreadNote { note: None });
        assert!(set_seen, "expected normalized SetThreadNote op");
        assert!(clear_seen, "expected clear SetThreadNote op");

        let _ = thread
            .thread
            .submit(Op::Shutdown {})
            .await
            .expect("shutdown should submit");
    }

    #[tokio::test]
    async fn resume_agent_rejects_invalid_id() {
        let (session, turn) = make_session_and_context().await;
        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "resume_agent",
            function_payload(json!({"id": "not-a-uuid"})),
        );
        let Err(err) = MultiAgentHandler.handle(invocation).await else {
            panic!("invalid id should be rejected");
        };
        let FunctionCallError::RespondToModel(msg) = err else {
            panic!("expected respond-to-model error");
        };
        assert!(msg.starts_with("invalid agent id not-a-uuid:"));
    }

    #[tokio::test]
    async fn resume_agent_reports_missing_agent() {
        let (mut session, turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();
        let agent_id = ThreadId::new();
        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "resume_agent",
            function_payload(json!({"id": agent_id.to_string()})),
        );
        let Err(err) = MultiAgentHandler.handle(invocation).await else {
            panic!("missing agent should be reported");
        };
        assert_eq!(
            err,
            FunctionCallError::RespondToModel(format!("agent with id {agent_id} not found"))
        );
    }

    #[tokio::test]
    async fn resume_agent_noops_for_active_agent() {
        let (mut session, turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();
        let config = turn.config.as_ref().clone();
        let thread = manager.start_thread(config).await.expect("start thread");
        let agent_id = thread.thread_id;
        let status_before = manager.agent_control().get_status(agent_id).await;
        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "resume_agent",
            function_payload(json!({"id": agent_id.to_string()})),
        );

        let output = MultiAgentHandler
            .handle(invocation)
            .await
            .expect("resume_agent should succeed");
        let ToolOutput::Function {
            body: FunctionCallOutputBody::Text(content),
            success,
            ..
        } = output
        else {
            panic!("expected function output");
        };
        let result: resume_agent::ResumeAgentResult =
            serde_json::from_str(&content).expect("resume_agent result should be json");
        assert_eq!(result.status, status_before);
        assert_eq!(success, Some(true));

        let thread_ids = manager.list_thread_ids().await;
        assert_eq!(thread_ids, vec![agent_id]);

        let _ = thread
            .thread
            .submit(Op::Shutdown {})
            .await
            .expect("shutdown should submit");
    }

    #[tokio::test]
    async fn resume_agent_restores_closed_agent_and_accepts_send_input() {
        let (mut session, turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();
        let config = turn.config.as_ref().clone();
        let thread = manager
            .resume_thread_with_history(
                config,
                InitialHistory::Forked(vec![RolloutItem::ResponseItem(ResponseItem::Message {
                    id: None,
                    role: "user".to_string(),
                    content: vec![ContentItem::InputText {
                        text: "materialized".to_string(),
                    }],
                    end_turn: None,
                    phase: None,
                })]),
                AuthManager::from_auth_for_testing(CodexAuth::from_api_key("dummy")),
                false,
            )
            .await
            .expect("start thread");
        let agent_id = thread.thread_id;
        let _ = manager
            .agent_control()
            .shutdown_agent(agent_id)
            .await
            .expect("shutdown agent");
        assert_eq!(
            manager.agent_control().get_status(agent_id).await,
            AgentStatus::NotFound
        );
        let session = Arc::new(session);
        let turn = Arc::new(turn);

        let resume_invocation = invocation(
            session.clone(),
            turn.clone(),
            "resume_agent",
            function_payload(json!({"id": agent_id.to_string()})),
        );
        let output = MultiAgentHandler
            .handle(resume_invocation)
            .await
            .expect("resume_agent should succeed");
        let ToolOutput::Function {
            body: FunctionCallOutputBody::Text(content),
            success,
            ..
        } = output
        else {
            panic!("expected function output");
        };
        let result: resume_agent::ResumeAgentResult =
            serde_json::from_str(&content).expect("resume_agent result should be json");
        assert_ne!(result.status, AgentStatus::NotFound);
        assert_eq!(success, Some(true));

        let send_invocation = invocation(
            session,
            turn,
            "send_input",
            function_payload(json!({"id": agent_id.to_string(), "message": "hello"})),
        );
        let output = MultiAgentHandler
            .handle(send_invocation)
            .await
            .expect("send_input should succeed after resume");
        let ToolOutput::Function {
            body: FunctionCallOutputBody::Text(content),
            success,
            ..
        } = output
        else {
            panic!("expected function output");
        };
        let result: serde_json::Value =
            serde_json::from_str(&content).expect("send_input result should be json");
        let submission_id = result
            .get("submission_id")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        assert!(!submission_id.is_empty());
        assert_eq!(success, Some(true));

        let _ = manager
            .agent_control()
            .shutdown_agent(agent_id)
            .await
            .expect("shutdown resumed agent");
    }

    #[tokio::test]
    async fn resume_agent_rejects_when_depth_limit_exceeded() {
        let (mut session, mut turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();

        let max_depth = turn.config.agent_max_depth;
        turn.session_source = SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id: session.conversation_id,
            depth: max_depth,
            agent_nickname: None,
            agent_role: None,
            agent_persona: None,
            thread_note: None,
            allow_list: None,
            deny_list: None,
        });

        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "resume_agent",
            function_payload(json!({"id": ThreadId::new().to_string()})),
        );
        let Err(err) = MultiAgentHandler.handle(invocation).await else {
            panic!("resume should fail when depth limit exceeded");
        };
        assert_eq!(
            err,
            FunctionCallError::RespondToModel(
                "Agent depth limit reached. Solve the task yourself.".to_string()
            )
        );
    }

    #[tokio::test]
    async fn wait_rejects_non_positive_timeout() {
        let (session, turn) = make_session_and_context().await;
        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "wait",
            function_payload(json!({
                "ids": [ThreadId::new().to_string()],
                "timeout_ms": 0
            })),
        );
        let Err(err) = MultiAgentHandler.handle(invocation).await else {
            panic!("non-positive timeout should be rejected");
        };
        assert_eq!(
            err,
            FunctionCallError::RespondToModel("timeout_ms must be greater than zero".to_string())
        );
    }

    #[tokio::test]
    async fn wait_rejects_invalid_id() {
        let (session, turn) = make_session_and_context().await;
        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "wait",
            function_payload(json!({"ids": ["invalid"]})),
        );
        let Err(err) = MultiAgentHandler.handle(invocation).await else {
            panic!("invalid id should be rejected");
        };
        let FunctionCallError::RespondToModel(msg) = err else {
            panic!("expected respond-to-model error");
        };
        assert!(msg.starts_with("invalid agent id invalid:"));
    }

    #[test]
    fn build_wait_agent_statuses_preserves_thread_note_metadata() {
        let thread_id = ThreadId::new();
        let statuses = HashMap::from([(thread_id, AgentStatus::Running)]);
        let receiver_agents = vec![CollabAgentRef {
            thread_id,
            agent_nickname: Some("atlas".to_string()),
            agent_role: Some("explorer".to_string()),
            agent_persona: None,
            thread_note: Some("focused on metadata sync".to_string()),
        }];

        let entries = build_wait_agent_statuses(&statuses, &receiver_agents);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].thread_id, thread_id);
        assert_eq!(
            entries[0].thread_note,
            Some("focused on metadata sync".to_string())
        );
    }

    #[tokio::test]
    async fn wait_rejects_empty_ids() {
        let (session, turn) = make_session_and_context().await;
        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "wait",
            function_payload(json!({"ids": []})),
        );
        let Err(err) = MultiAgentHandler.handle(invocation).await else {
            panic!("empty ids should be rejected");
        };
        assert_eq!(
            err,
            FunctionCallError::RespondToModel("ids must be non-empty".to_string())
        );
    }

    #[tokio::test]
    async fn wait_returns_not_found_for_missing_agents() {
        let (mut session, turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();
        let id_a = ThreadId::new();
        let id_b = ThreadId::new();
        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "wait",
            function_payload(json!({
                "ids": [id_a.to_string(), id_b.to_string()],
                "timeout_ms": 1000
            })),
        );
        let output = MultiAgentHandler
            .handle(invocation)
            .await
            .expect("wait should succeed");
        let ToolOutput::Function {
            body: FunctionCallOutputBody::Text(content),
            success,
            ..
        } = output
        else {
            panic!("expected function output");
        };
        let result: wait::WaitResult =
            serde_json::from_str(&content).expect("wait result should be json");
        assert_eq!(
            result,
            wait::WaitResult {
                status: HashMap::from([
                    (id_a, AgentStatus::NotFound),
                    (id_b, AgentStatus::NotFound),
                ]),
                timed_out: false
            }
        );
        assert_eq!(success, None);
    }

    #[tokio::test]
    async fn wait_times_out_when_status_is_not_final() {
        let (mut session, turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();
        let config = turn.config.as_ref().clone();
        let thread = manager.start_thread(config).await.expect("start thread");
        let agent_id = thread.thread_id;
        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "wait",
            function_payload(json!({
                "ids": [agent_id.to_string()],
                "timeout_ms": MIN_WAIT_TIMEOUT_MS
            })),
        );
        let output = MultiAgentHandler
            .handle(invocation)
            .await
            .expect("wait should succeed");
        let ToolOutput::Function {
            body: FunctionCallOutputBody::Text(content),
            success,
            ..
        } = output
        else {
            panic!("expected function output");
        };
        let result: wait::WaitResult =
            serde_json::from_str(&content).expect("wait result should be json");
        assert_eq!(
            result,
            wait::WaitResult {
                status: HashMap::new(),
                timed_out: true
            }
        );
        assert_eq!(success, None);

        let _ = thread
            .thread
            .submit(Op::Shutdown {})
            .await
            .expect("shutdown should submit");
    }

    #[tokio::test]
    async fn wait_clamps_short_timeouts_to_minimum() {
        let (mut session, turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();
        let config = turn.config.as_ref().clone();
        let thread = manager.start_thread(config).await.expect("start thread");
        let agent_id = thread.thread_id;
        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "wait",
            function_payload(json!({
                "ids": [agent_id.to_string()],
                "timeout_ms": 10
            })),
        );

        let early = timeout(
            Duration::from_millis(50),
            MultiAgentHandler.handle(invocation),
        )
        .await;
        assert!(
            early.is_err(),
            "wait should not return before the minimum timeout clamp"
        );

        let _ = thread
            .thread
            .submit(Op::Shutdown {})
            .await
            .expect("shutdown should submit");
    }

    #[tokio::test]
    async fn wait_returns_final_status_without_timeout() {
        let (mut session, turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();
        let config = turn.config.as_ref().clone();
        let thread = manager.start_thread(config).await.expect("start thread");
        let agent_id = thread.thread_id;
        let mut status_rx = manager
            .agent_control()
            .subscribe_status(agent_id)
            .await
            .expect("subscribe should succeed");

        let _ = thread
            .thread
            .submit(Op::Shutdown {})
            .await
            .expect("shutdown should submit");
        let _ = timeout(Duration::from_secs(1), status_rx.changed())
            .await
            .expect("shutdown status should arrive");

        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "wait",
            function_payload(json!({
                "ids": [agent_id.to_string()],
                "timeout_ms": 1000
            })),
        );
        let output = MultiAgentHandler
            .handle(invocation)
            .await
            .expect("wait should succeed");
        let ToolOutput::Function {
            body: FunctionCallOutputBody::Text(content),
            success,
            ..
        } = output
        else {
            panic!("expected function output");
        };
        let result: wait::WaitResult =
            serde_json::from_str(&content).expect("wait result should be json");
        assert_eq!(
            result,
            wait::WaitResult {
                status: HashMap::from([(agent_id, AgentStatus::Shutdown)]),
                timed_out: false
            }
        );
        assert_eq!(success, None);
    }

    #[tokio::test]
    async fn close_agent_submits_shutdown_and_returns_status() {
        let (mut session, turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();
        let config = turn.config.as_ref().clone();
        let thread = manager.start_thread(config).await.expect("start thread");
        let agent_id = thread.thread_id;
        let status_before = manager.agent_control().get_status(agent_id).await;

        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "close_agent",
            function_payload(json!({"id": agent_id.to_string()})),
        );
        let output = MultiAgentHandler
            .handle(invocation)
            .await
            .expect("close_agent should succeed");
        let ToolOutput::Function {
            body: FunctionCallOutputBody::Text(content),
            success,
            ..
        } = output
        else {
            panic!("expected function output");
        };
        let result: close_agent::CloseAgentResult =
            serde_json::from_str(&content).expect("close_agent result should be json");
        assert_eq!(result.status, status_before);
        assert_eq!(success, Some(true));

        let ops = manager.captured_ops();
        let submitted_shutdown = ops
            .iter()
            .any(|(id, op)| *id == agent_id && matches!(op, Op::Shutdown));
        assert_eq!(submitted_shutdown, true);

        let status_after = manager.agent_control().get_status(agent_id).await;
        assert_eq!(status_after, AgentStatus::NotFound);
    }

    #[tokio::test]
    async fn close_agent_cascades_to_descendants() {
        let (mut session, turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();
        let config = turn.config.as_ref().clone();
        let parent = manager
            .start_thread(config.clone())
            .await
            .expect("start parent");
        let parent_id = parent.thread_id;

        let child_id = manager
            .agent_control()
            .spawn_agent(
                config.clone(),
                vec![UserInput::Text {
                    text: "child".to_string(),
                    text_elements: Vec::new(),
                }],
                Some(thread_spawn_source(
                    parent_id, 1, None, None, None, None, None,
                )),
                None,
            )
            .await
            .expect("spawn child");
        let grandchild_id = manager
            .agent_control()
            .spawn_agent(
                config,
                vec![UserInput::Text {
                    text: "grandchild".to_string(),
                    text_elements: Vec::new(),
                }],
                Some(thread_spawn_source(
                    child_id, 2, None, None, None, None, None,
                )),
                None,
            )
            .await
            .expect("spawn grandchild");

        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            "close_agent",
            function_payload(json!({"id": parent_id.to_string()})),
        );
        let output = MultiAgentHandler
            .handle(invocation)
            .await
            .expect("close_agent should cascade");
        let ToolOutput::Function {
            body: FunctionCallOutputBody::Text(_),
            success,
            ..
        } = output
        else {
            panic!("expected function output");
        };
        assert_eq!(success, Some(true));

        let ops = manager.captured_ops();
        let submitted_parent_shutdown = ops
            .iter()
            .any(|(id, op)| *id == parent_id && matches!(op, Op::Shutdown));
        let submitted_child_shutdown = ops
            .iter()
            .any(|(id, op)| *id == child_id && matches!(op, Op::Shutdown));
        let submitted_grandchild_shutdown = ops
            .iter()
            .any(|(id, op)| *id == grandchild_id && matches!(op, Op::Shutdown));
        assert_eq!(submitted_parent_shutdown, true);
        assert_eq!(submitted_child_shutdown, true);
        assert_eq!(submitted_grandchild_shutdown, true);

        assert_eq!(
            manager.agent_control().get_status(parent_id).await,
            AgentStatus::NotFound
        );
        assert_eq!(
            manager.agent_control().get_status(child_id).await,
            AgentStatus::NotFound
        );
        assert_eq!(
            manager.agent_control().get_status(grandchild_id).await,
            AgentStatus::NotFound
        );
    }

    #[tokio::test]
    async fn close_agent_rejects_cross_subtree_shutdown_for_subagents() {
        let (mut session, mut turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();
        let config = turn.config.as_ref().clone();
        let parent = manager
            .start_thread(config.clone())
            .await
            .expect("start parent");
        let parent_id = parent.thread_id;

        let caller_id = manager
            .agent_control()
            .spawn_agent(
                config.clone(),
                vec![UserInput::Text {
                    text: "caller".to_string(),
                    text_elements: Vec::new(),
                }],
                Some(thread_spawn_source(
                    parent_id, 1, None, None, None, None, None,
                )),
                None,
            )
            .await
            .expect("spawn caller");
        let child_id = manager
            .agent_control()
            .spawn_agent(
                config.clone(),
                vec![UserInput::Text {
                    text: "child".to_string(),
                    text_elements: Vec::new(),
                }],
                Some(thread_spawn_source(
                    caller_id, 2, None, None, None, None, None,
                )),
                None,
            )
            .await
            .expect("spawn child");
        let sibling_id = manager
            .agent_control()
            .spawn_agent(
                config,
                vec![UserInput::Text {
                    text: "sibling".to_string(),
                    text_elements: Vec::new(),
                }],
                Some(thread_spawn_source(
                    parent_id, 1, None, None, None, None, None,
                )),
                None,
            )
            .await
            .expect("spawn sibling");

        session.conversation_id = caller_id;
        turn.session_source = SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id: parent_id,
            depth: 1,
            agent_nickname: None,
            agent_persona: None,
            agent_role: None,
            thread_note: None,
            allow_list: None,
            deny_list: None,
        });

        let session = Arc::new(session);
        let turn = Arc::new(turn);

        let sibling_invocation = invocation(
            Arc::clone(&session),
            Arc::clone(&turn),
            "close_agent",
            function_payload(json!({"id": sibling_id.to_string()})),
        );
        let Err(err) = MultiAgentHandler.handle(sibling_invocation).await else {
            panic!("close_agent should reject sibling termination");
        };
        assert_eq!(
            err,
            FunctionCallError::RespondToModel(
                "Not permitted to close agents outside your subtree.".to_string()
            )
        );
        assert_ne!(
            manager.agent_control().get_status(sibling_id).await,
            AgentStatus::NotFound
        );

        let child_invocation = invocation(
            Arc::clone(&session),
            Arc::clone(&turn),
            "close_agent",
            function_payload(json!({"id": child_id.to_string()})),
        );
        let output = MultiAgentHandler
            .handle(child_invocation)
            .await
            .expect("close_agent should allow descendant termination");
        let ToolOutput::Function {
            body: FunctionCallOutputBody::Text(_),
            success,
            ..
        } = output
        else {
            panic!("expected function output");
        };
        assert_eq!(success, Some(true));
        assert_eq!(
            manager.agent_control().get_status(child_id).await,
            AgentStatus::NotFound
        );
    }

    #[tokio::test]
    async fn build_agent_spawn_config_uses_turn_context_values() {
        fn pick_allowed_sandbox_policy(
            constraint: &crate::config::Constrained<SandboxPolicy>,
            base: SandboxPolicy,
        ) -> SandboxPolicy {
            let candidates = [
                SandboxPolicy::new_read_only_policy(),
                SandboxPolicy::new_workspace_write_policy(),
                SandboxPolicy::DangerFullAccess,
            ];
            candidates
                .into_iter()
                .find(|candidate| *candidate != base && constraint.can_set(candidate).is_ok())
                .unwrap_or(base)
        }

        let (_session, mut turn) = make_session_and_context().await;
        let base_instructions = BaseInstructions {
            text: "base".to_string(),
        };
        turn.developer_instructions = Some("dev".to_string());
        turn.compact_prompt = Some("compact".to_string());
        turn.shell_environment_policy = ShellEnvironmentPolicy {
            use_profile: true,
            ..ShellEnvironmentPolicy::default()
        };
        let temp_dir = tempfile::tempdir().expect("temp dir");
        turn.cwd = temp_dir.path().to_path_buf();
        turn.codex_linux_sandbox_exe = Some(PathBuf::from("/bin/echo"));
        let sandbox_policy = pick_allowed_sandbox_policy(
            &turn.config.permissions.sandbox_policy,
            turn.config.permissions.sandbox_policy.get().clone(),
        );
        turn.sandbox_policy
            .set(sandbox_policy)
            .expect("sandbox policy set");
        turn.approval_policy
            .set(AskForApproval::OnRequest)
            .expect("approval policy set");

        let config = build_agent_spawn_config(&base_instructions, &turn).expect("spawn config");
        let mut expected = (*turn.config).clone();
        expected.base_instructions = Some(base_instructions.text);
        expected.model = Some(turn.model_info.slug.clone());
        expected.model_provider = turn.provider.clone();
        expected.model_reasoning_effort = turn.reasoning_effort;
        expected.model_reasoning_summary = Some(turn.reasoning_summary);
        expected.developer_instructions = turn.developer_instructions.clone();
        expected.compact_prompt = turn.compact_prompt.clone();
        expected.permissions.shell_environment_policy = turn.shell_environment_policy.clone();
        expected.codex_linux_sandbox_exe = turn.codex_linux_sandbox_exe.clone();
        expected.cwd = turn.cwd.clone();
        expected
            .permissions
            .approval_policy
            .set(AskForApproval::OnRequest)
            .expect("approval policy set");
        expected
            .permissions
            .sandbox_policy
            .set(turn.sandbox_policy.get().clone())
            .expect("sandbox policy set");
        assert_eq!(config, expected);
    }

    #[tokio::test]
    async fn build_agent_spawn_config_preserves_base_user_instructions() {
        let (_session, mut turn) = make_session_and_context().await;
        let mut base_config = (*turn.config).clone();
        base_config.user_instructions = Some("base-user".to_string());
        turn.user_instructions = Some("resolved-user".to_string());
        turn.config = Arc::new(base_config.clone());
        let base_instructions = BaseInstructions {
            text: "base".to_string(),
        };

        let config = build_agent_spawn_config(&base_instructions, &turn).expect("spawn config");

        assert_eq!(config.user_instructions, base_config.user_instructions);
    }

    #[tokio::test]
    async fn build_agent_resume_config_clears_base_instructions() {
        let (_session, mut turn) = make_session_and_context().await;
        let mut base_config = (*turn.config).clone();
        base_config.base_instructions = Some("caller-base".to_string());
        turn.config = Arc::new(base_config);
        turn.approval_policy
            .set(AskForApproval::OnRequest)
            .expect("approval policy set");

        let config = build_agent_resume_config(&turn, 0).expect("resume config");

        let mut expected = (*turn.config).clone();
        expected.base_instructions = None;
        expected.model = Some(turn.model_info.slug.clone());
        expected.model_provider = turn.provider.clone();
        expected.model_reasoning_effort = turn.reasoning_effort;
        expected.model_reasoning_summary = Some(turn.reasoning_summary);
        expected.developer_instructions = turn.developer_instructions.clone();
        expected.compact_prompt = turn.compact_prompt.clone();
        expected.permissions.shell_environment_policy = turn.shell_environment_policy.clone();
        expected.codex_linux_sandbox_exe = turn.codex_linux_sandbox_exe.clone();
        expected.cwd = turn.cwd.clone();
        expected
            .permissions
            .approval_policy
            .set(AskForApproval::OnRequest)
            .expect("approval policy set");
        expected
            .permissions
            .sandbox_policy
            .set(turn.sandbox_policy.get().clone())
            .expect("sandbox policy set");
        assert_eq!(config, expected);
    }
}
