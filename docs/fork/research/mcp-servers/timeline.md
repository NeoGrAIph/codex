# Эволюция внешних MCP servers

Статус исследования: покрыты stable-релизы от первого внешнего MCP server commit `147a940449839b116b220b7e7d016d2a2890c134` до текущего локального `rust-v0.141.0`. Основной timeline строится по stable-contained commits; alpha/side-branch commits не включены в основной поток без отдельного указания.

## Поколения

- Initial stdio MCP: `[mcp_servers]` в config, запуск stdio server, tool-call routing, startup tolerance and timeouts.
- RMCP/streamable HTTP/OAuth: переход на `rmcp`, streamable HTTP transport, OAuth credentials, headers, scopes, resource listing/read и tool filtering.
- App-server/TUI projection: `/mcp`, startup status, app-server MCP status/resource/tool APIs, elicitations and approval UI.
- Apps/plugins/deferred tools: host-owned Apps MCP, plugin MCP declarations, tool search/deferred exposure and connector policy.
- Runtime/environment isolation: thread/runtime-scoped MCP config, executor-backed stdio, explicit environments, permission profiles and shutdown/process cleanup.
- Catalog/extensions/selected plugins: `codex-mcp` catalog, extension contributions, selected executor plugin MCPs and thread-scoped activation.

## `rust-v0.2.0`

- First containing stable for external MCP server support: `rust-v0.2.0`.

### Initial configured MCP servers (`#829`, `#854`)

- Коммиты: `147a940449` (`feat: support mcp_servers in config.toml (#829)`), `27198bfe11` (`fix: make McpConnectionManager tolerant of MCPs that fail to start (#854)`).
- Developer info: adds `[mcp_servers]` config, first `McpConnectionManager`, stdio server launch and MCP tool-call routing from model tool calls into configured servers; then makes startup tolerant when a server fails.
- Why: commit body for `#829` explicitly describes external MCP servers in Claude Desktop/Cursor style and calls out `mcp_connection_manager.rs` plus `mcp_tool_call.rs` as the main implementation paths.
- User info: Codex can use external tools such as a weather MCP server from config; one broken optional server no longer prevents the whole session from continuing.
- Compatibility/risks: the first implementation launched all configured servers eagerly, had limited recovery, and ran MCP server code outside the Codex sandbox boundary described in the commit body.
- Evidence: `EVID-002-initial-config`.

## `rust-v0.31.0`-`rust-v0.40.0`

### Startup and tool-call timeouts (`#3182`, `#3959`)

- Коммиты: `6efb52e545` (`feat(mcp): per-server startup timeout (#3182)`), `19f46439ae` (`timeouts for mcp tool calls (#3959)`).
- Developer info: separates startup timeout from tool-call timeout so slow initialization and long-running tool execution can be controlled independently.
- Why: path evidence touches MCP config/client code and stable containment maps the commits to `rust-v0.31.0` and `rust-v0.40.0`.
- User info: slow MCP servers fail with bounded startup behavior; individual tools also have bounded execution time.
- Compatibility/risks: timeout fields are config contract; future changes must update schema/docs and keep app-server/status diagnostics aligned.
- Evidence: `EVID-031-040-timeouts`.

## `rust-v0.42.0`-`rust-v0.48.0`

### RMCP, streamable HTTP, OAuth, resources and filters (`#4252`, `#4317`, `#4517`, `#5239`, `#5367`, `#5529`)

- Коммиты: `e555a36c6a` (`[MCP] Introduce an experimental official rust sdk based mcp client (#4252)`), `3a1be084f9` (`[MCP] Add experimental support for streamable HTTP MCP servers (#4317)`), `1d17ca1fa3` (`[MCP] Add support for MCP Oauth credentials (#4517)`), `40fba1bb4c` (`[MCP] Add support for resources (#5239)`), `740b4a95f4` (`[MCP] Add configuration options to enable or disable specific tools (#5367)`), `4cd6b01494` (`[MCP] Remove the legacy stdio client in favor of rmcp (#5529)`).
- Developer info: external MCP moves from the initial custom client toward RMCP-backed stdio and streamable HTTP, adds OAuth credential handling, resources/resource templates, and per-server tool allow/deny controls.
- Why: stable containment places these commits between `rust-v0.42.0` and `rust-v0.48.0`, and diffs touch `rmcp-client`, config parsing and MCP resource/tool paths.
- User info: Codex can connect to local stdio and HTTP MCP servers, log in to OAuth-backed servers, list/read MCP resources and hide specific server tools.
- Compatibility/risks: tool names, OAuth state and resource shapes become durable surfaces; hidden/disabled tools must be removed before model exposure, not only filtered in UI.
- Evidence: `EVID-042-048-rmcp-http-oauth-resources`.

