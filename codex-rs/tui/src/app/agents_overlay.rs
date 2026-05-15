//! App-owned integration for the full-screen agents overlay.
//!
//! The overlay is an inspection and routing surface. It projects existing thread state and delegates
//! lifecycle actions to the same thread-operation paths used elsewhere in the TUI.

use super::*;
use crate::agents_overlay::AgentOverlayActivity;
use crate::agents_overlay::AgentOverlayInput;
use crate::agents_overlay::AgentsOverlay;
use crate::agents_overlay::AgentsOverlayEvent;
use codex_app_server_protocol::CollabAgentStatus;
use codex_app_server_protocol::CollabAgentTool;
use codex_app_server_protocol::CollabAgentToolCallStatus;
use codex_app_server_protocol::CommandExecutionStatus;
use codex_app_server_protocol::DynamicToolCallStatus;
use codex_app_server_protocol::McpToolCallStatus;
use codex_app_server_protocol::PatchApplyStatus;
use codex_app_server_protocol::ThreadStatus;
use codex_app_server_protocol::UserInput;
use codex_protocol::protocol::SubAgentSource;

impl App {
    pub(super) async fn handle_overlay_event(
        &mut self,
        tui: &mut tui::Tui,
        app_server: &mut AppServerSession,
        event: TuiEvent,
    ) -> Result<bool> {
        if self.should_cycle_transcript_to_agents(&event) {
            self.open_agents_overlay_from_transcript(tui, app_server)
                .await;
            return Ok(true);
        }

        if matches!(self.overlay, Some(Overlay::Agents(_))) {
            if matches!(event, TuiEvent::Draw | TuiEvent::Resize) {
                self.refresh_agents_overlay(tui, app_server).await;
            }

            let action = {
                let has_suspended_transcript = self.suspended_transcript_overlay.is_some();
                let Some(Overlay::Agents(overlay)) = &mut self.overlay else {
                    return Ok(true);
                };
                overlay.handle_event(tui, event, has_suspended_transcript)?
            };

            match action {
                Some(AgentsOverlayEvent::CloseStack) => {
                    self.close_overlay_stack(tui);
                }
                Some(AgentsOverlayEvent::RestoreTranscript) => {
                    self.restore_suspended_transcript_overlay(tui);
                }
                Some(AgentsOverlayEvent::Connect(thread_id)) => {
                    self.close_overlay_stack(tui);
                    self.select_agent_thread_and_discard_side(tui, app_server, thread_id)
                        .await?;
                }
                Some(AgentsOverlayEvent::Shutdown(thread_id)) => {
                    self.submit_thread_op(app_server, thread_id, AppCommand::shutdown())
                        .await?;
                    self.refresh_agents_overlay(tui, app_server).await;
                }
                None => {
                    if self.overlay.as_ref().is_some_and(Overlay::is_done) {
                        self.close_overlay_stack(tui);
                    }
                }
            }
            schedule_agents_overlay_animation(tui, self.overlay.as_ref(), self.config.animations);
            tui.frame_requester().schedule_frame();
            return Ok(true);
        }

        self.handle_backtrack_overlay_event(tui, event).await
    }

    pub(super) fn for_each_transcript_overlay_mut(
        &mut self,
        mut f: impl FnMut(&mut TranscriptOverlay),
    ) {
        if let Some(Overlay::Transcript(overlay)) = &mut self.overlay {
            f(overlay);
        }
        if let Some(overlay) = &mut self.suspended_transcript_overlay {
            f(overlay);
        }
    }

    pub(super) fn close_overlay_stack(&mut self, tui: &mut tui::Tui) {
        let _ = tui.leave_alt_screen();
        let was_backtrack = self.backtrack.overlay_preview_active;
        if !self.deferred_history_lines.is_empty() {
            let lines = std::mem::take(&mut self.deferred_history_lines);
            tui.insert_history_lines_with_wrap_policy(lines, self.history_line_wrap_policy());
        }
        self.overlay = None;
        self.suspended_transcript_overlay = None;
        self.agent_overlay_active_since.clear();
        self.backtrack.overlay_preview_active = false;
        if was_backtrack {
            self.reset_backtrack_state();
        }
    }

    async fn open_agents_overlay_from_transcript(
        &mut self,
        tui: &mut tui::Tui,
        app_server: &mut AppServerSession,
    ) {
        if let Some(Overlay::Transcript(transcript)) = self.overlay.take() {
            self.suspended_transcript_overlay = Some(transcript);
        }
        self.open_agents_overlay(tui, app_server).await;
    }

