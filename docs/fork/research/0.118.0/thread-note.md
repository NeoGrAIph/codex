# `thread_note` research for `rust-v0.118.0`

- Baseline tag: `rust-v0.118.0`
- Baseline commit: `b630ce9a4e754d35a1f33e4366ba638d18626142`
- Research target: the current `thread_note` contract for collaboration metadata, persistence, and
  transcript rendering in `fork/118`
- Scope: `protocol`, `core`, `rollout`, `app-server`, and `tui`
- Research state: current `fork/118` staged work, not only committed history

## Executive summary

- `thread_note` is a metadata-only thread surface for specialization and competencies.
- The current `fork/118` state keeps note out of model-facing context.
- The durable recovery path is rollout-first, with `thread_note_index.jsonl` retained as a legacy
  fallback.
- App-server v2 collaboration history surfaces now carry `thread_note` in `CollabAgentState`.
- Top-level app-server `Thread` payloads still do not expose `thread_note`.

## Upstream gap

The `rust-v0.118.0` baseline does not provide this fork contract for collaboration-specific thread
note metadata, canonical note normalization, or the TUI `Note:` transcript rendering tied to
spawn/send-input snapshots.

## Fork adaptation

The current fork adapts the feature with the following decisions:

- normalize all note input through a dedicated protocol helper
- treat note as metadata-only rather than prompt-visible context
- persist note in session and turn metadata so restart/resume can reconstruct it
- retain the legacy note index as a fallback for older persisted threads
- project note-bearing collaboration metadata into app-server history state so replay and remote UI
  surfaces preserve the same transcript semantics

## Conflict-prone files

- `codex-rs/protocol/src/thread_note.rs`
- `codex-rs/protocol/src/protocol.rs`
- `codex-rs/core/src/codex.rs`
- `codex-rs/rollout/src/session_index.rs`
- `codex-rs/app-server-protocol/src/protocol/thread_history.rs`
- `codex-rs/app-server-protocol/src/protocol/v2.rs`
- `codex-rs/tui/src/chatwidget.rs`
- `codex-rs/tui/src/multi_agents.rs`

## Important divergences from older fork revisions

- Older fork docs that described `thread_note` as prompt-visible are obsolete for `fork/118`.
- Older fork docs that documented dedicated app-server note RPCs are obsolete for `fork/118`.
- Older `fork/117` docs that claimed app-server strips note from public collaboration state are
  obsolete for the current staged `fork/118`; `CollabAgentState.thread_note` is present again.

## Release-specific verification

- `cd codex-rs && cargo test -p codex-protocol`
- `cd codex-rs && cargo test -p codex-rollout`
- `cd codex-rs && cargo test -p codex-core multi_agents`
- `cd codex-rs && cargo test -p codex-tui multi_agents`
