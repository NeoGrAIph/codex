use anyhow::Result;
use app_test_support::McpProcess;
use app_test_support::create_fake_rollout_with_source;
use app_test_support::create_fake_rollout_with_text_elements;
use app_test_support::create_mock_responses_server_repeating_assistant;
use app_test_support::rollout_path;
use app_test_support::set_rollout_thread_spawn_agent_persona;
use app_test_support::to_response;
use codex_app_server_protocol::JSONRPCError;
use codex_app_server_protocol::JSONRPCResponse;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::SessionSource;
use codex_app_server_protocol::ThreadItem;
use codex_app_server_protocol::ThreadListParams;
use codex_app_server_protocol::ThreadListResponse;
use codex_app_server_protocol::ThreadNameUpdatedNotification;
use codex_app_server_protocol::ThreadNoteUpdatedNotification;
use codex_app_server_protocol::ThreadReadParams;
use codex_app_server_protocol::ThreadReadResponse;
use codex_app_server_protocol::ThreadResumeParams;
use codex_app_server_protocol::ThreadResumeResponse;
use codex_app_server_protocol::ThreadSetNameParams;
use codex_app_server_protocol::ThreadSetNameResponse;
use codex_app_server_protocol::ThreadSetNoteParams;
use codex_app_server_protocol::ThreadSetNoteResponse;
use codex_app_server_protocol::ThreadSourceKind;
use codex_app_server_protocol::ThreadStartParams;
use codex_app_server_protocol::ThreadStartResponse;
use codex_app_server_protocol::ThreadStatus;
use codex_app_server_protocol::TurnStartParams;
use codex_app_server_protocol::TurnStartResponse;
use codex_app_server_protocol::TurnStatus;
use codex_app_server_protocol::UserInput;
use codex_protocol::protocol::SessionSource as CoreSessionSource;
use codex_protocol::protocol::SubAgentSource;
use codex_protocol::user_input::ByteRange;
use codex_protocol::user_input::TextElement;
use core_test_support::responses;
use pretty_assertions::assert_eq;
use serde_json::Value;
use std::path::Path;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::time::sleep;
use tokio::time::timeout;
use uuid::Uuid;

const DEFAULT_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

#[tokio::test]
async fn thread_read_returns_summary_without_turns() -> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path(), &server.uri())?;

    let preview = "Saved user message";
    let text_elements = [TextElement::new(
        ByteRange { start: 0, end: 5 },
        Some("<note>".into()),
    )];
    let conversation_id = create_fake_rollout_with_text_elements(
        codex_home.path(),
        "2025-01-05T12-00-00",
        "2025-01-05T12:00:00Z",
        preview,
        text_elements
            .iter()
            .map(|elem| serde_json::to_value(elem).expect("serialize text element"))
            .collect(),
        Some("mock_provider"),
        None,
    )?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let read_id = mcp
        .send_thread_read_request(ThreadReadParams {
            thread_id: conversation_id.clone(),
            include_turns: false,
        })
        .await?;
    let read_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_id)),
    )
    .await??;
    let ThreadReadResponse { thread } = to_response::<ThreadReadResponse>(read_resp)?;

    assert_eq!(thread.id, conversation_id);
    assert_eq!(thread.preview, preview);
    assert_eq!(thread.model_provider, "mock_provider");
    assert!(!thread.ephemeral, "stored rollouts should not be ephemeral");
    assert!(thread.path.as_ref().expect("thread path").is_absolute());
    assert_eq!(thread.cwd, PathBuf::from("/"));
    assert_eq!(thread.cli_version, "0.0.0");
    assert_eq!(thread.source, SessionSource::Cli);
    assert_eq!(thread.git_info, None);
    assert_eq!(thread.turns.len(), 0);
    assert_eq!(thread.status, ThreadStatus::NotLoaded);

    Ok(())
}

