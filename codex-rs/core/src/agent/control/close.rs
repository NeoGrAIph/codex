use super::*;
use std::collections::HashSet;

#[derive(Clone, Copy)]
pub(super) enum ClosedEdgeErrorPolicy {
    Warn,
    Fail,
}

impl AgentControl {
    pub(crate) async fn close_resolved_agent(
        &self,
        target: ResolvedCloseTarget,
    ) -> CodexResult<String> {
        match target.multi_agent_version() {
            Some(MultiAgentVersion::V1 | MultiAgentVersion::V2) => {
                self.close_resolved_subtree(target).await
            }
            Some(MultiAgentVersion::Disabled) => unreachable!("disabled target was rejected"),
            None => Err(CodexErr::ThreadNotFound(target.thread_id())),
        }
    }

    async fn close_resolved_subtree(
        &self,
        root_target: ResolvedCloseTarget,
    ) -> CodexResult<String> {
        let state = self.upgrade()?;
        let root_thread_id = root_target.thread_id();
        self.validate_persisted_close_graph(&state, root_thread_id)
            .await?;
        let mut pending = VecDeque::from([(root_target, true)]);
        let mut visited = HashSet::from([root_thread_id]);
        let mut root_result = String::new();

        while let Some((target, is_root)) = pending.pop_front() {
            let thread_id = target.thread_id();
            let lifecycle_guard = state.lock_thread_lifecycle(thread_id).await;
            debug_assert_eq!(lifecycle_guard.thread_id(), thread_id);

            self.validate_bound_close_target(&state, &target, is_root)
                .await?;
            let children = self
                .resolve_direct_close_children(&state, thread_id, &mut visited)
                .await?;
            let result = self
                .close_single_resolved_target(&state, target, is_root)
                .await?;
            if is_root {
                root_result = result;
            }
            drop(lifecycle_guard);
            pending.extend(children.into_iter().map(|child| (child, false)));
        }

        Ok(root_result)
    }

    async fn validate_persisted_close_graph(
        &self,
        state: &Arc<ThreadManagerState>,
        root_thread_id: ThreadId,
    ) -> CodexResult<()> {
        let Some(agent_graph_store) = state.agent_graph_store() else {
            return Ok(());
        };
        let mut pending = VecDeque::from([root_thread_id]);
        let mut visited = HashSet::from([root_thread_id]);
        while let Some(parent_thread_id) = pending.pop_front() {
            let mut child_ids = agent_graph_store
                .list_thread_spawn_children(
                    parent_thread_id,
                    Some(codex_agent_graph_store::ThreadSpawnEdgeStatus::Open),
                )
                .await
                .map_err(|err| {
                    CodexErr::Fatal(format!(
                        "failed to validate open descendants while closing {root_thread_id}: {err}"
                    ))
                })?;
            child_ids.sort_by_key(ToString::to_string);
            for child_thread_id in child_ids {
                if !visited.insert(child_thread_id) {
                    return Err(CodexErr::InvalidRequest(format!(
                        "cyclic or multiply-owned thread-spawn graph while closing {root_thread_id}: repeated agent {child_thread_id}"
                    )));
                }
                pending.push_back(child_thread_id);
            }
        }
        Ok(())
    }

    async fn validate_bound_close_target(
        &self,
        state: &Arc<ThreadManagerState>,
        target: &ResolvedCloseTarget,
        is_root: bool,
    ) -> CodexResult<()> {
        match target {
            ResolvedCloseTarget::Live(target) => match state.get_thread(target.thread_id()).await {
                Ok(current) if Arc::ptr_eq(&current, target.thread()) => Ok(()),
                Ok(_) => Err(CodexErr::InvalidRequest(format!(
                    "agent with id {} changed runtime after close validation",
                    target.thread_id()
                ))),
                Err(CodexErr::ThreadNotFound(_)) => Ok(()),
                Err(err) => Err(err),
            },
            ResolvedCloseTarget::Persisted { thread_id, .. } => {
                match state.get_thread(*thread_id).await {
                    Ok(_) => Err(CodexErr::InvalidRequest(format!(
                        "agent with id {thread_id} became live after close validation"
                    ))),
                    Err(CodexErr::ThreadNotFound(_)) => Ok(()),
                    Err(err) => Err(err),
                }
            }
            ResolvedCloseTarget::Missing(thread_id) if is_root => {
                Err(CodexErr::ThreadNotFound(*thread_id))
            }
            ResolvedCloseTarget::Missing(_) => Ok(()),
        }
    }

