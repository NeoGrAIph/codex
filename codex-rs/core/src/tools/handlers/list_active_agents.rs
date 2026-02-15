// FORK COMMIT NEW FILE [SA]: list_active_agents tool handler for runtime spawned-thread metadata.
// Role: keep list_active_agents logic isolated from collab.rs to minimize native-file churn.
use async_trait::async_trait;
use codex_protocol::ThreadId;
use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::protocol::AgentStatus;
use codex_protocol::protocol::RolloutItem;
use codex_protocol::protocol::RolloutLine;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::path::Path;
use std::path::PathBuf;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::agent::agent_status_from_event;
use crate::codex::Session;
use crate::error::CodexErr;
use crate::function_tool::FunctionCallError;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolOutput;
use crate::tools::context::ToolPayload;
use crate::tools::handlers::parse_arguments;
use crate::tools::registry::ToolHandler;
use crate::tools::registry::ToolKind;

pub struct ListActiveAgentsHandler;

const ROLLOUT_SCAN_CHUNK_SIZE: usize = 16 * 1024;

#[derive(Debug, Deserialize, Default)]
struct ListActiveAgentsArgs {
    scope: Option<String>,
    #[serde(default)]
    include_tree: bool,
    #[serde(default)]
    include_closed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ListActiveAgentsScope {
    Children,
    Descendants,
    All,
}

impl ListActiveAgentsScope {
    fn parse(raw: Option<&str>) -> Result<Self, FunctionCallError> {
        match raw
            .unwrap_or("children")
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "children" => Ok(Self::Children),
            "descendants" => Ok(Self::Descendants),
            "all" => Ok(Self::All),
            other => Err(FunctionCallError::RespondToModel(format!(
                "unsupported scope {other:?}; expected one of: children, descendants, all"
            ))),
        }
    }
}

#[derive(Debug, Serialize)]
struct ListActiveAgentsResponse {
    agents: Vec<ActiveAgent>,
}

#[derive(Debug, Serialize)]
struct ActiveAgent {
    thread_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    thread_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    thread_note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    parent_thread_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    depth: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    agent_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    agent_name: Option<String>,
    status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    status_duration_sec: Option<u64>,
    model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    updated_at: Option<String>,
}

#[derive(Debug, Default)]
struct RolloutStatusScan {
    latest_timestamp: Option<String>,
    status_timestamp: Option<String>,
}

