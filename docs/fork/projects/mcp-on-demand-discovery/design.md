# MCP on-demand discovery design

## Canonical state

The canonical runtime state remains owned by `McpConnectionManager`. The fork adds only a read-oriented model tool, `list_mcp_servers`, to expose the manager's current configured server inventory to the agent.

## Native propagation path

- Source of truth: `ToolRegistry` construction in `codex-rs/core/src/tools/spec_plan.rs`.
- MCP producers: `Session::built_tools` obtains direct/deferred MCP tools from `McpConnectionManager::list_all_tools`.
- Discovery projection: `add_mcp_resource_tools` registers `ListMcpServersHandler` when direct or deferred MCP tools exist.
- Direct MCP projection: direct MCP tools still register through `McpHandler`.
- Deferred MCP projection: deferred MCP tools still register with `ToolExposure::Deferred`; `tool_search` indexes their `ToolSearchInfo`.
- Manager projection: `McpConnectionManager::server_names` and `list_available_server_infos` expose read-only metadata already held by the manager.

## Data flow

1. Session startup creates `McpConnectionManager` from effective MCP config and plugin/runtime sources.
2. Turn tool planning receives direct and deferred MCP tool lists.
3. `spec_plan` adds `list_mcp_servers` whenever MCP tools exist in either direct or deferred set.
4. The model calls `list_mcp_servers` with optional `server`, `include_tools` and `max_tools_per_server`.
5. The handler reads server names, origin/plugin metadata, cached/available server info and optional `list_all_tools` output from the existing manager.
6. When `include_tools=true`, the handler sorts tools by canonical model-visible name, returns at most `max_tools_per_server` items per server, defaults to 50, hard-caps at 200, and reports `tools_total` plus `tools_truncated`.
7. MCP tool execution remains routed through raw `(server_name, tool.name)` in `McpHandler`.

## Invariants

- No duplicated MCP tool registry.
- No parallel MCP cache owned by core.
- No model-visible hardcoded list of MCP servers or tools.
- No startup, refresh, auth, config, or plugin install side effects.
- No unbounded tool list output; every per-server tool summary list is capped and reports truncation metadata.
- Deferred MCP tools remain discoverable through `tool_search`; `list_mcp_servers` does not replace search ranking or matching.

## Intentional tradeoffs

This iteration does not connect arbitrary new MCP servers from a tool call. That historical behavior overlaps with current plugin/config/app-server refresh surfaces and needs a separate contract if restored. The safe first step is inventory discovery for servers already configured and owned by the native manager.

## Intentionally unaffected surfaces

- App-server protocol/schema and `mcp_server_refresh`.
- MCP config parsing and persistence.
- Plugin install/list tools.
- MCP startup events and auth flows.
- MCP resource read/list behavior, except that `list_mcp_servers` may appear alongside resource tools.
