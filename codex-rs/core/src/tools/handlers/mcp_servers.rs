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
use codex_tools::ToolName;
use codex_tools::ToolSpec;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;

pub struct ListMcpServersHandler;

const DEFAULT_MAX_TOOLS_PER_SERVER: usize = 50;
const MAX_TOOLS_PER_SERVER: usize = 200;
const MAX_SERVERS: usize = 50;
const MAX_TOTAL_TOOL_SUMMARIES: usize = 500;
const MAX_SERVER_FIELD_CHARS: usize = 160;
const MAX_SERVER_DESCRIPTION_CHARS: usize = 400;
const MAX_TOOL_DESCRIPTION_CHARS: usize = 240;

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
    servers_total: usize,
    servers_truncated: bool,
    tool_summaries_limit: usize,
}

#[derive(Debug, Serialize)]
struct McpServerSummary {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    origin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    plugin_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    info: Option<McpServerInfoSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<McpServerToolSummary>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools_total: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools_truncated: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct McpServerInfoSummary {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    website_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    icons_total: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    icons_truncated: Option<bool>,
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
        let all_server_names = manager.server_names();
        if let Some(server) = args.server.as_deref()
            && !all_server_names.iter().any(|name| name == server)
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

        let mut remaining_tool_summaries = MAX_TOTAL_TOOL_SUMMARIES;
        let filtered_server_names = all_server_names
            .iter()
            .filter(|name| args.server.as_ref().is_none_or(|server| server == *name))
            .cloned()
            .collect::<Vec<_>>();
        let servers_total = filtered_server_names.len();
        let servers_truncated = servers_total > MAX_SERVERS;
        let servers = filtered_server_names
            .into_iter()
            .take(MAX_SERVERS)
            .filter(|name| args.server.as_ref().is_none_or(|server| server == name))
            .map(|name| {
                let tools = tools_by_server.as_ref().map(|tools_by_server| {
                    let max_tools_for_this_server =
                        max_tools_per_server.min(remaining_tool_summaries);
                    limited_tools_for_server(
                        tools_by_server.get(&name).cloned().unwrap_or_default(),
                        max_tools_for_this_server,
                    )
                });
                if let Some(tools) = &tools {
                    remaining_tool_summaries =
                        remaining_tool_summaries.saturating_sub(tools.items.len());
                }
                McpServerSummary {
                    origin: manager
                        .server_origin(&name)
                        .map(|origin| truncate_field(origin, MAX_SERVER_FIELD_CHARS)),
                    plugin_id: manager
                        .plugin_id_for_mcp_server_name(&name)
                        .map(|plugin_id| truncate_field(plugin_id, MAX_SERVER_FIELD_CHARS)),
                    info: server_infos.get(&name).map(bounded_server_info),
                    tools: tools.as_ref().map(|tools| tools.items.clone()),
                    tools_total: tools.as_ref().map(|tools| tools.total),
                    tools_truncated: tools.as_ref().map(|tools| tools.truncated),
                    name: truncate_field(&name, MAX_SERVER_FIELD_CHARS),
                }
            })
            .collect();

        let output = serde_json::to_string_pretty(&ListMcpServersOutput {
            servers,
            servers_total,
            servers_truncated,
            tool_summaries_limit: MAX_TOTAL_TOOL_SUMMARIES,
        })
        .map_err(|err| {
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

fn bounded_server_info(info: &codex_protocol::mcp::McpServerInfo) -> McpServerInfoSummary {
    let icons_total = info.icons.as_ref().map(Vec::len);
    McpServerInfoSummary {
        name: truncate_field(&info.name, MAX_SERVER_FIELD_CHARS),
        title: info
            .title
            .as_deref()
            .map(|title| truncate_field(title, MAX_SERVER_FIELD_CHARS)),
        version: truncate_field(&info.version, MAX_SERVER_FIELD_CHARS),
        description: info
            .description
            .as_deref()
            .map(|description| truncate_field(description, MAX_SERVER_DESCRIPTION_CHARS)),
        website_url: info
            .website_url
            .as_deref()
            .map(|website_url| truncate_field(website_url, MAX_SERVER_FIELD_CHARS)),
        icons_total,
        icons_truncated: icons_total.map(|total| total > 0),
    }
}

fn truncate_field(value: &str, max_chars: usize) -> String {
    let mut truncated = value.chars().take(max_chars + 1).collect::<String>();
    if truncated.chars().count() <= max_chars {
        return truncated;
    }
    truncated = truncated.chars().take(max_chars).collect();
    truncated.push_str("...");
    truncated
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
            .map(|description| truncate_field(description, MAX_TOOL_DESCRIPTION_CHARS));
        tools_by_server
            .entry(server_name)
            .or_default()
            .push(McpServerToolSummary {
                name: format!("{}.{}", tool.callable_namespace, tool.callable_name),
                raw_name: truncate_field(&tool.tool.name, MAX_SERVER_FIELD_CHARS),
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
    fn groups_tools_by_server_caps_model_visible_fields() {
        let long_description = "d".repeat(MAX_TOOL_DESCRIPTION_CHARS + 20);
        let long_raw_name = "raw".repeat(MAX_SERVER_FIELD_CHARS);
        let tools = group_tools_by_server(vec![ToolInfo {
            server_name: "docs".to_string(),
            supports_parallel_tool_calls: false,
            server_origin: None,
            callable_name: "lookup".to_string(),
            callable_namespace: "mcp__docs".to_string(),
            namespace_description: None,
            tool: rmcp::model::Tool::new(
                long_raw_name,
                long_description,
                Arc::new(Default::default()),
            ),
            connector_id: None,
            connector_name: None,
            plugin_display_names: Vec::new(),
        }]);
        let tool = &tools.get("docs").expect("docs tools")[0];

        assert!(tool.raw_name.len() <= MAX_SERVER_FIELD_CHARS + 3);
        assert!(
            tool.description.as_ref().expect("description").len() <= MAX_TOOL_DESCRIPTION_CHARS + 3
        );
    }

    #[test]
    fn bounded_server_info_summarizes_icons_without_serializing_them() {
        let info = codex_protocol::mcp::McpServerInfo {
            name: "n".repeat(MAX_SERVER_FIELD_CHARS + 1),
            title: Some("t".repeat(MAX_SERVER_FIELD_CHARS + 1)),
            version: "v".repeat(MAX_SERVER_FIELD_CHARS + 1),
            description: Some("d".repeat(MAX_SERVER_DESCRIPTION_CHARS + 1)),
            icons: Some(vec![serde_json::json!({
                "src": "x".repeat(10_000),
            })]),
            website_url: Some("w".repeat(MAX_SERVER_FIELD_CHARS + 1)),
        };

        let summary = bounded_server_info(&info);

        assert!(summary.name.len() <= MAX_SERVER_FIELD_CHARS + 3);
        assert!(summary.title.expect("title").len() <= MAX_SERVER_FIELD_CHARS + 3);
        assert!(summary.version.len() <= MAX_SERVER_FIELD_CHARS + 3);
        assert!(
            summary.description.expect("description").len() <= MAX_SERVER_DESCRIPTION_CHARS + 3
        );
        assert_eq!(summary.icons_total, Some(1));
        assert_eq!(summary.icons_truncated, Some(true));
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