    async fn open_agents_overlay(&mut self, tui: &mut tui::Tui, app_server: &mut AppServerSession) {
        if !tui.is_alt_screen_active() {
            let _ = tui.enter_alt_screen();
        }
        let (inputs, degraded) = self.agent_overlay_inputs(app_server).await;
        let selected_thread_id = self.current_displayed_thread_id();
        self.overlay = Some(Overlay::Agents(Box::new(AgentsOverlay::new(
            inputs,
            selected_thread_id,
            degraded,
            self.config.animations,
            self.keymap.pager.clone(),
        ))));
        schedule_agents_overlay_animation(tui, self.overlay.as_ref(), self.config.animations);
        tui.frame_requester().schedule_frame();
    }

    async fn refresh_agents_overlay(
        &mut self,
        tui: &mut tui::Tui,
        app_server: &mut AppServerSession,
    ) {
        let (inputs, degraded) = self.agent_overlay_inputs(app_server).await;
        if let Some(Overlay::Agents(overlay)) = &mut self.overlay {
            overlay.replace_inputs(inputs, degraded);
        }
        schedule_agents_overlay_animation(tui, self.overlay.as_ref(), self.config.animations);
    }

    fn should_cycle_transcript_to_agents(&self, event: &TuiEvent) -> bool {
        matches!(self.overlay, Some(Overlay::Transcript(_)))
            && !self.backtrack.overlay_preview_active
            && matches!(event, TuiEvent::Key(key_event) if self.keymap.pager.close_transcript.is_pressed(*key_event))
    }

    fn restore_suspended_transcript_overlay(&mut self, tui: &mut tui::Tui) {
        if let Some(transcript) = self.suspended_transcript_overlay.take() {
            self.overlay = Some(Overlay::Transcript(transcript));
        } else {
            self.close_overlay_stack(tui);
        }
        self.backtrack.overlay_preview_active = false;
    }

    async fn agent_overlay_inputs(
        &mut self,
        app_server: &mut AppServerSession,
    ) -> (Vec<AgentOverlayInput>, bool) {
        let mut degraded = false;
        if !self.backfill_loaded_subagent_threads(app_server).await {
            degraded = true;
        }

        let ordered_threads: Vec<_> = self
            .agent_navigation
            .ordered_threads()
            .into_iter()
            .map(|(thread_id, entry)| (thread_id, entry.clone()))
            .collect();
        let mut inputs = Vec::new();
        for (thread_id, entry) in ordered_threads {
            if Some(thread_id) == self.primary_thread_id
                || self.side_threads.contains_key(&thread_id)
            {
                continue;
            }

            let mut details = self
                .local_thread_overlay_details(thread_id, entry.is_closed)
                .await;
            let mut parent_thread_id = None;
            let mut agent_nickname = entry.agent_nickname.clone();
            let mut agent_role = entry.agent_role.clone();
            let mut agent_persona = None;
            let mut is_closed = entry.is_closed;

            match app_server
                .thread_read(thread_id, /*include_turns*/ false)
                .await
            {
                Ok(thread) => {
                    let source = thread_spawn_source(&thread.source);
                    parent_thread_id = source.as_ref().and_then(|source| source.parent_thread_id);
                    agent_persona = source.and_then(|source| source.agent_persona);
                    details.status_label.get_or_insert_with(|| {
                        status_label_from_thread_status(&thread.status, is_closed).to_string()
                    });
                    details
                        .cwd
                        .get_or_insert_with(|| thread.cwd.into_path_buf());
                    if let Some(nickname) = thread.agent_nickname {
                        agent_nickname = Some(nickname);
                    }
                    if let Some(role) = thread.agent_role {
                        agent_role = Some(role);
                    }
                    if let Some(persona) = thread.agent_persona {
                        agent_persona = Some(persona);
                    }
                    if let Some(note) = thread.thread_note {
                        details.thread_note = Some(note);
                    }
                    is_closed |= matches!(thread.status, ThreadStatus::NotLoaded);
                }
                Err(err) => {
                    degraded = true;
                    tracing::warn!(thread_id = %thread_id, %err, "failed to refresh agent overlay row");
                }
            }

            let parent_details = self
                .collect_parent_overlay_details(parent_thread_id, thread_id)
                .await;
            if details.status_label.as_deref() == Some("Idle") {
                details.status_label = parent_details.status_label.or(details.status_label);
            }
            if details.model.is_none() {
                details.model = parent_details.model;
            }
            if details.reasoning.is_none() {
                details.reasoning = parent_details.reasoning;
            }
            if details.thread_note.is_none() {
                details.thread_note = parent_details.thread_note.or(entry.thread_note.clone());
            }
            if details.request_text.is_none() {
                details.request_text = parent_details.request_text;
            }
            if details.last_tool_text.is_none() {
                details.last_tool_text = parent_details.last_tool_text;
            }

            let status_label = details.status_label.unwrap_or_else(|| {
                if is_closed {
                    "Closed".to_string()
                } else {
                    "Idle".to_string()
                }
            });
            let activity = activity_for_status_label(&status_label, is_closed);
            let active_since = self.overlay_active_since(thread_id, activity);

            inputs.push(AgentOverlayInput {
                thread_id,
                parent_thread_id,
                agent_nickname,
                agent_role,
                agent_persona,
                is_current: self.current_displayed_thread_id() == Some(thread_id),
                is_closed,
                status_label,
                activity,
                active_since,
                model: details.model,
                reasoning: details.reasoning,
                cwd: details.cwd,
                thread_note: details.thread_note,
                request_text: details.request_text,
                last_tool_text: details.last_tool_text,
                plan_text: details.plan_text,
            });
        }

        (inputs, degraded)
    }

