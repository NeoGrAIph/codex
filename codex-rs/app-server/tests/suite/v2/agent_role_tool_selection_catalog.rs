use std::collections::HashMap;
use std::time::Duration;

use anyhow::Result;
use app_test_support::TestAppServer;
use app_test_support::to_response;
use codex_app_server_protocol::AgentRoleToolSelectionCatalogExposure;
use codex_app_server_protocol::AgentRoleToolSelectionCatalogReadParams;
use codex_app_server_protocol::AgentRoleToolSelectionCatalogReadResponse;
use codex_app_server_protocol::JSONRPCResponse;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ThreadStartParams;
use codex_app_server_protocol::ThreadStartResponse;
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

fn create_config_toml(codex_home: &std::path::Path) -> std::io::Result<()> {
    std::fs::write(
        codex_home.join("config.toml"),
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
    )
}
