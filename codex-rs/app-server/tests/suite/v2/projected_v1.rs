use anyhow::Context;
use anyhow::Result;
use app_test_support::TestAppServer;
use app_test_support::to_response;
use app_test_support::write_mock_responses_config_toml;
use app_test_support::write_models_cache;
use codex_app_server_protocol::ItemCompletedNotification;
use codex_app_server_protocol::JSONRPCMessage;
use codex_app_server_protocol::JSONRPCResponse;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::SessionSource;
use codex_app_server_protocol::SortDirection;
use codex_app_server_protocol::Thread;
use codex_app_server_protocol::ThreadHistoryMode;
use codex_app_server_protocol::ThreadItem;
use codex_app_server_protocol::ThreadListParams;
use codex_app_server_protocol::ThreadListResponse;
use codex_app_server_protocol::ThreadReadParams;
use codex_app_server_protocol::ThreadReadResponse;
use codex_app_server_protocol::ThreadResumeParams;
use codex_app_server_protocol::ThreadResumeResponse;
use codex_app_server_protocol::ThreadSourceKind;
use codex_app_server_protocol::ThreadStartParams;
use codex_app_server_protocol::ThreadStartResponse;
use codex_app_server_protocol::ThreadTurnsListParams;
use codex_app_server_protocol::ThreadTurnsListResponse;
use codex_app_server_protocol::TurnCompletedNotification;
use codex_app_server_protocol::TurnItemsView;
use codex_app_server_protocol::TurnStartParams;
use codex_app_server_protocol::TurnStartResponse;
use codex_app_server_protocol::UserInput;
use codex_features::Feature;
use codex_protocol::protocol::MultiAgentVersion;
use codex_protocol::protocol::SessionSource as CoreSessionSource;
use codex_protocol::protocol::SubAgentSource;
use codex_protocol::protocol::ThreadHistoryMode as CoreThreadHistoryMode;
use codex_rollout::read_session_meta_line;
use core_test_support::responses;
use serde_json::Value;
use serde_json::json;
use std::collections::BTreeMap;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::timeout;

#[cfg(windows)]
const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(25);
#[cfg(not(windows))]
const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(10);
const MULTI_AGENT_V1_NAMESPACE: &str = "multi_agent_v1";
const MULTI_AGENT_V2_NAMESPACE: &str = "collaboration";

