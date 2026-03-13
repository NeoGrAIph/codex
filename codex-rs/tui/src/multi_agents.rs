use crate::line_truncation::truncate_line_with_ellipsis_if_overflow;
use crate::render::line_utils::prefix_lines;
use crate::text_formatting::truncate_text;
use codex_protocol::ThreadId;
use codex_protocol::protocol::AgentStatus;
use codex_protocol::protocol::CollabAgentInteractionEndEvent;
use codex_protocol::protocol::CollabAgentRef;
use codex_protocol::protocol::CollabAgentSpawnEndEvent;
use codex_protocol::protocol::CollabAgentStatusEntry;
use codex_protocol::protocol::CollabCloseEndEvent;
use codex_protocol::protocol::CollabResumeBeginEvent;
use codex_protocol::protocol::CollabResumeEndEvent;
use codex_protocol::protocol::CollabWaitingBeginEvent;
use codex_protocol::protocol::CollabWaitingEndEvent;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;

const COLLAB_PROMPT_PREVIEW_GRAPHEMES: usize = 160;
const COLLAB_AGENT_ERROR_PREVIEW_GRAPHEMES: usize = 160;
const COLLAB_AGENT_RESPONSE_PREVIEW_GRAPHEMES: usize = 240;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentPickerThreadEntry {
    pub(crate) agent_nickname: Option<String>,
    pub(crate) agent_role: Option<String>,
    pub(crate) is_closed: bool,
}

#[derive(Clone, Copy)]
struct AgentLabel<'a> {
    thread_id: Option<ThreadId>,
    nickname: Option<&'a str>,
    role: Option<&'a str>,
}

pub(crate) fn agent_picker_status_dot_spans(is_closed: bool) -> Vec<Span<'static>> {
    let dot = if is_closed {
        "•".into()
    } else {
        "•".green()
    };
    vec![dot, " ".into()]
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

pub(crate) fn sort_agent_picker_threads(agent_threads: &mut [(ThreadId, AgentPickerThreadEntry)]) {
    agent_threads.sort_by(|(left_id, left), (right_id, right)| {
        left.is_closed
            .cmp(&right.is_closed)
            .then_with(|| left_id.to_string().cmp(&right_id.to_string()))
    });
}

pub(crate) fn spawn_end(ev: CollabAgentSpawnEndEvent) -> CollabHistoryCell {
    let CollabAgentSpawnEndEvent {
        call_id: _,
        sender_thread_id: _,
        new_thread_id,
        new_agent_nickname,
        new_agent_role,
        new_thread_note,
        prompt,
        status: _,
    } = ev;

    let title = match new_thread_id {
        Some(thread_id) => title_with_agent(
            "Spawned",
            AgentLabel {
                thread_id: Some(thread_id),
                nickname: new_agent_nickname.as_deref(),
                role: new_agent_role.as_deref(),
            },
        ),
        None => title_text("Agent spawn failed"),
    };

    let mut details = Vec::new();
    if let Some(line) = note_line(new_thread_note.as_deref()) {
        details.push(line);
    }
    if let Some(line) = prompt_line(&prompt) {
        details.push(CollabDetailLine::Plain(line));
    }
    collab_event(title, details)
}

pub(crate) fn interaction_end(ev: CollabAgentInteractionEndEvent) -> CollabHistoryCell {
    let CollabAgentInteractionEndEvent {
        call_id: _,
        sender_thread_id: _,
        receiver_thread_id,
        receiver_agent_nickname,
        receiver_agent_role,
        receiver_thread_note,
        prompt,
        status: _,
    } = ev;

    let title = title_with_agent(
        "Sent input to",
        AgentLabel {
            thread_id: Some(receiver_thread_id),
            nickname: receiver_agent_nickname.as_deref(),
            role: receiver_agent_role.as_deref(),
        },
    );

    let mut details = Vec::new();
    if let Some(line) = note_line(receiver_thread_note.as_deref()) {
        details.push(line);
    }
    if let Some(line) = prompt_line(&prompt) {
        details.push(CollabDetailLine::Plain(line));
    }
    collab_event(title, details)
}

