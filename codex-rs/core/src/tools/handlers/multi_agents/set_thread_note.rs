use super::*;
use codex_features::Feature;
use codex_protocol::protocol::ThreadNoteUpdatedEvent;
use codex_protocol::thread_note::normalize_thread_note_parts;

pub(crate) struct Handler;

impl ToolHandler for Handler {
    type Output = SetThreadNoteResult;

    fn tool_name(&self) -> ToolName {
        ToolName::plain("set_thread_note")
    }

    fn spec(&self) -> Option<codex_tools::ToolSpec> {
        Some(crate::tools::handlers::multi_agents_spec::create_set_thread_note_tool())
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    fn matches_kind(&self, payload: &ToolPayload) -> bool {
        matches!(payload, ToolPayload::Function { .. })
    }

    fn handle(
        &self,
        invocation: ToolInvocation,
    ) -> impl std::future::Future<Output = Result<Self::Output, FunctionCallError>> + Send {
        Box::pin(async move {
            let ToolInvocation {
                session,
                turn,
                payload,
                ..
            } = invocation;
            let arguments = function_arguments(payload)?;
            let is_multi_agent_v2 = turn.features.enabled(Feature::MultiAgentV2);
            let (target, raw_note, competencies) = if is_multi_agent_v2 {
                let args: SetThreadNoteArgsV2 = parse_arguments(&arguments)?;
                (args.target, args.thread_note, args.thread_note_competencies)
            } else {
                let args: SetThreadNoteArgsLegacy = parse_arguments(&arguments)?;
                (
                    args.target,
                    args.thread_note.or(args.note),
                    args.thread_note_competencies,
                )
            };
            let thread_note =
                normalize_thread_note_parts(raw_note.as_deref(), competencies.as_deref())
                    .map_err(|err| FunctionCallError::RespondToModel(err.to_string()))?;
            let (target_thread_id, target_label) = match target {
                Some(target) if is_multi_agent_v2 => {
                    let thread_id = crate::agent::agent_resolver::resolve_agent_target(
                        &session, &turn, &target,
                    )
                    .await?;
                    (thread_id, target)
                }
                Some(target) => (parse_agent_id_target(&target)?, target),
                None => (session.conversation_id, session.conversation_id.to_string()),
            };

            if target_thread_id != session.conversation_id
                && session
                    .services
                    .agent_control
                    .get_agent_metadata(target_thread_id)
                    .is_none()
            {
                return Err(FunctionCallError::RespondToModel(format!(
                    "agent target {target_label} is not visible from this thread"
                )));
            }

            session
                .services
                .agent_control
                .set_thread_note(target_thread_id, thread_note.clone())
                .await
                .map_err(|err| {
                    FunctionCallError::RespondToModel(format!("failed to set thread note: {err:?}"))
                })?;

            session
                .send_event(
                    &turn,
                    ThreadNoteUpdatedEvent {
                        thread_id: target_thread_id,
                        thread_note: thread_note.clone(),
                        updated_at_ms: crate::turn_timing::now_unix_timestamp_ms(),
                    }
                    .into(),
                )
                .await;

            Ok(SetThreadNoteResult {
                target: target_label,
                thread_note,
            })
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SetThreadNoteArgsV2 {
    target: Option<String>,
    thread_note: Option<String>,
    thread_note_competencies: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SetThreadNoteArgsLegacy {
    target: Option<String>,
    thread_note: Option<String>,
    thread_note_competencies: Option<String>,
    note: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct SetThreadNoteResult {
    target: String,
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
