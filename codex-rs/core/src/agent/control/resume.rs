use super::spawn::load_agent_model_context;
use super::target_version::require_v1_runtime;
use super::*;
use crate::thread_manager::normalize_multi_agent_runtime_intent;
use std::collections::HashSet;

struct ResumedAgent {
    thread: crate::thread_manager::NewThread,
    multi_agent_version: MultiAgentVersion,
    multi_agent_runtime: MultiAgentRuntimeIntent,
}

struct LockedResumedAgent {
    resumed: ResumedAgent,
    lifecycle_guard: crate::thread_manager::ThreadLifecycleGuard,
}

impl AgentControl {
    /// Resume an existing agent thread from a recorded rollout file.
    pub(crate) async fn resume_agent_from_rollout(
        &self,
        config: Config,
        thread_id: ThreadId,
        session_source: SessionSource,
        options: ResumeAgentOptions,
    ) -> CodexResult<ThreadId> {
        let resumed = Box::pin(self.resume_agent_from_rollout_with_runtime(
            config,
            thread_id,
            session_source,
            options,
            MultiAgentRuntimeIntent::Inherit,
        ))
        .await?;
        Ok(resumed.thread.thread_id)
    }

    pub(crate) async fn resume_projected_v1_agent_from_rollout(
        &self,
        config: Config,
        thread_id: ThreadId,
        session_source: SessionSource,
        options: ResumeAgentOptions,
    ) -> CodexResult<target_version::ValidatedV1Thread> {
        self.validate_v1_lifecycle_target(thread_id).await?;
        let resumed = Box::pin(self.resume_agent_from_rollout_with_runtime(
            config,
            thread_id,
            session_source,
            options,
            MultiAgentRuntimeIntent::ExactV1Resume,
        ))
        .await?;
        require_v1_runtime(thread_id, Some(resumed.multi_agent_version))?;
        Ok(target_version::ValidatedV1Thread::new(
            resumed.thread.thread_id,
            resumed.thread.thread,
        ))
    }

