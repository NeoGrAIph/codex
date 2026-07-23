use super::AgentControl;
use super::resolve_persisted_multi_agent_version_for_exact_v1;
use super::spawn::load_agent_model_context;
use crate::agent::AgentStatus;
use crate::codex_thread::CodexThread;
use codex_protocol::ThreadId;
use codex_protocol::error::CodexErr;
use codex_protocol::error::Result as CodexResult;
use codex_protocol::protocol::InitialHistory;
use codex_protocol::protocol::MultiAgentVersion;
use codex_protocol::protocol::Op;
use codex_protocol::protocol::ResumedHistory;
use codex_protocol::user_input::UserInput;
use codex_thread_store::ReadThreadParams;
use std::sync::Arc;
use tokio::sync::watch;

/// A projected V1 lifecycle target resolved against one concrete runtime instance.
///
/// Live targets retain the exact thread that passed the V1 check so later lifecycle effects do
/// not follow a reused UUID into a replacement V2 runtime. Persisted and missing targets retain
/// only the identity needed to preserve the native not-found and cold-resume behavior.
#[derive(Clone)]
pub(crate) enum ProjectedV1LifecycleTarget {
    Live(ValidatedV1Thread),
    Persisted(ThreadId),
    Missing(ThreadId),
}

/// A live thread whose runtime was verified as V1 when the projected call was accepted.
#[derive(Clone)]
pub(crate) struct ValidatedV1Thread {
    thread_id: ThreadId,
    thread: Arc<CodexThread>,
}

impl ProjectedV1LifecycleTarget {
    pub(crate) fn thread_id(&self) -> ThreadId {
        match self {
            Self::Live(target) => target.thread_id,
            Self::Persisted(thread_id) | Self::Missing(thread_id) => *thread_id,
        }
    }

    pub(crate) fn live_thread(&self) -> Option<&ValidatedV1Thread> {
        match self {
            Self::Live(target) => Some(target),
            Self::Persisted(_) | Self::Missing(_) => None,
        }
    }
}

impl ValidatedV1Thread {
    pub(super) fn new(thread_id: ThreadId, thread: Arc<CodexThread>) -> Self {
        Self { thread_id, thread }
    }

    pub(crate) fn thread_id(&self) -> ThreadId {
        self.thread_id
    }

    pub(super) fn thread(&self) -> &Arc<CodexThread> {
        &self.thread
    }

    pub(crate) async fn status(&self) -> AgentStatus {
        self.thread.agent_status().await
    }

    pub(crate) fn subscribe_status(&self) -> watch::Receiver<AgentStatus> {
        self.thread.subscribe_status()
    }
}

impl AgentControl {
    /// Resolve the actual live or persisted runtime selected by a projected V1 lifecycle target.
    ///
    /// Missing targets are left to the native handler so each tool preserves its existing
    /// not-found behavior. Unresolved, disabled, and V2 targets fail before lifecycle events or
    /// runtime mutation.
    pub(crate) async fn resolve_v1_lifecycle_target(
        &self,
        thread_id: ThreadId,
    ) -> CodexResult<ProjectedV1LifecycleTarget> {
        let state = self.upgrade()?;
        match state.get_thread(thread_id).await {
            Ok(thread) => {
                require_v1_runtime(thread_id, thread.multi_agent_version())?;
                return Ok(ProjectedV1LifecycleTarget::Live(ValidatedV1Thread::new(
                    thread_id, thread,
                )));
            }
            Err(CodexErr::ThreadNotFound(_)) => {}
            Err(err) => return Err(err),
        }

        let stored_thread = match state
            .read_stored_thread(ReadThreadParams {
                thread_id,
                include_archived: true,
                include_history: false,
            })
            .await
        {
            Ok(stored_thread) => stored_thread,
            Err(CodexErr::ThreadNotFound(_)) => {
                return Ok(ProjectedV1LifecycleTarget::Missing(thread_id));
            }
            Err(err) => return Err(err),
        };
        let Some(history) =
            load_agent_model_context(&state, thread_id, stored_thread.history_mode).await?
        else {
            return require_v1_runtime(thread_id, None)
                .map(|()| ProjectedV1LifecycleTarget::Persisted(thread_id));
        };
        let initial_history = InitialHistory::Resumed(ResumedHistory {
            conversation_id: thread_id,
            history: Arc::new(history),
            rollout_path: stored_thread.rollout_path,
        });
        require_v1_runtime(
            thread_id,
            resolve_persisted_multi_agent_version_for_exact_v1(
                &initial_history,
                stored_thread.history_mode,
            ),
        )?;
        Ok(ProjectedV1LifecycleTarget::Persisted(thread_id))
    }

