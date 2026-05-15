# Исследование нативной реализации Agents Overlay для rust-v0.130.0

## Baseline release

- Тег релиза: `rust-v0.130.0`
- Dereferenced commit: `58573da43ab697e8b79f152c53df4b42230395a8`
- Локальная baseline branch, наблюдаемая при исследовании: `fork/130-upstream`
- Рабочая branch, наблюдаемая при исследовании: `fork/130`

Этот документ описывает gap между upstream-shaped baseline 0.130 и fork behavior для
`agents-overlay`. Первая native implementation должна оставаться TUI-local.

## Базовое описание

`agents-overlay` восстанавливает dedicated full-screen `A G E N T S` TUI overlay для inspection и connection to multi-agent sessions.

Адаптация 0.130:

- `/agent` остается lightweight picker.
- `AGENTS -> Connect`, `/agent` и hotkey navigation должны reuse same existing thread-selection flow.
- Rows show known subagents; a currently displayed subagent is marked as current instead of being
  hidden.
- Primary/root thread по умолчанию не является agent row; его можно использовать только как tree
  root/context.
- Overlay - это projection over existing thread/collab/runtime state, а не new lifecycle authority.
- Первая native implementation должна быть TUI-local.
- `Close` is available as a confirmed TUI `Shutdown` operation against the selected agent thread.
  It still must not be simulated with `thread/unsubscribe`.

## Текущее состояние 0.130

Overlay infrastructure:

- `pager_overlay.rs` имеет `Overlay::{Transcript, Static}`, `TranscriptOverlay`, `StaticOverlay` и `PagerView`.
- `App` stores `overlay: Option<Overlay>`.
- `App::run` routes all events to `handle_backtrack_overlay_event(...)` while `overlay.is_some()`;
  normal `input.rs` key handling opens transcript only when no overlay is active.
- `app_backtrack.rs` содержит overlay event forwarding и backtrack behavior, которое semantically
  transcript-specific, но сейчас находится в generic overlay path.

Overlay stack отсутствует. Текущая model `Option<Overlay>` не может suspend transcript, open agents и later restore transcript.

Transcript overlay уже получает live updates через `AppEvent::InsertHistoryCell` и live tail sync из `ChatWidget`.

Lightweight agent picker:

- `/agent` и `/multi-agents` отправляют `AppEvent::OpenAgentPicker`.
- `session_lifecycle.rs` owns `open_agent_picker`, liveness refresh, selection и loaded subagent backfill.
- `agent_navigation.rs` owns stable order и active agent labels.
- `multi_agents.rs` owns picker entries, label formatting, status dots и collab tool-call history cells.

Current metadata sources:

- `ThreadEventStore` и `ThreadEventSnapshot`
- `AgentNavigationState`
- app-server `Thread { status, cwd, source, agent_nickname, agent_role, turns, ... }`
- `ThreadStatus::Active { active_flags }` для waiting-on-approval / waiting-on-user-input flags
- `ThreadItem::CollabAgentToolCall { tool, status, sender_thread_id, receiver_thread_ids, prompt, model, reasoning_effort, agents_states }`
- `CollabAgentState { status, message }`
- `loaded_threads.rs::find_loaded_subagent_threads_for_primary`, который уже walks
  `SessionSource::SubAgent(ThreadSpawn { parent_thread_id, .. })`, но returns a flat list и drops
  `depth`, `agent_path`, `cwd`, `status` и parent edge metadata, нужные overlay

Missing dependencies from older fork contracts:

- `thread_note` отсутствует в 0.130.
- `spawn_agent.cwd` override отсутствует, хотя `Thread.cwd` уже доступен для display.
- persona metadata is available from fork role-template/thread metadata; policy metadata remains
  enforcement-only and should not be projected.
- `agent_path` exists in `SessionSource::SubAgent(ThreadSpawn { ... })`, но текущий TUI backfill
  helper does not preserve it.

Close state:

- Core имеет model-tool close support.
- App-server `thread/unsubscribe` only unsubscribes a client connection.
- TUI can submit the existing `Shutdown` op to the selected thread, which is sufficient for the
  fork/118 overlay action without adding app-server schema.

## Gap analysis

Baseline behavior в 0.130:

- `/agent` picker is a bottom-pane selection view, not a full-screen inspection surface;
- `Ctrl-T` owns the transcript overlay in upstream; the fork intentionally extends it into the
  overlay cycle main -> transcript -> `A G E N T S` -> main.
