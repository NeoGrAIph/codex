# MCP on-demand discovery verification

## Required checks

- `cargo check -p codex-core`: verifies handler/spec wiring and cross-crate manager API use.
- `just test -p codex-core`: verifies tool planning, registry exposure and handler unit coverage.
- `just test -p codex-mcp`: verifies the manager crate after making metadata APIs public.
- `git diff --check`: verifies no whitespace errors.

## Scenarios

| Scenario | Expected result | Evidence |
| --- | --- | --- |
| Direct MCP tools exist | `list_mcp_servers` is model-visible together with MCP resource tools and direct MCP namespace tools. | `mcp_and_tool_search_follow_direct_and_deferred_tool_exposure` |
| Deferred MCP tools exist and search is available | `list_mcp_servers` and `tool_search` are model-visible; MCP resource tools are not exposed for deferred-only tools. | `mcp_and_tool_search_follow_direct_and_deferred_tool_exposure` |
| Deferred MCP tools exist but model lacks search tool | `list_mcp_servers` remains visible while `tool_search` is absent. | `mcp_and_tool_search_follow_direct_and_deferred_tool_exposure` |
| No MCP tools exist | `list_mcp_servers` is absent. | `mcp_and_tool_search_follow_direct_and_deferred_tool_exposure` |
| Tool output includes tools | `include_tools=true` groups normalized tool names by server and preserves raw MCP tool names. | `groups_tools_by_server_with_canonical_names` |
| Tool output is bounded | Per-server tool list is capped and reports total/truncation metadata. | `limits_tools_per_server_and_reports_truncation` |

## Coverage gaps

This iteration has no app-server or live external MCP integration test because it deliberately does not change app-server refresh or MCP startup behavior. A future iteration that creates or refreshes servers from a tool call must add an integration test that covers config/session refresh and auth failure behavior.