#[tokio::test]
async fn thread_read_can_include_turns() -> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path(), &server.uri())?;

    let preview = "Saved user message";
    let text_elements = vec![TextElement::new(
        ByteRange { start: 0, end: 5 },
        Some("<note>".into()),
    )];
    let conversation_id = create_fake_rollout_with_text_elements(
        codex_home.path(),
        "2025-01-05T12-00-00",
        "2025-01-05T12:00:00Z",
        preview,
        text_elements
            .iter()
            .map(|elem| serde_json::to_value(elem).expect("serialize text element"))
            .collect(),
        Some("mock_provider"),
        None,
    )?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let read_id = mcp
        .send_thread_read_request(ThreadReadParams {
            thread_id: conversation_id.clone(),
            include_turns: true,
        })
        .await?;
    let read_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_id)),
    )
    .await??;
    let ThreadReadResponse { thread } = to_response::<ThreadReadResponse>(read_resp)?;

    assert_eq!(thread.turns.len(), 1);
    let turn = &thread.turns[0];
    assert_eq!(turn.status, TurnStatus::Completed);
    assert_eq!(turn.items.len(), 1, "expected user message item");
    match &turn.items[0] {
        ThreadItem::UserMessage { content, .. } => {
            assert_eq!(
                content,
                &vec![UserInput::Text {
                    text: preview.to_string(),
                    text_elements: text_elements.clone().into_iter().map(Into::into).collect(),
                }]
            );
        }
        other => panic!("expected user message item, got {other:?}"),
    }
    assert_eq!(thread.status, ThreadStatus::NotLoaded);

    Ok(())
}

#[tokio::test]
async fn thread_read_loaded_thread_returns_precomputed_path_before_materialization() -> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path(), &server.uri())?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let start_id = mcp
        .send_thread_start_request(ThreadStartParams {
            model: Some("mock-model".to_string()),
            ..Default::default()
        })
        .await?;
    let start_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(start_id)),
    )
    .await??;
    let ThreadStartResponse { thread, .. } = to_response::<ThreadStartResponse>(start_resp)?;
    let thread_path = thread.path.clone().expect("thread path");
    assert!(
        !thread_path.exists(),
        "fresh thread rollout should not be materialized yet"
    );

    let read_id = mcp
        .send_thread_read_request(ThreadReadParams {
            thread_id: thread.id.clone(),
            include_turns: false,
        })
        .await?;
    let read_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_id)),
    )
    .await??;
    let ThreadReadResponse { thread: read } = to_response::<ThreadReadResponse>(read_resp)?;

    assert_eq!(read.id, thread.id);
    assert_eq!(read.path, Some(thread_path));
    assert!(read.preview.is_empty());
    assert_eq!(read.turns.len(), 0);
    assert_eq!(read.status, ThreadStatus::Idle);

    Ok(())
}

