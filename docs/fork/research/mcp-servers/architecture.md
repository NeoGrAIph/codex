# Архитектурная концепция внешних MCP servers

Статус документа: текущая карта HEAD по source-of-truth и extension points для Codex как MCP client/runtime. Исторический timeline покрыт в `timeline.md`; проверочные команды и commit evidence - в `evidence.md`.

## Термины

- External MCP server: сервер, который Codex запускает или вызывает как клиент MCP. Источник может быть пользовательский `[mcp_servers]`, plugin declaration, selected executor plugin, extension contribution, built-in/compatibility registration or host-owned Apps MCP.
- `codex-rs/mcp-server`: отдельный контур, где Codex сам выступает MCP server для внешних clients. Этот документ не исследует его как основной runtime и упоминает только для разграничения терминов.
- Stdio MCP: локальный или environment-owned process transport с `command`, `args`, `env`, `env_vars`, optional `cwd`.
- Streamable HTTP MCP: remote HTTP transport with `url`, optional bearer token env var, static/env HTTP headers and OAuth login state.
- Apps MCP / `codex_apps`: host-owned MCP surface for ChatGPT Apps/connectors. It shares runtime and tool exposure machinery but has connector policy and auth rules.
- Selected plugin MCP: MCP declaration discovered from a selected executor plugin/capability root and activated only for the owning thread.
- MCP catalog: resolved source-of-truth for MCP server registrations after config, plugin, selected plugin, compatibility and extension contributions have been merged by precedence.

## Текущая карта source-of-truth

- Config contract: `codex-rs/config/src/mcp_types.rs` owns `McpServerConfig`, `RawMcpServerConfig`, `McpServerTransportConfig`, per-tool config, OAuth config, `enabled`, `required`, `supports_parallel_tool_calls`, startup/tool timeouts, tool allow/deny lists and remote stdio `cwd` validation.
- Config loading and requirements: `codex-rs/core/src/config/mod.rs` owns runtime `Config.mcp_servers`, requirements filtering, plugin MCP requirements, catalog construction and unsupported plaintext `bearer_token` diagnostics. `codex-rs/config/src/mcp_edit.rs` and `codex-rs/cli/src/mcp_cmd.rs` own config editing commands.
- Registration/catalog: `codex-rs/codex-mcp/src/catalog.rs` owns `McpServerRegistration`, `McpServerSource`, precedence and name-veto behavior; `codex-rs/codex-mcp/src/plugin_config.rs` parses plugin MCP declarations; `codex-rs/ext/extension-api/src/contributors/mcp.rs` defines extension-owned add/replace/remove contributions; `codex-rs/ext/mcp/src/executor_plugin.rs` discovers selected executor plugin MCP declarations.
- Runtime/client ownership: `codex-rs/codex-mcp/src/mcp/mod.rs` owns `McpConfig`, effective server assembly, auth status snapshots, resource reads and Apps MCP helpers; `codex-rs/codex-mcp/src/connection_manager.rs` owns running clients, startup status, tools/resources/templates aggregation, call routing and elicitation request management; `codex-rs/codex-mcp/src/runtime.rs` owns environment resolution and sandbox state.
- Transport implementation: `codex-rs/rmcp-client/src/rmcp_client.rs`, `stdio_server_launcher.rs`, `executor_process_transport.rs`, `http_client_adapter.rs`, `oauth.rs` and `perform_oauth_login.rs` own RMCP stdio/HTTP transport, OAuth persistence, token refresh/recovery and process cleanup.
- Model-visible tool exposure: `codex-rs/core/src/mcp_tool_exposure.rs`, `codex-rs/core/src/tools/handlers/mcp.rs`, `codex-rs/core/src/mcp_tool_call.rs`, `codex-rs/tools/src/mcp_tool.rs` and `codex-rs/tools/src/tool_config.rs` own direct/deferred MCP tool exposure, namespace normalization, hook payloads, approval review and tool-call execution.
- Resources/templates: `codex-rs/protocol/src/mcp.rs` owns internal MCP resource/template/content types; `codex-rs/core/src/tools/handlers/mcp_resource.rs` and sibling handlers expose `list_mcp_resources`, `list_mcp_resource_templates` and `read_mcp_resource` to the model.
- App-server API: `codex-rs/app-server-protocol/src/protocol/v2/mcp.rs` owns v2 wire shapes for status/list, resource read, tool call, refresh, OAuth login, startup status and elicitations; `codex-rs/app-server/src/request_processors/mcp_processor.rs` owns request handling; `codex-rs/app-server/src/mcp_refresh.rs` owns reload/refresh queuing.
- TUI projections: `codex-rs/tui/src/chatwidget/mcp_startup.rs` renders startup state; `codex-rs/tui/src/history_cell/mcp.rs` renders inventory, tool calls and outputs; `codex-rs/tui/src/bottom_pane/mcp_server_elicitation.rs` renders server elicitation forms; slash command handling routes `/mcp` through the app-server/session surfaces.
- Diagnostics and trace: `codex-rs/rollout-trace/src/mcp.rs`, MCP tool spans/metrics and app-server/TUI history items are evidence/projection surfaces. Runtime behavior still belongs in config/catalog/connection manager/tool handlers.

