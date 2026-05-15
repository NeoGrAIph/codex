# agents-overlay Design

## Canonical State

The overlay is a projection, not lifecycle authority. It derives rows from existing thread state,
app-server thread data, navigation order, and collab history.

Recommended module:

- `codex-rs/tui/src/agents_overlay.rs`

Recommended state:

- `AgentsOverlay { projection, selected_row, action_menu, confirm_menu, inspected_thread,
  degraded_state }`
- `AgentsProjection { rows, root_thread_id, generated_at }`
- `AgentRow { thread_id, parent_thread_id, depth, agent_path, label, status, cwd, prompt_preview,
  model, reasoning_effort, pending_flags, thread_note, request_text, last_tool_text, plan_text }`

Inspect/action/confirm state is local UI state. It does not mutate thread lifecycle or app-server
state except when the user confirms `Close`, which submits the existing `Shutdown` operation to the
selected thread.

## Overlay Routing

- Add `Overlay::Agents(AgentsOverlay)`.
- Keep `App.overlay` as active overlay and add `suspended_overlay` for transcript restoration.
- Make overlay event routing kind-aware.
- Transcript overlay keeps transcript/backtrack behavior.
- Agents overlay owns `Esc`/`q`, inspect, and connect handling.

Exact routing changes:

- Replace the current generic `overlay.is_some() -> handle_backtrack_overlay_event(...)` assumption
  with a kind-aware overlay handler.
- `Overlay::Transcript` uses the existing transcript/backtrack path.
- `Overlay::Agents` handles its own keys and must never enter backtrack preview.
- `Overlay::Static` keeps simple pager forwarding.
- Main view `Ctrl-T` opens transcript.
- Transcript `Ctrl-T` suspends the transcript overlay and opens agents overlay.
- Agents `Ctrl-T` closes the whole overlay stack and returns to main.
- Agents `Esc`/`q` restores suspended transcript when it exists; otherwise it closes to main.

When transcript is suspended under agents, `InsertHistoryCell` and stream consolidation must update
the suspended transcript overlay. This preserves transcript -> agents -> transcript without losing
committed cells while agents is visible.

`Connect` closes the overlay stack and leaves alt screen before dispatching the existing selection
flow. Do not switch threads while an overlay still owns rendering.

## Projection

Rows are built from:

- `Thread.source` for parent/depth/tree shape.
- `Thread.cwd` for effective cwd.
- `Thread.status` and active flags for lifecycle.
- `ThreadItem::CollabAgentToolCall` for prompt/model/reasoning/status and parent-side fallback
  details.
- `TurnPlanUpdatedNotification` and `ThreadItem::Plan` for inspect-only compact plan summaries.
- `Thread.threadNote` and `CollabAgentState.threadNote` when available.
- `AgentNavigationState` for stable fallback ordering and labels.

The projection excludes the primary/root thread and side threads. If the active displayed thread is
a subagent, render it with a `(current)` marker and do not treat Connect as a state-changing action
for that row.

Do not use `AgentNavigationState::ordered_threads()` as the row list. It can include the primary
thread and carries only nickname/role/closed metadata. Use it only as a stable ordering and label
fallback.

Do not use `LoadedSubagentThread` as the final row model. It is intentionally lossy for picker
backfill. The overlay parser must preserve at least `Thread.source`, `Thread.cwd`, `Thread.status`,
depth, parent id, and `agent_path`.

Ordering default:

1. Parent-before-child tree order rooted at `primary_thread_id`.
2. Siblings ordered by `AgentNavigationState` first-seen order when present.
3. Fallback to `Thread.created_at`.
4. Final fallback to stringified `ThreadId`.

Status precedence:

1. Latest direct collab `agents_states`.
2. Local active-turn/runtime state from `ThreadEventStore`.
3. `Thread.status`.
4. Local closed marker from `AgentNavigationState`.
5. Not-loaded/failure fallback.

Plan display semantics:

- `Plan` is hidden by default and rendered only in `Inspect` for the selected row.
- Prefer the latest `TurnPlanUpdatedNotification.plan`.
- Render a compact summary, not the full step list:
  - with an active step: `Tasks completed/total - active step`;
  - without an active step: `Tasks completed/total`;
  - with no steps and an explanation: a short explanation fallback.
- If no plan/explanation is available, omit the `Plan:` line entirely.
- Do not derive plan text from reasoning or hidden model thought.

Thread note semantics:

- `threadNote` absent means no update or unknown.
- `threadNote: null` clears a stale cached note.
- `threadNote: "..."` replaces the note.
- Note is secondary metadata. It does not change identity, row label, ordering, tree shape, or
  connect target.
- V1 renders note in expanded details, not as the primary row label.

Cwd semantics:

- Display effective cwd from `Thread.cwd` when app-server data is available.
- Fallback to local session cwd only for locally known threads.
- Do not read cwd from model-facing spawn output; `subagent-cwd` intentionally does not return cwd
  there.
- Do not add overlay-specific app-server cwd fields.

Persona/policy:

- Display persona in the label as `(persona)` when it is present and not `default`.
- Do not display template policy. `agent-role-templates` enforces policy server-side, and overlay
  projection must not require those fields to exist.

## Actions

- `Inspect` toggles local detail state.
- `Connect` closes overlay stack and sends selection through the existing path.
- `Close` opens a confirmation row. `Yes` submits `Shutdown` to the selected thread via the existing
  thread operation path. `No`/`Esc` close the confirmation without submitting anything.

Connect target:

- Always dispatch `AppEvent::SelectAgentThread(thread_id)` or call the same selection helper used
  by that event after closing the overlay stack.
- If the target disappeared, the existing selection path owns the user-facing error.
- Agents overlay must not attach, resume, unsubscribe, or discard threads directly.

## Empty And Degraded States

- Empty projection renders the `A G E N T S` frame with an empty-state line.
- If `thread/loaded/list` fails, render locally known rows and mark projection degraded.
- If `thread/read(false)` fails for one id, skip that server-only row unless local state can supply
  enough data.
- Degraded state is informational only; it does not block local inspect/connect rows.

Recommended empty-state copy:

- `No sub-agent threads available.`

Recommended degraded-state copy:

- `Some agent details could not be refreshed.`

## Tradeoffs

- The overlay does not poll periodically in v1; it refreshes on open and rebuilds from local updates.
- Loaded/currently known subagents are shown; historical unloaded descendants are deferred.
- `Close` uses confirmed `Shutdown` instead of `thread/unsubscribe`, preserving the existing thread
  lifecycle contract.
- Thread-note display waits for the `thread-note` feature fields; absent fields do not degrade the
  overlay.

## App-server Compatibility

The overlay itself introduces no app-server protocol or Codex app changes.

Allowed data sources:

- existing `thread/loaded/list`;
- existing `thread/read`;
- existing notifications;
- existing `Thread`, `ThreadStatus`, `ThreadItem::CollabAgentToolCall`;
- optional `Thread.threadNote` / `CollabAgentState.threadNote` only after the `thread-note` feature
  has added them.

Disallowed in V1:

- schema changes solely for overlay;
- new app-server close method;
- `thread/unsubscribe` as close;
- policy projection requirement;
- Codex app UI changes.