#[async_trait]
impl ToolHandler for ListActiveAgentsHandler {
    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    fn matches_kind(&self, payload: &ToolPayload) -> bool {
        matches!(payload, ToolPayload::Function { .. })
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<ToolOutput, FunctionCallError> {
        let ToolInvocation {
            session, payload, ..
        } = invocation;
        let arguments = match payload {
            ToolPayload::Function { arguments } => arguments,
            _ => {
                return Err(FunctionCallError::RespondToModel(
                    "list_active_agents handler received unsupported payload".to_string(),
                ));
            }
        };

        handle_list_active_agents(session, arguments).await
    }
}

async fn handle_list_active_agents(
    session: std::sync::Arc<Session>,
    arguments: String,
) -> Result<ToolOutput, FunctionCallError> {
    let args: ListActiveAgentsArgs = parse_arguments(&arguments)?;
    let scope = ListActiveAgentsScope::parse(args.scope.as_deref())?;

    let mut snapshots = session
        .services
        .agent_control
        .list_thread_spawn_threads()
        .await
        .map_err(list_active_agents_error)?;
    if scope == ListActiveAgentsScope::Children {
        snapshots.retain(|snapshot| snapshot.parent_thread_id == session.conversation_id);
    } else if scope == ListActiveAgentsScope::Descendants {
        let descendants: HashSet<ThreadId> = session
            .services
            .agent_control
            .list_thread_spawn_descendants(session.conversation_id)
            .await
            .map_err(list_active_agents_error)?
            .into_iter()
            .collect();
        snapshots.retain(|snapshot| descendants.contains(&snapshot.thread_id));
    }
    if !args.include_closed {
        snapshots.retain(|snapshot| status_is_active(&snapshot.status));
    }

    let thread_ids: HashSet<ThreadId> = snapshots
        .iter()
        .map(|snapshot| snapshot.thread_id)
        .collect();
    let codex_home = session.codex_home().await;
    let thread_names = lookup_thread_names(codex_home.as_path(), &thread_ids).await;
    let thread_notes = lookup_thread_notes(codex_home.as_path(), &thread_ids).await;

    let mut agents = Vec::with_capacity(snapshots.len());
    for snapshot in snapshots {
        let thread_name = thread_names.get(&snapshot.thread_id).cloned();
        let thread_note = thread_notes.get(&snapshot.thread_id).cloned();
        let (status_duration_sec, updated_at) =
            status_duration_and_update(snapshot.rollout_path.clone(), &snapshot.status).await;
        let (parent_thread_id, depth) = if args.include_tree {
            (
                Some(snapshot.parent_thread_id.to_string()),
                Some(snapshot.depth),
            )
        } else {
            (None, None)
        };
        agents.push(ActiveAgent {
            thread_id: snapshot.thread_id.to_string(),
            thread_name,
            thread_note,
            parent_thread_id,
            depth,
            agent_type: snapshot.agent_type,
            agent_name: snapshot.agent_name,
            status: status_label(&snapshot.status).to_string(),
            status_duration_sec,
            model: snapshot.model,
            reasoning_effort: snapshot.reasoning_effort.map(|effort| effort.to_string()),
            updated_at,
        });
    }
    agents.sort_by(|left, right| left.thread_id.cmp(&right.thread_id));

    let content = serde_json::to_string(&ListActiveAgentsResponse { agents }).map_err(|err| {
        FunctionCallError::Fatal(format!(
            "failed to serialize list_active_agents result: {err}"
        ))
    })?;

    Ok(ToolOutput::Function {
        body: FunctionCallOutputBody::Text(content),
        success: Some(true),
    })
}

fn list_active_agents_error(err: CodexErr) -> FunctionCallError {
    match err {
        CodexErr::UnsupportedOperation(_) => {
            FunctionCallError::RespondToModel("collab manager unavailable".to_string())
        }
        err => FunctionCallError::RespondToModel(format!("collab tool failed: {err}")),
    }
}

fn status_is_active(status: &AgentStatus) -> bool {
    !matches!(status, AgentStatus::Shutdown | AgentStatus::NotFound)
}

fn status_label(status: &AgentStatus) -> &'static str {
    match status {
        AgentStatus::PendingInit => "pending_init",
        AgentStatus::Running => "running",
        AgentStatus::Completed(_) => "completed",
        AgentStatus::Errored(_) => "errored",
        AgentStatus::Shutdown => "shutdown",
        AgentStatus::NotFound => "not_found",
    }
}

fn status_kind_matches(current: &AgentStatus, observed: &AgentStatus) -> bool {
    match (current, observed) {
        (AgentStatus::PendingInit, AgentStatus::PendingInit) => true,
        (AgentStatus::Running, AgentStatus::Running) => true,
        (AgentStatus::Completed(_), AgentStatus::Completed(_)) => true,
        (AgentStatus::Errored(_), AgentStatus::Errored(_)) => true,
        (AgentStatus::Shutdown, AgentStatus::Shutdown) => true,
        (AgentStatus::NotFound, AgentStatus::NotFound) => true,
        _ => false,
    }
}

async fn lookup_thread_names(
    codex_home: &Path,
    thread_ids: &HashSet<ThreadId>,
) -> HashMap<ThreadId, String> {
    match crate::rollout::session_index::find_thread_names_by_ids(codex_home, thread_ids).await {
        Ok(names) => names,
        Err(err) => {
            tracing::warn!("failed to read thread names from index: {err}");
            HashMap::new()
        }
    }
}

async fn lookup_thread_notes(
    codex_home: &Path,
    thread_ids: &HashSet<ThreadId>,
) -> HashMap<ThreadId, String> {
    match crate::rollout::session_index::find_thread_notes_by_ids(codex_home, thread_ids).await {
        Ok(notes) => notes,
        Err(err) => {
            tracing::warn!("failed to read thread notes from index: {err}");
            HashMap::new()
        }
    }
}

async fn status_duration_and_update(
    rollout_path: Option<PathBuf>,
    status: &AgentStatus,
) -> (Option<u64>, Option<String>) {
    let Some(path) = rollout_path else {
        return (None, None);
    };
    let status = status.clone();
    let status_for_scan = status.clone();
    let path_for_scan = path.clone();
    let scan_result = tokio::task::spawn_blocking(move || {
        scan_rollout_status_from_end(path_for_scan.as_path(), &status_for_scan)
    })
    .await;

    let scan = match scan_result {
        Ok(Ok(scan)) => scan,
        Ok(Err(err)) => {
            tracing::warn!("failed to scan rollout for status timing: {err}");
            return (None, file_updated_at(path.as_path()));
        }
        Err(err) => {
            tracing::warn!("failed to join rollout scan task: {err}");
            return (None, file_updated_at(path.as_path()));
        }
    };

    let updated_at = scan
        .latest_timestamp
        .or_else(|| file_updated_at(path.as_path()));
    let status_timestamp = scan.status_timestamp.or_else(|| {
        if matches!(status, AgentStatus::PendingInit) {
            updated_at.clone()
        } else {
            None
        }
    });
    let status_duration_sec = status_timestamp
        .as_deref()
        .and_then(status_duration_secs_from_timestamp);

    (status_duration_sec, updated_at)
}

