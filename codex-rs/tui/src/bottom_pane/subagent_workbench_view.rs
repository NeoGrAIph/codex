use super::CancellationEvent;
use super::bottom_pane_view::BottomPaneView;
use super::selection_popup_common::render_menu_surface;
use crate::app_event::AppEvent;
use crate::app_event_sender::AppEventSender;
use crate::key_hint::KeyBindingListExt;
use crate::keymap::ListKeymap;
use crate::render::renderable::Renderable;
use codex_protocol::ThreadId;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::buffer::Buffer;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;

pub(crate) const SUBAGENT_WORKBENCH_VIEW_ID: &str = "subagent-workbench";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SubagentWorkbenchRow {
    pub(crate) thread_id: ThreadId,
    pub(crate) label: String,
    pub(crate) status: String,
    pub(crate) role: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) reasoning: Option<String>,
    pub(crate) service_tier: Option<String>,
    pub(crate) spawned: Option<String>,
    pub(crate) context: Option<String>,
    pub(crate) workdir: Option<String>,
    pub(crate) note: Option<String>,
    pub(crate) last_tool: Option<String>,
    pub(crate) current_activity: Option<String>,
    pub(crate) plan: Option<String>,
    pub(crate) request: Option<String>,
    pub(crate) is_current: bool,
    pub(crate) actions: Vec<SubagentWorkbenchAction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SubagentWorkbenchAction {
    pub(crate) label: String,
    pub(crate) kind: SubagentWorkbenchActionKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SubagentWorkbenchActionKind {
    Connect { thread_id: ThreadId },
    Message { thread_id: ThreadId },
    FollowUp { thread_id: ThreadId },
    Interrupt { thread_id: ThreadId },
    Close { thread_id: ThreadId },
    Retry { thread_id: ThreadId },
    Dismiss { thread_id: ThreadId },
    StopAll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkbenchFocus {
    Agents,
    Actions,
}

pub(crate) struct SubagentWorkbenchView {
    rows: Vec<SubagentWorkbenchRow>,
    selected_row_idx: usize,
    selected_action_idx: usize,
    focus: WorkbenchFocus,
    complete: bool,
    app_event_tx: AppEventSender,
    keymap: ListKeymap,
}

impl SubagentWorkbenchView {
    pub(crate) fn new(
        rows: Vec<SubagentWorkbenchRow>,
        initial_selected_idx: Option<usize>,
        app_event_tx: AppEventSender,
        keymap: ListKeymap,
    ) -> Self {
        let selected_row_idx = initial_selected_idx
            .filter(|idx| *idx < rows.len())
            .unwrap_or(0);
        Self {
            rows,
            selected_row_idx,
            selected_action_idx: 0,
            focus: WorkbenchFocus::Agents,
            complete: false,
            app_event_tx,
            keymap,
        }
    }

    fn selected_row(&self) -> Option<&SubagentWorkbenchRow> {
        self.rows.get(self.selected_row_idx)
    }

    fn selected_actions(&self) -> &[SubagentWorkbenchAction] {
        self.selected_row()
            .map(|row| row.actions.as_slice())
            .unwrap_or(&[])
    }

    fn move_agent(&mut self, step: isize) {
        if self.rows.is_empty() {
            return;
        }
        let len = self.rows.len();
        self.selected_row_idx = if step.is_negative() {
            self.selected_row_idx.checked_sub(1).unwrap_or(len - 1)
        } else {
            (self.selected_row_idx + 1) % len
        };
        self.selected_action_idx = 0;
    }

    fn move_action(&mut self, step: isize) {
        let len = self.selected_actions().len();
        if len == 0 {
            return;
        }
        self.selected_action_idx = if step.is_negative() {
            self.selected_action_idx.checked_sub(1).unwrap_or(len - 1)
        } else {
            (self.selected_action_idx + 1) % len
        };
    }

    fn activate(&mut self) {
        match self.focus {
            WorkbenchFocus::Agents => {
                if let Some(row) = self.selected_row() {
                    self.app_event_tx
                        .send(AppEvent::SelectAgentThread(row.thread_id));
                    self.complete = true;
                }
            }
            WorkbenchFocus::Actions => {
                let Some(action) = self.selected_actions().get(self.selected_action_idx) else {
                    return;
                };
                self.app_event_tx.send(action.kind.to_app_event());
                self.complete = true;
            }
        }
    }

    fn lines(&self) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        for (idx, row) in self.rows.iter().enumerate() {
            let selected = idx == self.selected_row_idx;
            let pointer = if selected { "▶ " } else { "  " };
            let status = status_display(row.status.as_str());
            let label = truncate_display(row.label.as_str(), /*max_graphemes*/ 28);
            let role = row.role.as_deref().unwrap_or("agent");
            let model = row.model.as_deref().unwrap_or("—");
            let reasoning = row.reasoning.as_deref().unwrap_or("—");
            let spawned = row.spawned.as_deref().unwrap_or("—");
            let line = format!(
                "{pointer}{label}  • {status}  Role: {role}  Model: {model}  Reasoning: {reasoning}  Spawned: {spawned}"
            );
            lines.push(if selected {
                Line::from(line.bold())
            } else {
                Line::from(line)
            });

            let context = row.context.as_deref().unwrap_or("—");
            let workdir = row.workdir.as_deref().unwrap_or("—");
            lines.push(Line::from(format!(
                "  Context left: {context}  |  Workdir: {workdir}"
            )));

            let last_tool = row.last_tool.as_deref().unwrap_or("—");
            let activity = row.current_activity.as_deref().unwrap_or("—");
            lines.push(Line::from(format!(
                "  Last tool: {last_tool:<20} |  Current: {activity}"
            )));

            if let Some(plan) = row.plan.as_deref().filter(|plan| !plan.trim().is_empty()) {
                lines.push(Line::from("  Plan:"));
                if let Some(note) = row.note.as_deref().filter(|note| !note.trim().is_empty()) {
                    lines.push(Line::from(format!("    Note: {note}")));
                }
                lines.push(Line::from(format!("    [>] {plan}")));
            }
        }

        lines.push(Line::from(""));
        let action_title = self
            .selected_row()
            .map(|row| format!("Actions for {}:", row.label))
            .unwrap_or_else(|| "Actions:".to_string());
        lines.push(Line::from(action_title.bold()));
        let actions = self.selected_actions();
        if actions.is_empty() {
            lines.push(Line::from("  —"));
        } else {
            for (idx, action) in actions.iter().enumerate() {
                let marker =
                    if self.focus == WorkbenchFocus::Actions && idx == self.selected_action_idx {
                        "> "
                    } else {
                        "  "
                    };
                lines.push(Line::from(format!("{marker}{}", action.label)));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from("Inspect:".bold()));
        if let Some(row) = self.selected_row() {
            lines.push(Line::from(format!("  Thread: {}", row.thread_id)));
            if let Some(note) = row.note.as_deref().filter(|value| !value.trim().is_empty()) {
                lines.push(Line::from(format!("  Note: {note}")));
            }
            if let Some(workdir) = row
                .workdir
                .as_deref()
                .filter(|value| !value.trim().is_empty())
            {
                lines.push(Line::from(format!("  Directory: {workdir}")));
            }
            if let Some(service_tier) = row
                .service_tier
                .as_deref()
                .filter(|value| !value.trim().is_empty())
            {
                lines.push(Line::from(format!("  Service tier: {service_tier}")));
            }
            if let Some(request) = row
                .request
                .as_deref()
                .filter(|value| !value.trim().is_empty())
            {
                lines.push(Line::from("  Request:"));
                for line in request.lines().take(4) {
                    lines.push(Line::from(format!("    {}", line.trim())));
                }
            }
        }
        lines
    }
}

impl SubagentWorkbenchActionKind {
    fn to_app_event(&self) -> AppEvent {
        match *self {
            Self::Connect { thread_id } => AppEvent::SelectAgentThread(thread_id),
            Self::Message { thread_id } => AppEvent::OpenAgentMessagePrompt { thread_id },
            Self::FollowUp { thread_id } => AppEvent::OpenAgentFollowupPrompt { thread_id },
            Self::Interrupt { thread_id } => AppEvent::OpenAgentInterruptConfirmation { thread_id },
            Self::Close { thread_id } => AppEvent::OpenAgentCloseConfirmation { thread_id },
            Self::Retry { thread_id } => AppEvent::OpenAgentRetryConfirmation { thread_id },
            Self::Dismiss { thread_id } => AppEvent::DismissAgentThread { thread_id },
            Self::StopAll => AppEvent::OpenAgentStopAllConfirmation,
        }
    }
}

impl BottomPaneView for SubagentWorkbenchView {
    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event {
            _ if self.keymap.move_up.is_pressed(key_event) => match self.focus {
                WorkbenchFocus::Agents => self.move_agent(/*step*/ -1),
                WorkbenchFocus::Actions => self.move_action(/*step*/ -1),
            },
            _ if self.keymap.move_down.is_pressed(key_event) => match self.focus {
                WorkbenchFocus::Agents => self.move_agent(/*step*/ 1),
                WorkbenchFocus::Actions => self.move_action(/*step*/ 1),
            },
            _ if self.keymap.move_left.is_pressed(key_event) => self.focus = WorkbenchFocus::Agents,
            _ if self.keymap.move_right.is_pressed(key_event)
                && !self.selected_actions().is_empty() =>
            {
                self.focus = WorkbenchFocus::Actions;
            }
            KeyEvent {
                code: KeyCode::Tab, ..
            } => {
                self.focus = match self.focus {
                    WorkbenchFocus::Agents if !self.selected_actions().is_empty() => {
                        WorkbenchFocus::Actions
                    }
                    WorkbenchFocus::Agents => WorkbenchFocus::Agents,
                    WorkbenchFocus::Actions => WorkbenchFocus::Agents,
                };
            }
            _ if self.keymap.accept.is_pressed(key_event) => self.activate(),
            _ if self.keymap.cancel.is_pressed(key_event) => self.complete = true,
            KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: KeyModifiers::NONE,
                ..
            } => self.complete = true,
            _ => {}
        }
    }

    fn is_complete(&self) -> bool {
        self.complete
    }

    fn view_id(&self) -> Option<&'static str> {
        Some(SUBAGENT_WORKBENCH_VIEW_ID)
    }

    fn prefer_esc_to_handle_key_event(&self) -> bool {
        true
    }

    fn on_ctrl_c(&mut self) -> CancellationEvent {
        self.complete = true;
        CancellationEvent::Handled
    }
}

impl Renderable for SubagentWorkbenchView {
    fn desired_height(&self, _width: u16) -> u16 {
        let line_count = self.lines().len() as u16;
        line_count.saturating_add(3)
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let [content_area, footer_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(area);
        let content_area = render_menu_surface(content_area, buf);
        Paragraph::new(self.lines()).render(content_area, buf);
        let hint = "↑/↓ agents/actions · → actions · ← agents · enter select · esc close".dim();
        Paragraph::new(Line::from(hint)).render(footer_area, buf);
    }
}

fn status_display(status: &str) -> String {
    match status {
        "running" => "Working".to_string(),
        "idle" => "Idle".to_string(),
        "waiting approval" | "waiting user" => "Waiting".to_string(),
        "error" => "Error".to_string(),
        "closed" => "Completed".to_string(),
        other => other.to_string(),
    }
}

fn truncate_display(value: &str, max_graphemes: usize) -> String {
    crate::text_formatting::truncate_text(value.trim(), max_graphemes)
}
