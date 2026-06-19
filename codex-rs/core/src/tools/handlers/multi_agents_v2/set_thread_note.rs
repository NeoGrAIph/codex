use super::*;
use crate::tools::handlers::multi_agents_spec::create_set_thread_note_tool;
use codex_tools::ToolSpec;

pub(crate) struct Handler;

impl ToolExecutor<ToolInvocation> for Handler {
    fn tool_name(&self) -> ToolName {
        ToolName::plain("set_thread_note")
    }

    fn spec(&self) -> ToolSpec {
        create_set_thread_note_tool()
    }

    fn handle(&self, invocation: ToolInvocation) -> codex_tools::ToolExecutorFuture<'_> {
        Box::pin(async move {
            handle_set_thread_note(invocation)
                .await
                .map(boxed_tool_output)
        })
    }
}

async fn handle_set_thread_note(
    invocation: ToolInvocation,
) -> Result<SetThreadNoteResult, FunctionCallError> {
    let ToolInvocation {
        session,
        turn,
        payload,
        ..
    } = invocation;
    let arguments = function_arguments(payload)?;
    let args: SetThreadNoteArgs = parse_arguments(&arguments)?;
    let thread_note = normalize_thread_note(args.thread_note)?;
    let (target_thread_id, target_agent_path) = match args.target {
        Some(target) => {
            let target_thread_id = resolve_agent_target(&session, &turn, &target).await?;
            let target_agent = session
                .services
                .agent_control
                .ensure_agent_known(target_thread_id)
                .map_err(|err| collab_agent_error(target_thread_id, err))?;
            let target_agent_path = target_agent.agent_path.ok_or_else(|| {
                FunctionCallError::RespondToModel(
                    "target agent is missing an agent_path".to_string(),
                )
            })?;
            if target_agent_path.is_root() {
                return Err(FunctionCallError::RespondToModel(
                    "root is not a spawned agent".to_string(),
                ));
            }
            enforce_thread_note_ownership(&turn.session_source, &target_agent_path)?;
            (target_thread_id, target_agent_path)
        }
        None => {
            let current_agent_path = turn.session_source.get_agent_path().ok_or_else(|| {
                FunctionCallError::RespondToModel(
                    "set_thread_note requires a spawned sub-agent target".to_string(),
                )
            })?;
            (session.thread_id, current_agent_path)
        }
    };

    session
        .services
        .agent_control
        .set_thread_note(target_thread_id, thread_note.clone())
        .await
        .map_err(|err| collab_agent_error(target_thread_id, err))?;

    Ok(SetThreadNoteResult {
        target: target_agent_path.to_string(),
        thread_note,
    })
}

impl CoreToolRuntime for Handler {
    fn matches_kind(&self, payload: &ToolPayload) -> bool {
        matches!(payload, ToolPayload::Function { .. })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SetThreadNoteArgs {
    target: Option<String>,
    thread_note: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct SetThreadNoteResult {
    target: String,
    thread_note: Option<String>,
}

fn enforce_thread_note_ownership(
    current_session_source: &codex_protocol::protocol::SessionSource,
    target_agent_path: &AgentPath,
) -> Result<(), FunctionCallError> {
    let current_agent_path = match current_session_source.get_agent_path() {
        Some(agent_path) => agent_path,
        None if current_session_source.is_non_root_agent() => {
            return Err(FunctionCallError::RespondToModel(
                "set_thread_note requires a path-backed author".to_string(),
            ));
        }
        None => AgentPath::root(),
    };
    if current_agent_path.is_root()
        || target_agent_path == &current_agent_path
        || agent_path_is_descendant_of(target_agent_path, &current_agent_path)
    {
        return Ok(());
    }
    Err(FunctionCallError::RespondToModel(format!(
        "agent `{}` cannot set thread_note for `{}` because the target is outside its sub-agent tree",
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