    async fn local_thread_overlay_details(
        &self,
        thread_id: ThreadId,
        is_closed: bool,
    ) -> AgentOverlayDetails {
        let Some(channel) = self.thread_event_channels.get(&thread_id) else {
            return AgentOverlayDetails::default();
        };
        let store = channel.store.lock().await;
        let mut details = collect_target_overlay_details_from_store(&store);
        details.status_label = Some(status_label_from_store(&store, is_closed));
        if let Some(session) = store.session.as_ref() {
            details.cwd = Some(session.cwd.clone().into_path_buf());
            details.model = Some(session.model.clone()).filter(|model| !model.trim().is_empty());
            details.reasoning = details
                .model
                .as_deref()
                .and_then(|model| Self::reasoning_label_for(model, session.reasoning_effort))
                .map(str::to_string);
        }
        details
    }

    async fn collect_parent_overlay_details(
        &self,
        parent_thread_id: Option<ThreadId>,
        target_thread_id: ThreadId,
    ) -> AgentOverlayDetails {
        let Some(parent_thread_id) = parent_thread_id else {
            return AgentOverlayDetails::default();
        };
        let Some(channel) = self.thread_event_channels.get(&parent_thread_id) else {
            return AgentOverlayDetails::default();
        };
        let store = channel.store.lock().await;
        let mut details = AgentOverlayDetails::default();

        for event in store.buffer.iter().rev() {
            match event {
                ThreadBufferedEvent::Notification(ServerNotification::ItemStarted(
                    notification,
                )) => {
                    apply_parent_collab_details(&mut details, &notification.item, target_thread_id);
                }
                ThreadBufferedEvent::Notification(ServerNotification::ItemCompleted(
                    notification,
                )) => {
                    apply_parent_collab_details(&mut details, &notification.item, target_thread_id);
                }
                _ => {}
            }
            if details.has_parent_summary() {
                return details;
            }
        }

        for turn in store.turns.iter().rev() {
            for item in turn.items.iter().rev() {
                apply_parent_collab_details(&mut details, item, target_thread_id);
                if details.has_parent_summary() {
                    return details;
                }
            }
        }

        details
    }

    fn overlay_active_since(
        &mut self,
        thread_id: ThreadId,
        activity: AgentOverlayActivity,
    ) -> Option<Instant> {
        match activity {
            AgentOverlayActivity::Active => Some(
                *self
                    .agent_overlay_active_since
                    .entry(thread_id)
                    .or_insert_with(Instant::now),
            ),
            AgentOverlayActivity::Static | AgentOverlayActivity::Closed => {
                self.agent_overlay_active_since.remove(&thread_id);
                None
            }
        }
    }
}

#[derive(Debug, Default)]
struct AgentOverlayDetails {
    status_label: Option<String>,
    model: Option<String>,
    reasoning: Option<String>,
    cwd: Option<PathBuf>,
    thread_note: Option<String>,
    request_text: Option<String>,
    last_tool_text: Option<String>,
    plan_text: Option<String>,
}

