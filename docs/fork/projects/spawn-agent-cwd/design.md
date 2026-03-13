# Spawn Agent CWD Design

## Canonical state

`spawn_agent.cwd` is a runtime-only override. The canonical applied value lives in the spawned
thread's runtime `Config.cwd`.

It is not canonical in:

- `SubAgentSource::ThreadSpawn`
- app-server thread source metadata
- role templates

## Data flow

1. `spawn_agent` parses optional `cwd`.
2. `resolve_spawn_agent_cwd(...)` normalizes and validates the value.
3. Child config is built from the parent turn and receives standard runtime overrides.
4. If `cwd` override exists, it replaces inherited `config.cwd`.
5. Spawn continues with the modified child config.

## Key invariants

- Missing `cwd` preserves inherited `turn.cwd`.
- Invalid `cwd` fails before spawn.
- Absolute paths are not rebased.
- Home-relative and plain relative paths resolve from user home, not from parent `turn.cwd`.
- Template augmentation must not change `cwd`.

## Intentional tradeoffs

- Keeping the override runtime-only avoids changing `ThreadSpawn` wire shape and persistence rules.
- Resolving relative paths from home matches the stashed implementation and gives stable semantics independent of caller cwd.
- Fail-fast validation is preferred over late tool-execution failures inside the child thread.
