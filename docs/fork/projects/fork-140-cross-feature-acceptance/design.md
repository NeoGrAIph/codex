# fork/140 cross-feature acceptance design

## Acceptance model

Cross-feature acceptance is a verification contract, not a runtime subsystem. The native source of truth stays inside each owning feature:

- Provider/model state: `Config.model_provider_id`, `Config.disabled_model_providers`, provider catalog/runtime state, app-server `modelProvider/*` and provider-aware `model/list`.
- Role authoring state: `$CODEX_HOME/agents/*.toml`, native role template parser/loader and TUI `/agent-roles`.
- Tool selection state: `[tool_selection] allowed_tools` in native config, enforced in `spec_plan` before `tool_search`, Code Mode synthetic tools and `ToolRegistry` dispatch.
- MCP state: configured MCP servers and active manager state owned by `McpConnectionManager`; app-server refresh remains `config/mcpServer/reload`.
- Workbench/runtime state: native MAv2 thread metadata, `AgentPath`, `ThreadEventStore`, `ThreadStatus`, app-server thread projections and TUI selection views.

## Cross-feature flows

| Flow | Expected native propagation |
| --- | --- |
| Role tool selection + MCP | Direct and deferred MCP tools are included or filtered by `spec_plan`; `tool_search` indexes only effective deferred tools; blocked MCP tools do not appear through registry dispatch. |
| Provider switch + next turn | Provider/model changes are applied to the next turn through thread runtime settings and provider-aware transport selection, not through TUI-only state. |
| Role template + provider defaults | Role TOML drafts can be seeded from current model/provider defaults while still using the native role file parser and `$CODEX_HOME/agents/*.toml` authoring path. |
| MCP config refresh + tool projection | `config/mcpServer/reload` queues native manager refresh for loaded threads; newly configured MCP tools appear on the next turn through normal request tool projection. |
| TUI role authoring + snapshots | `/agent-roles` exposes native TOML authoring and accepted snapshots; no pending `.snap.new` artifacts remain. |

## Non-goals

- No broad rewrite of MAv2, app-server, provider catalog or role loader.
- No model-visible MCP create/refresh tool.
- No sqlite migration for fork feature state.
- No manual snapshot file renaming.
- No claim that deferred workbench actions (`dismiss`, `retry`, `stop all`) are implemented.
