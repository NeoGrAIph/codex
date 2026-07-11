use super::*;
use crate::agent::control::render_input_preview;
use crate::tools::handlers::multi_agents_spec::create_send_input_tool_v1;
use codex_tools::ToolSpec;

pub(crate) struct Handler;

impl ToolExecutor<ToolInvocation> for Handler {
    fn tool_name(&self) -> ToolName {
        ToolName::namespaced(MULTI_AGENT_V1_NAMESPACE, "send_input")
    }

    fn spec(&self) -> ToolSpec {
        create_send_input_tool_v1()
    }

    fn search_info(&self) -> Option<ToolSearchInfo> {
        multi_agent_tool_search_info(
            "send_input send message existing agent subagent follow up interrupt redirect queue target",
            self.spec(),
        )
    }

    fn handle(&self, invocation: ToolInvocation) -> codex_tools::ToolExecutorFuture<'_> {
        Box::pin(self.handle_call(invocation))
    }
}

impl Handler {
    async fn handle_call(
        &self,
        invocation: ToolInvocation,
    ) -> Result<Box<dyn crate::tools::context::ToolOutput>, FunctionCallError> {
        let ToolInvocation {
            session,
            turn,
            payload,
            call_id,
            ..
        } = invocation;
        let arguments = function_arguments(payload)?;
        let args: SendInputArgs = parse_arguments(&arguments)?;
        let receiver_thread_id = parse_agent_id_target(&args.target)?;
        let input_items = parse_collab_input(args.message, args.items)?;
        let prompt = render_input_preview(&input_items);
        authorize_live_v1_agent_target(&session, &turn, receiver_thread_id)?;
        let receiver_agent = session
            .services
            .agent_control
            .get_agent_metadata(receiver_thread_id)
            .unwrap_or_default();
        let uses_transactional_lifecycle = turn.multi_agent_version == MultiAgentVersion::V2;
        session
            .emit_turn_item_started(
                &turn,
                &TurnItem::CollabAgentToolCall(CollabAgentToolCallItem {
                    id: call_id.clone(),
                    tool: CollabAgentTool::SendInput,
                    status: CollabAgentToolCallStatus::InProgress,
                    sender_thread_id: session.thread_id,
                    receiver_thread_ids: vec![receiver_thread_id],
                    receiver_agents: Vec::new(),
                    prompt: Some(prompt.clone()),
                    model: None,
                    reasoning_effort: None,
                    agents_states: Default::default(),
                }),
            )
            .await;
        let agent_control = session.services.agent_control.clone();
        let result = async {
            if uses_transactional_lifecycle && receiver_agent.agent_id.is_some() {
                let resume_config = build_agent_resume_config(turn.as_ref())?;
                agent_control
                    .ensure_v2_agent_loaded(resume_config, receiver_thread_id)
                    .await
                    .map_err(|err| collab_agent_error(receiver_thread_id, err))?;
            }
            if args.interrupt {
                if uses_transactional_lifecycle {
                    agent_control
                        .interrupt_agent_transactional(receiver_thread_id)
                        .await
                        .map_err(|err| collab_agent_error(receiver_thread_id, err))?;
                } else {
                    agent_control
                        .interrupt_agent(receiver_thread_id)
                        .await
                        .map_err(|err| collab_agent_error(receiver_thread_id, err))?;
                }
            }
            // V1 accepts rich UserInput items (images, local images, skills, and mentions) that
            // InterAgentCommunication cannot represent losslessly. Keep the legacy raw-input
            // transport under V2 instead of silently flattening those payloads to text.
            if uses_transactional_lifecycle {
                agent_control
                    .send_input_transactional(receiver_thread_id, input_items)
                    .await
                    .map_err(|err| collab_agent_error(receiver_thread_id, err))
            } else {
                agent_control
                    .send_input(receiver_thread_id, input_items)
                    .await
                    .map_err(|err| collab_agent_error(receiver_thread_id, err))
            }
        }
        .await;
        let status = session
            .services
            .agent_control
            .get_status(receiver_thread_id)
            .await;
        let tool_call_status = if result.is_err() {
            CollabAgentToolCallStatus::Failed
        } else {
            collab_tool_call_status(&status, Some(receiver_thread_id))
        };
        session
            .emit_turn_item_completed(
                &turn,
                TurnItem::CollabAgentToolCall(CollabAgentToolCallItem {
                    id: call_id,
                    tool: CollabAgentTool::SendInput,
                    status: tool_call_status,
                    sender_thread_id: session.thread_id,
                    receiver_thread_ids: vec![receiver_thread_id],
                    receiver_agents: vec![CollabAgentRef {
                        thread_id: receiver_thread_id,
                        agent_nickname: receiver_agent.agent_nickname,
                        agent_role: receiver_agent.agent_role,
                    }],
                    prompt: Some(prompt),
                    model: None,
                    reasoning_effort: None,
                    agents_states: [(receiver_thread_id, status)].into_iter().collect(),
                }),
            )
            .await;
        let submission_id = result?;

        Ok(boxed_tool_output(SendInputResult { submission_id }))
    }
}

impl CoreToolRuntime for Handler {
    fn matches_kind(&self, payload: &ToolPayload) -> bool {
        matches!(payload, ToolPayload::Function { .. })
    }
}

#[derive(Debug, Deserialize)]
struct SendInputArgs {
    target: String,
    message: Option<String>,
    items: Option<Vec<UserInput>>,
    #[serde(default)]
    interrupt: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct SendInputResult {
    submission_id: String,
}

impl ToolOutput for SendInputResult {
    fn log_preview(&self) -> String {
        tool_output_json_text(self, "send_input")
    }

    fn success_for_logging(&self) -> bool {
        true
    }

    fn to_response_item(&self, call_id: &str, payload: &ToolPayload) -> ResponseInputItem {
        tool_output_response_item(call_id, payload, self, Some(true), "send_input")
    }

    fn code_mode_result(&self, _payload: &ToolPayload) -> JsonValue {
        tool_output_code_mode_result(self, "send_input")
    }
}