- `Overlay` has no `Agents` variant and no stack/origin state;
- thread discovery is split between local `AgentNavigationState`, local `ThreadEventStore` and
  app-server `thread/loaded/list` + `thread/read`;
- app-server has no selected-agent close RPC for TUI.

Ожидаемый fork behavior:

- full-screen `A G E N T S` overlay projects known subagents without becoming lifecycle authority;
- `Connect` reuses `AppEvent::SelectAgentThread(thread_id)` /
  `select_agent_thread_and_discard_side(...)`;
- projection preserves tree/depth where available, filters primary root and side threads, and marks
  the current subagent row;
- transcript overlay can be suspended/restored if agents overlay is opened from it;
- no app-server or Codex app changes are required for first implementation.

Вне scope:

- no simulated close through `thread/unsubscribe`; use confirmed `Shutdown` only;
- no new app-server schema fields just to render the first TUI overlay;
- no duplicate thread switching path;
- no display of unavailable historical metadata as if it existed in 0.130.

## Направление нативной реализации

Добавить TUI-only overlay module, например `codex-rs/tui/src/agents_overlay.rs`.

Overlay routing:

- Добавить `Overlay::Agents(AgentsOverlay)` или equivalent kind-aware overlay state.
- Keep transcript live-tail and transcript backtrack behavior transcript-only.
- Добавить minimal overlay origin/suspend state, например `OverlayStack` / `OverlayOrigin` рядом с
  `App.overlay`, а не hidden inside `AgentsOverlay`:
  - main -> `Ctrl-T` opens transcript
  - transcript -> `Ctrl-T` opens agents while preserving transcript state
  - agents -> `Ctrl-T` closes overlay stack
  - `Esc`/`q` in agents restores transcript when opened from transcript
  - `Esc`/`q` in agents closes when opened directly
- `handle_backtrack_overlay_event(...)` должен стать kind-aware до добавления agents; иначе Esc in
  agents может случайно включить transcript backtrack semantics.

Projection model:

- Build `AgentsProjection` from existing TUI/app-server state.
- Preserve parent/depth/tree shape from `SessionSource::SubAgent(ThreadSpawn { parent_thread_id, depth, agent_path, ... })`.
- Exclude primary and side threads; mark the current active subagent row.
- Exclude `primary_thread_id` from agent rows; keep it only as root context for tree assembly.
- Use deterministic fallback ordering.
- Не использовать `AgentNavigationState::ordered_threads()` напрямую как projection list: он
  включает primary thread и несет только nickname/role/closed metadata.
- Не использовать `LoadedSubagentThread` как final row model; нужно либо расширить loader, либо
  добавить отдельный parser, который preserves `Thread.source`, `Thread.cwd`, `Thread.status` и
  depth/path.

Initial row fields:

- display label from nickname/role/path/id
- lifecycle status
- spinner for pending/running
- model/reasoning when known
- cwd from `Thread.cwd` or local session state
- prompt/request preview from collab tool-call prompt
- pending approval/input flags from `ThreadStatus::Active.active_flags` and local stores
- optional last tool and plan summaries when available locally
- persona label when present/non-default

Plan summary:

- Render only in `Inspect` for the selected row.
- Prefer `TurnPlanUpdatedNotification.plan/explanation`.
- Show `Tasks completed/total - active step`, `Tasks completed/total`, or short explanation
  fallback.
- Do not show a full persistent plan-step list and do not derive plan content from reasoning.

Status precedence should be explicit:

1. direct collab `agents_states`
2. local active turn state
3. `Thread.status`
4. local closed marker
5. not-loaded/failure fallback

Actions:

- `Inspect`: local expand/collapse.
- `Connect`: route through existing `AppEvent::SelectAgentThread(thread_id)` / selection helpers.
- `Close`: confirmation required; `Yes` submits existing `Shutdown`; `No`/`Esc` do nothing.

Do not duplicate thread switching logic.

## Risky integration points и source-of-truth files

Source-of-truth files для implementation:

- `codex-rs/tui/src/pager_overlay.rs`: current `Overlay` enum and pager rendering contracts.
- `codex-rs/tui/src/app.rs`: `overlay: Option<Overlay>`, draw/event routing and app state fields.
- `codex-rs/tui/src/app_backtrack.rs`: transcript-specific overlay forwarding, live-tail sync and
  backtrack behavior that must not run inside agents overlay.
