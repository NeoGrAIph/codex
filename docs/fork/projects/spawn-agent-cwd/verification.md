# Spawn Agent CWD Verification

## Scenarios

- `spawn_agent` without `cwd` inherits parent `turn.cwd`.
- `spawn_agent` with an absolute existing directory uses that directory.
- `spawn_agent` with a relative path resolves it from user home.
- `spawn_agent` with `~` and `~/...` resolves correctly from user home.
- empty `cwd`, missing path, file path, and missing home fail with the documented model-facing errors.
- existing thread-note behavior still works after applying cwd override support.

## Validation commands

- `cd codex-rs && cargo test -p codex-core spawn_agent_applies_cwd_override_to_spawned_thread`
- `cd codex-rs && cargo test -p codex-core resolve_spawn_agent_cwd_accepts_absolute_existing_directory`
- `cd codex-rs && cargo test -p codex-core resolve_spawn_agent_cwd_resolves_relative_against_home`
- `cd codex-rs && cargo test -p codex-core resolve_spawn_agent_cwd_expands_tilde_against_home`
- `cd codex-rs && cargo test -p codex-core resolve_spawn_agent_cwd_rejects_missing_home_for_relative_path`
- `cd codex-rs && cargo test -p codex-core resolve_spawn_agent_cwd_rejects_non_existing_path`
- `cd codex-rs && cargo test -p codex-core resolve_spawn_agent_cwd_rejects_file_path`
- `cd codex-rs && cargo test -p codex-core`

## Coverage notes

- focused core tests protect runtime behavior and validation;
- the tool-spec test protects schema exposure for `cwd`;
- no protocol/app-server coverage is required because the override is intentionally runtime-only.
