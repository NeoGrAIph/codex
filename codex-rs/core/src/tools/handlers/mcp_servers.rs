use crate::function_tool::FunctionCallError;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::context::boxed_tool_output;
use crate::tools::handlers::mcp_servers_spec::create_list_mcp_servers_tool;
use crate::tools::handlers::parse_arguments;
use crate::tools::registry::CoreToolRuntime;
use crate::tools::registry::ToolExecutor;
use codex_mcp::ToolInfo;
use codex_protocol::mcp::McpServerInfo;
use codex_tools::ToolName;
use codex_tools::ToolSpec;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;

pub struct ListMcpServersHandler;

const DEFAULT_MAX_TOOLS_PER_SERVER: usize = 50;
const MAX_TOOLS_PER_SERVER: usize = 200;

#[derive(Debug, Deserialize, Default)]
struct ListMcpServersArgs {
    #[serde(default)]
    server: Option<String>,
    #[serde(default)]
    include_tools: bool,
    #[serde(default)]
    max_tools_per_server: Option<usize>,
}

#[derive(Debug, Serialize)]
struct ListMcpServersOutput {
    servers: Vec<McpServerSummary>,
}

#[derive(Debug, Serialize)]
struct McpServerSummary {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    origin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    plugin_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    info: Option<McpServerInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<McpServerToolSummary>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools_total: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools_truncated: Option<bool>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct McpServerToolSummary {
    name: String,
    raw_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

impl ToolExecutor<ToolInvocation> for ListMcpServersHandler {
    fn tool_name(&self) -> ToolName {
        ToolName::plain("list_mcp_servers")
    }

    fn spec(&self) -> ToolSpec {
        create_list_mcp_servers_tool()
    }

    fn supports_parallel_tool_calls(&self) -> bool {
        true
    }

    fn handle(&self, invocation: ToolInvocation) -> codex_tools::ToolExecutorFuture<'_> {
        Box::pin(self.handle_call(invocation))
    }
}

impl ListMcpServersHandler {
    async fn handle_call(
        &self,
        invocation: ToolInvocation,
    ) -> Result<Box<dyn crate::tools::context::ToolOutput>, FunctionCallError> {
        let ToolInvocation {
            session, payload, ..
        } = invocation;
        let arguments = match payload {
            ToolPayload::Function { arguments } => arguments,
            _ => {
                return Err(FunctionCallError::RespondToModel(
                    "list_mcp_servers handler received unsupported payload".to_string(),
                ));
            }
        };
        let args: ListMcpServersArgs = parse_arguments(&arguments)?;
        let manager = session.services.mcp_connection_manager.load_full();
        let server_names = manager.server_names();
        if let Some(server) = args.server.as_deref()
            && !server_names.iter().any(|name| name == server)
        {
            return Err(FunctionCallError::RespondToModel(format!(
                "unknown MCP server `{server}`"
            )));
        }

        let server_infos = manager.list_available_server_infos().await;
        let tools_by_server = if args.include_tools {
            Some(group_tools_by_server(manager.list_all_tools().await))
        } else {
            None
        };
        let max_tools_per_server = args
            .max_tools_per_server
            .unwrap_or(DEFAULT_MAX_TOOLS_PER_SERVER)
            .min(MAX_TOOLS_PER_SERVER);

        let servers = server_names
            .into_iter()
            .filter(|name| args.server.as_ref().is_none_or(|server| server == name))
            .map(|name| {
                let tools = tools_by_server.as_ref().map(|tools_by_server| {
                    limited_tools_for_server(
                        tools_by_server.get(&name).cloned().unwrap_or_default(),
                        max_tools_per_server,
                    )
                });
                McpServerSummary {
                    origin: manager.server_origin(&name).map(str::to_string),
                    plugin_id: manager
                        .plugin_id_for_mcp_server_name(&name)
                        .map(str::to_string),
                    info: server_infos.get(&name).cloned(),
                    tools: tools.as_ref().map(|tools| tools.items.clone()),
                    tools_total: tools.as_ref().map(|tools| tools.total),
                    tools_truncated: tools.as_ref().map(|tools| tools.truncated),
                    name,
                }
            })
            .collect();

        let output =
            serde_json::to_string_pretty(&ListMcpServersOutput { servers }).map_err(|err| {
                FunctionCallError::RespondToModel(format!(
                    "failed to serialize list_mcp_servers output: {err}"
                ))
            })?;
        Ok(boxed_tool_output(FunctionToolOutput::from_text(
            output, None,
        )))
    }
}

impl CoreToolRuntime for ListMcpServersHandler {
    fn matches_kind(&self, payload: &ToolPayload) -> bool {
        matches!(payload, ToolPayload::Function { .. })
    }
}

struct LimitedMcpServerTools {
    items: Vec<McpServerToolSummary>,
    total: usize,
    truncated: bool,
}

fn limited_tools_for_server(
    mut tools: Vec<McpServerToolSummary>,
    max_tools_per_server: usize,
) -> LimitedMcpServerTools {
    let total = tools.len();
    tools.truncate(max_tools_per_server);
    LimitedMcpServerTools {
        items: tools,
        total,
        truncated: total > max_tools_per_server,
    }
}

fn group_tools_by_server(tools: Vec<ToolInfo>) -> BTreeMap<String, Vec<McpServerToolSummary>> {
    let mut tools_by_server: BTreeMap<String, Vec<McpServerToolSummary>> = BTreeMap::new();
    for tool in tools {
        let server_name = tool.server_name.clone();
        let description = tool
            .tool
            .description
            .as_deref()
            .map(str::trim)
            .filter(|description| !description.is_empty())
            .map(str::to_string);
        tools_by_server
            .entry(server_name)
            .or_default()
            .push(McpServerToolSummary {
                name: format!("{}.{}", tool.callable_namespace, tool.callable_name),
                raw_name: tool.tool.name.to_string(),
                description,
            });
    }

    for tools in tools_by_server.values_mut() {
        tools.sort_by(|left, right| left.name.cmp(&right.name));
    }
    tools_by_server
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn groups_tools_by_server_with_canonical_names() {
        let tools = group_tools_by_server(vec![ToolInfo {
            server_name: "docs".to_string(),
            supports_parallel_tool_calls: false,
            server_origin: None,
            callable_name: "lookup".to_string(),
            callable_namespace: "mcp__docs".to_string(),
            namespace_description: None,
            tool: rmcp::model::Tool::new(
                "lookup".to_string(),
                "Look up docs".to_string(),
                Arc::new(Default::default()),
            ),
            connector_id: None,
            connector_name: None,
            plugin_display_names: Vec::new(),
        }]);

        assert_eq!(
            tools.get("docs"),
            Some(&vec![McpServerToolSummary {
                name: "mcp__docs.lookup".to_string(),
                raw_name: "lookup".to_string(),
                description: Some("Look up docs".to_string()),
            }])
        );
    }

    #[test]
    fn limits_tools_per_server_and_reports_truncation() {
        let limited = limited_tools_for_server(
            vec![
                McpServerToolSummary {
                    name: "mcp__docs.alpha".to_string(),
                    raw_name: "alpha".to_string(),
                    description: None,
                },
                McpServerToolSummary {
                    name: "mcp__docs.beta".to_string(),
                    raw_name: "beta".to_string(),
                    description: None,
                },
            ],
            1,
        );

        assert_eq!(
            limited.items,
            vec![McpServerToolSummary {
                name: "mcp__docs.alpha".to_string(),
                raw_name: "alpha".to_string(),
                description: None,
            }]
        );
        assert_eq!(limited.total, 2);
        assert!(limited.truncated);
    }
}
