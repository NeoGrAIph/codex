//! Session, resume, fork, and subagent selection lifecycle for the TUI app.
//!
//! This module owns the high-level transitions between app-server threads: starting fresh sessions,
//! resuming/forking saved sessions, replacing ChatWidget instances, and maintaining the agent picker
//! cache used for multi-agent navigation.

use super::*;

impl App {
    pub(super) async fn open_agent_picker(&mut self, app_server: &mut AppServerSession) {
        self.backfill_loaded_subagent_threads(app_server).await;
        // V2 subagents are identified by canonical paths observed from activity events or loaded
        // thread metadata. Prefer local buffered turn state for liveness, and fall back to
        // thread/read only when no local event channel exists.
        let path_backed_thread_ids: Vec<_> = self
            .agent_navigation
            .ordered_path_backed_subagent_threads(self.primary_thread_id)
            .into_iter()
            .map(|(thread_id, _)| thread_id)
            .collect();
        for thread_id in path_backed_thread_ids {
            if let Some(channel) = self.thread_event_channels.get(&thread_id)
                && channel.attachment() == ThreadEventAttachment::Live
            {
                let is_running = channel.store.lock().await.active_turn_id().is_some();
                self.agent_navigation.set_running(thread_id, is_running);
            } else {
                self.refresh_agent_picker_thread_liveness(app_server, thread_id)
                    .await;
            }
        }
        let path_backed_thread_ids: HashSet<_> = self
            .agent_navigation
            .ordered_path_backed_subagent_threads(self.primary_thread_id)
            .into_iter()
            .map(|(thread_id, _)| thread_id)
            .collect();

        let mut thread_ids = self.agent_navigation.tracked_thread_ids();
        for thread_id in self.thread_event_channels.keys().copied() {
            if !thread_ids.contains(&thread_id) {
                thread_ids.push(thread_id);
            }
        }
        for thread_id in thread_ids {
            if path_backed_thread_ids.contains(&thread_id) {
                continue;
            }
            if self.side_threads.contains_key(&thread_id) {
                continue;
            }
            if !self
                .refresh_agent_picker_thread_liveness(app_server, thread_id)
                .await
            {
                continue;
            }
        }

        let has_non_primary_agent_thread = self
            .agent_navigation
            .has_non_primary_thread(self.primary_thread_id);
        if !self.config.features.enabled(Feature::Collab) && !has_non_primary_agent_thread {
            self.chat_widget.open_multi_agent_enable_prompt();
            return;
        }

        if self.agent_navigation.is_empty() {
            self.chat_widget
                .add_info_message("No agents available yet.".to_string(), /*hint*/ None);
            return;
        }

        let summary_line = {
            let ordered_threads = self.agent_navigation.ordered_threads();
            agent_picker_summary_line(&ordered_threads, self.primary_thread_id)
        };
        let ordered_thread_ids = self
            .agent_navigation
            .ordered_threads()
            .into_iter()
            .map(|(thread_id, _)| thread_id)
            .collect::<Vec<_>>();
        for thread_id in &ordered_thread_ids {
            self.hydrate_agent_picker_session_detail(*thread_id).await;
        }
        let mut recent_activity_by_thread_id = HashMap::new();
        let mut prompt_context_by_thread_id = HashMap::new();
        let mut token_usage_by_thread_id = HashMap::new();
        let mut plan_progress_by_thread_id = HashMap::new();
        for thread_id in ordered_thread_ids {
            let prompt_context = self.agent_picker_prompt_context(thread_id).await;
            if !prompt_context.is_empty() {
                prompt_context_by_thread_id.insert(thread_id, prompt_context);
            }
            let recent_activity = self.agent_picker_recent_activity(thread_id).await;
            if !recent_activity.is_empty() {
                recent_activity_by_thread_id.insert(thread_id, recent_activity);
            }
            if let Some(token_usage) = self.agent_picker_token_usage_summary(thread_id).await {
                token_usage_by_thread_id.insert(thread_id, token_usage);
            }
            if let Some(plan_progress) = self.agent_picker_plan_progress_summary(thread_id).await {
                plan_progress_by_thread_id.insert(thread_id, plan_progress);
            }
        }

        let mut initial_selected_idx = None;
        let items: Vec<SelectionItem> = self
            .agent_navigation
            .ordered_threads()
            .into_iter()
            .enumerate()
            .map(|(idx, (thread_id, entry))| {
                if self.active_thread_id == Some(thread_id) {
                    initial_selected_idx = Some(idx);
                }
                let id = thread_id;
                let is_primary = self.primary_thread_id == Some(thread_id);
                let name = format_agent_picker_item_name(
                    entry.agent_nickname.as_deref(),
                    entry.agent_role.as_deref(),
                    is_primary,
                );
                let uuid = thread_id.to_string();
                let is_current = self.active_thread_id == Some(thread_id);
                let recent_activity = recent_activity_by_thread_id
                    .get(&thread_id)
                    .map(Vec::as_slice)
                    .unwrap_or_default();
                let prompt_context = prompt_context_by_thread_id
                    .get(&thread_id)
                    .map(Vec::as_slice)
                    .unwrap_or_default();
                let token_usage_summary =
                    token_usage_by_thread_id.get(&thread_id).map(String::as_str);
                let plan_progress_summary = plan_progress_by_thread_id
                    .get(&thread_id)
                    .map(String::as_str);
                SelectionItem {
                    name: name.clone(),
                    name_prefix_spans: agent_picker_status_dot_spans(entry.status),
                    description: Some(agent_picker_item_description(thread_id, entry, is_primary)),
                    selected_description: Some(agent_picker_selected_description(
                        thread_id,
                        entry,
                        is_primary,
                        is_current,
                        AgentPickerSelectedDescriptionContext {
                            prompt_context,
                            recent_activity,
                            token_usage_summary,
                            plan_progress_summary,
                        },
                    )),
                    is_current,
                    actions: vec![Box::new(move |tx| {
                        tx.send(AppEvent::SelectAgentThread(id));
                    })],
                    dismiss_on_select: true,
                    search_value: Some(format!("{name} {uuid}")),
                    ..Default::default()
                }
            })
            .collect();
        let mut items = items;
        let root_owned_workbench = self.current_agent_picker_owner_path() == "/root";
        if root_owned_workbench {
            for (thread_id, entry) in self.agent_navigation.ordered_threads() {
                if self.agent_picker_message_preflight(thread_id).is_err() {
                    continue;
                }
                let name = format!(
                    "Send message {}",
                    format_agent_picker_item_name(
                        entry.agent_nickname.as_deref(),
                        entry.agent_role.as_deref(),
                        /*is_primary*/ false,
                    )
                );
                let agent_path = entry.agent_path.as_deref().unwrap_or_default().to_string();
                items.push(SelectionItem {
                    name: name.clone(),
                    name_prefix_spans: vec!["> ".cyan()],
                    description: Some(format!(
                        "{} · {} · queue message only",
                        entry.status.label(),
                        agent_path
                    )),
                    actions: vec![Box::new(move |tx| {
                        tx.send(AppEvent::OpenAgentMessagePrompt { thread_id });
                    })],
                    dismiss_on_select: true,
                    search_value: Some(format!("{name} {thread_id} {agent_path} message")),
                    ..Default::default()
                });
            }
        }
        for (thread_id, entry) in self.agent_navigation.ordered_threads() {
            if self.agent_picker_followup_preflight(thread_id).is_err() {
                continue;
            }
            let name = format!(
                "Follow up {}",
                format_agent_picker_item_name(
                    entry.agent_nickname.as_deref(),
                    entry.agent_role.as_deref(),
                    /*is_primary*/ false,
                )
            );
            let agent_path = entry.agent_path.as_deref().unwrap_or_default().to_string();
            items.push(SelectionItem {
                name: name.clone(),
                name_prefix_spans: vec!["> ".green()],
                description: Some(format!(
                    "{} · {} · starts a turn",
                    entry.status.label(),
                    agent_path
                )),
                actions: vec![Box::new(move |tx| {
                    tx.send(AppEvent::OpenAgentFollowupPrompt { thread_id });
                })],
                dismiss_on_select: true,
                search_value: Some(format!("{name} {thread_id} {agent_path} follow up task")),
                ..Default::default()
            });
        }
        for (thread_id, entry) in self.agent_navigation.ordered_threads() {
            if self.agent_picker_interrupt_preflight(thread_id).is_err() {
                continue;
            }
            let name = format!(
                "Interrupt {}",
                format_agent_picker_item_name(
                    entry.agent_nickname.as_deref(),
                    entry.agent_role.as_deref(),
                    /*is_primary*/ false,
                )
            );
            let agent_path = entry.agent_path.as_deref().unwrap_or_default().to_string();
            items.push(SelectionItem {
                name: name.clone(),
                name_prefix_spans: vec!["! ".red()],
                description: Some(format!(
                    "{} · {} · stop current turn only",
                    entry.status.label(),
                    agent_path
                )),
                actions: vec![Box::new(move |tx| {
                    tx.send(AppEvent::OpenAgentInterruptConfirmation { thread_id });
                })],
                dismiss_on_select: true,
                search_value: Some(format!("{name} {thread_id} {agent_path} interrupt")),
                ..Default::default()
            });
        }
        for (thread_id, entry) in self.agent_navigation.ordered_threads() {
            if self.agent_picker_close_preflight(thread_id).is_err() {
                continue;
            }
            let name = format!(
                "Close {}",
                format_agent_picker_item_name(
                    entry.agent_nickname.as_deref(),
                    entry.agent_role.as_deref(),
                    /*is_primary*/ false,
                )
            );
            let agent_path = entry.agent_path.as_deref().unwrap_or_default().to_string();
            items.push(SelectionItem {
                name: name.clone(),
                name_prefix_spans: vec!["x ".red()],
                description: Some(format!(
                    "{} · {} · close agent subtree",
                    entry.status.label(),
                    agent_path
                )),
                actions: vec![Box::new(move |tx| {
                    tx.send(AppEvent::OpenAgentCloseConfirmation { thread_id });
                })],
                dismiss_on_select: true,
                search_value: Some(format!("{name} {thread_id} {agent_path} close")),
                ..Default::default()
            });
        }
        if !root_owned_workbench {
            for (thread_id, entry) in self.agent_navigation.ordered_threads() {
                if self.agent_picker_message_preflight(thread_id).is_err() {
                    continue;
                }
                let name = format!(
                    "Send message {}",
                    format_agent_picker_item_name(
                        entry.agent_nickname.as_deref(),
                        entry.agent_role.as_deref(),
                        /*is_primary*/ false,
                    )
                );
                let agent_path = entry.agent_path.as_deref().unwrap_or_default().to_string();
                items.push(SelectionItem {
                    name: name.clone(),
                    name_prefix_spans: vec!["> ".cyan()],
                    description: Some(format!(
                        "{} · {} · queue message only",
                        entry.status.label(),
                        agent_path
                    )),
                    actions: vec![Box::new(move |tx| {
                        tx.send(AppEvent::OpenAgentMessagePrompt { thread_id });
                    })],
                    dismiss_on_select: true,
                    search_value: Some(format!("{name} {thread_id} {agent_path} message")),
                    ..Default::default()
                });
            }
        }

        let mut header = ColumnRenderable::new();
        header.push(Line::from("Subagents".bold()));
        header.push(Line::from(AgentNavigationState::picker_subtitle().dim()));
        header.push(Line::from(summary_line.dim()));

        self.chat_widget.show_selection_view(SelectionViewParams {
            header: Box::new(header),
            footer_hint: Some(standard_popup_hint_line()),
            items,
            initial_selected_idx,
            ..Default::default()
        });
    }

