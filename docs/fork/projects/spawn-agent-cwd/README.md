# Spawn agent cwd project

## Status

`fork/140` first iteration implemented for MAv2 `spawn_agent` only.

## Canonical links

- Feature contract: `docs/fork/features/spawn-agent-cwd.md`
- Shared architecture research: `docs/fork/research/multi-agents/architecture.md`
- Implementation: `codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs`, `codex-rs/core/src/tools/handlers/multi_agents_common.rs`, `codex-rs/core/src/tools/handlers/multi_agents_spec.rs`

## Implementation map

- Source of truth: child `Config.cwd`, `TurnEnvironmentSelections`, and `Config.permissions` workspace roots.
- Producer: MAv2 `spawn_agent.cwd` argument.
- Consumers: spawned `CodexThread` config snapshot, runtime environment selection, sandbox/permission profile materialization.
- Intentionally unaffected: V1 namespace spawn, app-server thread start protocol, TUI rendering, config schema.

## Current gaps

Resume-time failure policy for deleted child cwd, TUI display, and app-server-specific cwd surface are not part of this first iteration.
