use std::collections::HashMap;
use std::time::Duration;

use anyhow::Result;
use app_test_support::TestAppServer;
use app_test_support::to_response;
use codex_app_server_protocol::AgentRoleActionPolicySetParams;
use codex_app_server_protocol::AgentRoleActionPolicySetResponse;
use codex_app_server_protocol::AgentRoleToolSelectionCatalogExposure;
use codex_app_server_protocol::AgentRoleToolSelectionCatalogReadParams;
use codex_app_server_protocol::AgentRoleToolSelectionCatalogReadResponse;
use codex_app_server_protocol::AgentRoleToolSelectionSetParams;
use codex_app_server_protocol::AgentRoleToolSelectionSetResponse;
use codex_app_server_protocol::JSONRPCError;
use codex_app_server_protocol::JSONRPCResponse;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ThreadStartParams;
use codex_app_server_protocol::ThreadStartResponse;
use codex_protocol::protocol::SubAgentActionPolicyAction;
use pretty_assertions::assert_eq;
use serde_json::json;
use tempfile::TempDir;
use tokio::time::timeout;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

#[tokio::test]
async fn read_tool_selection_catalog_uses_loaded_thread_policy() -> Result<()> {
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path())?;
    let mut app = TestAppServer::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, app.initialize()).await??;

    let start_request_id = app
        .send_thread_start_request(ThreadStartParams {
            config: Some(HashMap::from([(
                "tool_selection.allowed_tools".to_string(),
                json!(["update_plan", "missing_tool"]),
            )])),
            ..Default::default()
        })
        .await?;
    let start_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        app.read_stream_until_response_message(RequestId::Integer(start_request_id)),
    )
    .await??;
    let ThreadStartResponse { thread, .. } = to_response::<ThreadStartResponse>(start_response)?;

    let read_request_id = app
        .send_agent_role_tool_selection_catalog_read_request(
            AgentRoleToolSelectionCatalogReadParams {
                thread_id: thread.id,
            },
        )
        .await?;
    let read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        app.read_stream_until_response_message(RequestId::Integer(read_request_id)),
    )
    .await??;
    let AgentRoleToolSelectionCatalogReadResponse {
        data,
        unmatched_allowed_tools,
    } = to_response::<AgentRoleToolSelectionCatalogReadResponse>(read_response)?;

    let update_plan = data
        .iter()
        .find(|entry| entry.name == "update_plan")
        .expect("catalog should include native update_plan tool");
    assert_eq!(update_plan.selected, true);
    assert_eq!(
        update_plan.exposure,
        AgentRoleToolSelectionCatalogExposure::Direct
    );
    assert_eq!(unmatched_allowed_tools, vec!["missing_tool".to_string()]);
    Ok(())
}