fn file_updated_at(path: &Path) -> Option<String> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    let modified: OffsetDateTime = modified.into();
    modified.format(&Rfc3339).ok()
}

fn status_duration_secs_from_timestamp(timestamp: &str) -> Option<u64> {
    let status_at = OffsetDateTime::parse(timestamp, &Rfc3339).ok()?;
    let now = OffsetDateTime::now_utc();
    let elapsed = now - status_at;
    if elapsed.is_negative() {
        return Some(0);
    }
    u64::try_from(elapsed.whole_seconds()).ok()
}

fn scan_rollout_status_from_end(
    path: &Path,
    current_status: &AgentStatus,
) -> std::io::Result<RolloutStatusScan> {
    let mut file = File::open(path)?;
    let mut remaining = file.metadata()?.len();
    let mut line_rev: Vec<u8> = Vec::new();
    let mut buf = vec![0u8; ROLLOUT_SCAN_CHUNK_SIZE];
    let mut latest_timestamp = None;

    while remaining > 0 {
        let read_size = usize::try_from(remaining.min(ROLLOUT_SCAN_CHUNK_SIZE as u64))
            .map_err(std::io::Error::other)?;
        remaining -= read_size as u64;
        file.seek(SeekFrom::Start(remaining))?;
        file.read_exact(&mut buf[..read_size])?;

        for &byte in buf[..read_size].iter().rev() {
            if byte == b'\n' {
                if let Some(status_timestamp) = parse_rollout_line_from_rev(
                    &mut line_rev,
                    current_status,
                    &mut latest_timestamp,
                )? {
                    return Ok(RolloutStatusScan {
                        latest_timestamp,
                        status_timestamp: Some(status_timestamp),
                    });
                }
                continue;
            }
            line_rev.push(byte);
        }
    }

    let status_timestamp =
        parse_rollout_line_from_rev(&mut line_rev, current_status, &mut latest_timestamp)?;
    Ok(RolloutStatusScan {
        latest_timestamp,
        status_timestamp,
    })
}

