use super::ResumeLineageAuthority;
use super::checked_resume_child_depth;
use crate::agent::control::AgentControl;
use crate::agent::control::thread_spawn_depth;
use crate::config::Config;
use crate::thread_manager::ResumeThreadWithHistoryOptions;
use codex_protocol::ThreadId;
use codex_protocol::error::CodexErr;
use codex_protocol::error::Result as CodexResult;
use codex_protocol::protocol::InitialHistory;
use codex_protocol::protocol::MultiAgentVersion;
use codex_protocol::protocol::ResumedHistory;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use codex_thread_store::ReadThreadParams;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::sync::Arc;
use tracing::warn;

impl AgentControl {
    pub(crate) async fn ensure_v2_agent_loaded(
        &self,
        mut config: Config,
        thread_id: ThreadId,
    ) -> CodexResult<()> {
        let _lifecycle_guard = self.acquire_lifecycle_gate().await;
        self.ensure_no_pending_lifecycle_cleanup(thread_id)?;
        let state = self.upgrade()?;
        if state.get_thread(thread_id).await.is_ok() {
            self.touch_loaded_v2_residency(&state, thread_id).await;
            return Ok(());
        }
        if self.state.agent_metadata_for_thread(thread_id).is_none() {
            return Err(CodexErr::ThreadNotFound(thread_id));
        }

        let mut stored_thread = state
            .read_stored_thread(ReadThreadParams {
                thread_id,
                include_archived: true,
                include_history: true,
            })
            .await?;
        stored_thread
            .project_identity_into_history()
            .map_err(|err| {
                CodexErr::InvalidRequest(format!("invalid stored agent identity: {err}"))
            })?;
        if let Some(model) = stored_thread.model.clone() {
            config.model = Some(model);
        }
        if let Some(reasoning_effort) = stored_thread.reasoning_effort.clone() {
            config.model_reasoning_effort = Some(reasoning_effort);
        }
        let stored_source = stored_thread.source.clone();
        let stored_parent_thread_id = stored_thread.parent_thread_id;
        let history = stored_thread
            .history
            .ok_or(CodexErr::ThreadNotFound(thread_id))?
            .items;
        let initial_history = InitialHistory::Resumed(ResumedHistory {
            conversation_id: thread_id,
            history: Arc::new(history),
            rollout_path: stored_thread.rollout_path,
        });
        if initial_history.get_multi_agent_version() != Some(MultiAgentVersion::V2) {
            return Err(CodexErr::ThreadNotFound(thread_id));
        }
        let (session_source, _) = initial_history
            .get_resumed_session_sources()
            .unwrap_or((stored_source, None));
        if let SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id,
            depth,
            agent_path,
            ..
        }) = &session_source
        {
            let Some(agent_path) = agent_path.as_ref() else {
                return Err(CodexErr::InvalidRequest(format!(
                    "stored V2 agent {thread_id} is missing a canonical path; resume it explicitly before sending input"
                )));
            };
            self.validate_persisted_v2_agent_path(
                &state,
                thread_id,
                *parent_thread_id,
                *depth,
                agent_path,
            )
            .await?;
        }
        let parent_thread_id = initial_history
            .get_resumed_parent_thread_id()
            .or(stored_parent_thread_id);
        let residency_slot = self
            .reserve_v2_residency_slot(&state, &config, Some(thread_id))
            .await?;
        let inherited_environments = self
            .inherited_environments_for_source(&state, Some(&session_source))
            .await;
        let inherited_exec_policy = self
            .inherited_exec_policy_for_source(&state, Some(&session_source), &config)
            .await;

        match state
            .resume_thread_with_history_with_source(ResumeThreadWithHistoryOptions {
                config,
                initial_history,
                agent_control: self.clone(),
                session_source,
                parent_thread_id,
                inherited_environments,
                inherited_exec_policy,
            })
            .await
        {
            Ok(reloaded_thread) => {
                residency_slot.commit(reloaded_thread.thread_id);
                state.notify_thread_created(reloaded_thread.thread_id);
                Ok(())
            }
            Err(err) => {
                if state.get_thread(thread_id).await.is_ok() {
                    drop(residency_slot);
                    self.touch_loaded_v2_residency(&state, thread_id).await;
                    return Ok(());
                }
                Err(err)
            }
        }
    }

    /// Resume an existing agent thread from a recorded rollout file.
    pub(crate) async fn resume_agent_from_rollout(
        &self,
        config: Config,
        thread_id: ThreadId,
        session_source: SessionSource,
    ) -> CodexResult<ThreadId> {
        self.resume_agent_from_rollout_with_authority(
            config,
            thread_id,
            session_source,
            ResumeLineageAuthority::PersistedMetadata,
        )
        .await
    }

    pub(crate) async fn resume_agent_from_persisted_graph_edge(
        &self,
        config: Config,
        thread_id: ThreadId,
        session_source: SessionSource,
    ) -> CodexResult<ThreadId> {
        self.resume_agent_from_rollout_with_authority(
            config,
            thread_id,
            session_source,
            ResumeLineageAuthority::PersistedGraphEdge,
        )
        .await
    }

    async fn resume_agent_from_rollout_with_authority(
        &self,
        config: Config,
        thread_id: ThreadId,
        session_source: SessionSource,
        lineage_authority: ResumeLineageAuthority,
    ) -> CodexResult<ThreadId> {
        let _lifecycle_guard = self.acquire_lifecycle_gate().await;
        if lineage_authority == ResumeLineageAuthority::PersistedGraphEdge {
            let SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth,
                ..
            }) = &session_source
            else {
                return Err(CodexErr::InvalidRequest(
                    "persisted graph resume requires thread-spawn lineage".to_string(),
                ));
            };
            let authoritative_lineage = self.persisted_agent_lineage(thread_id).await?;
            if authoritative_lineage.parent_thread_id != *parent_thread_id
                || authoritative_lineage.depth != *depth
            {
                return Err(CodexErr::InvalidRequest(
                    "persisted agent lineage changed while preparing resume".to_string(),
                ));
            }
        }
        let root_depth = thread_spawn_depth(&session_source).unwrap_or(0);
        let (resumed_thread_id, resumed_multi_agent_version) =
            Box::pin(self.resume_single_agent_from_rollout(
                config.clone(),
                thread_id,
                session_source,
                lineage_authority,
            ))
            .await?;
        let state = self.upgrade()?;
        if config.multi_agent_version_from_features() == MultiAgentVersion::V2
            || resumed_multi_agent_version == MultiAgentVersion::V2
        {
            return Ok(resumed_thread_id);
        }
        let Some(agent_graph_store) = state.agent_graph_store() else {
            return Ok(resumed_thread_id);
        };

        let mut visited = HashSet::from([thread_id]);
        let mut resume_queue = VecDeque::from([(thread_id, root_depth)]);
        while let Some((parent_thread_id, parent_depth)) = resume_queue.pop_front() {
            let child_ids = agent_graph_store
                .list_thread_spawn_children(
                    parent_thread_id,
                    Some(codex_agent_graph_store::ThreadSpawnEdgeStatus::Open),
                )
                .await
                .map_err(|err| {
                    warn!(%err, %parent_thread_id, "failed to load persisted thread-spawn children");
                    CodexErr::Fatal(
                        "failed to load persisted thread-spawn descendants".to_string(),
                    )
                })?;

            for child_thread_id in child_ids {
                if !visited.insert(child_thread_id) {
                    return Err(CodexErr::Fatal(format!(
                        "persisted agent graph contains a cycle while resuming {thread_id}"
                    )));
                }
                let child_depth = checked_resume_child_depth(parent_depth)?;
                let child_resumed = if state.get_thread(child_thread_id).await.is_ok() {
                    true
                } else {
                    let child_session_source =
                        SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                            parent_thread_id,
                            depth: child_depth,
                            agent_path: None,
                            agent_nickname: None,
                            agent_role: None,
                        });
                    match Box::pin(self.resume_single_agent_from_rollout(
                        config.clone(),
                        child_thread_id,
                        child_session_source,
                        ResumeLineageAuthority::PersistedGraphEdge,
                    ))
                    .await
                    {
                        Ok((_, _)) => true,
                        Err(err) => {
                            warn!("failed to resume descendant thread {child_thread_id}: {err}");
                            false
                        }
                    }
                };
                if child_resumed {
                    resume_queue.push_back((child_thread_id, child_depth));
                }
            }
        }

        Ok(resumed_thread_id)
    }
}
