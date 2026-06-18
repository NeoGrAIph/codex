# OpenClaude agent UX/runtime research for fork/140

## Цель

Этот research package фиксирует evidence второй фазы анализа OpenClaude `v0.19.0` и текущего `fork/140` для будущей реализации agent workbench/observability, role profile UX, lifecycle actions, team/status layer и role-level tool selection. Документ не является implementation plan сам по себе: он задаёт source-of-truth карту и границы, чтобы дальнейшие feature branches не создавали параллельные registry/config/runtime paths.

## OpenClaude reference

Основной UI snapshot: `~/repo/AGENTS/openclaude/docs/ui-snapshots/background-agents-tasks.snap`. Он показывает пять полезных поверхностей:

- `Agents menu`: список agent profiles (`default`, `plan`, `explore`, `verification`, `claude-code-guide`) с кратким назначением.
- `Agent detail`: роль, inherited model, tools and `When to use`.
- `Create agent wizard`: пошаговое создание agent profile.
- `Background tasks`: scan-first список agent/shell/workflow tasks со status, kind, label, age/current activity.
- `Agent task detail` и `Team status`: recovery anchors (`Role`, `Status`, `Workdir`, `Last tool`, `Progress`) и слой team members.

Код OpenClaude подтверждает, что snapshot нужно читать как UX reference, а не как готовую схему данных для Codex. Agent profile source живёт в `src/tools/AgentTool/loadAgentsDir.ts`; runtime worker state хранится в `src/tasks/LocalAgentTask/LocalAgentTask.tsx`; list/detail rendering идёт через `src/components/tasks/BackgroundTasksDialog.tsx` и `AsyncAgentDetailDialog.tsx`; authoring flow находится в `src/components/agents/new-agent-creation/*`; profile detail/editor находятся в `src/components/agents/AgentDetail.tsx` и `AgentEditor.tsx`; team layer находится в `src/components/teams/TeamsDialog.tsx` and `TeamStatus.tsx`.

## Current Codex fork/140 map

Codex already has separate native substrates that must remain authoritative:

| Capability | Native source of truth | Existing projection | Gap |
| --- | --- | --- | --- |
| Role/profile templates | `Config.agent_roles`, built-ins in `codex-rs/core/src/agent/role.rs`, `$CODEX_HOME/agents/*.toml` | `/agent-roles` and `/agents` via `codex-rs/tui/src/chatwidget/agent_role_templates.rs` | TOML draft wizard/detail/source states, bounded field provenance, per-field runtime TOML provenance and native config-refresh reload for next foreground turn are implemented; markdown/persona templates and richer staged controls remain future work |
| Spawn role binding | `spawn_agent.agent_type`, `apply_role_to_config`, `resolve_role_config` | model-visible spawn tool schema and sub-agent session metadata | runtime binding remains native; richer “in use” projection needs thread evidence |
| Sub-agent lineage | `SessionSource::SubAgent(ThreadSpawn { parent_thread_id, agent_path, agent_nickname, agent_role, thread_note })` | app-server `Thread.source`, TUI `AgentNavigationState`, loaded thread backfill | detail now surfaces path/role/note/cwd/model provider when available; model/reasoning/service-tier and token anchors hydrate from native TUI session state / token notifications when available |
| Runtime status | `AgentControl`, `AgentStatus`, app-server `ThreadStatus` and active flags | `/agent`, `/subagents`, `agent_status_feed` | derived status covers running/idle/waiting/error/closed; age semantics remain unresolved |
| Activity preview | `ThreadItem`, `ThreadEventStore`, `agent_status_feed` bounded summaries | unified `/agent` picker detail and legacy status-feed reference | selected detail read model exists for bounded recent activity plus prompt/context/token/plan-progress anchors; richer progress beyond latest `TurnPlanUpdated` remains pending |
| Lifecycle actions | MAv2 `spawn`, `followup_task`, `send_message`, `interrupt_agent`, `list_agents`, `wait_agent`, native `AgentControl::close_agent` | model tools, TUI thread selection, confirmed `interrupt one`, root/subtree-owner queue-only `send-message`, root/subtree-owner trigger-turn `follow-up` and confirmed `close one` | dismiss-only hiding, retry and stop-all remain deferred contracts |
| Tool availability | `spec_plan`, `ToolRegistry`, MCP `ToolFilter`, `ToolsToml`, permission profiles | model-visible tool specs and dispatch registry | allowlist-only role tool selection, stale-id diagnostics, TOML draft authoring and discovered user-role allowlist editing are implemented; external app-server write/management API remains deferred |

## Capability findings

### Profile plus runtime observability

OpenClaude keeps `selectedAgent` inside background task state, so user-facing runtime rows can explain both who the worker is and what it is doing. Codex now joins profile identity (`agent_role`, `agent_nickname`) and runtime lineage (`agent_path`, `thread_id`) in the existing `/agent` picker detail without creating a new task registry. Remaining profile-aware gaps are richer in-use provenance and age/current-work semantics.

### Scan-first agent list

OpenClaude's list is optimized for quick scanning. In Codex, the current `/agent` picker now covers `status`, `kind`, `label` and bounded recent activity through native `AgentNavigationState`, app-server `ThreadStatus`, `ThreadEventStore`, `Thread.cwd` and `thread_note`. Age/current-work timing is still intentionally unresolved because spawned age and active-turn age have different meanings.

### Detail/recovery panel

