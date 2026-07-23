use super::*;
use std::collections::HashSet;

#[derive(Clone, Copy)]
enum ClosedEdgeErrorPolicy {
    Warn,
    Fail,
}

impl AgentControl {
    /// Submit a shutdown request for a live agent without marking it explicitly closed in
    /// persisted spawn-edge state.
    pub(crate) async fn shutdown_live_agent(&self, agent_id: ThreadId) -> CodexResult<String> {
        let state = self.upgrade()?;
        let result = if let Ok(thread) = state.get_thread(agent_id).await {
            thread.session.ensure_rollout_materialized().await;
            thread.session.flush_rollout().await?;
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

    /// Close a projected V1 target without crossing into non-V1 descendant branches.
    pub(crate) async fn close_projected_v1_agent(
        &self,
        target: ProjectedV1LifecycleTarget,
    ) -> CodexResult<String> {
        let state = self.upgrade()?;
        let agent_id = target.thread_id();
        let lifecycle_guard = state.lock_thread_lifecycle(agent_id).await;
        debug_assert_eq!(lifecycle_guard.thread_id(), agent_id);
        let Some(root) = target.live_thread() else {
            if state.get_thread(agent_id).await.is_ok() {
                return Err(CodexErr::InvalidRequest(format!(
                    "agent with id {agent_id} changed runtime after V1 validation"
                )));
            }
            if matches!(target, ProjectedV1LifecycleTarget::Missing(_)) {
                return Err(CodexErr::ThreadNotFound(agent_id));
            }
            self.persist_closed_spawn_edge(&state, agent_id, ClosedEdgeErrorPolicy::Fail)
                .await?;
            return Ok(String::new());
        };

        match state.get_thread(agent_id).await {
            Ok(current) if !Arc::ptr_eq(&current, root.thread()) => {
                let _ = self.shutdown_validated_v1_agent(root).await;
                return Err(CodexErr::InvalidRequest(format!(
                    "agent with id {agent_id} changed runtime after V1 validation"
                )));
            }
            Ok(_) | Err(CodexErr::ThreadNotFound(_)) => {}
            Err(err) => return Err(err),
        }
        let should_persist_closed = !root.thread().config_snapshot().await.ephemeral;
        let descendants = self.live_projected_v1_descendants(agent_id).await?;
        if should_persist_closed {
            self.persist_closed_spawn_edge(&state, agent_id, ClosedEdgeErrorPolicy::Warn)
                .await?;
        }
        let (result, removed) = self.shutdown_validated_v1_agent(root).await;
        if !removed {
            match state.get_thread(agent_id).await {
                Ok(current) if Arc::ptr_eq(&current, root.thread()) => return result,
                Ok(_) => {
                    return Err(CodexErr::InvalidRequest(format!(
                        "agent with id {agent_id} changed runtime while it was closing"
                    )));
                }
                Err(CodexErr::ThreadNotFound(_)) => {}
                Err(err) => return Err(err),
            }
        }
        match result {
            Ok(_) | Err(CodexErr::ThreadNotFound(_)) | Err(CodexErr::InternalAgentDied) => {}
            Err(err) => return Err(err),
        }

        for descendant in descendants {
            let descendant_id = descendant.thread_id();
            let descendant_lifecycle_guard = state.lock_thread_lifecycle(descendant_id).await;
            debug_assert_eq!(descendant_lifecycle_guard.thread_id(), descendant_id);
            match state.get_thread(descendant_id).await {
                Ok(current) if !Arc::ptr_eq(&current, descendant.thread()) => {
                    return Err(CodexErr::InvalidRequest(format!(
                        "agent with id {descendant_id} changed runtime while its ancestor was closing"
                    )));
                }
                Ok(_) | Err(CodexErr::ThreadNotFound(_)) => {}
                Err(err) => return Err(err),
            }
            let (result, removed) = self.shutdown_validated_v1_agent(&descendant).await;
            if !removed {
                match state.get_thread(descendant_id).await {
                    Ok(current) if Arc::ptr_eq(&current, descendant.thread()) => return result,
                    Ok(_) => {
                        return Err(CodexErr::InvalidRequest(format!(
                            "agent with id {descendant_id} changed runtime while it was closing"
                        )));
                    }
                    Err(CodexErr::ThreadNotFound(_)) => {}
                    Err(err) => return Err(err),
                }
            }
            drop(descendant_lifecycle_guard);
            match result {
                Ok(_) | Err(CodexErr::ThreadNotFound(_)) | Err(CodexErr::InternalAgentDied) => {}
                Err(err) => return Err(err),
            }
        }
        Ok(String::new())
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
        let shutdown_result = Box::pin(self.shutdown_agent_tree(agent_id)).await;
        match shutdown_result {
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

    async fn live_projected_v1_descendants(
        &self,
        agent_id: ThreadId,
    ) -> CodexResult<Vec<target_version::ValidatedV1Thread>> {
        let state = self.upgrade()?;
        let mut children_by_parent = self.live_thread_spawn_children().await?;
        let mut descendants = Vec::new();
        let mut visited = HashSet::from([agent_id]);
        let mut stack = children_by_parent
            .remove(&agent_id)
            .unwrap_or_default()
            .into_iter()
            .map(|(child_thread_id, _)| child_thread_id)
            .rev()
            .collect::<Vec<_>>();

        while let Some(thread_id) = stack.pop() {
            if !visited.insert(thread_id) {
                return Err(CodexErr::InvalidRequest(format!(
                    "cyclic live thread-spawn graph encountered while closing {agent_id}: repeated thread {thread_id}"
                )));
            }
            let Ok(thread) = state.get_thread(thread_id).await else {
                continue;
            };
            if thread.multi_agent_version() != Some(MultiAgentVersion::V1) {
                continue;
            }
            descendants.push(target_version::ValidatedV1Thread::new(thread_id, thread));
            if let Some(children) = children_by_parent.remove(&thread_id) {
                for (child_thread_id, _) in children.into_iter().rev() {
                    stack.push(child_thread_id);
                }
            }
        }
        Ok(descendants)
    }

    async fn shutdown_validated_v1_agent(
        &self,
        target: &target_version::ValidatedV1Thread,
    ) -> (CodexResult<String>, bool) {
        let state = match self.upgrade() {
            Ok(state) => state,
            Err(err) => return (Err(err), false),
        };
        target.thread().ensure_rollout_materialized().await;
        if let Err(err) = target.thread().flush_rollout().await {
            return (Err(err.into()), false);
        }
        let result = if matches!(target.status().await, AgentStatus::Shutdown) {
            Ok(String::new())
        } else {
            state
                .send_op_to_thread(target.thread_id(), target.thread(), Op::Shutdown {})
                .await
        };
        target.thread().wait_until_terminated().await;
        let removed = state
            .remove_thread_if_same_with_cleanup(&target.thread_id(), target.thread(), || {
                self.state.release_spawned_thread(target.thread_id())
            })
            .await
            .is_some();
        if removed {
            debug_assert_eq!(
                target.thread().multi_agent_version(),
                Some(MultiAgentVersion::V1)
            );
        }
        (result, removed)
    }

    async fn persist_closed_spawn_edge(
        &self,
        state: &Arc<ThreadManagerState>,
        agent_id: ThreadId,
        error_policy: ClosedEdgeErrorPolicy,
    ) -> CodexResult<()> {
        let Some(agent_graph_store) = state.agent_graph_store() else {
            return Ok(());
        };
        match agent_graph_store
            .set_thread_spawn_edge_status(
                agent_id,
                codex_agent_graph_store::ThreadSpawnEdgeStatus::Closed,
            )
            .await
        {
            Ok(()) => Ok(()),
            Err(err) if matches!(error_policy, ClosedEdgeErrorPolicy::Warn) => {
                warn!("failed to persist thread-spawn edge status for {agent_id}: {err}");
                Ok(())
            }
            Err(err) => Err(CodexErr::Fatal(format!(
                "failed to persist stale thread-spawn edge status for {agent_id}: {err}"
            ))),
        }
    }
}
