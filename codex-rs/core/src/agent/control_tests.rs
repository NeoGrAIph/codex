use super::resume::checked_resume_child_depth;
use super::*;
use crate::CodexThread;
use crate::StateDbHandle;
use crate::ThreadManager;
use crate::agent::agent_status_from_event;
use crate::agent::control::MAX_AGENT_GRAPH_DEPTH;
use crate::agent_communication::AgentCommunicationContext;
use crate::agent_communication::AgentCommunicationKind;
use crate::config::AgentRoleConfig;
use crate::config::Config;
use crate::config::ConfigBuilder;
use crate::context::ContextualUserFragment;
use crate::context::SubagentNotification;
use crate::init_state_db;
use crate::local_agent_graph_store_from_state_db;
use crate::thread_manager::StartThreadOptions;
use crate::thread_manager::thread_store_from_config;
use assert_matches::assert_matches;
use codex_agent_graph_store::AgentGraphStore;
use codex_agent_graph_store::AgentGraphStoreError;
use codex_agent_graph_store::AgentGraphStoreFuture;
use codex_agent_graph_store::ThreadSpawnEdgeStatus;
use codex_extension_api::ExtensionDataInit;
use codex_extension_api::empty_extension_registry;
use codex_features::Feature;
use codex_login::AuthManager;
use codex_login::CodexAuth;
use codex_protocol::AgentPath;
use codex_protocol::capabilities::CapabilityRootLocation;
use codex_protocol::capabilities::SelectedCapabilityRoot;
use codex_protocol::config_types::ModeKind;
use codex_protocol::models::ContentItem;
use codex_protocol::models::MessagePhase;
use codex_protocol::models::ResponseItem;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::protocol::CompactedItem;
use codex_protocol::protocol::ErrorEvent;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::InitialHistory;
use codex_protocol::protocol::InterAgentCommunication;
use codex_protocol::protocol::RolloutItem;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use codex_protocol::protocol::TurnAbortReason;
use codex_protocol::protocol::TurnAbortedEvent;
use codex_protocol::protocol::TurnCompleteEvent;
use codex_protocol::protocol::TurnStartedEvent;
use codex_thread_store::ArchiveThreadParams;
use codex_thread_store::InMemoryThreadStore;
use codex_thread_store::LocalThreadStore;
use codex_thread_store::LocalThreadStoreConfig;
use codex_thread_store::ReadThreadParams;
use codex_thread_store::ThreadStore;
use codex_utils_path_uri::PathUri;
use pretty_assertions::assert_eq;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use tempfile::TempDir;
use tokio::time::Duration;
use tokio::time::sleep;
use tokio::time::timeout;
use toml::Value as TomlValue;

async fn test_config_with_cli_overrides(
    mut cli_overrides: Vec<(String, TomlValue)>,
) -> (TempDir, Config) {
    let home = TempDir::new().expect("create temp dir");
    cli_overrides.push((
        "model".to_string(),
        TomlValue::String("gpt-5.5".to_string()),
    ));
    let config = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(home.path().to_path_buf())
        .cli_overrides(cli_overrides)
        .build()
        .await
        .expect("load default test config");
    (home, config)
}

async fn test_config() -> (TempDir, Config) {
    test_config_with_cli_overrides(Vec::new()).await
}

fn text_input(text: &str) -> Vec<UserInput> {
    vec![UserInput::Text {
        text: text.to_string(),
        text_elements: Vec::new(),
    }]
}

fn assistant_message(text: &str, phase: Option<MessagePhase>) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "assistant".to_string(),
        content: vec![ContentItem::OutputText {
            text: text.to_string(),
        }],
        phase,
        internal_chat_message_metadata_passthrough: None,
    }
}

#[test]
fn register_session_root_skips_threads_with_explicit_parent() {
    let control = AgentControl::default();

    control.register_session_root(ThreadId::new(), Some(ThreadId::new()));

    assert_eq!(control.state.agent_id_for_path(&AgentPath::root()), None);
}

#[tokio::test]
async fn lifecycle_termination_timeout_releases_root_gate() {
    let control = AgentControl::default();
    let gate = control.lifecycle_gate_for_test();
    let task_gate = Arc::clone(&gate);
    let thread_id = ThreadId::new();
    let task = tokio::spawn(async move {
        let _guard = task_gate.lock_owned().await;
        crate::agent::control::await_lifecycle_operation(
            thread_id,
            Duration::from_millis(1),
            std::future::pending::<codex_protocol::error::Result<()>>(),
        )
        .await
    });

    let error = task
        .await
        .expect("timeout task should join")
        .expect_err("pending termination should time out");
    assert!(matches!(
        error,
        CodexErr::Fatal(message) if message.contains("lifecycle termination")
    ));
    let _next_guard = timeout(Duration::from_millis(100), gate.lock())
        .await
        .expect("lifecycle gate should be available after timeout");
}

#[tokio::test]
async fn late_termination_cleanup_releases_runtime_and_retains_materialized_quarantine() {
    let mut harness = AgentControlHarness::new().await;
    harness
        .config
        .features
        .enable(Feature::MultiAgentV2)
        .expect("test config should enable V2");
    let (root_thread_id, root_thread) = harness.start_thread().await;
    root_thread
        .codex
        .session
        .set_multi_agent_version_if_unset(MultiAgentVersion::V2);
    let child_path = AgentPath::root()
        .join("late_cleanup")
        .expect("late-cleanup path");
    let child_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("late cleanup child"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: root_thread_id,
                depth: 1,
                agent_path: Some(child_path.clone()),
                agent_nickname: None,
                agent_role: None,
            })),
        )
        .await
        .expect("child should spawn");
    let child_thread = harness
        .manager
        .get_thread(child_thread_id)
        .await
        .expect("child thread should exist");
    let state = harness.control.upgrade().expect("manager state");

    for _ in 0..2 {
        let error = harness
            .control
            .await_thread_lifecycle_operation(
                &state,
                Arc::clone(&child_thread),
                Duration::from_millis(1),
                TerminatedThreadCleanup::FailedSpawnRollback,
                std::future::pending::<codex_protocol::error::Result<()>>(),
            )
            .await
            .expect_err("pending lifecycle cleanup should time out");
        assert!(matches!(
            error,
            CodexErr::Fatal(message) if message.contains("lifecycle termination")
        ));
    }
    assert_eq!(
        harness.control.state.agent_id_for_path(&child_path),
        Some(child_thread_id)
    );
    assert!(matches!(
        harness
            .control
            .ensure_agent_known_or_persisted_descendant(child_thread_id)
            .await,
        Err(CodexErr::Fatal(message)) if message == "agent lifecycle cleanup is still pending"
    ));

    child_thread
        .shutdown_and_wait()
        .await
        .expect("late child shutdown should complete");
    timeout(Duration::from_secs(5), async {
        loop {
            if harness.manager.get_thread(child_thread_id).await.is_err()
                && harness
                    .control
                    .state
                    .agent_metadata_for_thread(child_thread_id)
                    .is_none()
                && harness
                    .control
                    .state
                    .agent_id_for_path(&child_path)
                    .is_none()
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("late termination watcher should clean runtime and registry state");
    assert_eq!(
        harness
            .state_db
            .as_ref()
            .expect("state db")
            .get_thread_spawn_edge(child_thread_id)
            .await
            .expect("late cleanup graph lookup should succeed"),
        Some((
            root_thread_id,
            codex_state::DirectionalThreadSpawnEdgeStatus::PendingActivation,
        )),
        "materialized rejected history must remain fail-closed after late cleanup"
    );
    let reservation = harness
        .control
        .state
        .reserve_spawn_slot(Some(1))
        .expect("deduplicated cleanup must release registry accounting exactly once");
    drop(reservation);
}

#[test]
fn live_agent_graph_traversal_accepts_depth_limit_and_rejects_overflow() {
    let root_thread_id = ThreadId::new();
    let thread_ids = (0..=MAX_AGENT_GRAPH_DEPTH)
        .map(|_| ThreadId::new())
        .collect::<Vec<_>>();
    let mut children_by_parent = HashMap::new();
    let mut parent_thread_id = root_thread_id;
    for child_thread_id in thread_ids.iter().take(MAX_AGENT_GRAPH_DEPTH as usize) {
        children_by_parent.insert(
            parent_thread_id,
            vec![(
                *child_thread_id,
                AgentMetadata {
                    agent_id: Some(*child_thread_id),
                    ..Default::default()
                },
            )],
        );
        parent_thread_id = *child_thread_id;
    }

    let descendants =
        collect_live_thread_spawn_descendants(root_thread_id, children_by_parent.clone())
            .expect("live traversal should accept the exact depth limit");
    assert_eq!(
        descendants,
        thread_ids[..MAX_AGENT_GRAPH_DEPTH as usize].to_vec()
    );

    let overflow_thread_id = thread_ids[MAX_AGENT_GRAPH_DEPTH as usize];
    children_by_parent.insert(
        parent_thread_id,
        vec![(
            overflow_thread_id,
            AgentMetadata {
                agent_id: Some(overflow_thread_id),
                ..Default::default()
            },
        )],
    );
    let error = collect_live_thread_spawn_descendants(root_thread_id, children_by_parent)
        .expect_err("live traversal must reject depth beyond the limit");
    assert!(matches!(
        error,
        CodexErr::Fatal(message) if message.contains("exceeds the traversal depth limit")
    ));
}

#[test]
fn recursive_resume_depth_accepts_limit_and_rejects_overflow() {
    assert_eq!(
        checked_resume_child_depth(MAX_AGENT_GRAPH_DEPTH - 1)
            .expect("resume traversal should accept the exact depth limit"),
        MAX_AGENT_GRAPH_DEPTH
    );
    let error = checked_resume_child_depth(MAX_AGENT_GRAPH_DEPTH)
        .expect_err("resume traversal must reject depth beyond the limit");
    assert!(matches!(
        error,
        CodexErr::Fatal(message) if message.contains("exceeds the resume depth limit")
    ));
}

fn spawn_agent_call(call_id: &str) -> ResponseItem {
    ResponseItem::FunctionCall {
        id: None,
        name: "spawn_agent".to_string(),
        namespace: None,
        arguments: "{}".to_string(),
        call_id: call_id.to_string(),
        internal_chat_message_metadata_passthrough: None,
    }
}

struct AgentControlHarness {
    _home: TempDir,
    config: Config,
    state_db: Option<StateDbHandle>,
    manager: ThreadManager,
    control: AgentControl,
}

struct BlockingUpsertAgentGraphStore {
    inner: Arc<dyn AgentGraphStore>,
    block_upsert: AtomicBool,
    fail_upsert: AtomicBool,
    fail_open_upserts_remaining: AtomicUsize,
    fail_remove: AtomicBool,
    fail_removes_remaining: AtomicUsize,
    fail_status_updates_remaining: AtomicUsize,
    last_upsert_child_thread_id: std::sync::Mutex<Option<ThreadId>>,
    upsert_entered: tokio::sync::Semaphore,
    release_upsert: tokio::sync::Semaphore,
    block_status_update: AtomicBool,
    status_update_entered: tokio::sync::Semaphore,
    release_status_update: tokio::sync::Semaphore,
}

impl BlockingUpsertAgentGraphStore {
    fn new(inner: Arc<dyn AgentGraphStore>) -> Self {
        Self {
            inner,
            block_upsert: AtomicBool::new(false),
            fail_upsert: AtomicBool::new(false),
            fail_open_upserts_remaining: AtomicUsize::new(0),
            fail_remove: AtomicBool::new(false),
            fail_removes_remaining: AtomicUsize::new(0),
            fail_status_updates_remaining: AtomicUsize::new(0),
            last_upsert_child_thread_id: std::sync::Mutex::new(None),
            upsert_entered: tokio::sync::Semaphore::new(0),
            release_upsert: tokio::sync::Semaphore::new(0),
            block_status_update: AtomicBool::new(false),
            status_update_entered: tokio::sync::Semaphore::new(0),
            release_status_update: tokio::sync::Semaphore::new(0),
        }
    }
}

impl AgentGraphStore for BlockingUpsertAgentGraphStore {
    fn upsert_thread_spawn_edge(
        &self,
        parent_thread_id: ThreadId,
        child_thread_id: ThreadId,
        status: ThreadSpawnEdgeStatus,
    ) -> AgentGraphStoreFuture<'_, ()> {
        Box::pin(async move {
            *self
                .last_upsert_child_thread_id
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(child_thread_id);
            if self.fail_upsert.swap(false, Ordering::AcqRel) {
                return Err(AgentGraphStoreError::Internal {
                    message: "injected upsert failure".to_string(),
                });
            }
            if status == ThreadSpawnEdgeStatus::Open
                && self
                    .fail_open_upserts_remaining
                    .fetch_update(Ordering::AcqRel, Ordering::Acquire, |remaining| {
                        remaining.checked_sub(1)
                    })
                    .is_ok()
            {
                return Err(AgentGraphStoreError::Internal {
                    message: "injected activation promotion failure".to_string(),
                });
            }
            if self.block_upsert.load(Ordering::Acquire) {
                self.upsert_entered.add_permits(1);
                let permit = self
                    .release_upsert
                    .acquire()
                    .await
                    .expect("test upsert release semaphore should remain open");
                permit.forget();
            }
            self.inner
                .upsert_thread_spawn_edge(parent_thread_id, child_thread_id, status)
                .await
        })
    }

    fn get_thread_spawn_parent(
        &self,
        child_thread_id: ThreadId,
    ) -> AgentGraphStoreFuture<'_, Option<ThreadId>> {
        self.inner.get_thread_spawn_parent(child_thread_id)
    }

    fn get_thread_spawn_edge(
        &self,
        child_thread_id: ThreadId,
    ) -> AgentGraphStoreFuture<'_, Option<codex_agent_graph_store::ThreadSpawnEdge>> {
        self.inner.get_thread_spawn_edge(child_thread_id)
    }

    fn remove_thread_spawn_edge(&self, child_thread_id: ThreadId) -> AgentGraphStoreFuture<'_, ()> {
        Box::pin(async move {
            let fail_remaining = self
                .fail_removes_remaining
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |remaining| {
                    remaining.checked_sub(1)
                })
                .is_ok();
            if self.fail_remove.swap(false, Ordering::AcqRel) || fail_remaining {
                return Err(AgentGraphStoreError::Internal {
                    message: "injected remove failure".to_string(),
                });
            }
            self.inner.remove_thread_spawn_edge(child_thread_id).await
        })
    }

    fn set_thread_spawn_edge_status(
        &self,
        child_thread_id: ThreadId,
        status: ThreadSpawnEdgeStatus,
    ) -> AgentGraphStoreFuture<'_, ()> {
        Box::pin(async move {
            if self
                .fail_status_updates_remaining
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |remaining| {
                    remaining.checked_sub(1)
                })
                .is_ok()
            {
                return Err(AgentGraphStoreError::Internal {
                    message: "injected status update failure".to_string(),
                });
            }
            self.status_update_entered.add_permits(1);
            if self.block_status_update.load(Ordering::Acquire) {
                let permit = self
                    .release_status_update
                    .acquire()
                    .await
                    .expect("test status release semaphore should remain open");
                permit.forget();
            }
            self.inner
                .set_thread_spawn_edge_status(child_thread_id, status)
                .await
        })
    }

    fn list_thread_spawn_children(
        &self,
        parent_thread_id: ThreadId,
        status_filter: Option<ThreadSpawnEdgeStatus>,
    ) -> AgentGraphStoreFuture<'_, Vec<ThreadId>> {
        self.inner
            .list_thread_spawn_children(parent_thread_id, status_filter)
    }

    fn list_thread_spawn_descendants(
        &self,
        root_thread_id: ThreadId,
        status_filter: Option<ThreadSpawnEdgeStatus>,
    ) -> AgentGraphStoreFuture<'_, Vec<ThreadId>> {
        self.inner
            .list_thread_spawn_descendants(root_thread_id, status_filter)
    }
}

impl AgentControlHarness {
    async fn new() -> Self {
        let (home, config) = test_config().await;
        Self::new_with_config(home, config).await
    }

    async fn new_with_config(home: TempDir, config: Config) -> Self {
        let state_db = init_state_db(&config).await;
        let manager = ThreadManager::with_models_provider_home_and_state_for_tests(
            CodexAuth::from_api_key("dummy"),
            config.model_provider.clone(),
            config.codex_home.to_path_buf(),
            std::sync::Arc::new(codex_exec_server::EnvironmentManager::default_for_tests()),
            state_db.clone(),
        );
        let control = manager.agent_control();
        Self {
            _home: home,
            config,
            state_db,
            manager,
            control,
        }
    }

    async fn start_thread(&self) -> (ThreadId, Arc<CodexThread>) {
        let new_thread = self
            .manager
            .start_thread(self.config.clone())
            .await
            .expect("start thread");
        (new_thread.thread_id, new_thread.thread)
    }
}

async fn persisted_originator(thread: &CodexThread) -> String {
    thread.ensure_rollout_materialized().await;
    thread
        .flush_rollout()
        .await
        .expect("thread rollout should flush");
    let stored_thread = thread
        .read_thread(
            /*include_archived*/ true, /*include_history*/ true,
        )
        .await
        .expect("thread should be readable");
    let history = stored_thread.history.expect("history should be loaded");
    history
        .items
        .iter()
        .find_map(|item| match item {
            RolloutItem::SessionMeta(meta_line) => Some(meta_line.meta.originator.clone()),
            RolloutItem::ResponseItem(_)
            | RolloutItem::InterAgentCommunication(_)
            | RolloutItem::InterAgentCommunicationMetadata { .. }
            | RolloutItem::EventMsg(_)
            | RolloutItem::Compacted(_)
            | RolloutItem::WorldState(_)
            | RolloutItem::TurnContext(_) => None,
        })
        .expect("session metadata should be persisted")
}