OpenClaude detail gathers prompt/plan/progress/tokens/errors from `LocalAgentTask`. Codex must build a bounded read model from `thread/read`, live thread event buffer, `AgentNavigationState`, `SessionMeta`, and app-server thread metadata. Detail must not dump full transcript, raw reasoning, secrets, or encrypted inter-agent messages.

The fork/140 implementation uses the live/replay `ThreadEventStore` buffer as the native source for selected `/agent` detail activity. It only summarizes `ThreadItem` start/completion events, caps the selected detail to three recent entries, renders command names without command output, uses reasoning summaries without raw reasoning, renders tool names without raw tool JSON, and ignores user prompts/hook prompts. Detail also hydrates `thread_note`, `agent_path`, cwd and model provider from existing thread metadata, model/reasoning/service tier from native TUI `ThreadSessionState` when available, token anchors from latest per-thread `ThreadTokenUsageUpdated`, bounded `Prompt:` from app-server `Thread.preview`, bounded receiver-matched `Context:` from buffered `CollabAgentToolCall.prompt`, and bounded `Plan:` progress from latest per-thread `TurnPlanUpdated` notification. Raw `ThreadItem::Plan`, `PlanDelta`, plan explanation text and transcript turns remain out of scope for the first progress slice.

### Role wizard

OpenClaude wizard is useful as UX shape, but its markdown/frontmatter storage, `.openclaude/agents` paths, color, memory injection, and direct markdown prompt body do not match Codex. Codex wizard must write parser-validated TOML role files and bind only through `spawn_agent.agent_type`. Until a native reload path is proven, wizard copy must say whether the role is available to the current thread or only new sessions.

The fork/140 implementation intentionally avoids an OpenClaude-style staged wizard but does provide a native authoring flow: `/agent-roles` opens a parser-validated TOML draft prompt and writes `$CODEX_HOME/agents/<role>.toml` only after the existing role-file parser accepts the draft. The same surface exposes editable native profile fields, parser-validation diagnostics from the existing role-file parser, active/shadowed source states, bounded field provenance and explicit new-session availability copy.

### Rich role detail and source states

Codex role detail should distinguish role declaration (`description`, `config_file`, `nickname_candidates`), role config layer contributions (`developer_instructions`, model/provider/reasoning/service tier, permission/sandbox config), and external subsystem summaries (MCP, hooks, skills, apps). OpenClaude `active/shadowed` does not map 1:1 because Codex `Config.agent_roles` is already effective and can inherit fields across config layers.

### Lifecycle actions

Codex-compatible first actions are view/watch/select, `followup_task`, `send_message`, `interrupt_agent`, `list_agents`, `wait_agent` and native close. The current TUI slice implements view/watch/select, confirmed `interrupt one` over the existing app-server `turn/interrupt` path, root/subtree-owner queue-only `send-message` through experimental app-server `agent/message/send` and native MAv2 `InterAgentCommunication { trigger_turn: false }`, root/subtree-owner trigger-turn `follow-up` through `agent/followup/send` and native `InterAgentCommunication { trigger_turn: true }`, and confirmed `close one` through `agent/close` plus native `AgentControl::close_agent`. OpenClaude-style foreground remains “select/watch thread”, not direct input into a MAv2 child thread. `stop all` is plausible only as root-owned interrupt/close across descendants with two-step confirmation. Dismiss-only hiding, retry and skip need separate contracts.

### Team/status layer

OpenClaude team layer uses its own team file, mailbox and pane backend. Codex should not copy that storage/control plane. A Codex team/status layer should be a projection over MAv2 lineage and roles: planner/implementer/verifier are ordinary role template names, not hardcoded enums; running/idle/waiting/error derive from `ThreadStatus` and activity events.

### Role-level tool selection

OpenClaude `ToolSelector` is valuable UX, but Codex must not copy its markdown `tools`/`disallowedTools` profile fields. The first fork/140 runtime slice implements allowlist-only `[tool_selection] allowed_tools` through native `ConfigToml`/`Config` and enforces it in `spec_plan` before `tool_search`, Code Mode nested tools, hosted tools and `ToolRegistry` dispatch are built. Permission profiles remain a separate security boundary and must not be widened by tool selection.

## Maintained documents

- `docs/fork/projects/subagent-workbench/README.md`, `design.md`, `verification.md`: central workbench contract, current state, source map and checks.
- `docs/fork/features/subagent-workbench.md`: update from historical SAW-only contract to fork/140 scan-first workbench contract.
- `docs/fork/features/agent-role-templates.md` and `docs/fork/projects/agent-role-templates/*`: extend with wizard/detail/source-state requirements.
- `docs/fork/features/agent-role-tool-selection.md` and `docs/fork/projects/agent-role-tool-selection/*`: native role tool-selection contract, implementation notes and verification.
- Optional separate feature docs if the work is split out: `subagent-lifecycle-actions`, `team-status-layer`, `agent-role-wizard`. If kept under `subagent-workbench` or `agent-role-templates`, explicitly record that decision.

## Open questions

- Whether current-session role reload is required for the wizard or the contract remains “new sessions load new roles”.
- Whether app-server protocol needs top-level model/token/detail anchors, or TUI should keep assembling workbench detail from existing `Thread.source`, thread metadata and local config.
- Whether role-level tool selection should stay allowlist-only or later add denylist/MCP bucket authoring with deterministic merge and conflict rules.
- Whether team/status layer remains observational in v1 or grows root-owned bulk lifecycle actions after separate destructive-action contracts.
