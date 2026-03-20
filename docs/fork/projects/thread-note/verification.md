# Verification

## Core scenarios

- Spawn an agent with `thread_note` and verify normalized note in result, session snapshot, and `SessionSource`.
- Update an existing thread note with `set_thread_note` and verify persistence entry.
- Clear a thread note and verify the latest lookup returns `None`.
- Resume a thread and verify note restoration from `thread_note_index.jsonl`.

## UI scenarios

- `Spawned ...` transcript row shows `Note:` before prompt preview.
- `Sent input to ...` transcript row shows `Note:` before prompt preview.
- Empty notes do not render a `Note:` line.

## Commands

- `cargo check --tests -p codex-protocol -p codex-core -p codex-tui -p codex-app-server -p codex-app-server-protocol -p codex-tui-app-server`
- `cargo test -p codex-protocol`
- `cargo test -p codex-core session_index_tests`
- `cargo test -p codex-core multi_agents_tests`
- `cargo test -p codex-tui multi_agents`
- `cargo test -p codex-app-server thread_list`

## Coverage Gaps

- No dedicated app-server integration test yet asserts that `thread_note` is absent from every v2 schema artifact; this is currently guarded by the app-server-owned enum mapping plus targeted thread list coverage.
