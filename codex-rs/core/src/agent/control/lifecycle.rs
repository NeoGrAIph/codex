use super::AgentControl;
use super::AgentMetadata;
use super::MAX_AGENT_GRAPH_DEPTH;
use crate::codex_thread::CodexThread;
use crate::thread_manager::ThreadManagerState;
use codex_protocol::ThreadId;
use codex_protocol::error::CodexErr;
use codex_protocol::error::Result as CodexResult;
use codex_protocol::protocol::SessionSource;
use codex_thread_store::ReadThreadParams;
use std::collections::HashMap;
use std::collections::HashSet;
use std::future::Future;
use std::sync::Arc;
#[cfg(test)]
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use tracing::warn;

const AGENT_LIFECYCLE_TERMINATION_TIMEOUT: Duration = Duration::from_secs(5);
const LATE_TERMINATION_CLEANUP_RETRY_DELAY: Duration = Duration::from_millis(100);
const LATE_TERMINATION_CLEANUP_SLOW_RETRY_DELAY: Duration = Duration::from_secs(1);
const LATE_TERMINATION_CLEANUP_QUICK_ATTEMPTS: usize = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TerminatedThreadCleanup {
    ReleaseAgent,
    UnloadResident,
    FailedSpawnRollback,
}

impl TerminatedThreadCleanup {
    fn merge(self, next: Self) -> Self {
        match (self, next) {
            (Self::FailedSpawnRollback, _) | (_, Self::FailedSpawnRollback) => {
                Self::FailedSpawnRollback
            }
            (Self::ReleaseAgent, _) | (_, Self::ReleaseAgent) => Self::ReleaseAgent,
            (Self::UnloadResident, Self::UnloadResident) => Self::UnloadResident,
        }
    }
}

#[derive(Default)]
pub(super) struct LifecycleCoordinator {
    gate: Arc<tokio::sync::Mutex<()>>,
    late_cleanups: std::sync::Mutex<HashMap<ThreadId, TerminatedThreadCleanup>>,
}

impl LifecycleCoordinator {
    pub(super) fn register_late_cleanup(
        &self,
        thread_id: ThreadId,
        cleanup: TerminatedThreadCleanup,
    ) -> bool {
        let mut late_cleanups = self
            .late_cleanups
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match late_cleanups.entry(thread_id) {
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(cleanup);
                true
            }
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                let merged_cleanup = (*entry.get()).merge(cleanup);
                entry.insert(merged_cleanup);
                false
            }
        }
    }

    pub(super) fn pending_cleanup(&self, thread_id: ThreadId) -> Option<TerminatedThreadCleanup> {
        self.late_cleanups
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&thread_id)
            .copied()
    }

    pub(super) fn finish_late_cleanup(&self, thread_id: ThreadId) {
        self.late_cleanups
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&thread_id);
    }
}

#[cfg(test)]
pub(super) struct SubmissionLoopCommitHook {
    block: AtomicBool,
    fail_next: AtomicBool,
    entered: tokio::sync::Semaphore,
    release: tokio::sync::Semaphore,
}

#[cfg(test)]
impl Default for SubmissionLoopCommitHook {
    fn default() -> Self {
        Self {
            block: AtomicBool::new(false),
            fail_next: AtomicBool::new(false),
            entered: tokio::sync::Semaphore::new(0),
            release: tokio::sync::Semaphore::new(0),
        }
    }
}

fn lifecycle_timeout_error(thread_id: ThreadId) -> CodexErr {
    CodexErr::Fatal(format!(
        "timed out waiting for agent {thread_id} lifecycle termination"
    ))
}

#[cfg(test)]
pub(super) async fn await_lifecycle_operation<T>(
    thread_id: ThreadId,
    timeout_duration: Duration,
    operation: impl Future<Output = CodexResult<T>>,
) -> CodexResult<T> {
    tokio::time::timeout(timeout_duration, operation)
        .await
        .map_err(|_| lifecycle_timeout_error(thread_id))?
}

