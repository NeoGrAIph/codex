use super::residency::is_v2_resident_session_source;
use super::*;
use codex_thread_store::ReadThreadParams;
use codex_thread_store::ThreadMetadataPatch;

mod entry;

enum ResumedAgentSource {
    ThreadSpawn {
        parent_thread_id: ThreadId,
        depth: i32,
        agent_path: Option<AgentPath>,
        agent_role: Option<String>,
        agent_nickname: Option<String>,
        had_persisted_agent_path: bool,
    },
    Legacy(SessionSource),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ResumeLineageAuthority {
    PersistedMetadata,
    PersistedGraphEdge,
}

impl AgentControl {
    pub(super) async fn validate_persisted_v2_agent_path(
        &self,
        state: &Arc<ThreadManagerState>,
        thread_id: ThreadId,
        parent_thread_id: ThreadId,
        depth: i32,
        agent_path: &AgentPath,
    ) -> CodexResult<()> {
        let parent_agent_path = self
            .persisted_v2_parent_agent_path(state, thread_id, parent_thread_id, depth)
            .await?;
        validate_agent_path_against_parent(thread_id, depth, agent_path, &parent_agent_path)
    }

    async fn persisted_v2_parent_agent_path(
        &self,
        state: &Arc<ThreadManagerState>,
        thread_id: ThreadId,
        parent_thread_id: ThreadId,
        depth: i32,
    ) -> CodexResult<AgentPath> {
        if depth == 1 {
            return Ok(AgentPath::root());
        }
        let parent_agent_path = if let Some(live_parent_agent_path) = self
            .state
            .agent_metadata_for_thread(parent_thread_id)
            .and_then(|metadata| metadata.agent_path)
        {
            live_parent_agent_path
        } else {
            let stored_parent = state
                .read_stored_thread(ReadThreadParams {
                    thread_id: parent_thread_id,
                    include_archived: true,
                    include_history: false,
                })
                .await?;
            let stored_parent_agent_path = stored_parent
                .agent_path
                .as_deref()
                .map(AgentPath::try_from)
                .transpose()
                .map_err(|err| {
                    CodexErr::InvalidRequest(format!(
                        "invalid stored parent agent path for {thread_id}: {err}"
                    ))
                })?;
            let source_parent_agent_path = stored_parent.source.get_agent_path();
            if let (Some(stored_parent_agent_path), Some(source_parent_agent_path)) = (
                stored_parent_agent_path.as_ref(),
                source_parent_agent_path.as_ref(),
            ) && stored_parent_agent_path != source_parent_agent_path
            {
                return Err(CodexErr::InvalidRequest(format!(
                    "stored parent agent path is inconsistent for thread {thread_id}"
                )));
            }
            stored_parent_agent_path
                .or(source_parent_agent_path)
                .ok_or_else(|| {
                    CodexErr::InvalidRequest(format!(
                        "stored parent {parent_thread_id} is missing a canonical agent path"
                    ))
                })?
        };
        if parent_agent_path.is_root() {
            return Err(CodexErr::InvalidRequest(format!(
                "stored parent {parent_thread_id} has a root path at nested depth {depth}"
            )));
        }
        Ok(parent_agent_path)
    }

