//! Helpers for rendering and navigating multi-agent state in the TUI.
//!
//! This module owns the shared presentation contracts for multi-agent history rows, `/agent` picker
//! entries, and the fast-switch keyboard shortcuts. Higher-level coordination, such as deciding
//! which thread becomes active or when a thread closes, stays in [`crate::app::App`].

use crate::history_cell::PlainHistoryCell;
use crate::render::line_utils::prefix_lines;
use crate::text_formatting::truncate_text;
use chrono::TimeZone;
use chrono::Utc;
use codex_app_server_protocol::CollabAgentState;
use codex_app_server_protocol::CollabAgentStatus;
use codex_app_server_protocol::CollabAgentTool;
use codex_app_server_protocol::CollabAgentToolCallStatus;
use codex_app_server_protocol::SubAgentActivityKind;
use codex_app_server_protocol::ThreadActiveFlag;
use codex_app_server_protocol::ThreadItem;
use codex_app_server_protocol::ThreadStatus;
use codex_app_server_protocol::ThreadTokenUsage;
use codex_app_server_protocol::TurnPlanStep;
use codex_app_server_protocol::TurnPlanStepStatus;
use codex_protocol::ThreadId;
use codex_protocol::num_format::format_with_separators;
use codex_protocol::openai_models::ReasoningEffort as ReasoningEffortConfig;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
#[cfg(target_os = "macos")]
use crossterm::event::KeyEventKind;
#[cfg(target_os = "macos")]
use crossterm::event::KeyModifiers;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use std::collections::HashSet;