#[tokio::test]
async fn thread_name_set_is_reflected_in_read_list_and_resume() -> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path(), &server.uri())?;

    let preview = "Saved user message";
    let conversation_id = create_fake_rollout_with_text_elements(
        codex_home.path(),
        "2025-01-05T12-00-00",
        "2025-01-05T12:00:00Z",
        preview,
        vec![],
        Some("mock_provider"),
        None,
    )?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    // Set a user-facing thread title.
    let new_name = "My renamed thread";
    let set_id = mcp
        .send_thread_set_name_request(ThreadSetNameParams {
            thread_id: conversation_id.clone(),
            name: new_name.to_string(),
        })
        .await?;
    let set_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(set_id)),
    )
    .await??;
    let _: ThreadSetNameResponse = to_response::<ThreadSetNameResponse>(set_resp)?;
    let notification = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("thread/name/updated"),
    )
    .await??;
    let notification: ThreadNameUpdatedNotification =
        serde_json::from_value(notification.params.expect("thread/name/updated params"))?;
    assert_eq!(notification.thread_id, conversation_id);
    assert_eq!(notification.thread_name.as_deref(), Some(new_name));

    // Read should now surface `thread.name`, and the wire payload must include `name`.
    let read_id = mcp
        .send_thread_read_request(ThreadReadParams {
            thread_id: conversation_id.clone(),
            include_turns: false,
        })
        .await?;
    let read_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_id)),
    )
    .await??;
    let read_result = read_resp.result.clone();
    let ThreadReadResponse { thread } = to_response::<ThreadReadResponse>(read_resp)?;
    assert_eq!(thread.id, conversation_id);
    assert_eq!(thread.name.as_deref(), Some(new_name));
    let thread_json = read_result
        .get("thread")
        .and_then(Value::as_object)
        .expect("thread/read result.thread must be an object");
    assert_eq!(
        thread_json.get("name").and_then(Value::as_str),
        Some(new_name),
        "thread/read must serialize `thread.name` on the wire"
    );
    assert_eq!(
        thread_json.get("ephemeral").and_then(Value::as_bool),
        Some(false),
        "thread/read must serialize `thread.ephemeral` on the wire"
    );

    // List should also surface the name.
    let list_id = mcp
        .send_thread_list_request(ThreadListParams {
            cursor: None,
            limit: Some(50),
            sort_key: None,
            model_providers: Some(vec!["mock_provider".to_string()]),
            source_kinds: Some(vec![ThreadSourceKind::SubAgentThreadSpawn]),
            archived: None,
            cwd: None,
            search_term: None,
        })
        .await?;
    let list_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(list_id)),
    )
    .await??;
    let list_result = list_resp.result.clone();
    let ThreadListResponse { data, .. } = to_response::<ThreadListResponse>(list_resp)?;
    let listed = data
        .iter()
        .find(|t| t.id == conversation_id)
        .expect("thread/list should include the created thread");
    assert_eq!(listed.name.as_deref(), Some(new_name));
    let listed_json = list_result
        .get("data")
        .and_then(Value::as_array)
        .expect("thread/list result.data must be an array")
        .iter()
        .find(|t| t.get("id").and_then(Value::as_str) == Some(&conversation_id))
        .and_then(Value::as_object)
        .expect("thread/list should include the created thread as an object");
    assert_eq!(
        listed_json.get("name").and_then(Value::as_str),
        Some(new_name),
        "thread/list must serialize `thread.name` on the wire"
    );
    assert_eq!(
        listed_json.get("ephemeral").and_then(Value::as_bool),
        Some(false),
        "thread/list must serialize `thread.ephemeral` on the wire"
    );

    // Resume should also surface the name.
    let resume_id = mcp
        .send_thread_resume_request(ThreadResumeParams {
            thread_id: conversation_id.clone(),
            ..Default::default()
        })
        .await?;
    let resume_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(resume_id)),
    )
    .await??;
    let resume_result = resume_resp.result.clone();
    let ThreadResumeResponse {
        thread: resumed, ..
    } = to_response::<ThreadResumeResponse>(resume_resp)?;
    assert_eq!(resumed.id, conversation_id);
    assert_eq!(resumed.name.as_deref(), Some(new_name));
    let resumed_json = resume_result
        .get("thread")
        .and_then(Value::as_object)
        .expect("thread/resume result.thread must be an object");
    assert_eq!(
        resumed_json.get("name").and_then(Value::as_str),
        Some(new_name),
        "thread/resume must serialize `thread.name` on the wire"
    );
    assert_eq!(
        resumed_json.get("ephemeral").and_then(Value::as_bool),
        Some(false),
        "thread/resume must serialize `thread.ephemeral` on the wire"
    );

    Ok(())
}