- `codex-rs/tui/src/app/input.rs`: current `Ctrl-T`, `Alt+Left` and `Alt+Right` key handling.
- `codex-rs/tui/src/app/session_lifecycle.rs`: `open_agent_picker`,
  `backfill_loaded_subagent_threads`, `select_agent_thread(...)` and loaded-thread hydration.
- `codex-rs/tui/src/app/agent_navigation.rs`: stable order and picker labels, but not full overlay
  projection.
- `codex-rs/tui/src/app/loaded_threads.rs`: existing spawn-tree walk, useful but too lossy for
  tree overlay rows.
- `codex-rs/tui/src/multi_agents.rs`: label/status-dot/collab tool-call rendering helpers.
- `codex-rs/app-server-protocol/src/protocol/v2/thread.rs`,
  `codex-rs/app-server-protocol/src/protocol/v2/thread_data.rs` and
  `codex-rs/app-server-protocol/src/protocol/v2/item.rs`: stable Thread, status and collab item
  payloads available without schema changes.

Рискованные integration points:

- `Option<Overlay>` слишком мал для transcript -> agents -> transcript restoration. Добавление
  `Overlay::Agents` without origin state потеряет transcript state.
- Existing overlay routing treats `Esc` in overlay as backtrack entry unless guarded by transcript
  kind.
- Loaded-thread discovery сейчас fetches loaded ids и затем `thread/read(false)` per id; если
  overlay нужны historical prompt previews для unloaded/local-missing rows, это отдельное
  read/latency decision.
- Projection must not make `AgentNavigationState` lifecycle-authoritative; it is a picker cache.

## Совместимость Codex App / App-server

Первая implementation должна использовать только existing app-server APIs:

- `thread/loaded/list`
- `thread/read`
- existing notifications
- existing `Thread` and `ThreadItem` fields

Изменения Codex app не требуются, потому что это TUI overlay, а не shared app-server UI surface.

Избегать app-server schema changes для первой implementation. Existing data enough for a useful overlay: identity, tree shape, cwd, status, model/reasoning и prompt preview.

`Close` must not be simulated with `thread/unsubscribe`. This fork implementation uses the existing
thread `Shutdown` op from the TUI side and does not add app-server schema.

## Release-specific verification notes

Projection tests:

- flat list and nested tree
- active-thread and side-thread exclusion
- primary/root thread exclusion from agent rows
- deterministic ordering
- fallback labels for missing metadata
- cwd/model/reasoning/request preview extraction
- status precedence
- `find_loaded_subagent_threads_for_primary` remains covered for existing picker backfill, while
  the new projection parser preserves depth/path/status

Overlay behavior tests:

- `Ctrl-T` main -> transcript
- `Ctrl-T` transcript -> agents
- `Ctrl-T` agents -> closes overlay stack
- `Esc`/`q` agents from transcript restores transcript
- restored transcript includes cells committed while agents overlay was visible
- transcript backtrack does not activate inside agents overlay

Action/snapshot tests:

- `Inspect` toggles row details
- `Connect` uses existing selection flow
- `Close` confirmation submits `Shutdown`; cancelled close submits nothing
- inspect-only plan summary hides empty plan state and never renders the full step list
- `/agent` remains unchanged
- empty, flat, nested, selected, inspect-expanded, running, completed, errored, shutdown/not-loaded и narrow terminal snapshots

No app-server schema artifacts are expected for the first implementation.

Рекомендуемые focused commands:

- `cargo test -p codex-tui agent_navigation`
- `cargo test -p codex-tui loaded_threads`
- `cargo test -p codex-tui transcript_overlay`
- `cargo test -p codex-tui agents_overlay`

## Открытые риски

- `Close` must stay a confirmed `Shutdown` operation and must not drift into `thread/unsubscribe`.
- Current `Option<Overlay>` model too small for transcript -> agents -> transcript restoration.
- Overlay/backtrack routing must become kind-aware to avoid transcript backtrack inside agents overlay.
- Status projection can be stale unless precedence is explicit and conservative.
- `thread-note`, persona и subagent-cwd metadata should be displayed only from existing native/fork
  sources.
- `AGENTS -> Connect` must reuse existing selection and must not introduce a second viewport restoration path.
- Assumption: first implementation может render useful rows только из loaded/currently known
  subagents. Persisted-but-not-loaded historical descendants требуют отдельного product/latency
  decision, если они должны появиться в overlay.