const COLLAB_PROMPT_PREVIEW_GRAPHEMES: usize = 160;
const COLLAB_AGENT_ERROR_PREVIEW_GRAPHEMES: usize = 160;
const COLLAB_AGENT_RESPONSE_PREVIEW_GRAPHEMES: usize = 240;
const AGENT_ACTIVITY_PREVIEW_GRAPHEMES: usize = 240;
const AGENT_PICKER_DETAIL_ACTIVITY_ITEMS: usize = 3;
const AGENT_PICKER_PROMPT_PREVIEW_GRAPHEMES: usize = 160;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentPickerThreadEntry {
    /// Human-friendly nickname shown in picker rows and footer labels.
    pub(crate) agent_nickname: Option<String>,
    /// Agent type shown in brackets when present, for example `worker`.
    pub(crate) agent_role: Option<String>,
    /// Canonical v2 agent path, when the thread was observed through v2 activity.
    pub(crate) agent_path: Option<String>,
    /// Bounded app-server preview, usually the first prompt for this thread.
    pub(crate) prompt_preview: Option<String>,
    /// Thread note captured in native sub-agent spawn metadata.
    pub(crate) thread_note: Option<String>,
    /// Working directory captured for this thread by app-server thread metadata.
    pub(crate) cwd: Option<String>,
    /// Model provider captured for this thread by app-server thread metadata.
    pub(crate) model_provider: Option<String>,
    /// Thread creation timestamp from native app-server thread metadata.
    pub(crate) created_at: Option<i64>,
    /// Thread last-updated timestamp from native app-server thread metadata.
    pub(crate) updated_at: Option<i64>,
    /// Model captured from the TUI thread session state when available.
    pub(crate) model: Option<String>,
    /// Reasoning effort captured from the TUI thread session state when available.
    pub(crate) reasoning_effort: Option<String>,
    /// Service tier captured from the TUI thread session state when available.
    pub(crate) service_tier: Option<String>,
    /// Latest status derived from native app-server/runtime state.
    pub(crate) status: AgentPickerThreadStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentPickerThreadStatus {
    Idle,
    Running,
    WaitingApproval,
    WaitingUser,
    Error,
    Closed,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct AgentRoleRuntimeUsage {
    pub(crate) total: usize,
    pub(crate) running: usize,
    pub(crate) waiting: usize,
    pub(crate) errors: usize,
    pub(crate) closed: usize,
    pub(crate) current_view: bool,
}

impl AgentPickerThreadStatus {
    pub(crate) fn from_thread_status(status: &ThreadStatus) -> Self {
        match status {
            ThreadStatus::NotLoaded => Self::Closed,
            ThreadStatus::Idle => Self::Idle,
            ThreadStatus::SystemError => Self::Error,
            ThreadStatus::Active { active_flags } => {
                if active_flags.contains(&ThreadActiveFlag::WaitingOnApproval) {
                    Self::WaitingApproval
                } else if active_flags.contains(&ThreadActiveFlag::WaitingOnUserInput) {
                    Self::WaitingUser
                } else {
                    Self::Running
                }
            }
        }
    }

    pub(crate) fn is_closed(self) -> bool {
        matches!(self, Self::Closed)
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Running => "running",
            Self::WaitingApproval => "waiting approval",
            Self::WaitingUser => "waiting user",
            Self::Error => "error",
            Self::Closed => "closed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SubAgentActivityDisplay {
    pub(crate) thread_id: ThreadId,
    pub(crate) agent_path: String,
    pub(crate) is_running_hint: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct AgentMetadata {
    /// Human-friendly nickname shown in rendered tool-call rows.
    pub(crate) agent_nickname: Option<String>,
    /// Agent type shown in brackets when present, for example `worker`.
    pub(crate) agent_role: Option<String>,
}

#[derive(Clone, Copy)]
struct AgentLabel<'a> {
    thread_id: Option<ThreadId>,
    nickname: Option<&'a str>,
    role: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpawnRequestSummary {
    pub(crate) model: String,
    pub(crate) reasoning_effort: ReasoningEffortConfig,
}

pub(crate) fn agent_picker_status_dot_spans(status: AgentPickerThreadStatus) -> Vec<Span<'static>> {
    let dot = if status.is_closed() {
        "•".into()
    } else if matches!(status, AgentPickerThreadStatus::Error) {
        "•".red()
    } else if matches!(
        status,
        AgentPickerThreadStatus::WaitingApproval | AgentPickerThreadStatus::WaitingUser
    ) {
        "•".cyan()
    } else {
        "•".green()
    };
    vec![dot, " ".into()]
}

pub(crate) fn agent_picker_item_description(
    thread_id: ThreadId,
    entry: &AgentPickerThreadEntry,
    is_primary: bool,
) -> String {
    let mut parts = vec![agent_picker_status_label(entry).to_string()];
    if is_primary {
        parts.push("main thread".to_string());
    } else if let Some(role) = entry
        .agent_role
        .as_deref()
        .filter(|role| !role.trim().is_empty())
    {
        parts.push(format!("role {role}"));
    } else {
        parts.push("agent".to_string());
    }
    if let Some(agent_path) = entry
        .agent_path
        .as_deref()
        .map(str::trim)
        .filter(|agent_path| !agent_path.is_empty())
    {
        parts.push(agent_path.to_string());
    } else {
        parts.push(short_thread_id(thread_id));
    }
    parts.join(" · ")
}

pub(crate) fn agent_picker_summary_line(
    threads: &[(ThreadId, &AgentPickerThreadEntry)],
    primary_thread_id: Option<ThreadId>,
) -> String {
    let mut total = 0usize;
    let mut running = 0usize;
    let mut waiting = 0usize;
    let mut errors = 0usize;
    let mut closed = 0usize;
    for (thread_id, entry) in threads {
        if Some(*thread_id) == primary_thread_id {
            continue;
        }
        total += 1;
        match entry.status {
            AgentPickerThreadStatus::Idle => {}
            AgentPickerThreadStatus::Running => running += 1,
            AgentPickerThreadStatus::WaitingApproval | AgentPickerThreadStatus::WaitingUser => {
                waiting += 1;
            }
            AgentPickerThreadStatus::Error => errors += 1,
            AgentPickerThreadStatus::Closed => closed += 1,
        }
    }
    format!(
        "Agents: {total} total · {running} running · {waiting} waiting · {errors} error · {closed} closed"
    )
}

pub(crate) struct AgentPickerSelectedDescriptionContext<'a> {
    pub(crate) prompt_context: &'a [String],
    pub(crate) recent_activity: &'a [String],
    pub(crate) token_usage_summary: Option<&'a str>,
    pub(crate) plan_progress_summary: Option<&'a str>,
}

pub(crate) fn agent_picker_selected_description(
    thread_id: ThreadId,
    entry: &AgentPickerThreadEntry,
    is_primary: bool,
    is_current: bool,
    context: AgentPickerSelectedDescriptionContext<'_>,
) -> String {
    let mut lines = vec![
        format!("Status: {}", agent_picker_status_label(entry)),
        format!("Current view: {}", if is_current { "yes" } else { "no" }),
    ];
    if is_primary {
        lines.push("Kind: main thread".to_string());
    } else {
        lines.push("Kind: sub-agent thread".to_string());
    }

    let mut inspect_lines = Vec::new();
    if let Some(nickname) = entry
        .agent_nickname
        .as_deref()
        .map(str::trim)
        .filter(|nickname| !nickname.is_empty())
    {
        inspect_lines.push(format!("Nickname: {nickname}"));
    }
    if let Some(role) = entry
        .agent_role
        .as_deref()
        .map(str::trim)
        .filter(|role| !role.is_empty())
    {
        inspect_lines.push(format!("Role: {role}"));
    }
    if let Some(agent_path) = entry
        .agent_path
        .as_deref()
        .map(str::trim)
        .filter(|agent_path| !agent_path.is_empty())
    {
        inspect_lines.push(format!("Agent path: {agent_path}"));
    }
    if let Some(prompt_preview) = agent_picker_prompt_preview(entry) {
        inspect_lines.push(format!("Prompt: {prompt_preview}"));
    }
    if let Some(thread_note) = entry
        .thread_note
        .as_deref()
        .map(str::trim)
        .filter(|thread_note| !thread_note.is_empty())
    {
        inspect_lines.push(format!("Note: {thread_note}"));
    }
    if let Some(cwd) = entry
        .cwd
        .as_deref()
        .map(str::trim)
        .filter(|cwd| !cwd.is_empty())
    {
        inspect_lines.push(format!("Cwd: {cwd}"));
    }
    if let Some(model_provider) = entry
        .model_provider
        .as_deref()
        .map(str::trim)
        .filter(|model_provider| !model_provider.is_empty())
    {
        inspect_lines.push(format!("Model provider: {model_provider}"));
    }
    if let Some(created_at) = entry.created_at.and_then(format_agent_picker_timestamp) {
        inspect_lines.push(format!("Created: {created_at}"));
    }
    if let Some(updated_at) = entry.updated_at.and_then(format_agent_picker_timestamp) {
        inspect_lines.push(format!("Updated: {updated_at}"));
    }
    if let Some(model) = entry
        .model
        .as_deref()
        .map(str::trim)
        .filter(|model| !model.is_empty())
    {
        inspect_lines.push(format!("Model: {model}"));
    }
    if let Some(reasoning_effort) = entry
        .reasoning_effort
        .as_deref()
        .map(str::trim)
        .filter(|reasoning_effort| !reasoning_effort.is_empty())
    {
        inspect_lines.push(format!("Reasoning: {reasoning_effort}"));
    }
    if let Some(service_tier) = entry
        .service_tier
        .as_deref()
        .map(str::trim)
        .filter(|service_tier| !service_tier.is_empty())
    {
        inspect_lines.push(format!("Service tier: {service_tier}"));
    }
    if let Some(token_usage_summary) = context
        .token_usage_summary
        .map(str::trim)
        .filter(|token_usage_summary| !token_usage_summary.is_empty())
    {
        inspect_lines.push(format!("Tokens: {token_usage_summary}"));
    }
    if let Some(plan_progress_summary) = context
        .plan_progress_summary
        .map(str::trim)
        .filter(|plan_progress_summary| !plan_progress_summary.is_empty())
    {
        inspect_lines.push(format!("Plan: {plan_progress_summary}"));
    }
    inspect_lines.push(format!("Thread: {thread_id}"));
    lines.push("Inspect:".to_string());
    lines.extend(
        inspect_lines
            .into_iter()
            .map(|detail| format!("- {detail}")),
    );
    if !context.prompt_context.is_empty() {
        lines.push("Context:".to_string());
        lines.extend(
            context
                .prompt_context
                .iter()
                .map(|context| format!("- {context}")),
        );
    }
    if !context.recent_activity.is_empty() {
        lines.push("Recent activity:".to_string());
        lines.extend(
            context
                .recent_activity
                .iter()
                .map(|activity| format!("- {activity}")),
        );
    }
    lines.push("Actions:".to_string());
    lines.push("- Enter: watch this thread".to_string());
    lines.join("\n")
}

fn agent_picker_status_label(entry: &AgentPickerThreadEntry) -> &'static str {
    entry.status.label()
}

fn short_thread_id(thread_id: ThreadId) -> String {
    thread_id.to_string().chars().take(8).collect::<String>()
}

fn agent_picker_prompt_preview(entry: &AgentPickerThreadEntry) -> Option<String> {
    bounded_prompt_preview(entry.prompt_preview.as_deref()?)
}

fn format_agent_picker_timestamp(timestamp_seconds: i64) -> Option<String> {
    Utc.timestamp_opt(timestamp_seconds, 0)
        .single()
        .map(|timestamp| timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string())
}

fn bounded_prompt_preview(preview: &str) -> Option<String> {
    let preview = preview.trim();
    if preview.is_empty() {
        return None;
    }
    bounded_activity_summary(&truncate_text(
        preview,
        AGENT_PICKER_PROMPT_PREVIEW_GRAPHEMES,
    ))
}

pub(crate) fn format_agent_picker_item_name(
    agent_nickname: Option<&str>,
    agent_role: Option<&str>,
    is_primary: bool,
) -> String {
    if is_primary {
        return "Main [default]".to_string();
    }

    let agent_nickname = agent_nickname
        .map(str::trim)
        .filter(|nickname| !nickname.is_empty());
    let agent_role = agent_role.map(str::trim).filter(|role| !role.is_empty());
    match (agent_nickname, agent_role) {
        (Some(agent_nickname), Some(agent_role)) => format!("{agent_nickname} [{agent_role}]"),
        (Some(agent_nickname), None) => agent_nickname.to_string(),
        (None, Some(agent_role)) => format!("[{agent_role}]"),
        (None, None) => "Agent".to_string(),
    }
}

pub(crate) fn previous_agent_shortcut() -> crate::key_hint::KeyBinding {
    crate::key_hint::alt(KeyCode::Left)
}

pub(crate) fn next_agent_shortcut() -> crate::key_hint::KeyBinding {
    crate::key_hint::alt(KeyCode::Right)
}

/// Matches the canonical "previous agent" binding plus platform-specific fallbacks that keep agent
/// navigation working when enhanced key reporting is unavailable.
pub(crate) fn previous_agent_shortcut_matches(
    key_event: KeyEvent,
    allow_word_motion_fallback: bool,
) -> bool {
    previous_agent_shortcut().is_press(key_event)
        || previous_agent_word_motion_fallback(key_event, allow_word_motion_fallback)
}

/// Matches the canonical "next agent" binding plus platform-specific fallbacks that keep agent
/// navigation working when enhanced key reporting is unavailable.
pub(crate) fn next_agent_shortcut_matches(
    key_event: KeyEvent,
    allow_word_motion_fallback: bool,
) -> bool {
    next_agent_shortcut().is_press(key_event)
        || next_agent_word_motion_fallback(key_event, allow_word_motion_fallback)
}

#[cfg(target_os = "macos")]
fn previous_agent_word_motion_fallback(
    key_event: KeyEvent,
    allow_word_motion_fallback: bool,
) -> bool {
    // Some terminals, especially on macOS, send Option+b/f as word-motion keys instead of
    // Option+arrow events unless enhanced keyboard reporting is enabled. Callers should only
    // enable this fallback when the composer is empty so draft editing retains the expected
    // word-wise motion behavior.
    allow_word_motion_fallback
        && matches!(
            key_event,
            KeyEvent {
                code: KeyCode::Char('b'),
                modifiers: KeyModifiers::ALT,
                kind: KeyEventKind::Press | KeyEventKind::Repeat,
                ..
            }
        )
}

#[cfg(not(target_os = "macos"))]
fn previous_agent_word_motion_fallback(
    _key_event: KeyEvent,
    _allow_word_motion_fallback: bool,
) -> bool {
    false
}

#[cfg(target_os = "macos")]
fn next_agent_word_motion_fallback(key_event: KeyEvent, allow_word_motion_fallback: bool) -> bool {
    // Some terminals, especially on macOS, send Option+b/f as word-motion keys instead of
    // Option+arrow events unless enhanced keyboard reporting is enabled. Callers should only
    // enable this fallback when the composer is empty so draft editing retains the expected
    // word-wise motion behavior.
    allow_word_motion_fallback
        && matches!(
            key_event,
            KeyEvent {
                code: KeyCode::Char('f'),
                modifiers: KeyModifiers::ALT,
                kind: KeyEventKind::Press | KeyEventKind::Repeat,
                ..
            }
        )
}

#[cfg(not(target_os = "macos"))]
fn next_agent_word_motion_fallback(
    _key_event: KeyEvent,
    _allow_word_motion_fallback: bool,
) -> bool {
    false
}

pub(crate) fn spawn_request_summary(item: &ThreadItem) -> Option<SpawnRequestSummary> {
    match item {
        ThreadItem::CollabAgentToolCall {
            tool: CollabAgentTool::SpawnAgent,
            model: Some(model),
            reasoning_effort: Some(reasoning_effort),
            ..
        } => Some(SpawnRequestSummary {
            model: model.clone(),
            reasoning_effort: reasoning_effort.clone(),
        }),
        _ => None,
    }
}

pub(crate) fn agent_picker_recent_activity_summaries<'a>(
    items_newest_first: impl Iterator<Item = &'a ThreadItem>,
) -> Vec<String> {
    let mut seen_item_ids = HashSet::new();
    let mut activity = Vec::new();
    for item in items_newest_first {
        if !seen_item_ids.insert(item.id().to_string()) {
            continue;
        }
        if let Some(summary) = thread_item_activity_summary(item) {
            activity.push(summary);
            if activity.len() == AGENT_PICKER_DETAIL_ACTIVITY_ITEMS {
                break;
            }
        }
    }
    activity.reverse();
    activity
}