    pub(super) fn open_agent_message_prompt(&mut self, thread_id: ThreadId) {
        if let Err(reason) = self.agent_picker_message_preflight(thread_id) {
            self.chat_widget.add_error_message(reason);
            return;
        }
        let Some(entry) = self.agent_navigation.get(&thread_id) else {
            self.chat_widget.add_error_message(format!(
                "Cannot message agent thread {thread_id}: it is no longer tracked."
            ));
            return;
        };
        let name = format_agent_picker_item_name(
            entry.agent_nickname.as_deref(),
            entry.agent_role.as_deref(),
            /*is_primary*/ false,
        );
        let agent_path = entry.agent_path.as_deref().unwrap_or_default();
        self.chat_widget
            .open_agent_message_prompt(thread_id, format!("{name} · {agent_path}"));
    }

    pub(super) fn open_agent_followup_prompt(&mut self, thread_id: ThreadId) {
        if let Err(reason) = self.agent_picker_followup_preflight(thread_id) {
            self.chat_widget.add_error_message(reason);
            return;
        }
        let Some(entry) = self.agent_navigation.get(&thread_id) else {
            self.chat_widget.add_error_message(format!(
                "Cannot follow up agent thread {thread_id}: it is no longer tracked."
            ));
            return;
        };
        let name = format_agent_picker_item_name(
            entry.agent_nickname.as_deref(),
            entry.agent_role.as_deref(),
            /*is_primary*/ false,
        );
        let agent_path = entry.agent_path.as_deref().unwrap_or_default();
        self.chat_widget
            .open_agent_followup_prompt(thread_id, format!("{name} · {agent_path}"));
    }

    pub(super) fn open_agent_followup_confirmation(
        &mut self,
        thread_id: ThreadId,
        message: String,
    ) {
        if message.trim().is_empty() {
            self.chat_widget
                .add_error_message("Cannot send an empty agent follow-up.".to_string());
            return;
        }
        if let Err(reason) = self.agent_picker_followup_preflight(thread_id) {
            self.chat_widget.add_error_message(reason);
            return;
        }
        let Some(entry) = self.agent_navigation.get(&thread_id) else {
            self.chat_widget.add_error_message(format!(
                "Cannot follow up agent thread {thread_id}: it is no longer tracked."
            ));
            return;
        };
        let name = format_agent_picker_item_name(
            entry.agent_nickname.as_deref(),
            entry.agent_role.as_deref(),
            /*is_primary*/ false,
        );
        let agent_path = entry.agent_path.as_deref().unwrap_or_default().to_string();
        let items = vec![
            SelectionItem {
                name: "Cancel".to_string(),
                description: Some("Do not start a follow-up turn.".to_string()),
                dismiss_on_select: true,
                ..Default::default()
            },
            SelectionItem {
                name: "Start follow-up".to_string(),
                name_prefix_spans: vec!["> ".green()],
                description: Some(
                    "Queue an encrypted follow-up and start this agent when capacity is available."
                        .to_string(),
                ),
                actions: vec![Box::new(move |tx| {
                    tx.send(AppEvent::FollowupAgentThreadConfirmed {
                        thread_id,
                        message: message.clone(),
                    });
                })],
                dismiss_on_select: true,
                ..Default::default()
            },
        ];

        self.chat_widget.show_selection_view(SelectionViewParams {
            title: Some("Start a follow-up turn?".to_string()),
            subtitle: Some(format!("{name} · {agent_path}")),
            footer_hint: Some(standard_popup_hint_line()),
            items,
            initial_selected_idx: Some(0),
            ..Default::default()
        });
    }

    pub(super) async fn send_agent_message(
        &mut self,
        app_server: &mut AppServerSession,
        thread_id: ThreadId,
        message: String,
    ) -> Result<()> {
        if message.trim().is_empty() {
            self.chat_widget
                .add_error_message("Cannot send an empty agent message.".to_string());
            return Ok(());
        }
        if let Err(reason) = self.agent_picker_message_preflight(thread_id) {
            self.chat_widget.add_error_message(reason);
            return Ok(());
        }
        let Some(author_thread_id) = self.agent_picker_action_author_thread_id() else {
            self.chat_widget.add_error_message(
                "Cannot send an agent message before the workbench thread is ready.".to_string(),
            );
            return Ok(());
        };
        let target_name = self
            .agent_navigation
            .get(&thread_id)
            .map(|entry| {
                format_agent_picker_item_name(
                    entry.agent_nickname.as_deref(),
                    entry.agent_role.as_deref(),
                    /*is_primary*/ false,
                )
            })
            .unwrap_or_else(|| thread_id.to_string());
        app_server
            .agent_message_send(author_thread_id, thread_id, message)
            .await?;
        self.refresh_agent_picker_thread_liveness(app_server, thread_id)
            .await;
        self.chat_widget.add_info_message(
            format!("Message queued for {target_name}."),
            /*hint*/ None,
        );
        Ok(())
    }

