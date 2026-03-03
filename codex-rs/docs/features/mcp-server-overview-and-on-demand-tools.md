# MCP Server Overview And On-Demand Tool Exposure

## Summary

This feature reduces initial prompt/context bloat from MCP servers by hiding non-app MCP tools
until the model explicitly asks for MCP server discovery.

Codex exposes a lightweight management tool, `list_mcp_servers`, for server-level overview and
selection state introspection.

In addition, Codex now injects a lightweight `## MCP Servers` overview into initial developer
context, so model sessions without tool access still see which MCP servers are configured.

## Behavior

- On startup, only `list_mcp_servers` is exposed for MCP discovery. Non-app MCP function tools and
  MCP resource tools stay hidden until unlock.
- MCP dispatch registers an internal fallback handler so qualified MCP calls can still route through
  MCP execution plumbing even when tool handlers are not registered one-by-one at startup.
- Initial context includes a lightweight `## MCP Servers` section:
  - server name + `description` from config (when `mcp_servers.<name>.description` is set),
  - otherwise server name + origin/transport hint fallback.
  This does not unlock tools and does not include tool schemas.
- `list_mcp_servers` provides server-level metadata and optional tool metadata without loading
  full tool schemas into the initial prompt.
- Any `list_mcp_servers` call unlocks non-app MCP tools:
  - with `server`: unlock tools only for that server;
  - without `server`: unlock tools for all non-app MCP servers.
- `search_tool_bm25` no longer unlocks non-app MCP tools; these remain hidden until
  `list_mcp_servers` is called.
- Calling `list_mcp_servers` with `activate_server` stores all tools from that server in
  `active_selected_tools`.
- Calling `list_mcp_servers` with `activate_server` for a server without available tools returns
  a model-facing error.
- Activated tools are exposed in subsequent turns via existing MCP selection plumbing.
- App tools (`codex_apps`) continue to use existing connector/mention-based behavior.
- Once a server is unlocked, tools from non-hydrated servers are still emitted with lightweight
  stub schemas until the first real MCP tool call hydrates that server.
- Hydration is tracked per server at MCP tool-call dispatch time and persisted in session state.

## Tool Contract

`list_mcp_servers` arguments:

- `server?: string` — optional server filter
- `include_tools?: boolean` — include per-server tool metadata (default: `false`)
- `activate_server?: string` — activate all tools for this server for future turns

Response payload includes:

- `servers[]` with `server`, optional `description`, `origin`, `toolCount`, `activated`, and
  optional `tools[]`
- `active_selected_tools[]`

## Resume/Fork

Selections from both `search_tool_bm25` and `list_mcp_servers` are restored from rollout via
`active_selected_tools`.

Hydrated MCP servers are also restored from rollout by scanning historical MCP tool calls
(`mcp__<server>__<tool>`), so previously hydrated servers keep full schemas after resume/fork.
