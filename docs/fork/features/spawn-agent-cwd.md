# Spawn Agent CWD

## Feature passport

- Code name: `spawn-agent-cwd`
- Status: `implemented`
- Goal: разрешить `spawn_agent` запускать дочерний thread в отдельной рабочей директории, не меняя wire shape `ThreadSpawn`
- Scope in:
- `codex-rs/core/src/tools/handlers/multi_agents.rs`
- `codex-rs/core/src/tools/spec.rs`
- Scope out:
- app-server thread mutation APIs
- persisted thread source metadata for cwd override
- role template ownership of cwd

Implementation dossier:

- [docs/fork/projects/spawn-agent-cwd/README.md](../projects/spawn-agent-cwd/README.md)

## User contract

`spawn_agent` принимает новый optional аргумент:

- `cwd`

Поведение:

- если `cwd` не передан, spawned agent наследует `turn.cwd` родителя;
- если `cwd` передан и это абсолютный путь, он используется как есть;
- если `cwd` равен `~`, `~/...`, `~\\...` или относительному пути, он резолвится относительно home-директории пользователя;
- путь обязан существовать и быть директорией;
- пустое значение, отсутствие home-директории для home-relative resolution, несуществующий путь и путь к файлу приводят к model-facing ошибке и abort spawn.

Критичные строки ошибок:

- `spawn_agent.cwd cannot be empty`
- `spawn_agent.cwd requires a home directory, but HOME/USERPROFILE is unavailable`
- `spawn_agent.cwd does not exist: <path>`
- `spawn_agent.cwd is not a directory: <path>`

## Integration and compatibility notes

- `cwd` override применяется только к runtime `Config.cwd` spawned thread.
- `SubAgentSource::ThreadSpawn` не меняется; override не становится persisted source metadata.
- Role templates и другие template augmentations не получают права менять `cwd`.
- При отсутствии `cwd` поведение остаётся прежним: child inherits parent `turn.cwd`.

## Verification matrix

- `cd codex-rs && cargo test -p codex-core spawn_agent_applies_cwd_override_to_spawned_thread`
- `cd codex-rs && cargo test -p codex-core resolve_spawn_agent_cwd`
- `cd codex-rs && cargo test -p codex-core build_agent_spawn_config_uses_turn_context_values`
- `cd codex-rs && cargo test -p codex-core spawn_agent_preserves_thread_note_in_result_and_session_source`
- `cd codex-rs && cargo test -p codex-core`

## Doc changelog

- 2026-03-13: Added `spawn_agent.cwd` runtime override contract for `fork/114`, based on the parked `fork/107` stash implementation and adapted to the current thread-note fork state.