pub(crate) fn agent_picker_prompt_context_summaries<'a>(
    receiver_thread_id: ThreadId,
    items_newest_first: impl Iterator<Item = &'a ThreadItem>,
) -> Vec<String> {
    let receiver_thread_id = receiver_thread_id.to_string();
    let mut initial_request = None;
    let mut latest_input = None;
    for item in items_newest_first {
        let ThreadItem::CollabAgentToolCall {
            tool,
            receiver_thread_ids,
            prompt,
            ..
        } = item
        else {
            continue;
        };
        if !receiver_thread_ids
            .iter()
            .any(|thread_id| thread_id == &receiver_thread_id)
        {
            continue;
        }
        let Some(prompt) = prompt.as_deref().and_then(bounded_prompt_preview) else {
            continue;
        };
        match tool {
            CollabAgentTool::SpawnAgent if initial_request.is_none() => {
                initial_request = Some(format!("Initial request: {prompt}"));
            }
            CollabAgentTool::SendInput if latest_input.is_none() => {
                latest_input = Some(format!("Latest input: {prompt}"));
            }
            CollabAgentTool::ResumeAgent
            | CollabAgentTool::Wait
            | CollabAgentTool::CloseAgent
            | CollabAgentTool::SpawnAgent
            | CollabAgentTool::SendInput => {}
        }
        if initial_request.is_some() && latest_input.is_some() {
            break;
        }
    }
    [initial_request, latest_input]
        .into_iter()
        .flatten()
        .collect()
}

pub(crate) fn thread_item_activity_summary(item: &ThreadItem) -> Option<String> {
    let summary = match item {
        ThreadItem::AgentMessage { text, .. } | ThreadItem::Plan { text, .. } => text,
        ThreadItem::Reasoning { summary, .. } => summary.last()?,
        ThreadItem::CommandExecution { command, .. } => {
            let command = truncate_text(
                command,
                AGENT_ACTIVITY_PREVIEW_GRAPHEMES.saturating_sub("$ ".len()),
            );
            return bounded_activity_summary(&format!("$ {command}"));
        }
        ThreadItem::FileChange { changes, .. } => {
            return bounded_activity_summary(&format!("Updated {} file(s)", changes.len()));
        }
        ThreadItem::McpToolCall { server, tool, .. } => {
            return bounded_activity_summary(&format!("MCP {server}/{tool}"));
        }
        ThreadItem::DynamicToolCall {
            namespace, tool, ..
        } => {
            let tool = namespace
                .as_ref()
                .map(|namespace| format!("{namespace}/{tool}"))
                .unwrap_or_else(|| tool.clone());
            return bounded_activity_summary(&format!("Tool {tool}"));
        }
        ThreadItem::CollabAgentToolCall { tool, .. } => {
            let action = match tool {
                CollabAgentTool::SpawnAgent => "Spawned an agent",
                CollabAgentTool::SendInput => "Sent input to an agent",
                CollabAgentTool::ResumeAgent => "Resumed an agent",
                CollabAgentTool::Wait => "Waited for an agent",
                CollabAgentTool::CloseAgent => "Closed an agent",
            };
            return Some(action.to_string());
        }
        ThreadItem::SubAgentActivity {
            kind, agent_path, ..
        } => return bounded_activity_summary(&sub_agent_activity_summary(*kind, agent_path)),
        ThreadItem::WebSearch { query, .. } => {
            return bounded_activity_summary(&format!("Web search: {query}"));
        }
        ThreadItem::ImageView { path, .. } => {
            return bounded_activity_summary(&format!("Viewed {}", path.display()));
        }
        ThreadItem::ImageGeneration { .. } => return Some("Generated an image".to_string()),
        ThreadItem::EnteredReviewMode { .. } => return Some("Entered review mode".to_string()),
        ThreadItem::ExitedReviewMode { .. } => return Some("Exited review mode".to_string()),
        ThreadItem::ContextCompaction { .. } => return Some("Compacted context".to_string()),
        ThreadItem::UserMessage { .. } | ThreadItem::HookPrompt { .. } => return None,
    };
    bounded_activity_summary(summary)
}

fn bounded_activity_summary(summary: &str) -> Option<String> {
    let summary = truncate_text(summary, AGENT_ACTIVITY_PREVIEW_GRAPHEMES);
    let summary = summary.split_whitespace().collect::<Vec<_>>().join(" ");
    (!summary.is_empty()).then_some(summary)
}

pub(crate) fn agent_picker_token_usage_summary(token_usage: &ThreadTokenUsage) -> Option<String> {
    let total = token_usage.total.total_tokens.max(0);
    let last = token_usage.last.total_tokens.max(0);
    if total == 0 && last == 0 && token_usage.model_context_window.is_none() {
        return None;
    }

    let mut parts = vec![
        format!("total {}", format_with_separators(total)),
        format!("last {}", format_with_separators(last)),
    ];
    if let Some(model_context_window) = token_usage.model_context_window {
        parts.push(format!(
            "context {}/{}",
            format_with_separators(last),
            format_with_separators(model_context_window.max(0))
        ));
    }
    Some(parts.join(" · "))
}

pub(crate) fn agent_picker_plan_progress_summary(plan: &[TurnPlanStep]) -> Option<String> {
    if plan.is_empty() {
        return None;
    }

    let total = plan.len();
    let completed = plan
        .iter()
        .filter(|step| matches!(step.status, TurnPlanStepStatus::Completed))
        .count();
    let mut summary = if completed == total {
        format!("all {total} complete")
    } else {
        format!("{completed}/{total} complete")
    };
    if let Some(current_step) = plan
        .iter()
        .find(|step| matches!(step.status, TurnPlanStepStatus::InProgress))
        .and_then(|step| bounded_activity_summary(&step.step))
    {
        summary.push_str(" · now: ");
        summary.push_str(&current_step);
    }
    Some(summary)
}

