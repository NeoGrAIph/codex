# MCP on-demand discovery

## Feature passport

- Code name: `mcp-on-demand-discovery`
- Status: первая итерация переноса на `fork/140` реализована и локально проверена.
- Goal: дать Codex model-visible путь для обнаружения MCP servers по требованию без предварительной загрузки всех MCP tools в основной контекст.
- Scope in: tool registry wiring, read-only server discovery tool, deferred/direct MCP tool visibility, docs, focused tests.
- Scope out: запуск новых MCP servers из tool call, plugin marketplace discovery/install flow, app-server protocol/schema changes, MCP startup/refresh lifecycle changes.

## Как работает для пользователя

Codex получает tool `list_mcp_servers`, когда в текущей сессии есть configured MCP servers. Tool возвращает имена server, origin/plugin metadata и, по `include_tools=true`, bounded список model-visible MCP tool names: canonical `namespace.name` плюс исходный `raw_name` там, где он нужен для диагностики. Tool summaries capped per server: default `max_tools_per_server=50`, hard cap `200`, а output содержит `tools_total` и `tools_truncated`. Для поиска конкретного deferred MCP tool agent использует native `tool_search`; сами MCP tool calls продолжают идти через `McpHandler`.

Польза: agent может сначала увидеть доступные MCP servers и только затем искать или вызывать нужные MCP tools, не требуя hardcoded tool lists и не расширяя основной контекст всеми deferred tools.

## Branches and commits

| Branch | Baseline | Commit | Date | Subject | Роль |
| --- | --- | --- | --- | --- | --- |
| `fork/106` | `rust-v0.106.0` | `d8bfecf5dd` | 2026-03-03 | `feat(mcp): add on-demand server discovery and document behavior` | Основная реализация и документация. |
| `fork/107` | `rust-v0.107.0` | `67a21f793a` | 2026-03-03 | `feat(mcp): add on-demand server discovery and document behavior` | Перенос на 0.107. |
| `fork/107` | `rust-v0.107.0` | `e46a9da9a0` | 2026-03-03 | `fix: align MCP test fixtures with rust-v0.107.0 fields` | Адаптация test fixtures к baseline 0.107. |

## Implementation notes

Diff evidence указывает на MCP fixtures/tests и docs. При дальнейшем анализе нужно смотреть конкретный discovery path в current MCP/tool registry.

## Native coverage in rust-v0.140.0

Status: `implemented-first-iteration`. Native release имеет deferred MCP/tool surfaces: `tool_search`, `list_available_plugins_to_install`, `request_plugin_install`, `ToolExposure::Deferred`, `ToolRouterParams.deferred_mcp_tools`, app-server `mcp_server_refresh`, `Op::RefreshMcpServers` and plugin-change best-effort MCP refresh. Fork iteration добавляет model-callable `list_mcp_servers` поверх текущего `McpConnectionManager`.

## Porting/current-state notes

Современный upstream имеет более развитые plugin/MCP surfaces, поэтому перенос сделан как additive discovery поверх native registry/manager path. `list_mcp_servers` не создаёт MCP servers, не перезапускает manager и не держит parallel cache; он читает already configured/running manager state. App-server refresh и plugin-change refresh остаются upstream behavior.

## Integration and compatibility

- Native source of truth: `codex-rs/core/src/tools/spec_plan.rs`, `ToolRegistry`, `ToolExposure`.
- MCP owner: `codex-rs/codex-mcp/src/connection_manager.rs`.
- Producers: `Session::built_tools` supplies direct/deferred MCP tool lists; `spec_plan` wires discovery alongside MCP resource/direct/deferred tools.
- Consumers: model-visible `list_mcp_servers`, existing `tool_search`, existing MCP tool handlers.
- Permission/security: discovery is read-only and does not widen sandbox, network, or tool-call permissions; optional tool lists are bounded per server to avoid unbounded model-visible output.
- Persistence/resume: no persisted state or rollout format changes.
- Intentionally unaffected: app-server protocol/schema, MCP config loading, plugin install flow, MCP startup/refresh, MCP tool execution semantics.

## Verification matrix

- `cargo check -p codex-core`: compile handler/spec wiring and cross-crate manager method.
- `just test -p codex-core`: focused core planning/handler tests.
- `just test -p codex-mcp`: focused manager crate compatibility.
- `git diff --check`: whitespace guard.

## Doc changelog

- 2026-06-18: Актуализирован verification evidence для `fork/140`: `cargo check -p codex-core`, focused `codex-core` MCP/run_skill pass 15/15 и `just test -p codex-mcp` 82/82.
- 2026-06-17: Зафиксирована первая `fork/140` итерация: `list_mcp_servers` через native tool registry и read-only `McpConnectionManager` metadata with bounded optional tool summaries.