async fn wait_for_lifecycle_waiters(control: &AgentControl, expected_waiters: usize) {
    timeout(Duration::from_secs(1), async {
        while control.lifecycle_waiter_count_for_test() < expected_waiters {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("competing lifecycle operation should queue on the root gate");
}

fn has_subagent_notification(history_items: &[ResponseItem]) -> bool {
    history_items.iter().any(|item| {
        let ResponseItem::Message { role, content, .. } = item else {
            return false;
        };
        if role != "user" {
            return false;
        }
        content.iter().any(|content_item| match content_item {
            ContentItem::InputText { text } | ContentItem::OutputText { text } => {
                SubagentNotification::matches_text(text)
            }
            ContentItem::InputImage { .. } => false,
        })
    })
}

/// Returns true when any message item contains `needle` in a text span.
fn history_contains_text(history_items: &[ResponseItem], needle: &str) -> bool {
    history_items.iter().any(|item| {
        let ResponseItem::Message { content, .. } = item else {
            return false;
        };
        content.iter().any(|content_item| match content_item {
            ContentItem::InputText { text } | ContentItem::OutputText { text } => {
                text.contains(needle)
            }
            ContentItem::InputImage { .. } => false,
        })
    })
}

fn history_contains_assistant_inter_agent_communication(
    history_items: &[ResponseItem],
    expected: &InterAgentCommunication,
) -> bool {
    history_items.iter().any(|item| {
        let ResponseItem::Message { role, content, .. } = item else {
            return false;
        };
        if role != "assistant" {
            return false;
        }
        content.iter().any(|content_item| match content_item {
            ContentItem::OutputText { text } => {
                serde_json::from_str::<InterAgentCommunication>(text)
                    .ok()
                    .as_ref()
                    == Some(expected)
            }
            ContentItem::InputText { .. } | ContentItem::InputImage { .. } => false,
        })
    })
}

async fn wait_for_subagent_notification(parent_thread: &Arc<CodexThread>) -> bool {
    let wait = async {
        loop {
            let history_items = parent_thread
                .codex
                .session
                .clone_history()
                .await
                .raw_items()
                .to_vec();
            if has_subagent_notification(&history_items) {
                return true;
            }
            sleep(Duration::from_millis(25)).await;
        }
    };
    // CI can take several seconds to schedule the detached completion watcher,
    // especially on slower Windows runners.
    timeout(Duration::from_secs(10), wait).await.is_ok()
}

async fn persist_thread_for_tree_resume(thread: &Arc<CodexThread>, message: &str) {
    thread
        .inject_user_message_without_turn(message.to_string())
        .await;
    thread.codex.session.ensure_rollout_materialized().await;
    thread
        .codex
        .session
        .flush_rollout()
        .await
        .expect("test thread rollout should flush");
}

async fn wait_for_live_thread_spawn_children(
    control: &AgentControl,
    parent_thread_id: ThreadId,
    expected_children: &[ThreadId],
) {
    let mut expected_children = expected_children.to_vec();
    expected_children.sort_by_key(std::string::ToString::to_string);

    timeout(Duration::from_secs(5), async {
        loop {
            let mut child_ids = control
                .open_thread_spawn_children(parent_thread_id)
                .await
                .expect("live child list should load")
                .into_iter()
                .map(|(thread_id, _)| thread_id)
                .collect::<Vec<_>>();
            child_ids.sort_by_key(std::string::ToString::to_string);
            if child_ids == expected_children {
                break;
            }
            sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("expected persisted child tree");
}

async fn assert_thread_not_loaded(manager: &ThreadManager, thread_id: ThreadId) {
    match manager.get_thread(thread_id).await {
        Err(CodexErr::ThreadNotFound(id)) => assert_eq!(id, thread_id),
        Err(err) => panic!("expected ThreadNotFound, got {err:?}"),
        Ok(_) => panic!("expected thread not to be loaded"),
    }
}

#[tokio::test]
async fn send_input_errors_when_manager_dropped() {
    let control = AgentControl::default();
    let err = control
        .send_input(
            ThreadId::new(),
            vec![UserInput::Text {
                text: "hello".to_string(),
                text_elements: Vec::new(),
            }],
        )
        .await
        .expect_err("send_input should fail without a manager");
    assert_eq!(
        err.to_string(),
        "unsupported operation: thread manager dropped"
    );
}

#[tokio::test]
async fn get_status_returns_not_found_without_manager() {
    let control = AgentControl::default();
    let got = control.get_status(ThreadId::new()).await;
    assert_eq!(got, AgentStatus::NotFound);
}

#[tokio::test]
async fn on_event_updates_status_from_task_started() {
    let status = agent_status_from_event(&EventMsg::TurnStarted(TurnStartedEvent {
        turn_id: "turn-1".to_string(),
        trace_id: None,
        started_at: None,
        model_context_window: None,
        collaboration_mode_kind: ModeKind::Default,
    }));
    assert_eq!(status, Some(AgentStatus::Running));
}

#[tokio::test]
async fn on_event_updates_status_from_task_complete() {
    let status = agent_status_from_event(&EventMsg::TurnComplete(TurnCompleteEvent {
        turn_id: "turn-1".to_string(),
        last_agent_message: Some("done".to_string()),
        completed_at: None,
        duration_ms: None,
        time_to_first_token_ms: None,
    }));
    let expected = AgentStatus::Completed(Some("done".to_string()));
    assert_eq!(status, Some(expected));
}

#[tokio::test]
async fn on_event_updates_status_from_error() {
    let status = agent_status_from_event(&EventMsg::Error(ErrorEvent {
        message: "boom".to_string(),
        codex_error_info: None,
    }));

    let expected = AgentStatus::Errored("boom".to_string());
    assert_eq!(status, Some(expected));
}

#[tokio::test]
async fn on_event_updates_status_from_turn_aborted() {
    let status = agent_status_from_event(&EventMsg::TurnAborted(TurnAbortedEvent {
        turn_id: Some("turn-1".to_string()),
        reason: TurnAbortReason::Interrupted,
        completed_at: None,
        duration_ms: None,
    }));

    let expected = AgentStatus::Interrupted;
    assert_eq!(status, Some(expected));
}

#[tokio::test]
async fn on_event_updates_status_from_shutdown_complete() {
    let status = agent_status_from_event(&EventMsg::ShutdownComplete);
    assert_eq!(status, Some(AgentStatus::Shutdown));
}

#[tokio::test]
async fn spawn_agent_errors_when_manager_dropped() {
    let control = AgentControl::default();
    let (_home, config) = test_config().await;
    let err = control
        .spawn_agent(config, text_input("hello"), /*session_source*/ None)
        .await
        .expect_err("spawn_agent should fail without a manager");
    assert_eq!(
        err.to_string(),
        "unsupported operation: thread manager dropped"
    );
}

#[tokio::test]
async fn resume_agent_errors_when_manager_dropped() {
    let control = AgentControl::default();
    let (_home, config) = test_config().await;
    let err = control
        .resume_agent_from_rollout(config, ThreadId::new(), SessionSource::Exec)
        .await
        .expect_err("resume_agent should fail without a manager");
    assert_eq!(
        err.to_string(),
        "unsupported operation: thread manager dropped"
    );
}

#[tokio::test]
async fn send_input_errors_when_thread_missing() {
    let harness = AgentControlHarness::new().await;
    let thread_id = ThreadId::new();
    let err = harness
        .control
        .send_input(
            thread_id,
            vec![UserInput::Text {
                text: "hello".to_string(),
                text_elements: Vec::new(),
            }],
        )
        .await
        .expect_err("send_input should fail for missing thread");
    assert_matches!(err, CodexErr::ThreadNotFound(id) if id == thread_id);
}

#[tokio::test]
async fn get_status_returns_not_found_for_missing_thread() {
    let harness = AgentControlHarness::new().await;
    let status = harness.control.get_status(ThreadId::new()).await;
    assert_eq!(status, AgentStatus::NotFound);
}

#[tokio::test]
async fn get_status_returns_pending_init_for_new_thread() {
    let harness = AgentControlHarness::new().await;
    let (thread_id, _) = harness.start_thread().await;
    let status = harness.control.get_status(thread_id).await;
    assert_eq!(status, AgentStatus::PendingInit);
}

#[tokio::test]
async fn subscribe_status_errors_for_missing_thread() {
    let harness = AgentControlHarness::new().await;
    let thread_id = ThreadId::new();
    let err = harness
        .control
        .subscribe_status(thread_id)
        .await
        .expect_err("subscribe_status should fail for missing thread");
    assert_matches!(err, CodexErr::ThreadNotFound(id) if id == thread_id);
}

#[tokio::test]
async fn subscribe_status_updates_on_shutdown() {
    let harness = AgentControlHarness::new().await;
    let (thread_id, thread) = harness.start_thread().await;
    let mut status_rx = harness
        .control
        .subscribe_status(thread_id)
        .await
        .expect("subscribe_status should succeed");
    assert_eq!(status_rx.borrow().clone(), AgentStatus::PendingInit);

    let _ = thread
        .submit(Op::Shutdown {})
        .await
        .expect("shutdown should submit");

    let _ = status_rx.changed().await;
    assert_eq!(status_rx.borrow().clone(), AgentStatus::Shutdown);
}

#[tokio::test]
async fn send_input_submits_user_message() {
    let harness = AgentControlHarness::new().await;
    let (thread_id, _thread) = harness.start_thread().await;

    let submission_id = harness
        .control
        .send_input(
            thread_id,
            vec![UserInput::Text {
                text: "hello from tests".to_string(),
                text_elements: Vec::new(),
            }],
        )
        .await
        .expect("send_input should succeed");
    assert!(!submission_id.is_empty());
    let expected = (
        thread_id,
        Op::UserInput {
            items: vec![UserInput::Text {
                text: "hello from tests".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            additional_context: Default::default(),
            thread_settings: Default::default(),
        },
    );
    let captured = harness
        .manager
        .captured_ops()
        .into_iter()
        .find(|entry| *entry == expected);
    assert_eq!(captured, Some(expected));
}

#[tokio::test]
async fn send_inter_agent_communication_without_turn_queues_message_without_triggering_turn() {
    let harness = AgentControlHarness::new().await;
    let (thread_id, thread) = harness.start_thread().await;
    let communication = InterAgentCommunication::new(
        AgentPath::root(),
        AgentPath::try_from("/root/worker").expect("agent path"),
        Vec::new(),
        "hello from tests".to_string(),
        /*trigger_turn*/ false,
    );

    let submission_id = harness
        .control
        .send_inter_agent_communication(
            thread_id,
            communication.clone(),
            AgentCommunicationContext::new(AgentCommunicationKind::Message, ThreadId::new()),
        )
        .await
        .expect("send_inter_agent_communication should succeed");
    assert!(!submission_id.is_empty());

    let expected = (
        thread_id,
        Op::InterAgentCommunication {
            communication: communication.clone(),
        },
    );
    let captured = harness
        .manager
        .captured_ops()
        .into_iter()
        .find(|entry| *entry == expected);
    assert_eq!(captured, Some(expected));

    timeout(Duration::from_secs(5), async {
        loop {
            if thread
                .codex
                .session
                .input_queue
                .has_pending_input(&thread.codex.session.active_turn)
                .await
            {
                break;
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("inter-agent communication should stay pending");

    let history_items = thread
        .codex
        .session
        .clone_history()
        .await
        .raw_items()
        .to_vec();
    assert!(!history_contains_assistant_inter_agent_communication(
        &history_items,
        &communication
    ));
}

#[tokio::test]
async fn ensure_v2_agent_loaded_reloads_registered_unloaded_agent() {
    let (home, mut config) = test_config().await;
    let _ = config.features.enable(Feature::MultiAgentV2);
    let _ = config.features.enable(Feature::Sqlite);
    let harness = AgentControlHarness::new_with_config(home, config).await;
    let (parent_thread_id, parent_thread) = harness.start_thread().await;
    parent_thread
        .codex
        .session
        .set_multi_agent_version_if_unset(MultiAgentVersion::V2);
    let agent_path = AgentPath::try_from("/root/worker").expect("agent path");
    let spawned_agent = harness
        .control
        .spawn_agent_with_metadata(
            harness.config.clone(),
            text_input("hello child"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: Some(agent_path.clone()),
                agent_nickname: None,
                agent_role: None,
            })),
            SpawnAgentOptions {
                parent_thread_id: Some(parent_thread_id),
                ..Default::default()
            },
        )
        .await
        .expect("spawn_agent should succeed");
    let child_thread = harness
        .manager
        .get_thread(spawned_agent.thread_id)
        .await
        .expect("child thread should exist");
    child_thread
        .inject_response_items(vec![assistant_message(
            "child persisted",
            Some(MessagePhase::FinalAnswer),
        )])
        .await
        .expect("child rollout should persist with v2 metadata");
    child_thread
        .shutdown_and_wait()
        .await
        .expect("child thread should shut down");

    assert!(
        harness
            .manager
            .remove_thread(&spawned_agent.thread_id)
            .await
            .is_some()
    );
    match harness.manager.get_thread(spawned_agent.thread_id).await {
        Err(CodexErr::ThreadNotFound(id)) => assert_eq!(id, spawned_agent.thread_id),
        Err(err) => panic!("expected ThreadNotFound, got {err:?}"),
        Ok(_) => panic!("expected thread to be removed"),
    }

    harness
        .control
        .ensure_v2_agent_loaded(harness.config.clone(), spawned_agent.thread_id)
        .await
        .expect("known v2 agent should reload");
    let _ = harness
        .manager
        .get_thread(spawned_agent.thread_id)
        .await
        .expect("reloaded child thread should exist");

    let communication = InterAgentCommunication::new(
        AgentPath::root(),
        agent_path,
        Vec::new(),
        "hello after reload".to_string(),
        /*trigger_turn*/ false,
    );
    harness
        .control
        .send_inter_agent_communication(
            spawned_agent.thread_id,
            communication.clone(),
            AgentCommunicationContext::new(AgentCommunicationKind::Message, ThreadId::new()),
        )
        .await
        .expect("send_inter_agent_communication should succeed after reload");
    let expected = (
        spawned_agent.thread_id,
        Op::InterAgentCommunication { communication },
    );
    let captured = harness
        .manager
        .captured_ops()
        .into_iter()
        .find(|entry| *entry == expected);
    assert_eq!(captured, Some(expected));
}

#[tokio::test]
async fn resume_agent_from_rollout_does_not_reopen_v2_descendants() {
    let (home, mut config) = test_config().await;
    let _ = config.features.enable(Feature::MultiAgentV2);
    let _ = config.features.enable(Feature::Sqlite);
    let harness = AgentControlHarness::new_with_config(home, config).await;
    let (parent_thread_id, parent_thread) = harness.start_thread().await;
    let worker_path = AgentPath::root().join("worker").expect("worker path");
    let worker_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("hello worker"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: Some(worker_path.clone()),
                agent_nickname: None,
                agent_role: Some("worker".to_string()),
            })),
        )
        .await
        .expect("worker spawn should succeed");
    let reviewer_path = worker_path.join("reviewer").expect("reviewer path");
    let reviewer_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("hello reviewer"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: worker_thread_id,
                depth: 2,
                agent_path: Some(reviewer_path.clone()),
                agent_nickname: None,
                agent_role: Some("reviewer".to_string()),
            })),
        )
        .await
        .expect("reviewer spawn should succeed");

    let worker_thread = harness
        .manager
        .get_thread(worker_thread_id)
        .await
        .expect("worker thread should exist");
    let reviewer_thread = harness
        .manager
        .get_thread(reviewer_thread_id)
        .await
        .expect("reviewer thread should exist");
    persist_thread_for_tree_resume(&parent_thread, "parent persisted").await;
    persist_thread_for_tree_resume(&worker_thread, "worker persisted").await;
    persist_thread_for_tree_resume(&reviewer_thread, "reviewer persisted").await;
    wait_for_live_thread_spawn_children(&harness.control, parent_thread_id, &[worker_thread_id])
        .await;
    wait_for_live_thread_spawn_children(&harness.control, worker_thread_id, &[reviewer_thread_id])
        .await;

    let report = harness
        .manager
        .shutdown_all_threads_bounded(Duration::from_secs(5))
        .await;
    assert_eq!(report.submit_failed, Vec::<ThreadId>::new());
    assert_eq!(report.timed_out, Vec::<ThreadId>::new());

    let resumed_manager = ThreadManager::with_models_provider_home_and_state_for_tests(
        CodexAuth::from_api_key("dummy"),
        harness.config.model_provider.clone(),
        harness.config.codex_home.to_path_buf(),
        std::sync::Arc::new(codex_exec_server::EnvironmentManager::default_for_tests()),
        harness.state_db.clone(),
    );
    let resumed_control = resumed_manager.agent_control();
    let resumed_parent_thread_id = resumed_control
        .resume_agent_from_rollout(
            harness.config.clone(),
            parent_thread_id,
            SessionSource::Exec,
        )
        .await
        .expect("v2 root resume should succeed");
    assert_eq!(resumed_parent_thread_id, parent_thread_id);
    assert_ne!(
        resumed_control.get_status(parent_thread_id).await,
        AgentStatus::NotFound
    );
    assert_thread_not_loaded(&resumed_manager, worker_thread_id).await;
    assert_thread_not_loaded(&resumed_manager, reviewer_thread_id).await;
}

#[tokio::test]
async fn encrypted_inter_agent_communication_clears_existing_last_task_message() {
    let harness = AgentControlHarness::new().await;
    let (parent_thread_id, _) = harness.start_thread().await;
    let agent_path = AgentPath::try_from("/root/worker").expect("agent path");
    let spawned_agent = harness
        .control
        .spawn_agent_with_metadata(
            harness.config.clone(),
            text_input("old plaintext task"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: Some(agent_path.clone()),
                agent_nickname: None,
                agent_role: None,
            })),
            SpawnAgentOptions {
                parent_thread_id: Some(parent_thread_id),
                ..Default::default()
            },
        )
        .await
        .expect("spawn_agent should succeed");
    assert_eq!(
        harness
            .control
            .state
            .agent_metadata_for_thread(spawned_agent.thread_id)
            .and_then(|metadata| metadata.last_task_message),
        Some("old plaintext task".to_string())
    );

    let communication = InterAgentCommunication::new_encrypted(
        AgentPath::root(),
        agent_path,
        Vec::new(),
        "encrypted-task".to_string(),
        /*trigger_turn*/ true,
    );
    harness
        .control
        .send_inter_agent_communication(
            spawned_agent.thread_id,
            communication,
            AgentCommunicationContext::new(AgentCommunicationKind::Followup, ThreadId::new()),
        )
        .await
        .expect("send_inter_agent_communication should succeed");

    assert_eq!(
        harness
            .control
            .state
            .agent_metadata_for_thread(spawned_agent.thread_id)
            .and_then(|metadata| metadata.last_task_message),
        None
    );
}

#[tokio::test]
async fn spawn_agent_creates_thread_and_sends_prompt() {
    let harness = AgentControlHarness::new().await;
    let thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("spawned"),
            /*session_source*/ None,
        )
        .await
        .expect("spawn_agent should succeed");
    let _thread = harness
        .manager
        .get_thread(thread_id)
        .await
        .expect("thread should be registered");
    let expected = (
        thread_id,
        Op::UserInput {
            items: vec![UserInput::Text {
                text: "spawned".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            additional_context: Default::default(),
            thread_settings: Default::default(),
        },
    );
    let captured = harness
        .manager
        .captured_ops()
        .into_iter()
        .find(|entry| *entry == expected);
    assert_eq!(captured, Some(expected));
}

#[tokio::test]
async fn ephemeral_spawn_does_not_persist_agent_graph_edge() {
    let (home, mut config) = test_config().await;
    config.ephemeral = true;
    let harness = AgentControlHarness::new_with_config(home, config).await;
    let (parent_thread_id, _parent_thread) = harness.start_thread().await;
    let child_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("spawned"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: None,
                agent_nickname: None,
                agent_role: None,
            })),
        )
        .await
        .expect("ephemeral agent spawn should succeed");

    let persisted_children = harness
        .state_db
        .as_ref()
        .expect("manager should retain state db")
        .list_thread_spawn_children(parent_thread_id)
        .await
        .expect("persisted child list should load");
    assert_eq!(persisted_children, Vec::<ThreadId>::new());
    assert!(
        harness.manager.get_thread(child_thread_id).await.is_ok(),
        "ephemeral child should remain live"
    );
}

