// FORK COMMIT NEW FILE [SAW]: spawn handler extracted into dedicated module.
// Role: isolate spawn_agent flow to keep collab handler minimal while preserving legacy inline module.
use super::*;
use crate::agent::AgentRole;
use crate::agent::next_thread_spawn_depth;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
struct SpawnAgentArgs {
    message: Option<String>,
    items: Option<Vec<UserInput>>,
    agent_type: Option<AgentRole>,
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
    let agent_role = args.agent_type.unwrap_or(AgentRole::Default);
    let input_items = parse_collab_input(args.message, args.items)?;
    let prompt = input_preview(&input_items);
    let session_source = turn.session_source.clone();
    let child_depth = next_thread_spawn_depth(&session_source);
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
    agent_role
        .apply_to_config(&mut config)
        .map_err(FunctionCallError::RespondToModel)?;

    // FORK COMMIT OPEN [SAW]: encode role/type metadata into spawn source for downstream policy decisions.
    // Role: propagate agent role into ThreadSpawn so tool policy can be derived later.
    let result = session
        .services
        .agent_control
        .spawn_agent(
            config,
            input_items,
            Some(thread_spawn_source(
                session.conversation_id,
                child_depth,
                agent_role_name(agent_role),
                None,
                None,
                None,
            )),
        )
        .await
        .map_err(collab_spawn_error);
    // FORK COMMIT CLOSE: spawn source metadata handoff.
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

fn agent_role_name(agent_role: AgentRole) -> Option<String> {
    match agent_role {
        AgentRole::Default => None,
        AgentRole::Orchestrator => Some("orchestrator".to_string()),
        AgentRole::Worker => Some("worker".to_string()),
        AgentRole::Explorer => Some("explorer".to_string()),
    }
}
// FORK COMMIT CLOSE: spawn handler extracted into dedicated module.
