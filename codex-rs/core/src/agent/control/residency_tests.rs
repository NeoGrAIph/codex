use crate::ThreadManager;
use crate::agent::AgentControl;
use crate::agent::AgentStatus;
use crate::agent::control::AuthorizedAgentTarget;
use crate::agent::control::PersistedAgentLineage;
use crate::codex_thread::CodexThread;
use crate::config::Config;
use crate::config::test_config;
use crate::init_state_db;
use crate::thread_manager::ThreadManagerState;
use codex_features::Feature;
use codex_login::CodexAuth;
use codex_protocol::AgentPath;
use codex_protocol::ThreadId;
use codex_protocol::error::CodexErr;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::MultiAgentVersion;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use codex_protocol::protocol::ThreadSource;
use codex_protocol::protocol::TurnAbortReason;
use codex_protocol::protocol::TurnAbortedEvent;
use codex_protocol::protocol::TurnCompleteEvent;
use codex_protocol::user_input::UserInput;
use pretty_assertions::assert_eq;
use std::sync::Arc;

#[tokio::test]
async fn explicit_v2_resume_registers_residency_and_is_evictable() {
    let mut config = test_config().await;
    let _ = config.features.enable(Feature::MultiAgentV2);
    config.multi_agent_v2.max_concurrent_threads_per_session = 2;
    let temp_home = tempfile::tempdir().expect("create temp home");
    config.codex_home = temp_home.path().to_path_buf().try_into().unwrap();
    config.cwd = temp_home.path().to_path_buf().try_into().unwrap();
    let state_db = init_state_db(&config).await;
    let manager = ThreadManager::with_models_provider_home_and_state_for_tests(
        CodexAuth::from_api_key("dummy"),
        config.model_provider.clone(),
        config.codex_home.to_path_buf(),
        Arc::new(codex_exec_server::EnvironmentManager::default_for_tests()),
        state_db,
    );
    let root = manager
        .start_thread(config.clone())
        .await
        .expect("start root thread");
    root.thread
        .codex
        .session
        .set_multi_agent_version_if_unset(MultiAgentVersion::V2);
    let control = manager.agent_control();
    let child_path = AgentPath::root()
        .join("resumed_resident")
        .expect("resumed resident path");
    let child_source = SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
        parent_thread_id: root.thread_id,
        depth: 1,
        agent_path: Some(child_path),
        agent_nickname: None,
        agent_role: None,
    });
    let child_thread_id = control
        .spawn_agent(
            config.clone(),
            vec![UserInput::Text {
                text: "persist resident child".to_string(),
                text_elements: Vec::new(),
            }],
            Some(child_source.clone()),
        )
        .await
        .expect("resident child should spawn");
    let child_thread = manager
        .get_thread(child_thread_id)
        .await
        .expect("resident child should exist");
    child_thread.ensure_rollout_materialized().await;
    child_thread
        .flush_rollout()
        .await
        .expect("resident child rollout should flush");
    control
        .shutdown_live_agent(child_thread_id)
        .await
        .expect("resident child should unload before explicit resume");

    control
        .resume_agent_from_rollout(config.clone(), child_thread_id, child_source.clone())
        .await
        .expect("explicit V2 resume should succeed");
    let resumed_thread = manager
        .get_thread(child_thread_id)
        .await
        .expect("resumed child should be loaded");
    mark_thread_completed(resumed_thread.as_ref()).await;

    let state = control.upgrade().expect("thread manager should be live");
    let pending_slot = control
        .reserve_v2_residency_slot(&state, &config, /*protected_thread_id*/ None)
        .await
        .expect("next residency reservation should evict the resumed child");
    match manager.get_thread(child_thread_id).await {
        Err(CodexErr::ThreadNotFound(thread_id)) => assert_eq!(thread_id, child_thread_id),
        Err(err) => panic!("expected evicted resumed child to be missing, got {err:?}"),
        Ok(_) => panic!("expected explicit-resume child to participate in residency eviction"),
    }
    assert_eq!(
        control.get_status(child_thread_id).await,
        AgentStatus::Completed(Some("done".to_string()))
    );
    assert_eq!(
        control
            .subscribe_status(child_thread_id)
            .await
            .expect("evicted terminal status should remain waitable")
            .borrow()
            .clone(),
        AgentStatus::Completed(Some("done".to_string()))
    );
    assert!(
        control
            .list_agents(&SessionSource::default(), /*path_prefix*/ None)
            .await
            .expect("agent listing should include evicted terminal residents")
            .into_iter()
            .any(|agent| {
                agent.agent_name == "/root/resumed_resident"
                    && agent.agent_status == AgentStatus::Completed(Some("done".to_string()))
            })
    );
    assert_eq!(
        control
            .ensure_agent_known_or_persisted_descendant(child_thread_id)
            .await
            .expect("unloaded resident should authorize through its persisted edge"),
        AuthorizedAgentTarget::Persisted(PersistedAgentLineage {
            parent_thread_id: root.thread_id,
            depth: 1,
        })
    );
    drop(pending_slot);

    control
        .resume_agent_from_persisted_graph_edge(config, child_thread_id, child_source)
        .await
        .expect("evicted registered child should reactivate without reserving its path again");
    assert!(
        manager.get_thread(child_thread_id).await.is_ok(),
        "reactivated child should be loaded"
    );
    assert_eq!(
        control
            .get_agent_metadata(child_thread_id)
            .expect("reactivated child should retain its registry metadata")
            .last_status,
        None
    );
}