impl AgentOverlayDetails {
    fn has_parent_summary(&self) -> bool {
        self.status_label.is_some()
            && self.thread_note.is_some()
            && self.request_text.is_some()
            && self.last_tool_text.is_some()
            && self.model.is_some()
            && self.reasoning.is_some()
    }
}

#[derive(Debug)]
struct ThreadSpawnSourceDetails {
    parent_thread_id: Option<ThreadId>,
    agent_persona: Option<String>,
}

fn thread_spawn_source(
    source: &codex_app_server_protocol::SessionSource,
) -> Option<ThreadSpawnSourceDetails> {
    match source {
        codex_app_server_protocol::SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id,
            agent_persona,
            ..
        }) => Some(ThreadSpawnSourceDetails {
            parent_thread_id: Some(*parent_thread_id),
            agent_persona: agent_persona.clone(),
        }),
        _ => None,
    }
}

fn collect_target_overlay_details_from_store(store: &ThreadEventStore) -> AgentOverlayDetails {
    let mut details = AgentOverlayDetails::default();

    for event in store.buffer.iter().rev() {
        match event {
            ThreadBufferedEvent::Notification(ServerNotification::TurnPlanUpdated(
                notification,
            )) if details.plan_text.is_none() => {
                details.plan_text =
                    summarize_plan_steps(&notification.plan, notification.explanation.as_deref());
            }
            ThreadBufferedEvent::Notification(ServerNotification::ItemStarted(notification)) => {
                apply_item_overlay_details(&mut details, &notification.item);
            }
            ThreadBufferedEvent::Notification(ServerNotification::ItemCompleted(notification)) => {
                apply_item_overlay_details(&mut details, &notification.item);
            }
            _ => {}
        }
        if details.request_text.is_some()
            && details.last_tool_text.is_some()
            && details.plan_text.is_some()
        {
            break;
        }
    }

    for turn in store.turns.iter().rev() {
        for item in turn.items.iter().rev() {
            apply_item_overlay_details(&mut details, item);
            if details.request_text.is_some()
                && details.last_tool_text.is_some()
                && details.plan_text.is_some()
            {
                break;
            }
        }
    }

    details
}

fn apply_item_overlay_details(details: &mut AgentOverlayDetails, item: &ThreadItem) {
    if details.request_text.is_none()
        && let ThreadItem::UserMessage { content, .. } = item
    {
        details.request_text = overlay_request_from_user_inputs(content);
    }
    if details.last_tool_text.is_none() {
        details.last_tool_text = summarize_tool_like_item(item);
    }
    if details.plan_text.is_none()
        && let ThreadItem::Plan { text, .. } = item
    {
        details.plan_text = compact_overlay_text(text, 72);
    }
}

fn apply_parent_collab_details(
    details: &mut AgentOverlayDetails,
    item: &ThreadItem,
    target_thread_id: ThreadId,
) {
    let target_thread_id = target_thread_id.to_string();
    let ThreadItem::CollabAgentToolCall {
        tool,
        status,
        receiver_thread_ids,
        prompt,
        model,
        reasoning_effort,
        agents_states,
        ..
    } = item
    else {
        return;
    };
    if !receiver_thread_ids
        .iter()
        .any(|thread_id| thread_id == &target_thread_id)
    {
        return;
    }

    if details.last_tool_text.is_none() {
        details.last_tool_text = Some(summarize_collab_tool(tool, status));
    }
    if details.request_text.is_none() {
        details.request_text = prompt
            .as_deref()
            .and_then(|text| compact_overlay_text(text, 96));
    }
    if details.model.is_none() {
        details.model = model.clone().filter(|model| !model.trim().is_empty());
    }
    if details.reasoning.is_none() {
        details.reasoning = details
            .model
            .as_deref()
            .zip(*reasoning_effort)
            .and_then(|(model, reasoning)| App::reasoning_label_for(model, Some(reasoning)))
            .map(str::to_string);
    }
    if let Some(agent_state) = agents_states.get(&target_thread_id) {
        if details.status_label.is_none() {
            details.status_label = Some(status_label_from_collab_status(&agent_state.status));
        }
        if details.thread_note.is_none() {
            details.thread_note = agent_state.thread_note.clone();
        }
    }
}