    pub(super) async fn followup_agent_thread_confirmed(
        &mut self,
        app_server: &mut AppServerSession,
        thread_id: ThreadId,
        message: String,
    ) -> Result<()> {
        if message.trim().is_empty() {
            self.chat_widget
                .add_error_message("Cannot send an empty agent follow-up.".to_string());
            return Ok(());
        }
        if let Err(reason) = self.agent_picker_followup_preflight(thread_id) {
            self.chat_widget.add_error_message(reason);
            return Ok(());
        }
        let Some(author_thread_id) = self.agent_picker_action_author_thread_id() else {
            self.chat_widget.add_error_message(
                "Cannot send an agent follow-up before the main thread is ready.".to_string(),
            );
            return Ok(());
        };
        let target_name = self
            .agent_navigation
            .get(&thread_id)
            .map(|entry| {
                format_agent_picker_item_name(
                    entry.agent_nickname.as_deref(),
                    entry.agent_role.as_deref(),
                    /*is_primary*/ false,
                )
            })
            .unwrap_or_else(|| thread_id.to_string());
        app_server
            .agent_followup_send(author_thread_id, thread_id, message)
            .await?;
        self.refresh_agent_picker_thread_liveness(app_server, thread_id)
            .await;
        self.chat_widget.add_info_message(
            format!("Follow-up started for {target_name}."),
            Some("The agent may request approvals under its existing sandbox.".to_string()),
        );
        Ok(())
    }

    pub(super) fn agent_picker_message_preflight(&self, thread_id: ThreadId) -> Result<(), String> {
        if self.primary_thread_id.is_none() {
            return Err(
                "Cannot send an agent message before the main thread is ready.".to_string(),
            );
        }
        if self.primary_thread_id == Some(thread_id) {
            return Err("Cannot send an agent message to the main thread.".to_string());
        }
        let Some(entry) = self.agent_navigation.get(&thread_id) else {
            return Err(format!(
                "Cannot message agent thread {thread_id}: it is no longer tracked."
            ));
        };
        match entry.status {
            crate::multi_agents::AgentPickerThreadStatus::Idle
            | crate::multi_agents::AgentPickerThreadStatus::Running
            | crate::multi_agents::AgentPickerThreadStatus::WaitingApproval
            | crate::multi_agents::AgentPickerThreadStatus::WaitingUser => {}
            crate::multi_agents::AgentPickerThreadStatus::Error => {
                return Err(
                    "Cannot message an agent thread that is already in error state.".to_string(),
                );
            }
            crate::multi_agents::AgentPickerThreadStatus::Closed => {
                return Err("Cannot message a closed agent thread.".to_string());
            }
        }
        let target_agent_path = entry
            .agent_path
            .as_deref()
            .map(str::trim)
            .filter(|agent_path| !agent_path.is_empty())
            .ok_or_else(|| "Cannot message an agent thread without agent_path.".to_string())?;
        if target_agent_path == "/root" {
            return Err("Cannot send an agent message to the root agent.".to_string());
        }
        let current_agent_path =
            self.current_agent_picker_path_backed_owner_path("Cannot send an agent message")?;
        if target_agent_path == current_agent_path {
            return Err("Cannot send an agent message to the current thread.".to_string());
        }
        if agent_path_can_interrupt(current_agent_path, target_agent_path) {
            Ok(())
        } else {
            Err(format!(
                "Agent `{current_agent_path}` cannot message `{target_agent_path}` because the target is outside its sub-agent tree."
            ))
        }
    }

    pub(super) fn agent_picker_followup_preflight(
        &self,
        thread_id: ThreadId,
    ) -> Result<(), String> {
        if self.primary_thread_id.is_none() {
            return Err(
                "Cannot send an agent follow-up before the main thread is ready.".to_string(),
            );
        }
        if self.primary_thread_id == Some(thread_id) {
            return Err("Cannot send an agent follow-up to the main thread.".to_string());
        }
        let Some(entry) = self.agent_navigation.get(&thread_id) else {
            return Err(format!(
                "Cannot follow up agent thread {thread_id}: it is no longer tracked."
            ));
        };
        match entry.status {
            crate::multi_agents::AgentPickerThreadStatus::Idle => {}
            crate::multi_agents::AgentPickerThreadStatus::Running
            | crate::multi_agents::AgentPickerThreadStatus::WaitingApproval
            | crate::multi_agents::AgentPickerThreadStatus::WaitingUser => {
                return Err(
                    "Cannot start a follow-up for an agent thread that is already running or waiting."
                        .to_string(),
                );
            }
            crate::multi_agents::AgentPickerThreadStatus::Error => {
                return Err(
                    "Cannot follow up an agent thread that is already in error state.".to_string(),
                );
            }
            crate::multi_agents::AgentPickerThreadStatus::Closed => {
                return Err("Cannot follow up a closed agent thread.".to_string());
            }
        }
        let target_agent_path = entry
            .agent_path
            .as_deref()
            .map(str::trim)
            .filter(|agent_path| !agent_path.is_empty())
            .ok_or_else(|| "Cannot follow up an agent thread without agent_path.".to_string())?;
        if target_agent_path == "/root" {
            return Err("Cannot send an agent follow-up to the root agent.".to_string());
        }
        let current_agent_path =
            self.current_agent_picker_path_backed_owner_path("Cannot send an agent follow-up")?;
        if target_agent_path == current_agent_path {
            return Err("An agent cannot follow up itself from the agent workbench.".to_string());
        }
        if agent_path_can_interrupt(current_agent_path, target_agent_path) {
            Ok(())
        } else {
            Err(format!(
                "Agent `{current_agent_path}` cannot follow up `{target_agent_path}` because the target is outside its sub-agent tree."
            ))
        }
    }

    pub(super) fn open_agent_interrupt_confirmation(&mut self, thread_id: ThreadId) {
        if let Err(reason) = self.agent_picker_interrupt_preflight(thread_id) {
            self.chat_widget.add_error_message(reason);
            return;
        }
        let Some(entry) = self.agent_navigation.get(&thread_id) else {
            self.chat_widget.add_error_message(format!(
                "Cannot interrupt agent thread {thread_id}: it is no longer tracked."
            ));
            return;
        };
        let name = format_agent_picker_item_name(
            entry.agent_nickname.as_deref(),
            entry.agent_role.as_deref(),
            /*is_primary*/ false,
        );
        let agent_path = entry.agent_path.as_deref().unwrap_or_default().to_string();
        let previous_status = entry.status.label();
        let items = vec![
            SelectionItem {
                name: "Cancel".to_string(),
                description: Some("Leave the agent turn running.".to_string()),
                dismiss_on_select: true,
                ..Default::default()
            },
            SelectionItem {
                name: "Interrupt agent".to_string(),
                name_prefix_spans: vec!["! ".red()],
                description: Some(format!(
                    "Current status: {previous_status}. Stop the current turn only. It does not close or delete the thread."
                )),
                actions: vec![Box::new(move |tx| {
                    tx.send(AppEvent::InterruptAgentThreadConfirmed { thread_id });
                })],
                dismiss_on_select: true,
                ..Default::default()
            },
        ];

        self.chat_widget.show_selection_view(SelectionViewParams {
            title: Some("Interrupt this agent?".to_string()),
            subtitle: Some(format!(
                "{name} · {agent_path} · current status: {previous_status}"
            )),
            footer_hint: Some(standard_popup_hint_line()),
            items,
            initial_selected_idx: Some(0),
            ..Default::default()
        });
    }

    pub(super) async fn interrupt_agent_thread_confirmed(
        &mut self,
        app_server: &mut AppServerSession,
        thread_id: ThreadId,
    ) -> Result<()> {
        match self.agent_picker_interrupt_preflight(thread_id) {
            Ok(()) => {
                let (target_name, previous_status) = self
                    .agent_navigation
                    .get(&thread_id)
                    .map(|entry| {
                        (
                            format_agent_picker_item_name(
                                entry.agent_nickname.as_deref(),
                                entry.agent_role.as_deref(),
                                /*is_primary*/ false,
                            ),
                            entry.status.label().to_string(),
                        )
                    })
                    .unwrap_or_else(|| (thread_id.to_string(), "unknown".to_string()));
                self.submit_thread_op(app_server, thread_id, AppCommand::interrupt())
                    .await?;
                self.refresh_agent_picker_thread_liveness(app_server, thread_id)
                    .await;
                self.chat_widget.add_info_message(
                    format!("Interrupt requested for {target_name} (previous status: {previous_status})."),
                    /*hint*/ None,
                );
            }
            Err(reason) => self.chat_widget.add_error_message(reason),
        }
        Ok(())
    }

