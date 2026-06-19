# fork/140 cross-feature acceptance design

## Acceptance model

Cross-feature acceptance is a verification contract, not a runtime subsystem. It must not introduce a new registry, config section, protocol value or UI state. The native source of truth stays inside each owning feature:

- Provider/model state: `Config.model_provider_id`, `Config.disabled_model_providers`, provider catalog/runtime state, app-server `modelProvider/*` and provider-aware `model/list`.
- Role authoring state: `$CODEX_HOME/agents/*.toml`, native role template parser/loader and TUI `/agent-roles`.
- Tool selection state: `[tool_selection] allowed_tools` / `denied_tools` in native config, enforced in `spec_plan` before `tool_search`, Code Mode synthetic tools and `ToolRegistry` dispatch.
- MCP state: configured MCP servers and active manager state owned by `McpConnectionManager`; app-server refresh remains `config/mcpServer/reload`.
- Workbench/runtime state: native MAv2 thread metadata, `SessionSource::SubAgent(ThreadSpawn)`, `AgentPath`, `ThreadEventStore`, `ThreadStatus`, app-server thread projections and TUI selection views.
- Workbench visibility and retry state: rollout `SessionMeta.agent_hidden`, thread-store metadata, app-server `Thread.agentHidden` and durable `ThreadSpawn.initial_task`.
- Action policy state: native role TOML `[subagent_action_policy]`, effective `SubAgentActionPolicySnapshot` persisted in `ThreadSpawn.action_policy`, `ThreadManager` server-side workbench checks and TUI action projection.

## Cross-feature flows

| Flow | Expected native propagation |
| --- | --- |
| Role tool selection + MCP | Direct and deferred MCP tools are included or filtered by `spec_plan`; `tool_search` indexes only effective deferred tools; blocked MCP tools do not appear through registry dispatch. |
| Role action policy + Workbench | Role-applied action policy is snapshotted at spawn, shown in Agent Window as read-only evidence and enforced server-side by `ThreadManager` for mutating workbench actions. |
| Provider switch + next turn | Provider/model changes are applied to the next turn through thread runtime settings and provider-aware transport selection, not through TUI-only state. |
| Role template + provider defaults | Role TOML drafts can be seeded from current model/provider defaults while still using the native role file parser and `$CODEX_HOME/agents/*.toml` authoring path. |
| MCP config refresh + tool projection | `config/mcpServer/reload` queues native manager refresh for loaded threads; newly configured MCP tools appear on the next turn through normal request tool projection. |
| TUI role authoring + snapshots | `/agent-roles` exposes native TOML authoring and accepted snapshots; no pending `.snap.new` artifacts remain. |
| Workbench visibility + Resume | Dismissed agents are hidden through persisted metadata and remain readable/resumable through native thread history; Agent Window filtering consumes app-server `Thread.agentHidden` rather than local UI cache. |
| Retry + Role/tool/policy retention | Retry spawns a fresh sibling from durable `initial_task` and inherits the target's native config snapshot, cwd/model/provider/sandbox/tool policy/thread note/action policy without reading current role files as enforcement authority. |
| Stop all + Ownership | Stop-all is scoped to root tree or current `AgentPath` subtree, closes only live descendants via native `AgentControl::close_agent`, skips idle/completed/error/closed rows and reports stopped/failed counts. |

## Non-goals

- No broad rewrite of MAv2, app-server, provider catalog or role loader.
- No model-visible MCP create/refresh tool.
- No sqlite migration for fork feature state.
- No manual snapshot file renaming.
- No separate MCP-only policy surface; current acceptance covers MCP allow/deny management and enforcement through native role tool selection, generic role tool-selection writes and direct MCP execution checks.
- No claim that runtime markdown/frontmatter role loading exists; current acceptance covers import/compile-to-native TOML only.
- No claim that current-turn tool schema mutates in place after role edit; role/config changes apply through documented reload/next-turn boundaries.
