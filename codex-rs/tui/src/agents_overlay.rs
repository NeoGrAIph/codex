use codex_core::protocol::AgentStatus;
use codex_protocol::ThreadId;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use std::collections::HashMap;
use std::time::Instant;

use crate::exec_cell::spinner;
use crate::shimmer::shimmer_spans;
use crate::status_indicator_widget::fmt_elapsed_compact;

#[derive(Debug, Clone)]
pub(crate) struct AgentSummaryEntry {
    pub(crate) thread_id: ThreadId,
    pub(crate) parent_thread_id: Option<ThreadId>,
    pub(crate) role: String,
    pub(crate) model: String,
    pub(crate) reasoning: String,
    pub(crate) status: AgentStatus,
    pub(crate) created_at: Instant,
    pub(crate) status_changed_at: Instant,
    pub(crate) context_left_percent: Option<i64>,
    // SAW COMMIT OPEN: last tool for SAW summary.
    // Role: surface the last tool-related event seen on this thread as a single summary line.
    pub(crate) last_tool: Option<String>,
    pub(crate) last_tool_detail: Option<String>,
    // SAW COMMIT CLOSE: last tool for SAW summary.
}

pub(crate) fn build_agents_overlay_lines(
    agents: &[AgentSummaryEntry],
    now: Instant,
    animations_enabled: bool,
) -> Vec<Line<'static>> {
    if agents.is_empty() {
        return vec!["No active sub-agent threads.".italic().into()];
    }

    let mut index_by_thread_id: HashMap<ThreadId, usize> = HashMap::new();
    for (index, entry) in agents.iter().enumerate() {
        index_by_thread_id.insert(entry.thread_id, index);
    }

    let mut children: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut roots: Vec<usize> = Vec::new();
    for (index, entry) in agents.iter().enumerate() {
        if let Some(parent_index) = entry
            .parent_thread_id
            .and_then(|parent| index_by_thread_id.get(&parent).copied())
        {
            children.entry(parent_index).or_default().push(index);
        } else {
            roots.push(index);
        }
    }

    roots.sort_by(|a, b| {
        agents[*a]
            .thread_id
            .to_string()
            .cmp(&agents[*b].thread_id.to_string())
    });
    for child_indexes in children.values_mut() {
        child_indexes.sort_by(|a, b| {
            agents[*a]
                .thread_id
                .to_string()
                .cmp(&agents[*b].thread_id.to_string())
        });
    }

    fn push_tree(
        index: usize,
        depth: usize,
        children: &HashMap<usize, Vec<usize>>,
        out: &mut Vec<(usize, usize)>,
    ) {
        out.push((index, depth));
        if let Some(kids) = children.get(&index) {
            for kid in kids {
                push_tree(*kid, depth + 1, children, out);
            }
        }
    }

    let mut ordered: Vec<(usize, usize)> = Vec::new();
    for root in roots {
        push_tree(root, 0, &children, &mut ordered);
    }

    let mut lines: Vec<Line<'static>> = Vec::new();
    for (index, depth) in ordered {
        let entry = &agents[index];
        let indent = "  ".repeat(depth);
        let spawned_elapsed =
            fmt_elapsed_compact(now.saturating_duration_since(entry.created_at).as_secs());
        let activity_elapsed = fmt_elapsed_compact(
            now.saturating_duration_since(entry.status_changed_at)
                .as_secs(),
        );
        let first_line_spans: Vec<Span<'static>> = vec![
            format!("{indent}• ").into(),
            "Role: ".dim(),
            entry.role.clone().into(),
            "  ".into(),
            "Model: ".dim(),
            entry.model.clone().dim(),
            "  ".into(),
            "Reasoning: ".dim(),
            entry.reasoning.clone().dim(),
            "  ".into(),
            "Spawned: ".dim(),
            spawned_elapsed.dim(),
        ];
        lines.push(first_line_spans.into());

        let mut second_line_spans: Vec<Span<'static>> = vec![
            format!("{indent}  ").into(),
            entry.thread_id.to_string().dim(),
            "  ".into(),
            "Status: ".dim(),
            status_span(&entry.status),
            "  ".into(),
            "Context left: ".dim(),
        ];
        second_line_spans.extend(context_left_display_spans(entry.context_left_percent));
        lines.push(second_line_spans.into());

        let is_active = matches!(
            entry.status,
            AgentStatus::PendingInit | AgentStatus::Running
        );
        let activity_label = activity_label(&entry.status);
        let mut third_line_spans: Vec<Span<'static>> = vec![format!("{indent}  ").into()];
        if is_active {
            third_line_spans.push(spinner(Some(entry.status_changed_at), animations_enabled));
        } else {
            third_line_spans.push("•".dim());
        }
        third_line_spans.push(" ".into());
        if is_active && animations_enabled {
            third_line_spans.extend(shimmer_spans(activity_label));
        } else {
            third_line_spans.push(activity_label_span(activity_label, &entry.status));
        }
        third_line_spans.push(" ".into());
        third_line_spans.push(format!("({activity_elapsed})").dim());
        lines.push(third_line_spans.into());

        // SAW COMMIT OPEN: render last tool line.
        // Role: show the most recent tool-related event (or approval request) seen for this agent.
        let mut fourth_line_spans: Vec<Span<'static>> =
            vec![format!("{indent}  ").into(), "Last tool: ".dim()];
        if let Some(tool) = entry.last_tool.as_ref() {
            fourth_line_spans.push(tool.clone().into());
        } else {
            fourth_line_spans.push("—".dim());
        }
        lines.push(fourth_line_spans.into());

        if let Some(detail) = entry.last_tool_detail.as_ref() {
            lines.push(vec![format!("{indent}    └ ").dim(), detail.clone().dim()].into());
        }
        // SAW COMMIT CLOSE: render last tool line.
    }

    lines
}

fn status_span(status: &AgentStatus) -> Span<'static> {
    match status {
        AgentStatus::PendingInit => "pending init".dim(),
        AgentStatus::Running => "running".cyan(),
        AgentStatus::Completed(_) => "completed".green(),
        AgentStatus::Errored(_) => "errored".red(),
        AgentStatus::Shutdown => "shutdown".dim(),
        AgentStatus::NotFound => "not found".red(),
    }
}

fn context_left_display_spans(percent: Option<i64>) -> Vec<Span<'static>> {
    match percent {
        Some(percent) => {
            let percent = percent.clamp(0, 100);
            let percent_span = if percent < 15 {
                format!("{percent}%").red()
            } else if percent < 30 {
                format!("{percent}%").magenta()
            } else {
                format!("{percent}%").into()
            };
            vec![percent_span, " left".dim()]
        }
        None => vec!["—".dim()],
    }
}

fn activity_label(status: &AgentStatus) -> &'static str {
    match status {
        AgentStatus::PendingInit | AgentStatus::Running => "Working",
        AgentStatus::Completed(_) => "Completed",
        AgentStatus::Errored(_) => "Errored",
        AgentStatus::Shutdown => "Shutdown",
        AgentStatus::NotFound => "Not found",
    }
}

fn activity_label_span(label: &'static str, status: &AgentStatus) -> Span<'static> {
    match status {
        AgentStatus::PendingInit => label.dim(),
        AgentStatus::Running => label.into(),
        AgentStatus::Completed(_) => label.green(),
        AgentStatus::Errored(_) => label.red(),
        AgentStatus::Shutdown => label.dim(),
        AgentStatus::NotFound => label.red(),
    }
}