    pub(super) fn open_agent_close_confirmation(&mut self, thread_id: ThreadId) {
        if let Err(reason) = self.agent_picker_close_preflight(thread_id) {
            self.chat_widget.add_error_message(reason);
            return;
        }
        let Some(entry) = self.agent_navigation.get(&thread_id) else {
            self.chat_widget.add_error_message(format!(
                "Cannot close agent thread {thread_id}: it is no longer tracked."
            ));
            return;
        };
        let name = format_agent_picker_item_name(
            entry.agent_nickname.as_deref(),
            entry.agent_role.as_deref(),
            /*is_primary*/ false,
        );
        let agent_path = entry.agent_path.as_deref().unwrap_or_default().to_string();
        let previous_status = entry.status.label();
        let items = vec![
            SelectionItem {
                name: "Cancel".to_string(),
                description: Some("Keep the agent available.".to_string()),
                dismiss_on_select: true,
                ..Default::default()
            },
            SelectionItem {
                name: "Close agent".to_string(),
                name_prefix_spans: vec!["x ".red()],
                description: Some(format!(
                    "Current status: {previous_status}. Mark this spawn edge closed and shut down the live target and descendants."
                )),
                actions: vec![Box::new(move |tx| {
                    tx.send(AppEvent::CloseAgentThreadConfirmed { thread_id });
                })],
                dismiss_on_select: true,
                ..Default::default()
            },
        ];

        self.chat_widget.show_selection_view(SelectionViewParams {
            title: Some("Close this agent?".to_string()),
            subtitle: Some(format!(
                "{name} · {agent_path} · current status: {previous_status}"
            )),
            footer_hint: Some(standard_popup_hint_line()),
            items,
            initial_selected_idx: Some(0),
            ..Default::default()
        });
    }

    pub(super) async fn close_agent_thread_confirmed(
        &mut self,
        app_server: &mut AppServerSession,
        thread_id: ThreadId,
    ) -> Result<()> {
        match self.agent_picker_close_preflight(thread_id) {
            Ok(()) => {
                let Some(author_thread_id) = self.agent_picker_action_author_thread_id() else {
                    self.chat_widget.add_error_message(
                        "Cannot close an agent before the workbench thread is ready.".to_string(),
                    );
                    return Ok(());
                };
                let target_name = self
                    .agent_navigation
                    .get(&thread_id)
                    .map(|entry| {
                        format_agent_picker_item_name(
                            entry.agent_nickname.as_deref(),
                            entry.agent_role.as_deref(),
                            /*is_primary*/ false,
                        )
                    })
                    .unwrap_or_else(|| thread_id.to_string());
                let response = app_server.agent_close(author_thread_id, thread_id).await?;
                self.mark_agent_picker_thread_closed(thread_id);
                self.refresh_agent_picker_thread_liveness(app_server, thread_id)
                    .await;
                self.chat_widget.add_info_message(
                    format!(
                        "Closed {target_name} (previous status: {}).",
                        collab_agent_state_status_label(&response.previous_status)
                    ),
                    Some(
                        "The thread remains available for replay and native resume paths."
                            .to_string(),
                    ),
                );
            }
            Err(reason) => self.chat_widget.add_error_message(reason),
        }
        Ok(())
    }

    pub(super) fn agent_picker_interrupt_preflight(
        &self,
        thread_id: ThreadId,
    ) -> Result<(), String> {
        if self.primary_thread_id == Some(thread_id) {
            return Err("Cannot interrupt the main thread from the agent workbench.".to_string());
        }
        let Some(entry) = self.agent_navigation.get(&thread_id) else {
            return Err(format!(
                "Cannot interrupt agent thread {thread_id}: it is no longer tracked."
            ));
        };
        match entry.status {
            crate::multi_agents::AgentPickerThreadStatus::Running
            | crate::multi_agents::AgentPickerThreadStatus::WaitingApproval
            | crate::multi_agents::AgentPickerThreadStatus::WaitingUser => {}
            crate::multi_agents::AgentPickerThreadStatus::Idle => {
                return Err("Cannot interrupt an idle agent thread.".to_string());
            }
            crate::multi_agents::AgentPickerThreadStatus::Error => {
                return Err(
                    "Cannot interrupt an agent thread that is already in error state.".to_string(),
                );
            }
            crate::multi_agents::AgentPickerThreadStatus::Closed => {
                return Err("Cannot interrupt a closed agent thread.".to_string());
            }
        }
        let target_agent_path = entry
            .agent_path
            .as_deref()
            .map(str::trim)
            .filter(|agent_path| !agent_path.is_empty())
            .ok_or_else(|| "Cannot interrupt an agent thread without agent_path.".to_string())?;
        if target_agent_path == "/root" {
            return Err("Cannot interrupt the root agent from the agent workbench.".to_string());
        }
        let current_agent_path = self
            .active_thread_id
            .filter(|active_thread_id| self.primary_thread_id != Some(*active_thread_id))
            .and_then(|active_thread_id| self.agent_navigation.get(&active_thread_id))
            .and_then(|entry| entry.agent_path.as_deref())
            .map(str::trim)
            .filter(|agent_path| !agent_path.is_empty())
            .unwrap_or("/root");
        if target_agent_path == current_agent_path {
            return Err("An agent cannot interrupt itself from the agent workbench.".to_string());
        }
        if agent_path_can_interrupt(current_agent_path, target_agent_path) {
            Ok(())
        } else {
            Err(format!(
                "Agent `{current_agent_path}` cannot interrupt `{target_agent_path}` because the target is outside its sub-agent tree."
            ))
        }
    }

    pub(super) fn agent_picker_close_preflight(&self, thread_id: ThreadId) -> Result<(), String> {
        if self.primary_thread_id == Some(thread_id) {
            return Err("Cannot close the main thread from the agent workbench.".to_string());
        }
        let Some(entry) = self.agent_navigation.get(&thread_id) else {
            return Err(format!(
                "Cannot close agent thread {thread_id}: it is no longer tracked."
            ));
        };
        if entry.status.is_closed() {
            return Err("Cannot close an already closed agent thread.".to_string());
        }
        let target_agent_path = entry
            .agent_path
            .as_deref()
            .map(str::trim)
            .filter(|agent_path| !agent_path.is_empty())
            .ok_or_else(|| "Cannot close an agent thread without agent_path.".to_string())?;
        if target_agent_path == "/root" {
            return Err("Cannot close the root agent from the agent workbench.".to_string());
        }
        let current_agent_path = self.current_agent_picker_owner_path();
        if target_agent_path == current_agent_path {
            return Err("An agent cannot close itself from the agent workbench.".to_string());
        }
        if agent_path_can_interrupt(current_agent_path, target_agent_path) {
            Ok(())
        } else {
            Err(format!(
                "Agent `{current_agent_path}` cannot close `{target_agent_path}` because the target is outside its sub-agent tree."
            ))
        }
    }

    fn current_agent_picker_owner_path(&self) -> &str {
        self.active_thread_id
            .filter(|active_thread_id| self.primary_thread_id != Some(*active_thread_id))
            .and_then(|active_thread_id| self.agent_navigation.get(&active_thread_id))
            .and_then(|entry| entry.agent_path.as_deref())
            .map(str::trim)
            .filter(|agent_path| !agent_path.is_empty())
            .unwrap_or("/root")
    }

    fn current_agent_picker_path_backed_owner_path(
        &self,
        error_prefix: &str,
    ) -> Result<&str, String> {
        let Some(active_thread_id) = self
            .active_thread_id
            .filter(|active_thread_id| self.primary_thread_id != Some(*active_thread_id))
        else {
            return Ok("/root");
        };
        let Some(entry) = self.agent_navigation.get(&active_thread_id) else {
            return Err(format!(
                "{error_prefix} from thread {active_thread_id}: it is no longer tracked."
            ));
        };
        entry
            .agent_path
            .as_deref()
            .map(str::trim)
            .filter(|agent_path| !agent_path.is_empty())
            .ok_or_else(|| format!("{error_prefix} from an agent thread without agent_path."))
    }

