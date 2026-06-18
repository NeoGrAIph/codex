# Agent Role-Level Tool Selection Project

## Current Status

Статус: first runtime slice, runtime stale-id diagnostics, core runtime catalog projection, loaded-thread app-server catalog read contract, cold resume verification, read-only role detail/effective-state projection, native TOML draft authoring, catalog-assisted create flow, existing user-role allowlist editing and `/agent-roles` current-thread catalog reference implemented locally in `fork/140` dirty state. Source of truth выбран: `[tool_selection] allowed_tools = [...]` в native config layer, применяемый через role `config_file` и `Config`/`TurnContext.config`.

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
| Effective policy source | `codex-rs/config/src/config_toml.rs::ToolSelectionToml` and `codex-rs/core/src/config/mod.rs::ToolSelectionConfig` carry `allowed_tools` through native config. |
| Tool planning is centralized | `codex-rs/core/src/tools/spec_plan.rs::build_tool_specs_and_registry` builds sources, then `tool_search`, then Code Mode executors, then model-visible specs and `ToolRegistry`. |
| Tool selection enforcement | `codex-rs/core/src/tools/spec_plan.rs::apply_tool_selection_policy` filters `PlannedTools.runtimes` and `PlannedTools.hosted_specs` before derived projections. |
| Runtime diagnostics/catalog | `codex-rs/core/src/tools/router.rs::ToolSelectionDiagnostics` and `codex-rs/core/src/session/turn.rs::built_tools` project unmatched allowlist entries from the enforced `PlannedTools` into an existing warning once per turn and expose planner-owned inventory catalog entries with selected state and exposure metadata for app-server and read-only TUI surfaces. |
| App-server catalog read | `codex-rs/core/src/codex_thread.rs::CodexThread::tool_selection_catalog`, `codex-rs/app-server/src/request_processors/catalog_processor.rs::read_agent_role_tool_selection_catalog` and `agentRole/toolSelectionCatalog/read` expose a thread-scoped read-only projection for loaded threads. |
| Role detail and authoring projection | `codex-rs/core/src/agent_role_templates.rs::AgentRoleTemplateLocks.allowed_tool_names`, `user_agent_role_template_draft_with_allowed_tools`, `update_user_agent_role_template_from_draft` and `codex-rs/tui/src/chatwidget/agent_role_templates.rs` show declared allowlist plus read-only effective-state copy, current-thread runtime catalog reference and native TOML create/edit flows when the loaded-thread app-server read succeeds. |
| Tool exposure has direct/deferred/code-mode implications | `codex-rs/tools/src/tool_executor.rs::ToolExposure` distinguishes `Direct`, `Deferred`, `DirectModelOnly` and `Hidden`. |
| MCP has raw and model-visible identities | `codex-rs/codex-mcp/src/tools.rs::ToolInfo` stores raw `server_name`/`tool.name` plus model-visible `callable_namespace`/`callable_name`. |
| Dispatch already has controlled unsupported behavior | `codex-rs/core/src/tools/registry.rs::dispatch_any_with_terminal_outcome` returns `unsupported call: <tool>` for missing runtimes. |

## Remaining Gaps

- Stronger persisted policy retention after source config removal/change remains a future contract question; cold resume with the same `$CODEX_HOME/config.toml` and fresh config restart reload are tested.
- Direct sandbox/approval negative assertion only if this feature later changes security-policy surfaces; native availability no-grant and blocked dispatch behavior are already tested.
- Optional app-server write/management contract for external clients. The local TUI editing path intentionally stays file-native: valid discovered user roles under `$CODEX_HOME/agents/*.toml` can be edited through a picker-backed TOML draft, while external `config_file` roles remain on the open-file path until ownership/write semantics are defined.

## Implemented Slice

Implemented v1:

- added native config type and parser/schema contract;
- exposed effective policy through `Config`/`TurnContext.config`;
- added planner-level filter over `PlannedTools`;
- covered model-visible specs, registry, `tool_search`, MCP direct/deferred tools, hosted specs and Code Mode in focused `codex-core` tests;
- covered blocked direct dispatch through the native unsupported-tool error path;
- covered fresh config restart reload through the native `$CODEX_HOME` role file path;
- covered cold thread resume through the same `$CODEX_HOME/config.toml`;
- added runtime stale-id diagnostics through `ToolRouter` and existing warning events;
- added runtime catalog projection with selected state and exposure metadata through `ToolSelectionDiagnostics.catalog_entries`;
- added loaded-thread app-server read contract `agentRole/toolSelectionCatalog/read`;
- added `/agent-roles` read-only current-thread runtime catalog reference with snapshot coverage;
- added read-only role detail/effective-state projection with TUI snapshot coverage;
- added a commented `[tool_selection] allowed_tools` example to the native TOML role-template draft;
- added catalog-assisted create UX that converts selected current-thread tool ids into an editable native TOML draft;
- added existing user-role edit UX that preselects the role's current allowlist, converts picker selection into an editable native TOML draft and overwrites the same `$CODEX_HOME/agents/*.toml` role file through the native parser-validated path;
- deferred external app-server write/management APIs until a client-facing editing contract is defined.
