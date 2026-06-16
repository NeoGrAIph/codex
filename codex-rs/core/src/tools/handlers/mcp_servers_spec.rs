use codex_tools::JsonSchema;
use codex_tools::ResponsesApiTool;
use codex_tools::ToolSpec;
use std::collections::BTreeMap;

pub fn create_list_mcp_servers_tool() -> ToolSpec {
    let properties = BTreeMap::from([
        (
            "server".to_string(),
            JsonSchema::string(Some(
                "MCP server name. Omit to list every configured MCP server.".to_string(),
            )),
        ),
        (
            "include_tools".to_string(),
            JsonSchema::boolean(Some(
                "Whether to include model-visible tool names for each server. Defaults to false. Tool lists are capped per server and report tools_total/tools_truncated in the response."
                    .to_string(),
            )),
        ),
        (
            "max_tools_per_server".to_string(),
            JsonSchema::integer(Some(
                "Maximum tools to return per server when include_tools is true. Defaults to 50 and is capped at 200."
                    .to_string(),
            )),
        ),
    ]);

    ToolSpec::Function(ResponsesApiTool {
        name: "list_mcp_servers".to_string(),
        description:
            "Lists configured MCP servers available in this session. Use tool_search to discover deferred MCP tools by task or capability."
                .to_string(),
        strict: false,
        defer_loading: None,
        parameters: JsonSchema::object(properties, /*required*/ None, Some(false.into())),
        output_schema: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_mcp_servers_spec_is_function_tool() {
        let ToolSpec::Function(tool) = create_list_mcp_servers_tool() else {
            panic!("expected function tool");
        };

        assert_eq!(tool.name, "list_mcp_servers");
        assert!(tool.description.contains("configured MCP servers"));
    }
}
