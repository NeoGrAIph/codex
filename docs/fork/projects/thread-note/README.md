# Project: thread-note

## Status

Implemented for `fork/130` on top of `rust-v0.130.0`.

Baseline release context is recorded in `../../research/0.130.0/thread-note.md`; the target tag is
`rust-v0.130.0` and the dereferenced baseline commit is
`58573da43ab697e8b79f152c53df4b42230395a8`.

Historical fork reference:
[`35187c0529`](https://github.com/NeoGrAIph/codex/commit/35187c0529b8f0797cd9460c714167c06d60f24b)
(`fork/118`, `feat(thread-note): add metadata-only thread note support`) is the canonical local
precedent for restart-safe file-backed thread-note persistence without a state DB migration.

## Canonical Links

- Feature contract: `../../features/thread-note.md`
- Release research: `../../research/0.130.0/thread-note.md`

## Goal

Add thread-owned metadata that describes purpose and competencies without changing model-facing
instructions or runtime behavior.

## Implementation Map

- Model tools: optional `spawn_agent.thread_note` / `spawn_agent.thread_note_competencies` in
  legacy v1 and MultiAgentV2, plus `set_thread_note` for the current thread or a visible
  sub-agent.
- Protocol metadata: `SessionMeta.thread_note` creation snapshot plus a persisted note update
  event/item for post-spawn updates and clears. `SubAgentSource::ThreadSpawn` intentionally does
  not expose `thread_note`; clients must use current thread/collab/list surfaces.
- Runtime metadata: `AgentMetadata` and thread/session snapshots mirror the latest note for live
  display without waiting for replay.
- Rollout/index: rollout recorder/list/metadata extraction plus an append-only `codex_home`
  `thread_note_index.jsonl` latest-value index keep the note restart-safe without a state DB
  migration.
- App-server: required-nullable `Thread.threadNote`, `CollabAgentState.threadNote`, and live
  `ThreadNoteUpdatedNotification` in the current fork server protocol v2, hydrated from
  rollout/index metadata rather than a new state DB column. Cross-version clients should still
  tolerate older servers omitting these fields.
- TUI: secondary note display in collab history plus agent navigation/cache propagation.

## Source Surfaces

- `codex-rs/protocol/src/protocol.rs`: `SubAgentSource`, `SessionMeta`, collab events, update event.
- `codex-rs/core/src/tools/handlers/multi_agents*.rs`: v1/v2 spawn args, schemas, output behavior.
- `codex-rs/core/src/agent/registry.rs` and `codex-rs/core/src/agent/control.rs`: live metadata.
- `codex-rs/rollout/src/**`: persistence, extraction, and the file-backed latest-value index.
- `codex-rs/app-server-protocol/src/protocol/v2/thread_data.rs`: `Thread`.
- `codex-rs/app-server-protocol/src/protocol/v2/item.rs`: `CollabAgentState`.
- `codex-rs/app-server-protocol/src/protocol/event_mapping.rs` and
  `codex-rs/app-server-protocol/src/protocol/thread_history.rs`: live/history mapping.
- `codex-rs/app-server/src/request_processors/thread_summary.rs` and
  `codex-rs/app-server/src/request_processors/thread_processor.rs`: read/list/resume/fork mapping.
- `codex-rs/tui/src/multi_agents.rs`, `app/agent_navigation.rs`, `app/thread_routing.rs`,
  `app/session_lifecycle.rs`, and `app/loaded_threads.rs`: cache and display.

## Locked Decisions

- `thread_note` is separate from persona, policy, role, and cwd.
- `thread_note_competencies` is input-only tool sugar and is folded into the single persisted
  `thread_note`; it is never sent through `message`.
- `TurnContextItem` is not canonical storage.
- `spawn_agent` output does not return note.
- `set_thread_note` output returns the updated target plus the normalized note or `null`.
- `list_agents` in legacy and MultiAgentV2 modes, plus legacy `wait_agent`, expose note metadata so
  orchestration can inspect the current responsibility of visible agents.
- App-server visibility is included in v1 through additive required-nullable fields emitted by the
  current fork server; older servers may omit them.
- No app-server stable mutation RPC is added in v1.
- `SessionMeta.thread_note` is only the creation snapshot; later updates require a persisted
  update event/item and an append-only file-backed index under `codex_home`.
- `threadNote: null` means no note or cleared note; string means current note. Missing field means
  older server/unknown.
- No state DB migration is required for v1 when app-server read/list hydrate `Thread.threadNote`
  from rollout metadata and the latest-value index.
