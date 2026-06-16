# MCP on-demand discovery project

## Status

Первая итерация для `fork/140` реализует read-only discovery поверх уже configured MCP manager state. Canonical feature contract: `docs/fork/features/mcp-on-demand-discovery.md`.

## Implementation map

- Tool planning: `codex-rs/core/src/tools/spec_plan.rs`.
- Tool spec: `codex-rs/core/src/tools/handlers/mcp_servers_spec.rs`.
- Tool handler: `codex-rs/core/src/tools/handlers/mcp_servers.rs`.
- MCP manager read API: `codex-rs/codex-mcp/src/connection_manager.rs`.
- Existing MCP execution path: `codex-rs/core/src/tools/handlers/mcp.rs`.
- Existing deferred discovery path: `codex-rs/core/src/tools/handlers/tool_search.rs`.

## Current user contract

`list_mcp_servers` appears only when the current turn has direct or deferred MCP tools. It lists configured server names and optional bounded tool summaries for already available MCP tools. `include_tools=true` defaults to 50 tools per server, accepts `max_tools_per_server`, hard-caps at 200 and reports `tools_total`/`tools_truncated`. It does not install plugins, mutate MCP config, refresh server connections, or start arbitrary new MCP servers.

For concrete task-oriented tool discovery, the agent should use native `tool_search`; `list_mcp_servers` is the inventory tool that tells the agent which MCP servers exist in the current session.

## Canonical links

- Feature passport: `docs/fork/features/mcp-on-demand-discovery.md`.
- Design: `docs/fork/projects/mcp-on-demand-discovery/design.md`.
- Verification: `docs/fork/projects/mcp-on-demand-discovery/verification.md`.
- Release research: `docs/fork/research/0.140.0/tools-mcp-skills.md`.
