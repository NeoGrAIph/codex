use super::*;

impl AgentControl {
    /// Submit a shutdown request for a live agent without marking it explicitly closed in
    /// persisted spawn-edge state.
    pub(crate) async fn shutdown_live_agent(&self, agent_id: ThreadId) -> CodexResult<String> {
        let state = self.upgrade()?;
        let result = if let Ok(thread) = state.get_thread(agent_id).await {
            thread.codex.session.ensure_rollout_materialized().await;
            thread.codex.session.flush_rollout().await?;
            let result = if matches!(thread.agent_status().await, AgentStatus::Shutdown) {
                Ok(String::new())
            } else {
                state.send_op(agent_id, Op::Shutdown {}).await
            };
            thread.wait_until_terminated().await;
            result
        } else {
            state.send_op(agent_id, Op::Shutdown {}).await
        };
        let _ = state.remove_thread(&agent_id).await;
        self.forget_v2_residency(agent_id);
        self.state.release_spawned_thread(agent_id);
        result
    }

    pub(super) async fn shutdown_live_agent_with_cleanup(
        &self,
        agent_id: ThreadId,
        cleanup: TerminatedThreadCleanup,
    ) -> CodexResult<String> {
        let state = self.upgrade()?;
        let live_thread = state.get_thread(agent_id).await.ok();
        let result = if let Some(thread) = live_thread.as_ref() {
            thread.codex.session.ensure_rollout_materialized().await;
            thread.codex.session.flush_rollout().await?;
            let result = if matches!(thread.agent_status().await, AgentStatus::Shutdown) {
                Ok(String::new())
            } else {
                state.send_op(agent_id, Op::Shutdown {}).await
            };
            self.wait_for_thread_termination_for_lifecycle(&state, Arc::clone(thread), cleanup)
                .await?;
            match result {
                Err(CodexErr::InternalAgentDied) => Ok(String::new()),
                result => result,
            }
        } else {
            state.send_op(agent_id, Op::Shutdown {}).await
        };
        if cleanup == TerminatedThreadCleanup::FailedSpawnRollback {
            return result;
        }
        let removed_or_missing = match live_thread.as_ref() {
            Some(thread) => {
                state
                    .remove_thread_if_same_or_missing(&agent_id, thread)
                    .await
            }
            None => {
                let _ = state.remove_thread(&agent_id).await;
                true
            }
        };
        if !removed_or_missing {
            return Err(CodexErr::Fatal(format!(
                "agent {agent_id} was replaced while completing lifecycle shutdown"
            )));
        }
        self.forget_v2_residency(agent_id);
        if cleanup == TerminatedThreadCleanup::ReleaseAgent {
            self.state.release_spawned_thread(agent_id);
        }
        self.lifecycle.finish_late_cleanup(agent_id);
        result
    }

    /// Mark `agent_id` as explicitly closed in persisted spawn-edge state, then shut down the
    /// agent and any live descendants reached from the in-memory tree.
    pub(crate) async fn close_agent(&self, agent_id: ThreadId) -> CodexResult<String> {
        let state = self.upgrade()?;
        let known_agent = self.state.agent_metadata_for_thread(agent_id).is_some();
        match state.get_thread(agent_id).await {
            Ok(thread) => {
                if !thread.config_snapshot().await.ephemeral
                    && let Some(agent_graph_store) = state.agent_graph_store()
                    && let Err(err) = agent_graph_store
                        .set_thread_spawn_edge_status(
                            agent_id,
                            codex_agent_graph_store::ThreadSpawnEdgeStatus::Closed,
                        )
                        .await
                {
                    warn!("failed to persist thread-spawn edge status for {agent_id}: {err}");
                }
            }
            Err(CodexErr::ThreadNotFound(_)) if known_agent => {
                if let Some(agent_graph_store) = state.agent_graph_store()
                    && let Err(err) = agent_graph_store
                        .set_thread_spawn_edge_status(
                            agent_id,
                            codex_agent_graph_store::ThreadSpawnEdgeStatus::Closed,
                        )
                        .await
                {
                    return Err(CodexErr::Fatal(format!(
                        "failed to persist stale thread-spawn edge status for {agent_id}: {err}"
                    )));
                }
            }
            Err(CodexErr::ThreadNotFound(_)) => {}
            Err(err) => {
                warn!("failed to inspect agent before close {agent_id}: {err}");
            }
        }
        match Box::pin(self.shutdown_agent_tree(agent_id)).await {
            Err(CodexErr::ThreadNotFound(_)) | Err(CodexErr::InternalAgentDied) if known_agent => {
                Ok(String::new())
            }
            result => result,
        }
    }

