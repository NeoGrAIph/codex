// SA: fork-specific `spawn_agent` implementation (template-backed roles/personas + overrides).
use super::*;
use crate::agent::AgentRole;
use crate::agent::exceeds_thread_spawn_depth_limit;
use crate::agent::next_thread_spawn_depth;
use codex_protocol::openai_models::ReasoningEffort as ReasoningEffortConfig;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
struct SpawnAgentArgs {
    message: Option<String>,
    items: Option<Vec<UserInput>>,
    agent_type: Option<String>,
    agent_name: Option<String>,
    model: Option<String>,
    reasoning_effort: Option<ReasoningEffortConfig>,
}

#[derive(Debug, Serialize)]
struct SpawnAgentResult {
    agent_id: String,
}

pub async fn handle(
    session: Arc<Session>,
    turn: Arc<TurnContext>,
    call_id: String,
    arguments: String,
) -> Result<ToolOutput, FunctionCallError> {
    let args: SpawnAgentArgs = parse_arguments(&arguments)?;
    let agent_type_raw = args.agent_type;
    let agent_type = agent_type_raw.as_deref().unwrap_or("default");
    let agent_type = agent_type.trim().to_ascii_lowercase();
    let built_in_role = match agent_type.as_str() {
        "default" => Some(AgentRole::Default),
        "orchestrator" => Some(AgentRole::Orchestrator),
        "worker" => Some(AgentRole::Worker),
        "explorer" => Some(AgentRole::Explorer),
        _ => None,
    };
    let agent_name = args
        .agent_name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_ascii_lowercase);
    if agent_type == "default" && agent_name.is_some() {
        return Err(FunctionCallError::RespondToModel(
            "agent_name requires a non-default agent_type".to_string(),
        ));
    }
    let model_override = args
        .model
        .as_deref()
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .map(ToString::to_string);
    // SAW COMMIT OPEN: preserve spawned agent role for SAW.
    // Role: keep SAW role stable for implicit spawns, while surfacing explicit roles in the TUI agents window.
    // Keep the UI role stable for non-explicit spawns, while showing the explicit role when set.
    let agent_type_for_source = if agent_type_raw.is_some() {
        Some(agent_type.clone())
    } else {
        None
    };
    let agent_name_for_source = if args.agent_name.is_some() {
        agent_name.clone()
    } else {
        None
    };
    // [SA] COMMIT OPEN: agent-type tool policy metadata
    // Role: carry template-defined `allow_list` / `deny_list` into runtime tool filtering.
    let mut allow_list_for_source: Option<Vec<String>> = None;
    let mut deny_list_for_source: Option<Vec<String>> = None;
    // SAW COMMIT CLOSE: preserve spawned agent role for SAW.
    let input_items = parse_collab_input(args.message, args.items)?;
    let prompt = input_preview(&input_items);
    let child_depth = next_thread_spawn_depth(&turn.session_source);
    if exceeds_thread_spawn_depth_limit(child_depth) {
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
    let mut config = build_agent_spawn_config(
        &session.get_base_instructions().await,
        turn.as_ref(),
        child_depth,
    )?;

    // Apply built-in role defaults first (legacy behavior). Template defaults and explicit
    // overrides can further refine the configuration below.
    if let Some(role) = built_in_role {
        role.apply_to_config(&mut config)
            .map_err(FunctionCallError::RespondToModel)?;
    }

    // [SA] COMMIT OPEN: dynamic agent templates
    // Role: allow `spawn_agent.agent_type` to be either a built-in role or a custom
    // `templates/agents/<agent_type>.md` template.
    let parsed_template = if agent_type == "default" {
        None
    } else {
        match crate::agent::role_templates::get_parsed(&agent_type) {
            Ok(parsed) => Some(parsed),
            Err(err) => {
                // Custom roles must provide a template. Built-in roles can fall back to the
                // legacy behavior if no template exists.
                if built_in_role.is_some() && agent_name.is_none() {
                    None
                } else {
                    return Err(FunctionCallError::RespondToModel(err));
                }
            }
        }
    };

    if let Some(parsed) = parsed_template {
        let selected_instructions = match agent_name.as_deref() {
            Some(name) => parsed
                .named_instructions
                .get(name)
                .cloned()
                .ok_or_else(|| {
                    FunctionCallError::RespondToModel(format!(
                        "unknown agent_name {name:?} for agent_type {agent_type:?}"
                    ))
                })?,
            None => {
                let default = parsed.default_instructions.trim();
                if !default.is_empty() {
                    parsed.default_instructions.clone()
                } else if parsed.named_instructions.len() == 1 {
                    parsed
                        .named_instructions
                        .values()
                        .next()
                        .cloned()
                        .unwrap_or_default()
                } else {
                    return Err(FunctionCallError::RespondToModel(format!(
                        "agent_type {agent_type:?} requires agent_name selection"
                    )));
                }
            }
        };
        config.base_instructions = Some(selected_instructions);
        allow_list_for_source = parsed.meta.allow_list.clone();
        deny_list_for_source = parsed.meta.deny_list.clone();

        if model_override.is_none()
            && let Some(model) = parsed.meta.model.as_ref()
        {
            config.model = Some(model.clone());
        }
        if args.reasoning_effort.is_none()
            && let Some(effort) = parsed.meta.reasoning_effort
        {
            config.model_reasoning_effort = Some(effort);
        }
    }
    if let Some(model) = model_override {
        config.model = Some(model);
    }
    if let Some(effort) = args.reasoning_effort {
        config.model_reasoning_effort = Some(effort);
    }
    // [SA] COMMIT CLOSE: agent-type tool policy metadata
    // [SA] COMMIT CLOSE: dynamic agent templates

    let result = session
        .services
        .agent_control
        .spawn_agent(
            config,
            input_items,
            Some(thread_spawn_source(
                session.conversation_id,
                child_depth,
                agent_type_for_source,
                agent_name_for_source,
                allow_list_for_source,
                deny_list_for_source,
            )),
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
    session
        .send_event(
            &turn,
            CollabAgentSpawnEndEvent {
                call_id,
                sender_thread_id: session.conversation_id,
                new_thread_id,
                prompt,
                status,
            }
            .into(),
        )
        .await;
    let new_thread_id = result?;

    let content = serde_json::to_string(&SpawnAgentResult {
        agent_id: new_thread_id.to_string(),
    })
    .map_err(|err| {
        FunctionCallError::Fatal(format!("failed to serialize spawn_agent result: {err}"))
    })?;

    Ok(ToolOutput::Function {
        body: FunctionCallOutputBody::Text(content),
        success: Some(true),
    })
}