#[tokio::test]
async fn thread_note_set_is_reflected_in_read_list_and_resume() -> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path(), &server.uri())?;

    let preview = "Saved user message";
    let parent_thread_id = codex_protocol::ThreadId::from_string(&Uuid::new_v4().to_string())?;
    let conversation_id = create_fake_rollout_with_source(
        codex_home.path(),
        "2025-01-05T12-00-00",
        "2025-01-05T12:00:00Z",
        preview,
        Some("mock_provider"),
        None,
        CoreSessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id,
            depth: 1,
            agent_nickname: None,
            agent_role: None,
            agent_persona: None,
            allow_list: None,
            deny_list: None,
            thread_note: None,
        }),
    )?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let new_note = "Keep the shell transcript concise";
    let set_id = mcp
        .send_thread_set_note_request(ThreadSetNoteParams {
            thread_id: conversation_id.clone(),
            note: Some(new_note.to_string()),
        })
        .await?;
    let set_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(set_id)),
    )
    .await??;
    let _: ThreadSetNoteResponse = to_response::<ThreadSetNoteResponse>(set_resp)?;
    let notification = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("thread/note/updated"),
    )
    .await??;
    let notification: ThreadNoteUpdatedNotification =
        serde_json::from_value(notification.params.expect("thread/note/updated params"))?;
    assert_eq!(notification.thread_id, conversation_id);
    assert_eq!(notification.thread_note.as_deref(), Some(new_note));

    let read_id = mcp
        .send_thread_read_request(ThreadReadParams {
            thread_id: conversation_id.clone(),
            include_turns: false,
        })
        .await?;
    let read_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_id)),
    )
    .await??;
    let read_result = read_resp.result.clone();
    let ThreadReadResponse { thread } = to_response::<ThreadReadResponse>(read_resp)?;
    assert_eq!(thread.id, conversation_id);
    assert_eq!(thread.thread_note.as_deref(), Some(new_note));
    assert_eq!(thread.source.get_thread_note().as_deref(), Some(new_note));
    let thread_json = read_result
        .get("thread")
        .and_then(Value::as_object)
        .expect("thread/read result.thread must be an object");
    assert_eq!(
        thread_json.get("threadNote").and_then(Value::as_str),
        Some(new_note),
        "thread/read must serialize `thread.threadNote` on the wire"
    );
    assert_eq!(
        thread_json
            .get("source")
            .and_then(Value::as_object)
            .and_then(|source| source.get("subAgent"))
            .and_then(Value::as_object)
            .and_then(|source| source.get("thread_spawn"))
            .and_then(Value::as_object)
            .and_then(|source| source.get("thread_note"))
            .and_then(Value::as_str),
        Some(new_note),
        "thread/read must keep nested thread_spawn.thread_note in sync"
    );

    let resume_id = mcp
        .send_thread_resume_request(ThreadResumeParams {
            thread_id: conversation_id.clone(),
            ..Default::default()
        })
        .await?;
    let resume_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(resume_id)),
    )
    .await??;
    let resume_result = resume_resp.result.clone();
    let ThreadResumeResponse { thread, .. } = to_response::<ThreadResumeResponse>(resume_resp)?;
    assert_eq!(thread.id, conversation_id);
    assert_eq!(thread.thread_note.as_deref(), Some(new_note));
    assert_eq!(thread.source.get_thread_note().as_deref(), Some(new_note));
    let resumed_json = resume_result
        .get("thread")
        .and_then(Value::as_object)
        .expect("thread/resume result.thread must be an object");
    assert_eq!(
        resumed_json
            .get("source")
            .and_then(Value::as_object)
            .and_then(|source| source.get("subAgent"))
            .and_then(Value::as_object)
            .and_then(|source| source.get("thread_spawn"))
            .and_then(Value::as_object)
            .and_then(|source| source.get("thread_note"))
            .and_then(Value::as_str),
        Some(new_note),
        "thread/resume must keep nested thread_spawn.thread_note in sync"
    );

    let clear_id = mcp
        .send_thread_set_note_request(ThreadSetNoteParams {
            thread_id: conversation_id.clone(),
            note: None,
        })
        .await?;
    let clear_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(clear_id)),
    )
    .await??;
    let _: ThreadSetNoteResponse = to_response::<ThreadSetNoteResponse>(clear_resp)?;
    let clear_notification = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("thread/note/updated"),
    )
    .await??;
    let clear_notification: ThreadNoteUpdatedNotification = serde_json::from_value(
        clear_notification
            .params
            .expect("thread/note/updated params for clear"),
    )?;
    assert_eq!(clear_notification.thread_id, conversation_id);
    assert_eq!(clear_notification.thread_note, None);

    let cleared_read_id = mcp
        .send_thread_read_request(ThreadReadParams {
            thread_id: conversation_id.clone(),
            include_turns: false,
        })
        .await?;
    let cleared_read_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(cleared_read_id)),
    )
    .await??;
    let cleared_read_result = cleared_read_resp.result.clone();
    let ThreadReadResponse { thread } = to_response::<ThreadReadResponse>(cleared_read_resp)?;
    assert_eq!(thread.thread_note, None);
    assert_eq!(thread.source.get_thread_note(), None);
    let cleared_thread_json = cleared_read_result
        .get("thread")
        .and_then(Value::as_object)
        .expect("thread/read result.thread after clear must be an object");
    assert_eq!(cleared_thread_json.get("threadNote"), Some(&Value::Null));
    assert_eq!(
        cleared_thread_json
            .get("source")
            .and_then(Value::as_object)
            .and_then(|source| source.get("subAgent"))
            .and_then(Value::as_object)
            .and_then(|source| source.get("thread_spawn"))
            .and_then(Value::as_object)
            .and_then(|source| source.get("thread_note")),
        Some(&Value::Null),
        "thread/read after clear must serialize nested thread_spawn.thread_note as null"
    );

    let cleared_list_id = mcp
        .send_thread_list_request(ThreadListParams {
            cursor: None,
            limit: Some(10),
            sort_key: None,
            model_providers: Some(vec!["mock_provider".to_string()]),
            source_kinds: Some(vec![ThreadSourceKind::SubAgentThreadSpawn]),
            archived: None,
            cwd: None,
            search_term: None,
        })
        .await?;
    let cleared_list_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(cleared_list_id)),
    )
    .await??;
    let cleared_list_result = cleared_list_resp.result.clone();
    let ThreadListResponse { data, .. } = to_response::<ThreadListResponse>(cleared_list_resp)?;
    let listed = data
        .into_iter()
        .find(|thread| thread.id == conversation_id)
        .expect("thread/list should include the cleared thread");
    assert_eq!(listed.thread_note, None);
    assert_eq!(listed.source.get_thread_note(), None);
    let listed_json = cleared_list_result
        .get("data")
        .and_then(Value::as_array)
        .expect("thread/list result.data must be an array")
        .iter()
        .find(|thread| thread.get("id").and_then(Value::as_str) == Some(&conversation_id))
        .and_then(Value::as_object)
        .expect("thread/list should include the cleared thread as an object");
    assert_eq!(listed_json.get("threadNote"), Some(&Value::Null));
    assert_eq!(
        listed_json
            .get("source")
            .and_then(Value::as_object)
            .and_then(|source| source.get("subAgent"))
            .and_then(Value::as_object)
            .and_then(|source| source.get("thread_spawn"))
            .and_then(Value::as_object)
            .and_then(|source| source.get("thread_note")),
        Some(&Value::Null),
        "thread/list after clear must serialize nested thread_spawn.thread_note as null"
    );

    let running_resume_id = mcp
        .send_thread_resume_request(ThreadResumeParams {
            thread_id: conversation_id.clone(),
            ..Default::default()
        })
        .await?;
    let running_resume_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(running_resume_id)),
    )
    .await??;
    let running_resume_result = running_resume_resp.result.clone();
    let ThreadResumeResponse {
        thread: running_resumed,
        ..
    } = to_response::<ThreadResumeResponse>(running_resume_resp)?;
    assert_eq!(running_resumed.thread_note, None);
    assert_eq!(running_resumed.source.get_thread_note(), None);
    let running_resumed_json = running_resume_result
        .get("thread")
        .and_then(Value::as_object)
        .expect("thread/resume result.thread after clear must be an object");
    assert_eq!(running_resumed_json.get("threadNote"), Some(&Value::Null));
    assert_eq!(
        running_resumed_json
            .get("source")
            .and_then(Value::as_object)
            .and_then(|source| source.get("subAgent"))
            .and_then(Value::as_object)
            .and_then(|source| source.get("thread_spawn"))
            .and_then(Value::as_object)
            .and_then(|source| source.get("thread_note")),
        Some(&Value::Null),
        "thread/resume after clear must serialize nested thread_spawn.thread_note as null"
    );

    Ok(())
}

