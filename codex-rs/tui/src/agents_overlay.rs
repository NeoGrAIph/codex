use crate::key_hint::KeyBindingListExt;
use crate::keymap::PagerKeymap;
use crate::motion::MotionMode;
use crate::motion::ReducedMotionIndicator;
use crate::motion::activity_indicator;
use crate::multi_agents::format_agent_picker_item_name;
use crate::pager_overlay::StaticOverlay;
use crate::status::format_directory_display;
use crate::tui;
use crate::tui::TuiEvent;
use codex_protocol::ThreadId;
use color_eyre::Result;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use textwrap::wrap;

pub(crate) const AGENTS_OVERLAY_TITLE: &str = "A G E N T S";
const AGENTS_OVERLAY_ACTION_LABELS: [&str; 3] = ["Inspect", "Connect", "Close"];
const AGENTS_OVERLAY_CONFIRM_LABELS: [&str; 2] = ["No", "Yes"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentOverlayInput {
    pub(crate) thread_id: ThreadId,
    pub(crate) parent_thread_id: Option<ThreadId>,
    pub(crate) agent_nickname: Option<String>,
    pub(crate) agent_role: Option<String>,
    pub(crate) agent_persona: Option<String>,
    pub(crate) is_current: bool,
    pub(crate) is_closed: bool,
    pub(crate) status_label: String,
    pub(crate) activity: AgentOverlayActivity,
    pub(crate) active_since: Option<Instant>,
    pub(crate) model: Option<String>,
    pub(crate) reasoning: Option<String>,
    pub(crate) cwd: Option<PathBuf>,
    pub(crate) thread_note: Option<String>,
    pub(crate) request_text: Option<String>,
    pub(crate) last_tool_text: Option<String>,
    pub(crate) plan_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AgentOverlayRow {
    thread_id: ThreadId,
    parent_thread_id: Option<ThreadId>,
    label: String,
    is_current: bool,
    is_closed: bool,
    status_label: String,
    activity: AgentOverlayActivity,
    active_since: Option<Instant>,
    model: Option<String>,
    reasoning: Option<String>,
    cwd: Option<PathBuf>,
    thread_note: Option<String>,
    request_text: Option<String>,
    last_tool_text: Option<String>,
    plan_text: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentOverlayActivity {
    Active,
    Static,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentsOverlayEvent {
    CloseStack,
    RestoreTranscript,
    Connect(ThreadId),
    Shutdown(ThreadId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AgentsOverlayAction {
    Inspect,
    Connect,
    Close,
}

impl AgentsOverlayAction {
    fn from_index(index: usize) -> Self {
        match index {
            0 => Self::Inspect,
            1 => Self::Connect,
            _ => Self::Close,
        }
    }
}

pub(crate) struct AgentsOverlay {
    rows: Vec<AgentOverlayRow>,
    selected_thread_id: Option<ThreadId>,
    thread_order: Vec<ThreadId>,
    first_line_by_thread: HashMap<ThreadId, usize>,
    inspected_thread_id: Option<ThreadId>,
    menu_open: bool,
    action_selected_idx: usize,
    confirm_open: bool,
    confirm_thread_id: Option<ThreadId>,
    confirm_selected_idx: usize,
    degraded: bool,
    has_active_agents: bool,
    animations_enabled: bool,
    view: StaticOverlay,
    keymap: PagerKeymap,
}

impl AgentsOverlay {
    pub(crate) fn new(
        inputs: Vec<AgentOverlayInput>,
        selected_thread_id: Option<ThreadId>,
        degraded: bool,
        animations_enabled: bool,
        keymap: PagerKeymap,
    ) -> Self {
        let mut overlay = Self {
            rows: build_agent_overlay_rows(inputs),
            selected_thread_id,
            thread_order: Vec::new(),
            first_line_by_thread: HashMap::new(),
            inspected_thread_id: None,
            menu_open: false,
            action_selected_idx: 0,
            confirm_open: false,
            confirm_thread_id: None,
            confirm_selected_idx: 0,
            degraded,
            has_active_agents: false,
            animations_enabled,
            view: StaticOverlay::with_title(
                Vec::new(),
                AGENTS_OVERLAY_TITLE.to_string(),
                keymap.clone(),
            ),
            keymap,
        };
        overlay.refresh_lines();
        overlay
    }

    pub(crate) fn replace_inputs(&mut self, inputs: Vec<AgentOverlayInput>, degraded: bool) {
        self.rows = build_agent_overlay_rows(inputs);
        self.degraded = degraded;
        self.refresh_lines_preserving_scroll();
    }

    pub(crate) fn has_active_agents(&self) -> bool {
        self.has_active_agents
    }

    pub(crate) fn handle_event(
        &mut self,
        tui: &mut tui::Tui,
        event: TuiEvent,
        has_suspended_transcript: bool,
    ) -> Result<Option<AgentsOverlayEvent>> {
        match event {
            TuiEvent::Key(key_event) if is_plain_press(key_event, KeyCode::Esc) => {
                if self.confirm_open {
                    self.close_confirm_menu();
                    self.refresh_lines();
                } else if self.menu_open {
                    self.close_actions_menu();
                    self.refresh_lines();
                } else {
                    return Ok(Some(if has_suspended_transcript {
                        AgentsOverlayEvent::RestoreTranscript
                    } else {
                        AgentsOverlayEvent::CloseStack
                    }));
                }
            }
            TuiEvent::Key(key_event) if self.keymap.close.is_pressed(key_event) => {
                return Ok(Some(if has_suspended_transcript {
                    AgentsOverlayEvent::RestoreTranscript
                } else {
                    AgentsOverlayEvent::CloseStack
                }));
            }
            TuiEvent::Key(key_event) if self.keymap.close_transcript.is_pressed(key_event) => {
                return Ok(Some(AgentsOverlayEvent::CloseStack));
            }
            TuiEvent::Key(key_event) if self.confirm_open => {
                if let Some(event) = self.handle_confirm_key(key_event) {
                    return Ok(event);
                }
            }
            TuiEvent::Key(key_event) if self.menu_open => {
                if let Some(event) = self.handle_action_menu_key(key_event) {
                    return Ok(event);
                }
            }
            TuiEvent::Key(key_event)
                if is_plain_press(key_event, KeyCode::Up)
                    || is_plain_press(key_event, KeyCode::Char('k')) =>
            {
                self.select_previous();
                self.refresh_lines();
            }
            TuiEvent::Key(key_event)
                if is_plain_press(key_event, KeyCode::Down)
                    || is_plain_press(key_event, KeyCode::Char('j')) =>
            {
                self.select_next();
                self.refresh_lines();
            }
            TuiEvent::Key(key_event)
                if is_plain_press(key_event, KeyCode::Char('i'))
                    || is_plain_press(key_event, KeyCode::Char('I')) =>
            {
                self.toggle_inspect_for_selected();
                self.refresh_lines();
            }
            TuiEvent::Key(key_event) if is_plain_press(key_event, KeyCode::Enter) => {
                self.open_actions_menu();
                self.refresh_lines();
            }
            other => {
                self.view.handle_event(tui, other)?;
            }
        }
        tui.frame_requester().schedule_frame();
        Ok(None)
    }

    pub(crate) fn is_done(&self) -> bool {
        self.view.is_done()
    }

    fn handle_confirm_key(&mut self, key_event: KeyEvent) -> Option<Option<AgentsOverlayEvent>> {
        if is_plain_press(key_event, KeyCode::Left) {
            self.confirm_selected_idx = previous_wrapped(self.confirm_selected_idx, 2);
            self.refresh_lines();
            return Some(None);
        }
        if is_plain_press(key_event, KeyCode::Right) {
            self.confirm_selected_idx = next_wrapped(self.confirm_selected_idx, 2);
            self.refresh_lines();
            return Some(None);
        }
        if is_plain_press(key_event, KeyCode::Enter) {
            let thread_id = self.confirm_thread_id;
            let should_close = self.confirm_selected_idx == 1;
            self.close_actions_menu();
            self.refresh_lines();
            return Some(
                thread_id
                    .filter(|_| should_close)
                    .map(AgentsOverlayEvent::Shutdown),
            );
        }
        Some(None)
    }

    fn handle_action_menu_key(
        &mut self,
        key_event: KeyEvent,
    ) -> Option<Option<AgentsOverlayEvent>> {
        if is_plain_press(key_event, KeyCode::Up) || is_plain_press(key_event, KeyCode::Char('k')) {
            self.select_previous();
            self.refresh_lines();
            return Some(None);
        }
        if is_plain_press(key_event, KeyCode::Down) || is_plain_press(key_event, KeyCode::Char('j'))
        {
            self.select_next();
            self.refresh_lines();
            return Some(None);
        }
        if is_plain_press(key_event, KeyCode::Left) {
            self.action_selected_idx =
                previous_wrapped(self.action_selected_idx, AGENTS_OVERLAY_ACTION_LABELS.len());
            self.refresh_lines();
            return Some(None);
        }
        if is_plain_press(key_event, KeyCode::Right) {
            self.action_selected_idx =
                next_wrapped(self.action_selected_idx, AGENTS_OVERLAY_ACTION_LABELS.len());
            self.refresh_lines();
            return Some(None);
        }
        if is_plain_press(key_event, KeyCode::Enter) {
            let Some(thread_id) = self.selected_thread_id else {
                return Some(None);
            };
            match AgentsOverlayAction::from_index(self.action_selected_idx) {
                AgentsOverlayAction::Inspect => {
                    self.toggle_inspect_for_selected();
                    self.refresh_lines();
                    Some(None)
                }
                AgentsOverlayAction::Connect => {
                    self.close_actions_menu();
                    Some(Some(AgentsOverlayEvent::Connect(thread_id)))
                }
                AgentsOverlayAction::Close => {
                    self.confirm_open = true;
                    self.confirm_thread_id = Some(thread_id);
                    self.confirm_selected_idx = 0;
                    self.refresh_lines();
                    Some(None)
                }
            }
        } else {
            Some(None)
        }
    }

    fn selected_index(&self) -> Option<usize> {
        self.selected_thread_id.and_then(|id| {
            self.thread_order
                .iter()
                .position(|thread_id| *thread_id == id)
        })
    }

    fn select_previous(&mut self) {
        let len = self.thread_order.len();
        if len == 0 {
            self.selected_thread_id = None;
            return;
        }
        let idx = self.selected_index().unwrap_or(0);
        self.selected_thread_id = Some(self.thread_order[previous_wrapped(idx, len)]);
        self.close_confirm_menu();
    }

    fn select_next(&mut self) {
        let len = self.thread_order.len();
        if len == 0 {
            self.selected_thread_id = None;
            return;
        }
        let idx = self.selected_index().unwrap_or(0);
        self.selected_thread_id = Some(self.thread_order[next_wrapped(idx, len)]);
        self.close_confirm_menu();
    }

    fn open_actions_menu(&mut self) {
        if self.selected_thread_id.is_none() {
            return;
        }
        self.menu_open = true;
        self.confirm_open = false;
        self.confirm_thread_id = None;
        self.action_selected_idx = 0;
    }

    fn close_actions_menu(&mut self) {
        self.menu_open = false;
        self.close_confirm_menu();
    }

    fn close_confirm_menu(&mut self) {
        self.confirm_open = false;
        self.confirm_thread_id = None;
        self.confirm_selected_idx = 0;
    }

    fn toggle_inspect_for_selected(&mut self) {
        let Some(thread_id) = self.selected_thread_id else {
            return;
        };
        if self.inspected_thread_id == Some(thread_id) {
            self.inspected_thread_id = None;
        } else {
            self.inspected_thread_id = Some(thread_id);
        }
    }

    fn refresh_lines(&mut self) {
        self.ensure_selected_thread();
        let render = build_agents_overlay_render(
            &self.rows,
            self.selected_thread_id,
            self.inspected_thread_id,
            self.menu_open,
            self.action_selected_idx,
            self.confirm_open,
            self.confirm_thread_id,
            self.confirm_selected_idx,
            self.degraded,
            self.animations_enabled,
        );
        self.apply_render(render, /*preserve_scroll*/ false);
    }

    fn refresh_lines_preserving_scroll(&mut self) {
        self.ensure_selected_thread();
        let render = build_agents_overlay_render(
            &self.rows,
            self.selected_thread_id,
            self.inspected_thread_id,
            self.menu_open,
            self.action_selected_idx,
            self.confirm_open,
            self.confirm_thread_id,
            self.confirm_selected_idx,
            self.degraded,
            self.animations_enabled,
        );
        self.apply_render(render, /*preserve_scroll*/ true);
    }

    fn apply_render(&mut self, render: AgentsOverlayRender, preserve_scroll: bool) {
        let previous_selected = self.selected_thread_id;
        self.thread_order = render.thread_order;
        self.first_line_by_thread = render.first_line_by_thread;
        self.has_active_agents = render.has_active_agents;
        self.selected_thread_id = previous_selected
            .filter(|id| self.thread_order.contains(id))
            .or_else(|| self.thread_order.first().copied());
        if !self
            .inspected_thread_id
            .is_some_and(|id| self.thread_order.contains(&id))
        {
            self.inspected_thread_id = None;
        }
        if !self
            .confirm_thread_id
            .is_some_and(|id| self.thread_order.contains(&id))
        {
            self.close_confirm_menu();
        }
        if preserve_scroll {
            self.view.replace_lines_preserving_scroll(render.lines);
        } else {
            self.view.replace_lines(render.lines);
        }
        if let Some(offset) = self
            .selected_thread_id
            .and_then(|id| self.first_line_by_thread.get(&id).copied())
        {
            self.view.scroll_chunk_into_view(offset);
        }
    }

    fn ensure_selected_thread(&mut self) {
        let order = ordered_tree_rows(&self.rows)
            .into_iter()
            .map(|(index, _)| self.rows[index].thread_id)
            .collect::<Vec<_>>();
        self.selected_thread_id = self
            .selected_thread_id
            .filter(|thread_id| order.contains(thread_id))
            .or_else(|| order.first().copied());
    }
}

#[derive(Debug, Default)]
struct AgentsOverlayRender {
    lines: Vec<Line<'static>>,
    thread_order: Vec<ThreadId>,
    first_line_by_thread: HashMap<ThreadId, usize>,
    has_active_agents: bool,
}

#[allow(clippy::too_many_arguments)]
fn build_agents_overlay_render(
    rows: &[AgentOverlayRow],
    selected_thread_id: Option<ThreadId>,
    inspected_thread_id: Option<ThreadId>,
    menu_open: bool,
    action_selected_idx: usize,
    confirm_open: bool,
    confirm_thread_id: Option<ThreadId>,
    confirm_selected_idx: usize,
    degraded: bool,
    animations_enabled: bool,
) -> AgentsOverlayRender {
    let mut render = AgentsOverlayRender::default();
    if degraded {
        render
            .lines
            .push("Some agent details could not be refreshed.".cyan().into());
        render.lines.push("".into());
    }

    if rows.is_empty() {
        render
            .lines
            .push("No sub-agent threads available.".italic().into());
        return render;
    }

    let ordered = ordered_tree_rows(rows);
    for (row_index, depth) in ordered {
        let row = &rows[row_index];
        let indent = "  ".repeat(depth);
        let is_selected = selected_thread_id == Some(row.thread_id);

        render
            .first_line_by_thread
            .insert(row.thread_id, render.lines.len());
        render.thread_order.push(row.thread_id);

        let mut header = vec![
            indent.clone().into(),
            if is_selected {
                "▶ ".cyan()
            } else {
                "  ".into()
            },
            match row.activity {
                AgentOverlayActivity::Active => {
                    render.has_active_agents = true;
                    activity_indicator(
                        row.active_since,
                        MotionMode::from_animations_enabled(animations_enabled),
                        ReducedMotionIndicator::StaticBullet,
                    )
                    .unwrap_or_else(|| "•".dim())
                }
                AgentOverlayActivity::Static => "•".green(),
                AgentOverlayActivity::Closed => "•".dim(),
            },
            " ".into(),
            row.label.clone().bold(),
        ];
        if row.is_current {
            header.push(" (current)".cyan());
        }
        render.lines.push(header.into());

        let mut summary = vec![
            format!("{indent}  ").into(),
            "Status: ".dim(),
            status_span(&row.status_label, row.is_closed),
        ];
        if let Some(model) = non_empty(row.model.as_deref()) {
            summary.extend(["  ".into(), "Model: ".dim(), model.to_string().dim()]);
        }
        if let Some(reasoning) = non_empty(row.reasoning.as_deref()) {
            summary.extend([
                "  ".into(),
                "Reasoning: ".dim(),
                reasoning.to_string().dim(),
            ]);
        }
        summary.extend(["  ".into(), row.thread_id.to_string().dim()]);
        render.lines.push(summary.into());

        if let Some(cwd) = row.cwd.as_ref() {
            render.lines.push(
                vec![
                    format!("{indent}  ").into(),
                    "Directory: ".dim(),
                    format_directory_display(cwd, Some(96)).dim(),
                ]
                .into(),
            );
        }

        if let Some(note) = non_empty(row.thread_note.as_deref()) {
            render
                .lines
                .extend(wrap_prefixed(&format!("{indent}  Note: "), note, 100));
        }

        if is_selected && menu_open {
            render
                .lines
                .push(render_action_line(&indent, action_selected_idx));
        }

        if is_selected && inspected_thread_id == Some(row.thread_id) {
            let mode = if row.is_closed { "Replay only" } else { "Live" };
            render
                .lines
                .push(vec![format!("{indent}  ").into(), "Inspect: ".dim(), mode.cyan()].into());
            if let Some(request) = non_empty(row.request_text.as_deref()) {
                render.lines.extend(wrap_prefixed(
                    &format!("{indent}    Request: "),
                    request,
                    100,
                ));
            }
            if let Some(tool) = non_empty(row.last_tool_text.as_deref()) {
                render
                    .lines
                    .extend(wrap_prefixed(&format!("{indent}    Tool: "), tool, 100));
            }
            if let Some(plan) = non_empty(row.plan_text.as_deref()) {
                render
                    .lines
                    .extend(wrap_prefixed(&format!("{indent}    Plan: "), plan, 100));
            }
        }

        if confirm_open && confirm_thread_id == Some(row.thread_id) {
            render
                .lines
                .push(render_confirm_line(&indent, confirm_selected_idx));
        }

        render.lines.push("".into());
    }
    if render.lines.last().is_some_and(line_is_empty) {
        render.lines.pop();
    }
    render
}

fn build_agent_overlay_rows(inputs: Vec<AgentOverlayInput>) -> Vec<AgentOverlayRow> {
    inputs
        .into_iter()
        .map(|input| AgentOverlayRow {
            thread_id: input.thread_id,
            parent_thread_id: input.parent_thread_id,
            label: format_agent_label(
                input.agent_nickname.as_deref(),
                input.agent_role.as_deref(),
                input.agent_persona.as_deref(),
            ),
            is_current: input.is_current,
            is_closed: input.is_closed,
            status_label: input.status_label,
            activity: input.activity,
            active_since: input.active_since,
            model: input.model,
            reasoning: input.reasoning,
            cwd: input.cwd,
            thread_note: input.thread_note,
            request_text: input.request_text,
            last_tool_text: input.last_tool_text,
            plan_text: input.plan_text,
        })
        .collect()
}

fn ordered_tree_rows(rows: &[AgentOverlayRow]) -> Vec<(usize, usize)> {
    let mut index_by_thread_id = HashMap::new();
    for (index, row) in rows.iter().enumerate() {
        index_by_thread_id.insert(row.thread_id, index);
    }

    let mut roots = Vec::new();
    let mut children: HashMap<usize, Vec<usize>> = HashMap::new();
    for (index, row) in rows.iter().enumerate() {
        if let Some(parent) = row
            .parent_thread_id
            .and_then(|parent| index_by_thread_id.get(&parent).copied())
        {
            children.entry(parent).or_default().push(index);
        } else {
            roots.push(index);
        }
    }

    fn push_tree(
        index: usize,
        depth: usize,
        children: &HashMap<usize, Vec<usize>>,
        out: &mut Vec<(usize, usize)>,
    ) {
        out.push((index, depth));
        if let Some(child_indexes) = children.get(&index) {
            for child_index in child_indexes {
                push_tree(*child_index, depth + 1, children, out);
            }
        }
    }

    let mut ordered = Vec::new();
    for root in roots {
        push_tree(root, 0, &children, &mut ordered);
    }
    ordered
}

fn render_action_line(indent: &str, selected_idx: usize) -> Line<'static> {
    let mut spans = vec![format!("{indent}  Actions: ").dim()];
    for (index, label) in AGENTS_OVERLAY_ACTION_LABELS.iter().enumerate() {
        if index > 0 {
            spans.push("  ".into());
        }
        if selected_idx == index {
            spans.push(format!("[{label}]").cyan());
        } else {
            spans.push((*label).into());
        }
    }
    spans.into()
}

fn render_confirm_line(indent: &str, selected_idx: usize) -> Line<'static> {
    let mut spans = vec![format!("{indent}  Close agent? ").red()];
    for (index, label) in AGENTS_OVERLAY_CONFIRM_LABELS.iter().enumerate() {
        if index > 0 {
            spans.push("  ".into());
        }
        if selected_idx == index {
            spans.push(format!("[{label}]").cyan());
        } else {
            spans.push((*label).into());
        }
    }
    spans.into()
}

fn status_span(status: &str, is_closed: bool) -> Span<'static> {
    if is_closed {
        return status.to_string().dim();
    }
    match status {
        "Pending init" => status.to_string().cyan(),
        "Running" | "Working" => status.to_string().magenta(),
        "Interrupted" => status.to_string().cyan(),
        "Errored" | "Not found" => status.to_string().red(),
        "Shutdown" | "Closed" => status.to_string().dim(),
        _ => status.to_string().green(),
    }
}

fn wrap_prefixed(prefix: &str, body: &str, width: usize) -> Vec<Line<'static>> {
    let effective_width = width.max(prefix.len() + 10);
    wrap(body.trim(), effective_width.saturating_sub(prefix.len()))
        .into_iter()
        .enumerate()
        .map(|(index, segment)| {
            if index == 0 {
                vec![prefix.to_string().dim(), segment.into_owned().dim()].into()
            } else {
                vec![" ".repeat(prefix.len()).into(), segment.into_owned().dim()].into()
            }
        })
        .collect()
}

fn format_agent_label(
    agent_nickname: Option<&str>,
    agent_role: Option<&str>,
    agent_persona: Option<&str>,
) -> String {
    let mut label =
        format_agent_picker_item_name(agent_nickname, agent_role, /*is_primary*/ false);
    if let Some(persona) = agent_persona
        .map(str::trim)
        .filter(|persona| !persona.is_empty() && !persona.eq_ignore_ascii_case("default"))
    {
        label.push_str(&format!(" ({persona})"));
    }
    label
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn next_wrapped(index: usize, len: usize) -> usize {
    if len == 0 { 0 } else { (index + 1) % len }
}

fn previous_wrapped(index: usize, len: usize) -> usize {
    if len == 0 || index == 0 {
        len.saturating_sub(1)
    } else {
        index - 1
    }
}

fn line_is_empty(line: &Line<'_>) -> bool {
    line.spans.iter().all(|span| span.content.is_empty())
}

fn is_plain_press(key_event: KeyEvent, code: KeyCode) -> bool {
    key_event.code == code
        && key_event.modifiers == KeyModifiers::NONE
        && matches!(key_event.kind, KeyEventKind::Press | KeyEventKind::Repeat)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn thread_id(suffix: u32) -> ThreadId {
        ThreadId::from_string(&format!("00000000-0000-0000-0000-{suffix:012}"))
            .expect("valid thread id")
    }

    fn input(thread_id: ThreadId) -> AgentOverlayInput {
        AgentOverlayInput {
            thread_id,
            parent_thread_id: None,
            agent_nickname: Some("Gibbs".to_string()),
            agent_role: Some("runner".to_string()),
            agent_persona: None,
            is_current: false,
            is_closed: false,
            status_label: "Completed".to_string(),
            activity: AgentOverlayActivity::Static,
            active_since: None,
            model: Some("gpt-5.5".to_string()),
            reasoning: Some("medium".to_string()),
            cwd: None,
            thread_note: None,
            request_text: None,
            last_tool_text: None,
            plan_text: None,
        }
    }

    fn plain_lines(lines: &[Line<'static>]) -> Vec<String> {
        lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect()
    }

    #[test]
    fn render_keeps_tree_order_and_actions() {
        let parent = thread_id(1);
        let child = thread_id(2);
        let mut parent_input = input(parent);
        parent_input.agent_nickname = Some("Scout".to_string());
        parent_input.agent_role = Some("explorer".to_string());
        parent_input.agent_persona = Some("critic".to_string());
        parent_input.is_current = true;
        let mut child_input = input(child);
        child_input.parent_thread_id = Some(parent);
        child_input.agent_nickname = Some("Build".to_string());
        child_input.agent_role = Some("worker".to_string());

        let rows = build_agent_overlay_rows(vec![parent_input, child_input]);
        let render = build_agents_overlay_render(
            &rows,
            Some(parent),
            None,
            true,
            1,
            false,
            None,
            0,
            false,
            false,
        );
        let lines = plain_lines(&render.lines);

        assert!(lines[0].contains("Scout [explorer] (critic) (current)"));
        assert!(
            lines
                .iter()
                .any(|line| line.contains("Actions: Inspect  [Connect]  Close"))
        );
        assert!(
            lines
                .iter()
                .any(|line| line.contains("    • Build [worker]"))
        );
        assert_eq!(render.thread_order, vec![parent, child]);
    }

    #[test]
    fn inspect_renders_request_tool_plan_only_when_selected() {
        let id = thread_id(3);
        let mut item = input(id);
        item.request_text = Some("Run focused tests".to_string());
        item.last_tool_text = Some("Command: cargo test (completed)".to_string());
        item.plan_text = Some("Tasks 1/2 - Run renderer tests".to_string());

        let rows = build_agent_overlay_rows(vec![item]);
        let collapsed = plain_lines(
            &build_agents_overlay_render(
                &rows,
                Some(id),
                None,
                false,
                0,
                false,
                None,
                0,
                false,
                false,
            )
            .lines,
        );
        assert!(!collapsed.iter().any(|line| line.contains("Plan:")));

        let expanded = plain_lines(
            &build_agents_overlay_render(
                &rows,
                Some(id),
                Some(id),
                false,
                0,
                false,
                None,
                0,
                false,
                false,
            )
            .lines,
        );
        assert!(
            expanded
                .iter()
                .any(|line| line.contains("Request: Run focused tests"))
        );
        assert!(
            expanded
                .iter()
                .any(|line| line.contains("Tool: Command: cargo test"))
        );
        assert!(expanded.iter().any(|line| line.contains("Plan: Tasks 1/2")));
    }

    #[test]
    fn confirm_line_is_scoped_to_selected_thread() {
        let id = thread_id(4);
        let rows = build_agent_overlay_rows(vec![input(id)]);
        let lines = plain_lines(
            &build_agents_overlay_render(
                &rows,
                Some(id),
                None,
                true,
                2,
                true,
                Some(id),
                1,
                false,
                false,
            )
            .lines,
        );

        assert!(
            lines
                .iter()
                .any(|line| line.contains("Actions: Inspect  Connect  [Close]"))
        );
        assert!(
            lines
                .iter()
                .any(|line| line.contains("Close agent? No  [Yes]"))
        );
    }

    #[test]
    fn action_menu_vertical_keys_move_selected_agent() {
        let first = thread_id(5);
        let second = thread_id(6);
        let mut overlay = AgentsOverlay::new(
            vec![input(first), input(second)],
            Some(first),
            /*degraded*/ false,
            /*animations_enabled*/ false,
            crate::keymap::RuntimeKeymap::defaults().pager,
        );
        overlay.open_actions_menu();
        overlay.action_selected_idx = 2;

        let down = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(overlay.handle_action_menu_key(down), Some(None));
        assert_eq!(overlay.selected_thread_id, Some(second));
        assert!(overlay.menu_open);
        assert_eq!(overlay.action_selected_idx, 2);

        let up = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(overlay.handle_action_menu_key(up), Some(None));
        assert_eq!(overlay.selected_thread_id, Some(first));
        assert!(overlay.menu_open);
        assert_eq!(overlay.action_selected_idx, 2);
    }
}
