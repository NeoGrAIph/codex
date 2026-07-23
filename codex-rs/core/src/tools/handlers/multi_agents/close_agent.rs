use super::*;
use crate::tools::handlers::multi_agents_spec::create_close_agent_tool_v1;
use codex_protocol::error::CodexErr;
use codex_tools::ToolSpec;

pub(crate) struct Handler;
pub(crate) struct ProjectedHandler;

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
        Box::pin(async move {
            handle_close_agent(invocation, V1ToolInvocationMode::Native)
                .await
                .map(boxed_tool_output)
        })
    }
}

impl ToolExecutor<ToolInvocation> for ProjectedHandler {
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
        Box::pin(async move {
            handle_close_agent(invocation, V1ToolInvocationMode::ProjectedFromV2)
                .await
                .map(boxed_tool_output)
        })
    }
}

async fn handle_close_agent(
    invocation: ToolInvocation,
    invocation_mode: V1ToolInvocationMode,
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
    let lifecycle_target = resolve_v1_lifecycle_target(&session, invocation_mode, agent_id).await?;
    let receiver_agent = session.services.agent_control.get_agent_metadata(agent_id);
    let known_agent = receiver_agent.is_some();
    let known_close_target = known_agent
        || lifecycle_target
            .as_ref()
            .is_some_and(|target| matches!(target, ProjectedV1LifecycleTarget::Persisted(_)));
    let receiver_agent = receiver_agent.unwrap_or_default();
    session
        .emit_turn_item_started(
            &turn,
            &TurnItem::CollabAgentToolCall(CollabAgentToolCallItem {
                id: call_id.clone(),
                tool: CollabAgentTool::CloseAgent,
                status: CollabAgentToolCallStatus::InProgress,
                sender_thread_id: session.thread_id,
                receiver_thread_ids: vec![agent_id],
                receiver_agents: Vec::new(),
                prompt: None,
                model: None,
                reasoning_effort: None,
                agents_states: Default::default(),
            }),
        )
        .await;
    let status_subscription = match lifecycle_target.as_ref() {
        Some(target) => session
            .services
            .agent_control
            .subscribe_projected_v1_target(target),
        None => {
            session
                .services
                .agent_control
                .subscribe_status(agent_id)
                .await
        }
    };
    let status = match status_subscription {
        Ok(mut status_rx) => status_rx.borrow_and_update().clone(),
        Err(CodexErr::ThreadNotFound(_)) if known_close_target => match lifecycle_target.as_ref() {
            Some(target) => {
                session
                    .services
                    .agent_control
                    .projected_v1_target_status(target)
                    .await
            }
            None => session.services.agent_control.get_status(agent_id).await,
        },
        Err(err) => {
            let status = match lifecycle_target.as_ref() {
                Some(target) => {
                    session
                        .services
                        .agent_control
                        .projected_v1_target_status(target)
                        .await
                }
                None => session.services.agent_control.get_status(agent_id).await,
            };
            session
                .emit_turn_item_completed(
                    &turn,
                    TurnItem::CollabAgentToolCall(CollabAgentToolCallItem {
                        id: call_id.clone(),
                        tool: CollabAgentTool::CloseAgent,
                        status: collab_tool_call_status(&status, Some(agent_id)),
                        sender_thread_id: session.thread_id(),
                        receiver_thread_ids: vec![agent_id],
                        receiver_agents: vec![CollabAgentRef {
                            thread_id: agent_id,
                            agent_nickname: receiver_agent.agent_nickname.clone(),
                            agent_role: receiver_agent.agent_role.clone(),
                        }],
                        prompt: None,
                        model: None,
                        reasoning_effort: None,
                        agents_states: [(agent_id, status)].into_iter().collect(),
                    }),
                )
                .await;
            return Err(collab_agent_error(agent_id, err));
        }
    };
    let close_result = match lifecycle_target {
        Some(target) => {
            Box::pin(
                session
                    .services
                    .agent_control
                    .close_projected_v1_agent(target),
            )
            .await
        }
        None => Box::pin(session.services.agent_control.close_agent(agent_id)).await,
    };
    let result = close_result
        .map_err(|err| collab_agent_error(agent_id, err))
        .map(|_| ());
    session
        .emit_turn_item_completed(
            &turn,
            TurnItem::CollabAgentToolCall(CollabAgentToolCallItem {
                id: call_id,
                tool: CollabAgentTool::CloseAgent,
                status: collab_tool_call_status(&status, Some(agent_id)),
                sender_thread_id: session.thread_id,
                receiver_thread_ids: vec![agent_id],
                receiver_agents: vec![CollabAgentRef {
                    thread_id: agent_id,
                    agent_nickname: receiver_agent.agent_nickname,
                    agent_role: receiver_agent.agent_role,
                }],
                prompt: None,
                model: None,
                reasoning_effort: None,
                agents_states: [(agent_id, status.clone())].into_iter().collect(),
            }),
        )
        .await;
    result?;

    Ok(CloseAgentResult {
        previous_status: status,
    })
}

impl CoreToolRuntime for Handler {
    fn matches_kind(&self, payload: &ToolPayload) -> bool {
        matches!(payload, ToolPayload::Function { .. })
    }
}

impl CoreToolRuntime for ProjectedHandler {
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