    pub(super) fn agent_picker_action_author_thread_id(&self) -> Option<ThreadId> {
        self.active_thread_id
            .filter(|active_thread_id| self.primary_thread_id != Some(*active_thread_id))
            .or(self.primary_thread_id)
    }

    async fn agent_picker_recent_activity(&self, thread_id: ThreadId) -> Vec<String> {
        let Some(channel) = self.thread_event_channels.get(&thread_id) else {
            return Vec::new();
        };
        let store = channel.store.lock().await;
        let items = store
            .buffer
            .iter()
            .rev()
            .filter_map(thread_item_for_agent_picker_activity);
        crate::multi_agents::agent_picker_recent_activity_summaries(items)
    }

    async fn agent_picker_prompt_context(&self, thread_id: ThreadId) -> Vec<String> {
        let stores = self
            .thread_event_channels
            .values()
            .map(|channel| channel.store.clone())
            .collect::<Vec<_>>();
        let mut context = Vec::new();
        for store in stores {
            let store = store.lock().await;
            let items = store
                .buffer
                .iter()
                .rev()
                .filter_map(thread_item_for_agent_picker_activity);
            context.extend(crate::multi_agents::agent_picker_prompt_context_summaries(
                thread_id, items,
            ));
        }
        context.sort();
        context.dedup();
        context
    }

    async fn agent_picker_token_usage_summary(&self, thread_id: ThreadId) -> Option<String> {
        let channel = self.thread_event_channels.get(&thread_id)?;
        let thread_id = thread_id.to_string();
        let store = channel.store.lock().await;
        store.buffer.iter().rev().find_map(|event| match event {
            ThreadBufferedEvent::Notification(ServerNotification::ThreadTokenUsageUpdated(
                notification,
            )) if notification.thread_id == thread_id => {
                crate::multi_agents::agent_picker_token_usage_summary(&notification.token_usage)
            }
            ThreadBufferedEvent::Notification(_)
            | ThreadBufferedEvent::Request(_)
            | ThreadBufferedEvent::HistoryEntryResponse(_)
            | ThreadBufferedEvent::FeedbackSubmission(_) => None,
        })
    }

    async fn agent_picker_plan_progress_summary(&self, thread_id: ThreadId) -> Option<String> {
        let channel = self.thread_event_channels.get(&thread_id)?;
        let thread_id = thread_id.to_string();
        let store = channel.store.lock().await;
        store.buffer.iter().rev().find_map(|event| match event {
            ThreadBufferedEvent::Notification(ServerNotification::TurnPlanUpdated(
                notification,
            )) if notification.thread_id == thread_id => {
                crate::multi_agents::agent_picker_plan_progress_summary(&notification.plan)
            }
            ThreadBufferedEvent::Notification(_)
            | ThreadBufferedEvent::Request(_)
            | ThreadBufferedEvent::HistoryEntryResponse(_)
            | ThreadBufferedEvent::FeedbackSubmission(_) => None,
        })
    }

    async fn hydrate_agent_picker_session_detail(&mut self, thread_id: ThreadId) {
        let Some(channel) = self.thread_event_channels.get(&thread_id) else {
            return;
        };
        let session = channel.store.lock().await.session.clone();
        let Some(session) = session else {
            return;
        };
        self.agent_navigation.update_thread_session_detail(
            thread_id,
            Some(session.model_provider_id),
            Some(session.model),
            session
                .reasoning_effort
                .map(|reasoning_effort| reasoning_effort.to_string()),
            session.service_tier,
        );
    }

    pub(super) fn update_agent_picker_thread_session_detail(
        &mut self,
        thread_id: ThreadId,
        session: &ThreadSessionState,
    ) {
        self.agent_navigation.update_thread_session_detail(
            thread_id,
            Some(session.model_provider_id.clone()),
            Some(session.model.clone()),
            session.reasoning_effort.as_ref().map(ToString::to_string),
            session.service_tier.clone(),
        );
    }

    pub(super) fn is_terminal_thread_read_error(err: &color_eyre::Report) -> bool {
        err.chain()
            .any(|cause| cause.to_string().contains("thread not loaded:"))
    }

    pub(super) fn closed_state_for_thread_read_error(
        err: &color_eyre::Report,
        existing_is_closed: Option<bool>,
    ) -> bool {
        Self::is_terminal_thread_read_error(err) || existing_is_closed.unwrap_or(false)
    }

    pub(super) fn can_fallback_from_include_turns_error(err: &color_eyre::Report) -> bool {
        err.chain().any(|cause| {
            let message = cause.to_string();
            message.contains("includeTurns is unavailable before first user message")
                || message.contains("ephemeral threads do not support includeTurns")
        })
    }

    /// Updates cached picker metadata and then mirrors any visible-label change into the footer.
    ///
    /// These two writes stay paired so the picker rows and contextual footer continue to describe
    /// the same displayed thread after nickname or role updates.
    pub(super) fn upsert_agent_picker_thread(
        &mut self,
        thread_id: ThreadId,
        agent_nickname: Option<String>,
        agent_role: Option<String>,
        is_closed: bool,
    ) {
        self.chat_widget.set_collab_agent_metadata(
            thread_id,
            agent_nickname.clone(),
            agent_role.clone(),
        );
        self.agent_navigation
            .upsert(thread_id, agent_nickname, agent_role, is_closed);
        self.sync_active_agent_label();
    }

    /// Marks a cached picker thread closed and recomputes the contextual footer label.
    ///
    /// Closing a thread is not the same as removing it: users can still inspect finished agent
    /// transcripts, and the stable next/previous traversal order should not collapse around them.
    pub(super) fn mark_agent_picker_thread_closed(&mut self, thread_id: ThreadId) {
        self.agent_navigation.mark_closed(thread_id);
        self.sync_active_agent_label();
    }

    pub(super) fn update_agent_picker_thread_detail_from_thread(
        &mut self,
        thread_id: ThreadId,
        thread: &codex_app_server_protocol::Thread,
    ) {
        let (agent_path, source_thread_note) = match &thread.source {
            codex_app_server_protocol::SessionSource::SubAgent(
                codex_protocol::protocol::SubAgentSource::ThreadSpawn {
                    agent_path,
                    thread_note,
                    ..
                },
            ) => (
                agent_path.as_ref().map(ToString::to_string),
                thread_note.clone(),
            ),
            _ => (None, None),
        };
        let thread_note = thread.thread_note.clone().or(source_thread_note);
        self.agent_navigation.update_thread_detail(
            thread_id,
            AgentPickerThreadDetail {
                agent_path,
                prompt_preview: Some(thread.preview.clone()),
                thread_note,
                cwd: Some(thread.cwd.display().to_string()),
                model_provider: Some(thread.model_provider.clone()),
                created_at: Some(thread.created_at),
                updated_at: Some(thread.updated_at),
            },
        );
    }

    pub(super) async fn refresh_agent_picker_thread_liveness(
        &mut self,
        app_server: &mut AppServerSession,
        thread_id: ThreadId,
    ) -> bool {
        let existing_entry = self.agent_navigation.get(&thread_id).cloned();
        let has_replay_channel = self.thread_event_channels.contains_key(&thread_id);
        match app_server
            .thread_read(thread_id, /*include_turns*/ false)
            .await
        {
            Ok(thread) => {
                let is_closed = matches!(
                    thread.status,
                    codex_app_server_protocol::ThreadStatus::NotLoaded
                );
                let status = crate::multi_agents::AgentPickerThreadStatus::from_thread_status(
                    &thread.status,
                );
                let agent_nickname = thread.agent_nickname.clone().or_else(|| {
                    existing_entry
                        .as_ref()
                        .and_then(|entry| entry.agent_nickname.clone())
                });
                let agent_role = thread.agent_role.clone().or_else(|| {
                    existing_entry
                        .as_ref()
                        .and_then(|entry| entry.agent_role.clone())
                });
                self.upsert_agent_picker_thread(thread_id, agent_nickname, agent_role, is_closed);
                self.update_agent_picker_thread_detail_from_thread(thread_id, &thread);
                self.agent_navigation.set_status(thread_id, status);
                true
            }
            Err(err) => {
                if Self::is_terminal_thread_read_error(&err) && !has_replay_channel {
                    self.agent_navigation.remove(thread_id);
                    return false;
                }
                let is_closed = Self::closed_state_for_thread_read_error(
                    &err,
                    existing_entry
                        .as_ref()
                        .map(|entry| entry.status.is_closed()),
                );
                if let Some(entry) = existing_entry {
                    self.upsert_agent_picker_thread(
                        thread_id,
                        entry.agent_nickname,
                        entry.agent_role,
                        is_closed,
                    );
                } else {
                    self.upsert_agent_picker_thread(
                        thread_id, /*agent_nickname*/ None, /*agent_role*/ None,
                        is_closed,
                    );
                }
                if !is_closed {
                    self.agent_navigation
                        .set_running(thread_id, /*is_running*/ false);
                }
                true
            }
        }
    }