## `rust-v0.59.0`-`rust-v0.67.0`

### Non-blocking startup, elicitations and in-session login (`#6334`, `#6947`, `#7751`)

- Коммиты: `03ffe4d595` (`core/tui: non-blocking MCP startup (#6334)`), `7561a6aaf0` (`support MCP elicitations (#6947)`), `893f5261eb` (`feat: support mcp in-session login (#7751)`).
- Developer info: MCP startup becomes non-blocking in the TUI/core path, MCP server elicitations enter the interaction model, and login can happen during an existing session.
- Why: commits are stable-contained in `rust-v0.59.0`, `rust-v0.64.0` and `rust-v0.67.0`; diffs touch core/TUI startup and MCP auth/elicitation flow.
- User info: Codex no longer freezes user work while MCP servers boot; servers can ask the client/user for required input and OAuth can be initiated without restarting Codex.
- Compatibility/risks: startup state is user-visible and asynchronous; elicitations must preserve turn/request correlation and avoid silently auto-accepting privileged input.
- Evidence: `EVID-059-067-startup-elicitation-login`.

## `rust-v0.81.0`-`rust-v0.99.0`

### Requirements, approvals and stdio cleanup (`#9101`, `#10200`, `#10710`)

- Коммиты: `2651980bdf` (`Restrict MCP servers from requirements.toml (#9101)`), `34f89b12d0` (`MCP tool call approval (simplified version) (#10200)`), `82c981cafc` (`Process-group cleanup for stdio MCP servers to prevent orphan process storms (#10710)`).
- Developer info: requirements layers can disable MCP servers, MCP tool calls get explicit approval handling, and stdio processes are cleaned up as process groups.
- Why: evidence commands confirm first containing stable tags `rust-v0.81.0`, `rust-v0.93.0` and `rust-v0.99.0`; current config still carries `disabled_reason`, approval modes and stdio runtime cleanup tests.
- User info: managed/project requirements can restrict available external servers; destructive or untrusted MCP tools can prompt for approval; stopped sessions should not leave process storms behind.
- Compatibility/risks: requirements must disable by source-of-truth config/catalog rather than by UI hiding; stdio teardown is part of the runtime safety contract.
- Evidence: `EVID-081-099-requirements-approvals-cleanup`.

## `rust-v0.111.0`-`rust-v0.119.0`

### App-server elicitations, `codex-mcp` extraction, Apps MCP and deferred custom tools (`#13425`, `#15919`, `#16082`, `#16944`, `#17043`)

- Коммиты: `926b2f19e8` (`feat(app-server): support mcp elicitations in v2 api (#13425)`), `59b68f5519` (`Extract MCP into codex-mcp crate (#15919)`), `5fe9ef06ce` (`[mcp] Support MCP Apps part 1. (#16082)`), `d7f99b0fa6` (`[mcp] Expand tool search to custom MCPs. (#16944)`), `7b6486a145` (`[mcp] Support server-driven elicitations (#17043)`).
- Developer info: app-server gains MCP elicitation protocol, MCP runtime is extracted into `codex-mcp`, host-owned Apps MCP starts integrating with the same runtime, and custom MCP tools become discoverable through tool search/deferred exposure.
- Why: stable containment maps these to `rust-v0.111.0` and `rust-v0.119.0`; current HEAD still has `codex-rs/codex-mcp`, app-server MCP elicitation types and deferred MCP exposure logic.
- User info: app clients can render/respond to MCP elicitations; large MCP tool inventories can stay out of the model context until discovered; Apps and custom MCP servers share more runtime plumbing.
- Compatibility/risks: app-server and TUI must use typed elicitation requests, and deferred tools still need exact routing back to the original MCP server/tool names.
- Evidence: `EVID-111-119-appserver-codex-mcp-apps`.

## `rust-v0.122.0`-`rust-v0.124.0`

### Deferred code-mode tools, environments, threadless resources, hooks and permission profiles (`#17287`, `#18085`, `#18212`, `#18292`, `#18385`, `#18286`)