fn status_label_from_store(store: &ThreadEventStore, is_closed: bool) -> String {
    if is_closed {
        return "Closed".to_string();
    }
    if store.active_turn_id().is_some() {
        return "Running".to_string();
    }
    if store.turns.iter().rev().any(|turn| turn.error.is_some()) {
        return "Errored".to_string();
    }
    if let Some(turn) = store.turns.iter().rev().find(|turn| !turn.items.is_empty()) {
        match turn.status {
            TurnStatus::Completed => "Completed".to_string(),
            TurnStatus::Interrupted => "Interrupted".to_string(),
            TurnStatus::Failed => "Errored".to_string(),
            TurnStatus::InProgress => "Running".to_string(),
        }
    } else {
        "Idle".to_string()
    }
}

fn status_label_from_thread_status(status: &ThreadStatus, is_closed: bool) -> &'static str {
    if is_closed {
        return "Closed";
    }
    match status {
        ThreadStatus::Active { active_flags } if active_flags.is_empty() => "Running",
        ThreadStatus::Active { .. } => "Working",
        ThreadStatus::Idle => "Idle",
        ThreadStatus::SystemError => "Errored",
        ThreadStatus::NotLoaded => "Closed",
    }
}

fn status_label_from_collab_status(status: &CollabAgentStatus) -> String {
    match status {
        CollabAgentStatus::PendingInit => "Pending init",
        CollabAgentStatus::Running => "Running",
        CollabAgentStatus::Interrupted => "Interrupted",
        CollabAgentStatus::Completed => "Completed",
        CollabAgentStatus::Errored => "Errored",
        CollabAgentStatus::Shutdown => "Shutdown",
        CollabAgentStatus::NotFound => "Not found",
    }
    .to_string()
}

fn activity_for_status_label(status_label: &str, is_closed: bool) -> AgentOverlayActivity {
    if is_closed {
        return AgentOverlayActivity::Closed;
    }
    match status_label {
        "Pending init" | "Running" | "Working" => AgentOverlayActivity::Active,
        _ => AgentOverlayActivity::Static,
    }
}

fn summarize_plan_steps(
    plan: &[codex_app_server_protocol::TurnPlanStep],
    explanation: Option<&str>,
) -> Option<String> {
    let total = plan.len();
    if total > 0 {
        let completed = plan
            .iter()
            .filter(|step| {
                matches!(
                    step.status,
                    codex_app_server_protocol::TurnPlanStepStatus::Completed
                )
            })
            .count();
        let active_step = plan.iter().find(|step| {
            matches!(
                step.status,
                codex_app_server_protocol::TurnPlanStepStatus::InProgress
            )
        });
        return Some(match active_step {
            Some(step) => format!(
                "Tasks {completed}/{total} - {}",
                compact_overlay_text(&step.step, 48).unwrap_or_else(|| step.step.clone())
            ),
            None => format!("Tasks {completed}/{total}"),
        });
    }

    explanation.and_then(|text| compact_overlay_text(text, 64))
}

fn overlay_request_from_user_inputs(content: &[UserInput]) -> Option<String> {
    let message = content
        .iter()
        .filter_map(|input| match input {
            UserInput::Text { text, .. } => Some(text.as_str()),
            UserInput::Image { .. }
            | UserInput::LocalImage { .. }
            | UserInput::Skill { .. }
            | UserInput::Mention { .. } => None,
        })
        .collect::<Vec<_>>()
        .join("");
    compact_overlay_text(&message, 96)
}

fn compact_overlay_text(text: &str, max_chars: usize) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let truncated: String = trimmed.chars().take(max_chars).collect();
    Some(if trimmed.chars().count() > max_chars {
        format!("{truncated}...")
    } else {
        truncated
    })
}

fn summarize_collab_tool(tool: &CollabAgentTool, status: &CollabAgentToolCallStatus) -> String {
    let label = match tool {
        CollabAgentTool::SpawnAgent => "spawn agent",
        CollabAgentTool::SendInput => "send input",
        CollabAgentTool::ResumeAgent => "resume agent",
        CollabAgentTool::Wait => "wait",
        CollabAgentTool::CloseAgent => "close agent",
    };
    let status = match status {
        CollabAgentToolCallStatus::InProgress => "running",
        CollabAgentToolCallStatus::Completed => "completed",
        CollabAgentToolCallStatus::Failed => "failed",
    };
    format!("{label} ({status})")
}