    /// Materializes a live thread into local replay state when the picker knows about it but the
    /// TUI has not cached a local event channel yet.
    ///
    /// Resume-time backfill intentionally avoids creating empty placeholder channels, because those
    /// placeholders make stale `/agent` entries open blank transcripts. When a user later selects a
    /// still-live discovered thread, attach it on demand with a real resumed snapshot.
    pub(super) async fn attach_live_thread_for_selection(
        &mut self,
        app_server: &mut AppServerSession,
        thread_id: ThreadId,
    ) -> Result<bool> {
        if self.thread_event_channels.contains_key(&thread_id) {
            return Ok(true);
        }

        let (session, turns, live_attached) = match app_server
            .resume_thread(self.config.clone(), thread_id)
            .await
        {
            Ok(started) => (started.session, started.turns, true),
            Err(resume_err) => {
                tracing::warn!(
                    thread_id = %thread_id,
                    error = %resume_err,
                    "failed to resume live thread for selection; falling back to thread/read"
                );
                let (thread, turns) = match app_server
                    .thread_read(thread_id, /*include_turns*/ true)
                    .await
                {
                    Ok(thread) => {
                        let turns = thread.turns.clone();
                        (thread, turns)
                    }
                    Err(err) if Self::can_fallback_from_include_turns_error(&err) => {
                        let thread = app_server
                            .thread_read(thread_id, /*include_turns*/ false)
                            .await?;
                        (thread, Vec::new())
                    }
                    Err(err) => return Err(err),
                };
                if turns.is_empty() {
                    // A `thread/read` fallback without turns would create a blank local replay
                    // channel with no live listener attached, which blocks later real re-attach.
                    return Err(color_eyre::eyre::eyre!(
                        "Agent thread {thread_id} is not yet available for replay or live attach."
                    ));
                }
                let mut session = self.session_state_for_thread_read(thread_id, &thread).await;
                // `thread/read` can seed replay state, but it does not attach the app-server
                // listener that `thread/resume` establishes, so treat this path as replay-only.
                session.model.clear();
                (session, turns, false)
            }
        };
        let channel = self.ensure_thread_channel(thread_id);
        if !live_attached {
            channel.mark_replay_only();
        }
        let mut store = channel.store.lock().await;
        store.set_session(session, turns);
        Ok(live_attached)
    }

    /// Replaces the chat widget and re-seeds the new widget's collab metadata from the navigation
    /// cache.
    ///
    /// Thread switches reconstruct the `ChatWidget`, which loses the `collab_agent_metadata` map.
    /// This helper copies every known nickname/role from `AgentNavigationState` into the
    /// replacement widget so that replayed collab items render agent names immediately.
    pub(super) fn replace_chat_widget(&mut self, mut chat_widget: ChatWidget) {
        // Transfer the last-written terminal title to the replacement widget
        // so it knows what OSC title is currently displayed. Without this, the
        // new widget would redundantly clear and rewrite the same title, causing
        // a visible flicker in some terminals.
        let previous_terminal_title = self.chat_widget.last_terminal_title.take();
        if chat_widget.last_terminal_title.is_none() {
            chat_widget.last_terminal_title = previous_terminal_title;
        }
        chat_widget.remote_connection = self.chat_widget.remote_connection.clone();
        for (thread_id, entry) in self.agent_navigation.ordered_threads() {
            chat_widget.set_collab_agent_metadata(
                thread_id,
                entry.agent_nickname.clone(),
                entry.agent_role.clone(),
            );
        }
        self.chat_widget = chat_widget;
        self.sync_active_agent_label();
    }

    pub(super) async fn select_agent_thread(
        &mut self,
        tui: &mut tui::Tui,
        app_server: &mut AppServerSession,
        thread_id: ThreadId,
    ) -> Result<()> {
        if self.active_thread_id == Some(thread_id) {
            return Ok(());
        }

        if !self
            .refresh_agent_picker_thread_liveness(app_server, thread_id)
            .await
        {
            self.chat_widget
                .add_error_message(format!("Agent thread {thread_id} is no longer available."));
            return Ok(());
        }

        let mut is_replay_only = self
            .agent_navigation
            .get(&thread_id)
            .is_some_and(|entry| entry.status.is_closed());
        let mut attached_replay_only = false;
        if self.should_attach_live_thread_for_selection(thread_id) {
            match self
                .attach_live_thread_for_selection(app_server, thread_id)
                .await
            {
                Ok(live_attached) => {
                    attached_replay_only = !live_attached;
                    if attached_replay_only {
                        is_replay_only = true;
                    }
                }
                Err(err) => {
                    self.chat_widget.add_error_message(format!(
                        "Failed to attach to agent thread {thread_id}: {err}"
                    ));
                    return Ok(());
                }
            }
        } else if !self.thread_event_channels.contains_key(&thread_id) && is_replay_only {
            self.chat_widget
                .add_error_message(format!("Agent thread {thread_id} is no longer available."));
            return Ok(());
        }

        let previous_thread_id = self.active_thread_id;
        self.store_active_thread_receiver().await;
        self.active_thread_id = None;
        let Some((receiver, mut snapshot)) = self.activate_thread_for_replay(thread_id).await
        else {
            self.chat_widget
                .add_error_message(format!("Agent thread {thread_id} is already active."));
            if let Some(previous_thread_id) = previous_thread_id {
                self.activate_thread_channel(previous_thread_id).await;
            }
            return Ok(());
        };

        self.refresh_snapshot_session_if_needed(
            app_server,
            thread_id,
            is_replay_only,
            &mut snapshot,
        )
        .await;

        self.active_thread_id = Some(thread_id);
        self.active_thread_rx = Some(receiver);

        let init = self.chatwidget_init_for_forked_or_resumed_thread(
            tui,
            self.config.clone(),
            /*initial_user_message*/ None,
        );
        self.replace_chat_widget(ChatWidget::new_with_app_event(init));

        self.reset_for_thread_switch(tui)?;
        self.replay_thread_snapshot(snapshot, !is_replay_only);
        if is_replay_only {
            let message = if attached_replay_only {
                format!(
                    "Agent thread {thread_id} could not be resumed live. Replaying saved transcript."
                )
            } else {
                format!("Agent thread {thread_id} is closed. Replaying saved transcript.")
            };
            self.chat_widget.add_info_message(message, /*hint*/ None);
        }
        self.drain_active_thread_events(tui).await?;
        self.refresh_pending_thread_approvals().await;

        Ok(())
    }

    pub(super) fn should_attach_live_thread_for_selection(&self, thread_id: ThreadId) -> bool {
        !self.thread_event_channels.contains_key(&thread_id)
            && self
                .agent_navigation
                .get(&thread_id)
                .is_none_or(|entry| !entry.status.is_closed())
    }

    pub(super) fn reset_for_thread_switch(&mut self, tui: &mut tui::Tui) -> Result<()> {
        self.reset_transcript_state_after_clear();
        tui.clear_pending_history_lines();
        Self::clear_terminal_for_thread_switch(&mut tui.terminal)?;
        Ok(())
    }