impl AgentControl {
    #[cfg(test)]
    pub(crate) fn lifecycle_gate_for_test(&self) -> Arc<tokio::sync::Mutex<()>> {
        Arc::clone(&self.lifecycle.gate)
    }

    pub(super) async fn acquire_lifecycle_gate(&self) -> tokio::sync::OwnedMutexGuard<()> {
        #[cfg(test)]
        self.lifecycle_waiters
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        let guard = Arc::clone(&self.lifecycle.gate).lock_owned().await;
        #[cfg(test)]
        self.lifecycle_waiters
            .fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
        guard
    }

    pub(super) fn ensure_no_pending_lifecycle_cleanup(
        &self,
        thread_id: ThreadId,
    ) -> CodexResult<()> {
        if self.lifecycle.pending_cleanup(thread_id).is_some() {
            return Err(CodexErr::Fatal(
                "agent lifecycle cleanup is still pending".to_string(),
            ));
        }
        Ok(())
    }

    pub(super) async fn await_thread_lifecycle_operation<T>(
        &self,
        state: &Arc<ThreadManagerState>,
        thread: Arc<CodexThread>,
        timeout_duration: Duration,
        cleanup: TerminatedThreadCleanup,
        operation: impl Future<Output = CodexResult<T>>,
    ) -> CodexResult<T> {
        let thread_id = thread.session_configured().thread_id;
        match tokio::time::timeout(timeout_duration, operation).await {
            Ok(result) => result,
            Err(_) => {
                self.start_late_termination_cleanup(Arc::clone(state), thread, cleanup);
                Err(lifecycle_timeout_error(thread_id))
            }
        }
    }

    pub(super) async fn shutdown_thread_for_lifecycle(
        &self,
        state: &Arc<ThreadManagerState>,
        thread: Arc<CodexThread>,
        cleanup: TerminatedThreadCleanup,
    ) -> CodexResult<()> {
        self.shutdown_thread_for_lifecycle_with_timeout(
            state,
            thread,
            cleanup,
            AGENT_LIFECYCLE_TERMINATION_TIMEOUT,
        )
        .await
    }

    pub(super) async fn shutdown_thread_for_lifecycle_with_timeout(
        &self,
        state: &Arc<ThreadManagerState>,
        thread: Arc<CodexThread>,
        cleanup: TerminatedThreadCleanup,
        timeout_duration: Duration,
    ) -> CodexResult<()> {
        let operation_thread = Arc::clone(&thread);
        self.await_thread_lifecycle_operation(
            state,
            thread,
            timeout_duration,
            cleanup,
            async move { operation_thread.shutdown_and_wait().await },
        )
        .await
    }

    pub(super) async fn wait_for_thread_termination_for_lifecycle(
        &self,
        state: &Arc<ThreadManagerState>,
        thread: Arc<CodexThread>,
        cleanup: TerminatedThreadCleanup,
    ) -> CodexResult<()> {
        let operation_thread = Arc::clone(&thread);
        self.await_thread_lifecycle_operation(
            state,
            thread,
            AGENT_LIFECYCLE_TERMINATION_TIMEOUT,
            cleanup,
            async move {
                operation_thread.wait_until_terminated().await;
                Ok(())
            },
        )
        .await
    }

