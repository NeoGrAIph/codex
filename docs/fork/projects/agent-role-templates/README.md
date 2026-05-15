# Project: agent-role-templates

## Status

Implemented for `fork/130` on top of `rust-v0.130.0`
(`58573da43ab697e8b79f152c53df4b42230395a8`).

## Canonical Links

- Feature contract: `../../features/agent-role-templates.md`
- Release research: `../../research/0.130.0/agent-role-templates.md`
- Cwd dependency: `../../features/subagent-cwd.md`
- Project design: `design.md`
- Verification plan: `verification.md`

## Goal

Add markdown personas and template policy while preserving the native TOML role pipeline as the
primary role system. The implementation must be an upstream-shaped adaptation for 0.130, not a
replacement for `[agents.<role>]` or `.codex/agents/*.toml`.

## Implementation Map

- Parser/registry: discover `.md` templates from child-cwd-scoped paths, parse strict frontmatter,
  validate persona blocks, and expose template-only roles for spawn guidance.
- Spawn: add `agent_persona`, resolve explicit `cwd` through `subagent-cwd`, apply native TOML role
  first, then markdown persona/defaults/policy.
- Policy: add a session-scoped allow/deny/read-only policy that filters model-visible specs and
  blocks dispatch without mutating global MCP config.
- Persistence: store optional persona metadata on the child session source; keep policy runtime-only.
- Compatibility: expose optional app-server persona metadata while avoiding SQLite migrations and
  TUI persona projection in v1.

## Source Surfaces

- Native TOML roles: `codex-rs/core/src/config/agent_roles.rs`,
  `codex-rs/core/src/agent/role.rs`, `codex-rs/config/src/config_toml.rs`.
- Spawn handlers: `codex-rs/core/src/tools/handlers/multi_agents/spawn.rs`,
  `codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs`,
  `codex-rs/core/src/tools/handlers/multi_agents_common.rs`,
  `codex-rs/core/src/tools/handlers/multi_agents_spec.rs`.
- Lifecycle/resume: `codex-rs/core/src/agent/control.rs`,
  `codex-rs/protocol/src/protocol.rs`, `codex-rs/state/src/runtime/threads.rs`,
  `codex-rs/thread-store/src/local/read_thread.rs`.
- Tool policy: `codex-rs/core/src/tools/router.rs`, `codex-rs/core/src/tools/spec.rs`,
  `codex-rs/core/src/tools/spec_plan.rs`, `codex-rs/core/src/tools/tool_search_entry.rs`,
  `codex-rs/core/src/tools/handlers/dynamic.rs`, `codex-rs/codex-mcp/src/tools.rs`,
  `codex-rs/codex-mcp/src/connection_manager.rs`.
- Import-only prior art: `codex-rs/external-agent-migration/src/lib.rs`.

## Locked Decisions

- TOML roles remain primary.
- Markdown templates augment TOML roles or create template-only roles only when no TOML role exists.
- Discovery is child-cwd scoped and runs after explicit `spawn_agent.cwd` config rebuild.
- Explicit `cwd` is incompatible with full-history fork and uses `environments: None`.
- `agent_persona` is model-facing tool input only; persona/policy/cwd are not returned in spawn
  output.
- v1 adds optional app-server `Thread.agentPersona` and optional source `agent_persona` metadata.
- v1 does not add SQLite persona projection or TUI policy display.
- Tool policy uses `wildmatch` masks with `*` and `?`.

## Handoff Notes

- `thread-note` owns human display notes. Agent personas must never be copied into
  `thread_note`, and notes must never become persona/developer instructions.
- TUI and `agents-overlay` may continue using role, nickname, persona, cwd, and thread-note
  metadata. They must not require policy projection in v1.
- Persona labels are owned by the `agents-overlay` contract when rendered; template policy remains
  server-side only.