    async fn resume_agent_from_rollout_with_runtime(
        &self,
        config: Config,
        thread_id: ThreadId,
        session_source: SessionSource,
        options: ResumeAgentOptions,
        multi_agent_runtime: MultiAgentRuntimeIntent,
    ) -> CodexResult<ResumedAgent> {
        let root_depth = thread_spawn_depth(&session_source).unwrap_or(0);
        let LockedResumedAgent {
            resumed: resumed_agent,
            lifecycle_guard: root_lifecycle_guard,
        } = Box::pin(self.resume_single_agent_from_rollout(
            config.clone(),
            thread_id,
            session_source,
            &options,
            multi_agent_runtime,
        ))
        .await?;
        let state = self.upgrade()?;
        if resumed_agent.multi_agent_version == MultiAgentVersion::V2 {
            drop(root_lifecycle_guard);
            return Ok(resumed_agent);
        }
        let Some(agent_graph_store) = state.agent_graph_store() else {
            drop(root_lifecycle_guard);
            return Ok(resumed_agent);
        };
        debug_assert_eq!(root_lifecycle_guard.thread_id(), thread_id);

        let effective_multi_agent_runtime = resumed_agent.multi_agent_runtime;
        let mut resume_queue = VecDeque::from([(
            thread_id,
            root_depth,
            Arc::clone(&resumed_agent.thread.thread),
        )]);
        let mut visited = HashSet::from([thread_id]);
        while let Some((parent_thread_id, parent_depth, parent_thread)) = resume_queue.pop_front() {
            let child_ids = if parent_thread_id == thread_id {
                let Ok(current_parent) = state.get_thread(parent_thread_id).await else {
                    continue;
                };
                if !Arc::ptr_eq(&current_parent, &parent_thread) {
                    warn!("skipping descendants for replaced thread runtime {parent_thread_id}");
                    continue;
                }
                match agent_graph_store
                    .list_thread_spawn_children(
                        parent_thread_id,
                        Some(codex_agent_graph_store::ThreadSpawnEdgeStatus::Open),
                    )
                    .await
                {
                    Ok(child_ids) => child_ids,
                    Err(err) => {
                        warn!(
                            "failed to load persisted thread-spawn children for {parent_thread_id}: {err}"
                        );
                        continue;
                    }
                }
            } else {
                let parent_lifecycle_guard = state.lock_thread_lifecycle(parent_thread_id).await;
                debug_assert_eq!(parent_lifecycle_guard.thread_id(), parent_thread_id);
                let Ok(current_parent) = state.get_thread(parent_thread_id).await else {
                    continue;
                };
                if !Arc::ptr_eq(&current_parent, &parent_thread) {
                    warn!("skipping descendants for replaced thread runtime {parent_thread_id}");
                    continue;
                }
                let child_ids = match agent_graph_store
                    .list_thread_spawn_children(
                        parent_thread_id,
                        Some(codex_agent_graph_store::ThreadSpawnEdgeStatus::Open),
                    )
                    .await
                {
                    Ok(child_ids) => child_ids,
                    Err(err) => {
                        warn!(
                            "failed to load persisted thread-spawn children for {parent_thread_id}: {err}"
                        );
                        continue;
                    }
                };
                drop(parent_lifecycle_guard);
                child_ids
            };

            for child_thread_id in child_ids {
                if !visited.insert(child_thread_id) {
                    return Err(CodexErr::InvalidRequest(format!(
                        "cyclic thread-spawn graph encountered while resuming {thread_id}: repeated thread {child_thread_id}"
                    )));
                }
                let child_depth = parent_depth + 1;
                let child_lifecycle_guard = state.lock_thread_lifecycle(child_thread_id).await;
                debug_assert_eq!(child_lifecycle_guard.thread_id(), child_thread_id);
                let child_thread = if let Ok(child_thread) = state.get_thread(child_thread_id).await
                {
                    let child_thread = if effective_multi_agent_runtime
                        == MultiAgentRuntimeIntent::ExactV1Resume
                        && child_thread.multi_agent_version() != Some(MultiAgentVersion::V1)
                    {
                        warn!(
                            "skipping non-V1 descendant thread {child_thread_id}: resolved runtime {:?}",
                            child_thread.multi_agent_version()
                        );
                        None
                    } else {
                        Some(child_thread)
                    };
                    drop(child_lifecycle_guard);
                    child_thread
                } else {
                    let child_session_source =
                        SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                            parent_thread_id,
                            depth: child_depth,
                            agent_path: None,
                            agent_nickname: None,
                            agent_role: None,
                        });
                    match Box::pin(self.resume_single_agent_from_rollout_locked(
                        Arc::clone(&state),
                        child_lifecycle_guard,
                        config.clone(),
                        child_thread_id,
                        child_session_source,
                        &options,
                        effective_multi_agent_runtime,
                    ))
                    .await
                    {
                        Ok(locked_resumed_child)
                            if effective_multi_agent_runtime
                                == MultiAgentRuntimeIntent::ExactV1Resume
                                && locked_resumed_child.resumed.multi_agent_version
                                    == MultiAgentVersion::V1 =>
                        {
                            let LockedResumedAgent {
                                resumed: resumed_child,
                                lifecycle_guard,
                            } = locked_resumed_child;
                            let child_thread = resumed_child.thread.thread;
                            drop(lifecycle_guard);
                            Some(child_thread)
                        }
                        Ok(locked_resumed_child)
                            if effective_multi_agent_runtime
                                != MultiAgentRuntimeIntent::ExactV1Resume =>
                        {
                            let LockedResumedAgent {
                                resumed: resumed_child,
                                lifecycle_guard,
                            } = locked_resumed_child;
                            let child_thread = resumed_child.thread.thread;
                            drop(lifecycle_guard);
                            Some(child_thread)
                        }
                        Ok(locked_resumed_child) => {
                            warn!(
                                "skipping non-V1 descendant thread {child_thread_id}: resolved runtime {:?}",
                                locked_resumed_child.resumed.multi_agent_version
                            );
                            None
                        }
                        Err(err) => {
                            warn!("failed to resume descendant thread {child_thread_id}: {err}");
                            None
                        }
                    }
                };
                if let Some(child_thread) = child_thread {
                    resume_queue.push_back((child_thread_id, child_depth, child_thread));
                }
            }
        }

        drop(root_lifecycle_guard);
        Ok(resumed_agent)
    }

    async fn resume_single_agent_from_rollout(
        &self,
        config: Config,
        thread_id: ThreadId,
        session_source: SessionSource,
        options: &ResumeAgentOptions,
        multi_agent_runtime: MultiAgentRuntimeIntent,
    ) -> CodexResult<LockedResumedAgent> {
        let state = self.upgrade()?;
        let lifecycle_guard = state.lock_thread_lifecycle(thread_id).await;
        debug_assert_eq!(lifecycle_guard.thread_id(), thread_id);
        self.resume_single_agent_from_rollout_locked(
            state,
            lifecycle_guard,
            config,
            thread_id,
            session_source,
            options,
            multi_agent_runtime,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn resume_single_agent_from_rollout_locked(
        &self,
        state: Arc<ThreadManagerState>,
        lifecycle_guard: crate::thread_manager::ThreadLifecycleGuard,
        config: Config,
        thread_id: ThreadId,
        session_source: SessionSource,
        options: &ResumeAgentOptions,
        multi_agent_runtime: MultiAgentRuntimeIntent,
    ) -> CodexResult<LockedResumedAgent> {
        debug_assert_eq!(lifecycle_guard.thread_id(), thread_id);
        let stored_thread = state
            .read_stored_thread(ReadThreadParams {
                thread_id,
                include_archived: true,
                include_history: false,
            })
            .await?;
        let resumed_agent_path = stored_thread
            .agent_path
            .as_deref()
            .map(AgentPath::try_from)
            .transpose()
            .map_err(|err| CodexErr::InvalidRequest(format!("invalid stored agent path: {err}")))?;
        let resumed_agent_nickname = stored_thread.agent_nickname.clone();
        let resumed_agent_role = stored_thread.agent_role.clone();
        let history = load_agent_model_context(&state, thread_id, stored_thread.history_mode)
            .await?
            .ok_or(CodexErr::ThreadNotFound(thread_id))?;
        let initial_history = InitialHistory::Resumed(ResumedHistory {
            conversation_id: thread_id,
            history: Arc::new(history),
            rollout_path: stored_thread.rollout_path,
        });
        let parent_thread_id = session_source
            .parent_thread_id()
            .or(stored_thread.parent_thread_id);
        let resumed_thread_source = initial_history.get_resumed_thread_source();
        let multi_agent_runtime = normalize_multi_agent_runtime_intent(
            multi_agent_runtime,
            &initial_history,
            Some(stored_thread.history_mode),
            &session_source,
            parent_thread_id,
            /*forked_from_thread_id*/ None,
            resumed_thread_source.as_ref(),
            &config,
        )?;
        if multi_agent_runtime == MultiAgentRuntimeIntent::ExactV1Resume {
            require_v1_runtime(
                thread_id,
                resolve_persisted_multi_agent_version_for_exact_v1(
                    &initial_history,
                    stored_thread.history_mode,
                ),
            )?;
        }
        let multi_agent_version = state
            .effective_multi_agent_version_for_spawn(
                &initial_history,
                Some(&session_source),
                parent_thread_id,
                /*forked_from_thread_id*/ None,
                &config,
                multi_agent_runtime,
            )
            .await;
        let agent_max_threads = config.effective_agent_max_threads(multi_agent_version);
        let mut reservation = self.state.reserve_spawn_slot(agent_max_threads)?;
        let (session_source, agent_metadata) = match session_source {
            SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth,
                agent_path,
                agent_role: _,
                agent_nickname: _,
            }) => self.prepare_thread_spawn(
                &mut reservation,
                &config,
                parent_thread_id,
                depth,
                agent_path.or(resumed_agent_path),
                resumed_agent_role,
                resumed_agent_nickname,
            )?,
            other => (other, AgentMetadata::default()),
        };
        let notification_source = session_source.clone();
        let inherited_environments = self
            .inherited_environments_for_source(&state, Some(&session_source))
            .await;
        let inherited_exec_policy = self
            .inherited_exec_policy_for_source(&state, Some(&session_source), &config)
            .await;

        let resumed_thread = state
            .resume_thread_with_history_with_source(ResumeThreadWithHistoryOptions {
                config: config.clone(),
                initial_history,
                history_mode: stored_thread.history_mode,
                agent_control: self.clone(),
                session_source,
                parent_thread_id,
                inherited_environments,
                inherited_exec_policy,
                environment_selections: options.environment_selections.clone(),
                multi_agent_runtime,
            })
            .await?;
        let mut agent_metadata = agent_metadata;
        agent_metadata.agent_id = Some(resumed_thread.thread_id);
        reservation.commit(agent_metadata.clone());
        // Resumed threads are re-registered in-memory and need the same listener
        // attachment path as freshly spawned threads.
        state.notify_thread_created(resumed_thread.thread_id);
        if multi_agent_version != MultiAgentVersion::V2 {
            let child_reference = agent_metadata
                .agent_path
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| resumed_thread.thread_id.to_string());
            self.maybe_start_completion_watcher(
                resumed_thread.thread_id,
                Some(Arc::clone(&resumed_thread.thread)),
                Some(notification_source.clone()),
                child_reference,
                agent_metadata.agent_path.clone(),
            );
        }
        self.persist_thread_spawn_edge_for_source(
            resumed_thread.thread.as_ref(),
            resumed_thread.thread_id,
            Some(&notification_source),
        )
        .await;

        Ok(LockedResumedAgent {
            resumed: ResumedAgent {
                thread: resumed_thread,
                multi_agent_version,
                multi_agent_runtime,
            },
            lifecycle_guard,
        })
    }
}