- Коммиты: `9c6d038622` (`[code mode] defer mcp tools from exec description (#17287)`), `b4be3617f9` (`[1/8] Add MCP server environment config (#18085)`), `996aa23e4c` (`[5/6] Wire executor-backed MCP stdio (#18212)`), `1132ef887c` (`Make MCP resource read threadless (#18292)`), `305825abd9` (`Support MCP tools in hooks (#18385)`), `ff22982d75` (`mcp: include permission profiles in sandbox state (#18286)`).
- Developer info: MCP tools can be deferred from code-mode descriptions, MCP servers gain environment placement, stdio can be launched through executor environments, app-server can read resources without a loaded thread, hooks can observe MCP tools, and sandbox state includes permission profiles.
- Why: stable tags place these commits in `rust-v0.122.0` through `rust-v0.124.0`; current source paths show `McpRuntimeContext`, threadless app-server resource reads, hook payload support and `SandboxState.permission_profile`.
- User info: external servers can be tied to execution environments, resources can be inspected from app-server status/resource APIs, and MCP tool/hook/permission behavior aligns better with active session policy.
- Compatibility/risks: remote stdio requires explicit absolute `cwd`; threadless resource reads lack turn-selected environment and must use config-level fallback rules only.
- Evidence: `EVID-122-124-environments-resources-hooks`.

## `rust-v0.126.0`-`rust-v0.135.0`

### External import, turn items, built-in runtime servers, explicit environments and naming manager (`#19949`, `#20677`, `#21356`, `#23583`, `#21576`)

- Коммиты: `cb8b1bbcd6` (`Support detect and import MCP, Subagents, hooks, commands from external (#19949)`), `c8c30d9d75` (`[codex] Emit MCP tool calls as turn items (#20677)`), `b2268999fe` (`feat: make built-in MCPs first-class runtime servers (#21356)`), `298e5cfce1` (`Route MCP servers through explicit environments (#23583)`), `ff7513cd83` (`Move MCP tool naming mode into manager (#21576)`).
- Developer info: external-agent migration can import MCP configs, MCP tool calls become protocol/turn items, built-in MCPs use the same runtime server path, server environment routing becomes explicit, and naming/prefix behavior moves into the manager.
- Why: stable containment spans `rust-v0.126.0` through `rust-v0.135.0`; current architecture has `McpConfig.mcp_server_catalog`, protocol MCP tool items and manager-owned `prefix_mcp_tool_names`.
- User info: imported external agent configs can bring MCP declarations into Codex, clients can render MCP calls as first-class history items, and built-in/custom/plugin MCPs converge on common runtime behavior.
- Compatibility/risks: migration must reject mixed/ambiguous transport configs; clients should render MCP turn items rather than scraping transcript text; tool naming compatibility must remain manager-owned.
- Evidence: `EVID-126-135-import-items-runtime`.

## `rust-v0.140.0`-`rust-v0.141.0`

### Catalog, thread-scoped contributions and selected executor plugin MCPs (`#27634`, `#27670`, `#27870`, `#27893`)

- Коммиты: `4a5a676499` (`Resolve MCP server registrations through a catalog (#27634)`), `693082f3c4` (`Make MCP server contributions thread-scoped (#27670)`), `b3c423e475` (`Discover stdio MCP servers from selected executor plugins (#27870)`), `c8c78b63a7` (`Activate selected executor plugin MCPs in app-server (#27893)`).
- Developer info: MCP declarations resolve through a catalog with source/precedence, extension contributions can be thread-scoped, selected executor plugins can contribute stdio MCP declarations, and app-server activates those only for selected thread capability roots.
- Why: commits are stable-contained in `rust-v0.140.0` and `rust-v0.141.0`; current source paths include `codex-mcp/src/catalog.rs`, `ext/extension-api/src/contributors/mcp.rs`, `ext/mcp/src/executor_plugin.rs` and app-server selected capability root wiring.
- User info: plugin/executor MCP tools can be available only to the thread that selected that plugin/root; unrelated threads do not inherit those tools globally.
- Compatibility/risks: selected plugin MCPs must not be registered globally; precedence and disabled registrations must be resolved in the catalog before connection startup.
- Evidence: `EVID-140-141-catalog-selected-plugins`.

## Current branch notes

- Current branch checked during this research: `fork/141`.
- `git log rust-v0.141.0..HEAD --grep='MCP|mcp'` produced no current-branch commits, so this package records no post-stable MCP branch notes.