## Lifecycle flows

### Config and registration

1. Config layers deserialize `[mcp_servers.<name>]` into `McpServerConfig`; invalid transport combinations fail during config parsing.
2. Requirements and managed config can disable or constrain servers, preserving `disabled_reason` for diagnostics.
3. Plugin, selected plugin, compatibility and extension declarations become `McpServerRegistration` entries.
4. The catalog resolves precedence and disabled name-veto behavior before any connection startup.

### Startup and refresh

1. Session/thread runtime builds `McpConfig` from current config, auth, plugins and extension contributions.
2. `McpRuntimeContext` resolves each server's environment. Local stdio requires a local environment; remote stdio requires explicit absolute `cwd`; HTTP can run without stdio environment fallback.
3. `McpConnectionManager` starts enabled servers, emits per-server startup status and retains metadata such as origin, required flag, approval defaults and parallel-call support.
4. App-server `config/mcpServer/reload` queues strict refresh for loaded threads; refresh must keep thread/runtime scope and not mutate global tool lists directly.

### Tool exposure and calls

1. `McpConnectionManager` lists tools and normalizes canonical/callable names while preserving raw server/tool names for the actual MCP call.
2. `McpToolExposure` decides whether tools are direct model-visible tools or deferred through tool search. Apps MCP tools are additionally filtered by connector policy and app accessibility.
3. `McpHandler` routes model tool calls through `handle_mcp_tool_call`, preserving hook payloads, approval behavior, telemetry tags and plugin provenance.
4. Tool results become protocol/turn items and TUI/app-server history cells rather than transcript-only strings.

### Resources and app-server operations

1. Model tools can list resources/templates across all configured servers or by server, then read a specific resource by server and URI.
2. App-server `mcpServerStatus/list` can fetch tools, auth status, server info, resources and templates with optional thread context and pagination.
3. App-server `mcpServer/resource/read` supports thread-bound reads through loaded thread runtime, or threadless reads from latest config with config-level environment fallback.
4. App-server `mcpServer/tool/call` requires `threadId` and injects thread metadata into `_meta` before routing to the thread's configured MCP runtime.

### Auth, approvals and elicitations

1. Streamable HTTP servers can use bearer token env vars or OAuth credentials; plaintext `bearer_token` is rejected with targeted diagnostics.
2. `mcpServer/oauth/login` resolves configured/requested/discovered scopes, honors OAuth client id/resource/callback settings and sends completion notification.
3. MCP tool approval can come from global approval policy, server-level default, per-tool override, Apps connector policy or approval reviewer integration.
4. Server-driven elicitations travel through typed app-server/TUI request/response surfaces and can pause active-time accounting while the user/client responds.