#[tokio::test]
async fn spawn_agent_can_fork_parent_thread_history_with_sanitized_items() {
    let harness = AgentControlHarness::new().await;
    let mut parent_config = harness.config.clone();
    let _ = parent_config.features.enable(Feature::MultiAgentV2);
    parent_config.multi_agent_v2.root_agent_usage_hint_text =
        Some("Parent root guidance.".to_string());
    parent_config.multi_agent_v2.subagent_usage_hint_text =
        Some("Parent subagent guidance.".to_string());
    let mut child_config = harness.config.clone();
    let _ = child_config.features.enable(Feature::MultiAgentV2);
    child_config.multi_agent_v2.root_agent_usage_hint_text =
        Some("Child root guidance.".to_string());
    child_config.multi_agent_v2.subagent_usage_hint_text =
        Some("Child subagent guidance.".to_string());
    let new_thread = harness
        .manager
        .start_thread(parent_config.clone())
        .await
        .expect("start parent thread");
    let parent_thread_id = new_thread.thread_id;
    let parent_thread = new_thread.thread;
    parent_thread
        .inject_user_message_without_turn("parent seed context".to_string())
        .await;
    let expected_parent_seed = parent_thread
        .codex
        .session
        .clone_history()
        .await
        .raw_items()
        .first()
        .cloned()
        .expect("parent seed should be recorded");
    let turn_context = parent_thread.codex.session.new_default_turn().await;
    let parent_spawn_call_id = "spawn-call-history".to_string();
    let trigger_message = InterAgentCommunication::new(
        AgentPath::root(),
        AgentPath::try_from("/root/worker").expect("agent path"),
        Vec::new(),
        "parent trigger message".to_string(),
        /*trigger_turn*/ true,
    );
    parent_thread
        .codex
        .session
        .record_conversation_items(
            turn_context.as_ref(),
            &[
                ResponseItem::Message {
                    id: None,
                    role: "developer".to_string(),
                    content: vec![ContentItem::InputText {
                        text: "Parent root guidance.".to_string(),
                    }],
                    phase: None,
                    internal_chat_message_metadata_passthrough: None,
                },
                ResponseItem::Message {
                    id: None,
                    role: "developer".to_string(),
                    content: vec![ContentItem::InputText {
                        text: "Parent subagent guidance.".to_string(),
                    }],
                    phase: None,
                    internal_chat_message_metadata_passthrough: None,
                },
                assistant_message("parent commentary", Some(MessagePhase::Commentary)),
                assistant_message("parent final answer", Some(MessagePhase::FinalAnswer)),
                assistant_message("parent unknown phase", /*phase*/ None),
                ResponseItem::Reasoning {
                    id: Some("parent-reasoning".to_string()),
                    summary: Vec::new(),
                    content: None,
                    encrypted_content: None,
                    internal_chat_message_metadata_passthrough: None,
                },
                trigger_message.to_response_input_item().into(),
                spawn_agent_call(&parent_spawn_call_id),
            ],
        )
        .await;
    let parent_reference_context_item = turn_context.to_turn_context_item();
    parent_thread
        .codex
        .session
        .persist_rollout_items(&[RolloutItem::TurnContext(
            parent_reference_context_item.clone(),
        )])
        .await;
    parent_thread
        .codex
        .session
        .ensure_rollout_materialized()
        .await;
    parent_thread
        .codex
        .session
        .flush_rollout()
        .await
        .expect("parent rollout should flush");
    let child_thread_id = harness
        .control
        .spawn_agent_with_metadata(
            child_config,
            text_input("child task"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: None,
                agent_nickname: None,
                agent_role: None,
            })),
            SpawnAgentOptions {
                fork_parent_spawn_call_id: Some(parent_spawn_call_id.clone()),
                fork_mode: Some(SpawnAgentForkMode::FullHistory),
                ..Default::default()
            },
        )
        .await
        .expect("forked spawn should succeed")
        .thread_id;

    let child_thread = harness
        .manager
        .get_thread(child_thread_id)
        .await
        .expect("child thread should be registered");
    assert_ne!(child_thread_id, parent_thread_id);
    let history = child_thread.codex.session.clone_history().await;
    let mut expected_final_answer =
        assistant_message("parent final answer", Some(MessagePhase::FinalAnswer));
    expected_final_answer.set_turn_id_if_missing(&turn_context.sub_id);
    let expected_history = [
        expected_parent_seed,
        expected_final_answer,
        ResponseItem::Message {
            id: None,
            role: "developer".to_string(),
            content: vec![ContentItem::InputText {
                text: "Child subagent guidance.".to_string(),
            }],
            phase: None,
            internal_chat_message_metadata_passthrough: None,
        },
    ];
    assert_eq!(
        history.raw_items(),
        &expected_history,
        "full-history forked child history should replace parent usage hints with the child subagent hint while filtering non-final assistant/tool chatter"
    );
    assert_eq!(
        serde_json::to_value(child_thread.codex.session.reference_context_item().await)
            .expect("serialize child reference context item"),
        serde_json::to_value(Some(parent_reference_context_item))
            .expect("serialize expected reference context item"),
        "full-history forked child should preserve the parent diff baseline"
    );

    let mut no_hint_child_config = harness.config.clone();
    let _ = no_hint_child_config.features.enable(Feature::MultiAgentV2);
    no_hint_child_config.multi_agent_v2.subagent_usage_hint_text = None;
    let no_hint_child_thread_id = harness
        .control
        .spawn_agent_with_metadata(
            no_hint_child_config,
            text_input("child task without hints"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: None,
                agent_nickname: None,
                agent_role: None,
            })),
            SpawnAgentOptions {
                fork_parent_spawn_call_id: Some(parent_spawn_call_id.clone()),
                fork_mode: Some(SpawnAgentForkMode::FullHistory),
                ..Default::default()
            },
        )
        .await
        .expect("forked spawn should honor an empty subagent usage hint")
        .thread_id;
    let no_hint_child_thread = harness
        .manager
        .get_thread(no_hint_child_thread_id)
        .await
        .expect("no-hint child thread should be registered");
    let no_hint_history = no_hint_child_thread.codex.session.clone_history().await;
    assert!(
        !history_contains_text(no_hint_history.raw_items(), "Child subagent guidance."),
        "full-history forked child should not add empty subagent guidance"
    );

    let expected = (
        child_thread_id,
        Op::UserInput {
            items: vec![UserInput::Text {
                text: "child task".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            additional_context: Default::default(),
            thread_settings: Default::default(),
        },
    );
    let captured = harness
        .manager
        .captured_ops()
        .into_iter()
        .find(|entry| *entry == expected);
    assert_eq!(captured, Some(expected));

    let _ = harness
        .control
        .shutdown_live_agent(child_thread_id)
        .await
        .expect("child shutdown should submit");
    let _ = harness
        .control
        .shutdown_live_agent(no_hint_child_thread_id)
        .await
        .expect("no-hint child shutdown should submit");
    let _ = parent_thread
        .submit(Op::Shutdown {})
        .await
        .expect("parent shutdown should submit");
}

#[tokio::test]
async fn spawn_agent_fork_strips_parent_usage_hints_from_compacted_history() {
    let harness = AgentControlHarness::new().await;
    let mut parent_config = harness.config.clone();
    let _ = parent_config.features.enable(Feature::MultiAgentV2);
    parent_config.multi_agent_v2.root_agent_usage_hint_text =
        Some("Parent root guidance.".to_string());
    parent_config.multi_agent_v2.subagent_usage_hint_text =
        Some("Parent subagent guidance.".to_string());
    let mut child_config = harness.config.clone();
    let _ = child_config.features.enable(Feature::MultiAgentV2);
    child_config.multi_agent_v2.root_agent_usage_hint_text =
        Some("Child root guidance.".to_string());
    child_config.multi_agent_v2.subagent_usage_hint_text =
        Some("Child subagent guidance.".to_string());
    let new_thread = harness
        .manager
        .start_thread(parent_config)
        .await
        .expect("start parent thread");
    let parent_thread_id = new_thread.thread_id;
    let parent_thread = new_thread.thread;
    let turn_context = parent_thread.codex.session.new_default_turn().await;
    let parent_spawn_call_id = "spawn-call-compacted-usage-hints".to_string();
    let replacement_history = vec![
        ResponseItem::Message {
            id: None,
            role: "user".to_string(),
            content: vec![ContentItem::InputText {
                text: "compacted parent summary".to_string(),
            }],
            phase: None,
            internal_chat_message_metadata_passthrough: None,
        },
        ResponseItem::Message {
            id: None,
            role: "developer".to_string(),
            content: vec![ContentItem::InputText {
                text: "Parent root guidance.".to_string(),
            }],
            phase: None,
            internal_chat_message_metadata_passthrough: None,
        },
    ];
    parent_thread
        .codex
        .session
        .persist_rollout_items(&[
            RolloutItem::Compacted(CompactedItem {
                message: String::new(),
                replacement_history: Some(replacement_history),
                window_number: None,
                first_window_id: None,
                previous_window_id: None,
                window_id: None,
            }),
            RolloutItem::TurnContext(turn_context.to_turn_context_item()),
            RolloutItem::ResponseItem(spawn_agent_call(&parent_spawn_call_id)),
        ])
        .await;
    parent_thread
        .codex
        .session
        .ensure_rollout_materialized()
        .await;
    parent_thread
        .codex
        .session
        .flush_rollout()
        .await
        .expect("parent rollout should flush");

    let child_thread_id = harness
        .control
        .spawn_agent_with_metadata(
            child_config,
            text_input("child task"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: None,
                agent_nickname: None,
                agent_role: None,
            })),
            SpawnAgentOptions {
                fork_parent_spawn_call_id: Some(parent_spawn_call_id),
                fork_mode: Some(SpawnAgentForkMode::FullHistory),
                ..Default::default()
            },
        )
        .await
        .expect("forked spawn should sanitize compacted usage hints")
        .thread_id;

    let child_thread = harness
        .manager
        .get_thread(child_thread_id)
        .await
        .expect("child thread should be registered");
    let history = child_thread.codex.session.clone_history().await;
    assert!(
        history_contains_text(history.raw_items(), "compacted parent summary"),
        "forked child history should retain compacted non-hint content"
    );
    assert!(
        !history_contains_text(history.raw_items(), "Parent root guidance."),
        "forked child history should strip stale parent hints from compacted replacement history"
    );
    assert!(
        history_contains_text(history.raw_items(), "Child subagent guidance."),
        "full-history forked child should add the child subagent hint after compacted-history sanitization"
    );

    let _ = harness
        .control
        .shutdown_live_agent(child_thread_id)
        .await
        .expect("child shutdown should submit");
    let _ = parent_thread
        .submit(Op::Shutdown {})
        .await
        .expect("parent shutdown should submit");
}

#[tokio::test]
async fn spawn_agent_fork_flushes_parent_rollout_before_loading_history() {
    let harness = AgentControlHarness::new().await;
    let (parent_thread_id, parent_thread) = harness.start_thread().await;
    let turn_context = parent_thread.codex.session.new_default_turn().await;
    let parent_spawn_call_id = "spawn-call-unflushed".to_string();
    parent_thread
        .codex
        .session
        .record_conversation_items(
            turn_context.as_ref(),
            &[
                assistant_message("unflushed final answer", Some(MessagePhase::FinalAnswer)),
                spawn_agent_call(&parent_spawn_call_id),
            ],
        )
        .await;

    let child_thread_id = harness
        .control
        .spawn_agent_with_metadata(
            harness.config.clone(),
            text_input("child task"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: None,
                agent_nickname: None,
                agent_role: None,
            })),
            SpawnAgentOptions {
                fork_parent_spawn_call_id: Some(parent_spawn_call_id.clone()),
                fork_mode: Some(SpawnAgentForkMode::FullHistory),
                ..Default::default()
            },
        )
        .await
        .expect("forked spawn should flush parent rollout before loading history")
        .thread_id;

    let child_thread = harness
        .manager
        .get_thread(child_thread_id)
        .await
        .expect("child thread should be registered");
    let history = child_thread.codex.session.clone_history().await;
    assert!(
        history_contains_text(history.raw_items(), "unflushed final answer"),
        "forked child history should include unflushed assistant final answers after flushing the parent rollout"
    );

    let _ = harness
        .control
        .shutdown_live_agent(child_thread_id)
        .await
        .expect("child shutdown should submit");
    let _ = parent_thread
        .submit(Op::Shutdown {})
        .await
        .expect("parent shutdown should submit");
}

#[tokio::test]
async fn spawn_agent_fork_last_n_turns_keeps_only_recent_turns() {
    let harness = AgentControlHarness::new().await;
    let (parent_thread_id, parent_thread) = harness.start_thread().await;

    parent_thread
        .inject_user_message_without_turn("old parent context".to_string())
        .await;
    let queued_communication = InterAgentCommunication::new(
        AgentPath::root(),
        AgentPath::try_from("/root/worker").expect("agent path"),
        Vec::new(),
        "queued message".to_string(),
        /*trigger_turn*/ false,
    );
    let queued_turn_context = parent_thread.codex.session.new_default_turn().await;
    parent_thread
        .codex
        .session
        .record_conversation_items(
            queued_turn_context.as_ref(),
            &[queued_communication.to_response_input_item().into()],
        )
        .await;

    let triggered_communication = InterAgentCommunication::new(
        AgentPath::root(),
        AgentPath::try_from("/root/worker").expect("agent path"),
        Vec::new(),
        "triggered context".to_string(),
        /*trigger_turn*/ true,
    );
    let triggered_turn_context = parent_thread.codex.session.new_default_turn().await;
    parent_thread
        .codex
        .session
        .record_conversation_items(
            triggered_turn_context.as_ref(),
            &[triggered_communication.to_response_input_item().into()],
        )
        .await;
    parent_thread
        .inject_user_message_without_turn("current parent task".to_string())
        .await;
    let spawn_turn_context = parent_thread.codex.session.new_default_turn().await;
    let parent_spawn_call_id = "spawn-call-last-n".to_string();
    parent_thread
        .codex
        .session
        .record_conversation_items(
            spawn_turn_context.as_ref(),
            &[spawn_agent_call(&parent_spawn_call_id)],
        )
        .await;
    parent_thread
        .codex
        .session
        .persist_rollout_items(&[RolloutItem::TurnContext(
            spawn_turn_context.to_turn_context_item(),
        )])
        .await;
    parent_thread
        .codex
        .session
        .ensure_rollout_materialized()
        .await;
    parent_thread
        .codex
        .session
        .flush_rollout()
        .await
        .expect("parent rollout should flush");

    let child_thread_id = harness
        .control
        .spawn_agent_with_metadata(
            harness.config.clone(),
            text_input("child task"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: None,
                agent_nickname: None,
                agent_role: None,
            })),
            SpawnAgentOptions {
                fork_parent_spawn_call_id: Some(parent_spawn_call_id.clone()),
                fork_mode: Some(SpawnAgentForkMode::LastNTurns(2)),
                ..Default::default()
            },
        )
        .await
        .expect("forked spawn should keep only the last two turns")
        .thread_id;

    let child_thread = harness
        .manager
        .get_thread(child_thread_id)
        .await
        .expect("child thread should be registered");
    let history = child_thread.codex.session.clone_history().await;

    assert!(
        !history_contains_text(history.raw_items(), "old parent context"),
        "forked child history should drop parent context outside the requested last-N turn window"
    );
    assert!(
        !history_contains_text(history.raw_items(), "queued message"),
        "forked child history should drop queued inter-agent messages outside the requested last-N turn window"
    );
    assert!(
        !history_contains_text(history.raw_items(), "triggered context"),
        "forked child history should filter assistant inter-agent messages even when they fall inside the requested last-N turn window"
    );
    assert!(
        history_contains_text(history.raw_items(), "current parent task"),
        "forked child history should keep the parent user message from the requested last-N turn window"
    );
    assert!(
        child_thread
            .codex
            .session
            .reference_context_item()
            .await
            .is_none(),
        "last-N forked child should rebuild context after truncating the cached prefix"
    );

    let _ = harness
        .control
        .shutdown_live_agent(child_thread_id)
        .await
        .expect("child shutdown should submit");
    let _ = parent_thread
        .submit(Op::Shutdown {})
        .await
        .expect("parent shutdown should submit");
}

#[tokio::test]
async fn spawn_agent_fork_last_n_turns_drops_parent_startup_prefix_when_under_limit() {
    let harness = AgentControlHarness::new().await;
    let selected_capability_roots = vec![SelectedCapabilityRoot {
        id: "demo@1".to_string(),
        location: CapabilityRootLocation::Environment {
            environment_id: "build".to_string(),
            path: PathUri::parse("file:///plugins/demo").expect("plugin root URI"),
        },
    }];
    let mut thread_extension_init = ExtensionDataInit::new();
    thread_extension_init.insert(selected_capability_roots.clone());
    let parent = harness
        .manager
        .start_thread_with_options(StartThreadOptions {
            config: harness.config.clone(),
            allow_provider_model_fallback: false,
            initial_history: InitialHistory::New,
            history_mode: None,
            session_source: None,
            thread_source: None,
            dynamic_tools: Vec::new(),
            metrics_service_name: None,
            parent_trace: None,
            environments: Vec::new(),
            thread_extension_init,
            supports_openai_form_elicitation: false,
        })
        .await
        .expect("start parent thread");
    let parent_thread_id = parent.thread_id;
    let parent_thread = parent.thread;
    let startup_turn_context = parent_thread.codex.session.new_default_turn().await;
    parent_thread
        .codex
        .session
        .record_conversation_items(
            startup_turn_context.as_ref(),
            &[ResponseItem::Message {
                id: None,
                role: "developer".to_string(),
                content: vec![ContentItem::InputText {
                    text: "parent startup developer context".to_string(),
                }],
                phase: None,
                internal_chat_message_metadata_passthrough: None,
            }],
        )
        .await;
    parent_thread
        .inject_user_message_without_turn("current parent task".to_string())
        .await;
    let spawn_turn_context = parent_thread.codex.session.new_default_turn().await;
    let parent_spawn_call_id = "spawn-call-last-n-under-limit".to_string();
    parent_thread
        .codex
        .session
        .record_conversation_items(
            spawn_turn_context.as_ref(),
            &[spawn_agent_call(&parent_spawn_call_id)],
        )
        .await;
    parent_thread
        .codex
        .session
        .ensure_rollout_materialized()
        .await;
    parent_thread
        .codex
        .session
        .flush_rollout()
        .await
        .expect("parent rollout should flush");

    let child_thread_id = harness
        .control
        .spawn_agent_with_metadata(
            harness.config.clone(),
            text_input("child task"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: None,
                agent_nickname: None,
                agent_role: None,
            })),
            SpawnAgentOptions {
                fork_parent_spawn_call_id: Some(parent_spawn_call_id),
                fork_mode: Some(SpawnAgentForkMode::LastNTurns(2)),
                ..Default::default()
            },
        )
        .await
        .expect("bounded forked spawn should drop startup prefix")
        .thread_id;

    let child_thread = harness
        .manager
        .get_thread(child_thread_id)
        .await
        .expect("child thread should be registered");
    let history = child_thread.codex.session.clone_history().await;
    assert!(
        history_contains_text(history.raw_items(), "current parent task"),
        "bounded fork should retain the requested recent parent turn"
    );
    assert!(
        !history_contains_text(history.raw_items(), "parent startup developer context"),
        "bounded fork should drop parent startup context even when fewer turns exist than requested"
    );
    assert_eq!(
        &child_thread
            .codex
            .session
            .services
            .selected_capability_roots,
        &selected_capability_roots
    );
    assert!(
        child_thread
            .codex
            .session
            .reference_context_item()
            .await
            .is_none(),
        "bounded forked child should still rebuild context after truncating the cached prefix"
    );

    let _ = harness
        .control
        .shutdown_live_agent(child_thread_id)
        .await
        .expect("child shutdown should submit");
    let _ = parent_thread
        .submit(Op::Shutdown {})
        .await
        .expect("parent shutdown should submit");
}

#[tokio::test]
async fn spawn_agent_fork_last_n_turns_strips_parent_usage_hints() {
    let harness = AgentControlHarness::new().await;
    let mut parent_config = harness.config.clone();
    let _ = parent_config.features.enable(Feature::MultiAgentV2);
    parent_config.multi_agent_v2.root_agent_usage_hint_text =
        Some("Parent root guidance.".to_string());
    let mut child_config = harness.config.clone();
    let _ = child_config.features.enable(Feature::MultiAgentV2);
    child_config.multi_agent_v2.subagent_usage_hint_text =
        Some("Child subagent guidance.".to_string());
    let new_thread = harness
        .manager
        .start_thread(parent_config)
        .await
        .expect("start parent thread");
    let parent_thread_id = new_thread.thread_id;
    let parent_thread = new_thread.thread;
    parent_thread
        .inject_user_message_without_turn("parent task".to_string())
        .await;
    let turn_context = parent_thread.codex.session.new_default_turn().await;
    let parent_spawn_call_id = "spawn-call-last-n-usage-hints".to_string();
    parent_thread
        .codex
        .session
        .record_conversation_items(
            turn_context.as_ref(),
            &[
                ResponseItem::Message {
                    id: None,
                    role: "developer".to_string(),
                    content: vec![ContentItem::InputText {
                        text: "Parent root guidance.".to_string(),
                    }],
                    phase: None,
                    internal_chat_message_metadata_passthrough: None,
                },
                spawn_agent_call(&parent_spawn_call_id),
            ],
        )
        .await;
    parent_thread
        .codex
        .session
        .ensure_rollout_materialized()
        .await;
    parent_thread
        .codex
        .session
        .flush_rollout()
        .await
        .expect("parent rollout should flush");

    let child_thread_id = harness
        .control
        .spawn_agent_with_metadata(
            child_config,
            text_input("child task"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: None,
                agent_nickname: None,
                agent_role: None,
            })),
            SpawnAgentOptions {
                fork_parent_spawn_call_id: Some(parent_spawn_call_id),
                fork_mode: Some(SpawnAgentForkMode::LastNTurns(2)),
                ..Default::default()
            },
        )
        .await
        .expect("bounded forked spawn should sanitize parent usage hints")
        .thread_id;

    let child_thread = harness
        .manager
        .get_thread(child_thread_id)
        .await
        .expect("child thread should be registered");
    let history = child_thread.codex.session.clone_history().await;
    assert!(
        history_contains_text(history.raw_items(), "parent task"),
        "bounded fork should retain the requested recent parent turn"
    );
    assert!(
        !history_contains_text(history.raw_items(), "Parent root guidance."),
        "bounded fork should strip stale parent root hints before the child rebuilds startup context"
    );

    let _ = harness
        .control
        .shutdown_live_agent(child_thread_id)
        .await
        .expect("child shutdown should submit");
    let _ = parent_thread
        .submit(Op::Shutdown {})
        .await
        .expect("parent shutdown should submit");
}

#[tokio::test]
async fn spawn_agent_respects_max_threads_limit() {
    let max_threads = 1usize;
    let (_home, config) = test_config_with_cli_overrides(vec![(
        "agents.max_threads".to_string(),
        TomlValue::Integer(max_threads as i64),
    )])
    .await;
    let manager = ThreadManager::with_models_provider_and_home_for_tests(
        CodexAuth::from_api_key("dummy"),
        config.model_provider.clone(),
        config.codex_home.to_path_buf(),
        std::sync::Arc::new(codex_exec_server::EnvironmentManager::default_for_tests()),
    );
    let control = manager.agent_control();

    let _ = manager
        .start_thread(config.clone())
        .await
        .expect("start thread");

    let first_agent_id = control
        .spawn_agent(
            config.clone(),
            text_input("hello"),
            /*session_source*/ None,
        )
        .await
        .expect("spawn_agent should succeed");

    let err = control
        .spawn_agent(
            config,
            text_input("hello again"),
            /*session_source*/ None,
        )
        .await
        .expect_err("spawn_agent should respect max threads");
    let CodexErr::AgentLimitReached {
        max_threads: seen_max_threads,
    } = err
    else {
        panic!("expected CodexErr::AgentLimitReached");
    };
    assert_eq!(seen_max_threads, max_threads);

    let _ = control
        .shutdown_live_agent(first_agent_id)
        .await
        .expect("shutdown agent");
}

#[tokio::test]
async fn spawn_agent_releases_slot_after_shutdown() {
    let max_threads = 1usize;
    let (_home, config) = test_config_with_cli_overrides(vec![(
        "agents.max_threads".to_string(),
        TomlValue::Integer(max_threads as i64),
    )])
    .await;
    let manager = ThreadManager::with_models_provider_and_home_for_tests(
        CodexAuth::from_api_key("dummy"),
        config.model_provider.clone(),
        config.codex_home.to_path_buf(),
        std::sync::Arc::new(codex_exec_server::EnvironmentManager::default_for_tests()),
    );
    let control = manager.agent_control();

    let first_agent_id = control
        .spawn_agent(
            config.clone(),
            text_input("hello"),
            /*session_source*/ None,
        )
        .await
        .expect("spawn_agent should succeed");
    let _ = control
        .shutdown_live_agent(first_agent_id)
        .await
        .expect("shutdown agent");

    let second_agent_id = control
        .spawn_agent(
            config.clone(),
            text_input("hello again"),
            /*session_source*/ None,
        )
        .await
        .expect("spawn_agent should succeed after shutdown");
    let _ = control
        .shutdown_live_agent(second_agent_id)
        .await
        .expect("shutdown agent");
}

#[tokio::test]
async fn spawn_agent_limit_shared_across_clones() {
    let max_threads = 1usize;
    let (_home, config) = test_config_with_cli_overrides(vec![(
        "agents.max_threads".to_string(),
        TomlValue::Integer(max_threads as i64),
    )])
    .await;
    let manager = ThreadManager::with_models_provider_and_home_for_tests(
        CodexAuth::from_api_key("dummy"),
        config.model_provider.clone(),
        config.codex_home.to_path_buf(),
        std::sync::Arc::new(codex_exec_server::EnvironmentManager::default_for_tests()),
    );
    let control = manager.agent_control();
    let cloned = control.clone();

    let first_agent_id = cloned
        .spawn_agent(
            config.clone(),
            text_input("hello"),
            /*session_source*/ None,
        )
        .await
        .expect("spawn_agent should succeed");

    let err = control
        .spawn_agent(
            config,
            text_input("hello again"),
            /*session_source*/ None,
        )
        .await
        .expect_err("spawn_agent should respect shared guard");
    let CodexErr::AgentLimitReached { max_threads } = err else {
        panic!("expected CodexErr::AgentLimitReached");
    };
    assert_eq!(max_threads, 1);

    let _ = control
        .shutdown_live_agent(first_agent_id)
        .await
        .expect("shutdown agent");
}