    pub(super) fn start_late_termination_cleanup(
        &self,
        state: Arc<ThreadManagerState>,
        thread: Arc<CodexThread>,
        cleanup: TerminatedThreadCleanup,
    ) {
        let thread_id = thread.session_configured().thread_id;
        if !self.lifecycle.register_late_cleanup(thread_id, cleanup) {
            return;
        }
        let termination = thread.session_loop_termination();
        let thread = Arc::downgrade(&thread);
        let state = Arc::downgrade(&state);
        let registry = Arc::downgrade(&self.state);
        let residency = Arc::downgrade(&self.v2_residency);
        let lifecycle = Arc::clone(&self.lifecycle);
        tokio::spawn(async move {
            termination.await;
            let mut attempt = 0usize;
            loop {
                attempt = attempt.saturating_add(1);
                let lifecycle_guard = Arc::clone(&lifecycle.gate).lock_owned().await;
                let Some(cleanup) = lifecycle.pending_cleanup(thread_id) else {
                    return;
                };
                let Some(state) = state.upgrade() else {
                    lifecycle.finish_late_cleanup(thread_id);
                    return;
                };
                if cleanup == TerminatedThreadCleanup::FailedSpawnRollback
                    && let Err(err) = finalize_rejected_spawn_edge(&state, thread_id).await
                {
                    if attempt <= LATE_TERMINATION_CLEANUP_QUICK_ATTEMPTS
                        || attempt.is_multiple_of(60)
                    {
                        warn!(
                            %err,
                            %thread_id,
                            attempt,
                            "late rejected-agent lifecycle graph cleanup attempt failed"
                        );
                    }
                    if attempt == LATE_TERMINATION_CLEANUP_QUICK_ATTEMPTS {
                        warn!(
                            %thread_id,
                            "late rejected-agent lifecycle graph cleanup remains quarantined and will retry in the background"
                        );
                    }
                    drop(state);
                    drop(lifecycle_guard);
                    let retry_delay = if attempt < LATE_TERMINATION_CLEANUP_QUICK_ATTEMPTS {
                        LATE_TERMINATION_CLEANUP_RETRY_DELAY
                    } else {
                        LATE_TERMINATION_CLEANUP_SLOW_RETRY_DELAY
                    };
                    tokio::time::sleep(retry_delay).await;
                    continue;
                }
                let removed_or_missing = match thread.upgrade() {
                    Some(thread) => {
                        state
                            .remove_thread_if_same_or_missing(&thread_id, &thread)
                            .await
                    }
                    None => state.get_thread(thread_id).await.is_err(),
                };
                if !removed_or_missing {
                    warn!(
                        %thread_id,
                        "skipping late lifecycle cleanup because a replacement runtime is loaded"
                    );
                    lifecycle.finish_late_cleanup(thread_id);
                    return;
                }
                if let Some(residency) = residency.upgrade() {
                    residency.remove(thread_id);
                }
                if cleanup != TerminatedThreadCleanup::UnloadResident
                    && let Some(registry) = registry.upgrade()
                {
                    registry.release_spawned_thread(thread_id);
                }
                lifecycle.finish_late_cleanup(thread_id);
                return;
            }
        });
    }

    #[cfg(test)]
    pub(crate) fn lifecycle_waiter_count_for_test(&self) -> usize {
        self.lifecycle_waiters
            .load(std::sync::atomic::Ordering::Acquire)
    }

    #[cfg(test)]
    pub(super) fn block_submission_loop_commit_for_test(&self) {
        self.submission_loop_commit_hook
            .block
            .store(true, std::sync::atomic::Ordering::Release);
    }

    #[cfg(test)]
    pub(super) async fn wait_for_submission_loop_commit_for_test(&self) {
        let permit = self
            .submission_loop_commit_hook
            .entered
            .acquire()
            .await
            .expect("submission loop commit hook should remain open");
        permit.forget();
    }

    #[cfg(test)]
    pub(super) fn release_submission_loop_commit_for_test(&self) {
        self.submission_loop_commit_hook
            .block
            .store(false, std::sync::atomic::Ordering::Release);
        self.submission_loop_commit_hook.release.add_permits(1);
    }

    #[cfg(test)]
    pub(super) fn fail_next_submission_loop_commit_for_test(&self) {
        self.submission_loop_commit_hook
            .fail_next
            .store(true, std::sync::atomic::Ordering::Release);
    }

    #[cfg(test)]
    pub(super) async fn pause_before_submission_loop_commit_for_test(&self) -> bool {
        if self
            .submission_loop_commit_hook
            .block
            .load(std::sync::atomic::Ordering::Acquire)
        {
            self.submission_loop_commit_hook.entered.add_permits(1);
            let permit = self
                .submission_loop_commit_hook
                .release
                .acquire()
                .await
                .expect("submission loop commit release hook should remain open");
            permit.forget();
        }
        self.submission_loop_commit_hook
            .fail_next
            .swap(false, std::sync::atomic::Ordering::AcqRel)
    }