fn parse_rollout_line_from_rev(
    line_rev: &mut Vec<u8>,
    current_status: &AgentStatus,
    latest_timestamp: &mut Option<String>,
) -> std::io::Result<Option<String>> {
    if line_rev.is_empty() {
        return Ok(None);
    }
    line_rev.reverse();
    let line = std::mem::take(line_rev);
    let Ok(mut line) = String::from_utf8(line) else {
        return Ok(None);
    };
    if line.ends_with('\r') {
        line.pop();
    }
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let Ok(rollout_line) = serde_json::from_str::<RolloutLine>(trimmed) else {
        return Ok(None);
    };
    if latest_timestamp.is_none() {
        *latest_timestamp = Some(rollout_line.timestamp.clone());
    }
    if let RolloutItem::EventMsg(msg) = rollout_line.item
        && let Some(observed_status) = agent_status_from_event(&msg)
        && status_kind_matches(current_status, &observed_status)
    {
        return Ok(Some(rollout_line.timestamp));
    }
    Ok(None)
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn status_is_active_treats_completed_and_errored_as_open_threads() {
        assert_eq!(status_is_active(&AgentStatus::PendingInit), true);
        assert_eq!(status_is_active(&AgentStatus::Running), true);
        assert_eq!(
            status_is_active(&AgentStatus::Completed(Some("done".to_string()))),
            true
        );
        assert_eq!(
            status_is_active(&AgentStatus::Errored("boom".to_string())),
            true
        );
        assert_eq!(status_is_active(&AgentStatus::Shutdown), false);
        assert_eq!(status_is_active(&AgentStatus::NotFound), false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CodexAuth;
    use crate::ThreadManager;
    use crate::built_in_model_providers;
    use crate::codex::TurnContext;
    use crate::codex::make_session_and_context;
    use crate::protocol::SessionSource;
    use crate::protocol::SubAgentSource;
    use crate::tools::context::ToolInvocation;
    use crate::turn_diff_tracker::TurnDiffTracker;
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    fn invocation(
        session: Arc<crate::codex::Session>,
        turn: Arc<TurnContext>,
        payload: ToolPayload,
    ) -> ToolInvocation {
        ToolInvocation {
            session,
            turn,
            tracker: Arc::new(Mutex::new(TurnDiffTracker::default())),
            call_id: "call-1".to_string(),
            tool_name: "list_active_agents".to_string(),
            payload,
        }
    }

    fn function_payload(args: serde_json::Value) -> ToolPayload {
        ToolPayload::Function {
            arguments: args.to_string(),
        }
    }

    fn text_input(text: &str) -> Vec<codex_protocol::user_input::UserInput> {
        vec![codex_protocol::user_input::UserInput::Text {
            text: text.to_string(),
            text_elements: Vec::new(),
        }]
    }

    fn thread_manager() -> ThreadManager {
        ThreadManager::with_models_provider_for_tests(
            CodexAuth::from_api_key("dummy"),
            built_in_model_providers()["openai"].clone(),
        )
    }

    #[tokio::test]
    async fn list_active_agents_rejects_invalid_scope() {
        let (session, turn) = make_session_and_context().await;
        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            function_payload(json!({"scope": "siblings"})),
        );
        let Err(err) = ListActiveAgentsHandler.handle(invocation).await else {
            panic!("invalid scope should be rejected");
        };
        assert_eq!(
            err,
            FunctionCallError::RespondToModel(
                "unsupported scope \"siblings\"; expected one of: children, descendants, all"
                    .to_string()
            )
        );
    }

    #[tokio::test]
    async fn list_active_agents_returns_descendants_with_tree_metadata() {
        let (mut session, turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();

        let child_id = manager
            .agent_control()
            .spawn_agent(
                (*turn.config).clone(),
                text_input("child"),
                Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                    parent_thread_id: session.conversation_id,
                    depth: 1,
                    agent_type: Some("worker".to_string()),
                    agent_name: Some("implementer".to_string()),
                    allow_list: None,
                    deny_list: None,
                })),
            )
            .await
            .expect("spawn child");
        let grandchild_id = manager
            .agent_control()
            .spawn_agent(
                (*turn.config).clone(),
                text_input("grandchild"),
                Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                    parent_thread_id: child_id,
                    depth: 2,
                    agent_type: Some("worker".to_string()),
                    agent_name: Some("fixer".to_string()),
                    allow_list: None,
                    deny_list: None,
                })),
            )
            .await
            .expect("spawn grandchild");
        let parent_id = session.conversation_id.to_string();

        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            function_payload(json!({
                "scope": "descendants",
                "include_tree": true,
                "include_closed": true
            })),
        );
        let output = ListActiveAgentsHandler
            .handle(invocation)
            .await
            .expect("list_active_agents should succeed");
        let ToolOutput::Function {
            body: FunctionCallOutputBody::Text(content),
            success,
            ..
        } = output
        else {
            panic!("expected function output");
        };
        assert_eq!(success, Some(true));

        let payload: serde_json::Value = serde_json::from_str(&content).expect("valid json");
        let agents = payload
            .get("agents")
            .and_then(serde_json::Value::as_array)
            .expect("agents array");
        assert_eq!(agents.len(), 2);

        let by_id: HashMap<String, &serde_json::Value> = agents
            .iter()
            .map(|agent| {
                (
                    agent
                        .get("thread_id")
                        .and_then(serde_json::Value::as_str)
                        .expect("thread_id")
                        .to_string(),
                    agent,
                )
            })
            .collect();
        assert_eq!(by_id.contains_key(&child_id.to_string()), true);
        assert_eq!(by_id.contains_key(&grandchild_id.to_string()), true);

        let child = by_id
            .get(&child_id.to_string())
            .expect("child entry should exist");
        assert_eq!(
            child
                .get("parent_thread_id")
                .and_then(serde_json::Value::as_str),
            Some(parent_id.as_str())
        );
        assert_eq!(
            child.get("depth").and_then(serde_json::Value::as_i64),
            Some(1)
        );
        let grandchild = by_id
            .get(&grandchild_id.to_string())
            .expect("grandchild entry should exist");
        let child_id_string = child_id.to_string();
        assert_eq!(
            grandchild
                .get("parent_thread_id")
                .and_then(serde_json::Value::as_str),
            Some(child_id_string.as_str())
        );
        assert_eq!(
            grandchild.get("depth").and_then(serde_json::Value::as_i64),
            Some(2)
        );

        let _ = manager
            .agent_control()
            .shutdown_agent(child_id)
            .await
            .expect("shutdown child subtree");
    }
}