pub(crate) fn tool_call_history_cell(
    item: &ThreadItem,
    cached_spawn_request: Option<&SpawnRequestSummary>,
    mut agent_metadata: impl FnMut(ThreadId) -> AgentMetadata,
) -> Option<PlainHistoryCell> {
    let ThreadItem::CollabAgentToolCall {
        tool,
        status,
        receiver_thread_ids,
        prompt,
        agents_states,
        ..
    } = item
    else {
        return None;
    };

    let first_receiver = receiver_thread_ids
        .first()
        .and_then(|id| parse_thread_id(id));
    let prompt = prompt.as_deref().unwrap_or_default();

    match tool {
        CollabAgentTool::SpawnAgent => {
            if matches!(status, CollabAgentToolCallStatus::InProgress) {
                return None;
            }
            let fallback_spawn_request = spawn_request_summary(item);
            let spawn_request = cached_spawn_request.or(fallback_spawn_request.as_ref());
            Some(spawn_end(
                first_receiver,
                prompt,
                spawn_request,
                &mut agent_metadata,
            ))
        }
        CollabAgentTool::SendInput => {
            if matches!(status, CollabAgentToolCallStatus::InProgress) {
                return None;
            }
            first_receiver.map(|receiver_thread_id| {
                interaction_end(receiver_thread_id, prompt, &mut agent_metadata)
            })
        }
        CollabAgentTool::ResumeAgent => first_receiver.map(|receiver_thread_id| {
            if matches!(status, CollabAgentToolCallStatus::InProgress) {
                resume_begin(receiver_thread_id, &mut agent_metadata)
            } else {
                let state = first_agent_state(receiver_thread_ids, agents_states);
                resume_end(
                    receiver_thread_id,
                    state,
                    "Agent resume failed",
                    &mut agent_metadata,
                )
            }
        }),
        CollabAgentTool::Wait => {
            if matches!(status, CollabAgentToolCallStatus::InProgress) {
                Some(waiting_begin(receiver_thread_ids, &mut agent_metadata))
            } else {
                Some(waiting_end(
                    receiver_thread_ids,
                    agents_states,
                    &mut agent_metadata,
                ))
            }
        }
        CollabAgentTool::CloseAgent => {
            if matches!(status, CollabAgentToolCallStatus::InProgress) {
                return None;
            }
            first_receiver
                .map(|receiver_thread_id| close_end(receiver_thread_id, &mut agent_metadata))
        }
    }
}

pub(crate) fn sub_agent_activity_display(item: &ThreadItem) -> Option<SubAgentActivityDisplay> {
    let ThreadItem::SubAgentActivity {
        kind,
        agent_thread_id,
        agent_path,
        ..
    } = item
    else {
        return None;
    };
    Some(SubAgentActivityDisplay {
        thread_id: parse_thread_id(agent_thread_id)?,
        agent_path: agent_path.clone(),
        is_running_hint: !matches!(kind, SubAgentActivityKind::Interrupted),
    })
}

pub(crate) fn sub_agent_activity_history_cell(item: &ThreadItem) -> Option<PlainHistoryCell> {
    let ThreadItem::SubAgentActivity {
        kind, agent_path, ..
    } = item
    else {
        return None;
    };
    Some(collab_event(
        sub_agent_activity_title(*kind, agent_path),
        Vec::new(),
    ))
}

pub(crate) fn sub_agent_activity_summary(kind: SubAgentActivityKind, agent_path: &str) -> String {
    match kind {
        SubAgentActivityKind::Started => format!("Started `{agent_path}`"),
        SubAgentActivityKind::Interacted => format!("Interacted with `{agent_path}`"),
        SubAgentActivityKind::Interrupted => format!("Interrupted `{agent_path}`"),
    }
}

fn sub_agent_activity_title(kind: SubAgentActivityKind, agent_path: &str) -> Line<'static> {
    let (prefix, path) = match kind {
        SubAgentActivityKind::Started => ("Started ", agent_path),
        SubAgentActivityKind::Interacted => ("Interacted with ", agent_path),
        SubAgentActivityKind::Interrupted => ("Interrupted ", agent_path),
    };
    title_spans_line(vec![
        Span::from(prefix).bold(),
        Span::from(format!("`{path}`")).cyan(),
    ])
}

fn spawn_end(
    new_thread_id: Option<ThreadId>,
    prompt: &str,
    spawn_request: Option<&SpawnRequestSummary>,
    agent_metadata: &mut impl FnMut(ThreadId) -> AgentMetadata,
) -> PlainHistoryCell {
    let title = match new_thread_id {
        Some(thread_id) => title_with_agent(
            "Spawned",
            agent_label(thread_id, &agent_metadata(thread_id)),
            spawn_request,
        ),
        None => title_text("Agent spawn failed"),
    };

    let mut details = Vec::new();
    if let Some(line) = prompt_line(prompt) {
        details.push(line);
    }
    collab_event(title, details)
}

fn interaction_end(
    receiver_thread_id: ThreadId,
    prompt: &str,
    agent_metadata: &mut impl FnMut(ThreadId) -> AgentMetadata,
) -> PlainHistoryCell {
    let title = title_with_agent(
        "Sent input to",
        agent_label(receiver_thread_id, &agent_metadata(receiver_thread_id)),
        /*spawn_request*/ None,
    );

    let mut details = Vec::new();
    if let Some(line) = prompt_line(prompt) {
        details.push(line);
    }
    collab_event(title, details)
}

fn waiting_begin(
    receiver_thread_ids: &[String],
    agent_metadata: &mut impl FnMut(ThreadId) -> AgentMetadata,
) -> PlainHistoryCell {
    let receiver_agents = receiver_thread_ids
        .iter()
        .filter_map(|thread_id| parse_thread_id(thread_id))
        .map(|thread_id| (thread_id, agent_metadata(thread_id)))
        .collect::<Vec<_>>();

    let title = match receiver_agents.as_slice() {
        [(thread_id, metadata)] => title_with_agent(
            "Waiting for",
            agent_label(*thread_id, metadata),
            /*spawn_request*/ None,
        ),
        [] => title_text("Waiting for agents"),
        _ => title_text(format!("Waiting for {} agents", receiver_agents.len())),
    };

    let details = if receiver_agents.len() > 1 {
        receiver_agents
            .iter()
            .map(|(thread_id, metadata)| agent_label_line(agent_label(*thread_id, metadata)))
            .collect()
    } else {
        Vec::new()
    };

    collab_event(title, details)
}

fn waiting_end(
    receiver_thread_ids: &[String],
    agents_states: &std::collections::HashMap<String, CollabAgentState>,
    agent_metadata: &mut impl FnMut(ThreadId) -> AgentMetadata,
) -> PlainHistoryCell {
    let details = wait_complete_lines(receiver_thread_ids, agents_states, agent_metadata);
    collab_event(title_text("Finished waiting"), details)
}

fn close_end(
    receiver_thread_id: ThreadId,
    agent_metadata: &mut impl FnMut(ThreadId) -> AgentMetadata,
) -> PlainHistoryCell {
    collab_event(
        title_with_agent(
            "Closed",
            agent_label(receiver_thread_id, &agent_metadata(receiver_thread_id)),
            /*spawn_request*/ None,
        ),
        Vec::new(),
    )
}

fn resume_begin(
    receiver_thread_id: ThreadId,
    agent_metadata: &mut impl FnMut(ThreadId) -> AgentMetadata,
) -> PlainHistoryCell {
    collab_event(
        title_with_agent(
            "Resuming",
            agent_label(receiver_thread_id, &agent_metadata(receiver_thread_id)),
            /*spawn_request*/ None,
        ),
        Vec::new(),
    )
}

