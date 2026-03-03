use async_trait::async_trait;
use codex_protocol::models::FunctionCallOutputBody;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;

use crate::function_tool::FunctionCallError;
use crate::mcp::CODEX_APPS_MCP_SERVER_NAME;
use crate::mcp::effective_mcp_servers;
use crate::mcp_connection_manager::ToolInfo;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolOutput;
use crate::tools::context::ToolPayload;
use crate::tools::handlers::parse_arguments;
use crate::tools::registry::ToolHandler;
use crate::tools::registry::ToolKind;

pub const LIST_MCP_SERVERS_TOOL_NAME: &str = "list_mcp_servers";

pub struct McpServersHandler;

fn include_tools_default() -> bool {
    false
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct ListMcpServersArgs {
    server: Option<String>,
    #[serde(default = "include_tools_default")]
    include_tools: bool,
    activate_server: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct McpServerToolSummary {
    name: String,
    tool_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct McpServerSummary {
    server: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    origin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    tool_count: usize,
    activated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<McpServerToolSummary>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ListMcpServersPayload {
    servers: Vec<McpServerSummary>,
    #[serde(rename = "active_selected_tools")]
    active_selected_tools: Vec<String>,
}

#[async_trait]
impl ToolHandler for McpServersHandler {
    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<ToolOutput, FunctionCallError> {
        let ToolInvocation {
            payload,
            session,
            turn,
            ..
        } = invocation;

        let arguments = match payload {
            ToolPayload::Function { arguments } => arguments,
            _ => {
                return Err(FunctionCallError::RespondToModel(
                    "mcp_servers handler received unsupported payload".to_string(),
                ));
            }
        };

        let args: ListMcpServersArgs = parse_arguments(&arguments)?;
        let server_filter = normalize_optional_string(args.server);
        let activate_server = normalize_optional_string(args.activate_server);

        let tools = session
            .services
            .mcp_connection_manager
            .read()
            .await
            .list_all_tools()
            .await;

        let mut grouped_tools = group_tools_by_server(tools);
        let mut active_selected_tools = session.get_mcp_tool_selection().await.unwrap_or_default();

        let tools_to_unlock = collect_unlock_tool_names(&grouped_tools, server_filter.as_deref());
        if !tools_to_unlock.is_empty() {
            active_selected_tools = session.merge_mcp_tool_selection(tools_to_unlock).await;
        }

        if let Some(server_to_activate) = activate_server {
            let tools_for_server = grouped_tools
                .get(&server_to_activate)
                .map(|entries| {
                    entries
                        .iter()
                        .map(|entry| entry.name.clone())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            if tools_for_server.is_empty() {
                return Err(FunctionCallError::RespondToModel(format!(
                    "MCP server `{server_to_activate}` has no tools to activate",
                )));
            }

            active_selected_tools = session.merge_mcp_tool_selection(tools_for_server).await;
        }

        let auth = session.services.auth_manager.auth().await;
        let effective_servers = effective_mcp_servers(&turn.config, auth.as_ref());
        let mut server_names = effective_servers.keys().cloned().collect::<Vec<_>>();
        let server_descriptions = effective_servers
            .iter()
            .filter_map(|(server_name, config)| {
                config
                    .description
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(|value| (server_name.clone(), value.to_string()))
            })
            .collect::<HashMap<_, _>>();
        for server_name in grouped_tools.keys() {
            if !server_names.contains(server_name) {
                server_names.push(server_name.clone());
            }
        }
        server_names.sort();

        let manager = session.services.mcp_connection_manager.read().await;
        let mut servers = server_names
            .into_iter()
            .filter(|server_name| {
                server_filter
                    .as_ref()
                    .is_none_or(|filter| filter == server_name)
            })
            .map(|server_name| {
                let mut tools = grouped_tools.remove(&server_name).unwrap_or_default();
                tools.sort_by(|a, b| a.name.cmp(&b.name));
                let tool_count = tools.len();
                let activated = tools
                    .iter()
                    .any(|tool| active_selected_tools.contains(&tool.name));

                McpServerSummary {
                    origin: manager.server_origin(&server_name).map(str::to_string),
                    description: server_descriptions.get(&server_name).cloned(),
                    server: server_name,
                    tool_count,
                    activated,
                    tools: args.include_tools.then_some(tools),
                }
            })
            .collect::<Vec<_>>();
        servers.sort_by(|a, b| a.server.cmp(&b.server));

        active_selected_tools.sort();
        let payload = ListMcpServersPayload {
            servers,
            active_selected_tools,
        };

        let content = serde_json::to_string(&payload).map_err(|err| {
            FunctionCallError::Fatal(format!(
                "failed to serialize list_mcp_servers payload: {err}"
            ))
        })?;

        Ok(ToolOutput::Function {
            body: FunctionCallOutputBody::Text(content),
            success: Some(true),
        })
    }
}

fn collect_unlock_tool_names(
    grouped_tools: &HashMap<String, Vec<McpServerToolSummary>>,
    server_filter: Option<&str>,
) -> Vec<String> {
    match server_filter {
        Some(server_name) => {
            if server_name == CODEX_APPS_MCP_SERVER_NAME {
                return Vec::new();
            }
            grouped_tools
                .get(server_name)
                .into_iter()
                .flatten()
                .map(|entry| entry.name.clone())
                .collect()
        }
        None => grouped_tools
            .iter()
            .filter(|(server_name, _)| server_name.as_str() != CODEX_APPS_MCP_SERVER_NAME)
            .flat_map(|(_, entries)| entries.iter().map(|entry| entry.name.clone()))
            .collect(),
    }
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|candidate| {
        let trimmed = candidate.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

fn group_tools_by_server(
    tools: HashMap<String, ToolInfo>,
) -> HashMap<String, Vec<McpServerToolSummary>> {
    let mut grouped = HashMap::new();
    for (name, info) in tools {
        grouped
            .entry(info.server_name)
            .or_insert_with(Vec::new)
            .push(McpServerToolSummary {
                name,
                tool_name: info.tool_name,
                title: info.tool.title.clone(),
                description: info.tool.description.map(|value| value.to_string()),
            });
    }
    grouped
}
