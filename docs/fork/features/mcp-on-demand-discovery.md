# MCP on-demand discovery

## Feature passport

- Code name: `mcp-on-demand-discovery`
- Status: историческая fork-возможность.
- Goal: дать Codex возможность обнаруживать MCP servers по требованию в процессе работы.
- Scope in: runtime discovery behavior, docs, related tests/fixtures.
- Scope out: общий MCP connection manager и marketplace/plugin discovery.

## Как работает для пользователя

MCP servers обнаруживаются по требованию, а не нагружают контекст agent заранее. Польза: MCP servers не занимают model-visible контекст, если они не нужны agent для текущей задачи.

## Branches and commits

| Branch | Baseline | Commit | Date | Subject | Роль |
| --- | --- | --- | --- | --- | --- |
| `fork/106` | `rust-v0.106.0` | `d8bfecf5dd` | 2026-03-03 | `feat(mcp): add on-demand server discovery and document behavior` | Основная реализация и документация. |
| `fork/107` | `rust-v0.107.0` | `67a21f793a` | 2026-03-03 | `feat(mcp): add on-demand server discovery and document behavior` | Перенос на 0.107. |
| `fork/107` | `rust-v0.107.0` | `e46a9da9a0` | 2026-03-03 | `fix: align MCP test fixtures with rust-v0.107.0 fields` | Адаптация test fixtures к baseline 0.107. |

## Implementation notes

Diff evidence указывает на MCP fixtures/tests и docs. При дальнейшем анализе нужно смотреть конкретный discovery path в current MCP/tool registry.

## Native coverage in rust-v0.140.0

Status: `partial`. Native release имеет deferred MCP/tool surfaces: `tool_search`, `list_available_plugins_to_install`, `request_plugin_install`, `ToolExposure::Deferred`, `ToolRouterParams.deferred_mcp_tools`, app-server `mcp_server_refresh`, `Op::RefreshMcpServers` and plugin-change best-effort MCP refresh. Не хватает универсального model-callable discovery/connect path для произвольного MCP server по historical fork contract.

## Porting/current-state notes

Современный upstream имеет более развитые plugin/MCP surfaces. Перед переносом нужно проверить, не покрывает ли native plugin/MCP catalog эту задачу лучше.
