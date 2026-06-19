# Agent Role-Level Tool Selection Project

## Current Status

Статус: native role tool policy v2 implemented locally in `fork/140` dirty state. Source of truth выбран: `[tool_selection] allowed_tools = [...]` и `denied_tools = [...]` в native config layer, применяемый через role `config_file` и `Config`/`TurnContext.config`. Реализованы runtime stale-id diagnostics, core runtime catalog projection, loaded-thread app-server catalog read, direct app-server MCP execution gate, whole-policy app-server write API for discovered user role files, read-only role detail/effective-state projection, native TOML draft authoring, catalog-assisted create/edit flow for allowed and denied tools, `/agent-roles` current-thread catalog reference and `ThreadSpawn.tool_selection` retention for path-backed sub-agent resume/retry.

Фича намеренно отделена от `agent-role-templates`: role templates отвечают за authoring/profile layer, а role-level tool selection меняет security-sensitive tool planning, deferred discovery, Code Mode nested tools and dispatch registry.

## Canonical Links

| Artifact | Purpose |
| --- | --- |
| `docs/fork/features/agent-role-tool-selection.md` | Feature-level contract, user behavior, propagation and verification matrix. |
| `docs/fork/projects/agent-role-tool-selection/design.md` | Native source-of-truth analysis and proposed architecture gate. |
| `docs/fork/projects/agent-role-tool-selection/verification.md` | Focused verification matrix and log for the implemented runtime/catalog slice. |
| `docs/fork/research/0.140.0/openclaude-agent-ux-runtime.md` | OpenClaude UX reference and Codex fork/140 source-map evidence. |

## Evidence Map

| Concern | Current source evidence |
| --- | --- |
| Role files are config-layer inputs | `codex-rs/core/src/config/agent_roles.rs::parse_agent_role_file_contents` removes role metadata and leaves the remaining TOML as role-applied config. |
| Role application is native config rebuild | `codex-rs/core/src/agent/role.rs::apply_role_to_config` loads the role config file and rebuilds `Config` for the child thread. |
| Effective policy source | `codex-rs/config/src/config_toml.rs::ToolSelectionToml` and `codex-rs/core/src/config/mod.rs::ToolSelectionConfig` carry `allowed_tools`/`denied_tools` through native config. |
| Tool planning is centralized | `codex-rs/core/src/tools/spec_plan.rs::build_tool_specs_and_registry` builds sources, then `tool_search`, then Code Mode executors, then model-visible specs and `ToolRegistry`. |
| Tool selection enforcement | `codex-rs/core/src/tools/spec_plan.rs::apply_tool_selection_policy` filters `PlannedTools.runtimes` and `PlannedTools.hosted_specs` before derived projections. |
| Runtime diagnostics/catalog | `codex-rs/core/src/tools/router.rs::ToolSelectionDiagnostics` and `codex-rs/core/src/session/turn.rs::built_tools` project unmatched allowlist entries from the enforced `PlannedTools` into an existing warning once per turn and expose planner-owned inventory catalog entries with selected state and exposure metadata for app-server and read-only TUI surfaces. |
| App-server catalog read/write | `codex-rs/core/src/codex_thread.rs::CodexThread::tool_selection_catalog`, `codex-rs/app-server/src/request_processors/catalog_processor.rs::read_agent_role_tool_selection_catalog`, `agentRole/toolSelectionCatalog/read` and `agentRole/toolSelection/set` expose thread-scoped read evidence and whole-policy writes for discovered user role TOML files. |
| Direct MCP execution gate | `codex-rs/core/src/session/mcp.rs::Session::call_tool` checks the loaded thread's effective `Config.tool_selection` against native MCP inventory canonical names before `McpConnectionManager::call_tool`. |
| Role detail and authoring projection | `codex-rs/core/src/agent_role_templates.rs`, `user_agent_role_template_draft_with_allowed_tools`, `user_agent_role_template_draft_with_denied_tools`, `update_user_agent_role_template_from_draft` and `codex-rs/tui/src/chatwidget/agent_role_templates.rs` show declared allow/deny policy plus read-only effective-state copy, current-thread runtime catalog reference and native TOML create/edit flows when the loaded-thread app-server read succeeds. |
| Tool exposure has direct/deferred/code-mode implications | `codex-rs/tools/src/tool_executor.rs::ToolExposure` distinguishes `Direct`, `Deferred`, `DirectModelOnly` and `Hidden`. |
| MCP has raw and model-visible identities | `codex-rs/codex-mcp/src/tools.rs::ToolInfo` stores raw `server_name`/`tool.name` plus model-visible `callable_namespace`/`callable_name`. |
| Dispatch already has controlled unsupported behavior | `codex-rs/core/src/tools/registry.rs::dispatch_any_with_terminal_outcome` returns `unsupported call: <tool>` for missing runtimes. |
| Sub-agent retention | `codex-rs/protocol/src/protocol.rs::SubAgentToolSelectionSnapshot` and `SessionSource::SubAgent(ThreadSpawn.tool_selection)` persist effective allow/deny policy for already-spawned path-backed sub-agents. |

## Remaining Gaps

- Direct sandbox/approval negative assertion only if this feature later changes security-policy surfaces; native availability no-grant and blocked dispatch behavior are already tested.
- App-server write API is intentionally whole-policy only for existing discovered `$CODEX_HOME/agents/*.toml` user roles. Partial patches, optimistic `expectedVersion`, built-in role editing and external `config_file` editing remain outside the current contract.

## Implemented Slice

Implemented v1:

- added native config type and parser/schema contract;
- exposed effective policy through `Config`/`TurnContext.config`;
- added planner-level filter over `PlannedTools`;
- covered model-visible specs, registry, `tool_search`, MCP direct/deferred tools, hosted specs, Code Mode and deny-wins-over-allow in focused `codex-core` tests;
- covered blocked direct dispatch through the native unsupported-tool error path;
- covered fresh config restart reload through the native `$CODEX_HOME` role file path;
- covered cold thread resume through the same `$CODEX_HOME/config.toml`;
- added runtime stale-id diagnostics through `ToolRouter` and existing warning events;
- added runtime catalog projection with selected state and exposure metadata through `ToolSelectionDiagnostics.catalog_entries`;
- added loaded-thread app-server read contract `agentRole/toolSelectionCatalog/read`;
- added `/agent-roles` read-only current-thread runtime catalog reference with snapshot coverage;
- added read-only role detail/effective-state projection with TUI snapshot coverage;
- added a commented `[tool_selection] allowed_tools` example to the native TOML role-template draft;
- added separate catalog-assisted TUI staged controls for discovered user-role `allowed_tools` and `denied_tools`, both ending in the same parser-validated native TOML draft path;
- added catalog-assisted create UX that converts selected current-thread tool ids into an editable native TOML draft;
- added existing user-role edit UX that preselects the role's current allowlist, converts picker selection into an editable native TOML draft and overwrites the same `$CODEX_HOME/agents/*.toml` role file through the native parser-validated path;
- added experimental `agentRole/toolSelection/set` as whole-policy app-server write API for discovered user role TOML files;
- added `ThreadSpawn.tool_selection` retention so path-backed sub-agent resume/retry preserves the original effective allow/deny boundary after source config changes.
