# thread-note

- Status: active
- Goal: persist a metadata-only note for spawned threads and show it in collaboration transcript snapshots.
- Scope in: `spawn_agent.thread_note`, `set_thread_note`, restart/resume persistence, TUI `Note:` rendering for spawn/send events.
- Scope out: model instructions, `environment_context`, app-server `thread/read`, app-server `thread/list`.

## User Contract

- `spawn_agent` accepts optional `thread_note`.
- `set_thread_note` sets or clears the current thread note.
- Notes are normalized to one line: `Назначение: <purpose> | Компетенции: <competencies>`.
- Empty or whitespace-only notes clear the stored value.
- TUI shows `Note:` only for:
  - `Spawned ...`
  - `Sent input to ...`
- TUI reads note text from event payload snapshots, not live thread lookup.

## Compatibility

- Upstream behavior remains unchanged when no note is set.
- `thread_note` is fork-specific metadata and is not injected into prompt or developer context.
- app-server v2 keeps sub-agent source metadata compatible while intentionally omitting `thread_note` from public wire payloads.

## Verification Matrix

- `cargo test -p codex-protocol`
- `cargo test -p codex-core session_index_tests`
- `cargo test -p codex-core multi_agents_tests`
- `cargo test -p codex-tui multi_agents`
- `cargo test -p codex-app-server thread_list`

## Doc Changelog

- 2026-03-20: adapted the historical fork feature to `rust-v0.116.0` split multi-agent handlers and current app-server boundary.