#[tokio::test]
async fn projected_v1_paginated_subagent_is_parented_persisted_and_accepts_direct_input()
-> Result<()> {
    const CHILD_PROMPT: &str = "projected V1 child: do work";
    const COLD_DIRECT_PROMPT: &str = "cold direct input for projected V1 child";
    const DIRECT_PROMPT: &str = "live direct input for projected V1 child";
    const PARENT_PROMPT: &str = "spawn a projected V1 child and continue";
    const SPAWN_CALL_ID: &str = "spawn-call-projected-v1";

    let server = responses::start_mock_server().await;
    let spawn_args = serde_json::to_string(&json!({ "message": CHILD_PROMPT }))?;
    let _parent_turn = responses::mount_sse_once_match(
        &server,
        |request: &wiremock::Request| body_contains(request, PARENT_PROMPT),
        responses::sse(vec![
            responses::ev_response_created("resp-parent-v1-1"),
            responses::ev_function_call_with_namespace(
                SPAWN_CALL_ID,
                MULTI_AGENT_V1_NAMESPACE,
                "spawn_agent",
                &spawn_args,
            ),
            responses::ev_completed("resp-parent-v1-1"),
        ]),
    )
    .await;
    let child_response = responses::sse_response(responses::sse(vec![
        responses::ev_response_created("resp-child-v1-1"),
        responses::ev_assistant_message("msg-child-v1-1", "child done"),
        responses::ev_completed("resp-child-v1-1"),
    ]))
    .set_delay(Duration::from_millis(500));
    let _child_turn = responses::mount_response_once_match(
        &server,
        |request: &wiremock::Request| {
            body_contains(request, CHILD_PROMPT) && !body_contains(request, SPAWN_CALL_ID)
        },
        child_response,
    )
    .await;
    let parent_follow_up = responses::mount_sse_once_match(
        &server,
        |request: &wiremock::Request| body_contains(request, SPAWN_CALL_ID),
        responses::sse(vec![
            responses::ev_response_created("resp-parent-v1-2"),
            responses::ev_assistant_message("msg-parent-v1-2", "parent done"),
            responses::ev_completed("resp-parent-v1-2"),
        ]),
    )
    .await;
    let live_direct_turn = responses::mount_sse_once_match(
        &server,
        |request: &wiremock::Request| body_contains(request, DIRECT_PROMPT),
        responses::sse(vec![
            responses::ev_response_created("resp-child-v1-live-direct"),
            responses::ev_assistant_message("msg-child-v1-live-direct", "live direct done"),
            responses::ev_completed("resp-child-v1-live-direct"),
        ]),
    )
    .await;

    let codex_home = TempDir::new()?;
    write_mock_responses_config_toml(
        codex_home.path(),
        &server.uri(),
        &BTreeMap::from([(Feature::Collab, true), (Feature::MultiAgentV2, true)]),
        100_000,
        /*requires_openai_auth*/ None,
        "mock_provider",
        "compact",
    )?;
    let config_path = codex_home.path().join("config.toml");
    let config_toml = std::fs::read_to_string(&config_path)?;
    std::fs::write(
        &config_path,
        format!("{config_toml}\n[agents]\nmax_depth = 2\n"),
    )?;
    write_models_cache(codex_home.path())?;

    let mut app = TestAppServer::builder()
        .with_codex_home(codex_home.path())
        .build()
        .await?;
    timeout(DEFAULT_READ_TIMEOUT, app.initialize()).await??;

    let start_request = app
        .send_thread_start_request_with_auto_env(ThreadStartParams {
            model: Some("gpt-5.4".to_string()),
            history_mode: Some(ThreadHistoryMode::Paginated),
            ..Default::default()
        })
        .await?;
    let start_response = timeout(
        DEFAULT_READ_TIMEOUT,
        app.read_stream_until_response_message(RequestId::Integer(start_request)),
    )
    .await??;
    let ThreadStartResponse { thread, .. } = to_response::<ThreadStartResponse>(start_response)?;
    let parent_thread_id = thread.id;

    let parent_turn_request = app
        .send_turn_start_request(TurnStartParams {
            thread_id: parent_thread_id.clone(),
            input: vec![text_input(PARENT_PROMPT)],
            ..Default::default()
        })
        .await?;
    let parent_turn_response = timeout(
        DEFAULT_READ_TIMEOUT,
        app.read_stream_until_response_message(RequestId::Integer(parent_turn_request)),
    )
    .await??;
    let TurnStartResponse { turn: parent_turn } =
        to_response::<TurnStartResponse>(parent_turn_response)?;
    let child_thread_id =
        wait_for_projected_spawn(&mut app, &parent_thread_id, &parent_turn.id, SPAWN_CALL_ID)
            .await?;
    let _ = parent_follow_up.single_request();

    let live_child = read_thread(&mut app, &child_thread_id).await?;
    assert_projected_child_identity(&live_child, &parent_thread_id, Some(true));

    let live_turn_request = app
        .send_turn_start_request(TurnStartParams {
            thread_id: child_thread_id.clone(),
            input: vec![text_input(DIRECT_PROMPT)],
            ..Default::default()
        })
        .await?;
    let live_turn_response = timeout(
        DEFAULT_READ_TIMEOUT,
        app.read_stream_until_response_message(RequestId::Integer(live_turn_request)),
    )
    .await??;
    let TurnStartResponse { turn: live_turn } =
        to_response::<TurnStartResponse>(live_turn_response)?;
    wait_for_turn_completed(&mut app, &child_thread_id, &live_turn.id).await?;
    let _ = live_direct_turn.single_request();

    drop(app);

    let mut restarted_app = TestAppServer::builder()
        .with_codex_home(codex_home.path())
        .build()
        .await?;
    timeout(DEFAULT_READ_TIMEOUT, restarted_app.initialize()).await??;

    let resume_request = restarted_app
        .send_thread_resume_request(ThreadResumeParams {
            thread_id: child_thread_id.clone(),
            exclude_turns: true,
            ..Default::default()
        })
        .await?;
    let resume_response: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        restarted_app.read_stream_until_response_message(RequestId::Integer(resume_request)),
    )
    .await??;
    let ThreadResumeResponse {
        thread: resumed_child,
        ..
    } = to_response::<ThreadResumeResponse>(resume_response)?;
    assert_projected_child_identity(&resumed_child, &parent_thread_id, Some(true));

    let rollout_path = resumed_child
        .path
        .as_deref()
        .context("resumed child should expose its rollout path")?;
    let session_meta = read_session_meta_line(rollout_path).await?.meta;
    assert_eq!(
        session_meta.multi_agent_version,
        Some(MultiAgentVersion::V1)
    );
    assert_eq!(session_meta.history_mode, CoreThreadHistoryMode::Paginated);
    assert_eq!(
        session_meta.parent_thread_id.map(|id| id.to_string()),
        Some(parent_thread_id.clone())
    );
    assert_eq!(session_meta.agent_path, None);
    match session_meta.source {
        CoreSessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id: source_parent_thread_id,
            depth,
            agent_path,
            ..
        }) => {
            assert_eq!(source_parent_thread_id.to_string(), parent_thread_id);
            assert_eq!(depth, 1);
            assert_eq!(agent_path, None);
        }
        other => panic!("expected pathless persisted V1 child, got {other:?}"),
    }

    let expected_prompt_cache_key = resumed_child.session_id.clone();
    let matcher_prompt_cache_key = expected_prompt_cache_key.clone();
    let cold_direct_turn = responses::mount_sse_once_match(
        &server,
        move |request: &wiremock::Request| {
            let body: Value = serde_json::from_slice(&request.body).unwrap_or_default();
            body["prompt_cache_key"].as_str() == Some(matcher_prompt_cache_key.as_str())
                && body_contains(request, COLD_DIRECT_PROMPT)
        },
        responses::sse(vec![
            responses::ev_response_created("resp-child-v1-cold-direct"),
            responses::ev_assistant_message("msg-child-v1-cold-direct", "cold direct done"),
            responses::ev_completed("resp-child-v1-cold-direct"),
        ]),
    )
    .await;
    let cold_turn_request = restarted_app
        .send_turn_start_request(TurnStartParams {
            thread_id: child_thread_id.clone(),
            input: vec![text_input(COLD_DIRECT_PROMPT)],
            ..Default::default()
        })
        .await?;
    let cold_turn_response = timeout(
        DEFAULT_READ_TIMEOUT,
        restarted_app.read_stream_until_response_message(RequestId::Integer(cold_turn_request)),
    )
    .await??;
    let TurnStartResponse { turn: cold_turn } =
        to_response::<TurnStartResponse>(cold_turn_response)?;
    wait_for_turn_completed(&mut restarted_app, &child_thread_id, &cold_turn.id).await?;

    let cold_request = cold_direct_turn.single_request();
    let cold_body = cold_request.body_json();
    assert_eq!(
        cold_body["prompt_cache_key"].as_str(),
        Some(expected_prompt_cache_key.as_str())
    );
    let tools = cold_body["tools"]
        .as_array()
        .context("cold-resumed child request should contain tools")?;
    assert!(
        tools
            .iter()
            .all(|tool| tool["name"].as_str() != Some(MULTI_AGENT_V2_NAMESPACE))
    );
    assert!(
        tools
            .iter()
            .any(|tool| tool["type"].as_str() == Some("tool_search"))
    );

    let list_request = restarted_app
        .send_thread_list_request(ThreadListParams {
            cursor: None,
            limit: Some(10),
            sort_key: None,
            sort_direction: None,
            model_providers: None,
            source_kinds: Some(vec![ThreadSourceKind::SubAgentThreadSpawn]),
            archived: None,
            cwd: None,
            use_state_db_only: false,
            search_term: None,
            parent_thread_id: Some(parent_thread_id.clone()),
            ancestor_thread_id: None,
        })
        .await?;
    let list_response = timeout(
        DEFAULT_READ_TIMEOUT,
        restarted_app.read_stream_until_response_message(RequestId::Integer(list_request)),
    )
    .await??;
    let ThreadListResponse { data, .. } = to_response::<ThreadListResponse>(list_response)?;
    assert_eq!(data.len(), 1);
    assert_eq!(data[0].id, child_thread_id);
    assert_projected_child_identity(&data[0], &parent_thread_id, None);

    let read_child = read_thread(&mut restarted_app, &child_thread_id).await?;
    assert_projected_child_identity(&read_child, &parent_thread_id, Some(true));

    let turns_request = restarted_app
        .send_thread_turns_list_request(ThreadTurnsListParams {
            thread_id: child_thread_id.clone(),
            cursor: None,
            limit: Some(50),
            sort_direction: Some(SortDirection::Asc),
            items_view: Some(TurnItemsView::Full),
        })
        .await?;
    let turns_response = timeout(
        DEFAULT_READ_TIMEOUT,
        restarted_app.read_stream_until_response_message(RequestId::Integer(turns_request)),
    )
    .await??;
    let ThreadTurnsListResponse { data, .. } =
        to_response::<ThreadTurnsListResponse>(turns_response)?;
    let user_messages = data
        .iter()
        .flat_map(|turn| &turn.items)
        .filter_map(|item| match item {
            ThreadItem::UserMessage { content, .. } => Some(content),
            _ => None,
        })
        .flatten()
        .filter_map(|input| match input {
            UserInput::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    for expected in [CHILD_PROMPT, DIRECT_PROMPT, COLD_DIRECT_PROMPT] {
        assert!(
            user_messages.contains(&expected),
            "missing user message {expected}"
        );
    }
    for expected in ["child done", "live direct done", "cold direct done"] {
        assert!(
            data.iter().flat_map(|turn| &turn.items).any(
                |item| matches!(item, ThreadItem::AgentMessage { text, .. } if text == expected)
            )
        );
    }

    Ok(())
}

fn body_contains(request: &wiremock::Request, text: &str) -> bool {
    String::from_utf8(request.body.clone())
        .ok()
        .is_some_and(|body| body.contains(text))
}

fn text_input(text: &str) -> UserInput {
    UserInput::Text {
        text: text.to_string(),
        text_elements: Vec::new(),
    }
}

async fn wait_for_projected_spawn(
    app: &mut TestAppServer,
    parent_thread_id: &str,
    parent_turn_id: &str,
    spawn_call_id: &str,
) -> Result<String> {
    timeout(DEFAULT_READ_TIMEOUT, async {
        let mut child_thread_id = None;
        let mut completed_thread_ids = Vec::new();
        let mut parent_turn_completed = false;
        loop {
            let JSONRPCMessage::Notification(notification) = app.read_next_message().await? else {
                continue;
            };
            match notification.method.as_str() {
                "item/completed" => {
                    let completed: ItemCompletedNotification = serde_json::from_value(
                        notification.params.context("item/completed params")?,
                    )?;
                    match completed.item {
                        ThreadItem::CollabAgentToolCall {
                            id,
                            receiver_thread_ids,
                            ..
                        } if id == spawn_call_id => {
                            let spawned_thread_id = receiver_thread_ids
                                .into_iter()
                                .next()
                                .context("spawn completion omitted child thread id")?;
                            if child_thread_id.replace(spawned_thread_id).is_some() {
                                anyhow::bail!("projected V1 spawn emitted duplicate completion");
                            }
                        }
                        ThreadItem::SubAgentActivity { id, .. } if id == spawn_call_id => {
                            anyhow::bail!("projected V1 spawn emitted V2 SubAgentActivity");
                        }
                        _ => {}
                    }
                }
                "turn/completed" => {
                    let completed: TurnCompletedNotification = serde_json::from_value(
                        notification.params.context("turn/completed params")?,
                    )?;
                    if completed.thread_id == parent_thread_id
                        && completed.turn.id == parent_turn_id
                    {
                        parent_turn_completed = true;
                    }
                    completed_thread_ids.push(completed.thread_id);
                }
                _ => {}
            }
            if parent_turn_completed
                && child_thread_id
                    .as_ref()
                    .is_some_and(|child_id| completed_thread_ids.contains(child_id))
            {
                return child_thread_id.context("spawn completion should provide child id");
            }
        }
    })
    .await?
}

async fn wait_for_turn_completed(
    app: &mut TestAppServer,
    thread_id: &str,
    turn_id: &str,
) -> Result<()> {
    timeout(DEFAULT_READ_TIMEOUT, async {
        loop {
            let notification = app
                .read_stream_until_notification_message("turn/completed")
                .await?;
            let completed: TurnCompletedNotification =
                serde_json::from_value(notification.params.context("turn/completed params")?)?;
            if completed.thread_id == thread_id && completed.turn.id == turn_id {
                return Ok::<(), anyhow::Error>(());
            }
        }
    })
    .await?
}

async fn read_thread(app: &mut TestAppServer, thread_id: &str) -> Result<Thread> {
    let request = app
        .send_thread_read_request(ThreadReadParams {
            thread_id: thread_id.to_string(),
            include_turns: false,
        })
        .await?;
    let response = timeout(
        DEFAULT_READ_TIMEOUT,
        app.read_stream_until_response_message(RequestId::Integer(request)),
    )
    .await??;
    let ThreadReadResponse { thread } = to_response::<ThreadReadResponse>(response)?;
    Ok(thread)
}

fn assert_projected_child_identity(
    thread: &Thread,
    parent_thread_id: &str,
    expected_direct_input: Option<bool>,
) {
    assert_eq!(thread.history_mode, ThreadHistoryMode::Paginated);
    assert_eq!(thread.can_accept_direct_input, expected_direct_input);
    assert_eq!(thread.parent_thread_id.as_deref(), Some(parent_thread_id));
    match &thread.source {
        SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id: source_parent_thread_id,
            depth,
            agent_path,
            ..
        }) => {
            assert_eq!(source_parent_thread_id.to_string(), parent_thread_id);
            assert_eq!(*depth, 1);
            assert_eq!(agent_path, &None);
        }
        other => panic!("expected pathless projected V1 child, got {other:?}"),
    }
}