    pub(crate) async fn close_agent_transactional(
        &self,
        agent_id: ThreadId,
    ) -> CodexResult<String> {
        let _lifecycle_guard = self.acquire_lifecycle_gate().await;
        self.ensure_no_pending_lifecycle_cleanup(agent_id)?;
        let state = self.upgrade()?;
        let known_agent = self.state.agent_metadata_for_thread(agent_id).is_some();
        let descendant_ids = self.live_thread_spawn_descendants(agent_id).await?;
        match state.get_thread(agent_id).await {
            Ok(thread) => {
                if !thread.config_snapshot().await.ephemeral
                    && matches!(
                        &thread.session_source,
                        SessionSource::SubAgent(SubAgentSource::ThreadSpawn { .. })
                    )
                    && let Some(agent_graph_store) = state.agent_graph_store()
                    && let Err(err) = agent_graph_store
                        .set_thread_spawn_edge_status(
                            agent_id,
                            codex_agent_graph_store::ThreadSpawnEdgeStatus::Closed,
                        )
                        .await
                {
                    warn!(%err, %agent_id, "failed to persist thread-spawn edge status");
                    return Err(CodexErr::Fatal(
                        "failed to persist closed agent lifecycle state".to_string(),
                    ));
                }
            }
            Err(CodexErr::ThreadNotFound(_)) if known_agent => {
                if let Some(agent_graph_store) = state.agent_graph_store() {
                    let has_persisted_edge = agent_graph_store
                        .get_thread_spawn_parent(agent_id)
                        .await
                        .map_err(|err| {
                            warn!(%err, %agent_id, "failed to inspect stale thread-spawn edge");
                            CodexErr::Fatal(
                                "failed to inspect closed agent lifecycle state".to_string(),
                            )
                        })?
                        .is_some();
                    if has_persisted_edge
                        && let Err(err) = agent_graph_store
                            .set_thread_spawn_edge_status(
                                agent_id,
                                codex_agent_graph_store::ThreadSpawnEdgeStatus::Closed,
                            )
                            .await
                    {
                        warn!(%err, %agent_id, "failed to persist stale thread-spawn edge status");
                        return Err(CodexErr::Fatal(
                            "failed to persist closed agent lifecycle state".to_string(),
                        ));
                    }
                }
            }
            Err(CodexErr::ThreadNotFound(_)) => {}
            Err(err) => {
                warn!("failed to inspect agent before close {agent_id}: {err}");
            }
        }
        match Box::pin(
            self.shutdown_agent_tree_with_descendants_transactional(agent_id, descendant_ids),
        )
        .await
        {
            Err(CodexErr::ThreadNotFound(_)) | Err(CodexErr::InternalAgentDied) if known_agent => {
                Ok(String::new())
            }
            result => result,
        }
    }

    /// Shut down `agent_id` and any live descendants reachable from the in-memory spawn tree.
    pub(crate) async fn shutdown_agent_tree(&self, agent_id: ThreadId) -> CodexResult<String> {
        let descendant_ids = self.live_thread_spawn_descendants(agent_id).await?;
        let result = self.shutdown_live_agent(agent_id).await;
        for descendant_id in descendant_ids {
            match self.shutdown_live_agent(descendant_id).await {
                Ok(_) | Err(CodexErr::ThreadNotFound(_)) | Err(CodexErr::InternalAgentDied) => {}
                Err(err) => return Err(err),
            }
        }
        result
    }

    async fn shutdown_agent_tree_with_descendants_transactional(
        &self,
        agent_id: ThreadId,
        descendant_ids: Vec<ThreadId>,
    ) -> CodexResult<String> {
        let result = self
            .shutdown_live_agent_with_cleanup(agent_id, TerminatedThreadCleanup::ReleaseAgent)
            .await;
        if let Err(err) = &result
            && !matches!(
                err,
                CodexErr::ThreadNotFound(_) | CodexErr::InternalAgentDied
            )
        {
            return result;
        }
        for descendant_id in descendant_ids {
            match self
                .shutdown_live_agent_with_cleanup(
                    descendant_id,
                    TerminatedThreadCleanup::ReleaseAgent,
                )
                .await
            {
                Ok(_) | Err(CodexErr::ThreadNotFound(_)) | Err(CodexErr::InternalAgentDied) => {}
                Err(err) => return Err(err),
            }
        }
        result
    }
}