    pub(super) async fn live_thread_spawn_children(
        &self,
    ) -> CodexResult<HashMap<ThreadId, Vec<(ThreadId, AgentMetadata)>>> {
        let state = self.upgrade()?;
        let mut children_by_parent = HashMap::<ThreadId, Vec<(ThreadId, AgentMetadata)>>::new();

        for (parent_thread_id, child_thread_id) in state.list_live_thread_spawn_edges().await {
            children_by_parent
                .entry(parent_thread_id)
                .or_default()
                .push((
                    child_thread_id,
                    self.state
                        .agent_metadata_for_thread(child_thread_id)
                        .unwrap_or(AgentMetadata {
                            agent_id: Some(child_thread_id),
                            ..Default::default()
                        }),
                ));
        }

        for children in children_by_parent.values_mut() {
            children.sort_by(|left, right| {
                left.1
                    .agent_path
                    .as_deref()
                    .unwrap_or_default()
                    .cmp(right.1.agent_path.as_deref().unwrap_or_default())
                    .then_with(|| left.0.to_string().cmp(&right.0.to_string()))
            });
        }

        Ok(children_by_parent)
    }

    pub(super) async fn persist_thread_spawn_edge_for_source(
        &self,
        child_thread: &crate::CodexThread,
        child_thread_id: ThreadId,
        session_source: Option<&SessionSource>,
        status: codex_agent_graph_store::ThreadSpawnEdgeStatus,
    ) -> CodexResult<()> {
        let Some(parent_thread_id) = session_source.and_then(SessionSource::parent_thread_id)
        else {
            return Ok(());
        };
        if child_thread.config_snapshot().await.ephemeral {
            return Ok(());
        }
        let state = self.upgrade()?;
        let Some(agent_graph_store) = state.agent_graph_store() else {
            return Err(CodexErr::Fatal(
                "authoritative agent graph is unavailable; refusing lifecycle transition"
                    .to_string(),
            ));
        };
        agent_graph_store
            .upsert_thread_spawn_edge(parent_thread_id, child_thread_id, status)
            .await
            .map_err(|err| {
                warn!(%err, %child_thread_id, "failed to persist thread-spawn edge");
                CodexErr::Fatal("failed to persist thread-spawn lifecycle state".to_string())
            })
    }

    pub(super) async fn live_thread_spawn_descendants(
        &self,
        root_thread_id: ThreadId,
    ) -> CodexResult<Vec<ThreadId>> {
        collect_live_thread_spawn_descendants(
            root_thread_id,
            self.live_thread_spawn_children().await?,
        )
    }
}

pub(super) fn collect_live_thread_spawn_descendants(
    root_thread_id: ThreadId,
    mut children_by_parent: HashMap<ThreadId, Vec<(ThreadId, AgentMetadata)>>,
) -> CodexResult<Vec<ThreadId>> {
    let mut visited = HashSet::from([root_thread_id]);
    let mut descendants = Vec::new();
    let mut stack = children_by_parent
        .remove(&root_thread_id)
        .unwrap_or_default()
        .into_iter()
        .map(|(child_thread_id, _)| (child_thread_id, 1))
        .rev()
        .collect::<Vec<_>>();

    while let Some((thread_id, depth)) = stack.pop() {
        if depth > MAX_AGENT_GRAPH_DEPTH {
            return Err(CodexErr::Fatal(format!(
                "live agent graph exceeds the traversal depth limit of {MAX_AGENT_GRAPH_DEPTH}"
            )));
        }
        if !visited.insert(thread_id) {
            return Err(CodexErr::Fatal(format!(
                "live agent graph contains a cycle below {root_thread_id}"
            )));
        }
        descendants.push(thread_id);
        if let Some(children) = children_by_parent.remove(&thread_id) {
            for (child_thread_id, _) in children.into_iter().rev() {
                stack.push((child_thread_id, depth + 1));
            }
        }
    }

    Ok(descendants)
}