#[tokio::test]
async fn residency_slot_reservation_unloads_oldest_idle_v2_agent() {
    let mut config = test_config().await;
    let _ = config.features.enable(Feature::MultiAgentV2);
    config.multi_agent_v2.max_concurrent_threads_per_session = 2;
    let temp_home = tempfile::tempdir().expect("create temp home");
    config.codex_home = temp_home.path().to_path_buf().try_into().unwrap();
    config.cwd = temp_home.path().to_path_buf().try_into().unwrap();
    let manager = ThreadManager::with_models_provider_and_home_for_tests(
        CodexAuth::from_api_key("dummy"),
        config.model_provider.clone(),
        config.codex_home.to_path_buf(),
        Arc::new(codex_exec_server::EnvironmentManager::default_for_tests()),
    );
    let root = manager
        .start_thread(config.clone())
        .await
        .expect("start root thread");
    let control = manager.agent_control();
    let state = control.upgrade().expect("thread manager should be live");

    let first_slot = control
        .reserve_v2_residency_slot(&state, &config, /*protected_thread_id*/ None)
        .await
        .expect("first resident slot");
    let first =
        spawn_v2_subagent(&control, &state, config.clone(), root.thread_id, "worker-1").await;
    first_slot.commit(first.thread_id);
    mark_thread_completed(first.thread.as_ref()).await;

    let second_slot = control
        .reserve_v2_residency_slot(&state, &config, /*protected_thread_id*/ None)
        .await
        .expect("second resident slot should evict the first idle agent");
    match manager.get_thread(first.thread_id).await {
        Err(CodexErr::ThreadNotFound(thread_id)) => assert_eq!(thread_id, first.thread_id),
        Err(err) => panic!("expected evicted thread to be missing, got {err:?}"),
        Ok(_) => panic!("expected evicted thread to be missing"),
    }
    let second = spawn_v2_subagent(&control, &state, config, root.thread_id, "worker-2").await;
    second_slot.commit(second.thread_id);

    assert!(manager.get_thread(root.thread_id).await.is_ok());
    assert!(manager.get_thread(second.thread_id).await.is_ok());
}

#[tokio::test]
async fn interrupted_v2_agent_is_not_residency_evictable() {
    let mut config = test_config().await;
    let _ = config.features.enable(Feature::MultiAgentV2);
    config.multi_agent_v2.max_concurrent_threads_per_session = 2;
    let temp_home = tempfile::tempdir().expect("create temp home");
    config.codex_home = temp_home.path().to_path_buf().try_into().unwrap();
    config.cwd = temp_home.path().to_path_buf().try_into().unwrap();
    let manager = ThreadManager::with_models_provider_and_home_for_tests(
        CodexAuth::from_api_key("dummy"),
        config.model_provider.clone(),
        config.codex_home.to_path_buf(),
        Arc::new(codex_exec_server::EnvironmentManager::default_for_tests()),
    );
    let root = manager
        .start_thread(config.clone())
        .await
        .expect("start root thread");
    let control = manager.agent_control();
    let state = control.upgrade().expect("thread manager should be live");

    let first_slot = control
        .reserve_v2_residency_slot(&state, &config, /*protected_thread_id*/ None)
        .await
        .expect("first resident slot");
    let first =
        spawn_v2_subagent(&control, &state, config.clone(), root.thread_id, "worker-1").await;
    first_slot.commit(first.thread_id);
    mark_thread_interrupted(first.thread.as_ref()).await;

    let Err(error) = control
        .reserve_v2_residency_slot(&state, &config, /*protected_thread_id*/ None)
        .await
    else {
        panic!("interrupted resident must not be evicted as terminal");
    };
    assert!(matches!(error, CodexErr::AgentLimitReached { .. }));

    assert!(manager.get_thread(root.thread_id).await.is_ok());
    assert!(manager.get_thread(first.thread_id).await.is_ok());
}

async fn spawn_v2_subagent(
    control: &AgentControl,
    state: &Arc<ThreadManagerState>,
    config: Config,
    parent_thread_id: ThreadId,
    label: &str,
) -> crate::thread_manager::NewThread {
    state
        .spawn_new_thread_with_source(
            config,
            control.clone(),
            SessionSource::SubAgent(SubAgentSource::Other(label.to_string())),
            Some(parent_thread_id),
            /*forked_from_thread_id*/ None,
            Some(ThreadSource::Subagent),
            /*metrics_service_name*/ None,
            /*inherited_environments*/ None,
            /*inherited_exec_policy*/ None,
            /*environments*/ None,
            /*submission_loop_activation*/ None,
        )
        .await
        .expect("spawn v2 subagent")
}

async fn mark_thread_completed(thread: &CodexThread) {
    let turn = thread.codex.session.new_default_turn().await;
    thread
        .codex
        .session
        .send_event(
            turn.as_ref(),
            EventMsg::TurnComplete(TurnCompleteEvent {
                turn_id: turn.sub_id.clone(),
                last_agent_message: Some("done".to_string()),
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            }),
        )
        .await;
    clear_active_turn(thread).await;
}

async fn mark_thread_interrupted(thread: &CodexThread) {
    let turn = thread.codex.session.new_default_turn().await;
    thread
        .codex
        .session
        .send_event(
            turn.as_ref(),
            EventMsg::TurnAborted(TurnAbortedEvent {
                turn_id: Some(turn.sub_id.clone()),
                reason: TurnAbortReason::Interrupted,
                completed_at: None,
                duration_ms: None,
            }),
        )
        .await;
    clear_active_turn(thread).await;
}

async fn clear_active_turn(thread: &CodexThread) {
    // The fixture has no task runner to clear the turn after the terminal event.
    *thread.codex.session.active_turn.lock().await = None;
}