#[tokio::test]
async fn agent_role_tool_selection_set_updates_user_role_file() -> Result<()> {
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path())?;
    let agents_dir = codex_home.path().join("agents");
    std::fs::create_dir_all(&agents_dir)?;
    let role_path = agents_dir.join("reviewer.toml");
    std::fs::write(
        &role_path,
        r#"name = "reviewer"
description = "Review code changes before handoff."
developer_instructions = "Review the diff and report concrete risks."

[tool_selection]
allowed_tools = ["update_plan"]
"#,
    )?;
    let mut app = TestAppServer::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, app.initialize()).await??;

    let request_id = app
        .send_agent_role_tool_selection_set_request(AgentRoleToolSelectionSetParams {
            role_name: "reviewer".to_string(),
            allowed_tools: Some(vec!["tool_search".to_string(), "update_plan".to_string()]),
            denied_tools: Some(vec!["apply_patch".to_string()]),
        })
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        app.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let set_response = to_response::<AgentRoleToolSelectionSetResponse>(response)?;

    assert_eq!(set_response.role_name, "reviewer");
    assert_eq!(
        set_response.allowed_tools,
        Some(vec!["tool_search".to_string(), "update_plan".to_string()])
    );
    assert_eq!(
        set_response.denied_tools,
        Some(vec!["apply_patch".to_string()])
    );
    assert_eq!(set_response.file_path.as_path(), role_path.as_path());
    let contents = std::fs::read_to_string(&role_path)?;
    assert!(contents.contains("[tool_selection]\nallowed_tools = ["));
    assert!(contents.contains("\"tool_search\""));
    assert!(contents.contains("\"update_plan\""));
    assert!(contents.contains("denied_tools = ["));
    assert!(contents.contains("\"apply_patch\""));

    let request_id = app
        .send_agent_role_tool_selection_set_request(AgentRoleToolSelectionSetParams {
            role_name: "reviewer".to_string(),
            allowed_tools: Some(Vec::new()),
            denied_tools: Some(vec!["tool_search".to_string()]),
        })
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        app.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let set_response = to_response::<AgentRoleToolSelectionSetResponse>(response)?;

    assert_eq!(set_response.allowed_tools, Some(Vec::new()));
    assert_eq!(
        set_response.denied_tools,
        Some(vec!["tool_search".to_string()])
    );
    let contents = std::fs::read_to_string(&role_path)?;
    assert!(contents.contains("[tool_selection]\nallowed_tools = []"));
    assert!(contents.contains("denied_tools = ["));
    assert!(contents.contains("\"tool_search\""));

    let request_id = app
        .send_agent_role_tool_selection_set_request(AgentRoleToolSelectionSetParams {
            role_name: "reviewer".to_string(),
            allowed_tools: None,
            denied_tools: None,
        })
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        app.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let set_response = to_response::<AgentRoleToolSelectionSetResponse>(response)?;

    assert_eq!(set_response.allowed_tools, None);
    assert_eq!(set_response.denied_tools, None);
    let contents = std::fs::read_to_string(role_path)?;
    assert!(!contents.contains("[tool_selection]"));
    assert!(!contents.contains("allowed_tools"));
    assert!(!contents.contains("denied_tools"));
    Ok(())
}

#[tokio::test]
async fn agent_role_tool_selection_set_rejects_external_config_file() -> Result<()> {
    let codex_home = TempDir::new()?;
    let external_dir = TempDir::new()?;
    let external_role = external_dir.path().join("reviewer.toml");
    std::fs::write(
        &external_role,
        r#"name = "reviewer"
description = "Review code changes before handoff."
developer_instructions = "Review the diff and report concrete risks."
"#,
    )?;
    create_config_toml_with_extra(
        codex_home.path(),
        &format!(
            r#"
[agents.reviewer]
description = "Review code changes before handoff."
config_file = "{}"
"#,
            external_role.display()
        ),
    )?;
    let mut app = TestAppServer::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, app.initialize()).await??;

    let request_id = app
        .send_agent_role_tool_selection_set_request(AgentRoleToolSelectionSetParams {
            role_name: "reviewer".to_string(),
            allowed_tools: Some(vec!["update_plan".to_string()]),
            denied_tools: None,
        })
        .await?;
    let error: JSONRPCError = timeout(
        DEFAULT_TIMEOUT,
        app.read_stream_until_error_message(RequestId::Integer(request_id)),
    )
    .await??;

    assert_eq!(error.error.code, -32600);
    assert!(
        error
            .error
            .message
            .contains("not a discovered user role under $CODEX_HOME/agents")
    );
    let contents = std::fs::read_to_string(external_role)?;
    assert!(!contents.contains("[tool_selection]"));
    Ok(())
}

