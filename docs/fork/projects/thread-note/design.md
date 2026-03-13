# Thread Note Design

## Canonical State

`thread_note` is restart-safe thread metadata. For a running session, the active value lives in
session configuration. For restart/resume and other closed-thread recovery flows, the canonical
persisted value lives in append-only `thread_note_index.jsonl`. The current fork state does not
project note into sqlite or app-server read/list surfaces.

Projections derived from that canonical state:

- collaboration event snapshots used by transcript replay;
- top-level thread metadata returned to thread-management flows.

`thread_note` is not canonical in:

- developer instructions;
- environment context XML;
- ad hoc TUI state;
- transcript history rows.

## Data Flow

1. A thread starts with optional `thread_note` metadata.
2. Plain-text note input is normalized to `Назначение: <text> | Компетенции:`. Structured input is
   normalized to the same canonical one-line format.
3. Spawn/update paths propagate the current note through session metadata and collaboration event
   payloads.
4. TUI transcript rows render `Note:` from event payload snapshots, not from live lookups during
   replay.
5. `set_thread_note` appends the normalized value into `thread_note_index.jsonl` before mutating
   live session state.
6. Restart/resume reconstruction prefers the latest note-index entry. If the index does not have a
   value, it falls back to the in-memory snapshot and then to the note embedded in
   `session_source`.
7. App-server `thread/note/set` and note read/list APIs remain out of scope in this release
   adaptation.

## Key Invariants

- Empty or whitespace-only values are treated as absent notes.
- Transcript replay must remain stable even if the note later changes.
- `thread_note` must stay separate from `thread name`; they are different metadata fields with
  different UX roles.
- `thread_note` must stay separate from `agent_role`, `agent_persona`, and `agent_nickname`; it is
  the narrower specialization/competency profile of a specific thread, not the broad role class,
  prompt template, or display identity.
- `thread_note` uses a one-line canonical format with two sections: `Назначение:` and
  `Компетенции:`.
- `thread_note` must remain metadata-only; it must not be injected into model-facing prompt
  surfaces, because note text can otherwise become accidental behavioral guidance.
- `thread_note` must not be used for transient state like `waiting`, `idle`, or per-turn task
  descriptions; those belong to status surfaces and `send_input`.
- When a thread updates its own competencies, it should preserve `Назначение:` and change only
  `Компетенции:`.

## Intentional Tradeoffs

- The append-only note index keeps restart-safe persistence localized to core and avoids
  reintroducing the wider sqlite/app-server note surface from older fork revisions.
- Transcript rendering prefers event snapshots so history remains faithful to the state at emission
  time.
- Keeping note out of model context avoids the class of failures where note content becomes an
  implicit instruction when the actual user prompt is empty, technical, or underspecified.
- Keeping note out of app-server thread read/list APIs avoids coupling the persistence fix to a
  larger wire-contract rollback.
