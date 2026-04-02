# Thread Note Design

## Canonical state

`thread_note` is restart-safe thread metadata.

For a running session, the live value is held in session configuration and mirrored into the
current reference turn context. For persisted recovery, the authoritative order is:

1. latest persisted `TurnContext.thread_note`
2. `SessionMeta.thread_note`
3. legacy `thread_note_index.jsonl`
4. embedded `SessionSource::SubAgent(SubAgentSource::ThreadSpawn { thread_note, .. })`

`thread_note` is not canonical in:

- developer instructions
- `environment_context`
- prompt templates
- ad hoc TUI-only state

## Data flow

1. A thread starts with optional `thread_note` metadata.
2. Input note text is normalized through `codex-rs/protocol/src/thread_note.rs`.
3. `spawn_agent` carries the normalized note into child session metadata and spawn-end event
   payloads.
4. `set_thread_note` updates live session configuration, updates the reference context item, and
   persists the note so restart/resume can reconstruct it.
5. Rollout persistence stores note in `SessionMeta` and `TurnContextItem`.
6. Legacy note-index persistence remains available as a fallback path.
7. App-server history reducers map note-bearing collaboration events into `CollabAgentState`.
8. TUI renders `Note:` from event snapshots or cached collab metadata instead of a live lookup.

## Key invariants

- Empty or whitespace-only values are treated as absence.
- Plain-text note input canonicalizes to `Назначение: <text> | Компетенции:`.
- The note remains one line and contains two conceptual sections: `Назначение:` and
  `Компетенции:`.
- `thread_note` must stay distinct from `thread_name`, `agent_role`, and `agent_nickname`.
- Transcript replay must remain stable even if the underlying note later changes.
- Note content must not become implicit behavioral guidance for the model.
- When only competencies evolve, `Назначение:` should remain stable.

## Intentional tradeoffs

- Rollout-backed restoration is preferred over the older note-index-only model because it keeps the
  durable value closer to the actual session transcript and turn state.
- The legacy note index is retained as a fallback to avoid breaking older persisted threads during
  migration.
- App-server currently exposes note in collaboration state only, not in top-level thread payloads,
  to keep the public wire contract narrower while preserving transcript/history fidelity.
- Snapshot-based TUI rendering is preferred so historical transcript rows remain faithful to the
  metadata visible at the time the event was emitted.