    pub(crate) async fn validate_v1_lifecycle_target(
        &self,
        thread_id: ThreadId,
    ) -> CodexResult<()> {
        self.resolve_v1_lifecycle_target(thread_id).await.map(drop)
    }

    pub(crate) async fn projected_v1_target_status(
        &self,
        target: &ProjectedV1LifecycleTarget,
    ) -> AgentStatus {
        match target.live_thread() {
            Some(target) => target.status().await,
            None => AgentStatus::NotFound,
        }
    }

    pub(crate) fn subscribe_projected_v1_target(
        &self,
        target: &ProjectedV1LifecycleTarget,
    ) -> CodexResult<watch::Receiver<AgentStatus>> {
        target
            .live_thread()
            .map(ValidatedV1Thread::subscribe_status)
            .ok_or_else(|| CodexErr::ThreadNotFound(target.thread_id()))
    }

    pub(crate) async fn send_input_to_projected_v1_target(
        &self,
        target: &ProjectedV1LifecycleTarget,
        input: Vec<UserInput>,
    ) -> CodexResult<String> {
        let target = require_live_target(target)?;
        let state = self.upgrade()?;
        self.ensure_execution_capacity_for_bound_turn_start(
            target.thread(),
            /*starts_turn*/ true,
        )
        .await?;
        let result = state
            .send_op_to_thread(target.thread_id(), target.thread(), input.into())
            .await;
        self.handle_projected_v1_request_result(target, &state, result)
            .await
    }

    pub(crate) async fn interrupt_projected_v1_target(
        &self,
        target: &ProjectedV1LifecycleTarget,
    ) -> CodexResult<String> {
        let target = require_live_target(target)?;
        let state = self.upgrade()?;
        let result = state
            .send_op_to_thread(target.thread_id(), target.thread(), Op::Interrupt)
            .await;
        self.handle_projected_v1_request_result(target, &state, result)
            .await
    }

    async fn handle_projected_v1_request_result(
        &self,
        target: &ValidatedV1Thread,
        state: &Arc<crate::thread_manager::ThreadManagerState>,
        result: CodexResult<String>,
    ) -> CodexResult<String> {
        if matches!(result, Err(CodexErr::InternalAgentDied))
            && state
                .remove_thread_if_same_with_cleanup(&target.thread_id(), target.thread(), || {
                    self.state.release_spawned_thread(target.thread_id())
                })
                .await
                .is_some()
        {
            debug_assert_eq!(
                target.thread().multi_agent_version(),
                Some(MultiAgentVersion::V1)
            );
        }
        result
    }
}

fn require_live_target(target: &ProjectedV1LifecycleTarget) -> CodexResult<&ValidatedV1Thread> {
    target
        .live_thread()
        .ok_or_else(|| CodexErr::ThreadNotFound(target.thread_id()))
}

pub(super) fn require_v1_runtime(
    thread_id: ThreadId,
    multi_agent_version: Option<MultiAgentVersion>,
) -> CodexResult<()> {
    match multi_agent_version {
        Some(MultiAgentVersion::V1) => Ok(()),
        Some(MultiAgentVersion::Disabled | MultiAgentVersion::V2) => Err(CodexErr::InvalidRequest(
            format!("agent with id {thread_id} does not use the V1 multi-agent runtime"),
        )),
        None => Err(CodexErr::InvalidRequest(format!(
            "agent with id {thread_id} has no resolved multi-agent runtime"
        ))),
    }
}
