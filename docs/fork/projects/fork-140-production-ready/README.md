# Fork/140 production-ready feature pass

Этот пакет фиксирует текущий production-ready проход по связанным feature-группам `fork/140`: Subagent Workbench / Agent Window, Agent role templates, Agent role tool selection, Subagent policy and ownership. Цель прохода не в том, чтобы добавить отдельные кнопки, а в том, чтобы довести deferred surfaces до native Codex contracts с source of truth, runtime enforcement, persistence/resume, protocol/schema, TUI, tests and audits.

## Current baseline

- Branch: `fork/140`.
- Starting commit: `03325ffcb3 fix(fork): harden feature integration findings`.
- Starting status: clean worktree at goal activation.
- Relevant existing feature docs:
  - `docs/fork/features/subagent-workbench.md`
  - `docs/fork/features/agent-role-templates.md`
  - `docs/fork/features/agent-role-tool-selection.md`
  - `docs/fork/features/subagent-policy-and-ownership.md`
- Relevant existing project docs:
  - `docs/fork/projects/subagent-workbench/`
  - `docs/fork/projects/agent-role-templates/`
  - `docs/fork/projects/agent-role-tool-selection/`
  - `docs/fork/projects/subagent-policy-and-ownership/`
- Current research coordination surface: `docs/fork/research/0.140.0/openclaude-agent-ux-runtime-backlog.md`.

## Production-ready slices

| Slice | Status | Native owner / evidence surface | Why first/next |
| --- | --- | --- | --- |
| PR-001 Policy enforcement substrate | Partial, implemented through current slices | `SessionSource`/`SubAgentActionPolicySnapshot`, `ThreadManager`, `AgentControl`, `spec_plan`, MCP exposure, permission/sandbox checks | Workbench action allow/deny, `read_only`, trust-gated roles, read-only `.agents` and `agentRole/actionPolicy/set` are implemented over native runtime/config paths; MCP allow/deny is covered by generic role `[tool_selection]` management rather than a separate MCP-only policy surface. |
| PR-002 Role tool policy contract v2 | Partial, implemented for current contract | `ConfigToml`/`Config.agent_roles`, role loader, `spec_plan`, `ToolRegistry`, app-server protocol/schema | Native allow/deny, direct app-server MCP execution enforcement, whole-policy `agentRole/toolSelection/set` and path-backed retention through `ThreadSpawn.tool_selection` are implemented; remaining limits are external `config_file` write ownership and live mutation of already materialized in-flight child role layers. |
| PR-003 Role template import/authoring v2 | Partial, implemented for current import/authoring contract | Native TOML role files and parser; markdown/frontmatter/persona import compiles to native role config | `/agent-roles` exposes the existing `/import` flow, imported markdown/frontmatter agents compile to native TOML, inline existing user-role TOML editing and current-model/current model/provider-only/current-reasoning/current-tools staged create/update use the same parser/write/reload path, catalog-backed model/provider/reasoning picker controls seed selected `ModelPreset` defaults into editable native TOML drafts, discovered user roles can set or clear current model defaults, update only current model/provider defaults, update only current reasoning/service-tier defaults, plus edit allowed and denied tools through TOML drafts, and built-in `reviewer` is a normal bundled role with native read-only defaults. Runtime markdown loading remains intentionally out of scope. |
| PR-004 Workbench visibility and lifecycle v2 | Partial, implemented for current lifecycle contract | `AgentNavigationState`, `ThreadManager`, `AgentControl`, thread metadata/session source, app-server `agent/*` methods | Connect/switch copy, Ctrl+T transcript-to-Agent-Window parity, non-destructive `dismiss`, native fresh-sibling `retry`, scoped destructive `stop all` and read-only role/profile policy hydration from `ThreadSpawn` snapshots are implemented over native picker/thread metadata/app-server paths. Remaining work is final acceptance and any later UX polish that does not create a parallel runtime state. |
| PR-005 Cross-feature acceptance | Active acceptance evidence recorded; current docs/schema High/Medium findings closed | Combined native projections: policy + tools + roles + workbench + resume/restart | Final proof that new behavior propagates through existing Codex surfaces without breaking upstream paths. |

## Current non-goals unless proven necessary

- No new database migration unless existing thread/session metadata, config files or rollout/session structures cannot safely represent the state.
- No separate OpenClaude-style profile registry.
- No UI-only mutating state for workbench lifecycle.
- No silent fallback for policy, role import or tool-selection errors.
- No broad keybinding takeover for first-press `Ctrl+T`; native transcript behavior stays first, and Agent Window parity uses repeated `Ctrl+T` from the transcript overlay.
- No runtime markdown/frontmatter role loader; markdown/frontmatter remains import-only and compiles to native TOML roles.

## Audit evidence used in this pass

The current pass uses read-only sub-agents for independent current-state evidence and docs/code consistency checks:

- Policy/runtime/security source-of-truth audit.
- Workbench lifecycle/TUI/native visibility audit.
- Role template import/authoring audit.
- Role tool-selection app-server/protocol audit.
- Verification/security test-matrix audit.

Their results are reconciled in feature/project docs and verification evidence. High/Medium findings must be fixed or explicitly accepted before the production-ready pass can be considered complete.
