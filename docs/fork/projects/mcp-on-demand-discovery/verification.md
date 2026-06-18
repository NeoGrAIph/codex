# MCP on-demand discovery verification

## Required checks

- `cargo check -p codex-core`: verifies handler/spec wiring and cross-crate manager API use.
- `just test -p codex-core`: verifies tool planning, registry exposure and handler unit coverage.
- `just test -p codex-mcp`: verifies the manager crate after making metadata APIs public.
- `git diff --check`: verifies no whitespace errors.

## Verification log

| Date | Command | Result | Notes |
| --- | --- | --- | --- |
| 2026-06-18 | `cargo check -p codex-core` | passed | Подтверждает compile path для core handler/spec wiring и `codex-mcp` manager API use. |
| 2026-06-18 | `just test -p codex-core mcp_and_tool_search_follow_direct_and_deferred_tool_exposure groups_tools_by_server_with_canonical_names limits_tools_per_server_and_reports_truncation run_skill_script environment_count_controls_environment_backed_tools environment_id_is_only_present_for_multiple_environments run_skill_script_executes_enabled_skill_helper_through_unified_exec` | 15/15 passed | Совместный focused pass для MCP discovery и `run_skill_script`; MCP-specific assertions покрывают model-visible exposure, bounded output и canonical names. |
| 2026-06-18 | `just test -p codex-mcp` | 82/82 passed | Подтверждает, что публичные metadata/listing paths не ломают existing MCP connection manager, runtime, auth, catalog and filtering behavior. |
| 2026-06-18 | `just test -p codex-core mcp_and_tool_search_follow_direct_and_deferred_tool_exposure groups_tools_by_server_with_canonical_names limits_tools_per_server_and_reports_truncation ...`; `just test -p codex-mcp` | core 16/16; mcp 82/82 passed | Current-state smoke confirms `list_mcp_servers` exposure still follows direct/deferred MCP tool planning, grouped tool summaries stay bounded, canonical/raw names are preserved, and the MCP manager crate remains compatible. |

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
