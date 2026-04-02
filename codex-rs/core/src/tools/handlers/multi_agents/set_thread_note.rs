use super::*;
use codex_protocol::thread_note::normalize_thread_note;

pub(crate) struct Handler;

#[async_trait]
impl ToolHandler for Handler {
    type Output = SetThreadNoteResult;

    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    fn matches_kind(&self, payload: &ToolPayload) -> bool {
        matches!(payload, ToolPayload::Function { .. })
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<Self::Output, FunctionCallError> {
        let ToolInvocation {
            session, payload, ..
        } = invocation;
        let arguments = function_arguments(payload)?;
        let args: SetThreadNoteArgs = parse_arguments(&arguments)?;
        let note = normalize_thread_note(args.note.as_deref());

        session
            .services
            .agent_control
            .set_thread_note(session.conversation_id, note.clone())
            .await
            .map_err(|err| {
                FunctionCallError::RespondToModel(format!("failed to set thread note: {err:?}"))
            })?;

        Ok(SetThreadNoteResult { thread_note: note })
    }
}

#[derive(Debug, Deserialize)]
struct SetThreadNoteArgs {
    note: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct SetThreadNoteResult {
    thread_note: Option<String>,
}

impl ToolOutput for SetThreadNoteResult {
    fn log_preview(&self) -> String {
        tool_output_json_text(self, "set_thread_note")
    }

    fn success_for_logging(&self) -> bool {
        true
    }

    fn to_response_item(&self, call_id: &str, payload: &ToolPayload) -> ResponseInputItem {
        tool_output_response_item(call_id, payload, self, Some(true), "set_thread_note")
    }

    fn code_mode_result(&self, _payload: &ToolPayload) -> JsonValue {
        tool_output_code_mode_result(self, "set_thread_note")
    }
}
