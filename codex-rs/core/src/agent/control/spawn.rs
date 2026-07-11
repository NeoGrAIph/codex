use super::residency::is_v2_resident_session_source;
use super::*;
use crate::session::SubmissionLoopActivation;
use codex_extension_api::ExtensionDataInit;
use codex_thread_store::ReadThreadParams;

const AGENT_NAMES: &str = include_str!("../agent_names.txt");

struct SpawnAgentThreadInheritance {
    environments: Option<TurnEnvironmentSnapshot>,
    exec_policy: Option<Arc<crate::exec_policy::ExecPolicyManager>>,
}

/// Initial input delivered after a spawned agent acquires execution capacity.
///
/// V2 communication spawns keep the communication and its context paired so centralized
/// submission and lifecycle logging cannot receive one without the other. Other spawn sources
/// provide user input directly, making an uncontextualized inter-agent communication
/// unrepresentable.
enum SpawnInitialInput {
    UserInput(Vec<UserInput>),
    InterAgentCommunication(InterAgentCommunication, AgentCommunicationContext),
}

fn default_agent_nickname_list() -> Vec<&'static str> {
    AGENT_NAMES
        .lines()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .collect()
}

pub(super) fn agent_nickname_candidates(config: &Config, role_name: Option<&str>) -> Vec<String> {
    let role_name = role_name.unwrap_or(DEFAULT_ROLE_NAME);
    if let Some(candidates) =
        resolve_role_config(config, role_name).and_then(|role| role.nickname_candidates.clone())
    {
        return candidates;
    }

    default_agent_nickname_list()
        .into_iter()
        .map(ToOwned::to_owned)
        .collect()
}

fn keep_forked_rollout_item(item: &RolloutItem, preserve_reference_context_item: bool) -> bool {
    match item {
        RolloutItem::ResponseItem(ResponseItem::Message { role, phase, .. }) => match role.as_str()
        {
            "system" | "developer" | "user" => true,
            "assistant" => *phase == Some(MessagePhase::FinalAnswer),
            _ => false,
        },
        RolloutItem::ResponseItem(
            ResponseItem::AdditionalTools { .. }
            | ResponseItem::AgentMessage { .. }
            | ResponseItem::Reasoning { .. }
            | ResponseItem::LocalShellCall { .. }
            | ResponseItem::FunctionCall { .. }
            | ResponseItem::ToolSearchCall { .. }
            | ResponseItem::FunctionCallOutput { .. }
            | ResponseItem::CustomToolCall { .. }
            | ResponseItem::CustomToolCallOutput { .. }
            | ResponseItem::ToolSearchOutput { .. }
            | ResponseItem::WebSearchCall { .. }
            | ResponseItem::ImageGenerationCall { .. }
            | ResponseItem::Compaction { .. }
            | ResponseItem::CompactionTrigger { .. }
            | ResponseItem::ContextCompaction { .. }
            | ResponseItem::Other,
        ) => false,
        RolloutItem::InterAgentCommunication(_)
        | RolloutItem::InterAgentCommunicationMetadata { .. } => false,
        // Full-history forks preserve the cached prompt prefix and can keep diffing
        // from the parent's durable baseline. Truncated forks drop part of that prompt,
        // so they must rebuild context on their first child turn.
        RolloutItem::TurnContext(_) | RolloutItem::WorldState(_) => preserve_reference_context_item,
        RolloutItem::Compacted(_) | RolloutItem::EventMsg(_) | RolloutItem::SessionMeta(_) => true,
    }
}

fn is_multi_agent_v2_usage_hint_message(item: &ResponseItem, usage_hint_texts: &[String]) -> bool {
    let ResponseItem::Message { role, content, .. } = item else {
        return false;
    };
    if role != "developer" {
        return false;
    }
    let [ContentItem::InputText { text }] = content.as_slice() else {
        return false;
    };

    usage_hint_texts
        .iter()
        .any(|usage_hint_text| usage_hint_text == text)
}