    pub(super) fn clear_terminal_for_thread_switch<B>(
        terminal: &mut crate::custom_terminal::Terminal<B>,
    ) -> Result<()>
    where
        B: Backend + Write,
    {
        terminal.clear_scrollback_and_visible_screen_ansi()?;
        let mut area = terminal.viewport_area;
        if area.y > 0 {
            area.y = 0;
            terminal.set_viewport_area(area);
        }
        Ok(())
    }

    pub(super) fn reset_thread_event_state(&mut self) {
        self.abort_all_thread_event_listeners();
        self.thread_event_channels.clear();
        self.agent_navigation.clear();
        self.side_threads.clear();
        self.active_thread_id = None;
        self.active_thread_rx = None;
        self.primary_thread_id = None;
        self.last_subagent_backfill_attempt = None;
        self.primary_session_configured = None;
        self.pending_primary_events.clear();
        self.pending_app_server_requests.clear();
        self.pending_startup_thread_start = false;
        self.chat_widget.set_pending_thread_approvals(Vec::new());
        self.sync_active_agent_label();
    }

    pub(super) async fn handle_startup_thread_started(
        &mut self,
        app_server: &mut AppServerSession,
        result: Result<AppServerStartedThread, String>,
    ) -> Result<()> {
        if !self.pending_startup_thread_start {
            if let Ok(started) = result {
                let thread_id = started.session.thread_id;
                if let Err(err) = app_server.thread_unsubscribe(thread_id).await {
                    tracing::warn!(
                        thread_id = %thread_id,
                        "failed to unsubscribe stale startup thread: {err}"
                    );
                }
                self.discard_thread_local_state(thread_id).await;
            }
            return Ok(());
        }

        self.pending_startup_thread_start = false;
        self.chat_widget
            .set_queue_submissions_until_session_configured(/*queue*/ false);
        match result {
            Ok(started) => {
                self.enqueue_primary_thread_session(started.session, started.turns)
                    .await?;
                self.chat_widget.maybe_send_next_queued_input();
            }
            Err(err) => {
                return Err(color_eyre::eyre::eyre!(
                    "Failed to start a fresh session through the app server: {err}"
                ));
            }
        }
        Ok(())
    }

    pub(super) async fn start_fresh_session_with_summary_hint(
        &mut self,
        tui: &mut tui::Tui,
        app_server: &mut AppServerSession,
        session_start_source: Option<ThreadStartSource>,
        initial_user_message: Option<crate::chatwidget::UserMessage>,
    ) {
        // Start a fresh in-memory session while preserving resumability via persisted rollout
        // history. If an initial message is provided, `enqueue_primary_thread_session` suppresses it
        // until the new session is configured and any replayed turns have been rendered.
        self.refresh_in_memory_config_from_disk_best_effort("starting a new thread")
            .await;
        let model = self.chat_widget.current_model().to_string();
        let config = self.fresh_session_config();
        let summary = session_summary(
            self.chat_widget.token_usage(),
            self.chat_widget.thread_id(),
            self.chat_widget.thread_name(),
            self.chat_widget.rollout_path().as_deref(),
        );
        self.shutdown_current_thread(app_server).await;
        let tracked_thread_ids: Vec<ThreadId> =
            self.thread_event_channels.keys().copied().collect();
        for thread_id in tracked_thread_ids {
            if let Err(err) = app_server.thread_unsubscribe(thread_id).await {
                tracing::warn!("failed to unsubscribe tracked thread {thread_id}: {err}");
            }
        }
        self.config = config.clone();
        match app_server
            .start_thread_with_session_start_source(&config, session_start_source)
            .await
        {
            Ok(started) => {
                if let Err(err) = self
                    .replace_chat_widget_with_app_server_thread(
                        tui,
                        app_server,
                        started,
                        initial_user_message,
                    )
                    .await
                {
                    self.chat_widget.add_error_message(format!(
                        "Failed to attach to fresh app-server thread: {err}"
                    ));
                } else if let Some(summary) = summary {
                    let mut lines: Vec<Line<'static>> = Vec::new();
                    if let Some(usage_line) = summary.usage_line {
                        lines.push(usage_line.into());
                    }
                    if let Some(command) = summary.resume_hint {
                        let spans = vec!["To continue this session, run ".into(), command.cyan()];
                        lines.push(spans.into());
                    }
                    self.chat_widget.add_plain_history_lines(lines);
                }
            }
            Err(err) => {
                self.chat_widget.add_error_message(format!(
                    "Failed to start a fresh session through the app server: {err}"
                ));
                self.config.model = Some(model);
            }
        }
        tui.frame_requester().schedule_frame();
    }

    pub(super) async fn replace_chat_widget_with_app_server_thread(
        &mut self,
        tui: &mut tui::Tui,
        app_server: &mut AppServerSession,
        started: AppServerStartedThread,
        initial_user_message: Option<crate::chatwidget::UserMessage>,
    ) -> Result<()> {
        // Initial messages are for freshly attached primary threads only. Thread switches and
        // resume/fork flows pass `None` so they cannot replay old history and then auto-submit a new
        // user turn by accident.
        self.reset_thread_event_state();
        let init = self.chatwidget_init_for_forked_or_resumed_thread(
            tui,
            self.config.clone(),
            initial_user_message,
        );
        self.replace_chat_widget(ChatWidget::new_with_app_event(init));
        self.enqueue_primary_thread_session(started.session, started.turns)
            .await?;
        self.backfill_loaded_subagent_threads(app_server).await;
        Ok(())
    }

    /// Fetches all loaded threads from the app server and registers descendants of the primary
    /// thread in the navigation cache and chat widget metadata.
    ///
    /// Called after `replace_chat_widget_with_app_server_thread` during resume, fork, and new
    /// thread creation so that the `/agent` picker and keyboard navigation are pre-populated even
    /// if the TUI did not witness the original spawn events.
    ///
    /// The loaded-thread list is fetched in full (no pagination) and the spawn tree is walked
    /// by `find_loaded_subagent_threads_for_primary`. Each discovered subagent is registered via
    /// `upsert_agent_picker_thread`, which writes to both `AgentNavigationState` and the
    /// `ChatWidget` metadata map.
    pub(super) async fn backfill_loaded_subagent_threads(
        &mut self,
        app_server: &mut AppServerSession,
    ) -> bool {
        let Some(primary_thread_id) = self.primary_thread_id else {
            return false;
        };

        let loaded_thread_ids = match app_server
            .thread_loaded_list(ThreadLoadedListParams {
                cursor: None,
                limit: None,
            })
            .await
        {
            Ok(response) => response.data,
            Err(err) => {
                tracing::warn!(%err, "failed to list loaded threads for subagent backfill");
                return false;
            }
        };

        let mut threads = Vec::new();
        let mut had_read_error = false;
        for thread_id in loaded_thread_ids {
            let Ok(thread_id) = ThreadId::from_string(&thread_id) else {
                tracing::warn!("ignoring loaded thread with invalid id during subagent backfill");
                continue;
            };

            if thread_id == primary_thread_id {
                continue;
            }

            match app_server
                .thread_read(thread_id, /*include_turns*/ false)
                .await
            {
                Ok(thread) => threads.push(thread),
                Err(err) => {
                    had_read_error = true;
                    tracing::warn!(thread_id = %thread_id, %err, "failed to read loaded thread");
                }
            }
        }

        for thread in find_loaded_subagent_threads_for_primary(threads, primary_thread_id) {
            let agent_path = thread.agent_path;
            self.upsert_agent_picker_thread(
                thread.thread_id,
                thread.agent_nickname,
                thread.agent_role,
                /*is_closed*/ false,
            );
            self.agent_navigation
                .set_agent_path(thread.thread_id, agent_path);
        }
        self.sync_active_agent_label();

        !had_read_error
    }