    async fn resolve_direct_close_children(
        &self,
        state: &Arc<ThreadManagerState>,
        parent_thread_id: ThreadId,
        visited: &mut HashSet<ThreadId>,
    ) -> CodexResult<Vec<ResolvedCloseTarget>> {
        let mut child_ids = self
            .state
            .catalog_child_ids(parent_thread_id)
            .into_iter()
            .collect::<HashSet<_>>();
        child_ids.extend(
            self.open_thread_spawn_children(parent_thread_id)
                .await?
                .into_iter()
                .map(|(thread_id, _)| thread_id),
        );
        if let Some(agent_graph_store) = state.agent_graph_store() {
            child_ids.extend(
                agent_graph_store
                    .list_thread_spawn_children(
                        parent_thread_id,
                        Some(codex_agent_graph_store::ThreadSpawnEdgeStatus::Open),
                    )
                    .await
                    .map_err(|err| {
                        CodexErr::Fatal(format!(
                            "failed to load open children while closing {parent_thread_id}: {err}"
                        ))
                    })?,
            );
        }
        let mut child_ids = child_ids.into_iter().collect::<Vec<_>>();
        child_ids.sort_by_key(ToString::to_string);

        let mut children = Vec::with_capacity(child_ids.len());
        for child_thread_id in child_ids {
            if !visited.insert(child_thread_id) {
                return Err(CodexErr::InvalidRequest(format!(
                    "cyclic or multiply-owned thread-spawn graph while closing {parent_thread_id}: repeated agent {child_thread_id}"
                )));
            }
            let target = match state.get_thread(child_thread_id).await {
                Ok(thread) => {
                    let multi_agent_version = target_version::require_closeable_runtime(
                        child_thread_id,
                        thread.multi_agent_version(),
                    )?;
                    ResolvedCloseTarget::Live(target_version::BoundAgentThread::new(
                        child_thread_id,
                        thread,
                        multi_agent_version,
                        true,
                    ))
                }
                Err(CodexErr::ThreadNotFound(_)) => {
                    self.resolve_close_target(child_thread_id).await?
                }
                Err(err) => return Err(err),
            };
            children.push(target);
        }
        Ok(children)
    }

    async fn close_single_resolved_target(
        &self,
        state: &Arc<ThreadManagerState>,
        target: ResolvedCloseTarget,
        is_root: bool,
    ) -> CodexResult<String> {
        let thread_id = target.thread_id();
        match target {
            ResolvedCloseTarget::Live(target) => {
                match state.get_thread(thread_id).await {
                    Ok(current) if Arc::ptr_eq(&current, target.thread()) => {}
                    Ok(_) => {
                        return Err(CodexErr::InvalidRequest(format!(
                            "agent with id {thread_id} changed runtime during close"
                        )));
                    }
                    Err(CodexErr::ThreadNotFound(_)) => {
                        self.persist_resolved_close_edge(
                            state,
                            &ResolvedCloseTarget::Live(target.clone()),
                        )
                        .await?;
                        self.forget_v2_residency(thread_id);
                        self.state.release_spawned_thread(thread_id);
                        return Ok(String::new());
                    }
                    Err(err) => return Err(err),
                }
                self.persist_resolved_close_edge(state, &ResolvedCloseTarget::Live(target.clone()))
                    .await?;
                let (result, _) = self.shutdown_bound_agent(&target).await;
                result
            }
            ResolvedCloseTarget::Persisted { .. } | ResolvedCloseTarget::Missing(_) if !is_root => {
                self.persist_resolved_close_edge(state, &target).await?;
                self.forget_v2_residency(thread_id);
                self.state.release_spawned_thread(thread_id);
                Ok(String::new())
            }
            ResolvedCloseTarget::Persisted { .. } => {
                self.persist_resolved_close_edge(state, &target).await?;
                self.forget_v2_residency(thread_id);
                self.state.release_spawned_thread(thread_id);
                Ok(String::new())
            }
            ResolvedCloseTarget::Missing(_) => Err(CodexErr::ThreadNotFound(thread_id)),
        }
    }

    async fn persist_resolved_close_edge(
        &self,
        state: &Arc<ThreadManagerState>,
        target: &ResolvedCloseTarget,
    ) -> CodexResult<()> {
        let error_policy = match target {
            ResolvedCloseTarget::Live(target)
                if target.multi_agent_version() == MultiAgentVersion::V1 =>
            {
                if target.thread().config_snapshot().await.ephemeral {
                    return Ok(());
                }
                ClosedEdgeErrorPolicy::Warn
            }
            ResolvedCloseTarget::Live(_)
            | ResolvedCloseTarget::Persisted { .. }
            | ResolvedCloseTarget::Missing(_) => ClosedEdgeErrorPolicy::Fail,
        };
        self.persist_closed_spawn_edge(state, target.thread_id(), error_policy)
            .await
    }

    async fn shutdown_bound_agent(
        &self,
        target: &target_version::BoundAgentThread,
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
                self.forget_v2_residency(target.thread_id());
                self.state.release_spawned_thread(target.thread_id());
            })
            .await
            .is_some();
        if removed {
            debug_assert_eq!(
                target.thread().multi_agent_version(),
                Some(target.multi_agent_version())
            );
        }
        (result, removed)
    }

    pub(super) async fn persist_closed_spawn_edge(
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
                "failed to persist Closed thread-spawn edge status for {agent_id}: {err}"
            ))),
        }
    }
}
