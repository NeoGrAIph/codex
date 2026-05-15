# Исследование нативной реализации Agent Switch Viewport для rust-v0.130.0

## Baseline release

- Тег релиза: `rust-v0.130.0`
- Dereferenced commit: `58573da43ab697e8b79f152c53df4b42230395a8`
- Локальная baseline branch, наблюдаемая при исследовании: `fork/130-upstream`
- Рабочая branch, наблюдаемая при исследовании: `fork/130`

Этот документ описывает исследованный historical gap для `agent-switch-viewport`. После анализа
0.130 feature отложен: текущие fork features должны использовать upstream native switch/replay path
и не добавлять отдельный visual-cache branch без нового измеренного gap.

## Базовое описание

`agent-switch-viewport` - historical TUI feature для восстановления committed viewport per-agent
при переключении между agent threads. Для `fork/130` он deferred / obsoleted by upstream 0.130
native switch path.

Контракт:

- `/agent`, `Alt+Left` / `Alt+Right`, approval overlay open-thread и side-thread return/connect flows используют одинаковое switch behavior.
- Terminal hard reset все равно происходит перед restore или replay.
- Clean, committed, non-running thread state может восстанавливаться из visual cache.
- Dirty, missing, running или otherwise unsafe state fallback к текущему snapshot replay.
- Feature TUI-local и не должен добавлять app-server protocol, Codex app или wire/API changes.

## Текущее состояние 0.130

0.130 имеет centralized thread switching, но не имеет per-thread visual cache.

Подтвержденные entry points:

- `/agent` отправляет `AppEvent::OpenAgentPicker`.
- `Alt+Left` / `Alt+Right` вызывают adjacent thread navigation и route into selection.
- approval overlay open-thread отправляет `AppEvent::SelectAgentThread(thread_id)`.
- side-thread return/connect flows проходят через `select_agent_thread_and_discard_side(...)` или
  `select_agent_thread(...)`.

Текущий switch flow:

- `session_lifecycle.rs::select_agent_thread(...)` refreshes liveness, attaches state, stores current receiver, activates target, replaces `ChatWidget`, resets terminal/transcript state и вызывает `replay_thread_snapshot(...)`.
- `reset_for_thread_switch(...)` очищает transcript state и hard-resets terminal scrollback.
- `thread_routing.rs::store_active_thread_receiver(...)` captures `ThreadInputState`, но не visual transcript state.
- `replay_thread_snapshot(...)` monolithic: restores session/input, committed turns, buffered events, autosend state и status line.
- `ThreadInputState` уже хранит private `task_running` и `agent_turn_running`, но не exposes
  focused accessor для visual-restore eligibility.

Transcript/overlay model:

- `App.transcript_cells` - committed transcript cell list для currently displayed thread.
- `AppEvent::InsertHistoryCell` pushes committed cells into `transcript_cells`, updates transcript overlay if open и writes terminal history lines.
- Transcript overlay и backtrack читают из `transcript_cells` плюс current live tail.
- Stream-time cells могут находиться в хвосте transcript до consolidation; текущий
  `resize_reflow` treats trailing `AgentMessageCell` / `ProposedPlanStreamCell` runs как
  resize-sensitive transient state.

Resize/reflow:

- `resize_reflow.rs::render_transcript_lines_for_reflow(width)` рендерит из `transcript_cells` и уже handles row caps and stream spacing.
- Это native renderer, который нужно reuse для cached visual restore.

## Gap analysis

Baseline behavior в 0.130:

- каждый thread switch делает terminal hard reset и полный `replay_thread_snapshot(...)`;
- committed visual state хранится только для currently displayed thread в `App.transcript_cells`;
- inactive thread state хранит session/turns/events/input в `ThreadEventStore`, но не хранит
  rendered/committed viewport;
- resize row cap уже optimized через `BeginThreadSwitchHistoryReplayBuffer`, но это все равно
  replay path, а не per-thread restore.

Ожидаемый fork behavior:

- clean inactive thread with committed, non-running, non-dirty visual state can restore
  `transcript_cells` and scrollback from TUI-local cache;
- unsafe state must stay on current full replay path;
- all selection entry points must share one restore/fallback branch inside
  `select_agent_thread(...)`;
- restore must keep session/input/runtime replay semantics without duplicating completed turns.

Вне scope:

- no new app-server methods, fields, notifications or generated schema artifacts;
- no Codex app client changes;
- no alternate switch implementation outside the existing selection helpers.

## Направление нативной реализации

Реализовать TUI-only visual cache в split 0.130 TUI architecture.

Рекомендуемый module:

- `codex-rs/tui/src/app/thread_visual_state.rs`

Рекомендуемый state:

- Добавить `thread_visual_states: HashMap<ThreadId, ThreadVisualState>` в `App`.
- `ThreadVisualState` stores committed `transcript_cells`, optional rendered lines/width metadata,
  render mode / row-cap metadata, `dirty` и `restorable`.
- Rendered lines - только optimization. Каноничный restore source остается cloned
  `Arc<dyn HistoryCell>` cells.

Рекомендуемая integration:

- Перед `store_active_thread_receiver()` в `select_agent_thread(...)` capture current active thread visual state.
- Добавить focused accessors в `ThreadInputState` for `task_running` / `agent_turn_running`, или
  pure helper рядом с `ThreadVisualState`, и помечать running/task-active state non-restorable.
