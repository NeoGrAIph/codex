# fork/140 cross-feature acceptance

Этот project dossier фиксирует финальный acceptance pass для `fork/140` после последовательного слияния feature slices. Это не новая пользовательская feature, а проверочный слой, который связывает уже реализованные fork-возможности и доказывает, что они проходят через native source of truth без параллельных registry/config/runtime paths.

## Scope

- Проверить цепочки между `subagent-workbench`, `agent-role-templates`, `agent-role-tool-selection`, `subagent-policy-and-ownership`, `provider-aware-models-deepseek`, `mcp-on-demand-discovery` и `run-skill-script`.
- Зафиксировать, какие native surfaces наследуют новые возможности: `ConfigToml`/`Config`, native role TOML loader, `SessionSource::SubAgent(ThreadSpawn)`, `spec_plan`, `ToolRegistry`, `McpConnectionManager`, app-server protocol/schema, thread-store/rollout metadata, TUI selection flows and snapshots.
- Не добавлять новую runtime-функциональность, protocol values, migrations или user config fields в этом acceptance slice.

## Canonical links

- Feature ledger: `docs/fork/features/README.md`.
- Release research backlog: `docs/fork/research/0.140.0/openclaude-agent-ux-runtime-backlog.md`.
- MCP/tools research: `docs/fork/research/0.140.0/tools-mcp-skills.md`.
- Provider/model project: `docs/fork/projects/provider-aware-models-deepseek/`.
- Role template project: `docs/fork/projects/agent-role-templates/`.
- Role tool-selection project: `docs/fork/projects/agent-role-tool-selection/`.
- MCP discovery project: `docs/fork/projects/mcp-on-demand-discovery/`.

## Current status

Status: active acceptance package on `fork/140`; docs-only reconciliation slice после реализации нескольких production-ready контрактов.

Current acceptance focus is intentionally narrow: собрать доказуемую cross-feature матрицу поверх уже реализованных native contracts, убрать устаревшие gap statements и связать existing focused tests с production-ready требованиями. Fresh representative focused runs, generated artifact review, `cargo insta pending-snapshots --workspace-root .`, `git diff --check` and current docs/schema High/Medium re-audit closure are recorded in `verification.md`; this package still is not a claim that the whole production-ready goal is complete while remaining product gaps and release-level manual smoke are open.

## Current acceptance groups

| Group | Native owners | Acceptance signal |
| --- | --- | --- |
| Role policy + Workbench actions | `$CODEX_HOME/agents/*.toml`, `[subagent_action_policy]`, `SubAgentActionPolicySnapshot`, `ThreadManager`, app-server `agent/*`, TUI Agent Window action rows | Workbench message/follow-up/close/dismiss/retry/stop-all are denied or allowed by server-side ownership/read-only/action policy, with TUI as projection only. |
| Role tool selection + MCP | Native `[tool_selection]`, `spec_plan`, `ToolRegistry`, `McpConnectionManager`, app-server direct MCP call path | Allowed/denied tools affect model-visible specs, deferred `tool_search`, Code Mode synthetic tools, `ToolRegistry` dispatch and direct app-server MCP execution. |
| Role templates + Import | Built-ins, native role TOML parser/loader, `$CODEX_HOME/agents/*.toml`, external-agent migration import | `/agent-roles` authors native TOML, imports markdown/frontmatter as compile-to-native TOML, reloads config through existing app-server path and exposes `reviewer` as a bundled config-backed role. |
| Workbench lifecycle + Persistence | `SessionSource::SubAgent(ThreadSpawn)`, `initial_task`, `tool_selection`, `action_policy`, rollout `SessionMeta.agent_hidden`, thread-store metadata, app-server `Thread` projection | `dismiss` is non-destructive visibility state, `retry` uses durable initial task and preserved effective policy/config, `stop all` closes live descendants only and reports partial failures. |
| Provider/model + Agent runtime | Provider-aware catalog/config, `ThreadSessionState`, app-server model/provider methods, Chat Completions transport | Provider/model state remains native and is not encoded into role/profile/workbench-local state; workbench only hydrates bounded read-only model/provider detail when native session/thread metadata proves it. |

## Current remaining gaps

- MCP allow/deny management is intentionally not a separate MCP-specific surface. Current management uses the generic native role `[tool_selection]` contract: `/agent-roles` catalog-assisted allow/deny authoring, `agentRole/toolSelection/set` whole-policy writes for discovered user role TOML files, `spec_plan` / `ToolRegistry` enforcement and direct app-server MCP call checks all consume the same canonical tool ids. A dedicated MCP-only UI/API would be a parallel policy path unless a future feature proves a native MCP source-of-truth need.
- Role-template authoring now includes native TOML draft creation, inline existing-role TOML editor, current-model seed/update, current model/provider-only update, current-reasoning update, catalog-backed model/provider/reasoning picker controls, current-tools seed, discovered user-role allowed/denied tool editing and import entry. Runtime markdown/frontmatter loader remains intentionally out of scope.
- Stop-all partial failure aggregation and TUI-visible partial result copy are covered; broader real shutdown-failure injection remains lower-priority resilience coverage.
- Final cross-feature acceptance still needs manual smoke where appropriate. Current focused representative commands, generated artifact review and docs/schema High/Medium re-audit closure are recorded in `verification.md`.