impl AgentControl {
    /// Spawn a new agent thread and submit the initial prompt.
    #[cfg(test)]
    pub(crate) async fn spawn_agent(
        &self,
        config: Config,
        initial_input: Vec<UserInput>,
        session_source: Option<SessionSource>,
    ) -> CodexResult<ThreadId> {
        let spawned_agent = Box::pin(self.spawn_agent_internal(
            config,
            SpawnInitialInput::UserInput(initial_input),
            session_source,
            SpawnAgentOptions::default(),
        ))
        .await?;
        Ok(spawned_agent.thread_id)
    }

    /// Spawn an agent thread with some metadata.
    pub(crate) async fn spawn_agent_with_metadata(
        &self,
        config: Config,
        initial_input: Vec<UserInput>,
        session_source: Option<SessionSource>,
        options: SpawnAgentOptions, // TODO(jif) drop with new fork.
    ) -> CodexResult<LiveAgent> {
        Box::pin(self.spawn_agent_internal(
            config,
            SpawnInitialInput::UserInput(initial_input),
            session_source,
            options,
        ))
        .await
    }

    pub(crate) async fn spawn_agent_with_communication(
        &self,
        config: Config,
        communication: InterAgentCommunication,
        context: AgentCommunicationContext,
        session_source: Option<SessionSource>,
        options: SpawnAgentOptions,
    ) -> CodexResult<LiveAgent> {
        Box::pin(self.spawn_agent_internal(
            config,
            SpawnInitialInput::InterAgentCommunication(communication, context),
            session_source,
            options,
        ))
        .await
    }

