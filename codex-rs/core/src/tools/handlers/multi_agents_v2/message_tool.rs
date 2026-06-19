//! Shared argument parsing and dispatch for the v2 agent messaging tools.
//!
//! `send_message` and `followup_task` share the same submission path and differ only in whether the
//! resulting `InterAgentCommunication` should wake the target immediately.

use super::*;
use crate::tools::context::FunctionToolOutput;
use crate::turn_timing::now_unix_timestamp_ms;
use codex_protocol::protocol::InterAgentCommunication;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum MessageDeliveryMode {
    QueueOnly,
    TriggerTurn,
}

impl MessageDeliveryMode {
    /// Returns whether the produced communication should start a turn immediately.
    fn apply(self, communication: InterAgentCommunication) -> InterAgentCommunication {
        match self {
            Self::QueueOnly => InterAgentCommunication {
                trigger_turn: false,
                ..communication
            },
            Self::TriggerTurn => InterAgentCommunication {
                trigger_turn: true,
                ..communication
            },
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
/// Input for the MultiAgentV2 `send_message` tool.
pub(crate) struct SendMessageArgs {
    pub(crate) target: String,
    pub(crate) message: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
/// Input for the MultiAgentV2 `followup_task` tool.
pub(crate) struct FollowupTaskArgs {
    pub(crate) target: String,
    pub(crate) message: String,
}

fn message_content(message: String) -> Result<String, FunctionCallError> {
    if message.trim().is_empty() {
        return Err(FunctionCallError::RespondToModel(
            "Empty message can't be sent to an agent".to_string(),
        ));
    }
    Ok(message)
}

/// Handles the shared MultiAgentV2 message flow for both `send_message` and `followup_task`.
pub(crate) async fn handle_message_string_tool(
    invocation: ToolInvocation,
    mode: MessageDeliveryMode,
    target: String,
    message: String,
) -> Result<FunctionToolOutput, FunctionCallError> {
    let message = message_content(message)?;
    let ToolInvocation {
        session,
        turn,
        call_id,
        ..
    } = invocation;
    let receiver_thread_id = resolve_agent_target(&session, &turn, &target).await?;
    let receiver_agent = session
        .services
        .agent_control
        .ensure_agent_known(receiver_thread_id)
        .map_err(|err| collab_agent_error(receiver_thread_id, err))?;
    let receiver_agent_path = receiver_agent.agent_path.clone().ok_or_else(|| {
        FunctionCallError::RespondToModel("target agent is missing an agent_path".to_string())
    })?;
    if receiver_agent_path.is_root() {
        let message = match mode {
            MessageDeliveryMode::QueueOnly => "Agent messaging cannot target the root agent.",
            MessageDeliveryMode::TriggerTurn => "Follow-up tasks can't target the root agent",
        };
        return Err(FunctionCallError::RespondToModel(message.to_string()));
    }
    let resume_config = build_agent_resume_config(turn.as_ref())?;
    session
        .services
        .agent_control
        .ensure_v2_agent_loaded(resume_config, receiver_thread_id)
        .await
        .map_err(|err| collab_agent_error(receiver_thread_id, err))?;
    let author = match turn.session_source.get_agent_path() {
        Some(agent_path) => agent_path,
        None if turn.session_source.is_non_root_agent() => {
            let tool_name = match mode {
                MessageDeliveryMode::QueueOnly => "send_message",
                MessageDeliveryMode::TriggerTurn => "followup_task",
            };
            return Err(FunctionCallError::RespondToModel(format!(
                "{tool_name} requires a path-backed author"
            )));
        }
        None => AgentPath::root(),
    };
    enforce_message_ownership(mode, &author, &receiver_agent_path)?;
    let communication =
        communication_from_tool_message(author, receiver_agent_path.clone(), message);
    let result = session
        .services
        .agent_control
        .send_inter_agent_communication(receiver_thread_id, mode.apply(communication))
        .await
        .map_err(|err| collab_agent_error(receiver_thread_id, err));
    result?;
    session
        .send_event(
            &turn,
            SubAgentActivityEvent {
                event_id: call_id,
                occurred_at_ms: now_unix_timestamp_ms(),
                agent_thread_id: receiver_thread_id,
                agent_path: receiver_agent_path,
                kind: SubAgentActivityKind::Interacted,
            }
            .into(),
        )
        .await;

    Ok(FunctionToolOutput::from_text(String::new(), Some(true)))
}

fn enforce_message_ownership(
    mode: MessageDeliveryMode,
    author: &AgentPath,
    target: &AgentPath,
) -> Result<(), FunctionCallError> {
    if target == author {
        let message = match mode {
            MessageDeliveryMode::QueueOnly => "Cannot send an agent message to the current thread.",
            MessageDeliveryMode::TriggerTurn => {
                "Cannot send an agent follow-up to the current thread."
            }
        };
        return Err(FunctionCallError::RespondToModel(message.to_string()));
    }
    if author.is_root() || agent_path_is_descendant_of(target, author) {
        return Ok(());
    }
    let action = match mode {
        MessageDeliveryMode::QueueOnly => "send_message",
        MessageDeliveryMode::TriggerTurn => "followup_task",
    };
    Err(FunctionCallError::RespondToModel(format!(
        "agent `{}` cannot {action} to `{}` because the target is outside its sub-agent tree",
        author.as_str(),
        target.as_str()
    )))
}

fn agent_path_is_descendant_of(agent_path: &AgentPath, ancestor: &AgentPath) -> bool {
    agent_path
        .as_str()
        .strip_prefix(ancestor.as_str())
        .is_some_and(|suffix| suffix.starts_with('/'))
}