pub(crate) fn waiting_begin(ev: CollabWaitingBeginEvent) -> CollabHistoryCell {
    let CollabWaitingBeginEvent {
        sender_thread_id: _,
        receiver_thread_ids,
        receiver_agents,
        call_id: _,
    } = ev;
    let receiver_agents = merge_wait_receivers(&receiver_thread_ids, receiver_agents);

    let title = match receiver_agents.as_slice() {
        [receiver] => title_with_agent("Waiting for", agent_label_from_ref(receiver)),
        [] => title_text("Waiting for agents"),
        _ => title_text(format!("Waiting for {} agents", receiver_agents.len())),
    };

    let details = if receiver_agents.len() > 1 {
        receiver_agents
            .iter()
            .map(|receiver| agent_label_line(agent_label_from_ref(receiver)))
            .collect()
    } else {
        Vec::new()
    };

    collab_event(
        title,
        details
            .into_iter()
            .map(CollabDetailLine::Plain)
            .collect::<Vec<_>>(),
    )
}

pub(crate) fn waiting_end(ev: CollabWaitingEndEvent) -> CollabHistoryCell {
    let CollabWaitingEndEvent {
        call_id: _,
        sender_thread_id: _,
        agent_statuses,
        statuses,
    } = ev;
    let details = wait_complete_lines(&statuses, &agent_statuses)
        .into_iter()
        .map(CollabDetailLine::Plain)
        .collect::<Vec<_>>();
    collab_event(title_text("Finished waiting"), details)
}

pub(crate) fn close_end(ev: CollabCloseEndEvent) -> CollabHistoryCell {
    let CollabCloseEndEvent {
        call_id: _,
        sender_thread_id: _,
        receiver_thread_id,
        receiver_agent_nickname,
        receiver_agent_role,
        status: _,
    } = ev;

    collab_event(
        title_with_agent(
            "Closed",
            AgentLabel {
                thread_id: Some(receiver_thread_id),
                nickname: receiver_agent_nickname.as_deref(),
                role: receiver_agent_role.as_deref(),
            },
        ),
        Vec::new(),
    )
}

pub(crate) fn resume_begin(ev: CollabResumeBeginEvent) -> CollabHistoryCell {
    let CollabResumeBeginEvent {
        call_id: _,
        sender_thread_id: _,
        receiver_thread_id,
        receiver_agent_nickname,
        receiver_agent_role,
    } = ev;

    collab_event(
        title_with_agent(
            "Resuming",
            AgentLabel {
                thread_id: Some(receiver_thread_id),
                nickname: receiver_agent_nickname.as_deref(),
                role: receiver_agent_role.as_deref(),
            },
        ),
        Vec::new(),
    )
}

