# Feature: agent-switch-viewport

## Feature Passport

- Code name: `agent-switch-viewport`
- Status: deferred / obsoleted by upstream 0.130 native switch path.
- Goal: originally intended to restore per-agent committed TUI transcript state when switching
  threads; not implemented for `fork/130`.
- Scope in: TUI-local visual cache, switch-path restore/fallback, replay split, invalidation.
- Scope out: app-server protocol, Codex app, generated schemas, live-tail preservation, terminal
  scroll-offset preservation, rendered-row cache in v1.

## User Contract

For `fork/130`, this feature is not a user-facing fork behavior. Upstream 0.130 already provides
the native thread switch/replay path used by `/agent`, fast agent navigation, approval open-thread,
and side-thread transitions. The fork must not add a second viewport restore path unless a future
measured gap is documented in a new contract.

- `/agent`, `Alt+Left`, `Alt+Right`, approval open-thread, side-thread connect, and side-thread
  return all use the same restore/fallback behavior.
- Terminal hard reset still happens before restore or replay.
- Clean idle committed state may restore from TUI cache after the hard reset.
- Dirty, missing, running, in-progress, stream-time, buffered-event, or otherwise unsafe state falls
  back to the current full snapshot replay.
- Completed turns must not be replayed twice after cached restore.
- Draft composer text, queued input, collaboration mode, autosend suppression, and status line state
  still come from the existing thread snapshot path.
- The feature does not preserve arbitrary terminal scroll offset or live active tail in v1.
- Restore failure is controlled: clear any partial visual restore and fall back to the existing full
  replay path without duplicating committed cells.

## Empty/Error States

- If a thread has no cache, switch behavior remains identical to upstream 0.130 replay.
- If a cache is invalidated while the thread is inactive, the next switch silently uses full replay;
  this is not user-visible unless the existing replay path emits an error.
- If a target thread is closed or replay-only, existing informational messages remain authoritative.

## Integration And Compatibility

The expected integration point is upstream-native for `fork/130`:

- `AppEvent::SelectAgentThread` remains the single selection event.
- `select_agent_thread(...)` and the existing 0.130 replay/buffer behavior remain authoritative.
- `agents-overlay` must consume that existing path instead of depending on this deferred feature.

- TUI-only; no app-server or Codex app changes.
- Cache source is committed `HistoryCell` data, not rendered terminal rows.
- Restore rendering uses `render_transcript_lines_for_reflow(width)` so row-cap and wrapping
  behavior match the existing resize/reflow source of truth.
- The restore branch lives inside `select_agent_thread(...)`, after `reset_for_thread_switch(...)`;
  callers must not add alternate switching paths.
- `replay_thread_snapshot(...)` may be split only to extract a minimal runtime-state application
  helper. The existing full replay behavior remains the fallback.
- `ThreadInputState` needs focused running-state accessors or an equivalent local helper for
  restore eligibility; the input-state persistence path itself stays unchanged.
- Side-thread and approval-overlay switches must reuse the same selection path.
- App-server interactions remain the existing `thread/resume`, `thread/read`, `thread/loaded/list`,
  and notifications. No generated JSON/TypeScript artifacts are expected.

## Verification Matrix

| Surface | Required coverage |
| --- | --- |
| Restore | clean cache, width change, row cap, no duplicate turns |
| Runtime state | composer draft, queued input, collaboration mode, autosend, status line |
| Fallback | dirty, missing, running, in-progress, stream tail, any buffered snapshot event |
| Invalidation | inactive notifications, requests, history, feedback, refreshed snapshot, rollback, side discard, reset |
| Entry points | `/agent`, hotkeys, approval open-thread, side connect |
| UI | transcript overlay and backtrack use restored cells |
| Compatibility | no app-server schema, Codex app, or generated artifact changes |

## Doc Changelog

- 2026-05-12: Initial `fork/130` contract.
- 2026-05-12: Tightened V1 eligibility, switch-path integration, failure handling, and
  app-server non-impact contract.
- 2026-05-15: Marked deferred / obsoleted by upstream 0.130 native switch path.
