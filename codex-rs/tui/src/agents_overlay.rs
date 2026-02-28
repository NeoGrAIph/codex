use codex_protocol::ThreadId;
use codex_protocol::plan_tool::StepStatus;
use codex_protocol::plan_tool::UpdatePlanArgs;
use codex_protocol::protocol::AgentStatus;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use std::collections::HashMap;
use std::time::Instant;
use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

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
    pub(crate) note: Option<String>,
    pub(crate) status: AgentStatus,
    pub(crate) created_at: Instant,
    pub(crate) status_changed_at: Instant,
    pub(crate) status_detail: Option<String>,
    pub(crate) plan_update: Option<UpdatePlanArgs>,
    pub(crate) context_left_percent: Option<i64>,
    pub(crate) last_tool: Option<String>,
    pub(crate) last_tool_detail: Option<String>,
}

pub(crate) fn build_agents_overlay_lines(
    agents: &[AgentSummaryEntry],
    now: Instant,
    animations_enabled: bool,
    line_width: u16,
) -> Vec<Line<'static>> {
    const LAST_TOOL_COLUMN_WIDTH: usize = 28;
    const NOTE_COLUMN_START: usize = 115;

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
        let is_active = matches!(
            entry.status,
            AgentStatus::PendingInit | AgentStatus::Running
        );
        let activity_label = activity_label(&entry.status);
        let mut first_line_spans: Vec<Span<'static>> = vec![
            indent.to_string().into(),
            if is_active {
                spinner(Some(entry.status_changed_at), animations_enabled)
            } else {
                "•".dim()
            },
            " ".into(),
        ];
        if is_active && animations_enabled {
            first_line_spans.extend(shimmer_spans(activity_label));
        } else {
            first_line_spans.push(activity_label_span(activity_label, &entry.status));
        }
        first_line_spans.extend([
            " ".into(),
            format!("({activity_elapsed})").dim(),
            "  ".into(),
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
        ]);
        if let Some(note) = entry.note.as_ref() {
            let first_line_width = first_line_spans
                .iter()
                .map(|span| UnicodeWidthStr::width(span.content.as_ref()))
                .sum::<usize>();
            if first_line_width < NOTE_COLUMN_START {
                first_line_spans.push(" ".repeat(NOTE_COLUMN_START - first_line_width).into());
            }
            first_line_spans.push("|".dim());
            first_line_spans.push("  ".into());
            first_line_spans.push("Note: ".dim());
            let note_prefix_width = first_line_spans
                .iter()
                .map(|span| UnicodeWidthStr::width(span.content.as_ref()))
                .sum::<usize>();
            let available_note_width = usize::from(line_width).saturating_sub(note_prefix_width);
            let note = truncate_note_to_width(note, available_note_width);
            if !note.is_empty() {
                first_line_spans.push(note.dim());
            }
        }
        lines.push(first_line_spans.into());

        let mut second_line_spans: Vec<Span<'static>> =
            vec![format!("{indent}  ").into(), "Context left: ".dim()];
        second_line_spans.extend(context_left_display_spans(entry.context_left_percent));
        second_line_spans.extend(["  ".into(), entry.thread_id.to_string().dim()]);
        lines.push(second_line_spans.into());

        let mut third_line_spans: Vec<Span<'static>> =
            vec![format!("{indent}  ").into(), "Last tool: ".dim()];
        let tool_label = entry.last_tool.clone().unwrap_or_else(|| "—".to_string());
        let status_detail = entry
            .status_detail
            .clone()
            .unwrap_or_else(|| "—".to_string());
        let tool_width = UnicodeWidthStr::width(tool_label.as_str());
        third_line_spans.push(tool_label.into());
        if tool_width < LAST_TOOL_COLUMN_WIDTH {
            third_line_spans.push(" ".repeat(LAST_TOOL_COLUMN_WIDTH - tool_width).into());
        }
        third_line_spans.push("  ".into());
        third_line_spans.push("|".dim());
        third_line_spans.push("  ".into());
        third_line_spans.push(status_detail.dim());
        lines.push(third_line_spans.into());

        if let Some(detail) = entry.last_tool_detail.as_ref() {
            lines.push(vec![format!("{indent}    └ ").dim(), detail.clone().dim()].into());
        }
        if let Some(plan_update) = entry.plan_update.as_ref()
            && (!plan_update.plan.is_empty()
                || plan_update
                    .explanation
                    .as_ref()
                    .is_some_and(|text| !text.is_empty()))
        {
            lines.push(vec![format!("{indent}  ").into(), "Plan:".dim()].into());
            if let Some(explanation) = plan_update.explanation.as_ref() {
                lines.push(
                    vec![
                        format!("{indent}    ").into(),
                        "Note: ".dim(),
                        explanation.clone().dim(),
                    ]
                    .into(),
                );
            }
            for item in &plan_update.plan {
                let status_span = match item.status {
                    StepStatus::Pending => "[ ]".dim(),
                    StepStatus::InProgress => "[>]".magenta(),
                    StepStatus::Completed => "[x]".green(),
                };
                lines.push(
                    vec![
                        format!("{indent}    ").into(),
                        status_span,
                        " ".into(),
                        item.step.clone().dim(),
                    ]
                    .into(),
                );
            }
        }
    }

    lines
}

