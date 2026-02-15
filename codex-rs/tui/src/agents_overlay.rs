// FORK COMMIT NEW FILE [SAW]: AGENTS overlay renderer extracted into its own module.
// Role: keep SubAgentsWindow rendering logic isolated from App state management.
use codex_core::protocol::AgentStatus;
use codex_protocol::ThreadId;
use codex_protocol::plan_tool::StepStatus;
use codex_protocol::plan_tool::UpdatePlanArgs;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use std::collections::HashMap;
use std::time::Instant;
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
    pub(crate) status: AgentStatus,
    pub(crate) created_at: Instant,
    pub(crate) status_changed_at: Instant,
    pub(crate) status_detail: Option<String>,
    // FORK COMMIT OPEN [SAW]: plan update for AGENTS overlay.
    // Role: render the most recent update_plan payload under the tool summary.
    pub(crate) plan_update: Option<UpdatePlanArgs>,
    // FORK COMMIT CLOSE: plan update for AGENTS overlay.
    pub(crate) context_left_percent: Option<i64>,
    // FORK COMMIT OPEN [SAW]: last tool for SAW summary.
    // Role: surface the last tool-related event seen on this thread as a single summary line.
    pub(crate) last_tool: Option<String>,
    pub(crate) last_tool_detail: Option<String>,
    // FORK COMMIT CLOSE: last tool for SAW summary.
}

pub(crate) fn build_agents_overlay_lines(
    agents: &[AgentSummaryEntry],
    now: Instant,
    animations_enabled: bool,
) -> Vec<Line<'static>> {
    const LAST_TOOL_COLUMN_WIDTH: usize = 28;

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
            format!("{indent}").into(),
            // FORK COMMIT [SAW]: move status indicator + label into the first line.
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
        lines.push(first_line_spans.into());

        let mut second_line_spans: Vec<Span<'static>> =
            vec![format!("{indent}  ").into(), "Context left: ".dim()];
        second_line_spans.extend(context_left_display_spans(entry.context_left_percent));
        second_line_spans.extend(["  ".into(), entry.thread_id.to_string().dim()]);
        lines.push(second_line_spans.into());

        // FORK COMMIT OPEN [SAW]: render last tool line.
        // Role: show the most recent tool-related event (or approval request) seen for this agent.
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
        // FORK COMMIT [SAW]: keep separator in a stable column to reduce visual jitter.
        third_line_spans.push("  ".into());
        third_line_spans.push("|".dim());
        third_line_spans.push("  ".into());
        third_line_spans.push(status_detail.dim());
        lines.push(third_line_spans.into());

        if let Some(detail) = entry.last_tool_detail.as_ref() {
            lines.push(vec![format!("{indent}    └ ").dim(), detail.clone().dim()].into());
        }
        // FORK COMMIT OPEN [SAW]: render plan update under the tool summary.
        // Role: surface active plan items when the model emits update_plan.
        if let Some(plan_update) = entry.plan_update.as_ref() {
            if !plan_update.plan.is_empty()
                || plan_update
                    .explanation
                    .as_ref()
                    .is_some_and(|text| !text.is_empty())
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
                for item in plan_update.plan.iter() {
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
        // FORK COMMIT CLOSE: render plan update under the tool summary.
        // FORK COMMIT CLOSE: render last tool line.
    }

    lines
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
        AgentStatus::PendingInit => label.dim(), // "Working" в приглушённом цвете: агент ещё не стартовал.
        AgentStatus::Running => label.into(),    // "Working" в обычном цвете: агент активен.
        AgentStatus::Completed(_) => label.green(), // "Completed" зелёным: успешное завершение.
        AgentStatus::Errored(_) => label.red(),  // "Errored" красным: ошибка выполнения.
        AgentStatus::Shutdown => label.dim(),    // "Shutdown" приглушённым: корректное выключение.
        AgentStatus::NotFound => label.red(),    // "Not found" красным: агент не найден.
    }
}
