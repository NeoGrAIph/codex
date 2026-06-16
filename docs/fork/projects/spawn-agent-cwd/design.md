# Spawn agent cwd design

## Canonical state

The canonical cwd for a spawned agent remains the native thread config/environment state. `spawn_agent.cwd` is only an input to build the child `Config`; no separate cwd registry is introduced.

## Data flow

`spawn_agent.cwd` is parsed as a trimmed string, rejected unless it is an absolute path, checked against the parent turn's effective workspace roots, and then verified to exist as a directory. On success, the child config receives the requested cwd and keeps the same workspace roots and permission profile. `spawn_new_thread_with_source` and forked spawn then use the existing environment selection path to synchronize the primary environment cwd.

## Invariants

- Invalid, relative, non-workspace, nonexistent, or non-directory cwd fails before spawning.
- Child cwd does not add workspace roots and does not mark a new path trusted.
- Permission profile is re-applied through `Config.permissions.set_permission_profile`.
- Full-history fork may set cwd because cwd is runtime environment state, not model/role history state.

## Tradeoffs

This iteration avoids a new app-server contract and sqlite migration. It verifies runtime cwd through the existing thread config snapshot and documents resume/TUI gaps for the next stage.