    pub(super) async fn resume_single_agent_from_rollout(
        &self,
        mut config: Config,
        thread_id: ThreadId,
        requested_session_source: SessionSource,
        lineage_authority: ResumeLineageAuthority,
    ) -> CodexResult<(ThreadId, MultiAgentVersion)> {
        let state = self.upgrade()?;
        let mut stored_thread = state
            .read_stored_thread(ReadThreadParams {
                thread_id,
                include_archived: true,
                include_history: true,
            })
            .await?;
        let requested_task_name = requested_session_source
            .get_agent_path()
            .map(|path| path.name().to_string());
        if lineage_authority == ResumeLineageAuthority::PersistedGraphEdge
            && let SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth,
                agent_role,
                agent_nickname,
                ..
            }) = &requested_session_source
        {
            let (persisted_agent_path, persisted_agent_role, persisted_agent_nickname) =
                match &stored_thread.source {
                    SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                        agent_path,
                        agent_role,
                        agent_nickname,
                        ..
                    }) => (
                        agent_path.clone(),
                        agent_role.clone(),
                        agent_nickname.clone(),
                    ),
                    _ => (None, None, None),
                };
            stored_thread.parent_thread_id = Some(*parent_thread_id);
            stored_thread.source = SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: *parent_thread_id,
                depth: *depth,
                agent_path: persisted_agent_path,
                agent_role: stored_thread
                    .agent_role
                    .clone()
                    .or(persisted_agent_role)
                    .or_else(|| agent_role.clone()),
                agent_nickname: stored_thread
                    .agent_nickname
                    .clone()
                    .or(persisted_agent_nickname)
                    .or_else(|| agent_nickname.clone()),
            });
        }
        stored_thread
            .project_identity_into_history()
            .map_err(|err| {
                CodexErr::InvalidRequest(format!("invalid stored agent identity: {err}"))
            })?;
        let stored_source = stored_thread.source.clone();
        let needs_thread_spawn_identity = matches!(
            &stored_source,
            SessionSource::SubAgent(SubAgentSource::ThreadSpawn { .. })
        ) || matches!(
            &requested_session_source,
            SessionSource::SubAgent(SubAgentSource::ThreadSpawn { .. })
        );
        let stored_agent_path = needs_thread_spawn_identity
            .then(|| {
                stored_thread
                    .agent_path
                    .as_deref()
                    .map(AgentPath::try_from)
                    .transpose()
            })
            .transpose()
            .map_err(|err| CodexErr::InvalidRequest(format!("invalid stored agent path: {err}")))?
            .flatten();
        let mut resumed_source = match stored_source {
            SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth,
                agent_path: source_agent_path,
                agent_role,
                agent_nickname,
            }) => {
                if let Some(metadata_parent_thread_id) = stored_thread.parent_thread_id
                    && metadata_parent_thread_id != parent_thread_id
                {
                    return Err(CodexErr::InvalidRequest(format!(
                        "stored agent lineage is inconsistent for thread {thread_id}: source parent {parent_thread_id} does not match metadata parent {metadata_parent_thread_id}"
                    )));
                }
                if let (Some(stored_agent_path), Some(source_agent_path)) =
                    (stored_agent_path.as_ref(), source_agent_path.as_ref())
                    && stored_agent_path != source_agent_path
                {
                    return Err(CodexErr::InvalidRequest(format!(
                        "stored agent path is inconsistent for thread {thread_id}"
                    )));
                }
                let agent_path = stored_agent_path.or(source_agent_path);
                let had_persisted_agent_path = agent_path.is_some();
                ResumedAgentSource::ThreadSpawn {
                    parent_thread_id,
                    depth,
                    agent_path,
                    agent_role: stored_thread.agent_role.clone().or(agent_role),
                    agent_nickname: stored_thread.agent_nickname.clone().or(agent_nickname),
                    had_persisted_agent_path,
                }
            }
            SessionSource::Unknown => match requested_session_source.clone() {
                SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                    parent_thread_id,
                    depth,
                    agent_path: _,
                    agent_role,
                    agent_nickname,
                }) => {
                    let had_persisted_agent_path = stored_agent_path.is_some();
                    ResumedAgentSource::ThreadSpawn {
                        parent_thread_id: stored_thread
                            .parent_thread_id
                            .unwrap_or(parent_thread_id),
                        depth,
                        agent_path: stored_agent_path,
                        agent_role: stored_thread.agent_role.clone().or(agent_role),
                        agent_nickname: stored_thread.agent_nickname.clone().or(agent_nickname),
                        had_persisted_agent_path,
                    }
                }
                source => ResumedAgentSource::Legacy(source),
            },
            _stored_non_agent_source => match requested_session_source.clone() {
                SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                    parent_thread_id,
                    depth,
                    agent_path: _,
                    agent_role,
                    agent_nickname,
                }) => {
                    let had_persisted_agent_path = stored_agent_path.is_some();
                    ResumedAgentSource::ThreadSpawn {
                        parent_thread_id: stored_thread
                            .parent_thread_id
                            .unwrap_or(parent_thread_id),
                        depth,
                        agent_path: stored_agent_path,
                        agent_role: stored_thread.agent_role.clone().or(agent_role),
                        agent_nickname: stored_thread.agent_nickname.clone().or(agent_nickname),
                        had_persisted_agent_path,
                    }
                }
                source => ResumedAgentSource::Legacy(source),
            },
        };
        if let ResumedAgentSource::ThreadSpawn {
            parent_thread_id,
            depth,
            ..
        } = &resumed_source
        {
            if *parent_thread_id == thread_id {
                return Err(CodexErr::InvalidRequest(format!(
                    "stored agent lineage for thread {thread_id} points to itself as parent"
                )));
            }
            if *depth < 1 {
                return Err(CodexErr::InvalidRequest(format!(
                    "stored agent lineage for thread {thread_id} has invalid depth {depth}"
                )));
            }
        }
        if let Some(model) = stored_thread.model.clone() {
            config.model = Some(model);
        }
        if let Some(reasoning_effort) = stored_thread.reasoning_effort.clone() {
            config.model_reasoning_effort = Some(reasoning_effort);
        }
        let history = stored_thread
            .history
            .ok_or_else(|| CodexErr::ThreadNotFound(thread_id))?
            .items;
        let mut initial_history = InitialHistory::Resumed(ResumedHistory {
            conversation_id: thread_id,
            history: Arc::new(history),
            rollout_path: stored_thread.rollout_path,
        });
        let (parent_thread_id, source_for_version) = match &resumed_source {
            ResumedAgentSource::ThreadSpawn {
                parent_thread_id,
                depth,
                agent_path,
                agent_role,
                agent_nickname,
                ..
            } => (
                Some(*parent_thread_id),
                SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                    parent_thread_id: *parent_thread_id,
                    depth: *depth,
                    agent_path: agent_path.clone(),
                    agent_nickname: agent_nickname.clone(),
                    agent_role: agent_role.clone(),
                }),
            ),
            ResumedAgentSource::Legacy(source) => (stored_thread.parent_thread_id, source.clone()),
        };
        let multi_agent_version = state
            .effective_multi_agent_version_for_spawn(
                &initial_history,
                Some(&source_for_version),
                parent_thread_id,
                /*forked_from_thread_id*/ None,
                &config,
            )
            .await;
        if multi_agent_version == MultiAgentVersion::V2
            && let ResumedAgentSource::ThreadSpawn {
                parent_thread_id: stored_source_parent_thread_id,
                depth,
                agent_path,
                ..
            } = &mut resumed_source
        {
            let parent_agent_path = self
                .persisted_v2_parent_agent_path(
                    &state,
                    thread_id,
                    *stored_source_parent_thread_id,
                    *depth,
                )
                .await?;
            if let Some(existing_agent_path) = agent_path.as_ref() {
                validate_agent_path_against_parent(
                    thread_id,
                    *depth,
                    existing_agent_path,
                    &parent_agent_path,
                )?;
            } else {
                let task_name = requested_task_name.as_deref().ok_or_else(|| {
                    CodexErr::InvalidRequest(format!(
                        "stored V2 agent {thread_id} is missing a canonical path"
                    ))
                })?;
                *agent_path = Some(parent_agent_path.join(task_name).map_err(|err| {
                    CodexErr::InvalidRequest(format!(
                        "failed to backfill agent path for {thread_id}: {err}"
                    ))
                })?);
            }
        }
        let resume_uses_v2_residency = multi_agent_version == MultiAgentVersion::V2
            && is_v2_resident_session_source(&source_for_version);
        let mut residency_slot = if resume_uses_v2_residency {
            Some(
                self.reserve_v2_residency_slot(&state, &config, Some(thread_id))
                    .await?,
            )
        } else {
            None
        };
        let agent_max_threads = config.effective_agent_max_threads(multi_agent_version);
        let reservation_max_threads = if resume_uses_v2_residency {
            None
        } else {
            agent_max_threads
        };
        let registered_agent_metadata = self.state.agent_metadata_for_thread(thread_id);
        let reactivating_registered_agent = registered_agent_metadata.is_some();
        let resume_failure_cleanup = if reactivating_registered_agent {
            TerminatedThreadCleanup::UnloadResident
        } else {
            TerminatedThreadCleanup::ReleaseAgent
        };
        let mut registration_reservation = if reactivating_registered_agent {
            None
        } else {
            Some(self.state.reserve_spawn_slot(reservation_max_threads)?)
        };
        let (session_source, agent_metadata, had_persisted_agent_path) = match (
            resumed_source,
            registered_agent_metadata.as_ref(),
        ) {
            (
                ResumedAgentSource::ThreadSpawn {
                    parent_thread_id,
                    depth,
                    agent_path,
                    agent_role,
                    agent_nickname,
                    had_persisted_agent_path,
                },
                Some(registered_agent_metadata),
            ) => {
                if registered_agent_metadata.agent_path != agent_path
                    || registered_agent_metadata.agent_role != agent_role
                    || registered_agent_metadata.agent_nickname != agent_nickname
                {
                    return Err(CodexErr::InvalidRequest(format!(
                        "registered agent identity is inconsistent with persisted identity for thread {thread_id}"
                    )));
                }
                let session_source = SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                    parent_thread_id,
                    depth,
                    agent_path,
                    agent_nickname,
                    agent_role,
                });
                (
                    session_source,
                    registered_agent_metadata.clone(),
                    had_persisted_agent_path,
                )
            }
            (
                ResumedAgentSource::ThreadSpawn {
                    parent_thread_id,
                    depth,
                    agent_path,
                    agent_role,
                    agent_nickname,
                    had_persisted_agent_path,
                },
                None,
            ) => {
                let Some(reservation) = registration_reservation.as_mut() else {
                    return Err(CodexErr::Fatal(
                        "resume registration reservation unavailable for persisted agent"
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
                    agent_nickname,
                )?;
                (session_source, agent_metadata, had_persisted_agent_path)
            }
            (ResumedAgentSource::Legacy(session_source), Some(registered_agent_metadata)) => {
                (session_source, registered_agent_metadata.clone(), true)
            }
            (ResumedAgentSource::Legacy(session_source), None) => {
                (session_source, AgentMetadata::default(), true)
            }
        };
        let notification_source = session_source.clone();
        if !had_persisted_agent_path
            && let InitialHistory::Resumed(resumed_history) = &mut initial_history
            && let Some(RolloutItem::SessionMeta(meta_line)) = Arc::make_mut(
                &mut resumed_history.history,
            )
            .iter_mut()
            .find(
                |item| matches!(item, RolloutItem::SessionMeta(meta) if meta.meta.id == thread_id),
            )
        {
            meta_line.meta.parent_thread_id = notification_source.parent_thread_id();
            meta_line.meta.agent_nickname = notification_source.get_nickname();
            meta_line.meta.agent_role = notification_source.get_agent_role();
            meta_line.meta.agent_path = notification_source.get_agent_path().map(Into::into);
            meta_line.meta.source = notification_source.clone();
        }
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
                agent_control: self.clone(),
                session_source,
                parent_thread_id,
                inherited_environments,
                inherited_exec_policy,
            })
            .await?;
        if resumed_thread.thread.config_snapshot().await.session_source != notification_source {
            return Err(CodexErr::InvalidRequest(format!(
                "thread {} was resumed concurrently with a different agent identity",
                resumed_thread.thread_id
            )));
        }
        if !had_persisted_agent_path && let Some(agent_path) = notification_source.get_agent_path()
        {
            let patch = ThreadMetadataPatch {
                source: Some(notification_source.clone()),
                agent_path: Some(Some(agent_path.to_string())),
                ..Default::default()
            };
            if let Err(err) = resumed_thread
                .thread
                .update_thread_metadata(patch, /*include_archived*/ true)
                .await
            {
                if let Err(shutdown_err) = self
                    .shutdown_thread_for_lifecycle(
                        &state,
                        Arc::clone(&resumed_thread.thread),
                        resume_failure_cleanup,
                    )
                    .await
                {
                    let mut retained_agent_metadata = agent_metadata;
                    retained_agent_metadata.agent_id = Some(resumed_thread.thread_id);
                    if let Some(registration_reservation) = registration_reservation.take() {
                        registration_reservation.commit(retained_agent_metadata);
                    }
                    if let Some(residency_slot) = residency_slot.take() {
                        residency_slot.commit(resumed_thread.thread_id);
                    }
                    return Err(CodexErr::Fatal(format!(
                        "failed to persist backfilled agent path for thread {}: {err}; the resumed thread could not be confirmed stopped: {shutdown_err}",
                        resumed_thread.thread_id
                    )));
                }
                let _ = state
                    .remove_thread_if_same_or_missing(
                        &resumed_thread.thread_id,
                        &resumed_thread.thread,
                    )
                    .await;
                return Err(CodexErr::Fatal(format!(
                    "failed to persist backfilled agent path for thread {}: {err}",
                    resumed_thread.thread_id
                )));
            }
        }
        let mut agent_metadata = agent_metadata;
        agent_metadata.agent_id = Some(resumed_thread.thread_id);
        if let Err(persist_err) = self
            .persist_thread_spawn_edge_for_source(
                resumed_thread.thread.as_ref(),
                resumed_thread.thread_id,
                Some(&notification_source),
                codex_agent_graph_store::ThreadSpawnEdgeStatus::Open,
            )
            .await
        {
            if let Err(shutdown_err) = self
                .shutdown_thread_for_lifecycle(
                    &state,
                    Arc::clone(&resumed_thread.thread),
                    resume_failure_cleanup,
                )
                .await
            {
                if let Some(registration_reservation) = registration_reservation.take() {
                    registration_reservation.commit(agent_metadata.clone());
                }
                if let Some(residency_slot) = residency_slot.take() {
                    residency_slot.commit(resumed_thread.thread_id);
                }
                warn!(
                    %shutdown_err,
                    thread_id = %resumed_thread.thread_id,
                    "failed to stop resumed agent after lifecycle persistence failure"
                );
                return Err(CodexErr::Fatal(
                    "failed to persist resumed agent lifecycle state and could not confirm shutdown"
                        .to_string(),
                ));
            }
            let _ = state
                .remove_thread_if_same_or_missing(&resumed_thread.thread_id, &resumed_thread.thread)
                .await;
            return Err(persist_err);
        }
        if let Some(registration_reservation) = registration_reservation.take() {
            registration_reservation.commit(agent_metadata.clone());
        } else {
            self.state.clear_last_status(resumed_thread.thread_id);
        }
        if let Some(residency_slot) = residency_slot.take() {
            residency_slot.commit(resumed_thread.thread_id);
        }
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
                Some(notification_source.clone()),
                child_reference,
                agent_metadata.agent_path.clone(),
            );
        }
        Ok((resumed_thread.thread_id, multi_agent_version))
    }
}