fn summarize_tool_like_item(item: &ThreadItem) -> Option<String> {
    match item {
        ThreadItem::CommandExecution {
            command, status, ..
        } => {
            let status = match status {
                CommandExecutionStatus::InProgress => "running",
                CommandExecutionStatus::Completed => "completed",
                CommandExecutionStatus::Failed => "failed",
                CommandExecutionStatus::Declined => "declined",
            };
            Some(format!(
                "Command: {} ({status})",
                compact_overlay_text(command, 56).unwrap_or_else(|| command.clone())
            ))
        }
        ThreadItem::FileChange {
            changes, status, ..
        } => {
            let status = match status {
                PatchApplyStatus::InProgress => "running",
                PatchApplyStatus::Completed => "completed",
                PatchApplyStatus::Failed => "failed",
                PatchApplyStatus::Declined => "declined",
            };
            Some(format!("Patch: {} change(s) ({status})", changes.len()))
        }
        ThreadItem::McpToolCall {
            server,
            tool,
            status,
            ..
        } => {
            let status = match status {
                McpToolCallStatus::InProgress => "running",
                McpToolCallStatus::Completed => "completed",
                McpToolCallStatus::Failed => "failed",
            };
            Some(format!("MCP: {server}/{tool} ({status})"))
        }
        ThreadItem::DynamicToolCall { tool, status, .. } => {
            let status = match status {
                DynamicToolCallStatus::InProgress => "running",
                DynamicToolCallStatus::Completed => "completed",
                DynamicToolCallStatus::Failed => "failed",
            };
            Some(format!("Tool: {tool} ({status})"))
        }
        ThreadItem::CollabAgentToolCall { tool, status, .. } => {
            Some(format!("Agent: {}", summarize_collab_tool(tool, status)))
        }
        ThreadItem::WebSearch { query, .. } => Some(format!(
            "Web search: {}",
            compact_overlay_text(query, 52).unwrap_or_else(|| query.clone())
        )),
        ThreadItem::ImageView { path, .. } => Some(format!("Image view: {}", path.display())),
        ThreadItem::ImageGeneration { status, .. } => Some(format!("Image generation: {status}")),
        ThreadItem::EnteredReviewMode { .. } => Some("Review mode entered".to_string()),
        ThreadItem::ExitedReviewMode { .. } => Some("Review mode exited".to_string()),
        ThreadItem::ContextCompaction { .. } => Some("Context compaction".to_string()),
        ThreadItem::HookPrompt { .. }
        | ThreadItem::UserMessage { .. }
        | ThreadItem::AgentMessage { .. }
        | ThreadItem::Plan { .. }
        | ThreadItem::Reasoning { .. } => None,
    }
}

fn schedule_agents_overlay_animation(
    tui: &mut tui::Tui,
    overlay: Option<&Overlay>,
    animations_enabled: bool,
) {
    if animations_enabled
        && overlay.is_some_and(|overlay| match overlay {
            Overlay::Agents(overlay) => overlay.has_active_agents(),
            Overlay::Transcript(_) | Overlay::Static(_) => false,
        })
    {
        tui.frame_requester()
            .schedule_frame_in(tui::TARGET_FRAME_INTERVAL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_app_server_protocol::TurnPlanStep;
    use codex_app_server_protocol::TurnPlanStepStatus;
    use pretty_assertions::assert_eq;

    #[test]
    fn summarize_plan_steps_uses_progress_and_active_step() {
        let plan = vec![
            TurnPlanStep {
                step: "Done".to_string(),
                status: TurnPlanStepStatus::Completed,
            },
            TurnPlanStep {
                step: "Implement AGENTS overlay menu".to_string(),
                status: TurnPlanStepStatus::InProgress,
            },
            TurnPlanStep {
                step: "Verify".to_string(),
                status: TurnPlanStepStatus::Pending,
            },
        ];

        assert_eq!(
            summarize_plan_steps(&plan, Some("ignored")),
            Some("Tasks 1/3 - Implement AGENTS overlay menu".to_string())
        );
    }

    #[test]
    fn summarize_plan_steps_omits_empty_plan_without_explanation() {
        assert_eq!(summarize_plan_steps(&[], None), None);
        assert_eq!(
            summarize_plan_steps(&[], Some("Investigating overlay behavior")),
            Some("Investigating overlay behavior".to_string())
        );
    }
}