pub(super) async fn finalize_rejected_spawn_edge(
    state: &Arc<ThreadManagerState>,
    child_thread_id: ThreadId,
) -> CodexResult<()> {
    let quarantine_result = quarantine_rejected_spawn_edge(state, child_thread_id).await;
    let persisted_parent_thread_id = match state
        .read_stored_thread(ReadThreadParams {
            thread_id: child_thread_id,
            include_archived: true,
            include_history: true,
        })
        .await
    {
        Ok(stored_thread) => Some(stored_thread.source.parent_thread_id().ok_or_else(|| {
            CodexErr::Fatal(
                "rejected agent rollout is missing its persisted parent identity".to_string(),
            )
        })?),
        Err(CodexErr::ThreadNotFound(_)) => None,
        Err(err) => {
            warn!(
                %err,
                %child_thread_id,
                "failed to inspect rejected agent rollout before graph cleanup"
            );
            return Err(CodexErr::Fatal(
                "failed to inspect rejected agent durability before lifecycle cleanup".to_string(),
            ));
        }
    };
    if let Some(parent_thread_id) = persisted_parent_thread_id {
        match quarantine_result {
            Ok(true) => return Ok(()),
            Ok(false) => {
                let Some(agent_graph_store) = state.agent_graph_store() else {
                    return Err(CodexErr::Fatal(
                        "authoritative agent graph is unavailable; rejected agent remains unquarantined"
                            .to_string(),
                    ));
                };
                agent_graph_store
                    .upsert_thread_spawn_edge(
                        parent_thread_id,
                        child_thread_id,
                        codex_agent_graph_store::ThreadSpawnEdgeStatus::PendingActivation,
                    )
                    .await
                    .map_err(|err| {
                        warn!(
                            %err,
                            %child_thread_id,
                            "failed to restore rejected thread-spawn quarantine"
                        );
                        CodexErr::Fatal(
                            "failed to restore rejected agent lifecycle quarantine".to_string(),
                        )
                    })?;
                return Ok(());
            }
            Err(err) => return Err(err),
        }
    }
    if matches!(&quarantine_result, Ok(false)) {
        return Ok(());
    }
    let Some(agent_graph_store) = state.agent_graph_store() else {
        return Ok(());
    };
    let remove_result = agent_graph_store
        .remove_thread_spawn_edge(child_thread_id)
        .await;
    match (quarantine_result, remove_result) {
        (_, Ok(())) => Ok(()),
        (Ok(_), Err(remove_err)) => {
            warn!(
                %remove_err,
                %child_thread_id,
                "failed to remove rejected thread-spawn edge"
            );
            Err(CodexErr::Fatal(
                "failed to remove rejected agent lifecycle state".to_string(),
            ))
        }
        (Err(close_err), Err(remove_err)) => {
            warn!(
                %close_err,
                %remove_err,
                %child_thread_id,
                "failed to close or remove rejected thread-spawn edge"
            );
            Err(CodexErr::Fatal(
                "failed to quarantine rejected agent lifecycle state".to_string(),
            ))
        }
    }
}

pub(super) async fn quarantine_rejected_spawn_edge(
    state: &Arc<ThreadManagerState>,
    child_thread_id: ThreadId,
) -> CodexResult<bool> {
    let Some(agent_graph_store) = state.agent_graph_store() else {
        return Ok(false);
    };
    let Some(_) = agent_graph_store
        .get_thread_spawn_parent(child_thread_id)
        .await
        .map_err(|err| {
            warn!(%err, %child_thread_id, "failed to inspect rejected thread-spawn edge");
            CodexErr::Fatal("failed to inspect rejected agent lifecycle state".to_string())
        })?
    else {
        return Ok(false);
    };
    agent_graph_store
        .set_thread_spawn_edge_status(
            child_thread_id,
            codex_agent_graph_store::ThreadSpawnEdgeStatus::PendingActivation,
        )
        .await
        .map_err(|err| {
            warn!(%err, %child_thread_id, "failed to quarantine rejected thread-spawn edge");
            CodexErr::Fatal("failed to quarantine rejected agent lifecycle state".to_string())
        })?;
    Ok(true)
}