    async fn spawn_agent_internal(
        &self,
        config: Config,
        initial_input: SpawnInitialInput,
        session_source: Option<SessionSource>,
        options: SpawnAgentOptions,
    ) -> CodexResult<LiveAgent> {
        let state = self.upgrade()?;
        let multi_agent_version = state
            .effective_multi_agent_version_for_spawn(
                &InitialHistory::New,
                session_source.as_ref(),
                options.parent_thread_id,
                /*forked_from_thread_id*/ None,
                &config,
            )
            .await;
        let uses_transactional_activation = multi_agent_version == MultiAgentVersion::V2
            && matches!(
                session_source.as_ref(),
                Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn { .. }))
            );
        let _lifecycle_guard = if uses_transactional_activation {
            Some(self.acquire_lifecycle_gate().await)
        } else {
            None
        };
        if uses_transactional_activation && state.agent_graph_store().is_none() {
            return Err(CodexErr::Fatal(
                "authoritative agent graph is unavailable; refusing V2 agent spawn".to_string(),
            ));
        }
        if uses_transactional_activation
            && let Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth,
                ..
            })) = session_source.as_ref()
        {
            self.ensure_no_pending_lifecycle_cleanup(*parent_thread_id)?;
            state.get_thread(*parent_thread_id).await?;
            if *depth > 1
                && self
                    .state
                    .agent_metadata_for_thread(*parent_thread_id)
                    .is_none()
            {
                return Err(CodexErr::ThreadNotFound(*parent_thread_id));
            }
        }
        if let Some(session_source) = session_source.as_ref() {
            self.ensure_execution_capacity(multi_agent_version, session_source)?;
        }
        let agent_max_threads = config.effective_agent_max_threads(multi_agent_version);
        let spawn_uses_v2_residency = multi_agent_version == MultiAgentVersion::V2
            && session_source
                .as_ref()
                .is_some_and(is_v2_resident_session_source);
        let mut residency_slot = if spawn_uses_v2_residency {
            Some(
                self.reserve_v2_residency_slot(&state, &config, /*protected_thread_id*/ None)
                    .await?,
            )
        } else {
            None
        };
        let reservation_max_threads = if spawn_uses_v2_residency {
            None
        } else {
            agent_max_threads
        };
        let mut reservation = Some(self.state.reserve_spawn_slot(reservation_max_threads)?);
        let inheritance = SpawnAgentThreadInheritance {
            environments: self
                .inherited_environments_for_source(&state, session_source.as_ref())
                .await,
            exec_policy: self
                .inherited_exec_policy_for_source(&state, session_source.as_ref(), &config)
                .await,
        };
        let (session_source, mut agent_metadata) = match session_source {
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth,
                agent_path,
                agent_role,
                ..
            })) => {
                let Some(reservation) = reservation.as_mut() else {
                    return Err(CodexErr::Fatal(
                        "spawn reservation unavailable before child identity preparation"
                            .to_string(),
                    ));
                };
                let (session_source, agent_metadata) = self.prepare_thread_spawn(
                    reservation,
                    &config,
                    parent_thread_id,
                    depth,
                    agent_path,
                    agent_role,
                    /*preferred_agent_nickname*/ None,
                )?;
                (Some(session_source), agent_metadata)
            }
            other => (other, AgentMetadata::default()),
        };
        let notification_source = session_source.clone();
        let (mut submission_loop_activation_tx, mut submission_loop_activation_rx) =
            if uses_transactional_activation {
                let (tx, rx) = tokio::sync::oneshot::channel();
                (Some(tx), Some(rx))
            } else {
                (None, None)
            };

        // The same `AgentControl` is sent to spawn the thread.
        let new_thread = match (session_source, options.fork_mode.as_ref(), inheritance) {
            (Some(session_source), Some(_), inheritance) => {
                Box::pin(self.spawn_forked_thread(
                    &state,
                    config,
                    session_source,
                    &options,
                    inheritance,
                    multi_agent_version,
                    submission_loop_activation_rx.take(),
                ))
                .await?
            }
            (Some(session_source), None, inheritance) => {
                Box::pin(state.spawn_new_thread_with_source(
                    config.clone(),
                    self.clone(),
                    session_source,
                    options.parent_thread_id,
                    /*forked_from_thread_id*/ None,
                    /*thread_source*/ Some(ThreadSource::Subagent),
                    /*metrics_service_name*/ None,
                    inheritance.environments,
                    inheritance.exec_policy,
                    options.environments.clone(),
                    submission_loop_activation_rx.take(),
                ))
                .await?
            }
            (None, _, _) => Box::pin(state.spawn_new_thread(config.clone(), self.clone())).await?,
        };
        agent_metadata.agent_id = Some(new_thread.thread_id);
        if !uses_transactional_activation {
            let Some(reservation) = reservation.take() else {
                return Err(CodexErr::Fatal(
                    "spawn reservation unavailable before compatibility publication".to_string(),
                ));
            };
            reservation.commit(agent_metadata.clone());
            if let Some(residency_slot) = residency_slot.take() {
                residency_slot.commit(new_thread.thread_id);
            }
            self.emit_spawn_started_analytics(&state, &new_thread, notification_source.as_ref())
                .await;
            state.notify_thread_created(new_thread.thread_id);
            if let Err(err) = self
                .persist_thread_spawn_edge_for_source(
                    new_thread.thread.as_ref(),
                    new_thread.thread_id,
                    notification_source.as_ref(),
                    codex_agent_graph_store::ThreadSpawnEdgeStatus::Open,
                )
                .await
            {
                warn!(
                    %err,
                    thread_id = %new_thread.thread_id,
                    "failed to persist compatibility thread-spawn edge"
                );
            }
            match initial_input {
                SpawnInitialInput::UserInput(input) => {
                    self.send_input_after_capacity_check(new_thread.thread_id, &state, input)
                        .await?;
                }
                SpawnInitialInput::InterAgentCommunication(communication, context) => {
                    self.send_inter_agent_communication_after_capacity_check(
                        new_thread.thread_id,
                        &state,
                        communication,
                        context,
                    )
                    .await?;
                }
            }
            let child_reference = agent_metadata
                .agent_path
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| new_thread.thread_id.to_string());
            self.maybe_start_completion_watcher(
                new_thread.thread_id,
                notification_source,
                child_reference,
                agent_metadata.agent_path.clone(),
            );
            return Ok(LiveAgent {
                thread_id: new_thread.thread_id,
                metadata: agent_metadata,
                status: self.get_status(new_thread.thread_id).await,
            });
        }
        if let Err(persist_err) = self
            .persist_thread_spawn_edge_for_source(
                new_thread.thread.as_ref(),
                new_thread.thread_id,
                notification_source.as_ref(),
                codex_agent_graph_store::ThreadSpawnEdgeStatus::PendingActivation,
            )
            .await
        {
            if let Some(activation_tx) = submission_loop_activation_tx.take() {
                let _ = activation_tx.send(SubmissionLoopActivation::Abort);
            }
            if let Err(shutdown_err) = self
                .shutdown_thread_for_lifecycle(
                    &state,
                    Arc::clone(&new_thread.thread),
                    TerminatedThreadCleanup::FailedSpawnRollback,
                )
                .await
            {
                if let Some(reservation) = reservation.take() {
                    reservation.commit(agent_metadata.clone());
                }
                if let Some(residency_slot) = residency_slot.take() {
                    residency_slot.commit(new_thread.thread_id);
                }
                warn!(
                    %shutdown_err,
                    thread_id = %new_thread.thread_id,
                    "failed to stop agent after lifecycle persistence failure"
                );
                return Err(CodexErr::Fatal(
                    "failed to persist agent lifecycle state and could not confirm agent shutdown"
                        .to_string(),
                ));
            }
            if let Err(err) = finalize_rejected_spawn_edge(&state, new_thread.thread_id).await {
                if let Some(reservation) = reservation.take() {
                    reservation.commit(agent_metadata.clone());
                }
                if let Some(residency_slot) = residency_slot.take() {
                    residency_slot.commit(new_thread.thread_id);
                }
                self.start_late_termination_cleanup(
                    Arc::clone(&state),
                    Arc::clone(&new_thread.thread),
                    TerminatedThreadCleanup::FailedSpawnRollback,
                );
                warn!(
                    %err,
                    thread_id = %new_thread.thread_id,
                    "failed to finalize provisional edge after lifecycle persistence failure"
                );
                return Err(CodexErr::Fatal(
                    "failed to persist agent lifecycle state and graph repair is required"
                        .to_string(),
                ));
            }
            let _ = state
                .remove_thread_if_same_or_missing(&new_thread.thread_id, &new_thread.thread)
                .await;
            return Err(persist_err);
        }
        #[cfg(test)]
        let reject_initial_input = self
            .fail_next_initial_input
            .swap(false, std::sync::atomic::Ordering::AcqRel);
        #[cfg(not(test))]
        let reject_initial_input = false;
        let initial_task_message = match &initial_input {
            SpawnInitialInput::UserInput(input) => {
                non_empty_task_message(render_input_preview(input))
            }
            SpawnInitialInput::InterAgentCommunication(communication, _) => {
                last_task_message_from_communication(communication)
            }
        };
        let initial_input_result = if reject_initial_input {
            Err(CodexErr::Fatal(
                "injected initial input failure".to_string(),
            ))
        } else {
            match initial_input {
                SpawnInitialInput::UserInput(input) => {
                    self.send_input_after_capacity_check(new_thread.thread_id, &state, input)
                        .await
                }
                SpawnInitialInput::InterAgentCommunication(communication, context) => {
                    self.send_inter_agent_communication_after_capacity_check(
                        new_thread.thread_id,
                        &state,
                        communication,
                        context,
                    )
                    .await
                }
            }
        };
        if initial_input_result.is_ok() {
            agent_metadata.last_task_message = initial_task_message;
        }
        let mut submission_loop_aborted = false;
        let mut activation_result = match initial_input_result {
            Ok(_) => {
                self.persist_thread_spawn_edge_for_source(
                    new_thread.thread.as_ref(),
                    new_thread.thread_id,
                    notification_source.as_ref(),
                    codex_agent_graph_store::ThreadSpawnEdgeStatus::Open,
                )
                .await
            }
            Err(err) => Err(err),
        };
        if activation_result.is_ok() {
            if let Some(reservation) = reservation.take() {
                reservation.commit(agent_metadata.clone());
            }
            if let Some(residency_slot) = residency_slot.take() {
                residency_slot.commit(new_thread.thread_id);
            }
            #[cfg(test)]
            let force_commit_failure = self.pause_before_submission_loop_commit_for_test().await;
            #[cfg(not(test))]
            let force_commit_failure = false;
            if let Some(activation_tx) = submission_loop_activation_tx.take()
                && (force_commit_failure
                    || activation_tx
                        .send(SubmissionLoopActivation::Commit)
                        .is_err())
            {
                activation_result = Err(CodexErr::InternalAgentDied);
                submission_loop_aborted = true;
            }
        } else if let Some(activation_tx) = submission_loop_activation_tx.take() {
            let _ = activation_tx.send(SubmissionLoopActivation::Abort);
            submission_loop_aborted = true;
        }
        if let Err(input_err) = activation_result {
            let rollback_result = if submission_loop_aborted {
                self.wait_for_thread_termination_for_lifecycle(
                    &state,
                    Arc::clone(&new_thread.thread),
                    TerminatedThreadCleanup::FailedSpawnRollback,
                )
                .await
            } else {
                self.shutdown_live_agent_with_cleanup(
                    new_thread.thread_id,
                    TerminatedThreadCleanup::FailedSpawnRollback,
                )
                .await
                .map(|_| ())
            };
            if let Err(err) = rollback_result {
                if let Some(reservation) = reservation.take() {
                    reservation.commit(agent_metadata.clone());
                }
                if let Some(residency_slot) = residency_slot.take() {
                    residency_slot.commit(new_thread.thread_id);
                }
                warn!(%err, thread_id = %new_thread.thread_id, "failed to stop rejected agent");
                if let Err(quarantine_err) =
                    quarantine_rejected_spawn_edge(&state, new_thread.thread_id).await
                {
                    warn!(
                        %quarantine_err,
                        thread_id = %new_thread.thread_id,
                        "failed to quarantine rejected agent lifecycle edge"
                    );
                    return Err(CodexErr::Fatal(
                        "agent rejected its initial input; runtime shutdown was not confirmed and lifecycle graph repair is required"
                            .to_string(),
                    ));
                }
                return Err(CodexErr::Fatal(
                    "agent rejected its initial input and lifecycle rollback was incomplete"
                        .to_string(),
                ));
            }
            if let Err(err) = finalize_rejected_spawn_edge(&state, new_thread.thread_id).await {
                if let Some(reservation) = reservation.take() {
                    reservation.commit(agent_metadata.clone());
                }
                if let Some(residency_slot) = residency_slot.take() {
                    residency_slot.commit(new_thread.thread_id);
                }
                warn!(%err, thread_id = %new_thread.thread_id, "failed to finalize rejected agent edge");
                self.start_late_termination_cleanup(
                    Arc::clone(&state),
                    Arc::clone(&new_thread.thread),
                    TerminatedThreadCleanup::FailedSpawnRollback,
                );
                return Err(CodexErr::Fatal(
                    "agent rejected its initial input and lifecycle graph repair is required"
                        .to_string(),
                ));
            }
            if !state
                .remove_thread_if_same_or_missing(&new_thread.thread_id, &new_thread.thread)
                .await
            {
                if let Some(reservation) = reservation.take() {
                    reservation.commit(agent_metadata.clone());
                }
                if let Some(residency_slot) = residency_slot.take() {
                    residency_slot.commit(new_thread.thread_id);
                }
                return Err(CodexErr::Fatal(
                    "agent rejected its initial input but its runtime was replaced during cleanup"
                        .to_string(),
                ));
            }
            self.forget_v2_residency(new_thread.thread_id);
            self.state.release_spawned_thread(new_thread.thread_id);
            self.lifecycle.finish_late_cleanup(new_thread.thread_id);
            return Err(input_err);
        }
        new_thread.thread.publish_initial_task();
        self.emit_spawn_started_analytics(&state, &new_thread, notification_source.as_ref())
            .await;

        // Notify clients only after the initial task is queued and the graph edge is durably open.
        state.notify_thread_created(new_thread.thread_id);

        Ok(LiveAgent {
            thread_id: new_thread.thread_id,
            metadata: agent_metadata,
            status: self.get_status(new_thread.thread_id).await,
        })
    }

    async fn emit_spawn_started_analytics(
        &self,
        state: &Arc<ThreadManagerState>,
        new_thread: &crate::thread_manager::NewThread,
        notification_source: Option<&SessionSource>,
    ) {
        let Some(SessionSource::SubAgent(
            subagent_source @ SubAgentSource::ThreadSpawn {
                parent_thread_id, ..
            },
        )) = notification_source
        else {
            return;
        };
        let client_metadata = match state.get_thread(*parent_thread_id).await {
            Ok(parent_thread) => {
                parent_thread
                    .codex
                    .session
                    .app_server_client_metadata()
                    .await
            }
            Err(error) => {
                tracing::warn!(
                    error = %error,
                    parent_thread_id = %parent_thread_id,
                    "skipping subagent thread analytics: failed to load parent thread metadata"
                );
                crate::session::session::AppServerClientMetadata {
                    client_name: None,
                    client_version: None,
                }
            }
        };
        let thread_config = new_thread.thread.codex.thread_config_snapshot().await;
        let parent_thread_id = thread_config.parent_thread_id;
        emit_subagent_session_started(
            &new_thread
                .thread
                .codex
                .session
                .services
                .analytics_events_client,
            client_metadata,
            new_thread.thread.codex.session.session_id(),
            new_thread.thread_id,
            parent_thread_id,
            thread_config,
            subagent_source.clone(),
        );
    }

    #[allow(clippy::too_many_arguments)]
    async fn spawn_forked_thread(
        &self,
        state: &Arc<ThreadManagerState>,
        config: Config,
        session_source: SessionSource,
        options: &SpawnAgentOptions,
        inheritance: SpawnAgentThreadInheritance,
        multi_agent_version: MultiAgentVersion,
        submission_loop_activation: Option<
            tokio::sync::oneshot::Receiver<SubmissionLoopActivation>,
        >,
    ) -> CodexResult<crate::thread_manager::NewThread> {
        let SpawnAgentThreadInheritance {
            environments: inherited_environments,
            exec_policy: inherited_exec_policy,
        } = inheritance;
        if options.fork_parent_spawn_call_id.is_none() {
            return Err(CodexErr::Fatal(
                "spawn_agent fork requires a parent spawn call id".to_string(),
            ));
        }
        let Some(fork_mode) = options.fork_mode.as_ref() else {
            return Err(CodexErr::Fatal(
                "spawn_agent fork requires a fork mode".to_string(),
            ));
        };
        let SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id, ..
        }) = &session_source
        else {
            return Err(CodexErr::Fatal(
                "spawn_agent fork requires a thread-spawn session source".to_string(),
            ));
        };

        let parent_thread_id = *parent_thread_id;
        let parent_thread = state.get_thread(parent_thread_id).await.ok();
        if let Some(parent_thread) = parent_thread.as_ref() {
            // `record_conversation_items` only queues persistence writes asynchronously.
            // Flush before snapshotting store history for a fork.
            parent_thread.ensure_rollout_materialized().await;
            parent_thread.flush_rollout().await?;
        }

        let parent_history = state
            .read_stored_thread(ReadThreadParams {
                thread_id: parent_thread_id,
                include_archived: true,
                include_history: true,
            })
            .await?
            .history
            .ok_or_else(|| {
                CodexErr::Fatal(format!(
                    "parent thread history unavailable for fork: {parent_thread_id}"
                ))
            })?;

        let selected_capability_roots = parent_history
            .items
            .iter()
            .find_map(|item| {
                let RolloutItem::SessionMeta(meta_line) = item else {
                    return None;
                };
                Some(meta_line.meta.selected_capability_roots.clone())
            })
            .unwrap_or_default();
        let mut forked_rollout_items = parent_history.items;
        if let SpawnAgentForkMode::LastNTurns(last_n_turns) = fork_mode {
            forked_rollout_items =
                truncate_rollout_to_last_n_fork_turns(&forked_rollout_items, *last_n_turns);
        }
        let multi_agent_v2_usage_hint_texts_to_filter: Vec<String> =
            if let Some(parent_thread) = parent_thread.as_ref() {
                if multi_agent_version == MultiAgentVersion::V2 {
                    let parent_config = parent_thread.codex.session.get_config().await;
                    [
                        parent_config
                            .multi_agent_v2
                            .root_agent_usage_hint_text
                            .clone(),
                        parent_config
                            .multi_agent_v2
                            .subagent_usage_hint_text
                            .clone(),
                    ]
                    .into_iter()
                    .flatten()
                    .collect()
                } else {
                    Vec::new()
                }
            } else if multi_agent_version == MultiAgentVersion::V2 {
                [
                    config.multi_agent_v2.root_agent_usage_hint_text.clone(),
                    config.multi_agent_v2.subagent_usage_hint_text.clone(),
                ]
                .into_iter()
                .flatten()
                .collect()
            } else {
                Vec::new()
            };
        let preserve_reference_context_item = matches!(fork_mode, SpawnAgentForkMode::FullHistory);
        forked_rollout_items.retain(|item| {
            keep_forked_rollout_item(item, preserve_reference_context_item)
                && !matches!(
                    item,
                    RolloutItem::ResponseItem(response_item)
                        if is_multi_agent_v2_usage_hint_message(
                            response_item,
                            &multi_agent_v2_usage_hint_texts_to_filter,
                        )
                )
        });
        for item in &mut forked_rollout_items {
            if let RolloutItem::Compacted(compacted) = item
                && let Some(replacement_history) = compacted.replacement_history.as_mut()
            {
                replacement_history.retain(|response_item| {
                    !is_multi_agent_v2_usage_hint_message(
                        response_item,
                        &multi_agent_v2_usage_hint_texts_to_filter,
                    )
                });
            }
        }
        if preserve_reference_context_item
            && multi_agent_version == MultiAgentVersion::V2
            && let Some(subagent_usage_hint_text) =
                config.multi_agent_v2.subagent_usage_hint_text.clone()
            && let Some(subagent_usage_hint_message) =
                crate::context_manager::updates::build_developer_update_item(vec![
                    subagent_usage_hint_text,
                ])
        {
            forked_rollout_items.push(RolloutItem::ResponseItem(subagent_usage_hint_message));
        }
        let mut thread_extension_init = ExtensionDataInit::new();
        thread_extension_init.insert(selected_capability_roots);

        state
            .fork_thread_with_source(
                config.clone(),
                InitialHistory::Forked(forked_rollout_items),
                self.clone(),
                session_source,
                /*thread_source*/ Some(ThreadSource::Subagent),
                /*parent_thread_id*/ Some(parent_thread_id),
                /*forked_from_thread_id*/ Some(parent_thread_id),
                inherited_environments,
                inherited_exec_policy,
                options.environments.clone(),
                thread_extension_init,
                submission_loop_activation,
            )
            .await
    }
}
