use super::*;
use crate::agent::status::is_final;
use crate::session::session::Session;
use crate::tools::handlers::multi_agents_spec::WaitAgentTimeoutOptions;
use crate::tools::handlers::multi_agents_spec::create_projected_wait_agent_tool_v1;
use crate::tools::handlers::multi_agents_spec::create_wait_agent_tool_v1;
use codex_protocol::error::CodexErr;
use codex_tools::ToolSpec;
use futures::FutureExt;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch::Receiver;
use tokio::time::Instant;

use tokio::time::timeout_at;

pub(crate) const MAX_PROJECTED_WAIT_RESULT_BYTES: usize = 32 * 1_024;
const MAX_PROJECTED_WAIT_STATUS_BYTES: usize = 8 * 1_024;
const PROJECTED_WAIT_TRUNCATION_MARKER: &str = "\n...[truncated]";

#[derive(Default)]
pub(crate) struct Handler {
    options: WaitAgentTimeoutOptions,
    target_policy: WaitTargetPolicy,
}

#[derive(Clone, Copy, Default)]
enum WaitTargetPolicy {
    #[default]
    Legacy,
    ProjectedV2,
}

impl Handler {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn new_projected(options: WaitAgentTimeoutOptions) -> Self {
        Self {
            options,
            target_policy: WaitTargetPolicy::ProjectedV2,
        }
    }
}

impl ToolExecutor<ToolInvocation> for Handler {
    fn tool_name(&self) -> ToolName {
        ToolName::namespaced(MULTI_AGENT_V1_NAMESPACE, "wait_agent")
    }

    fn spec(&self) -> ToolSpec {
        match self.target_policy {
            WaitTargetPolicy::Legacy => {
                create_wait_agent_tool_v1(WaitAgentTimeoutOptions::default())
            }
            WaitTargetPolicy::ProjectedV2 => create_projected_wait_agent_tool_v1(self.options),
        }
    }