#[tokio::test]
async fn resume_agent_respects_max_threads_limit() {
    let max_threads = 1usize;
    let (_home, config) = test_config_with_cli_overrides(vec![(
        "agents.max_threads".to_string(),
        TomlValue::Integer(max_threads as i64),
    )])
    .await;
    let manager = ThreadManager::with_models_provider_and_home_for_tests(
        CodexAuth::from_api_key("dummy"),
        config.model_provider.clone(),
        config.codex_home.to_path_buf(),
        std::sync::Arc::new(codex_exec_server::EnvironmentManager::default_for_tests()),
    );
    let control = manager.agent_control();

    let resumable_id = control
        .spawn_agent(
            config.clone(),
            text_input("hello"),
            /*session_source*/ None,
        )
        .await
        .expect("spawn_agent should succeed");
    let _ = control
        .shutdown_live_agent(resumable_id)
        .await
        .expect("shutdown resumable thread");

    let active_id = control
        .spawn_agent(
            config.clone(),
            text_input("occupy"),
            /*session_source*/ None,
        )
        .await
        .expect("spawn_agent should succeed for active slot");

    let err = control
        .resume_agent_from_rollout(config, resumable_id, SessionSource::Exec)
        .await
        .expect_err("resume should respect max threads");
    let CodexErr::AgentLimitReached {
        max_threads: seen_max_threads,
    } = err
    else {
        panic!("expected CodexErr::AgentLimitReached");
    };
    assert_eq!(seen_max_threads, max_threads);

    let _ = control
        .shutdown_live_agent(active_id)
        .await
        .expect("shutdown active thread");
}

#[tokio::test]
async fn resume_agent_releases_slot_after_resume_failure() {
    let max_threads = 1usize;
    let (_home, config) = test_config_with_cli_overrides(vec![(
        "agents.max_threads".to_string(),
        TomlValue::Integer(max_threads as i64),
    )])
    .await;
    let manager = ThreadManager::with_models_provider_and_home_for_tests(
        CodexAuth::from_api_key("dummy"),
        config.model_provider.clone(),
        config.codex_home.to_path_buf(),
        std::sync::Arc::new(codex_exec_server::EnvironmentManager::default_for_tests()),
    );
    let control = manager.agent_control();

    let _ = control
        .resume_agent_from_rollout(config.clone(), ThreadId::new(), SessionSource::Exec)
        .await
        .expect_err("resume should fail for missing rollout path");

    let resumed_id = control
        .spawn_agent(config, text_input("hello"), /*session_source*/ None)
        .await
        .expect("spawn should succeed after failed resume");
    let _ = control
        .shutdown_live_agent(resumed_id)
        .await
        .expect("shutdown resumed thread");
}

#[tokio::test]
async fn spawn_child_completion_notifies_parent_history() {
    let harness = AgentControlHarness::new().await;
    let (parent_thread_id, parent_thread) = harness.start_thread().await;

    let child_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("hello child"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: None,
                agent_nickname: None,
                agent_role: Some("explorer".to_string()),
            })),
        )
        .await
        .expect("child spawn should succeed");

    let child_thread = harness
        .manager
        .get_thread(child_thread_id)
        .await
        .expect("child thread should exist");
    let _ = child_thread
        .submit(Op::Shutdown {})
        .await
        .expect("child shutdown should submit");

    assert_eq!(wait_for_subagent_notification(&parent_thread).await, true);
}

#[tokio::test]
async fn multi_agent_v2_completion_ignores_dead_direct_parent() {
    let harness = AgentControlHarness::new().await;
    let mut config = harness.config.clone();
    let _ = config.features.enable(Feature::MultiAgentV2);
    let root = harness
        .manager
        .start_thread(config.clone())
        .await
        .expect("root thread should start");
    let root_thread_id = root.thread_id;
    let root_thread = root.thread;
    let worker_path = AgentPath::root().join("worker_a").expect("worker path");
    let worker_thread_id = harness
        .control
        .spawn_agent(
            config.clone(),
            text_input("hello worker"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: root_thread_id,
                depth: 1,
                agent_path: Some(worker_path.clone()),
                agent_nickname: None,
                agent_role: Some("explorer".to_string()),
            })),
        )
        .await
        .expect("worker spawn should succeed");
    let tester_path = worker_path.join("tester").expect("tester path");
    let tester_thread_id = harness
        .control
        .spawn_agent(
            config,
            text_input("hello tester"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: worker_thread_id,
                depth: 2,
                agent_path: Some(tester_path.clone()),
                agent_nickname: None,
                agent_role: Some("explorer".to_string()),
            })),
        )
        .await
        .expect("tester spawn should succeed");
    harness
        .control
        .shutdown_live_agent(worker_thread_id)
        .await
        .expect("worker shutdown should succeed");

    let tester_thread = harness
        .manager
        .get_thread(tester_thread_id)
        .await
        .expect("tester thread should exist");
    let tester_turn = tester_thread.codex.session.new_default_turn().await;
    tester_thread
        .codex
        .session
        .send_event(
            tester_turn.as_ref(),
            EventMsg::TurnComplete(TurnCompleteEvent {
                turn_id: tester_turn.sub_id.clone(),
                last_agent_message: Some("done".to_string()),
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            }),
        )
        .await;

    sleep(Duration::from_millis(100)).await;

    assert!(
        !harness
            .manager
            .captured_ops()
            .into_iter()
            .any(|(thread_id, op)| {
                thread_id == worker_thread_id
                    && matches!(
                        op,
                        Op::InterAgentCommunication { communication }
                            if communication.author == tester_path
                                && communication.recipient == worker_path
                                && communication.content == "done"
                    )
            })
    );

    let root_history_items = root_thread
        .codex
        .session
        .clone_history()
        .await
        .raw_items()
        .to_vec();
    assert!(!history_contains_assistant_inter_agent_communication(
        &root_history_items,
        &InterAgentCommunication::new(
            tester_path,
            AgentPath::root(),
            Vec::new(),
            "done".to_string(),
            /*trigger_turn*/ true,
        )
    ));
    assert!(!has_subagent_notification(&root_history_items));
}

#[tokio::test]
async fn multi_agent_v2_completion_queues_message_for_direct_parent() {
    let harness = AgentControlHarness::new().await;
    let (_root_thread_id, root_thread) = harness.start_thread().await;
    let (worker_thread_id, _worker_thread) = harness.start_thread().await;
    let mut tester_config = harness.config.clone();
    let _ = tester_config.features.enable(Feature::MultiAgentV2);
    let tester_thread_id = harness
        .manager
        .start_thread(tester_config.clone())
        .await
        .expect("tester thread should start")
        .thread_id;
    let tester_thread = harness
        .manager
        .get_thread(tester_thread_id)
        .await
        .expect("tester thread should exist");
    let worker_path = AgentPath::root().join("worker_a").expect("worker path");
    let tester_path = worker_path.join("tester").expect("tester path");
    harness.control.maybe_start_completion_watcher(
        tester_thread_id,
        Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id: worker_thread_id,
            depth: 2,
            agent_path: Some(tester_path.clone()),
            agent_nickname: None,
            agent_role: Some("explorer".to_string()),
        })),
        tester_path.to_string(),
        Some(tester_path.clone()),
    );
    let tester_turn = tester_thread.codex.session.new_default_turn().await;
    tester_thread
        .codex
        .session
        .send_event(
            tester_turn.as_ref(),
            EventMsg::TurnComplete(TurnCompleteEvent {
                turn_id: tester_turn.sub_id.clone(),
                last_agent_message: Some("done".to_string()),
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            }),
        )
        .await;

    let expected_message = crate::session_prefix::format_inter_agent_completion_message(
        worker_path.clone(),
        tester_path.clone(),
        &AgentStatus::Completed(Some("done".to_string())),
    )
    .expect("completed status should render");
    let expected = (
        worker_thread_id,
        Op::InterAgentCommunication {
            communication: InterAgentCommunication::new(
                tester_path.clone(),
                worker_path.clone(),
                Vec::new(),
                expected_message.clone(),
                /*trigger_turn*/ false,
            ),
        },
    );

    timeout(Duration::from_secs(5), async {
        loop {
            let captured = harness
                .manager
                .captured_ops()
                .into_iter()
                .find(|entry| *entry == expected);
            if captured == Some(expected.clone()) {
                break;
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("completion watcher should queue a direct-parent message");

    let root_history_items = root_thread
        .codex
        .session
        .clone_history()
        .await
        .raw_items()
        .to_vec();
    assert!(!history_contains_assistant_inter_agent_communication(
        &root_history_items,
        &InterAgentCommunication::new(
            tester_path,
            AgentPath::root(),
            Vec::new(),
            expected_message,
            /*trigger_turn*/ false,
        )
    ));
}

#[tokio::test]
async fn completion_watcher_notifies_parent_when_child_is_missing() {
    let harness = AgentControlHarness::new().await;
    let (parent_thread_id, parent_thread) = harness.start_thread().await;
    let child_thread_id = ThreadId::new();

    harness.control.maybe_start_completion_watcher(
        child_thread_id,
        Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id,
            depth: 1,
            agent_path: None,
            agent_nickname: None,
            agent_role: Some("explorer".to_string()),
        })),
        child_thread_id.to_string(),
        /*child_agent_path*/ None,
    );

    assert_eq!(wait_for_subagent_notification(&parent_thread).await, true);

    let history_items = parent_thread
        .codex
        .session
        .clone_history()
        .await
        .raw_items()
        .to_vec();
    assert_eq!(
        history_contains_text(
            &history_items,
            &format!("\"agent_path\":\"{child_thread_id}\"")
        ),
        true
    );
    assert_eq!(
        history_contains_text(&history_items, "\"status\":\"not_found\""),
        true
    );
}

#[tokio::test]
async fn spawn_thread_subagent_gets_random_nickname_in_session_source() {
    let harness = AgentControlHarness::new().await;
    let (parent_thread_id, _parent_thread) = harness.start_thread().await;

    let child_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("hello child"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: None,
                agent_nickname: None,
                agent_role: Some("explorer".to_string()),
            })),
        )
        .await
        .expect("child spawn should succeed");

    let child_thread = harness
        .manager
        .get_thread(child_thread_id)
        .await
        .expect("child thread should be registered");
    let snapshot = child_thread.config_snapshot().await;

    let SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
        parent_thread_id: seen_parent_thread_id,
        depth,
        agent_nickname,
        agent_role,
        ..
    }) = snapshot.session_source
    else {
        panic!("expected thread-spawn sub-agent source");
    };
    assert_eq!(seen_parent_thread_id, parent_thread_id);
    assert_eq!(depth, 1);
    assert!(agent_nickname.is_some());
    assert_eq!(agent_role, Some("explorer".to_string()));
}

#[tokio::test]
async fn spawn_thread_subagents_persist_parent_originator_across_new_and_truncated_fork() {
    let harness = AgentControlHarness::new().await;
    let parent = harness
        .manager
        .start_thread_with_options(StartThreadOptions {
            config: harness.config.clone(),
            allow_provider_model_fallback: false,
            initial_history: InitialHistory::New,
            history_mode: None,
            session_source: None,
            thread_source: None,
            dynamic_tools: Vec::new(),
            metrics_service_name: Some("codex_work_desktop".to_string()),
            parent_trace: None,
            environments: Vec::new(),
            thread_extension_init: ExtensionDataInit::default(),
            supports_openai_form_elicitation: false,
        })
        .await
        .expect("parent thread should start");
    let parent_originator = persisted_originator(&parent.thread).await;
    assert_eq!(parent_originator, "codex_work_desktop");

    let child_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("hello child"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: parent.thread_id,
                depth: 1,
                agent_path: None,
                agent_nickname: None,
                agent_role: Some("explorer".to_string()),
            })),
        )
        .await
        .expect("child spawn should succeed");

    let child_thread = harness
        .manager
        .get_thread(child_thread_id)
        .await
        .expect("child thread should be registered");
    let child_originator = persisted_originator(&child_thread).await;
    assert_eq!(child_originator, parent_originator);

    let child = harness
        .control
        .spawn_agent_with_metadata(
            harness.config.clone(),
            text_input("hello forked child"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: parent.thread_id,
                depth: 1,
                agent_path: None,
                agent_nickname: None,
                agent_role: Some("explorer".to_string()),
            })),
            SpawnAgentOptions {
                fork_parent_spawn_call_id: Some("spawn-call-last-n".to_string()),
                fork_mode: Some(SpawnAgentForkMode::LastNTurns(1)),
                ..Default::default()
            },
        )
        .await
        .expect("forked child spawn should succeed");

    let child_thread = harness
        .manager
        .get_thread(child.thread_id)
        .await
        .expect("child thread should be registered");
    let child_originator = persisted_originator(&child_thread).await;
    assert_eq!(child_originator, parent_originator);
}

#[tokio::test]
async fn spawn_thread_subagent_uses_role_specific_nickname_candidates() {
    let mut harness = AgentControlHarness::new().await;
    harness.config.agent_roles.insert(
        "researcher".to_string(),
        AgentRoleConfig {
            description: Some("Research role".to_string()),
            config_file: None,
            nickname_candidates: Some(vec!["Atlas".to_string()]),
        },
    );
    let (parent_thread_id, _parent_thread) = harness.start_thread().await;

    let child_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("hello child"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: None,
                agent_nickname: None,
                agent_role: Some("researcher".to_string()),
            })),
        )
        .await
        .expect("child spawn should succeed");

    let child_thread = harness
        .manager
        .get_thread(child_thread_id)
        .await
        .expect("child thread should be registered");
    let snapshot = child_thread.config_snapshot().await;

    let SessionSource::SubAgent(SubAgentSource::ThreadSpawn { agent_nickname, .. }) =
        snapshot.session_source
    else {
        panic!("expected thread-spawn sub-agent source");
    };
    assert_eq!(agent_nickname, Some("Atlas".to_string()));
}

#[tokio::test]
async fn resume_thread_subagent_restores_stored_metadata() {
    let (home, mut config) = test_config().await;
    config
        .features
        .enable(Feature::MultiAgentV2)
        .expect("test config should allow V2 role reconstruction");
    config
        .features
        .enable(Feature::Sqlite)
        .expect("test config should allow persisted graph reconstruction");
    let role_name = "resume-role";
    let role_path = home.path().join("resume-role.toml");
    tokio::fs::write(
        &role_path,
        r#"
model = "spawned-role-model"
approval_policy = "never"
approvals_reviewer = "auto_review"
sandbox_mode = "danger-full-access"
"#,
    )
    .await
    .expect("write resume role config");
    config.agent_roles.insert(
        role_name.to_string(),
        AgentRoleConfig {
            description: Some("Resume role".to_string()),
            config_file: Some(role_path.clone()),
            nickname_candidates: None,
        },
    );
    crate::config::agent_roles::materialize_agent_role_for_test(&mut config, role_name)
        .await
        .expect("resume role should materialize");
    let state_db = init_state_db(&config).await;
    let thread_store = thread_store_from_config(&config, state_db.clone());
    let manager = ThreadManager::new(
        &config,
        AuthManager::from_auth_for_testing(CodexAuth::from_api_key("dummy")),
        SessionSource::Exec,
        Arc::new(codex_exec_server::EnvironmentManager::default_for_tests()),
        empty_extension_registry(),
        Arc::new(crate::test_support::EmptyUserInstructionsProvider),
        /*analytics_events_client*/ None,
        thread_store.clone(),
        local_agent_graph_store_from_state_db(state_db.as_ref()),
        uuid::Uuid::new_v4().to_string(),
        /*attestation_provider*/ None,
        /*external_time_provider*/ None,
    );
    let control = manager.agent_control();
    let harness = AgentControlHarness {
        _home: home,
        config,
        state_db,
        manager,
        control,
    };
    let (parent_thread_id, parent_thread) = harness.start_thread().await;
    parent_thread.codex.session.new_default_turn().await;
    let agent_path = AgentPath::from_string("/root/resume_role".to_string())
        .expect("test agent path should be valid");

    let mut spawned_config = harness.config.clone();
    crate::agent::role::apply_role_to_config(&mut spawned_config, Some(role_name))
        .await
        .expect("spawn role should apply");
    let child_thread_id = harness
        .control
        .spawn_agent(
            spawned_config,
            text_input("hello child"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: Some(agent_path.clone()),
                agent_nickname: None,
                agent_role: Some(role_name.to_string()),
            })),
        )
        .await
        .expect("child spawn should succeed");

    let child_thread = harness
        .manager
        .get_thread(child_thread_id)
        .await
        .expect("child thread should exist");
    child_thread
        .codex
        .session
        .ensure_rollout_materialized()
        .await;
    child_thread
        .codex
        .session
        .flush_rollout()
        .await
        .expect("flush child rollout");
    let mut status_rx = harness
        .control
        .subscribe_status(child_thread_id)
        .await
        .expect("status subscription should succeed");
    if matches!(status_rx.borrow().clone(), AgentStatus::PendingInit) {
        timeout(Duration::from_secs(5), async {
            loop {
                status_rx
                    .changed()
                    .await
                    .expect("child status should advance past pending init");
                if !matches!(status_rx.borrow().clone(), AgentStatus::PendingInit) {
                    break;
                }
            }
        })
        .await
        .expect("child should initialize before shutdown");
    }
    let original_snapshot = child_thread.config_snapshot().await;
    assert_eq!(original_snapshot.model, "spawned-role-model");
    let original_nickname = original_snapshot
        .session_source
        .get_nickname()
        .expect("spawned sub-agent should have a nickname");
    timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(stored_thread) = thread_store
                .read_thread(ReadThreadParams {
                    thread_id: child_thread_id,
                    include_archived: true,
                    include_history: false,
                })
                .await
                && stored_thread.agent_nickname.is_some()
                && stored_thread.agent_role.as_deref() == Some(role_name)
                && stored_thread.agent_path.as_deref() == Some(agent_path.as_str())
            {
                break;
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("child thread metadata should be persisted to sqlite before shutdown");
    tokio::fs::write(
        &role_path,
        r#"
model = "resumed-role-model"
approval_policy = "never"
approvals_reviewer = "auto_review"
sandbox_mode = "danger-full-access"
"#,
    )
    .await
    .expect("update trusted resume role config");

    let _ = harness
        .control
        .shutdown_live_agent(child_thread_id)
        .await
        .expect("child shutdown should submit");
    assert_eq!(
        harness
            .control
            .ensure_agent_known_or_persisted_descendant(child_thread_id)
            .await
            .expect("cold child should authorize through its persisted graph edge"),
        AuthorizedAgentTarget::Persisted(PersistedAgentLineage {
            parent_thread_id,
            depth: 1,
        })
    );
    let state_db = harness.state_db.as_ref().expect("state db");
    state_db
        .remove_thread_spawn_edge(child_thread_id)
        .await
        .expect("test should remove the authoritative edge");
    let missing_edge_error = harness
        .control
        .resume_agent_from_rollout(
            harness.config.clone(),
            child_thread_id,
            SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: None,
                agent_nickname: None,
                agent_role: None,
            }),
        )
        .await
        .expect_err("V2 resume without an authoritative edge must fail closed");
    assert!(matches!(
        missing_edge_error,
        CodexErr::InvalidRequest(message)
            if message
                == "cannot resume a V2 agent without authoritative lifecycle state"
    ));
    state_db
        .upsert_thread_spawn_edge(
            parent_thread_id,
            child_thread_id,
            codex_state::DirectionalThreadSpawnEdgeStatus::Open,
        )
        .await
        .expect("test should restore the authoritative edge");

    let resumed_thread_id = harness
        .control
        .resume_agent_from_rollout(
            harness.config.clone(),
            child_thread_id,
            SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: None,
                agent_nickname: None,
                agent_role: None,
            }),
        )
        .await
        .expect("resume should succeed");
    assert_eq!(resumed_thread_id, child_thread_id);

    let resumed_snapshot = harness
        .manager
        .get_thread(resumed_thread_id)
        .await
        .expect("resumed child thread should exist")
        .config_snapshot()
        .await;
    // Role files are materialized once per loaded Config; changing the source file does not
    // mutate the trusted snapshot used by another thread in the same runtime.
    assert_eq!(resumed_snapshot.model, "spawned-role-model");
    assert_eq!(
        resumed_snapshot.approval_policy,
        harness.config.permissions.approval_policy.value()
    );
    assert_eq!(
        resumed_snapshot.approvals_reviewer,
        harness.config.approvals_reviewer
    );
    assert_eq!(
        resumed_snapshot.permission_profile,
        harness.config.permissions.permission_profile().clone()
    );
    let SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
        parent_thread_id: resumed_parent_thread_id,
        depth: resumed_depth,
        agent_path: resumed_agent_path,
        agent_nickname: resumed_nickname,
        agent_role: resumed_role,
        ..
    }) = resumed_snapshot.session_source
    else {
        panic!("expected thread-spawn sub-agent source");
    };
    assert_eq!(resumed_parent_thread_id, parent_thread_id);
    assert_eq!(resumed_depth, 1);
    assert_eq!(resumed_agent_path, Some(agent_path));
    assert_eq!(resumed_nickname, Some(original_nickname));
    assert_eq!(resumed_role, Some(role_name.to_string()));

    let _ = harness
        .control
        .shutdown_live_agent(resumed_thread_id)
        .await
        .expect("resumed child shutdown should submit");
}