fn resume_end(
    receiver_thread_id: ThreadId,
    status: Option<&CollabAgentState>,
    fallback_error: &str,
    agent_metadata: &mut impl FnMut(ThreadId) -> AgentMetadata,
) -> PlainHistoryCell {
    collab_event(
        title_with_agent(
            "Resumed",
            agent_label(receiver_thread_id, &agent_metadata(receiver_thread_id)),
            /*spawn_request*/ None,
        ),
        vec![status_summary_line(status, fallback_error)],
    )
}

fn collab_event(title: Line<'static>, details: Vec<Line<'static>>) -> PlainHistoryCell {
    let mut lines: Vec<Line<'static>> = vec![title];
    if !details.is_empty() {
        lines.extend(prefix_lines(details, "  └ ".dim(), "    ".into()));
    }
    PlainHistoryCell::new(lines)
}

fn title_text(title: impl Into<String>) -> Line<'static> {
    title_spans_line(vec![Span::from(title.into()).bold()])
}

fn title_with_agent(
    prefix: &str,
    agent: AgentLabel<'_>,
    spawn_request: Option<&SpawnRequestSummary>,
) -> Line<'static> {
    let mut spans = vec![Span::from(format!("{prefix} ")).bold()];
    spans.extend(agent_label_spans(agent));
    spans.extend(spawn_request_spans(spawn_request));
    title_spans_line(spans)
}

fn title_spans_line(mut spans: Vec<Span<'static>>) -> Line<'static> {
    let mut title = Vec::with_capacity(spans.len() + 1);
    title.push(Span::from("• ").dim());
    title.append(&mut spans);
    title.into()
}

fn parse_thread_id(thread_id: &str) -> Option<ThreadId> {
    ThreadId::from_string(thread_id).ok()
}

fn agent_label(thread_id: ThreadId, metadata: &AgentMetadata) -> AgentLabel<'_> {
    AgentLabel {
        thread_id: Some(thread_id),
        nickname: metadata.agent_nickname.as_deref(),
        role: metadata.agent_role.as_deref(),
    }
}

fn agent_label_line(agent: AgentLabel<'_>) -> Line<'static> {
    agent_label_spans(agent).into()
}

fn agent_label_spans(agent: AgentLabel<'_>) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let nickname = agent
        .nickname
        .map(str::trim)
        .filter(|nickname| !nickname.is_empty());
    let role = agent.role.map(str::trim).filter(|role| !role.is_empty());

    if let Some(nickname) = nickname {
        spans.push(Span::from(nickname.to_string()).cyan().bold());
    } else if let Some(thread_id) = agent.thread_id {
        spans.push(Span::from(thread_id.to_string()).cyan());
    } else {
        spans.push(Span::from("agent").cyan());
    }

    if let Some(role) = role {
        spans.push(Span::from(" ").dim());
        spans.push(Span::from(format!("[{role}]")));
    }

    spans
}

fn spawn_request_spans(spawn_request: Option<&SpawnRequestSummary>) -> Vec<Span<'static>> {
    let Some(spawn_request) = spawn_request else {
        return Vec::new();
    };

    let model = spawn_request.model.trim();
    if model.is_empty() && spawn_request.reasoning_effort == ReasoningEffortConfig::default() {
        return Vec::new();
    }

    let details = if model.is_empty() {
        format!("({})", spawn_request.reasoning_effort)
    } else {
        format!("({model} {})", spawn_request.reasoning_effort)
    };

    vec![Span::from(" ").dim(), Span::from(details).magenta()]
}

fn prompt_line(prompt: &str) -> Option<Line<'static>> {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(Line::from(Span::from(truncate_text(
            trimmed,
            COLLAB_PROMPT_PREVIEW_GRAPHEMES,
        ))))
    }
}

fn wait_complete_lines(
    receiver_thread_ids: &[String],
    agents_states: &std::collections::HashMap<String, CollabAgentState>,
    agent_metadata: &mut impl FnMut(ThreadId) -> AgentMetadata,
) -> Vec<Line<'static>> {
    let mut seen = HashSet::new();
    let mut entries = receiver_thread_ids
        .iter()
        .filter_map(|thread_id| {
            let parsed_thread_id = parse_thread_id(thread_id)?;
            let status = agents_states.get(thread_id)?;
            seen.insert(parsed_thread_id);
            Some((parsed_thread_id, agent_metadata(parsed_thread_id), status))
        })
        .collect::<Vec<_>>();

    let mut extras = agents_states
        .iter()
        .filter_map(|(thread_id, status)| {
            let parsed_thread_id = parse_thread_id(thread_id)?;
            (!seen.contains(&parsed_thread_id))
                .then(|| (parsed_thread_id, agent_metadata(parsed_thread_id), status))
        })
        .collect::<Vec<_>>();
    extras.sort_by_key(|entry| entry.0.to_string());
    entries.extend(extras);

    if entries.is_empty() {
        vec![Line::from(Span::from("No agents completed yet"))]
    } else {
        entries
            .into_iter()
            .map(|(thread_id, metadata, status)| {
                let mut spans = agent_label_spans(agent_label(thread_id, &metadata));
                spans.push(Span::from(": ").dim());
                spans.extend(status_summary_spans(status));
                spans.into()
            })
            .collect()
    }
}

fn first_agent_state<'a>(
    receiver_thread_ids: &[String],
    agents_states: &'a std::collections::HashMap<String, CollabAgentState>,
) -> Option<&'a CollabAgentState> {
    receiver_thread_ids
        .iter()
        .find_map(|thread_id| agents_states.get(thread_id))
        .or_else(|| {
            agents_states
                .iter()
                .min_by(|left, right| left.0.cmp(right.0))
                .map(|(_, status)| status)
        })
}

fn status_summary_line(status: Option<&CollabAgentState>, fallback_error: &str) -> Line<'static> {
    match status {
        Some(status) => status_summary_spans(status).into(),
        None => error_summary_spans(fallback_error).into(),
    }
}

fn status_summary_spans(status: &CollabAgentState) -> Vec<Span<'static>> {
    match status.status {
        CollabAgentStatus::PendingInit => vec![Span::from("Pending init").cyan()],
        CollabAgentStatus::Running => vec![Span::from("Running").cyan().bold()],
        // Allow `.yellow()`
        #[allow(clippy::disallowed_methods)]
        CollabAgentStatus::Interrupted => vec![Span::from("Interrupted").yellow()],
        CollabAgentStatus::Completed => {
            let mut spans = vec![Span::from("Completed").green()];
            if let Some(message) = status.message.as_ref() {
                let message_preview = truncate_text(
                    &message.split_whitespace().collect::<Vec<_>>().join(" "),
                    COLLAB_AGENT_RESPONSE_PREVIEW_GRAPHEMES,
                );
                if !message_preview.is_empty() {
                    spans.push(Span::from(" - ").dim());
                    spans.push(Span::from(message_preview));
                }
            }
            spans
        }
        CollabAgentStatus::Errored => {
            error_summary_spans(status.message.as_deref().unwrap_or("Agent errored"))
        }
        CollabAgentStatus::Shutdown => vec![Span::from("Shutdown")],
        CollabAgentStatus::NotFound => vec![Span::from("Not found").red()],
    }
}

