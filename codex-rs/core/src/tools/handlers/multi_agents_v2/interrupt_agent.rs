use super::*;
use crate::tools::handlers::multi_agents_spec::create_interrupt_agent_tool_v2;
use crate::turn_timing::now_unix_timestamp_ms;
use codex_protocol::error::CodexErr;
use codex_tools::ToolSpec;

pub(crate) struct Handler;

impl ToolExecutor<ToolInvocation> for Handler {
    fn tool_name(&self) -> ToolName {
        ToolName::plain("interrupt_agent")
    }

    fn spec(&self) -> ToolSpec {
        create_interrupt_agent_tool_v2()
    }

    fn handle(&self, invocation: ToolInvocation) -> codex_tools::ToolExecutorFuture<'_> {
        Box::pin(async move {
            handle_interrupt_agent(invocation)
                .await
                .map(boxed_tool_output)
        })
    }
}

async fn handle_interrupt_agent(
    invocation: ToolInvocation,
) -> Result<InterruptAgentResult, FunctionCallError> {
    let ToolInvocation {
        session,
        turn,
        payload,
        call_id,
        ..
    } = invocation;
    let arguments = function_arguments(payload)?;
    let args: InterruptAgentArgs = parse_arguments(&arguments)?;
    let agent_id = resolve_agent_target(&session, &turn, &args.target).await?;
    let receiver_agent = session
        .services
        .agent_control
        .ensure_agent_known(agent_id)
        .map_err(|err| collab_agent_error(agent_id, err))?;
    if receiver_agent
        .agent_path
        .as_ref()
        .is_some_and(AgentPath::is_root)
    {
        return Err(FunctionCallError::RespondToModel(
            "root is not a spawned agent".to_string(),
        ));
    }
    if agent_id == session.thread_id {
        return Err(FunctionCallError::RespondToModel(
            "an agent cannot interrupt itself; return your result and let the parent interrupt you if needed"
                .to_string(),
        ));
    }
    let receiver_agent_path = receiver_agent.agent_path.clone().ok_or_else(|| {
        FunctionCallError::RespondToModel("target agent is missing an agent_path".to_string())
    })?;
    enforce_interrupt_ownership(&turn.session_source, &receiver_agent_path)?;
    let status = session.services.agent_control.get_status(agent_id).await;
    let result = match session
        .services
        .agent_control
        .interrupt_agent(agent_id)
        .await
    {
        Ok(_) | Err(CodexErr::ThreadNotFound(_)) | Err(CodexErr::InternalAgentDied) => Ok(()),
        Err(err) => Err(collab_agent_error(agent_id, err)),
    };
    result?;
    session
        .send_event(
            &turn,
            SubAgentActivityEvent {
                event_id: call_id,
                occurred_at_ms: now_unix_timestamp_ms(),
                agent_thread_id: agent_id,
                agent_path: receiver_agent_path,
                kind: SubAgentActivityKind::Interrupted,
            }
            .into(),
        )
        .await;

    Ok(InterruptAgentResult {
        previous_status: status,
    })
}

impl CoreToolRuntime for Handler {
    fn matches_kind(&self, payload: &ToolPayload) -> bool {
        matches!(payload, ToolPayload::Function { .. })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InterruptAgentArgs {
    target: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct InterruptAgentResult {
    pub(crate) previous_status: AgentStatus,
}

fn enforce_interrupt_ownership(
    current_session_source: &codex_protocol::protocol::SessionSource,
    target_agent_path: &AgentPath,
) -> Result<(), FunctionCallError> {
    let current_agent_path = current_session_source
        .get_agent_path()
        .unwrap_or_else(AgentPath::root);
    if current_agent_path.is_root()
        || agent_path_is_descendant_of(target_agent_path, &current_agent_path)
    {
        return Ok(());
    }
    Err(FunctionCallError::RespondToModel(format!(
        "agent `{}` cannot interrupt `{}` because the target is outside its sub-agent tree",
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

impl ToolOutput for InterruptAgentResult {
    fn log_preview(&self) -> String {
        tool_output_json_text(self, "interrupt_agent")
    }

    fn success_for_logging(&self) -> bool {
        true
    }

    fn to_response_item(&self, call_id: &str, payload: &ToolPayload) -> ResponseInputItem {
        tool_output_response_item(call_id, payload, self, Some(true), "interrupt_agent")
    }

    fn code_mode_result(&self, _payload: &ToolPayload) -> JsonValue {
        tool_output_code_mode_result(self, "interrupt_agent")
    }
}