fn truncate_note_to_width(note: &str, max_width: usize) -> String {
    let note_width = UnicodeWidthStr::width(note);
    if note_width <= max_width {
        return note.to_string();
    }

    if max_width == 0 {
        return String::new();
    }
    if max_width <= 3 {
        return ".".repeat(max_width);
    }

    let target_width = max_width - 3;
    let mut current_width = 0usize;
    let mut truncated = String::new();
    for ch in note.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if current_width + ch_width > target_width {
            break;
        }
        truncated.push(ch);
        current_width += ch_width;
    }
    truncated.push_str("...");
    truncated
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
            vec![percent_span]
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

#[cfg(test)]
mod tests {
    use super::*;
    use codex_protocol::plan_tool::PlanItemArg;
    use insta::assert_snapshot;
    use std::time::Duration;

    fn render_lines(lines: Vec<Line<'static>>) -> String {
        lines
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn agents_overlay_empty_state_snapshot() {
        let rendered = render_lines(build_agents_overlay_lines(&[], Instant::now(), false, 160));
        assert_snapshot!("agents_overlay_empty_state", rendered);
    }

    #[test]
    fn truncate_note_adds_ellipsis_when_needed() {
        assert_eq!(truncate_note_to_width("short", 10), "short");
        assert_eq!(
            truncate_note_to_width("очень длинная заметка", 7),
            "очен..."
        );
    }

    #[test]
    fn agents_overlay_running_with_plan_snapshot() {
        let now = Instant::now();
        let root =
            ThreadId::from_string("00000000-0000-0000-0000-000000000001").expect("valid thread");
        let child =
            ThreadId::from_string("00000000-0000-0000-0000-000000000002").expect("valid thread");
        let entries = vec![
            AgentSummaryEntry {
                thread_id: root,
                parent_thread_id: None,
                role: "orchestrator".to_string(),
                model: "gpt-5".to_string(),
                reasoning: "medium".to_string(),
                note: Some("Main flow".to_string()),
                status: AgentStatus::Running,
                created_at: now - Duration::from_secs(300),
                status_changed_at: now - Duration::from_secs(42),
                status_detail: Some("Collecting context".to_string()),
                plan_update: Some(UpdatePlanArgs {
                    explanation: Some("Main flow".to_string()),
                    plan: vec![
                        PlanItemArg {
                            step: "Open AGENTS overlay".to_string(),
                            status: StepStatus::Completed,
                        },
                        PlanItemArg {
                            step: "Refresh summary".to_string(),
                            status: StepStatus::InProgress,
                        },
                    ],
                }),
                context_left_percent: Some(67),
                last_tool: Some("shell".to_string()),
                last_tool_detail: Some("cargo test -p codex-tui".to_string()),
            },
            AgentSummaryEntry {
                thread_id: child,
                parent_thread_id: Some(root),
                role: "worker/runner".to_string(),
                model: "gpt-5.3-codex".to_string(),
                reasoning: "high".to_string(),
                note: None,
                status: AgentStatus::Completed(Some("done".to_string())),
                created_at: now - Duration::from_secs(200),
                status_changed_at: now - Duration::from_secs(8),
                status_detail: None,
                plan_update: None,
                context_left_percent: Some(12),
                last_tool: None,
                last_tool_detail: None,
            },
        ];

        let rendered = render_lines(build_agents_overlay_lines(&entries, now, false, 160));
        assert_snapshot!("agents_overlay_running_with_plan", rendered);
    }
}
