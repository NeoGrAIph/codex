# Evidence: external MCP servers

Файл фиксирует проверочные команды и решения triage для исследования внешних MCP servers. Основной язык - русский; commit subjects, PR ids, API/type/tool names сохраняются как в upstream.

## Общий scope и refs

- Scope in: Codex как MCP client/runtime для внешних MCP servers, включая `[mcp_servers]`, stdio/streamable HTTP, OAuth/auth, resources/templates, tool exposure, approvals/elicitations, app-server/TUI projections, plugins/apps/extensions and environment routing.
- Scope out: `codex-rs/mcp-server` как Codex-as-MCP-server binary/API. Коммиты вроде `21cd953dbd` (`feat: introduce mcp-server crate (#792)`), `2b72d05c5e` (`feat: make Codex available as a tool when running it as an MCP server (#811)`) и `497c5396c0` (`feat: add mcp subcommand to CLI to run Codex as an MCP server (#934)`) намеренно не включены в основной timeline.
- Начальный external-MCP commit: `147a940449839b116b220b7e7d016d2a2890c134`.
- Первый stable tag, содержащий начальный commit: `rust-v0.2.0`.
- Верхняя stable граница локального исследования: `rust-v0.141.0`.

## Базовые команды проверки

```bash
git show --quiet --format='%H%x09%cI%x09%s' <sha>
git show --stat --name-only --oneline <sha>
git show --no-color --patch <sha> -- <selected-paths>
git tag --contains <sha> | rg '^rust-v0\.[0-9]+\.0$' | sort -V | head -1
git log rust-v0.141.0 --reverse --date=short --format='%h %cd %s' --grep='MCP' --grep='mcp' --all-match
git log rust-v0.141.0 --reverse --date=short --format='%h %cd %s' -- <mcp-paths>
git log rust-v0.141.0..HEAD --reverse --date=short --format='%h %cd %s' --grep='MCP' --grep='mcp' --extended-regexp
```

Release range проверен командами:

```bash
git tag --contains 147a940449839b116b220b7e7d016d2a2890c134 | rg '^rust-v0\.[0-9]+\.0$' | sort -V | head -10
git tag -l 'rust-v0.*.0' | rg '^rust-v0\.[0-9]+\.0$' | sort -V | tail -10
```

Набор source paths для targeted history scan. Включены исторические пути ранней реализации, даже если они уже удалены или перенесены в current HEAD; current source-of-truth отдельно указан в `architecture.md`.

```bash
git log rust-v0.141.0 --reverse --date=short --format='%h %cd %s' -- \
  codex-rs/codex-mcp \
  codex-rs/rmcp-client \
  codex-rs/core/src/mcp.rs \
  codex-rs/core/src/mcp_tool_call.rs \
  codex-rs/core/src/mcp_connection_manager.rs \
  codex-rs/core/src/mcp_server_config.rs \
  codex-rs/core/src/tools/handlers/mcp.rs \
  codex-rs/core/src/tools/handlers/mcp_resource.rs \
  codex-rs/protocol/src/mcp.rs \
  codex-rs/config/src/mcp_types.rs \
  codex-rs/app-server/src/request_processors/mcp_processor.rs \
  codex-rs/tui/src/chatwidget/mcp_startup.rs \
  codex-rs/tui/src/history_cell/mcp.rs \
  codex-rs/ext/extension-api/src/contributors/mcp.rs \
  codex-rs/ext/mcp/src
```

## Release triage evidence

### `EVID-002-initial-config`

- Included commits: `147a940449839b116b220b7e7d016d2a2890c134`, `27198bfe11`.
- First containing stable tag: `rust-v0.2.0` for both entries.
- Commands: `git show --quiet --format='%H %cI %s' 147a940449839b116b220b7e7d016d2a2890c134`, `git show --stat --name-only --oneline 147a940449839b116b220b7e7d016d2a2890c134`, `git tag --contains 147a940449839b116b220b7e7d016d2a2890c134 | rg '^rust-v0\.[0-9]+\.0$' | sort -V | head -1`.
- Evidence notes: commit body for `#829` names `mcp_connection_manager.rs` and `mcp_tool_call.rs` as primary implementation and states that all configured MCP servers were initially launched on each Codex run.

### `EVID-031-040-timeouts`

- Included commits: `6efb52e545` (`rust-v0.31.0`), `19f46439ae` (`rust-v0.40.0`).
- Commands: `git show --stat --name-only --oneline 6efb52e545 19f46439ae`, plus stable containment via `git tag --contains`.
- Evidence notes: subjects and path diffs identify separate startup and tool-call timeout work for external MCP server lifecycle.

### `EVID-042-048-rmcp-http-oauth-resources`

- Included commits: `e555a36c6a` (`rust-v0.42.0`), `3a1be084f9` (`rust-v0.43.0`), `1d17ca1fa3` (`rust-v0.45.0`), `40fba1bb4c` (`rust-v0.47.0`), `740b4a95f4` (`rust-v0.48.0`), `4cd6b01494` (`rust-v0.48.0`).
- Commands: `git show --stat --name-only --oneline <sha>` for each commit and targeted patches under `codex-rs/rmcp-client`, `codex-rs/config/src/mcp_types.rs`, `codex-rs/core/src/mcp_tool_call.rs`, and MCP resource handlers where present.
- Evidence notes: this group establishes the transition from custom stdio-only behavior to RMCP-backed stdio/HTTP, OAuth credentials, resource listing/read and tool allow/deny configuration.

### `EVID-059-067-startup-elicitation-login`