#[tokio::test]
async fn thread_note_clear_on_unloaded_thread_keeps_nested_source_in_sync() -> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path(), &server.uri())?;

    let parent_thread_id = codex_protocol::ThreadId::from_string(&Uuid::new_v4().to_string())?;
    let conversation_id = create_fake_rollout_with_source(
        codex_home.path(),
        "2025-01-06T12-00-00",
        "2025-01-06T12:00:00Z",
        "Saved user message",
        Some("mock_provider"),
        None,
        CoreSessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id,
            depth: 1,
            agent_nickname: None,
            agent_role: None,
            agent_persona: None,
            allow_list: None,
            deny_list: None,
            thread_note: None,
        }),
    )?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    for note in [Some("remember the audit"), None] {
        let request_id = mcp
            .send_thread_set_note_request(ThreadSetNoteParams {
                thread_id: conversation_id.clone(),
                note: note.map(str::to_string),
            })
            .await?;
        let response: JSONRPCResponse = timeout(
            DEFAULT_READ_TIMEOUT,
            mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
        )
        .await??;
        let _: ThreadSetNoteResponse = to_response::<ThreadSetNoteResponse>(response)?;
        let notification = timeout(
            DEFAULT_READ_TIMEOUT,
            mcp.read_stream_until_notification_message("thread/note/updated"),
        )
        .await??;
        let notification: ThreadNoteUpdatedNotification =
            serde_json::from_value(notification.params.expect("thread/note/updated params"))?;
        assert_eq!(notification.thread_id, conversation_id);
        assert_eq!(notification.thread_note.as_deref(), note);
    }

    let read_id = mcp
        .send_thread_read_request(ThreadReadParams {
            thread_id: conversation_id.clone(),
            include_turns: false,
        })
        .await?;
    let read_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_id)),
    )
    .await??;
    let read_result = read_resp.result.clone();
    let ThreadReadResponse { thread } = to_response::<ThreadReadResponse>(read_resp)?;
    assert_eq!(thread.thread_note, None);
    assert_eq!(thread.source.get_thread_note(), None);
    let thread_json = read_result
        .get("thread")
        .and_then(Value::as_object)
        .expect("thread/read result.thread after unloaded clear must be an object");
    assert_eq!(thread_json.get("threadNote"), Some(&Value::Null));
    assert_eq!(
        thread_json
            .get("source")
            .and_then(Value::as_object)
            .and_then(|source| source.get("subAgent"))
            .and_then(Value::as_object)
            .and_then(|source| source.get("thread_spawn"))
            .and_then(Value::as_object)
            .and_then(|source| source.get("thread_note")),
        Some(&Value::Null),
        "thread/read after unloaded clear must serialize nested thread_spawn.thread_note as null"
    );

    let list_id = mcp
        .send_thread_list_request(ThreadListParams {
            cursor: None,
            limit: Some(10),
            sort_key: None,
            model_providers: Some(vec!["mock_provider".to_string()]),
            source_kinds: Some(vec![ThreadSourceKind::SubAgentThreadSpawn]),
            archived: None,
            cwd: None,
            search_term: None,
        })
        .await?;
    let list_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(list_id)),
    )
    .await??;
    let list_result = list_resp.result.clone();
    let ThreadListResponse { data, .. } = to_response::<ThreadListResponse>(list_resp)?;
    let listed = data
        .into_iter()
        .find(|thread| thread.id == conversation_id)
        .expect("thread/list should include the unloaded thread");
    assert_eq!(listed.thread_note, None);
    assert_eq!(listed.source.get_thread_note(), None);
    let listed_json = list_result
        .get("data")
        .and_then(Value::as_array)
        .expect("thread/list result.data after unloaded clear must be an array")
        .iter()
        .find(|thread| thread.get("id").and_then(Value::as_str) == Some(&conversation_id))
        .and_then(Value::as_object)
        .expect("thread/list should include the unloaded thread as an object");
    assert_eq!(listed_json.get("threadNote"), Some(&Value::Null));
    assert_eq!(
        listed_json
            .get("source")
            .and_then(Value::as_object)
            .and_then(|source| source.get("subAgent"))
            .and_then(Value::as_object)
            .and_then(|source| source.get("thread_spawn"))
            .and_then(Value::as_object)
            .and_then(|source| source.get("thread_note")),
        Some(&Value::Null),
        "thread/list after unloaded clear must serialize nested thread_spawn.thread_note as null"
    );

    Ok(())
}