#[tokio::test]
async fn v2_thread_spawn_without_authoritative_graph_fails_closed() {
    let (_home, mut config) = test_config().await;
    config
        .features
        .enable(Feature::MultiAgentV2)
        .expect("test config should allow multi-agent V2");
    let manager = ThreadManager::new(
        &config,
        AuthManager::from_auth_for_testing(CodexAuth::from_api_key("dummy")),
        SessionSource::Exec,
        Arc::new(codex_exec_server::EnvironmentManager::default_for_tests()),
        empty_extension_registry(),
        Arc::new(crate::test_support::EmptyUserInstructionsProvider),
        /*analytics_events_client*/ None,
        Arc::new(InMemoryThreadStore::default()),
        /*agent_graph_store*/ None,
        uuid::Uuid::new_v4().to_string(),
        /*attestation_provider*/ None,
        /*external_time_provider*/ None,
    );
    let root = manager
        .start_thread(config.clone())
        .await
        .expect("root thread should start");
    let control = manager.agent_control();
    let child_source = SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
        parent_thread_id: root.thread_id,
        depth: 1,
        agent_path: Some(
            AgentPath::root()
                .join("graphless_child")
                .expect("graphless child path"),
        ),
        agent_nickname: None,
        agent_role: None,
    });
    let error = control
        .spawn_agent(
            config.clone(),
            text_input("persist graphless child"),
            Some(child_source),
        )
        .await
        .expect_err("V2 spawn without an authoritative graph must fail closed");
    assert!(matches!(
        error,
        CodexErr::Fatal(message)
            if message == "authoritative agent graph is unavailable; refusing V2 agent spawn"
    ));
    assert_eq!(manager.list_thread_ids().await, vec![root.thread_id]);
    let reservation = control
        .state
        .reserve_spawn_slot(Some(1))
        .expect("preflight failure must not consume a spawn slot");
    drop(reservation);
}

#[tokio::test]
async fn pathless_v2_agent_resume_backfills_and_reuses_a_stable_path() {
    let (home, mut config) = test_config().await;
    config
        .features
        .enable(Feature::MultiAgentV2)
        .expect("test config should allow feature update");
    let harness = AgentControlHarness::new_with_config(home, config).await;
    let (parent_thread_id, parent_thread) = harness.start_thread().await;
    parent_thread
        .codex
        .session
        .set_multi_agent_version_if_unset(MultiAgentVersion::V2);
    let child_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("historical pathless child"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: None,
                agent_nickname: None,
                agent_role: None,
            })),
        )
        .await
        .expect("historical pathless child should spawn");
    let child_thread = harness
        .manager
        .get_thread(child_thread_id)
        .await
        .expect("pathless child should exist");
    persist_thread_for_tree_resume(&child_thread, "persist pathless child").await;
    harness
        .control
        .shutdown_live_agent(child_thread_id)
        .await
        .expect("pathless child should shut down");

    let first_path = AgentPath::try_from("/root/agent_first").expect("first fallback path");
    harness
        .control
        .resume_agent_from_rollout(
            harness.config.clone(),
            child_thread_id,
            SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: Some(first_path.clone()),
                agent_nickname: None,
                agent_role: None,
            }),
        )
        .await
        .expect("first resume should backfill a path");
    let first_resumed_thread = harness
        .manager
        .get_thread(child_thread_id)
        .await
        .expect("first resumed child should exist");
    assert_eq!(
        first_resumed_thread
            .config_snapshot()
            .await
            .session_source
            .get_agent_path(),
        Some(first_path.clone())
    );
    persist_thread_for_tree_resume(&first_resumed_thread, "persist backfilled path").await;
    let persisted_child = harness
        .state_db
        .as_ref()
        .expect("state db")
        .get_thread(child_thread_id)
        .await
        .expect("backfilled metadata query should succeed")
        .expect("backfilled metadata should exist");
    assert_eq!(
        persisted_child.agent_path.as_deref(),
        Some(first_path.as_str())
    );
    harness
        .control
        .shutdown_live_agent(child_thread_id)
        .await
        .expect("backfilled child should shut down");

    let different_fallback =
        AgentPath::try_from("/root/agent_second").expect("second fallback path");
    harness
        .control
        .resume_agent_from_rollout(
            harness.config.clone(),
            child_thread_id,
            SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: Some(different_fallback),
                agent_nickname: None,
                agent_role: None,
            }),
        )
        .await
        .expect("second resume should succeed");
    let second_resumed_snapshot = harness
        .manager
        .get_thread(child_thread_id)
        .await
        .expect("second resumed child should exist")
        .config_snapshot()
        .await;
    assert_eq!(
        second_resumed_snapshot.session_source.get_agent_path(),
        Some(first_path)
    );
}

#[tokio::test]
async fn nested_agent_resume_preserves_persisted_lineage_model_and_reasoning() {
    let (home, mut config) = test_config().await;
    config
        .features
        .enable(Feature::MultiAgentV2)
        .expect("test config should allow feature update");
    let harness = AgentControlHarness::new_with_config(home, config).await;
    let (root_thread_id, root_thread) = harness.start_thread().await;
    root_thread
        .codex
        .session
        .set_multi_agent_version_if_unset(MultiAgentVersion::V2);
    let parent_path = AgentPath::try_from("/root/parent").expect("parent path");
    let parent_thread_id = harness
        .control
        .spawn_agent_with_metadata(
            harness.config.clone(),
            text_input("parent"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: root_thread_id,
                depth: 1,
                agent_path: Some(parent_path.clone()),
                agent_nickname: None,
                agent_role: None,
            })),
            SpawnAgentOptions {
                parent_thread_id: Some(root_thread_id),
                ..Default::default()
            },
        )
        .await
        .expect("parent should spawn")
        .thread_id;
    let grandchild_path = parent_path.join("grandchild").expect("grandchild path");
    let grandchild_thread_id = harness
        .control
        .spawn_agent_with_metadata(
            harness.config.clone(),
            text_input("grandchild"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 2,
                agent_path: Some(grandchild_path.clone()),
                agent_nickname: None,
                agent_role: None,
            })),
            SpawnAgentOptions {
                parent_thread_id: Some(parent_thread_id),
                ..Default::default()
            },
        )
        .await
        .expect("grandchild should spawn")
        .thread_id;
    let grandchild_thread = harness
        .manager
        .get_thread(grandchild_thread_id)
        .await
        .expect("grandchild should exist");
    persist_thread_for_tree_resume(&grandchild_thread, "persist nested grandchild").await;
    grandchild_thread
        .update_thread_metadata(
            codex_thread_store::ThreadMetadataPatch {
                model: Some("persisted-agent-model".to_string()),
                reasoning_effort: Some(ReasoningEffort::High),
                source: Some(SessionSource::Unknown),
                ..Default::default()
            },
            /*include_archived*/ true,
        )
        .await
        .expect("persist effective model metadata");
    harness
        .control
        .shutdown_live_agent(grandchild_thread_id)
        .await
        .expect("grandchild should shut down");

    let mut caller_config = harness.config.clone();
    caller_config.model = Some("caller-model".to_string());
    caller_config.model_reasoning_effort = Some(ReasoningEffort::Low);
    harness
        .control
        .resume_agent_from_rollout(
            caller_config,
            grandchild_thread_id,
            SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: root_thread_id,
                depth: 1,
                agent_path: Some(
                    AgentPath::try_from("/root/agent_caller_fallback")
                        .expect("caller fallback path"),
                ),
                agent_nickname: None,
                agent_role: None,
            }),
        )
        .await
        .expect("root should be allowed to resume its nested descendant");

    let resumed_snapshot = harness
        .manager
        .get_thread(grandchild_thread_id)
        .await
        .expect("resumed grandchild should exist")
        .config_snapshot()
        .await;
    assert_eq!(resumed_snapshot.model, "persisted-agent-model");
    assert_eq!(
        resumed_snapshot.reasoning_effort,
        Some(ReasoningEffort::High)
    );
    assert_eq!(
        resumed_snapshot.session_source,
        SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id,
            depth: 2,
            agent_path: Some(grandchild_path),
            agent_nickname: resumed_snapshot.session_source.get_nickname(),
            agent_role: None,
        })
    );
}

#[tokio::test]
async fn cold_resume_rejects_removed_persisted_agent_role_without_loading_thread() {
    let (home, mut config) = test_config().await;
    config
        .features
        .enable(Feature::MultiAgentV2)
        .expect("test config should allow V2 role reconstruction");
    let harness = AgentControlHarness::new_with_config(home, config).await;
    let (parent_thread_id, parent_thread) = harness.start_thread().await;
    parent_thread.codex.session.new_default_turn().await;
    let child_path = AgentPath::try_from("/root/removed_role").expect("child path");
    let child_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("child with a role that will be unavailable on resume"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: Some(child_path.clone()),
                agent_nickname: None,
                agent_role: Some("removed-role".to_string()),
            })),
        )
        .await
        .expect("child should spawn before role reconstruction");
    let child_thread = harness
        .manager
        .get_thread(child_thread_id)
        .await
        .expect("child should exist");
    persist_thread_for_tree_resume(&child_thread, "persist unavailable role").await;
    harness
        .control
        .shutdown_live_agent(child_thread_id)
        .await
        .expect("child should shut down");

    let err = harness
        .control
        .resume_agent_from_rollout(
            harness.config.clone(),
            child_thread_id,
            SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: Some(child_path),
                agent_nickname: None,
                agent_role: None,
            }),
        )
        .await
        .expect_err("removed role should reject cold resume");

    assert!(matches!(
        err,
        CodexErr::InvalidRequest(message)
            if message
                == "failed to restore resumed agent role: unknown agent_type 'removed-role'"
    ));
    assert!(
        harness.manager.get_thread(child_thread_id).await.is_err(),
        "failed role reconstruction must not leave the child loaded"
    );
}

#[tokio::test]
async fn native_v1_cold_resume_does_not_reapply_persisted_role() {
    let (home, mut config) = test_config().await;
    config
        .features
        .disable(Feature::MultiAgentV2)
        .expect("test config should allow native V1");
    config
        .features
        .enable(Feature::Collab)
        .expect("test config should allow collaboration");
    let harness = AgentControlHarness::new_with_config(home, config).await;
    let (parent_thread_id, _parent_thread) = harness.start_thread().await;
    let child_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("native V1 child with historical role"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: None,
                agent_nickname: None,
                agent_role: Some("removed-role".to_string()),
            })),
        )
        .await
        .expect("native V1 child should spawn without resolving stored role metadata");
    let child_thread = harness
        .manager
        .get_thread(child_thread_id)
        .await
        .expect("child should exist");
    persist_thread_for_tree_resume(&child_thread, "persist native V1 role metadata").await;
    harness
        .control
        .shutdown_live_agent(child_thread_id)
        .await
        .expect("child should shut down");

    harness
        .control
        .resume_agent_from_rollout(
            harness.config.clone(),
            child_thread_id,
            SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: None,
                agent_nickname: None,
                agent_role: Some("removed-role".to_string()),
            }),
        )
        .await
        .expect("native V1 resume should preserve its historical no-reapply behavior");
    assert!(harness.manager.get_thread(child_thread_id).await.is_ok());
    harness
        .control
        .shutdown_live_agent(child_thread_id)
        .await
        .expect("test cleanup should stop child");
}

#[tokio::test]
async fn resume_agent_from_rollout_reads_archived_rollout_path() {
    let harness = AgentControlHarness::new().await;
    let child_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("hello"),
            /*session_source*/ None,
        )
        .await
        .expect("child spawn should succeed");

    let child_thread = harness
        .manager
        .get_thread(child_thread_id)
        .await
        .expect("child thread should exist");
    persist_thread_for_tree_resume(&child_thread, "persist before archiving").await;
    let _ = harness
        .control
        .shutdown_live_agent(child_thread_id)
        .await
        .expect("child shutdown should succeed");
    let store = LocalThreadStore::new(
        LocalThreadStoreConfig::from_config(&harness.config),
        harness.state_db.clone(),
    );
    store
        .archive_thread(ArchiveThreadParams {
            thread_id: child_thread_id,
        })
        .await
        .expect("child thread should archive");

    let resumed_thread_id = harness
        .control
        .resume_agent_from_rollout(harness.config.clone(), child_thread_id, SessionSource::Exec)
        .await
        .expect("resume should find archived rollout");
    assert_eq!(resumed_thread_id, child_thread_id);

    let _ = harness
        .control
        .shutdown_live_agent(child_thread_id)
        .await
        .expect("resumed child shutdown should succeed");
}

#[tokio::test]
async fn list_agent_subtree_thread_ids_includes_anonymous_and_closed_descendants() {
    let harness = AgentControlHarness::new().await;
    let (parent_thread_id, _parent_thread) = harness.start_thread().await;
    let worker_path = AgentPath::root().join("worker").expect("worker path");
    let reviewer_path = AgentPath::root().join("reviewer").expect("reviewer path");

    let worker_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("hello worker"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: Some(worker_path.clone()),
                agent_nickname: None,
                agent_role: Some("worker".to_string()),
            })),
        )
        .await
        .expect("worker spawn should succeed");
    let worker_child_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("hello worker child"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: worker_thread_id,
                depth: 2,
                agent_path: Some(
                    worker_path
                        .join("child")
                        .expect("worker child path should be valid"),
                ),
                agent_nickname: None,
                agent_role: Some("worker".to_string()),
            })),
        )
        .await
        .expect("worker child spawn should succeed");
    let no_path_child_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("hello anonymous child"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: worker_thread_id,
                depth: 2,
                agent_path: None,
                agent_nickname: None,
                agent_role: Some("worker".to_string()),
            })),
        )
        .await
        .expect("no-path child spawn should succeed");
    let no_path_grandchild_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("hello anonymous grandchild"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: no_path_child_thread_id,
                depth: 3,
                agent_path: None,
                agent_nickname: None,
                agent_role: Some("worker".to_string()),
            })),
        )
        .await
        .expect("no-path grandchild spawn should succeed");
    let _reviewer_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("hello reviewer"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: Some(reviewer_path),
                agent_nickname: None,
                agent_role: Some("reviewer".to_string()),
            })),
        )
        .await
        .expect("reviewer spawn should succeed");

    let _ = harness
        .control
        .shutdown_live_agent(no_path_grandchild_thread_id)
        .await
        .expect("no-path grandchild shutdown should succeed");

    let mut worker_subtree_thread_ids = harness
        .manager
        .list_agent_subtree_thread_ids(worker_thread_id)
        .await
        .expect("worker subtree thread ids should load");
    worker_subtree_thread_ids.sort_by_key(ToString::to_string);
    let mut expected_worker_subtree_thread_ids = vec![
        worker_thread_id,
        worker_child_thread_id,
        no_path_child_thread_id,
        no_path_grandchild_thread_id,
    ];
    expected_worker_subtree_thread_ids.sort_by_key(ToString::to_string);
    assert_eq!(
        worker_subtree_thread_ids,
        expected_worker_subtree_thread_ids
    );

    let mut no_path_child_subtree_thread_ids = harness
        .manager
        .list_agent_subtree_thread_ids(no_path_child_thread_id)
        .await
        .expect("no-path subtree thread ids should load");
    no_path_child_subtree_thread_ids.sort_by_key(ToString::to_string);
    let mut expected_no_path_child_subtree_thread_ids =
        vec![no_path_child_thread_id, no_path_grandchild_thread_id];
    expected_no_path_child_subtree_thread_ids.sort_by_key(ToString::to_string);
    assert_eq!(
        no_path_child_subtree_thread_ids,
        expected_no_path_child_subtree_thread_ids
    );
}

#[tokio::test]
async fn agent_target_authorization_is_scoped_to_the_root_control_tree() {
    let harness = AgentControlHarness::new().await;
    let (root_thread_id, _root_thread) = harness.start_thread().await;
    let child_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("owned child"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: root_thread_id,
                depth: 1,
                agent_path: Some(
                    AgentPath::root()
                        .join("owned_child")
                        .expect("owned child path"),
                ),
                agent_nickname: None,
                agent_role: None,
            })),
        )
        .await
        .expect("owned child should spawn");

    let foreign_harness = AgentControlHarness::new().await;
    let (foreign_thread_id, _foreign_thread) = foreign_harness.start_thread().await;

    assert_eq!(
        harness
            .control
            .ensure_agent_known_or_persisted_descendant(child_thread_id)
            .await
            .expect("owned live child should be authorized"),
        AuthorizedAgentTarget::Live
    );
    assert!(matches!(
        harness
            .control
            .ensure_agent_known_or_persisted_descendant(foreign_thread_id)
            .await,
        Err(CodexErr::ThreadNotFound(id)) if id == foreign_thread_id
    ));

    harness
        .control
        .close_agent(child_thread_id)
        .await
        .expect("owned child should close");
    let fresh_control = harness.manager.agent_control();
    fresh_control.register_session_root(root_thread_id, /*current_parent_thread_id*/ None);
    assert_eq!(
        fresh_control
            .ensure_agent_known_or_persisted_descendant(child_thread_id)
            .await
            .expect(
                "fresh root control should authorize its closed child from the persisted graph"
            ),
        AuthorizedAgentTarget::Persisted(PersistedAgentLineage {
            parent_thread_id: root_thread_id,
            depth: 1,
        })
    );
    harness
        .state_db
        .as_ref()
        .expect("state db")
        .set_thread_spawn_edge_status(
            child_thread_id,
            codex_state::DirectionalThreadSpawnEdgeStatus::PendingActivation,
        )
        .await
        .expect("closed child should become pending for central resume gate test");
    let resume_error = fresh_control
        .resume_agent_from_rollout(harness.config.clone(), child_thread_id, SessionSource::Exec)
        .await
        .expect_err("central thread manager must reject pending agent activation");
    assert!(matches!(
        resume_error,
        CodexErr::InvalidRequest(message)
            if message
                == "cannot resume an agent whose initial task was not durably activated"
    ));
    harness
        .state_db
        .as_ref()
        .expect("state db")
        .set_thread_spawn_edge_status(
            child_thread_id,
            codex_state::DirectionalThreadSpawnEdgeStatus::Closed,
        )
        .await
        .expect("central resume gate test should restore the closed edge");
    let pending_thread_id = ThreadId::new();
    harness
        .state_db
        .as_ref()
        .expect("state db")
        .upsert_thread_spawn_edge(
            root_thread_id,
            pending_thread_id,
            codex_state::DirectionalThreadSpawnEdgeStatus::PendingActivation,
        )
        .await
        .expect("pending activation edge should persist");
    assert!(matches!(
        fresh_control
            .ensure_agent_known_or_persisted_descendant(pending_thread_id)
            .await,
        Err(CodexErr::ThreadNotFound(thread_id)) if thread_id == pending_thread_id
    ));
}

#[tokio::test]
async fn persisted_authorization_from_resumed_subtree_uses_absolute_graph_depth() {
    let harness = AgentControlHarness::new().await;
    let (root_thread_id, _root_thread) = harness.start_thread().await;
    let resumed_subtree_root_id = ThreadId::new();
    let descendant_thread_id = ThreadId::new();
    let state_db = harness.state_db.as_ref().expect("state db");
    state_db
        .upsert_thread_spawn_edge(
            root_thread_id,
            resumed_subtree_root_id,
            codex_state::DirectionalThreadSpawnEdgeStatus::Open,
        )
        .await
        .expect("subtree-root edge should persist");
    state_db
        .upsert_thread_spawn_edge(
            resumed_subtree_root_id,
            descendant_thread_id,
            codex_state::DirectionalThreadSpawnEdgeStatus::Open,
        )
        .await
        .expect("descendant edge should persist");

    let resumed_subtree_control = harness.manager.agent_control();
    resumed_subtree_control.register_session_root(
        resumed_subtree_root_id,
        /*current_parent_thread_id*/ None,
    );
    assert_eq!(
        resumed_subtree_control
            .ensure_agent_known_or_persisted_descendant(descendant_thread_id)
            .await
            .expect("resumed subtree should authorize its persisted descendant"),
        AuthorizedAgentTarget::Persisted(PersistedAgentLineage {
            parent_thread_id: resumed_subtree_root_id,
            depth: 2,
        })
    );
}

#[tokio::test]
async fn persisted_graph_resume_revalidates_authoritative_lineage_under_lifecycle_gate() {
    let harness = AgentControlHarness::new().await;
    let (root_thread_id, _root_thread) = harness.start_thread().await;
    let child_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("owned child"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: root_thread_id,
                depth: 1,
                agent_path: None,
                agent_nickname: None,
                agent_role: None,
            })),
        )
        .await
        .expect("owned child should spawn");
    harness
        .control
        .close_agent(child_thread_id)
        .await
        .expect("owned child should close");

    let fresh_control = harness.manager.agent_control();
    fresh_control.register_session_root(root_thread_id, /*current_parent_thread_id*/ None);
    let error = fresh_control
        .resume_agent_from_persisted_graph_edge(
            harness.config.clone(),
            child_thread_id,
            SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: ThreadId::new(),
                depth: 1,
                agent_path: None,
                agent_nickname: None,
                agent_role: None,
            }),
        )
        .await
        .expect_err("stale caller lineage must not reach rollout loading");

    assert!(matches!(
        error,
        CodexErr::InvalidRequest(message)
            if message == "persisted agent lineage changed while preparing resume"
    ));
}