pub(crate) fn resume_end(ev: CollabResumeEndEvent) -> CollabHistoryCell {
    let CollabResumeEndEvent {
        call_id: _,
        sender_thread_id: _,
        receiver_thread_id,
        receiver_agent_nickname,
        receiver_agent_role,
        status,
    } = ev;

    collab_event(
        title_with_agent(
            "Resumed",
            AgentLabel {
                thread_id: Some(receiver_thread_id),
                nickname: receiver_agent_nickname.as_deref(),
                role: receiver_agent_role.as_deref(),
            },
        ),
        vec![CollabDetailLine::Plain(status_summary_line(&status))],
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CollabDetailLine {
    Plain(Line<'static>),
    Note(Line<'static>),
}

#[derive(Debug)]
pub(crate) struct CollabHistoryCell {
    lines: Vec<Line<'static>>,
    note_line_indexes: BTreeSet<usize>,
}

impl CollabHistoryCell {
    fn new(lines: Vec<Line<'static>>, note_line_indexes: BTreeSet<usize>) -> Self {
        Self {
            lines,
            note_line_indexes,
        }
    }
}

impl crate::history_cell::HistoryCell for CollabHistoryCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        let max_width = usize::from(width);
        self.lines
            .iter()
            .enumerate()
            .map(|(index, line)| {
                if self.note_line_indexes.contains(&index) {
                    truncate_line_with_ellipsis_if_overflow(line.clone(), max_width)
                } else {
                    line.clone()
                }
            })
            .collect()
    }
}

fn collab_event(title: Line<'static>, details: Vec<CollabDetailLine>) -> CollabHistoryCell {
    let mut lines: Vec<Line<'static>> = vec![title];
    if !details.is_empty() {
        let note_detail_indexes = details
            .iter()
            .enumerate()
            .filter_map(|(index, detail)| match detail {
                CollabDetailLine::Note(_) => Some(index),
                CollabDetailLine::Plain(_) => None,
            })
            .collect::<BTreeSet<_>>();
        let detail_lines = details
            .into_iter()
            .map(|detail| match detail {
                CollabDetailLine::Plain(line) | CollabDetailLine::Note(line) => line,
            })
            .collect::<Vec<_>>();
        lines.extend(prefix_lines(detail_lines, "  └ ".dim(), "    ".into()));
        let base_index = 1usize;
        let note_line_indexes = note_detail_indexes
            .into_iter()
            .map(|index| base_index + index)
            .collect::<BTreeSet<_>>();
        return CollabHistoryCell::new(lines, note_line_indexes);
    }
    CollabHistoryCell::new(lines, BTreeSet::new())
}

fn title_text(title: impl Into<String>) -> Line<'static> {
    title_spans_line(vec![Span::from(title.into()).bold()])
}

fn title_with_agent(prefix: &str, agent: AgentLabel<'_>) -> Line<'static> {
    let mut spans = vec![Span::from(format!("{prefix} ")).bold()];
    spans.extend(agent_label_spans(agent));
    title_spans_line(spans)
}

fn title_spans_line(mut spans: Vec<Span<'static>>) -> Line<'static> {
    let mut title = Vec::with_capacity(spans.len() + 1);
    title.push(Span::from("• ").dim());
    title.append(&mut spans);
    title.into()
}

fn agent_label_from_ref(agent: &CollabAgentRef) -> AgentLabel<'_> {
    AgentLabel {
        thread_id: Some(agent.thread_id),
        nickname: agent.agent_nickname.as_deref(),
        role: agent.agent_role.as_deref(),
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

fn note_line(note: Option<&str>) -> Option<CollabDetailLine> {
    let trimmed = note.map(str::trim).filter(|note| !note.is_empty())?;
    Some(CollabDetailLine::Note(
        vec!["Note: ".dim(), Span::from(trimmed.to_string())].into(),
    ))
}

fn merge_wait_receivers(
    receiver_thread_ids: &[ThreadId],
    mut receiver_agents: Vec<CollabAgentRef>,
) -> Vec<CollabAgentRef> {
    if receiver_agents.is_empty() {
        return receiver_thread_ids
            .iter()
            .map(|thread_id| CollabAgentRef {
                thread_id: *thread_id,
                agent_nickname: None,
                agent_role: None,
            })
            .collect();
    }

    let mut seen = receiver_agents
        .iter()
        .map(|agent| agent.thread_id)
        .collect::<HashSet<_>>();
    for thread_id in receiver_thread_ids {
        if seen.insert(*thread_id) {
            receiver_agents.push(CollabAgentRef {
                thread_id: *thread_id,
                agent_nickname: None,
                agent_role: None,
            });
        }
    }
    receiver_agents
}

fn wait_complete_lines(
    statuses: &HashMap<ThreadId, AgentStatus>,
    agent_statuses: &[CollabAgentStatusEntry],
) -> Vec<Line<'static>> {
    if statuses.is_empty() && agent_statuses.is_empty() {
        return vec![Line::from(Span::from("No agents completed yet"))];
    }

    let entries = if agent_statuses.is_empty() {
        let mut entries = statuses
            .iter()
            .map(|(thread_id, status)| CollabAgentStatusEntry {
                thread_id: *thread_id,
                agent_nickname: None,
                agent_role: None,
                status: status.clone(),
            })
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| left.thread_id.to_string().cmp(&right.thread_id.to_string()));
        entries
    } else {
        let mut entries = agent_statuses.to_vec();
        let seen = entries
            .iter()
            .map(|entry| entry.thread_id)
            .collect::<HashSet<_>>();
        let mut extras = statuses
            .iter()
            .filter(|(thread_id, _)| !seen.contains(thread_id))
            .map(|(thread_id, status)| CollabAgentStatusEntry {
                thread_id: *thread_id,
                agent_nickname: None,
                agent_role: None,
                status: status.clone(),
            })
            .collect::<Vec<_>>();
        extras.sort_by(|left, right| left.thread_id.to_string().cmp(&right.thread_id.to_string()));
        entries.extend(extras);
        entries
    };

    entries
        .into_iter()
        .map(|entry| {
            let CollabAgentStatusEntry {
                thread_id,
                agent_nickname,
                agent_role,
                status,
            } = entry;
            let mut spans = agent_label_spans(AgentLabel {
                thread_id: Some(thread_id),
                nickname: agent_nickname.as_deref(),
                role: agent_role.as_deref(),
            });
            spans.push(Span::from(": ").dim());
            spans.extend(status_summary_spans(&status));
            spans.into()
        })
        .collect()
}

fn status_summary_line(status: &AgentStatus) -> Line<'static> {
    status_summary_spans(status).into()
}

fn status_summary_spans(status: &AgentStatus) -> Vec<Span<'static>> {
    match status {
        AgentStatus::PendingInit => vec![Span::from("Pending init").cyan()],
        AgentStatus::Running => vec![Span::from("Running").cyan().bold()],
        AgentStatus::Completed(message) => {
            let mut spans = vec![Span::from("Completed").green()];
            if let Some(message) = message.as_ref() {
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
        AgentStatus::Errored(error) => {
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
        AgentStatus::Shutdown => vec![Span::from("Shutdown")],
        AgentStatus::NotFound => vec![Span::from("Not found").red()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history_cell::HistoryCell;
    use insta::assert_snapshot;
    use pretty_assertions::assert_eq;
    use ratatui::style::Color;
    use ratatui::style::Modifier;

    #[test]
    fn collab_events_snapshot() {
        let sender_thread_id = ThreadId::from_string("00000000-0000-0000-0000-000000000001")
            .expect("valid sender thread id");
        let robie_id = ThreadId::from_string("00000000-0000-0000-0000-000000000002")
            .expect("valid robie thread id");
        let bob_id = ThreadId::from_string("00000000-0000-0000-0000-000000000003")
            .expect("valid bob thread id");

        let spawn = spawn_end(CollabAgentSpawnEndEvent {
            call_id: "call-spawn".to_string(),
            sender_thread_id,
            new_thread_id: Some(robie_id),
            new_agent_nickname: Some("Robie".to_string()),
            new_agent_role: Some("explorer".to_string()),
            new_thread_note: Some("Focus on edge cases".to_string()),
            prompt: "Compute 11! and reply with just the integer result.".to_string(),
            status: AgentStatus::PendingInit,
        });

        let send = interaction_end(CollabAgentInteractionEndEvent {
            call_id: "call-send".to_string(),
            sender_thread_id,
            receiver_thread_id: robie_id,
            receiver_agent_nickname: Some("Robie".to_string()),
            receiver_agent_role: Some("explorer".to_string()),
            receiver_thread_note: Some("Focus on edge cases".to_string()),
            prompt: "Please continue and return the answer only.".to_string(),
            status: AgentStatus::Running,
        });

        let waiting = waiting_begin(CollabWaitingBeginEvent {
            sender_thread_id,
            receiver_thread_ids: vec![robie_id],
            receiver_agents: vec![CollabAgentRef {
                thread_id: robie_id,
                agent_nickname: Some("Robie".to_string()),
                agent_role: Some("explorer".to_string()),
            }],
            call_id: "call-wait".to_string(),
        });

        let mut statuses = HashMap::new();
        statuses.insert(
            robie_id,
            AgentStatus::Completed(Some("39916800".to_string())),
        );
        statuses.insert(bob_id, AgentStatus::Errored("tool timeout".to_string()));
        let finished = waiting_end(CollabWaitingEndEvent {
            sender_thread_id,
            call_id: "call-wait".to_string(),
            agent_statuses: vec![
                CollabAgentStatusEntry {
                    thread_id: robie_id,
                    agent_nickname: Some("Robie".to_string()),
                    agent_role: Some("explorer".to_string()),
                    status: AgentStatus::Completed(Some("39916800".to_string())),
                },
                CollabAgentStatusEntry {
                    thread_id: bob_id,
                    agent_nickname: Some("Bob".to_string()),
                    agent_role: Some("worker".to_string()),
                    status: AgentStatus::Errored("tool timeout".to_string()),
                },
            ],
            statuses,
        });

        let close = close_end(CollabCloseEndEvent {
            call_id: "call-close".to_string(),
            sender_thread_id,
            receiver_thread_id: robie_id,
            receiver_agent_nickname: Some("Robie".to_string()),
            receiver_agent_role: Some("explorer".to_string()),
            status: AgentStatus::Completed(Some("39916800".to_string())),
        });

        let snapshot = [spawn, send, waiting, finished, close]
            .iter()
            .map(cell_to_text)
            .collect::<Vec<_>>()
            .join("\n\n");
        assert_snapshot!("collab_agent_transcript", snapshot);
    }

    #[test]
    fn title_styles_nickname_and_role() {
        let sender_thread_id = ThreadId::from_string("00000000-0000-0000-0000-000000000001")
            .expect("valid sender thread id");
        let robie_id = ThreadId::from_string("00000000-0000-0000-0000-000000000002")
            .expect("valid robie thread id");
        let cell = spawn_end(CollabAgentSpawnEndEvent {
            call_id: "call-spawn".to_string(),
            sender_thread_id,
            new_thread_id: Some(robie_id),
            new_agent_nickname: Some("Robie".to_string()),
            new_agent_role: Some("explorer".to_string()),
            new_thread_note: None,
            prompt: String::new(),
            status: AgentStatus::PendingInit,
        });

        let lines = cell.display_lines(200);
        let title = &lines[0];
        assert_eq!(title.spans[2].content.as_ref(), "Robie");
        assert_eq!(title.spans[2].style.fg, Some(Color::Cyan));
        assert!(title.spans[2].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(title.spans[4].content.as_ref(), "[explorer]");
        assert_eq!(title.spans[4].style.fg, None);
        assert!(!title.spans[4].style.add_modifier.contains(Modifier::DIM));
    }

    #[test]
    fn note_line_is_absent_for_empty_note() {
        assert_eq!(note_line(None), None);
        assert_eq!(note_line(Some("   ")), None);
    }

    #[test]
    fn note_line_uses_full_text_when_width_allows_it() {
        let cell = spawn_end(CollabAgentSpawnEndEvent {
            call_id: "call-spawn".to_string(),
            sender_thread_id: ThreadId::from_string("00000000-0000-0000-0000-000000000001")
                .expect("valid sender thread id"),
            new_thread_id: Some(
                ThreadId::from_string("00000000-0000-0000-0000-000000000002")
                    .expect("valid receiver thread id"),
            ),
            new_agent_nickname: Some("Robie".to_string()),
            new_agent_role: Some("explorer".to_string()),
            new_thread_note: Some("Focus on edge cases and verify resume semantics".to_string()),
            prompt: String::new(),
            status: AgentStatus::PendingInit,
        });

        let lines = cell.display_lines(120);
        assert_eq!(
            line_to_text(&lines[1]),
            "  └ Note: Focus on edge cases and verify resume semantics"
        );
    }

    #[test]
    fn note_line_truncates_only_when_width_is_too_small() {
        let cell = spawn_end(CollabAgentSpawnEndEvent {
            call_id: "call-spawn".to_string(),
            sender_thread_id: ThreadId::from_string("00000000-0000-0000-0000-000000000001")
                .expect("valid sender thread id"),
            new_thread_id: Some(
                ThreadId::from_string("00000000-0000-0000-0000-000000000002")
                    .expect("valid receiver thread id"),
            ),
            new_agent_nickname: Some("Robie".to_string()),
            new_agent_role: Some("explorer".to_string()),
            new_thread_note: Some("Focus on edge cases and verify resume semantics".to_string()),
            prompt: String::new(),
            status: AgentStatus::PendingInit,
        });

        let lines = cell.display_lines(28);
        assert_eq!(line_to_text(&lines[1]), "  └ Note: Focus on edge cas…");
    }

    fn cell_to_text(cell: &impl HistoryCell) -> String {
        cell.display_lines(200)
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