### Plugins, extensions and thread scope

1. Plugin MCP declarations are parsed into normal MCP configs and attributed for UI/tool provenance.
2. Selected executor plugin MCP declarations are discovered from the selected environment-owned capability root and activated only for the thread that selected that root.
3. Extension MCP contributions add/replace/remove registrations through contributor APIs and must remain scoped to the runtime context they were resolved for.
4. Thread-scoped MCP contributions must not leak startup status, tools, resources or plugin provenance into unrelated threads.

## Developer guidance for fork changes

- Add new MCP config fields in `codex-rs/config/src/mcp_types.rs`, update schema/tests and prove the exact runtime parser accepts the field before documenting it as usable.
- Prefer `codex-rs/codex-mcp/src/connection_manager.rs`, `catalog.rs`, `runtime.rs` and `mcp/mod.rs` for MCP runtime changes; avoid plumbing ad hoc server/tool maps through `codex-core`.
- Preserve one source of truth for registration: config/plugin/extension declarations should become catalog entries, then effective servers. Do not add hardcoded tool lists or one-off UI switches.
- If a change affects model-visible tools, update direct/deferred exposure, namespace normalization, tool search metadata, hook payloads and telemetry together.
- If a change affects app-server behavior, update `app-server-protocol` v2 types, generated schema/fixtures and app-server README entries together.
- If a change affects TUI-visible startup, inventory, tool-call or elicitation rendering, update TUI tests/snapshots and keep thread-id routing explicit.
- If a change affects sandbox, cwd, environment id, OAuth, bearer tokens, approvals or elicitations, include positive and negative security tests; do not add silent fallback for unsafe or unparseable config.
- For selected plugin or extension MCPs, validate thread scope: selected declarations should be visible only in the owning thread/runtime and refresh should preserve that boundary.

## Native propagation rule for fork work

Новая MCP capability должна проходить через существующий source-of-truth и projections:

- config field -> config/schema/tests -> catalog/effective server -> connection manager -> app-server/TUI diagnostics;
- plugin/extension declaration -> registration catalog -> effective server -> thread/runtime scoped startup -> tool/resource status;
- tool metadata -> MCP tool normalization -> `McpHandler` -> protocol item/app-server history -> TUI history cell;
- resource metadata -> protocol MCP resource types -> model resource tools -> app-server status/resource read;
- approval/elicitation behavior -> config/app policy -> connection manager/reviewer -> app-server request -> TUI bottom pane response;
- environment/cwd/permission state -> `McpRuntimeContext`/sandbox state -> RMCP transport -> teardown/recovery tests.

Антипаттерны: parallel config, duplicated server registries, global plugin MCP activation, UI-only hiding, transcript scraping, direct app-server-to-plugin calls that bypass catalog/runtime resolution, and fallback to local cwd for named remote stdio servers.

## Compatibility notes and open questions

- `mcp__` tool prefixes and later manager-owned naming modes are compatibility-sensitive; raw server/tool names must remain available for actual MCP protocol calls.
- `enabled_tools` and `disabled_tools` are exposure filters within one server, not a universal permission model for built-in tools.
- `required = true` changes startup failure semantics and can make `codex exec` fail fast; UI-only warnings are not equivalent.
- Threadless app-server resource reads are useful for inventory, but they do not have turn-selected environment/cwd and must not pretend to inherit a thread runtime.
- Apps MCP uses host/auth/connector policy in addition to generic MCP rules; custom MCP servers should not inherit Apps-specific auth or route filters unless they are explicitly host-owned Apps registrations.
- Selected executor plugin MCPs currently cover stdio declarations; HTTP executor/plugin placement and dynamic lifecycle remain separate design decisions unless future source evidence changes this.
- `codex-rs/mcp-server` may share names and app-server protocol concepts, but it is not the owner of external MCP server runtime behavior.