pub(super) fn checked_resume_child_depth(parent_depth: i32) -> CodexResult<i32> {
    let child_depth = parent_depth
        .checked_add(1)
        .ok_or_else(|| CodexErr::Fatal("persisted agent graph depth overflow".to_string()))?;
    if child_depth > MAX_AGENT_GRAPH_DEPTH {
        return Err(CodexErr::Fatal(format!(
            "persisted agent graph exceeds the resume depth limit of {MAX_AGENT_GRAPH_DEPTH}"
        )));
    }
    Ok(child_depth)
}

fn validate_agent_path_against_parent(
    thread_id: ThreadId,
    depth: i32,
    agent_path: &AgentPath,
    parent_agent_path: &AgentPath,
) -> CodexResult<()> {
    let actual_parent_path = agent_path
        .as_str()
        .rsplit_once('/')
        .map(|(parent, _)| parent);
    let path_depth = agent_path
        .as_str()
        .strip_prefix("/root/")
        .and_then(|path| i32::try_from(path.split('/').count()).ok());
    if actual_parent_path != Some(parent_agent_path.as_str()) || path_depth != Some(depth) {
        return Err(CodexErr::InvalidRequest(format!(
            "stored agent path is inconsistent with parent lineage for thread {thread_id}"
        )));
    }
    Ok(())
}