    fn search_info(&self) -> Option<ToolSearchInfo> {
        multi_agent_tool_search_info(
            "wait_agent wait agent subagent status final result complete timeout targets",
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
        let args: WaitArgs = parse_arguments(&arguments)?;
        let receiver_thread_ids = match self.target_policy {
            WaitTargetPolicy::Legacy => parse_agent_id_targets(args.targets)?,
            WaitTargetPolicy::ProjectedV2 => parse_projected_agent_id_targets(args.targets)?,
        };
        let mut receiver_agents = Vec::with_capacity(receiver_thread_ids.len());
        let mut target_by_thread_id = HashMap::with_capacity(receiver_thread_ids.len());
        for receiver_thread_id in &receiver_thread_ids {
            authorize_live_v1_agent_target(&session, &turn, *receiver_thread_id)?;
            let agent_metadata = session
                .services
                .agent_control
                .get_agent_metadata(*receiver_thread_id)
                .unwrap_or_default();
            target_by_thread_id.insert(*receiver_thread_id, receiver_thread_id.to_string());
            receiver_agents.push(CollabAgentRef {
                thread_id: *receiver_thread_id,
                agent_nickname: agent_metadata.agent_nickname,
                agent_role: agent_metadata.agent_role,
            });
        }

        let timeout_options = match self.target_policy {
            WaitTargetPolicy::Legacy => WaitAgentTimeoutOptions::default(),
            WaitTargetPolicy::ProjectedV2 => self.options,
        };
        let timeout_ms = args
            .timeout_ms
            .unwrap_or(timeout_options.default_timeout_ms);
        let timeout_ms = match timeout_ms {
            ms if ms < 0 || (ms == 0 && timeout_options.min_timeout_ms > 0) => {
                return Err(FunctionCallError::RespondToModel(
                    "timeout_ms must be greater than zero".to_owned(),
                ));
            }
            ms => ms.clamp(
                timeout_options.min_timeout_ms,
                timeout_options.max_timeout_ms,
            ),
        };

        session
            .emit_turn_item_started(
                &turn,
                &TurnItem::CollabAgentToolCall(CollabAgentToolCallItem {
                    id: call_id.clone(),
                    tool: CollabAgentTool::Wait,
                    status: CollabAgentToolCallStatus::InProgress,
                    sender_thread_id: session.thread_id,
                    receiver_thread_ids: receiver_thread_ids.clone(),
                    receiver_agents: receiver_agents.clone(),
                    prompt: None,
                    model: None,
                    reasoning_effort: None,
                    agents_states: Default::default(),
                }),
            )
            .await;

        let mut status_rxs = Vec::with_capacity(receiver_thread_ids.len());
        let mut initial_final_statuses = Vec::new();
        for id in &receiver_thread_ids {
            match session.services.agent_control.subscribe_status(*id).await {
                Ok(rx) => {
                    let status = rx.borrow().clone();
                    if is_final(&status) {
                        initial_final_statuses.push((*id, status));
                    }
                    status_rxs.push((*id, rx));
                }
                Err(CodexErr::ThreadNotFound(_)) => {
                    initial_final_statuses
                        .push((*id, session.services.agent_control.get_status(*id).await));
                }
                Err(err) => {
                    let mut statuses = HashMap::with_capacity(1);
                    statuses.insert(*id, session.services.agent_control.get_status(*id).await);
                    session
                        .emit_turn_item_completed(
                            &turn,
                            TurnItem::CollabAgentToolCall(CollabAgentToolCallItem {
                                id: call_id.clone(),
                                tool: CollabAgentTool::Wait,
                                status: wait_tool_call_status(&statuses),
                                sender_thread_id: session.thread_id,
                                receiver_thread_ids: statuses.keys().copied().collect(),
                                receiver_agents: wait_receiver_agents(&statuses, &receiver_agents),
                                prompt: None,
                                model: None,
                                reasoning_effort: None,
                                agents_states: statuses,
                            }),
                        )
                        .await;
                    return Err(collab_agent_error(*id, err));
                }
            }
        }

        let statuses = if !initial_final_statuses.is_empty() {
            initial_final_statuses
        } else {
            let mut futures = FuturesUnordered::new();
            for (id, rx) in status_rxs.into_iter() {
                let session = session.clone();
                futures.push(wait_for_final_status(session, id, rx));
            }
            let mut results = Vec::new();
            let deadline = Instant::now() + Duration::from_millis(timeout_ms as u64);
            loop {
                match timeout_at(deadline, futures.next()).await {
                    Ok(Some(Some(result))) => {
                        results.push(result);
                        break;
                    }
                    Ok(Some(None)) => continue,
                    Ok(None) | Err(_) => break,
                }
            }
            if !results.is_empty() {
                loop {
                    match futures.next().now_or_never() {
                        Some(Some(Some(result))) => results.push(result),
                        Some(Some(None)) => continue,
                        Some(None) | None => break,
                    }
                }
            }
            results
        };

        let statuses = match self.target_policy {
            WaitTargetPolicy::Legacy => statuses,
            WaitTargetPolicy::ProjectedV2 => bound_projected_wait_statuses(statuses),
        };
        let timed_out = statuses.is_empty();
        let statuses_by_id = statuses.clone().into_iter().collect::<HashMap<_, _>>();
        let result = WaitAgentResult {
            status: statuses
                .into_iter()
                .filter_map(|(thread_id, status)| {
                    target_by_thread_id
                        .get(&thread_id)
                        .cloned()
                        .map(|target| (target, status))
                })
                .collect(),
            timed_out,
        };

        session
            .emit_turn_item_completed(
                &turn,
                TurnItem::CollabAgentToolCall(CollabAgentToolCallItem {
                    id: call_id,
                    tool: CollabAgentTool::Wait,
                    status: wait_tool_call_status(&statuses_by_id),
                    sender_thread_id: session.thread_id,
                    receiver_thread_ids: statuses_by_id.keys().copied().collect(),
                    receiver_agents: wait_receiver_agents(&statuses_by_id, &receiver_agents),
                    prompt: None,
                    model: None,
                    reasoning_effort: None,
                    agents_states: statuses_by_id,
                }),
            )
            .await;

        Ok(boxed_tool_output(result))
    }
}

pub(crate) fn bound_projected_wait_statuses(
    statuses: Vec<(ThreadId, AgentStatus)>,
) -> Vec<(ThreadId, AgentStatus)> {
    let mut bounded = statuses
        .iter()
        .map(|(thread_id, status)| (*thread_id, status_without_text(status)))
        .collect::<Vec<_>>();
    let base_result = WaitAgentResult {
        status: bounded
            .iter()
            .cloned()
            .map(|(thread_id, status)| (thread_id.to_string(), status))
            .collect(),
        timed_out: bounded.is_empty(),
    };
    let base_len = serde_json::to_vec(&base_result)
        .map(|serialized| serialized.len())
        .unwrap_or(MAX_PROJECTED_WAIT_RESULT_BYTES);
    let mut remaining = MAX_PROJECTED_WAIT_RESULT_BYTES.saturating_sub(base_len);
    let mut text_statuses_remaining = statuses
        .iter()
        .filter(|(_, status)| status_text(status).is_some())
        .count();

    for ((_, original), (_, target)) in statuses.iter().zip(&mut bounded) {
        let Some(text) = status_text(original) else {
            continue;
        };
        let base_status_len = serde_json::to_vec(target)
            .map(|serialized| serialized.len())
            .unwrap_or(MAX_PROJECTED_WAIT_STATUS_BYTES);
        let per_status_budget = MAX_PROJECTED_WAIT_STATUS_BYTES.saturating_sub(base_status_len);
        let fair_share = remaining / text_statuses_remaining.max(1);
        let text_budget = per_status_budget.min(fair_share);
        let bounded_text = truncate_json_string_content(text, text_budget);
        let replacement = match original {
            AgentStatus::Completed(_) => AgentStatus::Completed(Some(bounded_text)),
            AgentStatus::Errored(_) => AgentStatus::Errored(bounded_text),
            AgentStatus::PendingInit
            | AgentStatus::Running
            | AgentStatus::Interrupted
            | AgentStatus::Shutdown
            | AgentStatus::NotFound => unreachable!("status_text returned text for unit variant"),
        };
        let replacement_len = serde_json::to_vec(&replacement)
            .map(|serialized| serialized.len())
            .unwrap_or(base_status_len);
        remaining = remaining.saturating_sub(replacement_len.saturating_sub(base_status_len));
        text_statuses_remaining = text_statuses_remaining.saturating_sub(1);
        *target = replacement;
    }

    bounded
}

fn status_without_text(status: &AgentStatus) -> AgentStatus {
    match status {
        AgentStatus::Completed(Some(_)) => AgentStatus::Completed(Some(String::new())),
        AgentStatus::Errored(_) => AgentStatus::Errored(String::new()),
        status => status.clone(),
    }
}

fn status_text(status: &AgentStatus) -> Option<&str> {
    match status {
        AgentStatus::Completed(Some(text)) | AgentStatus::Errored(text) => Some(text),
        AgentStatus::PendingInit
        | AgentStatus::Running
        | AgentStatus::Interrupted
        | AgentStatus::Completed(None)
        | AgentStatus::Shutdown
        | AgentStatus::NotFound => None,
    }
}

fn truncate_json_string_content(value: &str, escaped_budget: usize) -> String {
    let value_len = escaped_json_string_content_len(value);
    if value_len <= escaped_budget {
        return value.to_string();
    }

    let marker_len = escaped_json_string_content_len(PROJECTED_WAIT_TRUNCATION_MARKER);
    if marker_len > escaped_budget {
        return String::new();
    }
    let prefix_budget = escaped_budget - marker_len;
    let mut prefix_end = 0;
    let mut prefix_len = 0;
    for (index, character) in value.char_indices() {
        let character_len = escaped_json_character_len(character);
        if prefix_len + character_len > prefix_budget {
            break;
        }
        prefix_len += character_len;
        prefix_end = index + character.len_utf8();
    }

    let mut truncated = value[..prefix_end].to_string();
    truncated.push_str(PROJECTED_WAIT_TRUNCATION_MARKER);
    truncated
}

fn escaped_json_string_content_len(value: &str) -> usize {
    value.chars().map(escaped_json_character_len).sum()
}

fn escaped_json_character_len(character: char) -> usize {
    match character {
        '"' | '\\' | '\u{0008}' | '\u{000C}' | '\n' | '\r' | '\t' => 2,
        '\u{0000}'..='\u{001F}' => 6,
        character => character.len_utf8(),
    }
}

fn wait_tool_call_status(statuses: &HashMap<ThreadId, AgentStatus>) -> CollabAgentToolCallStatus {
    if statuses
        .values()
        .any(|status| matches!(status, AgentStatus::Errored(_) | AgentStatus::NotFound))
    {
        CollabAgentToolCallStatus::Failed
    } else {
        CollabAgentToolCallStatus::Completed
    }
}

fn wait_receiver_agents(
    statuses: &HashMap<ThreadId, AgentStatus>,
    receiver_agents: &[CollabAgentRef],
) -> Vec<CollabAgentRef> {
    if statuses.is_empty() {
        return Vec::new();
    }

    let mut agents = Vec::with_capacity(statuses.len());
    let mut seen = HashMap::with_capacity(receiver_agents.len());
    for receiver_agent in receiver_agents {
        seen.insert(receiver_agent.thread_id, ());
        if statuses.contains_key(&receiver_agent.thread_id) {
            agents.push(receiver_agent.clone());
        }
    }

    let mut extras = statuses
        .keys()
        .filter(|thread_id| !seen.contains_key(thread_id))
        .map(|thread_id| CollabAgentRef {
            thread_id: *thread_id,
            agent_nickname: None,
            agent_role: None,
        })
        .collect::<Vec<_>>();
    extras.sort_by_key(|agent| agent.thread_id.to_string());
    agents.extend(extras);
    agents
}

impl CoreToolRuntime for Handler {
    fn matches_kind(&self, payload: &ToolPayload) -> bool {
        matches!(payload, ToolPayload::Function { .. })
    }
}

#[derive(Debug, Deserialize)]
struct WaitArgs {
    #[serde(default)]
    targets: Vec<String>,
    timeout_ms: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct WaitAgentResult {
    pub(crate) status: HashMap<String, AgentStatus>,
    pub(crate) timed_out: bool,
}

impl ToolOutput for WaitAgentResult {
    fn log_preview(&self) -> String {
        tool_output_json_text(self, "wait_agent")
    }

    fn success_for_logging(&self) -> bool {
        true
    }

    fn to_response_item(&self, call_id: &str, payload: &ToolPayload) -> ResponseInputItem {
        tool_output_response_item(call_id, payload, self, /*success*/ None, "wait_agent")
    }

    fn code_mode_result(&self, _payload: &ToolPayload) -> JsonValue {
        tool_output_code_mode_result(self, "wait_agent")
    }
}

async fn wait_for_final_status(
    session: Arc<Session>,
    thread_id: ThreadId,
    mut status_rx: Receiver<AgentStatus>,
) -> Option<(ThreadId, AgentStatus)> {
    let mut status = status_rx.borrow().clone();
    if is_final(&status) {
        return Some((thread_id, status));
    }

    loop {
        if status_rx.changed().await.is_err() {
            let latest = session.services.agent_control.get_status(thread_id).await;
            return is_final(&latest).then_some((thread_id, latest));
        }
        status = status_rx.borrow().clone();
        if is_final(&status) {
            return Some((thread_id, status));
        }
    }
}
