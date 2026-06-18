# fork/140 cross-feature acceptance

Этот project dossier фиксирует финальный acceptance pass для `fork/140` после последовательного слияния feature slices. Это не новая пользовательская feature, а проверочный слой, который связывает уже реализованные fork-возможности и доказывает, что они проходят через native source of truth без параллельных registry/config/runtime paths.

## Scope

- Проверить цепочки между `agent-role-templates`, `agent-role-tool-selection`, `provider-aware-models-deepseek`, `mcp-on-demand-discovery`, `run-skill-script` и sub-agent runtime/workbench docs.
- Зафиксировать, какие native surfaces должны наследовать новые возможности: `ConfigToml`/`Config`, role template TOML loader, `spec_plan`, `ToolRegistry`, `McpConnectionManager`, app-server protocol, TUI selection flows and snapshots.
- Не добавлять новую runtime-функциональность, protocol values, migrations или user config fields.

## Canonical links

- Feature ledger: `docs/fork/features/README.md`.
- Release research backlog: `docs/fork/research/0.140.0/openclaude-agent-ux-runtime-backlog.md`.
- MCP/tools research: `docs/fork/research/0.140.0/tools-mcp-skills.md`.
- Provider/model project: `docs/fork/projects/provider-aware-models-deepseek/`.
- Role template project: `docs/fork/projects/agent-role-templates/`.
- Role tool-selection project: `docs/fork/projects/agent-role-tool-selection/`.
- MCP discovery project: `docs/fork/projects/mcp-on-demand-discovery/`.

## Current status

Status: completed locally on `feature/140/cross-feature-acceptance`.

Current acceptance focus is intentionally narrow: reuse representative focused tests that cover cross-feature propagation, no pending TUI snapshots, no whitespace errors, and no new High/Medium audit findings.
