# agent-switch-viewport Design

## Canonical State

Deferred for `fork/130`: no `App.thread_visual_states` state is added. Upstream 0.130 native
switch/replay remains the source of truth.

Historical proposed module, not implemented for `fork/130`:

- `codex-rs/tui/src/app/thread_visual_state.rs`

Recommended state shape:

- `ThreadVisualState { cells, dirty, invalidated_reason }`
- `cells: Vec<Arc<dyn HistoryCell>>`
- `dirty: bool`
- `invalidated_reason: Option<ThreadVisualInvalidationReason>` for diagnostics and tests

The cached cells are the only restore source. Width, row cap, raw-output mode, and render mode are
read from the current `App` when the target thread is restored.

## Switch Data Flow

1. Before storing active receiver, capture active thread visual state if safe.
2. Store existing input state through the current path.
3. Activate target thread and obtain snapshot.
4. Replace `ChatWidget` and run existing terminal hard reset.
5. Try visual restore.
6. On success, apply runtime-only snapshot state without replaying completed turns.
7. On failure, call existing `replay_thread_snapshot(...)`.

Exact integration in 0.130:

- Insert capture in `select_agent_thread(...)` immediately before `store_active_thread_receiver()`.
- Keep `store_active_thread_receiver()`, `activate_thread_for_replay(...)`,
  `refresh_snapshot_session_if_needed(...)`, `replace_chat_widget(...)`, and
  `reset_for_thread_switch(tui)?` in their current order.
- Insert `try_restore_thread_visual_state(...)` immediately after `reset_for_thread_switch(tui)?`.
- If restore succeeds, call the runtime helper extracted from `replay_thread_snapshot(...)`.
- If restore fails, call the unchanged full replay path.

No caller should bypass `select_agent_thread(...)`; `/agent`, hotkeys, approval open-thread, and
side-thread flows must all reach the same branch.

## Restore Eligibility

Restore is allowed only when:

- cache exists and is not dirty;
- snapshot has no in-progress turns;
- `snapshot.events` is empty in V1;
- input state is not task-running or agent-turn-running;
- transcript has no stream-time tail.

Use a conservative default: if eligibility cannot be proven from cache, `ThreadEventSnapshot`, and
`ThreadInputState`, use full replay.

`ThreadInputState` currently stores `task_running` and `agent_turn_running` privately. Add focused
accessors or an equivalent helper in the visual-state module; do not change the input-state capture
or restore contract.

Do not cache active transcript state while `should_mark_reflow_as_stream_time()` is true, because
trailing `AgentMessageCell` / `ProposedPlanStreamCell` runs are transient until consolidation.

## Runtime-only Snapshot Application

Extract the smallest useful helper from `replay_thread_snapshot(...)`.

It may:

- handle session state, including side-thread session handling;
- restore `ThreadInputState`;
- preserve queue autosend suppression and initial-message suppression semantics;
- submit pending initial/queued messages only in the same cases as the existing replay path;
- refresh the status line.

It must not:

- call `replay_thread_turns(...)` for completed turns;
- replay buffered events after a cached restore in V1;
- write committed history cells that are already restored from cache.

Full `replay_thread_snapshot(...)` remains the fallback and should keep current behavior for all
unsafe cases.

## Invalidation

Dirty or remove cached state on inactive notifications, requests, history responses, feedback,
rollback, refreshed snapshots, side discard, thread removal, and global thread-event reset.

Specific hooks:

- `enqueue_thread_notification(...)`, `enqueue_thread_request(...)`,
  `enqueue_thread_history_entry_response(...)`, and `enqueue_thread_feedback_event(...)` mark the
  inactive target thread dirty.
- `apply_refreshed_snapshot_thread(...)` removes that thread's cache.
- `handle_thread_rollback_response(...)` removes that thread's cache.
- Side discard removes the discarded side thread cache.
- `reset_thread_event_state()` clears all visual cache.
- Active-thread mutations do not need dirty marking; the active cache is recaptured on switch.

Raw-output mode changes and row-cap configuration changes do not invalidate cached cells in V1
because rendered rows are not cached. The next restore renders from cells using the current mode.

## Failure Modes

- Missing cache: full replay.
- Dirty cache: full replay.
- Running or in-progress snapshot: full replay.
- Non-empty buffered snapshot events: full replay.
- Restore render error after hard reset: clear partial `transcript_cells`, reset deferred/reflow
  state, and run full replay.
- Closed/replay-only thread with clean cache: restore may be used, but existing replay-only
  informational messages remain authoritative.

## Tradeoffs

- v1 does not preserve arbitrary terminal scroll offset.
- v1 does not cache rendered rows, avoiding raw-output/row-cap drift.
- v1 does not preserve live active tail across inactive switches.
- v1 treats all buffered snapshot events as unsafe even though some notice events may be
  theoretically replayable; this keeps the first implementation auditable.
- Runtime-only replay is the highest-risk code split and must stay minimal.

## App-server Compatibility

No app-server protocol, Codex app, generated schema, or `experimentalApi` changes are part of this
feature. Existing `thread/resume`, `thread/read`, `thread/loaded/list`, and notifications remain
sufficient because the cache is entirely TUI-local.
