# tools-mcp-skills research for 0.140.0

## Baseline

- Release baseline: `rust-v0.140.0` / `fork/140` working branch.
- Feature branch: `feature/140/tools-mcp-skills`.
- Historical sources: `fork/106` for MCP on-demand discovery and `fork/multi-agent` for `run_skill_script`.

## Current upstream-shaped architecture

- Tool source of truth is `codex-rs/core/src/tools/spec_plan.rs`, which builds model-visible specs and `ToolRegistry` from native producers.
- MCP connection ownership is `codex-rs/codex-mcp/src/connection_manager.rs`; core should read manager state but not duplicate MCP cache or startup ownership.
- MCP tool execution is `codex-rs/core/src/tools/handlers/mcp.rs`.
- Deferred tool discovery is `ToolExposure::Deferred` plus `tool_search`.
- Unified exec execution is `codex-rs/core/src/tools/handlers/unified_exec/exec_command.rs`.
- Skill runtime metadata for the turn is `TurnContext.turn_skills.outcome`.

## Gap analysis

`mcp-on-demand-discovery` historical behavior included broader discovery/connect semantics. In 0.140.0, native MCP/plugin/app-server refresh surfaces already cover some of that space. First iteration therefore implements only read-only server inventory as `list_mcp_servers`, leaving model-visible server creation/refresh out of scope. Optional tool summaries are bounded per server so discovery cannot inject an unbounded tool list into model context.

`fork/140` v2 verifies the native app-server refresh path instead of adding a second lifecycle manager: `config/mcpServer/reload` reloads config, queues `Op::RefreshMcpServers` for loaded threads, and core replaces the thread's `McpConnectionManager` before the next turn. Focused acceptance covers adding an MCP server to config after thread start and observing the new direct MCP tool in the next Responses request.

`run-skill-script` historical behavior exposed a dedicated tool. In 0.140.0, native skills and unified exec already exist, so the fork implementation is an adapter that validates an enabled local skill script, preserves Bash hook lifecycle on the outer registry dispatch and delegates execution to `exec_command`.

## Risky integration points

- Tool visibility must stay in `spec_plan`; adding side registries would break direct/deferred/code-mode behavior.
- MCP metadata APIs must be read-only; manager startup, refresh and auth behavior must remain native.
- App-server refresh acceptance must prove next-turn tool visibility through the normal request projection, not a side cache or direct test-only manager mutation.
- `list_mcp_servers include_tools=true` must stay bounded and report truncation metadata.
- `run_skill_script` must not introduce a parallel permission model or bypass Bash PreToolUse/PostToolUse hooks.
- Non-primary or remote skill execution cannot safely reuse host validation in this iteration; it must fail fast until a native filesystem-aware execution contract exists.
- Code mode inherits tool visibility through existing nested tool collection; new specs must not require custom code-mode branches.

## Verification notes

Focused verification should cover `codex-core` because both tools are planned and dispatched there, and `codex-mcp` because manager metadata APIs changed. No app-server schema regeneration is expected because this iteration adds no protocol type or app-server command. The v2 app-server acceptance is `just test -p codex-app-server mcp_server_refresh_exposes_added_tools_on_next_turn`; it passed locally and confirms the existing `config/mcpServer/reload` contract makes new configured MCP tools visible on the next turn.
