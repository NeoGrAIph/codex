# Project: agent-switch-viewport

## Status

Deferred / obsoleted for `fork/130` by the upstream 0.130 native switch path.

Baseline context:

- Release tag: `rust-v0.130.0`
- Baseline commit: `58573da43ab697e8b79f152c53df4b42230395a8`
- Research package: `docs/fork/research/0.130.0/agent-switch-viewport.md`

## Canonical Links

- Feature contract: `../../features/agent-switch-viewport.md`
- Release research: `../../research/0.130.0/agent-switch-viewport.md`
- Related overlay feature: `../agents-overlay/README.md`

## Goal

This project records the historical fork idea for restoring committed TUI transcript state when
switching between agent threads. It is not implemented for `fork/130`: upstream 0.130 already
provides the native switch/replay path that current fork features should reuse.

## Implementation Map

- Switch path: `codex-rs/tui/src/app/session_lifecycle.rs::select_agent_thread(...)`.
- Existing replay source of truth: `codex-rs/tui/src/app/thread_routing.rs::replay_thread_snapshot(...)`.
- Visual cache: not implemented for `fork/130`.
- Resize renderer: `codex-rs/tui/src/app/resize_reflow.rs::render_transcript_lines_for_reflow(...)`.
- Transcript mutation: `codex-rs/tui/src/app/event_dispatch.rs` for insert/consolidation events.
- Input/running state: `codex-rs/tui/src/chatwidget.rs::ThreadInputState`.
- Invalidation: inactive event queues, refreshed snapshots, rollback, reset, side discard.

## Locked Decisions

- TUI-only; no app-server or Codex app changes.
- Cache committed `HistoryCell` values, not rendered rows.
- Use existing resize/reflow renderer during restore.
- Unsafe states always fall back to current replay path.
- In V1, any buffered snapshot event makes restore ineligible.
- Runtime-only snapshot application may restore session/input/status/autosend state, but it must
  not replay completed turns after visual restore.
- The feature does not preserve live tail or terminal scroll offset in V1.

## Current 0.130 Facts

- `select_agent_thread(...)` currently always hard-resets and then calls
  `replay_thread_snapshot(...)`.
- `store_active_thread_receiver(...)` captures `ThreadInputState`, not committed visual state.
- `App.transcript_cells` is only the committed transcript for the currently displayed thread.
- `render_transcript_lines_for_reflow(...)` is already the source-backed row-cap renderer.
- Side-thread connect/return flows call the same selection helpers and must remain on that path.

## Implementation Order

1. Keep this project deferred for `fork/130`.
2. Reuse upstream 0.130 `select_agent_thread(...)` for `agents-overlay` and later TUI features.
3. Re-open this project only if a measured gap remains after native switch/replay behavior.