#[tokio::test]
async fn persisted_agent_authorization_rejects_graph_cycles() {
    let harness = AgentControlHarness::new().await;
    let (root_thread_id, _root_thread) = harness.start_thread().await;
    let child_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("cycle child"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: root_thread_id,
                depth: 1,
                agent_path: Some(
                    AgentPath::root()
                        .join("cycle_child")
                        .expect("cycle child path"),
                ),
                agent_nickname: None,
                agent_role: None,
            })),
        )
        .await
        .expect("cycle child should spawn");
    let state_db = harness.state_db.as_ref().expect("state db");
    harness
        .control
        .close_agent(child_thread_id)
        .await
        .expect("cycle child should close before persisted authorization");
    state_db
        .upsert_thread_spawn_edge(
            child_thread_id,
            root_thread_id,
            codex_state::DirectionalThreadSpawnEdgeStatus::Open,
        )
        .await
        .expect("cycle-closing edge should be persisted for the corruption test");

    let fresh_control = harness.manager.agent_control();
    fresh_control.register_session_root(root_thread_id, /*current_parent_thread_id*/ None);
    let err = fresh_control
        .ensure_agent_known_or_persisted_descendant(child_thread_id)
        .await
        .expect_err("cyclic graph must fail closed");

    assert!(matches!(
        err,
        CodexErr::Fatal(message) if message.contains("persisted agent graph contains a cycle")
    ));
}

#[tokio::test]
async fn persisted_agent_authorization_accepts_depth_limit_and_rejects_overflow() {
    let harness = AgentControlHarness::new().await;
    let (root_thread_id, _root_thread) = harness.start_thread().await;
    let state_db = harness.state_db.as_ref().expect("state db");
    let mut parent_thread_id = root_thread_id;
    let mut target_parent_thread_id = root_thread_id;
    let mut target_thread_id = root_thread_id;
    for depth in 1..=MAX_AGENT_GRAPH_DEPTH {
        let child_thread_id = ThreadId::new();
        state_db
            .upsert_thread_spawn_edge(
                parent_thread_id,
                child_thread_id,
                codex_state::DirectionalThreadSpawnEdgeStatus::Open,
            )
            .await
            .expect("boundary chain edge should persist");
        if depth == MAX_AGENT_GRAPH_DEPTH {
            target_parent_thread_id = parent_thread_id;
            target_thread_id = child_thread_id;
        }
        parent_thread_id = child_thread_id;
    }

    let fresh_control = harness.manager.agent_control();
    fresh_control.register_session_root(root_thread_id, /*current_parent_thread_id*/ None);
    assert_eq!(
        fresh_control
            .ensure_agent_known_or_persisted_descendant(target_thread_id)
            .await
            .expect("authorization depth limit should remain valid"),
        AuthorizedAgentTarget::Persisted(PersistedAgentLineage {
            parent_thread_id: target_parent_thread_id,
            depth: MAX_AGENT_GRAPH_DEPTH,
        })
    );

    let overflow_thread_id = ThreadId::new();
    state_db
        .upsert_thread_spawn_edge(
            target_thread_id,
            overflow_thread_id,
            codex_state::DirectionalThreadSpawnEdgeStatus::Open,
        )
        .await
        .expect("overflow edge should persist");
    let error = fresh_control
        .ensure_agent_known_or_persisted_descendant(overflow_thread_id)
        .await
        .expect_err("authorization depth overflow must fail closed");
    assert!(matches!(
        error,
        CodexErr::Fatal(message) if message.contains("authorization depth limit")
    ));
}

#[tokio::test]
async fn spawn_publication_waits_for_open_activation() {
    let (_home, mut config) = test_config().await;
    config
        .features
        .enable(Feature::MultiAgentV2)
        .expect("test config should allow V2 activation");
    let state_db = init_state_db(&config).await;
    let inner_graph_store = local_agent_graph_store_from_state_db(state_db.as_ref())
        .expect("state db should provide a graph store");
    let graph_store = Arc::new(BlockingUpsertAgentGraphStore::new(inner_graph_store));
    let manager = ThreadManager::new(
        &config,
        AuthManager::from_auth_for_testing(CodexAuth::from_api_key("dummy")),
        SessionSource::Exec,
        Arc::new(codex_exec_server::EnvironmentManager::default_for_tests()),
        empty_extension_registry(),
        Arc::new(crate::test_support::EmptyUserInstructionsProvider),
        /*analytics_events_client*/ None,
        thread_store_from_config(&config, state_db),
        Some(graph_store.clone()),
        uuid::Uuid::new_v4().to_string(),
        /*attestation_provider*/ None,
        /*external_time_provider*/ None,
    );
    let control = manager.agent_control();
    let root_thread = manager
        .start_thread(config.clone())
        .await
        .expect("root thread should start");
    root_thread
        .thread
        .codex
        .session
        .set_multi_agent_version_if_unset(MultiAgentVersion::V2);
    let root_thread_id = root_thread.thread_id;
    root_thread
        .thread
        .inject_user_message_without_turn("fork publication seed".to_string())
        .await;
    root_thread
        .thread
        .codex
        .session
        .ensure_rollout_materialized()
        .await;
    root_thread
        .thread
        .codex
        .session
        .flush_rollout()
        .await
        .expect("fork source rollout should flush");
    let agent_path = AgentPath::root()
        .join("publication_guard")
        .expect("agent path");

    control.block_submission_loop_commit_for_test();
    graph_store.block_upsert.store(true, Ordering::Release);
    let spawn_task = tokio::spawn({
        let control = control.clone();
        let config = config.clone();
        let agent_path = agent_path.clone();
        async move {
            control
                .spawn_agent_with_metadata(
                    config,
                    text_input("publish only after activation"),
                    Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                        parent_thread_id: root_thread_id,
                        depth: 1,
                        agent_path: Some(agent_path),
                        agent_nickname: None,
                        agent_role: None,
                    })),
                    SpawnAgentOptions {
                        fork_parent_spawn_call_id: Some("publication-guard-spawn".to_string()),
                        fork_mode: Some(SpawnAgentForkMode::FullHistory),
                        ..Default::default()
                    },
                )
                .await
                .map(|agent| agent.thread_id)
        }
    });
    let pending_upsert = timeout(Duration::from_secs(5), graph_store.upsert_entered.acquire())
        .await
        .expect("spawn should reach pending edge upsert")
        .expect("upsert semaphore should remain open");
    pending_upsert.forget();
    let child_thread_id = graph_store
        .last_upsert_child_thread_id
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .expect("pending upsert should expose the child id");
    let state = control.upgrade().expect("thread manager state");
    assert_eq!(
        graph_store
            .get_thread_spawn_edge(child_thread_id)
            .await
            .expect("materialized fork edge lookup should succeed")
            .expect("materialized fork should already have a pending edge")
            .status,
        ThreadSpawnEdgeStatus::PendingActivation
    );
    assert!(
        !state
            .get_thread(child_thread_id)
            .await
            .expect("provisional runtime should exist")
            .is_initial_task_published()
    );
    assert!(matches!(
        manager.get_thread(child_thread_id).await,
        Err(CodexErr::ThreadNotFound(thread_id)) if thread_id == child_thread_id
    ));
    assert!(!manager.list_thread_ids().await.contains(&child_thread_id));
    assert_eq!(control.state.agent_id_for_path(&agent_path), None);

    graph_store.release_upsert.add_permits(1);
    let open_upsert = timeout(Duration::from_secs(5), graph_store.upsert_entered.acquire())
        .await
        .expect("spawn should reach open edge upsert")
        .expect("upsert semaphore should remain open");
    open_upsert.forget();
    assert_eq!(
        graph_store
            .get_thread_spawn_edge(child_thread_id)
            .await
            .expect("pending edge lookup should succeed")
            .expect("pending edge should exist")
            .status,
        ThreadSpawnEdgeStatus::PendingActivation
    );
    assert!(
        !state
            .get_thread(child_thread_id)
            .await
            .expect("pending runtime should exist")
            .is_initial_task_published()
    );
    assert!(matches!(
        manager.get_thread(child_thread_id).await,
        Err(CodexErr::ThreadNotFound(thread_id)) if thread_id == child_thread_id
    ));
    assert!(!manager.list_thread_ids().await.contains(&child_thread_id));
    assert_eq!(control.state.agent_id_for_path(&agent_path), None);

    graph_store.block_upsert.store(false, Ordering::Release);
    graph_store.release_upsert.add_permits(1);
    timeout(
        Duration::from_secs(5),
        control.wait_for_submission_loop_commit_for_test(),
    )
    .await
    .expect("spawn should reach submission-loop commit");
    assert_eq!(
        graph_store
            .get_thread_spawn_edge(child_thread_id)
            .await
            .expect("open edge lookup should succeed")
            .expect("open edge should exist")
            .status,
        ThreadSpawnEdgeStatus::Open
    );
    assert_eq!(
        control.state.agent_id_for_path(&agent_path),
        Some(child_thread_id),
        "registry identity must be committed before the child loop is released"
    );
    assert!(
        control.is_v2_resident_for_test(child_thread_id),
        "residency must be committed before the child loop is released"
    );
    assert!(
        !state
            .get_thread(child_thread_id)
            .await
            .expect("pre-publication runtime should exist")
            .is_initial_task_published()
    );
    assert!(matches!(
        manager.get_thread(child_thread_id).await,
        Err(CodexErr::ThreadNotFound(thread_id)) if thread_id == child_thread_id
    ));
    control.release_submission_loop_commit_for_test();
    assert_eq!(
        timeout(Duration::from_secs(5), spawn_task)
            .await
            .expect("spawn should finish")
            .expect("spawn task should join")
            .expect("spawn should succeed"),
        child_thread_id
    );
    assert_eq!(
        graph_store
            .get_thread_spawn_edge(child_thread_id)
            .await
            .expect("open edge lookup should succeed")
            .expect("open edge should exist")
            .status,
        ThreadSpawnEdgeStatus::Open
    );
    assert!(
        manager
            .get_thread(child_thread_id)
            .await
            .expect("published child should be externally visible")
            .is_initial_task_published()
    );
    assert!(manager.list_thread_ids().await.contains(&child_thread_id));
    assert_eq!(
        control.state.agent_id_for_path(&agent_path),
        Some(child_thread_id)
    );
}

#[tokio::test]
async fn latch_commit_failure_rolls_back_and_retains_materialized_pending_quarantine() {
    let mut harness = AgentControlHarness::new().await;
    harness
        .config
        .features
        .enable(Feature::MultiAgentV2)
        .expect("test config should enable V2");
    let (root_thread_id, root_thread) = harness.start_thread().await;
    root_thread
        .codex
        .session
        .set_multi_agent_version_if_unset(MultiAgentVersion::V2);
    root_thread
        .inject_user_message_without_turn("commit failure parent seed".to_string())
        .await;
    root_thread
        .codex
        .session
        .ensure_rollout_materialized()
        .await;
    root_thread
        .codex
        .session
        .flush_rollout()
        .await
        .expect("fork source rollout should flush");
    let agent_path = AgentPath::root()
        .join("commit_failure")
        .expect("agent path");
    let rejected_task = "commit failure task must not execute";

    harness.control.block_submission_loop_commit_for_test();
    let spawn_task = tokio::spawn({
        let control = harness.control.clone();
        let config = harness.config.clone();
        let agent_path = agent_path.clone();
        async move {
            control
                .spawn_agent_with_metadata(
                    config,
                    text_input(rejected_task),
                    Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                        parent_thread_id: root_thread_id,
                        depth: 1,
                        agent_path: Some(agent_path),
                        agent_nickname: None,
                        agent_role: None,
                    })),
                    SpawnAgentOptions {
                        fork_parent_spawn_call_id: Some("commit-failure-spawn".to_string()),
                        fork_mode: Some(SpawnAgentForkMode::FullHistory),
                        ..Default::default()
                    },
                )
                .await
        }
    });
    timeout(
        Duration::from_secs(5),
        harness.control.wait_for_submission_loop_commit_for_test(),
    )
    .await
    .expect("spawn should reach submission-loop commit");
    let child_thread_id = harness
        .control
        .state
        .agent_id_for_path(&agent_path)
        .expect("registry should be committed before the latch");
    let manager_state = harness.control.upgrade().expect("manager state");
    assert_eq!(
        harness
            .state_db
            .as_ref()
            .expect("state db")
            .get_thread_spawn_edge(child_thread_id)
            .await
            .expect("open edge should load"),
        Some((
            root_thread_id,
            codex_state::DirectionalThreadSpawnEdgeStatus::Open,
        ))
    );
    assert!(harness.control.is_v2_resident_for_test(child_thread_id));
    assert!(
        !manager_state
            .get_thread(child_thread_id)
            .await
            .expect("provisional runtime should exist")
            .is_initial_task_published()
    );
    assert!(matches!(
        harness.manager.get_thread(child_thread_id).await,
        Err(CodexErr::ThreadNotFound(thread_id)) if thread_id == child_thread_id
    ));

    harness.control.fail_next_submission_loop_commit_for_test();
    harness.control.release_submission_loop_commit_for_test();
    let error = timeout(Duration::from_secs(5), spawn_task)
        .await
        .expect("failed commit spawn should finish")
        .expect("spawn task should join")
        .expect_err("injected commit failure should reject spawn");
    assert!(matches!(error, CodexErr::InternalAgentDied));
    assert!(manager_state.get_thread(child_thread_id).await.is_err());
    assert!(
        !harness
            .manager
            .list_thread_ids()
            .await
            .contains(&child_thread_id)
    );
    assert_eq!(harness.control.state.agent_id_for_path(&agent_path), None);
    assert!(!harness.control.is_v2_resident_for_test(child_thread_id));
    assert_eq!(
        harness
            .state_db
            .as_ref()
            .expect("state db")
            .get_thread_spawn_edge(child_thread_id)
            .await
            .expect("quarantined edge should load"),
        Some((
            root_thread_id,
            codex_state::DirectionalThreadSpawnEdgeStatus::PendingActivation,
        )),
        "a materialized rejected fork must retain restart-durable quarantine"
    );
    let stored_thread = manager_state
        .read_stored_thread(ReadThreadParams {
            thread_id: child_thread_id,
            include_archived: true,
            include_history: true,
        })
        .await
        .expect("materialized rejected fork should remain readable internally");
    assert!(
        stored_thread
            .history
            .expect("rejected fork history should load")
            .items
            .iter()
            .all(|item| match item {
                RolloutItem::EventMsg(EventMsg::UserMessage(event)) => {
                    event.message != rejected_task
                }
                RolloutItem::ResponseItem(ResponseItem::Message { role, content, .. })
                    if role == "user" =>
                {
                    !content.iter().any(|item| {
                        matches!(item, ContentItem::InputText { text } if text == rejected_task)
                    })
                }
                _ => true,
            }),
        "the rejected initial task must be drained before submission"
    );

    let reopened_state = codex_state::StateRuntime::init(
        harness.config.sqlite_home.clone(),
        harness.config.model_provider_id.clone(),
    )
    .await
    .expect("state db should reopen");
    assert_eq!(
        reopened_state
            .get_thread_spawn_edge(child_thread_id)
            .await
            .expect("reopened edge should load"),
        Some((
            root_thread_id,
            codex_state::DirectionalThreadSpawnEdgeStatus::PendingActivation,
        ))
    );
    let fresh_control = harness.manager.agent_control();
    fresh_control.register_session_root(root_thread_id, /*current_parent_thread_id*/ None);
    assert!(matches!(
        fresh_control
            .ensure_agent_known_or_persisted_descendant(child_thread_id)
            .await,
        Err(CodexErr::ThreadNotFound(thread_id)) if thread_id == child_thread_id
    ));
}

#[tokio::test]
async fn native_v1_spawn_keeps_graph_persistence_best_effort_and_skips_v2_latch() {
    let (_home, config) = test_config().await;
    let state_db = init_state_db(&config).await;
    let inner_graph_store = local_agent_graph_store_from_state_db(state_db.as_ref())
        .expect("state db should provide a graph store");
    let graph_store = Arc::new(BlockingUpsertAgentGraphStore::new(inner_graph_store));
    let manager = ThreadManager::new(
        &config,
        AuthManager::from_auth_for_testing(CodexAuth::from_api_key("dummy")),
        SessionSource::Exec,
        Arc::new(codex_exec_server::EnvironmentManager::default_for_tests()),
        empty_extension_registry(),
        Arc::new(crate::test_support::EmptyUserInstructionsProvider),
        /*analytics_events_client*/ None,
        thread_store_from_config(&config, state_db),
        Some(graph_store.clone()),
        uuid::Uuid::new_v4().to_string(),
        /*attestation_provider*/ None,
        /*external_time_provider*/ None,
    );
    let control = manager.agent_control();
    let root_thread_id = manager
        .start_thread(config.clone())
        .await
        .expect("root thread should start")
        .thread_id;
    let agent_path = AgentPath::root()
        .join("native_v1_compatibility")
        .expect("agent path");

    control.block_submission_loop_commit_for_test();
    graph_store.fail_upsert.store(true, Ordering::Release);
    let spawned = timeout(
        Duration::from_secs(5),
        control.spawn_agent(
            config,
            text_input("native V1 input"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: root_thread_id,
                depth: 1,
                agent_path: Some(agent_path.clone()),
                agent_nickname: None,
                agent_role: None,
            })),
        ),
    )
    .await
    .expect("native V1 spawn must not wait for the V2 submission latch")
    .expect("native V1 graph persistence must remain best effort");
    control.release_submission_loop_commit_for_test();

    assert_eq!(control.state.agent_id_for_path(&agent_path), Some(spawned));
    assert!(manager.list_thread_ids().await.contains(&spawned));
    assert!(manager.get_thread(spawned).await.is_ok());
}

#[tokio::test]
async fn spawn_and_close_are_serialized_within_root_control() {
    let (home, mut config) = test_config().await;
    config.agent_max_depth = 3;
    config
        .features
        .enable(Feature::MultiAgentV2)
        .expect("test config should allow V2 activation");
    let state_db = init_state_db(&config).await;
    let inner_graph_store = local_agent_graph_store_from_state_db(state_db.as_ref())
        .expect("state db should provide a graph store");
    let graph_store = Arc::new(BlockingUpsertAgentGraphStore::new(inner_graph_store));
    let manager = ThreadManager::new(
        &config,
        AuthManager::from_auth_for_testing(CodexAuth::from_api_key("dummy")),
        SessionSource::Exec,
        Arc::new(codex_exec_server::EnvironmentManager::default_for_tests()),
        empty_extension_registry(),
        Arc::new(crate::test_support::EmptyUserInstructionsProvider),
        /*analytics_events_client*/ None,
        thread_store_from_config(&config, state_db.clone()),
        Some(graph_store.clone()),
        uuid::Uuid::new_v4().to_string(),
        /*attestation_provider*/ None,
        /*external_time_provider*/ None,
    );
    let control = manager.agent_control();
    let root_thread = manager
        .start_thread(config.clone())
        .await
        .expect("root thread should start");
    root_thread
        .thread
        .codex
        .session
        .set_multi_agent_version_if_unset(MultiAgentVersion::V2);
    let root_thread_id = root_thread.thread_id;
    let parent_path = AgentPath::root()
        .join("serialized_parent")
        .expect("parent path");
    let parent_thread_id = control
        .spawn_agent(
            config.clone(),
            text_input("start parent"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: root_thread_id,
                depth: 1,
                agent_path: Some(parent_path.clone()),
                agent_nickname: None,
                agent_role: None,
            })),
        )
        .await
        .expect("parent should spawn");

    assert!(control.lifecycle.register_late_cleanup(
        parent_thread_id,
        TerminatedThreadCleanup::FailedSpawnRollback,
    ));
    let pending_parent_error = control
        .spawn_agent(
            config.clone(),
            text_input("must not start under quarantined parent"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 2,
                agent_path: Some(
                    parent_path
                        .join("quarantined_grandchild")
                        .expect("quarantined grandchild path"),
                ),
                agent_nickname: None,
                agent_role: None,
            })),
        )
        .await
        .expect_err("spawn must reject a parent with pending lifecycle cleanup");
    assert!(matches!(
        pending_parent_error,
        CodexErr::Fatal(message) if message == "agent lifecycle cleanup is still pending"
    ));
    control.lifecycle.finish_late_cleanup(parent_thread_id);
    assert!(
        graph_store
            .list_thread_spawn_children(parent_thread_id, /*status_filter*/ None)
            .await
            .expect("quarantined parent graph children should load")
            .is_empty()
    );

    graph_store.block_upsert.store(true, Ordering::Release);
    let spawn_task = tokio::spawn({
        let control = control.clone();
        let config = config.clone();
        async move {
            control
                .spawn_agent(
                    config,
                    text_input("start grandchild"),
                    Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                        parent_thread_id,
                        depth: 2,
                        agent_path: Some(parent_path.join("grandchild").expect("grandchild path")),
                        agent_nickname: None,
                        agent_role: None,
                    })),
                )
                .await
        }
    });
    let upsert_entered = timeout(Duration::from_secs(2), graph_store.upsert_entered.acquire())
        .await
        .expect("grandchild spawn should reach graph upsert")
        .expect("upsert semaphore should remain open");
    upsert_entered.forget();

    let close_task = tokio::spawn({
        let control = control.clone();
        async move { control.close_agent_transactional(parent_thread_id).await }
    });
    wait_for_lifecycle_waiters(&control, /*expected_waiters*/ 1).await;
    assert!(
        graph_store.status_update_entered.try_acquire().is_err(),
        "close must wait for the in-flight spawn lifecycle transaction"
    );

    graph_store.block_upsert.store(false, Ordering::Release);
    graph_store.release_upsert.add_permits(1);
    let grandchild_thread_id = timeout(Duration::from_secs(5), spawn_task)
        .await
        .expect("grandchild spawn should finish")
        .expect("grandchild spawn task should join")
        .expect("grandchild should spawn before close acquires the lifecycle gate");
    timeout(Duration::from_secs(5), close_task)
        .await
        .expect("close should finish")
        .expect("close task should join")
        .expect("close should succeed");

    assert!(manager.get_thread(parent_thread_id).await.is_err());
    assert!(manager.get_thread(grandchild_thread_id).await.is_err());
    assert_eq!(
        control.get_status(parent_thread_id).await,
        AgentStatus::NotFound
    );
    assert_eq!(
        control.get_status(grandchild_thread_id).await,
        AgentStatus::NotFound
    );

    while let Ok(permit) = graph_store.status_update_entered.try_acquire() {
        permit.forget();
    }
    let second_parent_path = AgentPath::root()
        .join("closing_parent")
        .expect("second parent path");
    let second_parent_thread_id = control
        .spawn_agent(
            config.clone(),
            text_input("start second parent"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: root_thread_id,
                depth: 1,
                agent_path: Some(second_parent_path.clone()),
                agent_nickname: None,
                agent_role: None,
            })),
        )
        .await
        .expect("second parent should spawn");
    graph_store
        .block_status_update
        .store(true, Ordering::Release);
    let close_task = tokio::spawn({
        let control = control.clone();
        async move {
            control
                .close_agent_transactional(second_parent_thread_id)
                .await
        }
    });
    let status_entered = timeout(
        Duration::from_secs(2),
        graph_store.status_update_entered.acquire(),
    )
    .await
    .expect("close should reach graph status update")
    .expect("status semaphore should remain open");
    status_entered.forget();

    graph_store.block_upsert.store(true, Ordering::Release);
    let blocked_spawn_task = tokio::spawn({
        let control = control.clone();
        let config = config.clone();
        async move {
            control
                .spawn_agent(
                    config,
                    text_input("must not start after close"),
                    Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                        parent_thread_id: second_parent_thread_id,
                        depth: 2,
                        agent_path: Some(
                            second_parent_path
                                .join("rejected_grandchild")
                                .expect("rejected grandchild path"),
                        ),
                        agent_nickname: None,
                        agent_role: None,
                    })),
                )
                .await
        }
    });
    wait_for_lifecycle_waiters(&control, /*expected_waiters*/ 1).await;
    assert!(
        graph_store.upsert_entered.try_acquire().is_err(),
        "spawn must wait while close owns the lifecycle transaction"
    );
    let blocked_delivery_task = tokio::spawn({
        let control = control.clone();
        async move {
            control
                .send_input_transactional(
                    second_parent_thread_id,
                    text_input("must not deliver after close"),
                )
                .await
        }
    });
    wait_for_lifecycle_waiters(&control, /*expected_waiters*/ 2).await;

    graph_store
        .block_status_update
        .store(false, Ordering::Release);
    graph_store.release_status_update.add_permits(1);
    timeout(Duration::from_secs(5), close_task)
        .await
        .expect("second close should finish")
        .expect("second close task should join")
        .expect("second close should succeed");
    graph_store.block_upsert.store(false, Ordering::Release);
    let spawn_error = timeout(Duration::from_secs(5), blocked_spawn_task)
        .await
        .expect("blocked spawn should finish after close")
        .expect("blocked spawn task should join")
        .expect_err("spawn must reject a parent closed before it acquired the lifecycle gate");
    assert!(matches!(
        spawn_error,
        CodexErr::ThreadNotFound(thread_id) if thread_id == second_parent_thread_id
    ));
    let delivery_error = timeout(Duration::from_secs(5), blocked_delivery_task)
        .await
        .expect("blocked delivery should finish after close")
        .expect("blocked delivery task should join")
        .expect_err("delivery must reject an agent closed before it acquired the lifecycle gate");
    assert!(matches!(
        delivery_error,
        CodexErr::ThreadNotFound(thread_id) if thread_id == second_parent_thread_id
    ));
    assert!(
        graph_store
            .list_thread_spawn_children(second_parent_thread_id, /*status_filter*/ None)
            .await
            .expect("second parent graph children should load")
            .is_empty()
    );
    drop(home);
}

