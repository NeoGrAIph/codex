# Spawn Agent CWD on rust-v0.114.0

## Baseline

- Upstream baseline: `rust-v0.114.0`
- Fork branch: `fork/114`
- Source candidate: parked stash `stash@{0}` (`On 107: wip: park local changes`)

## Gap summary

`fork/114` already supports spawned thread metadata such as `thread_note`, but `spawn_agent`
still inherits `turn.cwd` unconditionally and does not expose a runtime cwd override.

The parked stash contains a runtime-only `spawn_agent.cwd` implementation plus tests and legacy
docs, but those docs target the older pre-`docs/fork` structure and must be adapted.

## Upstream-shaped adaptation

The adaptation for `fork/114` keeps cwd override localized to the collab tool/runtime layer:

- add optional `cwd` arg to `spawn_agent`;
- resolve and validate it in `core/src/tools/handlers/multi_agents.rs`;
- override only `Config.cwd` for the spawned child;
- leave `ThreadSpawn` wire/persistence shape unchanged;
- document the feature under `docs/fork/...`.

## Risk points

- `multi_agents.rs` already diverged from the stashed `fork/107` version because `fork/114`
  carries `thread_note`; the stash patch cannot be applied wholesale.
- error messages must stay stable because they are model-facing contract strings.
- relative path semantics are now an explicit fork contract and should not silently drift to
  parent-cwd semantics later.

## Verification

- `cd codex-rs && cargo test -p codex-core spawn_agent_applies_cwd_override_to_spawned_thread`
- `cd codex-rs && cargo test -p codex-core resolve_spawn_agent_cwd`
- `cd codex-rs && cargo test -p codex-core`