#[tokio::test]
async fn thread_read_list_and_resume_preserve_agent_persona() -> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path(), &server.uri())?;

    let parent_thread_id = codex_protocol::ThreadId::from_string(&Uuid::new_v4().to_string())?;
    let conversation_id = create_fake_rollout_with_source(
        codex_home.path(),
        "2025-01-06T12-00-00",
        "2025-01-06T12:00:00Z",
        "Saved user message",
        Some("mock_provider"),
        None,
        CoreSessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id,
            depth: 1,
            agent_nickname: None,
            agent_role: None,
            agent_persona: None,
            allow_list: None,
            deny_list: None,
            thread_note: None,
        }),
    )?;
    let rollout_path = rollout_path(
        codex_home.path(),
        "2025-01-06T12-00-00",
        conversation_id.as_str(),
    );
    set_rollout_thread_spawn_agent_persona(rollout_path.as_path(), Some("researcher"))?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let read_id = mcp
        .send_thread_read_request(ThreadReadParams {
            thread_id: conversation_id.clone(),
            include_turns: false,
        })
        .await?;
    let read_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_id)),
    )
    .await??;
    let read_result = read_resp.result.clone();
    let ThreadReadResponse { thread } = to_response::<ThreadReadResponse>(read_resp)?;
    assert_eq!(thread.agent_persona.as_deref(), Some("researcher"));
    let read_thread_json = read_result
        .get("thread")
        .and_then(Value::as_object)
        .expect("thread/read result.thread must be an object");
    assert_eq!(
        read_thread_json.get("agentPersona").and_then(Value::as_str),
        Some("researcher")
    );
    assert_eq!(
        read_thread_json
            .get("source")
            .and_then(Value::as_object)
            .and_then(|source| source.get("subAgent"))
            .and_then(Value::as_object)
            .and_then(|source| source.get("thread_spawn"))
            .and_then(Value::as_object)
            .and_then(|source| source.get("agent_persona"))
            .and_then(Value::as_str),
        Some("researcher")
    );

    let resume_id = mcp
        .send_thread_resume_request(ThreadResumeParams {
            thread_id: conversation_id.clone(),
            ..Default::default()
        })
        .await?;
    let resume_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(resume_id)),
    )
    .await??;
    let resume_result = resume_resp.result.clone();
    let ThreadResumeResponse {
        thread: resumed, ..
    } = to_response::<ThreadResumeResponse>(resume_resp)?;
    assert_eq!(resumed.agent_persona.as_deref(), Some("researcher"));
    let resumed_json = resume_result
        .get("thread")
        .and_then(Value::as_object)
        .expect("thread/resume result.thread must be an object");
    assert_eq!(
        resumed_json.get("agentPersona").and_then(Value::as_str),
        Some("researcher")
    );
    assert_eq!(
        resumed_json
            .get("source")
            .and_then(Value::as_object)
            .and_then(|source| source.get("subAgent"))
            .and_then(Value::as_object)
            .and_then(|source| source.get("thread_spawn"))
            .and_then(Value::as_object)
            .and_then(|source| source.get("agent_persona"))
            .and_then(Value::as_str),
        Some("researcher")
    );

    let list_deadline = tokio::time::Instant::now() + DEFAULT_READ_TIMEOUT;
    let (list_result, data) = loop {
        let list_id = mcp
            .send_thread_list_request(ThreadListParams {
                cursor: None,
                limit: Some(50),
                sort_key: None,
                model_providers: None,
                source_kinds: Some(vec![ThreadSourceKind::SubAgentThreadSpawn]),
                archived: None,
                cwd: None,
                search_term: None,
            })
            .await?;
        let list_resp: JSONRPCResponse = timeout(
            DEFAULT_READ_TIMEOUT,
            mcp.read_stream_until_response_message(RequestId::Integer(list_id)),
        )
        .await??;
        let list_result = list_resp.result.clone();
        let ThreadListResponse { data, .. } = to_response::<ThreadListResponse>(list_resp)?;
        if data.iter().any(|thread| thread.id == conversation_id) {
            break (list_result, data);
        }
        if tokio::time::Instant::now() >= list_deadline {
            anyhow::bail!("thread/list did not include {conversation_id} before timeout");
        }
        sleep(std::time::Duration::from_millis(50)).await;
    };
    let listed = data
        .iter()
        .find(|thread| thread.id == conversation_id)
        .expect("thread/list should include the thread");
    assert_eq!(listed.agent_persona.as_deref(), Some("researcher"));
    let listed_json = list_result
        .get("data")
        .and_then(Value::as_array)
        .expect("thread/list result.data must be an array")
        .iter()
        .find(|thread| thread.get("id").and_then(Value::as_str) == Some(&conversation_id))
        .and_then(Value::as_object)
        .expect("thread/list should include the thread as an object");
    assert_eq!(
        listed_json.get("agentPersona").and_then(Value::as_str),
        Some("researcher")
    );
    assert_eq!(
        listed_json
            .get("source")
            .and_then(Value::as_object)
            .and_then(|source| source.get("subAgent"))
            .and_then(Value::as_object)
            .and_then(|source| source.get("thread_spawn"))
            .and_then(Value::as_object)
            .and_then(|source| source.get("agent_persona"))
            .and_then(Value::as_str),
        Some("researcher")
    );

    Ok(())
}