#[tokio::test]
async fn spawn_lifecycle_rollbacks_preserve_runtime_graph_and_registry_invariants() {
    let (home, mut config) = test_config().await;
    config
        .features
        .enable(Feature::MultiAgentV2)
        .expect("test config should allow V2 activation");
    let state_db = init_state_db(&config).await;
    let inner_graph_store = local_agent_graph_store_from_state_db(state_db.as_ref())
        .expect("state db should provide a graph store");
    let graph_store = Arc::new(BlockingUpsertAgentGraphStore::new(inner_graph_store));
    let manager = ThreadManager::new(
        &config,
        AuthManager::from_auth_for_testing(CodexAuth::from_api_key("dummy")),
        SessionSource::Exec,
        Arc::new(codex_exec_server::EnvironmentManager::default_for_tests()),
        empty_extension_registry(),
        Arc::new(crate::test_support::EmptyUserInstructionsProvider),
        /*analytics_events_client*/ None,
        thread_store_from_config(&config, state_db),
        Some(graph_store.clone()),
        uuid::Uuid::new_v4().to_string(),
        /*attestation_provider*/ None,
        /*external_time_provider*/ None,
    );
    let control = manager.agent_control();
    let root_thread = manager
        .start_thread(config.clone())
        .await
        .expect("root thread should start");
    root_thread
        .thread
        .codex
        .session
        .set_multi_agent_version_if_unset(MultiAgentVersion::V2);
    let root_thread_id = root_thread.thread_id;

    let upsert_failure_path = AgentPath::root()
        .join("upsert_failure")
        .expect("upsert failure path");
    graph_store.fail_upsert.store(true, Ordering::Release);
    let upsert_error = control
        .spawn_agent(
            config.clone(),
            text_input("fail edge persistence"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: root_thread_id,
                depth: 1,
                agent_path: Some(upsert_failure_path.clone()),
                agent_nickname: None,
                agent_role: None,
            })),
        )
        .await
        .expect_err("injected upsert failure should reject spawn");
    assert!(matches!(
        upsert_error,
        CodexErr::Fatal(message) if message == "failed to persist thread-spawn lifecycle state"
    ));
    assert_eq!(manager.list_thread_ids().await, vec![root_thread_id]);
    assert_eq!(control.state.agent_id_for_path(&upsert_failure_path), None);

    let children_before_input_failure = graph_store
        .list_thread_spawn_children(root_thread_id, /*status_filter*/ None)
        .await
        .expect("root graph children should load");
    let input_failure_path = AgentPath::root()
        .join("input_failure")
        .expect("input failure path");
    control.fail_next_initial_input_for_test();
    let input_error = control
        .spawn_agent(
            config.clone(),
            text_input("fail initial input"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: root_thread_id,
                depth: 1,
                agent_path: Some(input_failure_path.clone()),
                agent_nickname: None,
                agent_role: None,
            })),
        )
        .await
        .expect_err("injected input failure should reject spawn");
    assert!(
        matches!(
            &input_error,
            CodexErr::Fatal(message) if message == "injected initial input failure"
        ),
        "unexpected initial-input rollback error: {input_error:?}"
    );
    assert_eq!(manager.list_thread_ids().await, vec![root_thread_id]);
    assert_eq!(control.state.agent_id_for_path(&input_failure_path), None);
    assert_eq!(
        graph_store
            .list_thread_spawn_children(root_thread_id, /*status_filter*/ None)
            .await
            .expect("root graph children should reload"),
        children_before_input_failure
    );

    let real_enqueue_failure_path = AgentPath::root()
        .join("real_enqueue_failure")
        .expect("real enqueue failure path");
    graph_store.block_upsert.store(true, Ordering::Release);
    let enqueue_failure_task = tokio::spawn({
        let control = control.clone();
        let config = config.clone();
        let real_enqueue_failure_path = real_enqueue_failure_path.clone();
        async move {
            control
                .spawn_agent(
                    config,
                    text_input("exercise the real enqueue failure path"),
                    Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                        parent_thread_id: root_thread_id,
                        depth: 1,
                        agent_path: Some(real_enqueue_failure_path),
                        agent_nickname: None,
                        agent_role: None,
                    })),
                )
                .await
        }
    });
    let blocked_upsert = timeout(Duration::from_secs(2), graph_store.upsert_entered.acquire())
        .await
        .expect("spawn should reach its pending edge upsert")
        .expect("upsert semaphore should remain open");
    blocked_upsert.forget();
    let enqueue_failure_thread_id = graph_store
        .last_upsert_child_thread_id
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .expect("pending edge upsert should expose the child id");
    control
        .upgrade()
        .expect("thread manager state")
        .remove_thread(&enqueue_failure_thread_id)
        .await
        .expect("test should remove the provisional runtime before enqueue");
    graph_store.block_upsert.store(false, Ordering::Release);
    graph_store.release_upsert.add_permits(1);
    let enqueue_error = timeout(Duration::from_secs(5), enqueue_failure_task)
        .await
        .expect("spawn should finish after the enqueue failure")
        .expect("spawn task should join")
        .expect_err("the production enqueue path should observe the missing runtime");
    assert!(matches!(
        enqueue_error,
        CodexErr::ThreadNotFound(thread_id) if thread_id == enqueue_failure_thread_id
    ));
    assert_eq!(manager.list_thread_ids().await, vec![root_thread_id]);
    assert_eq!(
        control.state.agent_id_for_path(&real_enqueue_failure_path),
        None
    );
    assert_eq!(
        graph_store
            .list_thread_spawn_children(root_thread_id, /*status_filter*/ None)
            .await
            .expect("real enqueue rollback should leave no edge"),
        children_before_input_failure
    );

    let promotion_failure_path = AgentPath::root()
        .join("promotion_failure")
        .expect("promotion failure path");
    graph_store
        .fail_open_upserts_remaining
        .store(1, Ordering::Release);
    let promotion_error = control
        .spawn_agent(
            config.clone(),
            text_input("must stay queued until graph activation"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: root_thread_id,
                depth: 1,
                agent_path: Some(promotion_failure_path.clone()),
                agent_nickname: None,
                agent_role: None,
            })),
        )
        .await
        .expect_err("failed pending-to-open promotion should abort the deferred runtime");
    assert!(matches!(
        promotion_error,
        CodexErr::Fatal(message) if message == "failed to persist thread-spawn lifecycle state"
    ));
    let promotion_failure_id = graph_store
        .last_upsert_child_thread_id
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .expect("promotion failure agent id should be recorded");
    timeout(Duration::from_secs(5), async {
        loop {
            let graph_edge = graph_store
                .get_thread_spawn_parent(promotion_failure_id)
                .await
                .expect("promotion failure graph lookup should succeed");
            if manager.get_thread(promotion_failure_id).await.is_err()
                && control
                    .state
                    .agent_id_for_path(&promotion_failure_path)
                    .is_none()
                && graph_edge.is_none()
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("failed activation promotion should clean up runtime, registry, and graph state");
    let stored_promotion_failure = control
        .upgrade()
        .expect("manager state should remain live")
        .read_stored_thread(ReadThreadParams {
            thread_id: promotion_failure_id,
            include_archived: true,
            include_history: true,
        })
        .await;
    match stored_promotion_failure {
        Ok(stored_thread) => {
            let promotion_failure_history = stored_thread
                .history
                .expect("aborted provisional thread history should load")
                .items;
            assert!(
                promotion_failure_history.iter().all(|item| match item {
                    RolloutItem::EventMsg(EventMsg::UserMessage(_))
                    | RolloutItem::InterAgentCommunication(_) => false,
                    RolloutItem::ResponseItem(ResponseItem::Message { role, .. })
                        if role == "user" =>
                    {
                        false
                    }
                    _ => true,
                }),
                "deferred submission must not be dispatched before durable graph activation: {promotion_failure_history:#?}"
            );
        }
        Err(CodexErr::ThreadNotFound(thread_id)) => {
            assert_eq!(thread_id, promotion_failure_id);
        }
        Err(err) => panic!("failed to inspect aborted provisional thread: {err:?}"),
    }

    let compensated_rollback_path = AgentPath::root()
        .join("compensated_rollback")
        .expect("compensated rollback path");
    control.fail_next_initial_input_for_test();
    graph_store.fail_remove.store(true, Ordering::Release);
    let rollback_error = control
        .spawn_agent(
            config.clone(),
            text_input("fail rollback"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: root_thread_id,
                depth: 1,
                agent_path: Some(compensated_rollback_path.clone()),
                agent_nickname: None,
                agent_role: None,
            })),
        )
        .await
        .expect_err("incomplete rollback should fail closed");
    assert!(matches!(
        rollback_error,
        CodexErr::Fatal(message)
            if message
                == "agent rejected its initial input and lifecycle graph repair is required"
    ));
    let compensated_rollback_id = graph_store
        .last_upsert_child_thread_id
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .expect("compensated rollback agent id should be recorded");
    timeout(Duration::from_secs(5), async {
        loop {
            let graph_edge = graph_store
                .get_thread_spawn_parent(compensated_rollback_id)
                .await
                .expect("compensated rollback graph lookup should succeed");
            if manager.get_thread(compensated_rollback_id).await.is_err()
                && control
                    .state
                    .agent_id_for_path(&compensated_rollback_path)
                    .is_none()
                && control
                    .lifecycle
                    .pending_cleanup(compensated_rollback_id)
                    .is_none()
                && graph_edge.is_none()
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("transient edge removal failure should be retried to completion");

    let repair_required_path = AgentPath::root()
        .join("repair_required")
        .expect("repair-required path");
    control.fail_next_initial_input_for_test();
    graph_store
        .fail_status_updates_remaining
        .store(usize::MAX, Ordering::Release);
    graph_store
        .fail_removes_remaining
        .store(usize::MAX, Ordering::Release);
    let repair_error = control
        .spawn_agent(
            config.clone(),
            text_input("fail rollback and compensation"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: root_thread_id,
                depth: 1,
                agent_path: Some(repair_required_path.clone()),
                agent_nickname: None,
                agent_role: None,
            })),
        )
        .await
        .expect_err("double graph failure should require explicit repair");
    assert!(matches!(
        repair_error,
        CodexErr::Fatal(message)
            if message
                == "agent rejected its initial input and lifecycle graph repair is required"
    ));
    let repair_edge = graph_store
        .last_upsert_child_thread_id
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .expect("repair-required agent id should be recorded");
    timeout(Duration::from_secs(5), async {
        while control.lifecycle.pending_cleanup(repair_edge).is_none() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("failed graph repair should remain owned by the lifecycle quarantine");
    let authorization_error = control
        .ensure_agent_known_or_persisted_descendant(repair_edge)
        .await
        .expect_err("quarantined rejected agent must not authorize");
    assert!(matches!(
        authorization_error,
        CodexErr::Fatal(message) if message == "agent lifecycle cleanup is still pending"
    ));
    assert_eq!(
        graph_store
            .list_thread_spawn_children(
                root_thread_id,
                Some(ThreadSpawnEdgeStatus::PendingActivation),
            )
            .await
            .expect("repair-required pending edge should remain inspectable"),
        vec![repair_edge]
    );
    let mut retained_thread_ids = manager.list_thread_ids().await;
    retained_thread_ids.sort_by_key(ToString::to_string);
    let mut expected_thread_ids = vec![root_thread_id];
    expected_thread_ids.sort_by_key(ToString::to_string);
    assert_eq!(retained_thread_ids, expected_thread_ids);
    assert!(matches!(
        manager.get_thread(repair_edge).await,
        Err(CodexErr::ThreadNotFound(thread_id)) if thread_id == repair_edge
    ));
    assert_eq!(
        control.state.agent_id_for_path(&repair_required_path),
        Some(repair_edge)
    );
    let close_error = control
        .close_agent_transactional(repair_edge)
        .await
        .expect_err("explicit close must not bypass an owned rollback quarantine");
    assert!(matches!(
        close_error,
        CodexErr::Fatal(message) if message == "agent lifecycle cleanup is still pending"
    ));
    graph_store
        .fail_status_updates_remaining
        .store(0, Ordering::Release);
    graph_store
        .fail_removes_remaining
        .store(0, Ordering::Release);
    timeout(Duration::from_secs(5), async {
        loop {
            let graph_edge = graph_store
                .get_thread_spawn_parent(repair_edge)
                .await
                .expect("repair-required graph lookup should succeed");
            if manager.get_thread(repair_edge).await.is_err()
                && control
                    .state
                    .agent_id_for_path(&repair_required_path)
                    .is_none()
                && control.lifecycle.pending_cleanup(repair_edge).is_none()
                && graph_edge.is_none()
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("background quarantine owner should repair state after persistence recovers");

    let transient_repair_path = AgentPath::root()
        .join("transient_repair")
        .expect("transient-repair path");
    control.fail_next_initial_input_for_test();
    graph_store
        .fail_status_updates_remaining
        .store(2, Ordering::Release);
    graph_store
        .fail_removes_remaining
        .store(2, Ordering::Release);
    let transient_error = control
        .spawn_agent(
            config,
            text_input("retry transient rollback repair"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: root_thread_id,
                depth: 1,
                agent_path: Some(transient_repair_path.clone()),
                agent_nickname: None,
                agent_role: None,
            })),
        )
        .await
        .expect_err("transient graph failure should reject the spawn");
    assert!(matches!(
        transient_error,
        CodexErr::Fatal(message)
            if message
                == "agent rejected its initial input and lifecycle graph repair is required"
    ));
    let transient_thread_id = graph_store
        .last_upsert_child_thread_id
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .expect("transient repair should record its child thread id");
    timeout(Duration::from_secs(5), async {
        loop {
            let graph_edge = graph_store
                .get_thread_spawn_parent(transient_thread_id)
                .await
                .expect("transient graph lookup should succeed");
            if manager.get_thread(transient_thread_id).await.is_err()
                && control
                    .state
                    .agent_id_for_path(&transient_repair_path)
                    .is_none()
                && control
                    .lifecycle
                    .pending_cleanup(transient_thread_id)
                    .is_none()
                && graph_edge.is_none()
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("late cleanup should recover from a transient graph quarantine failure");
    drop(home);
}

#[tokio::test]
async fn close_agent_does_not_require_graph_edge_for_non_thread_spawn_agent() {
    let harness = AgentControlHarness::new().await;
    let (root_thread_id, _root_thread) = harness.start_thread().await;
    let child_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("non-thread-spawn worker"),
            Some(SessionSource::SubAgent(SubAgentSource::Other(
                "agent-job".to_string(),
            ))),
        )
        .await
        .expect("non-thread-spawn worker should start");
    assert!(
        harness
            .state_db
            .as_ref()
            .expect("state db")
            .get_thread_spawn_parent(child_thread_id)
            .await
            .expect("graph lookup should succeed")
            .is_none()
    );

    harness
        .control
        .close_agent(child_thread_id)
        .await
        .expect("non-thread-spawn worker should close without a graph edge");

    assert!(harness.manager.get_thread(root_thread_id).await.is_ok());
    assert!(harness.manager.get_thread(child_thread_id).await.is_err());
    assert!(
        harness
            .control
            .get_agent_metadata(child_thread_id)
            .is_none()
    );
}

#[tokio::test]
async fn transactional_close_keeps_live_thread_running_when_persisted_edge_is_missing() {
    let mut harness = AgentControlHarness::new().await;
    harness
        .config
        .features
        .enable(Feature::MultiAgentV2)
        .expect("test config should enable V2");
    let (root_thread_id, root_thread) = harness.start_thread().await;
    root_thread
        .codex
        .session
        .set_multi_agent_version_if_unset(MultiAgentVersion::V2);
    let child_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("child with missing lifecycle edge"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: root_thread_id,
                depth: 1,
                agent_path: Some(
                    AgentPath::root()
                        .join("missing_edge_child")
                        .expect("child path"),
                ),
                agent_nickname: None,
                agent_role: None,
            })),
        )
        .await
        .expect("child should spawn");
    harness
        .state_db
        .as_ref()
        .expect("state db")
        .remove_thread_spawn_edge(child_thread_id)
        .await
        .expect("edge should be removed for corruption test");

    let err = harness
        .control
        .close_agent_transactional(child_thread_id)
        .await
        .expect_err("close must fail before shutting down when lifecycle state cannot persist");

    assert!(matches!(
        err,
        CodexErr::Fatal(message) if message == "failed to persist closed agent lifecycle state"
    ));
    assert!(
        harness
            .control
            .upgrade()
            .expect("manager state")
            .get_thread(child_thread_id)
            .await
            .is_ok(),
        "transactional close failure must retain the raw runtime"
    );
    assert_ne!(
        harness.control.get_status(child_thread_id).await,
        AgentStatus::Shutdown
    );
    harness
        .control
        .shutdown_live_agent(child_thread_id)
        .await
        .expect("test cleanup should stop child");
}

#[tokio::test]
async fn native_v1_close_keeps_graph_persistence_best_effort() {
    let harness = AgentControlHarness::new().await;
    let (root_thread_id, _root_thread) = harness.start_thread().await;
    let child_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("native V1 child with missing lifecycle edge"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: root_thread_id,
                depth: 1,
                agent_path: Some(
                    AgentPath::root()
                        .join("native_v1_missing_edge")
                        .expect("child path"),
                ),
                agent_nickname: None,
                agent_role: None,
            })),
        )
        .await
        .expect("native V1 child should spawn");
    harness
        .state_db
        .as_ref()
        .expect("state db")
        .remove_thread_spawn_edge(child_thread_id)
        .await
        .expect("edge should be removed for compatibility test");

    harness
        .control
        .close_agent(child_thread_id)
        .await
        .expect("native V1 close should continue after graph persistence failure");

    assert!(harness.manager.get_thread(child_thread_id).await.is_err());
    assert_eq!(
        harness.control.get_status(child_thread_id).await,
        AgentStatus::NotFound
    );
}

