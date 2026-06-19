# Subagent policy and ownership project

## Status

`fork/140` ownership slices implemented path-based MAv2 interrupt ownership, V1 legacy `close_agent` ownership, workbench/app-server mutating-action ownership, non-root read-only denial, explicit workbench action allow/deny enforcement, and experimental app-server `agentRole/actionPolicy/set` management for native user role TOML. `feature/140/agent-policy-control-plane` adds the durable action-policy snapshot substrate in `SubAgentSource::ThreadSpawn`; ThreadManager workbench mutations consume that persisted snapshot for `allowed_actions`/`denied_actions`. Role-level `read_only` is handled through native role-applied `default_permissions = ":read-only"` and `PermissionProfile::read_only()`, including v1/MAv2 spawn paths that refresh live runtime context after applying the role. Repo-local roles are trust-gated through native enabled/disabled project config layers, and `.agents` is protected by native filesystem sandbox metadata carveouts.

## Canonical links

- Feature contract: `docs/fork/features/subagent-policy-and-ownership.md`
- Shared architecture research: `docs/fork/research/multi-agents/architecture.md`
- Implementation: `codex-rs/core/src/tools/handlers/multi_agents_v2/interrupt_agent.rs`, `codex-rs/core/src/thread_manager.rs`, `codex-rs/core/src/agent/control.rs`, `codex-rs/app-server/src/request_processors/thread_processor.rs`, `codex-rs/tui/src/app/session_lifecycle.rs`

## Implementation map

- Source of truth: `AgentMetadata.agent_path` in the root-scoped `AgentControl` registry plus current caller `SessionSource` / author thread source metadata.
- Producer: spawned thread registration through `AgentControl::prepare_thread_spawn`.
- Consumers: MAv2 `interrupt_agent` target resolution and ownership guard; V1 `close_agent` ownership guard; app-server/TUI workbench message, follow-up, close, dismiss, retry and stop-all through `ThreadManager`.
- Action-policy substrate: `SubAgentSource::ThreadSpawn.action_policy` is the durable carrier for workbench action allow/deny semantics. It records versioned default/config/role-applied source and persists effective policy across resume/retry even if the source role file changes later.
- Action-policy management API: `agentRole/actionPolicy/set` is an experimental whole-policy write path for discovered user roles under `$CODEX_HOME/agents/*.toml`. It edits native `[subagent_action_policy]` TOML, rejects built-ins and external `config_file` roles, validates through the native role parser and must refresh loaded thread configs before success without mutating existing persisted snapshots.
- Trust/read-only metadata: repo-local `.codex/agents/*.toml` roles are discovered only from enabled project layers; untrusted/unknown project layers stay disabled. Top-level `.agents` is part of native protected metadata alongside `.git` and `.codex` and remains read-only under workspace-write sandbox policy.
- Intentionally unaffected: MCP-only policy registry/UI, permission-profile authoring UI, direct edits to built-in role files and non-workbench runtime policy state. MCP allow/deny is intentionally managed by the generic role `[tool_selection]` contract rather than by `SubAgentActionPolicySnapshot`.

## Current gaps

MCP allow/deny management is covered through `agent-role-tool-selection`: native `[tool_selection]`, `/agent-roles` catalog-assisted TOML authoring, `agentRole/toolSelection/set`, `spec_plan`/`ToolRegistry` and direct `mcpServer/tool/call` checks. This policy/ownership package therefore does not add a dedicated MCP-only management surface. Current coverage includes non-root interrupt/close/message/follow-up/dismiss/retry/stop-all boundaries, V1 close parity, role-applied read-only permissions through the native config path, workbench read-only denial, explicit action allow/deny enforcement, app-server action-policy management for writable user role TOML, project role trust gating and read-only `.agents` sandbox metadata protection.