#[tokio::test]
async fn thread_read_include_turns_rejects_unmaterialized_loaded_thread() -> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path(), &server.uri())?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let start_id = mcp
        .send_thread_start_request(ThreadStartParams {
            model: Some("mock-model".to_string()),
            ..Default::default()
        })
        .await?;
    let start_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(start_id)),
    )
    .await??;
    let ThreadStartResponse { thread, .. } = to_response::<ThreadStartResponse>(start_resp)?;
    let thread_path = thread.path.clone().expect("thread path");
    assert!(
        !thread_path.exists(),
        "fresh thread rollout should not be materialized yet"
    );

    let read_id = mcp
        .send_thread_read_request(ThreadReadParams {
            thread_id: thread.id.clone(),
            include_turns: true,
        })
        .await?;
    let read_err: JSONRPCError = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(read_id)),
    )
    .await??;

    assert!(
        read_err
            .error
            .message
            .contains("includeTurns is unavailable before first user message"),
        "unexpected error: {}",
        read_err.error.message
    );

    Ok(())
}

#[tokio::test]
async fn thread_read_reports_system_error_idle_flag_after_failed_turn() -> Result<()> {
    let server = responses::start_mock_server().await;
    let _response_mock = responses::mount_sse_once(
        &server,
        responses::sse_failed("resp-1", "server_error", "simulated failure"),
    )
    .await;
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path(), &server.uri())?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let start_id = mcp
        .send_thread_start_request(ThreadStartParams {
            model: Some("mock-model".to_string()),
            ..Default::default()
        })
        .await?;
    let start_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(start_id)),
    )
    .await??;
    let ThreadStartResponse { thread, .. } = to_response::<ThreadStartResponse>(start_resp)?;

    let turn_start_id = mcp
        .send_turn_start_request(TurnStartParams {
            thread_id: thread.id.clone(),
            input: vec![UserInput::Text {
                text: "fail this turn".to_string(),
                text_elements: Vec::new(),
            }],
            ..Default::default()
        })
        .await?;
    let turn_start_response: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(turn_start_id)),
    )
    .await??;
    let _: TurnStartResponse = to_response::<TurnStartResponse>(turn_start_response)?;
    timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("error"),
    )
    .await??;

    let read_id = mcp
        .send_thread_read_request(ThreadReadParams {
            thread_id: thread.id,
            include_turns: false,
        })
        .await?;
    let read_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_id)),
    )
    .await??;
    let ThreadReadResponse { thread } = to_response::<ThreadReadResponse>(read_resp)?;

    assert_eq!(thread.status, ThreadStatus::SystemError,);

    Ok(())
}

// Helper to create a config.toml pointing at the mock model server.
fn create_config_toml(codex_home: &Path, server_uri: &str) -> std::io::Result<()> {
    let config_toml = codex_home.join("config.toml");
    std::fs::write(
        config_toml,
        format!(
            r#"
model = "mock-model"
approval_policy = "never"
sandbox_mode = "read-only"

model_provider = "mock_provider"

[model_providers.mock_provider]
name = "Mock provider for test"
base_url = "{server_uri}/v1"
wire_api = "responses"
request_max_retries = 0
stream_max_retries = 0
"#
        ),
    )
}