    /// Returns the adjacent thread id for keyboard navigation, backfilling from the server if the
    /// local cache has no neighbor.
    ///
    /// Tries the fast path first: ask `AgentNavigationState` directly. If it returns `None` (no
    /// adjacent entry exists, typically because the cache was never populated with remote
    /// subagents), performs a full `backfill_loaded_subagent_threads` and retries. This ensures the
    /// first next/previous keypress in a resumed remote session discovers subagents on demand
    /// without requiring the user to wait for a proactive fetch.
    pub(super) async fn adjacent_thread_id_with_backfill(
        &mut self,
        app_server: &mut AppServerSession,
        direction: AgentNavigationDirection,
    ) -> Option<ThreadId> {
        let current_thread = self.current_displayed_thread_id();
        if let Some(thread_id) = self
            .agent_navigation
            .adjacent_thread_id(current_thread, direction)
        {
            return Some(thread_id);
        }

        let primary_thread_id = self.primary_thread_id?;
        if self.last_subagent_backfill_attempt == Some(primary_thread_id) {
            return None;
        }

        if self.backfill_loaded_subagent_threads(app_server).await {
            self.last_subagent_backfill_attempt = Some(primary_thread_id);
        }
        self.agent_navigation
            .adjacent_thread_id(self.current_displayed_thread_id(), direction)
    }

    pub(super) fn fresh_session_config(&self) -> Config {
        let mut config = self.config.clone();
        config.service_tier = self.chat_widget.configured_service_tier();
        config
    }
    pub(super) async fn resume_target_session(
        &mut self,
        tui: &mut tui::Tui,
        app_server: &mut AppServerSession,
        target_session: SessionTarget,
    ) -> Result<AppRunControl> {
        if self.ignore_same_thread_resume(&target_session) {
            tui.frame_requester().schedule_frame();
            return Ok(AppRunControl::Continue);
        }

        let current_cwd = self.config.cwd.to_path_buf();
        let resume_cwd = if self.app_server_target.uses_remote_workspace() {
            current_cwd.clone()
        } else {
            match crate::session_resume::resolve_cwd_for_resume_or_fork(
                tui,
                self.state_db.as_deref(),
                &current_cwd,
                target_session.thread_id,
                target_session.path.as_deref(),
                CwdPromptAction::Resume,
                /*allow_prompt*/ true,
            )
            .await?
            {
                crate::session_resume::ResolveCwdOutcome::Continue(Some(cwd)) => cwd,
                crate::session_resume::ResolveCwdOutcome::Continue(None) => current_cwd.clone(),
                crate::session_resume::ResolveCwdOutcome::Exit => {
                    return Ok(AppRunControl::Exit(ExitReason::UserRequested));
                }
            }
        };

        let mut resume_config = match self
            .rebuild_config_for_resume_or_fallback(&current_cwd, resume_cwd)
            .await
        {
            Ok(cfg) => cfg,
            Err(err) => {
                self.chat_widget.add_error_message(format!(
                    "Failed to rebuild configuration for resume: {err}"
                ));
                return Ok(AppRunControl::Continue);
            }
        };
        self.apply_runtime_policy_overrides(&mut resume_config);

        let summary = session_summary(
            self.chat_widget.token_usage(),
            self.chat_widget.thread_id(),
            self.chat_widget.thread_name(),
            self.chat_widget.rollout_path().as_deref(),
        );
        match app_server
            .resume_thread(resume_config.clone(), target_session.thread_id)
            .await
        {
            Ok(resumed) => {
                let resumed_thread_id = resumed.session.thread_id;
                self.shutdown_current_thread(app_server).await;
                self.config = resume_config;
                tui.set_notification_settings(
                    self.config.tui_notifications.method,
                    self.config.tui_notifications.condition,
                );
                self.file_search
                    .update_search_dir(self.config.cwd.to_path_buf());
                match self
                    .replace_chat_widget_with_app_server_thread(
                        tui, app_server, resumed, /*initial_user_message*/ None,
                    )
                    .await
                {
                    Ok(()) => {
                        if let Some(summary) = summary {
                            let mut lines: Vec<Line<'static>> = Vec::new();
                            if let Some(usage_line) = summary.usage_line {
                                lines.push(usage_line.into());
                            }
                            if let Some(command) = summary.resume_hint {
                                let spans =
                                    vec!["To continue this session, run ".into(), command.cyan()];
                                lines.push(spans.into());
                            }
                            self.chat_widget.add_plain_history_lines(lines);
                        }
                        self.maybe_prompt_resume_paused_goal_after_resume(
                            app_server,
                            resumed_thread_id,
                        )
                        .await;
                    }
                    Err(err) => {
                        self.chat_widget.add_error_message(format!(
                            "Failed to attach to resumed app-server thread: {err}"
                        ));
                    }
                }
            }
            Err(err) => {
                let path_display = target_session.display_label();
                self.chat_widget.add_error_message(format!(
                    "Failed to resume session from {path_display}: {err}"
                ));
            }
        }

        Ok(AppRunControl::Continue)
    }
}

fn thread_item_for_agent_picker_activity(event: &ThreadBufferedEvent) -> Option<&ThreadItem> {
    match event {
        ThreadBufferedEvent::Notification(ServerNotification::ItemCompleted(event)) => {
            Some(&event.item)
        }
        ThreadBufferedEvent::Notification(ServerNotification::ItemStarted(event)) => {
            Some(&event.item)
        }
        ThreadBufferedEvent::Notification(_)
        | ThreadBufferedEvent::Request(_)
        | ThreadBufferedEvent::HistoryEntryResponse(_)
        | ThreadBufferedEvent::FeedbackSubmission(_) => None,
    }
}

fn agent_path_can_interrupt(current_agent_path: &str, target_agent_path: &str) -> bool {
    if current_agent_path == "/root" {
        return target_agent_path.starts_with("/root/");
    }
    target_agent_path
        .strip_prefix(current_agent_path)
        .is_some_and(|suffix| suffix.starts_with('/'))
}

fn collab_agent_state_status_label(state: &codex_app_server_protocol::CollabAgentState) -> String {
    match &state.status {
        codex_app_server_protocol::CollabAgentStatus::PendingInit => "pending".to_string(),
        codex_app_server_protocol::CollabAgentStatus::Running => "running".to_string(),
        codex_app_server_protocol::CollabAgentStatus::Interrupted => "interrupted".to_string(),
        codex_app_server_protocol::CollabAgentStatus::Completed => {
            state.message.as_deref().unwrap_or("completed").to_string()
        }
        codex_app_server_protocol::CollabAgentStatus::Errored => state
            .message
            .as_deref()
            .map(|message| format!("error: {message}"))
            .unwrap_or_else(|| "error".to_string()),
        codex_app_server_protocol::CollabAgentStatus::Shutdown => "shutdown".to_string(),
        codex_app_server_protocol::CollabAgentStatus::NotFound => "not found".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_thread_read_error_detection_matches_not_loaded_errors() {
        let err = color_eyre::eyre::eyre!(
            "thread/read failed during TUI session lookup: thread/read failed: thread not loaded: thr_123"
        );

        assert!(App::is_terminal_thread_read_error(&err));
    }

    #[test]
    fn terminal_thread_read_error_detection_ignores_transient_failures() {
        let err = color_eyre::eyre::eyre!(
            "thread/read failed during TUI session lookup: thread/read transport error: broken pipe"
        );

        assert!(!App::is_terminal_thread_read_error(&err));
    }

    #[test]
    fn closed_state_for_thread_read_error_preserves_live_state_without_cache_on_transient_error() {
        let err = color_eyre::eyre::eyre!(
            "thread/read failed during TUI session lookup: thread/read transport error: broken pipe"
        );

        assert!(!App::closed_state_for_thread_read_error(
            &err, /*existing_is_closed*/ None
        ));
    }

    #[test]
    fn closed_state_for_thread_read_error_marks_terminal_uncached_threads_closed() {
        let err = color_eyre::eyre::eyre!(
            "thread/read failed during TUI session lookup: thread/read failed: thread not loaded: thr_123"
        );

        assert!(App::closed_state_for_thread_read_error(
            &err, /*existing_is_closed*/ None
        ));
    }

    #[test]
    fn include_turns_fallback_detection_handles_unmaterialized_and_ephemeral_threads() {
        let unmaterialized = color_eyre::eyre::eyre!(
            "thread/read failed during TUI session lookup: thread/read failed: thread thr_123 is not materialized yet; includeTurns is unavailable before first user message"
        );
        let ephemeral = color_eyre::eyre::eyre!(
            "thread/read failed during TUI session lookup: thread/read failed: ephemeral threads do not support includeTurns"
        );

        assert!(App::can_fallback_from_include_turns_error(&unmaterialized));
        assert!(App::can_fallback_from_include_turns_error(&ephemeral));
    }
}