#[tokio::test]
async fn agent_role_action_policy_set_updates_user_role_file() -> Result<()> {
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path())?;
    let agents_dir = codex_home.path().join("agents");
    std::fs::create_dir_all(&agents_dir)?;
    let role_path = agents_dir.join("reviewer.toml");
    std::fs::write(
        &role_path,
        r#"name = "reviewer"
description = "Review code changes before handoff."
developer_instructions = "Review the diff and report concrete risks."

[subagent_action_policy]
allowed_actions = ["agent_message_send"]
"#,
    )?;
    let mut app = TestAppServer::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, app.initialize()).await??;

    let request_id = app
        .send_agent_role_action_policy_set_request(AgentRoleActionPolicySetParams {
            role_name: "reviewer".to_string(),
            allowed_actions: Some(vec![
                SubAgentActionPolicyAction::AgentMessageSend,
                SubAgentActionPolicyAction::AgentRetry,
            ]),
            denied_actions: Some(vec![SubAgentActionPolicyAction::AgentClose]),
        })
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        app.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let set_response = to_response::<AgentRoleActionPolicySetResponse>(response)?;

    assert_eq!(set_response.role_name, "reviewer");
    assert_eq!(
        set_response.allowed_actions,
        Some(vec![
            SubAgentActionPolicyAction::AgentMessageSend,
            SubAgentActionPolicyAction::AgentRetry,
        ])
    );
    assert_eq!(
        set_response.denied_actions,
        Some(vec![SubAgentActionPolicyAction::AgentClose])
    );
    assert_eq!(set_response.file_path.as_path(), role_path.as_path());
    let contents = std::fs::read_to_string(&role_path)?;
    assert!(contents.contains("[subagent_action_policy]\nallowed_actions = ["));
    assert!(contents.contains("\"agent_message_send\""));
    assert!(contents.contains("\"agent_retry\""));
    assert!(contents.contains("denied_actions = ["));
    assert!(contents.contains("\"agent_close\""));

    let request_id = app
        .send_agent_role_action_policy_set_request(AgentRoleActionPolicySetParams {
            role_name: "reviewer".to_string(),
            allowed_actions: None,
            denied_actions: None,
        })
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        app.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let set_response = to_response::<AgentRoleActionPolicySetResponse>(response)?;

    assert_eq!(set_response.role_name, "reviewer");
    assert_eq!(set_response.allowed_actions, None);
    assert_eq!(set_response.denied_actions, None);
    let contents = std::fs::read_to_string(role_path)?;
    assert!(!contents.contains("[subagent_action_policy]"));
    assert!(!contents.contains("allowed_actions"));
    assert!(!contents.contains("denied_actions"));
    Ok(())
}

#[tokio::test]
async fn agent_role_action_policy_set_rejects_external_config_file() -> Result<()> {
    let codex_home = TempDir::new()?;
    let external_dir = TempDir::new()?;
    let external_role = external_dir.path().join("reviewer.toml");
    std::fs::write(
        &external_role,
        r#"name = "reviewer"
description = "Review code changes before handoff."
developer_instructions = "Review the diff and report concrete risks."
"#,
    )?;
    create_config_toml_with_extra(
        codex_home.path(),
        &format!(
            r#"
[agents.reviewer]
description = "Review code changes before handoff."
config_file = "{}"
"#,
            external_role.display()
        ),
    )?;
    let mut app = TestAppServer::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, app.initialize()).await??;

    let request_id = app
        .send_agent_role_action_policy_set_request(AgentRoleActionPolicySetParams {
            role_name: "reviewer".to_string(),
            allowed_actions: Some(vec![SubAgentActionPolicyAction::AgentMessageSend]),
            denied_actions: None,
        })
        .await?;
    let error: JSONRPCError = timeout(
        DEFAULT_TIMEOUT,
        app.read_stream_until_error_message(RequestId::Integer(request_id)),
    )
    .await??;

    assert_eq!(error.error.code, -32600);
    assert!(
        error
            .error
            .message
            .contains("not a discovered user role under $CODEX_HOME/agents")
    );
    let contents = std::fs::read_to_string(external_role)?;
    assert!(!contents.contains("[subagent_action_policy]"));
    Ok(())
}

fn create_config_toml(codex_home: &std::path::Path) -> std::io::Result<()> {
    create_config_toml_with_extra(codex_home, "")
}

fn create_config_toml_with_extra(codex_home: &std::path::Path, extra: &str) -> std::io::Result<()> {
    std::fs::write(
        codex_home.join("config.toml"),
        format!(
            r#"
model = "mock-model"
sandbox_mode = "read-only"

model_provider = "mock_provider"

[model_providers.mock_provider]
name = "Mock provider for test"
base_url = "http://127.0.0.1:9/v1"
wire_api = "responses"
request_max_retries = 0
stream_max_retries = 0
"#,
        ) + extra,
    )
}