- Также skip restore, если snapshot содержит `TurnStatus::InProgress`, pending interactive
  request, stream-time transcript tail или buffered event set, который меняет visible history.
- Оставить existing input-state persistence path unchanged.
- После target activation, replacement и `reset_for_thread_switch(...)` попытаться clean visual restore.
- При restore:
  - set `self.transcript_cells` from cache
  - reset deferred/reflow/overlay/backtrack state
  - re-render committed cells at current terminal width using `render_transcript_lines_for_reflow`
  - restore session/input/status/autosend suppression semantics без replay completed
    committed turns
  - replay только runtime-visible buffered events, которые все еще valid after cache eligibility
    check
- Если restore fails, fallback к текущему `replay_thread_snapshot(...)`.

Dirty/cleanup hooks:

- Mark cached state dirty, когда inactive threads получают notifications, requests, history responses или feedback.
- Mark dirty или remove cache on rollback, refreshed snapshots, side discard, thread removal и global reset.
- Clear visual cache в thread event reset paths.
- Invalidate cached rendered lines при raw-output/history render mode change, terminal row-cap
  config change или любом code path, который mutates `transcript_cells` outside normal
  insert/consolidation.

## Risky integration points и source-of-truth files

Source-of-truth files для implementation:

- `codex-rs/tui/src/app/session_lifecycle.rs`: canonical thread switch sequence and only place
  where restore/fallback should branch.
- `codex-rs/tui/src/app/thread_routing.rs`: `ThreadEventStore`, `ThreadEventSnapshot`,
  active receiver storage and `replay_thread_snapshot(...)`.
- `codex-rs/tui/src/app/thread_events.rs`: buffered event categories and pending interactive replay
  state.
- `codex-rs/tui/src/app/resize_reflow.rs`: source-backed renderer, row cap behavior and stream-time
  reflow guards.
- `codex-rs/tui/src/app/event_dispatch.rs`: committed transcript mutation via
  `InsertHistoryCell` and stream consolidation.
- `codex-rs/tui/src/app_backtrack.rs` and `codex-rs/tui/src/pager_overlay.rs`: transcript overlay,
  live tail and backtrack state that must follow restored `transcript_cells`.
- `codex-rs/tui/src/chatwidget.rs`: `ThreadInputState` capture/restore and running flags.
- `codex-rs/tui/src/app/side.rs`: side-thread connect/return/discard flows that must reuse the
  same selection path.

Рискованные integration points:

- Splitting `replay_thread_snapshot(...)` - самый рискованный change, потому что сейчас он owns
  session display, input restore, turn replay, event replay, autosend suppression и status refresh
  в одной ordered function.
- `render_transcript_once(...)` is not the right primitive for cached restore; it does not model
  row caps the same way as resize reflow.
- `transcript_cells` may contain transient stream cells near consolidation boundaries; cache only
  when stream-time state is absent.
- `ThreadEventStore.active_turn_id` is not directly in `ThreadEventSnapshot`; restore eligibility
  must infer running state from turns, buffered events and `ThreadInputState`.

## Совместимость Codex App / App-server

Эта feature TUI-local.

Она не должна добавлять или менять:

- app-server methods
- app-server fields
- app-server notifications
- generated JSON/TypeScript schemas
- `experimentalApi` behavior
- Codex app client behavior

Существующие app-server interactions остаются достаточными: `thread/resume`, `thread/read`,
`thread/loaded/list` и current notifications. Если implementation требует app-server field/schema
change, это означает выход за scope этого research item и должно быть вынесено в отдельный
compatibility decision.

## Release-specific verification notes

Unit/behavior coverage:

- Clean cached transcript restore does not replay completed turns twice.
- Dirty cache falls back to snapshot replay.
- Running/task-active input state skips restore.
- Width changes re-render cached cells.
- Row cap and resize reflow behavior matches current replay rendering.
- Inactive notifications/requests/history/feedback dirty cached state.
- Runtime-only replay restores in-progress state and queued input correctly.
- `/agent`, `Alt+Left` / `Alt+Right`, approval overlay open-thread и side-thread connect all use the same restore path.
- Transcript overlay and backtrack after restore use the restored thread's `transcript_cells`.

Рекомендуемые focused commands:

- `cargo test -p codex-tui thread_visual_state`
- `cargo test -p codex-tui replay_thread_snapshot`
- `cargo test -p codex-tui thread_switch_replay_buffer`
- `cargo test -p codex-tui transcript_overlay`
- `cargo test -p codex-tui resize_reflow`

## Открытые риски

- Splitting runtime-only replay out of `replay_thread_snapshot(...)` должен preserve autosend suppression, pending message handling, status refresh и side-thread behavior.
- Cached `Arc<dyn HistoryCell>` vectors удерживают memory; cleanup must be explicit.
- Dirty marking must be conservative to avoid stale scrollback.
- Live active tail must not be treated as committed cache.
- Resize reflow state must not leak between threads.
- Restore errors happen after terminal hard reset; implementation должен fallback to replay without duplicating cells.
- Assumption: visual restore разрешен только для committed, consolidated transcript state. Если позже
  потребуется preserving live tail across inactive switches, это отдельный contract with higher
  runtime risk.