- Included commits: `03ffe4d595` (`rust-v0.59.0`), `7561a6aaf0` (`rust-v0.64.0`), `893f5261eb` (`rust-v0.67.0`).
- Commands: `git show --stat --name-only --oneline 03ffe4d595 7561a6aaf0 893f5261eb`, plus stable containment checks.
- Evidence notes: diffs show startup status moving out of blocking startup, MCP elicitation support entering client/runtime paths, and login becoming available during an active session.

### `EVID-081-099-requirements-approvals-cleanup`

- Included commits: `2651980bdf` (`rust-v0.81.0`), `34f89b12d0` (`rust-v0.93.0`), `82c981cafc` (`rust-v0.99.0`).
- Commands: `git show --stat --name-only --oneline 2651980bdf 34f89b12d0 82c981cafc`.
- Evidence notes: current config still exposes `McpServerDisabledReason`, per-server/per-tool approval modes and stdio process cleanup tests; this group is security/runtime boundary evidence rather than UI-only evidence.

### `EVID-111-119-appserver-codex-mcp-apps`

- Included commits: `926b2f19e8` (`rust-v0.111.0`), `59b68f5519` (`rust-v0.119.0`), `5fe9ef06ce` (`rust-v0.119.0`), `d7f99b0fa6` (`rust-v0.119.0`), `7b6486a145` (`rust-v0.119.0`).
- Commands: `git show --stat --name-only --oneline 926b2f19e8 59b68f5519 5fe9ef06ce d7f99b0fa6 7b6486a145`.
- Evidence notes: current HEAD confirms this as durable architecture: `codex-rs/codex-mcp`, app-server v2 MCP elicitation types, Apps MCP support and `McpToolExposure` deferred tool behavior remain present.

### `EVID-122-124-environments-resources-hooks`

- Included commits: `9c6d038622` (`rust-v0.122.0`), `b4be3617f9` (`rust-v0.122.0`), `996aa23e4c` (`rust-v0.122.0`), `1132ef887c` (`rust-v0.123.0`), `305825abd9` (`rust-v0.124.0`), `ff22982d75` (`rust-v0.124.0`).
- Commands: `git show --stat --name-only --oneline <sha>` and targeted source inspection of `codex-rs/codex-mcp/src/runtime.rs`, `codex-rs/app-server/src/request_processors/mcp_processor.rs`, `codex-rs/ext/extension-api/src/contributors/mcp.rs`, `codex-rs/core/src/tools/handlers/mcp.rs`.
- Evidence notes: current `McpRuntimeContext` validates local/remote environment placement; app-server threadless resource reads explicitly document their fallback rules; hooks and sandbox state include MCP-specific metadata.

### `EVID-126-135-import-items-runtime`

- Included commits: `cb8b1bbcd6` (`rust-v0.126.0`), `c8c30d9d75` (`rust-v0.129.0`), `b2268999fe` (`rust-v0.130.0`), `298e5cfce1` (`rust-v0.134.0`), `ff7513cd83` (`rust-v0.135.0`).
- Commands: `git show --stat --name-only --oneline cb8b1bbcd6 c8c30d9d75 b2268999fe 298e5cfce1 ff7513cd83`.
- Evidence notes: external-agent import includes MCP config detection/import; MCP tool calls become first-class turn items; built-in MCPs and naming mode move into common runtime/manager paths.

### `EVID-140-141-catalog-selected-plugins`

- Included commits: `4a5a676499` (`rust-v0.140.0`), `693082f3c4` (`rust-v0.140.0`), `b3c423e475` (`rust-v0.141.0`), `c8c78b63a7` (`rust-v0.141.0`).
- Commands: `git show --stat --name-only --oneline 4a5a676499 693082f3c4 b3c423e475 c8c78b63a7`, `rg -n "McpServerRegistration|McpServerContribution|selected plugin|executor plugin" codex-rs/codex-mcp/src codex-rs/ext codex-rs/app-server/src`.
- Evidence notes: current source confirms catalog precedence, thread-scoped contribution APIs and selected executor plugin MCP activation in app-server/runtime paths.

## Current HEAD source inspection

- Config contract: `codex-rs/config/src/mcp_types.rs` defines `McpServerConfig`, transport variants, environment id, `enabled`, `required`, `supports_parallel_tool_calls`, startup/tool timeouts, approval defaults, tool allow/deny, scopes, OAuth client config and OAuth resource.
- Runtime/catalog: `codex-rs/codex-mcp/src/catalog.rs`, `server.rs`, `mcp/mod.rs`, `runtime.rs`, `connection_manager.rs` own registration precedence, effective servers, runtime environment resolution and running clients.
- Tool/resource exposure: `codex-rs/core/src/tools/handlers/mcp.rs`, `codex-rs/core/src/mcp_tool_call.rs`, `codex-rs/core/src/mcp_tool_exposure.rs`, `codex-rs/core/src/tools/handlers/mcp_resource.rs`.
- App-server API: `codex-rs/app-server-protocol/src/protocol/v2/mcp.rs`, `codex-rs/app-server/src/request_processors/mcp_processor.rs`, `codex-rs/app-server/src/mcp_refresh.rs`.
- TUI and UI surfaces: `codex-rs/tui/src/chatwidget/mcp_startup.rs`, `codex-rs/tui/src/history_cell/mcp.rs`, `codex-rs/tui/src/bottom_pane/mcp_server_elicitation.rs`.
- Extension/plugin surfaces: `codex-rs/ext/extension-api/src/contributors/mcp.rs`, `codex-rs/ext/mcp/src/executor_plugin.rs`, `codex-rs/codex-mcp/src/plugin_config.rs`.

## Evidence gaps

- This pass did not inspect GitHub PR pages or external MCP specification pages; rationale is based on local commit subjects/bodies/diffs and current source.
- Compact timeline intentionally omits stable releases without material external-MCP changes; release coverage is provided through the command log and stable containment checks rather than no-change sections.