fn error_summary_spans(error: &str) -> Vec<Span<'static>> {
    let mut spans = vec![Span::from("Error").red()];
    let error_preview = truncate_text(
        &error.split_whitespace().collect::<Vec<_>>().join(" "),
        COLLAB_AGENT_ERROR_PREVIEW_GRAPHEMES,
    );
    if !error_preview.is_empty() {
        spans.push(Span::from(" - ").dim());
        spans.push(Span::from(error_preview));
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history_cell::HistoryCell;
    #[cfg(target_os = "macos")]
    use crossterm::event::KeyEvent;
    #[cfg(target_os = "macos")]
    use crossterm::event::KeyModifiers;
    use insta::assert_snapshot;
    use pretty_assertions::assert_eq;
    use ratatui::style::Color;
    use ratatui::style::Modifier;
    use std::collections::HashMap;

    fn summary_entry(status: AgentPickerThreadStatus) -> AgentPickerThreadEntry {
        AgentPickerThreadEntry {
            agent_nickname: None,
            agent_role: None,
            agent_path: None,
            prompt_preview: None,
            thread_note: None,
            cwd: None,
            model_provider: None,
            created_at: None,
            updated_at: None,
            model: None,
            reasoning_effort: None,
            service_tier: None,
            status,
        }
    }

    #[test]
    fn agent_picker_item_description_summarizes_status_role_and_path() {
        let thread_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000102").expect("valid thread id");
        let entry = AgentPickerThreadEntry {
            agent_nickname: Some("Robie".to_string()),
            agent_role: Some("explorer".to_string()),
            agent_path: Some("/root/explorer".to_string()),
            prompt_preview: None,
            thread_note: None,
            cwd: None,
            model_provider: None,
            created_at: None,
            updated_at: None,
            model: None,
            reasoning_effort: None,
            service_tier: None,
            status: AgentPickerThreadStatus::Running,
        };

        assert_eq!(
            agent_picker_item_description(thread_id, &entry, /*is_primary*/ false),
            "running · role explorer · /root/explorer"
        );
    }

    #[test]
    fn agent_picker_summary_counts_sub_agent_statuses() {
        let primary =
            ThreadId::from_string("00000000-0000-0000-0000-000000000101").expect("valid thread id");
        let running =
            ThreadId::from_string("00000000-0000-0000-0000-000000000102").expect("valid thread id");
        let waiting =
            ThreadId::from_string("00000000-0000-0000-0000-000000000103").expect("valid thread id");
        let error =
            ThreadId::from_string("00000000-0000-0000-0000-000000000104").expect("valid thread id");
        let closed =
            ThreadId::from_string("00000000-0000-0000-0000-000000000105").expect("valid thread id");
        let primary_entry = summary_entry(AgentPickerThreadStatus::Idle);
        let running_entry = summary_entry(AgentPickerThreadStatus::Running);
        let waiting_entry = summary_entry(AgentPickerThreadStatus::WaitingUser);
        let error_entry = summary_entry(AgentPickerThreadStatus::Error);
        let closed_entry = summary_entry(AgentPickerThreadStatus::Closed);
        let threads = vec![
            (primary, &primary_entry),
            (running, &running_entry),
            (waiting, &waiting_entry),
            (error, &error_entry),
            (closed, &closed_entry),
        ];

        assert_eq!(
            agent_picker_summary_line(&threads, Some(primary)),
            "Agents: 4 total · 1 running · 1 waiting · 1 error · 1 closed"
        );
    }

    #[test]
    fn agent_picker_selected_description_includes_workbench_detail() {
        let thread_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000102").expect("valid thread id");
        let entry = AgentPickerThreadEntry {
            agent_nickname: Some("Robie".to_string()),
            agent_role: Some("explorer".to_string()),
            agent_path: Some("/root/explorer".to_string()),
            prompt_preview: Some(
                "Inspect the parser state\nand report concise evidence.".to_string(),
            ),
            thread_note: Some("Investigate parser state".to_string()),
            cwd: Some("/workspace/project".to_string()),
            model_provider: Some("deepseek".to_string()),
            created_at: Some(1_735_689_600),
            updated_at: Some(1_735_693_200),
            model: Some("deepseek-v4-flash".to_string()),
            reasoning_effort: Some("high".to_string()),
            service_tier: Some("priority".to_string()),
            status: AgentPickerThreadStatus::Closed,
        };

        assert_eq!(
            agent_picker_selected_description(
                thread_id,
                &entry,
                /*is_primary*/ false,
                /*is_current*/ true,
                AgentPickerSelectedDescriptionContext {
                    prompt_context: &[
                        "Initial request: Map parser entry points.".to_string(),
                        "Latest input: Focus on app-server projection.".to_string(),
                    ],
                    recent_activity: &[
                        "$ cargo test -p codex-tui agent_picker_workbench_snapshot".to_string(),
                        "Checked the bounded detail projection.".to_string(),
                    ],
                    token_usage_summary: Some("total 10 · last 4 · context 4/950,000"),
                    plan_progress_summary: Some("1/3 complete · now: Verify TUI workbench anchors",),
                },
            ),
            "Status: closed\nCurrent view: yes\nKind: sub-agent thread\nInspect:\n- Nickname: Robie\n- Role: explorer\n- Agent path: /root/explorer\n- Prompt: Inspect the parser state and report concise evidence.\n- Note: Investigate parser state\n- Cwd: /workspace/project\n- Model provider: deepseek\n- Created: 2025-01-01 00:00:00 UTC\n- Updated: 2025-01-01 01:00:00 UTC\n- Model: deepseek-v4-flash\n- Reasoning: high\n- Service tier: priority\n- Tokens: total 10 · last 4 · context 4/950,000\n- Plan: 1/3 complete · now: Verify TUI workbench anchors\n- Thread: 00000000-0000-0000-0000-000000000102\nContext:\n- Initial request: Map parser entry points.\n- Latest input: Focus on app-server projection.\nRecent activity:\n- $ cargo test -p codex-tui agent_picker_workbench_snapshot\n- Checked the bounded detail projection.\nActions:\n- Enter: watch this thread"
        );
    }

    #[test]
    fn agent_picker_status_labels_cover_waiting_and_error_states() {
        let thread_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000102").expect("valid thread id");
        let mut entry = AgentPickerThreadEntry {
            agent_nickname: Some("Robie".to_string()),
            agent_role: Some("explorer".to_string()),
            agent_path: Some("/root/explorer".to_string()),
            prompt_preview: None,
            thread_note: None,
            cwd: None,
            model_provider: None,
            created_at: None,
            updated_at: None,
            model: None,
            reasoning_effort: None,
            service_tier: None,
            status: AgentPickerThreadStatus::WaitingApproval,
        };

        assert_eq!(
            agent_picker_item_description(thread_id, &entry, /*is_primary*/ false),
            "waiting approval · role explorer · /root/explorer"
        );

        entry.status = AgentPickerThreadStatus::WaitingUser;
        assert_eq!(
            agent_picker_selected_description(
                thread_id,
                &entry,
                /*is_primary*/ false,
                /*is_current*/ false,
                AgentPickerSelectedDescriptionContext {
                    prompt_context: &[],
                    recent_activity: &[],
                    token_usage_summary: None,
                    plan_progress_summary: None,
                },
            ),
            "Status: waiting user\nCurrent view: no\nKind: sub-agent thread\nInspect:\n- Nickname: Robie\n- Role: explorer\n- Agent path: /root/explorer\n- Thread: 00000000-0000-0000-0000-000000000102\nActions:\n- Enter: watch this thread"
        );

        entry.status = AgentPickerThreadStatus::Error;
        assert_eq!(
            agent_picker_item_description(thread_id, &entry, /*is_primary*/ false),
            "error · role explorer · /root/explorer"
        );
    }

    #[test]
    fn agent_picker_prompt_context_matches_receiver_and_hides_raw_messages() {
        let receiver =
            ThreadId::from_string("00000000-0000-0000-0000-000000000102").expect("valid thread id");
        let other_receiver =
            ThreadId::from_string("00000000-0000-0000-0000-000000000103").expect("valid thread id");
        let spawn = ThreadItem::CollabAgentToolCall {
            id: "spawn-1".to_string(),
            tool: CollabAgentTool::SpawnAgent,
            status: CollabAgentToolCallStatus::Completed,
            sender_thread_id: ThreadId::new().to_string(),
            receiver_thread_ids: vec![receiver.to_string()],
            prompt: Some("Map parser entry points.\nReport evidence.".to_string()),
            model: None,
            reasoning_effort: None,
            agents_states: HashMap::new(),
        };
        let send = ThreadItem::CollabAgentToolCall {
            id: "send-1".to_string(),
            tool: CollabAgentTool::SendInput,
            status: CollabAgentToolCallStatus::Completed,
            sender_thread_id: ThreadId::new().to_string(),
            receiver_thread_ids: vec![receiver.to_string()],
            prompt: Some("Focus on app-server projection.".to_string()),
            model: None,
            reasoning_effort: None,
            agents_states: HashMap::new(),
        };
        let other = ThreadItem::CollabAgentToolCall {
            id: "spawn-other".to_string(),
            tool: CollabAgentTool::SpawnAgent,
            status: CollabAgentToolCallStatus::Completed,
            sender_thread_id: ThreadId::new().to_string(),
            receiver_thread_ids: vec![other_receiver.to_string()],
            prompt: Some("Do not show this other agent prompt.".to_string()),
            model: None,
            reasoning_effort: None,
            agents_states: HashMap::new(),
        };
        let user_message = ThreadItem::UserMessage {
            id: "user-1".to_string(),
            client_id: None,
            content: vec![codex_app_server_protocol::UserInput::Text {
                text: "raw user message should stay hidden".to_string(),
                text_elements: Vec::new(),
            }],
        };

        let context = agent_picker_prompt_context_summaries(
            receiver,
            [&send, &user_message, &other, &spawn].into_iter(),
        );

        assert_eq!(
            context,
            vec![
                "Initial request: Map parser entry points. Report evidence.".to_string(),
                "Latest input: Focus on app-server projection.".to_string(),
            ]
        );
        let rendered = context.join("\n");
        assert!(!rendered.contains("other agent prompt"));
        assert!(!rendered.contains("raw user message"));
    }

    #[test]
    fn agent_picker_recent_activity_is_bounded_and_privacy_safe() {
        let command = ThreadItem::CommandExecution {
            id: "command-1".to_string(),
            command: "cargo test -p codex-tui".to_string(),
            cwd: codex_utils_absolute_path::AbsolutePathBuf::try_from("/workspace")
                .expect("absolute path"),
            process_id: None,
            source: codex_app_server_protocol::CommandExecutionSource::Agent,
            status: codex_app_server_protocol::CommandExecutionStatus::Completed,
            command_actions: Vec::new(),
            aggregated_output: Some("secret output\n".repeat(100)),
            exit_code: Some(0),
            duration_ms: Some(12),
        };
        let reasoning = ThreadItem::Reasoning {
            id: "reasoning-1".to_string(),
            summary: vec!["safe reasoning summary".to_string()],
            content: vec!["hidden raw reasoning".to_string()],
        };
        let raw_user_message = ThreadItem::UserMessage {
            id: "user-1".to_string(),
            client_id: None,
            content: vec![codex_app_server_protocol::UserInput::Text {
                text: "do not show user prompt".to_string(),
                text_elements: Vec::new(),
            }],
        };

        let activity = agent_picker_recent_activity_summaries(
            [&reasoning, &raw_user_message, &command].into_iter(),
        );

        assert_eq!(
            activity,
            vec![
                "$ cargo test -p codex-tui".to_string(),
                "safe reasoning summary".to_string()
            ]
        );
        let rendered = activity.join("\n");
        assert!(!rendered.contains("secret output"));
        assert!(!rendered.contains("hidden raw reasoning"));
        assert!(!rendered.contains("do not show user prompt"));
    }

    #[test]
    fn agent_picker_plan_progress_summary_is_bounded_and_structured() {
        let plan = vec![
            TurnPlanStep {
                step: "Inspect native notification source".to_string(),
                status: TurnPlanStepStatus::Completed,
            },
            TurnPlanStep {
                step: format!(
                    "{} {}",
                    "Verify workbench projection",
                    "carefully ".repeat(80)
                ),
                status: TurnPlanStepStatus::InProgress,
            },
            TurnPlanStep {
                step: "Update docs".to_string(),
                status: TurnPlanStepStatus::Pending,
            },
        ];

        let summary = agent_picker_plan_progress_summary(&plan).expect("summary");

        assert!(summary.starts_with("1/3 complete · now: Verify workbench projection"));
        assert!(summary.len() < 280);
        assert!(!summary.contains("Update docs"));
        assert_eq!(agent_picker_plan_progress_summary(&[]), None);
        assert_eq!(
            agent_picker_plan_progress_summary(&[TurnPlanStep {
                step: "Done".to_string(),
                status: TurnPlanStepStatus::Completed,
            }]),
            Some("all 1 complete".to_string())
        );
    }

    #[test]
    fn agent_picker_token_usage_summary_is_bounded_and_optional() {
        let token_usage = ThreadTokenUsage {
            total: codex_app_server_protocol::TokenUsageBreakdown {
                total_tokens: 12_345,
                input_tokens: 4,
                cached_input_tokens: 1,
                output_tokens: 5,
                reasoning_output_tokens: 0,
            },
            last: codex_app_server_protocol::TokenUsageBreakdown {
                total_tokens: 678,
                input_tokens: 4,
                cached_input_tokens: 1,
                output_tokens: 5,
                reasoning_output_tokens: 0,
            },
            model_context_window: Some(950_000),
        };

        assert_eq!(
            agent_picker_token_usage_summary(&token_usage),
            Some("total 12,345 · last 678 · context 678/950,000".to_string())
        );

        let empty_token_usage = ThreadTokenUsage {
            total: codex_app_server_protocol::TokenUsageBreakdown {
                total_tokens: 0,
                input_tokens: 0,
                cached_input_tokens: 0,
                output_tokens: 0,
                reasoning_output_tokens: 0,
            },
            last: codex_app_server_protocol::TokenUsageBreakdown {
                total_tokens: 0,
                input_tokens: 0,
                cached_input_tokens: 0,
                output_tokens: 0,
                reasoning_output_tokens: 0,
            },
            model_context_window: None,
        };

        assert_eq!(agent_picker_token_usage_summary(&empty_token_usage), None);
    }

    #[test]
    fn collab_events_snapshot() {
        let sender_thread_id = ThreadId::from_string("00000000-0000-0000-0000-000000000001")
            .expect("valid sender thread id");
        let robie_id = ThreadId::from_string("00000000-0000-0000-0000-000000000002")
            .expect("valid robie thread id");
        let bob_id = ThreadId::from_string("00000000-0000-0000-0000-000000000003")
            .expect("valid bob thread id");

        let spawn = tool_call_history_cell(
            &ThreadItem::CollabAgentToolCall {
                id: "call-spawn".to_string(),
                tool: CollabAgentTool::SpawnAgent,
                status: CollabAgentToolCallStatus::Completed,
                sender_thread_id: sender_thread_id.to_string(),
                receiver_thread_ids: vec![robie_id.to_string()],
                prompt: Some("Compute 11! and reply with just the integer result.".to_string()),
                model: Some("gpt-5".to_string()),
                reasoning_effort: Some(ReasoningEffortConfig::High),
                agents_states: HashMap::from([(
                    robie_id.to_string(),
                    agent_state(CollabAgentStatus::PendingInit, /*message*/ None),
                )]),
            },
            /*cached_spawn_request*/ None,
            |thread_id| metadata_for(thread_id, robie_id, bob_id),
        )
        .expect("spawn item renders");

        let send = tool_call_history_cell(
            &ThreadItem::CollabAgentToolCall {
                id: "call-send".to_string(),
                tool: CollabAgentTool::SendInput,
                status: CollabAgentToolCallStatus::Completed,
                sender_thread_id: sender_thread_id.to_string(),
                receiver_thread_ids: vec![robie_id.to_string()],
                prompt: Some("Please continue and return the answer only.".to_string()),
                model: None,
                reasoning_effort: None,
                agents_states: HashMap::from([(
                    robie_id.to_string(),
                    agent_state(CollabAgentStatus::Running, /*message*/ None),
                )]),
            },
            /*cached_spawn_request*/ None,
            |thread_id| metadata_for(thread_id, robie_id, bob_id),
        )
        .expect("send-input item renders");

        let waiting = tool_call_history_cell(
            &ThreadItem::CollabAgentToolCall {
                id: "call-wait".to_string(),
                tool: CollabAgentTool::Wait,
                status: CollabAgentToolCallStatus::InProgress,
                sender_thread_id: sender_thread_id.to_string(),
                receiver_thread_ids: vec![robie_id.to_string()],
                prompt: None,
                model: None,
                reasoning_effort: None,
                agents_states: HashMap::new(),
            },
            /*cached_spawn_request*/ None,
            |thread_id| metadata_for(thread_id, robie_id, bob_id),
        )
        .expect("wait begin item renders");

        let finished = tool_call_history_cell(
            &ThreadItem::CollabAgentToolCall {
                id: "call-wait".to_string(),
                tool: CollabAgentTool::Wait,
                status: CollabAgentToolCallStatus::Completed,
                sender_thread_id: sender_thread_id.to_string(),
                receiver_thread_ids: vec![robie_id.to_string(), bob_id.to_string()],
                prompt: None,
                model: None,
                reasoning_effort: None,
                agents_states: HashMap::from([
                    (
                        robie_id.to_string(),
                        agent_state(CollabAgentStatus::Completed, Some("39916800")),
                    ),
                    (
                        bob_id.to_string(),
                        agent_state(CollabAgentStatus::Errored, Some("tool timeout")),
                    ),
                ]),
            },
            /*cached_spawn_request*/ None,
            |thread_id| metadata_for(thread_id, robie_id, bob_id),
        )
        .expect("wait end item renders");

        let close = tool_call_history_cell(
            &ThreadItem::CollabAgentToolCall {
                id: "call-close".to_string(),
                tool: CollabAgentTool::CloseAgent,
                status: CollabAgentToolCallStatus::Completed,
                sender_thread_id: sender_thread_id.to_string(),
                receiver_thread_ids: vec![robie_id.to_string()],
                prompt: None,
                model: None,
                reasoning_effort: None,
                agents_states: HashMap::from([(
                    robie_id.to_string(),
                    agent_state(CollabAgentStatus::Completed, Some("39916800")),
                )]),
            },
            /*cached_spawn_request*/ None,
            |thread_id| metadata_for(thread_id, robie_id, bob_id),
        )
        .expect("close item renders");

        let snapshot = [spawn, send, waiting, finished, close]
            .iter()
            .map(cell_to_text)
            .collect::<Vec<_>>()
            .join("\n\n");
        assert_snapshot!("collab_agent_transcript", snapshot);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn agent_shortcut_matches_option_arrow_word_motion_fallbacks_only_when_allowed() {
        assert!(previous_agent_shortcut_matches(
            KeyEvent::new(KeyCode::Left, KeyModifiers::ALT),
            /*allow_word_motion_fallback*/ false,
        ));
        assert!(next_agent_shortcut_matches(
            KeyEvent::new(KeyCode::Right, KeyModifiers::ALT),
            /*allow_word_motion_fallback*/ false,
        ));
        assert!(previous_agent_shortcut_matches(
            KeyEvent::new(KeyCode::Char('b'), KeyModifiers::ALT),
            /*allow_word_motion_fallback*/ true,
        ));
        assert!(next_agent_shortcut_matches(
            KeyEvent::new(KeyCode::Char('f'), KeyModifiers::ALT),
            /*allow_word_motion_fallback*/ true,
        ));
        assert!(!previous_agent_shortcut_matches(
            KeyEvent::new(KeyCode::Char('b'), KeyModifiers::ALT),
            /*allow_word_motion_fallback*/ false,
        ));
        assert!(!next_agent_shortcut_matches(
            KeyEvent::new(KeyCode::Char('f'), KeyModifiers::ALT),
            /*allow_word_motion_fallback*/ false,
        ));
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn agent_shortcut_matches_option_arrows_only() {
        assert!(previous_agent_shortcut_matches(
            KeyEvent::new(KeyCode::Left, crossterm::event::KeyModifiers::ALT,),
            /*allow_word_motion_fallback*/ false
        ));
        assert!(next_agent_shortcut_matches(
            KeyEvent::new(KeyCode::Right, crossterm::event::KeyModifiers::ALT,),
            /*allow_word_motion_fallback*/ false
        ));
        assert!(!previous_agent_shortcut_matches(
            KeyEvent::new(KeyCode::Char('b'), crossterm::event::KeyModifiers::ALT,),
            /*allow_word_motion_fallback*/ false
        ));
        assert!(!next_agent_shortcut_matches(
            KeyEvent::new(KeyCode::Char('f'), crossterm::event::KeyModifiers::ALT,),
            /*allow_word_motion_fallback*/ false
        ));
    }

    #[test]
    fn title_styles_nickname_and_role() {
        let sender_thread_id = ThreadId::from_string("00000000-0000-0000-0000-000000000001")
            .expect("valid sender thread id");
        let robie_id = ThreadId::from_string("00000000-0000-0000-0000-000000000002")
            .expect("valid robie thread id");
        let cell = tool_call_history_cell(
            &ThreadItem::CollabAgentToolCall {
                id: "call-spawn".to_string(),
                tool: CollabAgentTool::SpawnAgent,
                status: CollabAgentToolCallStatus::Completed,
                sender_thread_id: sender_thread_id.to_string(),
                receiver_thread_ids: vec![robie_id.to_string()],
                prompt: Some(String::new()),
                model: Some("gpt-5".to_string()),
                reasoning_effort: Some(ReasoningEffortConfig::High),
                agents_states: HashMap::from([(
                    robie_id.to_string(),
                    agent_state(CollabAgentStatus::PendingInit, /*message*/ None),
                )]),
            },
            /*cached_spawn_request*/ None,
            |thread_id| metadata_for(thread_id, robie_id, ThreadId::new()),
        )
        .expect("spawn item renders");

        let lines = cell.display_lines(/*width*/ 200);
        let title = &lines[0];
        assert_eq!(title.spans[2].content.as_ref(), "Robie");
        assert_eq!(title.spans[2].style.fg, Some(Color::Cyan));
        assert!(title.spans[2].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(title.spans[4].content.as_ref(), "[explorer]");
        assert_eq!(title.spans[4].style.fg, None);
        assert!(!title.spans[4].style.add_modifier.contains(Modifier::DIM));
        assert_eq!(title.spans[6].content.as_ref(), "(gpt-5 high)");
        assert_eq!(title.spans[6].style.fg, Some(Color::Magenta));
    }

    #[test]
    fn collab_resume_interrupted_snapshot() {
        let sender_thread_id = ThreadId::from_string("00000000-0000-0000-0000-000000000001")
            .expect("valid sender thread id");
        let robie_id = ThreadId::from_string("00000000-0000-0000-0000-000000000002")
            .expect("valid robie thread id");

        let cell = tool_call_history_cell(
            &ThreadItem::CollabAgentToolCall {
                id: "call-resume".to_string(),
                tool: CollabAgentTool::ResumeAgent,
                status: CollabAgentToolCallStatus::Completed,
                sender_thread_id: sender_thread_id.to_string(),
                receiver_thread_ids: vec![robie_id.to_string()],
                prompt: None,
                model: None,
                reasoning_effort: None,
                agents_states: HashMap::from([(
                    robie_id.to_string(),
                    agent_state(CollabAgentStatus::Interrupted, /*message*/ None),
                )]),
            },
            /*cached_spawn_request*/ None,
            |thread_id| metadata_for(thread_id, robie_id, ThreadId::new()),
        )
        .expect("resume item renders");

        assert_snapshot!("collab_resume_interrupted", cell_to_text(&cell));
    }

    fn agent_state(status: CollabAgentStatus, message: Option<&str>) -> CollabAgentState {
        CollabAgentState {
            status,
            message: message.map(str::to_string),
            thread_note: None,
        }
    }

    fn metadata_for(thread_id: ThreadId, robie_id: ThreadId, bob_id: ThreadId) -> AgentMetadata {
        if thread_id == robie_id {
            AgentMetadata {
                agent_nickname: Some("Robie".to_string()),
                agent_role: Some("explorer".to_string()),
            }
        } else if thread_id == bob_id {
            AgentMetadata {
                agent_nickname: Some("Bob".to_string()),
                agent_role: Some("worker".to_string()),
            }
        } else {
            AgentMetadata::default()
        }
    }

    fn cell_to_text(cell: &PlainHistoryCell) -> String {
        cell.display_lines(/*width*/ 200)
            .iter()
            .map(line_to_text)
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn line_to_text(line: &Line<'static>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<Vec<_>>()
            .join("")
    }
}
