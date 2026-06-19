use super::*;
use crate::tools::handlers::multi_agents_spec::create_close_agent_tool_v1;
use crate::turn_timing::now_unix_timestamp_ms;
use codex_protocol::error::CodexErr;
use codex_tools::ToolSpec;

pub(crate) struct Handler;

impl ToolExecutor<ToolInvocation> for Handler {
    fn tool_name(&self) -> ToolName {
        ToolName::namespaced(MULTI_AGENT_V1_NAMESPACE, "close_agent")
    }

    fn spec(&self) -> ToolSpec {
        create_close_agent_tool_v1()
    }

    fn search_info(&self) -> Option<ToolSearchInfo> {
        multi_agent_tool_search_info(
            "close_agent close shutdown stop agent subagent thread status target",
            self.spec(),
        )
    }

    fn handle(&self, invocation: ToolInvocation) -> codex_tools::ToolExecutorFuture<'_> {
        Box::pin(async move { handle_close_agent(invocation).await.map(boxed_tool_output) })
    }
}

async fn handle_close_agent(
    invocation: ToolInvocation,
) -> Result<CloseAgentResult, FunctionCallError> {
    let ToolInvocation {
        session,
        turn,
        payload,
        call_id,
        ..
    } = invocation;
    let arguments = function_arguments(payload)?;
    let args: CloseAgentArgs = parse_arguments(&arguments)?;
    let agent_id = parse_agent_id_target(&args.target)?;
    let receiver_agent = session.services.agent_control.get_agent_metadata(agent_id);
    let known_agent = receiver_agent.is_some();
    let receiver_agent = receiver_agent.unwrap_or_default();
    enforce_close_ownership(
        &turn.session_source,
        session.thread_id,
        agent_id,
        receiver_agent.agent_path.as_ref(),
    )?;
    session
        .send_event(
            &turn,
            CollabCloseBeginEvent {
                call_id: call_id.clone(),
                started_at_ms: now_unix_timestamp_ms(),
                sender_thread_id: session.thread_id,
                receiver_thread_id: agent_id,
            }
            .into(),
        )
        .await;
    let status = match session
        .services
        .agent_control
        .subscribe_status(agent_id)
        .await
    {
        Ok(mut status_rx) => status_rx.borrow_and_update().clone(),
        Err(CodexErr::ThreadNotFound(_)) if known_agent => {
            session.services.agent_control.get_status(agent_id).await
        }
        Err(err) => {
            let status = session.services.agent_control.get_status(agent_id).await;
            session
                .send_event(
                    &turn,
                    CollabCloseEndEvent {
                        call_id: call_id.clone(),
                        completed_at_ms: now_unix_timestamp_ms(),
                        sender_thread_id: session.thread_id(),
                        receiver_thread_id: agent_id,
                        receiver_agent_nickname: receiver_agent.agent_nickname.clone(),
                        receiver_agent_role: receiver_agent.agent_role.clone(),
                        receiver_agent_thread_note: receiver_agent.thread_note.clone(),
                        status,
                    }
                    .into(),
                )
                .await;
            return Err(collab_agent_error(agent_id, err));
        }
    };
    let result = Box::pin(session.services.agent_control.close_agent(agent_id))
        .await
        .map_err(|err| collab_agent_error(agent_id, err))
        .map(|_| ());
    session
        .send_event(
            &turn,
            CollabCloseEndEvent {
                call_id,
                completed_at_ms: now_unix_timestamp_ms(),
                sender_thread_id: session.thread_id,
                receiver_thread_id: agent_id,
                receiver_agent_nickname: receiver_agent.agent_nickname,
                receiver_agent_role: receiver_agent.agent_role,
                receiver_agent_thread_note: receiver_agent.thread_note,
                status: status.clone(),
            }
            .into(),
        )
        .await;
    result?;

    Ok(CloseAgentResult {
        previous_status: status,
    })
}

fn enforce_close_ownership(
    current_session_source: &codex_protocol::protocol::SessionSource,
    current_thread_id: ThreadId,
    target_thread_id: ThreadId,
    target_agent_path: Option<&AgentPath>,
) -> Result<(), FunctionCallError> {
    if current_thread_id == target_thread_id {
        return Err(FunctionCallError::RespondToModel(
            "an agent cannot close itself; return your result and let the parent close you if needed"
                .to_string(),
        ));
    }
    if target_agent_path.is_some_and(AgentPath::is_root) {
        return Err(FunctionCallError::RespondToModel(
            "root is not a spawned agent".to_string(),
        ));
    }

    let Some(current_agent_path) = current_session_source.get_agent_path() else {
        if current_session_source.is_non_root_agent() {
            return Err(FunctionCallError::RespondToModel(
                "agent without agent_path cannot close other agents".to_string(),
            ));
        }
        return Ok(());
    };

    let Some(target_agent_path) = target_agent_path else {
        return Err(FunctionCallError::RespondToModel(
            "target agent is missing an agent_path".to_string(),
        ));
    };
    if agent_path_is_descendant_of(target_agent_path, &current_agent_path) {
        return Ok(());
    }

    Err(FunctionCallError::RespondToModel(format!(
        "agent `{}` cannot close `{}` because the target is outside its sub-agent tree",
        current_agent_path.as_str(),
        target_agent_path.as_str()
    )))
}

fn agent_path_is_descendant_of(agent_path: &AgentPath, ancestor: &AgentPath) -> bool {
    agent_path
        .as_str()
        .strip_prefix(ancestor.as_str())
        .is_some_and(|suffix| suffix.starts_with('/'))
}

impl CoreToolRuntime for Handler {
    fn matches_kind(&self, payload: &ToolPayload) -> bool {
        matches!(payload, ToolPayload::Function { .. })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct CloseAgentResult {
    pub(crate) previous_status: AgentStatus,
}

impl ToolOutput for CloseAgentResult {
    fn log_preview(&self) -> String {
        tool_output_json_text(self, "close_agent")
    }

    fn success_for_logging(&self) -> bool {
        true
    }

    fn to_response_item(&self, call_id: &str, payload: &ToolPayload) -> ResponseInputItem {
        tool_output_response_item(call_id, payload, self, Some(true), "close_agent")
    }

    fn code_mode_result(&self, _payload: &ToolPayload) -> JsonValue {
        tool_output_code_mode_result(self, "close_agent")
    }
}

#[derive(Debug, Deserialize)]
struct CloseAgentArgs {
    target: String,
}