#[tokio::test]
async fn list_agent_subtree_thread_ids_finds_live_descendants_of_unloaded_root() {
    let (_home, config) = test_config().await;
    let manager = ThreadManager::with_models_provider_home_and_state_for_tests(
        CodexAuth::from_api_key("dummy"),
        config.model_provider.clone(),
        config.codex_home.to_path_buf(),
        std::sync::Arc::new(codex_exec_server::EnvironmentManager::default_for_tests()),
        /*state_db*/ None,
    );
    let control = manager.agent_control();
    let parent_thread_id = manager
        .start_thread(config.clone())
        .await
        .expect("parent should start")
        .thread_id;

    let child_thread_id = control
        .spawn_agent(
            config.clone(),
            text_input("hello child"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: None,
                agent_nickname: None,
                agent_role: Some("explorer".to_string()),
            })),
        )
        .await
        .expect("child spawn should succeed");
    let grandchild_thread_id = control
        .spawn_agent(
            config,
            text_input("hello grandchild"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: child_thread_id,
                depth: 2,
                agent_path: None,
                agent_nickname: None,
                agent_role: Some("worker".to_string()),
            })),
        )
        .await
        .expect("grandchild spawn should succeed");

    manager.remove_thread(&parent_thread_id).await;

    let mut subtree_thread_ids = manager
        .list_agent_subtree_thread_ids(parent_thread_id)
        .await
        .expect("live subtree should load");
    subtree_thread_ids.sort_by_key(ToString::to_string);
    let mut expected_subtree_thread_ids =
        vec![parent_thread_id, child_thread_id, grandchild_thread_id];
    expected_subtree_thread_ids.sort_by_key(ToString::to_string);

    assert_eq!(subtree_thread_ids, expected_subtree_thread_ids);
}

#[tokio::test]
async fn shutdown_agent_tree_closes_live_descendants() {
    let harness = AgentControlHarness::new().await;
    let (parent_thread_id, _parent_thread) = harness.start_thread().await;

    let child_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("hello child"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: None,
                agent_nickname: None,
                agent_role: Some("explorer".to_string()),
            })),
        )
        .await
        .expect("child spawn should succeed");
    let grandchild_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("hello grandchild"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: child_thread_id,
                depth: 2,
                agent_path: None,
                agent_nickname: None,
                agent_role: Some("worker".to_string()),
            })),
        )
        .await
        .expect("grandchild spawn should succeed");

    let child_thread = harness
        .manager
        .get_thread(child_thread_id)
        .await
        .expect("child thread should exist");
    let grandchild_thread = harness
        .manager
        .get_thread(grandchild_thread_id)
        .await
        .expect("grandchild thread should exist");
    persist_thread_for_tree_resume(&child_thread, "child persisted").await;
    persist_thread_for_tree_resume(&grandchild_thread, "grandchild persisted").await;
    wait_for_live_thread_spawn_children(&harness.control, parent_thread_id, &[child_thread_id])
        .await;
    wait_for_live_thread_spawn_children(&harness.control, child_thread_id, &[grandchild_thread_id])
        .await;

    let _ = harness
        .control
        .shutdown_agent_tree(parent_thread_id)
        .await
        .expect("tree shutdown should succeed");

    assert_eq!(
        harness.control.get_status(parent_thread_id).await,
        AgentStatus::NotFound
    );
    assert_eq!(
        harness.control.get_status(child_thread_id).await,
        AgentStatus::NotFound
    );
    assert_eq!(
        harness.control.get_status(grandchild_thread_id).await,
        AgentStatus::NotFound
    );

    let shutdown_ids = harness
        .manager
        .captured_ops()
        .into_iter()
        .filter_map(|(thread_id, op)| matches!(op, Op::Shutdown).then_some(thread_id))
        .collect::<Vec<_>>();
    let mut expected_shutdown_ids = vec![parent_thread_id, child_thread_id, grandchild_thread_id];
    expected_shutdown_ids.sort_by_key(std::string::ToString::to_string);
    let mut shutdown_ids = shutdown_ids;
    shutdown_ids.sort_by_key(std::string::ToString::to_string);
    assert_eq!(shutdown_ids, expected_shutdown_ids);
}

#[tokio::test]
async fn shutdown_agent_tree_closes_descendants_when_started_at_child() {
    let harness = AgentControlHarness::new().await;
    let (parent_thread_id, _parent_thread) = harness.start_thread().await;

    let child_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("hello child"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: None,
                agent_nickname: None,
                agent_role: Some("explorer".to_string()),
            })),
        )
        .await
        .expect("child spawn should succeed");
    let grandchild_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("hello grandchild"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: child_thread_id,
                depth: 2,
                agent_path: None,
                agent_nickname: None,
                agent_role: Some("worker".to_string()),
            })),
        )
        .await
        .expect("grandchild spawn should succeed");

    let child_thread = harness
        .manager
        .get_thread(child_thread_id)
        .await
        .expect("child thread should exist");
    let grandchild_thread = harness
        .manager
        .get_thread(grandchild_thread_id)
        .await
        .expect("grandchild thread should exist");
    persist_thread_for_tree_resume(&child_thread, "child persisted").await;
    persist_thread_for_tree_resume(&grandchild_thread, "grandchild persisted").await;
    wait_for_live_thread_spawn_children(&harness.control, parent_thread_id, &[child_thread_id])
        .await;
    wait_for_live_thread_spawn_children(&harness.control, child_thread_id, &[grandchild_thread_id])
        .await;

    let _ = harness
        .control
        .close_agent(child_thread_id)
        .await
        .expect("child close should succeed");

    let _ = harness
        .control
        .shutdown_agent_tree(parent_thread_id)
        .await
        .expect("tree shutdown should succeed");

    assert_eq!(
        harness.control.get_status(child_thread_id).await,
        AgentStatus::NotFound
    );
    assert_eq!(
        harness.control.get_status(grandchild_thread_id).await,
        AgentStatus::NotFound
    );
    assert_eq!(
        harness.control.get_status(parent_thread_id).await,
        AgentStatus::NotFound
    );

    let shutdown_ids = harness
        .manager
        .captured_ops()
        .into_iter()
        .filter_map(|(thread_id, op)| matches!(op, Op::Shutdown).then_some(thread_id))
        .collect::<Vec<_>>();
    let mut expected_shutdown_ids = vec![parent_thread_id, child_thread_id, grandchild_thread_id];
    expected_shutdown_ids.sort_by_key(std::string::ToString::to_string);
    let mut shutdown_ids = shutdown_ids;
    shutdown_ids.sort_by_key(std::string::ToString::to_string);
    assert_eq!(shutdown_ids, expected_shutdown_ids);
}

#[tokio::test]
async fn resume_agent_from_rollout_does_not_reopen_closed_descendants() {
    let harness = AgentControlHarness::new().await;
    let (parent_thread_id, parent_thread) = harness.start_thread().await;

    let child_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("hello child"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: None,
                agent_nickname: None,
                agent_role: Some("explorer".to_string()),
            })),
        )
        .await
        .expect("child spawn should succeed");
    let grandchild_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("hello grandchild"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: child_thread_id,
                depth: 2,
                agent_path: None,
                agent_nickname: None,
                agent_role: Some("worker".to_string()),
            })),
        )
        .await
        .expect("grandchild spawn should succeed");

    let child_thread = harness
        .manager
        .get_thread(child_thread_id)
        .await
        .expect("child thread should exist");
    let grandchild_thread = harness
        .manager
        .get_thread(grandchild_thread_id)
        .await
        .expect("grandchild thread should exist");
    persist_thread_for_tree_resume(&parent_thread, "parent persisted").await;
    persist_thread_for_tree_resume(&child_thread, "child persisted").await;
    persist_thread_for_tree_resume(&grandchild_thread, "grandchild persisted").await;
    wait_for_live_thread_spawn_children(&harness.control, parent_thread_id, &[child_thread_id])
        .await;
    wait_for_live_thread_spawn_children(&harness.control, child_thread_id, &[grandchild_thread_id])
        .await;

    let _ = harness
        .control
        .close_agent(child_thread_id)
        .await
        .expect("child close should succeed");
    let _ = harness
        .control
        .shutdown_live_agent(parent_thread_id)
        .await
        .expect("parent shutdown should succeed");

    let resumed_parent_thread_id = harness
        .control
        .resume_agent_from_rollout(
            harness.config.clone(),
            parent_thread_id,
            SessionSource::Exec,
        )
        .await
        .expect("single-thread resume should succeed");
    assert_eq!(resumed_parent_thread_id, parent_thread_id);
    assert_ne!(
        harness.control.get_status(parent_thread_id).await,
        AgentStatus::NotFound
    );
    assert_eq!(
        harness.control.get_status(child_thread_id).await,
        AgentStatus::NotFound
    );
    assert_eq!(
        harness.control.get_status(grandchild_thread_id).await,
        AgentStatus::NotFound
    );

    let _ = harness
        .control
        .shutdown_agent_tree(parent_thread_id)
        .await
        .expect("tree shutdown after resume should succeed");
}

#[tokio::test]
async fn resume_closed_child_reopens_open_descendants() {
    let harness = AgentControlHarness::new().await;
    let (parent_thread_id, parent_thread) = harness.start_thread().await;

    let child_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("hello child"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: None,
                agent_nickname: None,
                agent_role: Some("explorer".to_string()),
            })),
        )
        .await
        .expect("child spawn should succeed");
    let grandchild_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("hello grandchild"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: child_thread_id,
                depth: 2,
                agent_path: None,
                agent_nickname: None,
                agent_role: Some("worker".to_string()),
            })),
        )
        .await
        .expect("grandchild spawn should succeed");

    let child_thread = harness
        .manager
        .get_thread(child_thread_id)
        .await
        .expect("child thread should exist");
    let grandchild_thread = harness
        .manager
        .get_thread(grandchild_thread_id)
        .await
        .expect("grandchild thread should exist");
    persist_thread_for_tree_resume(&parent_thread, "parent persisted").await;
    persist_thread_for_tree_resume(&child_thread, "child persisted").await;
    persist_thread_for_tree_resume(&grandchild_thread, "grandchild persisted").await;
    wait_for_live_thread_spawn_children(&harness.control, parent_thread_id, &[child_thread_id])
        .await;
    wait_for_live_thread_spawn_children(&harness.control, child_thread_id, &[grandchild_thread_id])
        .await;

    let _ = harness
        .control
        .close_agent(child_thread_id)
        .await
        .expect("child close should succeed");

    let resumed_child_thread_id = harness
        .control
        .resume_agent_from_rollout(
            harness.config.clone(),
            child_thread_id,
            SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: None,
                agent_nickname: None,
                agent_role: None,
            }),
        )
        .await
        .expect("child resume should succeed");
    assert_eq!(resumed_child_thread_id, child_thread_id);
    assert_ne!(
        harness.control.get_status(child_thread_id).await,
        AgentStatus::NotFound
    );
    assert_ne!(
        harness.control.get_status(grandchild_thread_id).await,
        AgentStatus::NotFound
    );

    let _ = harness
        .control
        .close_agent(child_thread_id)
        .await
        .expect("child close after resume should succeed");
    let _ = harness
        .control
        .shutdown_live_agent(parent_thread_id)
        .await
        .expect("parent shutdown should succeed");
}

#[tokio::test]
async fn resume_agent_from_rollout_reopens_open_descendants_after_manager_shutdown() {
    let harness = AgentControlHarness::new().await;
    let (parent_thread_id, parent_thread) = harness.start_thread().await;

    let child_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("hello child"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: None,
                agent_nickname: None,
                agent_role: Some("explorer".to_string()),
            })),
        )
        .await
        .expect("child spawn should succeed");
    let grandchild_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("hello grandchild"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: child_thread_id,
                depth: 2,
                agent_path: None,
                agent_nickname: None,
                agent_role: Some("worker".to_string()),
            })),
        )
        .await
        .expect("grandchild spawn should succeed");

    let child_thread = harness
        .manager
        .get_thread(child_thread_id)
        .await
        .expect("child thread should exist");
    let grandchild_thread = harness
        .manager
        .get_thread(grandchild_thread_id)
        .await
        .expect("grandchild thread should exist");
    persist_thread_for_tree_resume(&parent_thread, "parent persisted").await;
    persist_thread_for_tree_resume(&child_thread, "child persisted").await;
    persist_thread_for_tree_resume(&grandchild_thread, "grandchild persisted").await;
    wait_for_live_thread_spawn_children(&harness.control, parent_thread_id, &[child_thread_id])
        .await;
    wait_for_live_thread_spawn_children(&harness.control, child_thread_id, &[grandchild_thread_id])
        .await;

    let report = harness
        .manager
        .shutdown_all_threads_bounded(Duration::from_secs(5))
        .await;
    assert_eq!(report.submit_failed, Vec::<ThreadId>::new());
    assert_eq!(report.timed_out, Vec::<ThreadId>::new());

    let resumed_parent_thread_id = harness
        .control
        .resume_agent_from_rollout(
            harness.config.clone(),
            parent_thread_id,
            SessionSource::Exec,
        )
        .await
        .expect("tree resume should succeed");
    assert_eq!(resumed_parent_thread_id, parent_thread_id);
    assert_ne!(
        harness.control.get_status(parent_thread_id).await,
        AgentStatus::NotFound
    );
    assert_ne!(
        harness.control.get_status(child_thread_id).await,
        AgentStatus::NotFound
    );
    assert_ne!(
        harness.control.get_status(grandchild_thread_id).await,
        AgentStatus::NotFound
    );

    let _ = harness
        .control
        .shutdown_agent_tree(parent_thread_id)
        .await
        .expect("tree shutdown after subtree resume should succeed");
}

#[tokio::test]
async fn resume_agent_from_rollout_rejects_persisted_graph_cycles() {
    let harness = AgentControlHarness::new().await;
    let (parent_thread_id, parent_thread) = harness.start_thread().await;
    let child_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("cycle child"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: None,
                agent_nickname: None,
                agent_role: None,
            })),
        )
        .await
        .expect("child should spawn");
    let child_thread = harness
        .manager
        .get_thread(child_thread_id)
        .await
        .expect("child thread should exist");
    persist_thread_for_tree_resume(&parent_thread, "parent persisted").await;
    persist_thread_for_tree_resume(&child_thread, "child persisted").await;
    let report = harness
        .manager
        .shutdown_all_threads_bounded(Duration::from_secs(5))
        .await;
    assert_eq!(report.submit_failed, Vec::<ThreadId>::new());
    assert_eq!(report.timed_out, Vec::<ThreadId>::new());

    harness
        .state_db
        .as_ref()
        .expect("state db")
        .upsert_thread_spawn_edge(
            child_thread_id,
            parent_thread_id,
            codex_state::DirectionalThreadSpawnEdgeStatus::Open,
        )
        .await
        .expect("cycle-closing edge should be persisted");

    let error = timeout(
        Duration::from_secs(5),
        harness.control.resume_agent_from_rollout(
            harness.config.clone(),
            parent_thread_id,
            SessionSource::Exec,
        ),
    )
    .await
    .expect("cyclic resume should not hang")
    .expect_err("cyclic resume must fail closed");
    assert!(matches!(
        error,
        CodexErr::Fatal(message) if message.contains("persisted agent graph contains a cycle")
    ));

    let report = harness
        .manager
        .shutdown_all_threads_bounded(Duration::from_secs(5))
        .await;
    assert_eq!(report.submit_failed, Vec::<ThreadId>::new());
    assert_eq!(report.timed_out, Vec::<ThreadId>::new());
}

#[tokio::test]
async fn resume_agent_from_rollout_uses_edge_data_when_descendant_metadata_source_is_stale() {
    let harness = AgentControlHarness::new().await;
    let (parent_thread_id, parent_thread) = harness.start_thread().await;

    let child_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("hello child"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: None,
                agent_nickname: None,
                agent_role: Some("explorer".to_string()),
            })),
        )
        .await
        .expect("child spawn should succeed");
    let grandchild_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("hello grandchild"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: child_thread_id,
                depth: 2,
                agent_path: None,
                agent_nickname: None,
                agent_role: Some("worker".to_string()),
            })),
        )
        .await
        .expect("grandchild spawn should succeed");

    let child_thread = harness
        .manager
        .get_thread(child_thread_id)
        .await
        .expect("child thread should exist");
    let grandchild_thread = harness
        .manager
        .get_thread(grandchild_thread_id)
        .await
        .expect("grandchild thread should exist");
    persist_thread_for_tree_resume(&parent_thread, "parent persisted").await;
    persist_thread_for_tree_resume(&child_thread, "child persisted").await;
    persist_thread_for_tree_resume(&grandchild_thread, "grandchild persisted").await;
    wait_for_live_thread_spawn_children(&harness.control, parent_thread_id, &[child_thread_id])
        .await;
    wait_for_live_thread_spawn_children(&harness.control, child_thread_id, &[grandchild_thread_id])
        .await;

    let state_db = grandchild_thread
        .state_db()
        .expect("sqlite state db should be available");
    let mut stale_metadata = state_db
        .get_thread(grandchild_thread_id)
        .await
        .expect("grandchild metadata query should succeed")
        .expect("grandchild metadata should exist");
    stale_metadata.source =
        serde_json::to_string(&SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id: ThreadId::new(),
            depth: 99,
            agent_path: None,
            agent_nickname: None,
            agent_role: Some("worker".to_string()),
        }))
        .expect("stale session source should serialize");
    state_db
        .upsert_thread(&stale_metadata)
        .await
        .expect("stale grandchild metadata should persist");

    let report = harness
        .manager
        .shutdown_all_threads_bounded(Duration::from_secs(5))
        .await;
    assert_eq!(report.submit_failed, Vec::<ThreadId>::new());
    assert_eq!(report.timed_out, Vec::<ThreadId>::new());

    let resumed_parent_thread_id = harness
        .control
        .resume_agent_from_rollout(
            harness.config.clone(),
            parent_thread_id,
            SessionSource::Exec,
        )
        .await
        .expect("tree resume should succeed");
    assert_eq!(resumed_parent_thread_id, parent_thread_id);
    assert_ne!(
        harness.control.get_status(parent_thread_id).await,
        AgentStatus::NotFound
    );
    assert_ne!(
        harness.control.get_status(child_thread_id).await,
        AgentStatus::NotFound
    );
    assert_ne!(
        harness.control.get_status(grandchild_thread_id).await,
        AgentStatus::NotFound
    );

    let resumed_grandchild_snapshot = harness
        .manager
        .get_thread(grandchild_thread_id)
        .await
        .expect("resumed grandchild thread should exist")
        .config_snapshot()
        .await;
    let SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
        parent_thread_id: resumed_parent_thread_id,
        depth: resumed_depth,
        ..
    }) = resumed_grandchild_snapshot.session_source
    else {
        panic!("expected thread-spawn sub-agent source");
    };
    assert_eq!(resumed_parent_thread_id, child_thread_id);
    assert_eq!(resumed_depth, 2);

    let _ = harness
        .control
        .shutdown_agent_tree(parent_thread_id)
        .await
        .expect("tree shutdown after subtree resume should succeed");
}

#[tokio::test]
async fn resume_agent_from_rollout_skips_descendants_when_parent_resume_fails() {
    let harness = AgentControlHarness::new().await;
    let (parent_thread_id, parent_thread) = harness.start_thread().await;

    let child_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("hello child"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: None,
                agent_nickname: None,
                agent_role: Some("explorer".to_string()),
            })),
        )
        .await
        .expect("child spawn should succeed");
    let grandchild_thread_id = harness
        .control
        .spawn_agent(
            harness.config.clone(),
            text_input("hello grandchild"),
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id: child_thread_id,
                depth: 2,
                agent_path: None,
                agent_nickname: None,
                agent_role: Some("worker".to_string()),
            })),
        )
        .await
        .expect("grandchild spawn should succeed");

    let child_thread = harness
        .manager
        .get_thread(child_thread_id)
        .await
        .expect("child thread should exist");
    let grandchild_thread = harness
        .manager
        .get_thread(grandchild_thread_id)
        .await
        .expect("grandchild thread should exist");
    persist_thread_for_tree_resume(&parent_thread, "parent persisted").await;
    persist_thread_for_tree_resume(&child_thread, "child persisted").await;
    persist_thread_for_tree_resume(&grandchild_thread, "grandchild persisted").await;
    wait_for_live_thread_spawn_children(&harness.control, parent_thread_id, &[child_thread_id])
        .await;
    wait_for_live_thread_spawn_children(&harness.control, child_thread_id, &[grandchild_thread_id])
        .await;

    let child_rollout_path = child_thread
        .rollout_path()
        .expect("child thread should have rollout path");
    let report = harness
        .manager
        .shutdown_all_threads_bounded(Duration::from_secs(5))
        .await;
    assert_eq!(report.submit_failed, Vec::<ThreadId>::new());
    assert_eq!(report.timed_out, Vec::<ThreadId>::new());
    tokio::fs::remove_file(&child_rollout_path)
        .await
        .expect("child rollout path should be removable");

    let resumed_parent_thread_id = harness
        .control
        .resume_agent_from_rollout(
            harness.config.clone(),
            parent_thread_id,
            SessionSource::Exec,
        )
        .await
        .expect("root resume should succeed");
    assert_eq!(resumed_parent_thread_id, parent_thread_id);
    assert_ne!(
        harness.control.get_status(parent_thread_id).await,
        AgentStatus::NotFound
    );
    assert_eq!(
        harness.control.get_status(child_thread_id).await,
        AgentStatus::NotFound
    );
    assert_eq!(
        harness.control.get_status(grandchild_thread_id).await,
        AgentStatus::NotFound
    );

    let _ = harness
        .control
        .shutdown_agent_tree(parent_thread_id)
        .await
        .expect("tree shutdown after partial subtree resume should succeed");
}
