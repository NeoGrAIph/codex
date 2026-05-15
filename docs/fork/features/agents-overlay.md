# Feature: agents-overlay

## Feature Passport

- Code name: `agents-overlay`
- Status: implemented for `fork/130`
- Goal: add a full-screen TUI `A G E N T S` overlay for inspecting and connecting to subagent threads.
- Scope in: TUI overlay, `Ctrl-T` cycle, kind-aware overlay routing, suspended transcript
  restoration, projection, action menu, inspect/details, connect action, shutdown action with
  confirmation, persona label display, and compact plan summary display.
- Scope out: app-server schema changes, Codex app UI, database/schema migrations, and policy
  projection.

## User Contract

- `/agent` remains the lightweight picker.
- `A G E N T S` is a full-screen inspection overlay in the alternate screen.
- `Ctrl-T` cycles main -> transcript -> `A G E N T S` -> main.
- Rows show known loaded/currently tracked subagents. If the active displayed thread is itself a
  subagent, it is shown with a `(current)` marker.
- Primary/root thread is tree context, not an agent row.
- Side threads are excluded from connectable rows.
- `Enter` opens a row-local action menu: `Inspect`, `Connect`, `Close`.
- `Inspect` expands/collapses local details only.
- `Inspect` may show `Request`, `Tool`, and `Plan` details for the selected row.
- `Plan` is hidden outside `Inspect`. When present, it is a compact summary only:
  `Tasks completed/total - active step`, `Tasks completed/total`, or a short explanation fallback.
  The overlay does not render a full plan-step list.
- `Connect` closes the overlay stack and uses the existing thread selection path.
- `Close` asks for confirmation and then submits `Shutdown` to that agent thread. It does not use
  `thread/unsubscribe`.
- The overlay shows effective cwd only from existing `Thread.cwd` / session cwd surfaces.
- The overlay may show thread note as secondary metadata when `thread-note` has provided
  `Thread.threadNote` or `CollabAgentState.threadNote`.
- Persona is shown in the row label as `(persona)` when present and not `default`; policy/template
  metadata is not projected.

## Empty/Error States

- Empty projection shows an empty `A G E N T S` overlay instead of falling back to `/agent`.
- If `thread/loaded/list` fails, the overlay opens with locally known rows and a non-blocking
  degraded-data state.
- If `thread/read` fails for one loaded id, the overlay keeps locally known row data and marks the
  projection degraded.
- `Close` is rendered only as a confirmed `Shutdown` operation. Do not simulate close with
  `thread/unsubscribe`.

## Integration And Compatibility

- TUI-only in v1.
- Uses existing app-server APIs: `thread/loaded/list`, `thread/read`, notifications, `Thread`, `ThreadItem`.
- Uses upstream 0.130 native thread switching/replay. It does not depend on
  `agent-switch-viewport`, which is deferred / obsoleted for `fork/130`.
- Does not simulate close with `thread/unsubscribe`.
- Displays persona when it is already available from existing thread metadata; does not require
  policy projection from `agent-role-templates`.
- Consumes `Thread.threadNote`/`CollabAgentState.threadNote` if `thread-note` is implemented.
  `threadNote` absent means no update/unknown, `null` clears stale note, and string replaces note.
- `threadNote` is secondary metadata: it must not affect identity, ordering, selection, or tree
  shape.
- `subagent-cwd` does not add overlay-specific app-server fields; effective cwd is read from
  existing thread/session surfaces.
- The feature itself must not create app-server schema, generated JSON/TypeScript, Codex app, or
  `experimentalApi` changes.

## Verification Matrix

| Surface | Required coverage |
| --- | --- |
| Projection | flat/nested tree, primary/side exclusions, current marker, cwd/status/prompt/model/persona extraction |
| Metadata | threadNote absent/null/string semantics, no policy dependency |
| Routing | transcript -> agents -> transcript, close stack, suspended transcript updates, backtrack isolation |
| Actions | action menu, inspect toggles details, connect uses existing selection path, close confirms then submits `Shutdown` |
| Plan summary | hidden by default; inspect-only summary from `TurnPlanUpdated` or legacy `Plan` item |
| Snapshots | empty, degraded data, flat, nested, selected, expanded, running, completed, errored, narrow |
| Compatibility | no app-server schema artifacts in v1 |

## Doc Changelog

- 2026-05-12: Initial `fork/130` contract.
- 2026-05-12: Locked V1 metadata, routing, empty/error state, and app-server non-impact rules.
- 2026-05-15: Switched activation contract to `Ctrl-T` cycle and removed the
  `agent-switch-viewport` dependency.
- 2026-05-15: Marked implemented for `fork/130`.
- 2026-05-15: Updated contract to match `fork/118` action menu, confirmed `Shutdown`, persona label,
  live details, and inspect-only compact plan summary behavior.
