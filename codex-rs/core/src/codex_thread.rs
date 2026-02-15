use crate::agent::AgentStatus;
use crate::codex::Codex;
use crate::codex::SteerInputError;
use crate::error::Result as CodexResult;
use crate::features::Feature;
use crate::protocol::Event;
use crate::protocol::Op;
use crate::protocol::Submission;
use codex_protocol::approvals::ExecPolicyAmendment;
use codex_protocol::config_types::Personality;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::ReviewDecision;
use codex_protocol::protocol::SandboxPolicy;
use codex_protocol::protocol::SessionSource;
use codex_protocol::user_input::UserInput;
use std::path::PathBuf;
use tokio::sync::watch;

use crate::state_db::StateDbHandle;

#[derive(Clone, Debug)]
pub struct ThreadConfigSnapshot {
    pub model: String,
    pub model_provider_id: String,
    pub approval_policy: AskForApproval,
    pub sandbox_policy: SandboxPolicy,
    pub cwd: PathBuf,
    pub reasoning_effort: Option<ReasoningEffort>,
    pub personality: Option<Personality>,
    pub session_source: SessionSource,
}

pub struct CodexThread {
    codex: Codex,
    rollout_path: Option<PathBuf>,
}

/// Conduit for the bidirectional stream of messages that compose a thread
/// (formerly called a conversation) in Codex.
impl CodexThread {
    pub(crate) fn new(codex: Codex, rollout_path: Option<PathBuf>) -> Self {
        Self {
            codex,
            rollout_path,
        }
    }

    pub async fn submit(&self, op: Op) -> CodexResult<String> {
        self.codex.submit(op).await
    }

    pub async fn steer_input(
        &self,
        input: Vec<UserInput>,
        expected_turn_id: Option<&str>,
    ) -> Result<String, SteerInputError> {
        self.codex.steer_input(input, expected_turn_id).await
    }

    /// Use sparingly: this is intended to be removed soon.
    pub async fn submit_with_id(&self, sub: Submission) -> CodexResult<()> {
        self.codex.submit_with_id(sub).await
    }

    pub async fn next_event(&self) -> CodexResult<Event> {
        self.codex.next_event().await
    }

    pub async fn agent_status(&self) -> AgentStatus {
        self.codex.agent_status().await
    }

    pub(crate) fn subscribe_status(&self) -> watch::Receiver<AgentStatus> {
        self.codex.agent_status.clone()
    }

    pub fn rollout_path(&self) -> Option<PathBuf> {
        self.rollout_path.clone()
    }

    pub fn state_db(&self) -> Option<StateDbHandle> {
        self.codex.state_db()
    }

    pub async fn config_snapshot(&self) -> ThreadConfigSnapshot {
        self.codex.thread_config_snapshot().await
    }

    // FORK COMMIT OPEN [SA]: expose parent-thread approval routing for sub-agent cwd overrides.
    // Role: let AgentControl request approval through a thread's current user-facing turn context.
    pub(crate) async fn request_command_approval_for_current_turn(
        &self,
        call_id: String,
        command: Vec<String>,
        cwd: PathBuf,
        reason: Option<String>,
        proposed_execpolicy_amendment: Option<ExecPolicyAmendment>,
    ) -> CodexResult<ReviewDecision> {
        self.codex
            .session
            .request_command_approval_for_current_turn(
                call_id,
                command,
                cwd,
                reason,
                proposed_execpolicy_amendment,
            )
            .await
    }
    // FORK COMMIT CLOSE: expose parent-thread approval routing for sub-agent cwd overrides.

    pub fn enabled(&self, feature: Feature) -> bool {
        self.codex.enabled(feature)
    }

    #[cfg(test)]
    pub(crate) async fn set_active_turn_for_tests(&self) {
        *self.codex.session.active_turn.lock().await = Some(crate::state::ActiveTurn::default());
    }

    #[cfg(test)]
    pub(crate) async fn notify_approval_for_tests(
        &self,
        approval_id: &str,
        decision: ReviewDecision,
    ) {
        self.codex
            .session
            .notify_approval(approval_id, decision)
            .await;
    }
}
