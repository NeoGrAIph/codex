# Thread Note Verification

## Scenarios

The feature is considered healthy when these scenarios stay true:

- spawned-agent transcript rows show `Note:` only when the spawn event carries a non-empty note;
- sent-input transcript rows show `Note:` only when the interaction end event carries a non-empty
  note snapshot;
- note snapshots remain stable during transcript replay even if the thread note changes later;
- plain-text note input is normalized to `Назначение: <text> | Компетенции:`;
- structured note updates preserve the canonical one-line format;
- tool descriptions and examples make it clear that `thread_note` is stable specialization/competency metadata, not temporary status or the current task;
- the model does not receive note content through initial context or settings update items;
- restart/resume reconstruction prefers the latest note-index entry and falls back to embedded
  `ThreadSpawn` metadata only when no persisted note exists;
- unloaded-thread and resume flows continue to read persisted note metadata correctly;
- note persistence remains a core-only concern; app-server thread read/list APIs do not expose it.

## Validation Commands

- `cd codex-rs && cargo check --tests -p codex-protocol -p codex-core -p codex-tui`
- `cd codex-rs && cargo test -p codex-protocol`
- `cd codex-rs && cargo test -p codex-core send_input_records_receiver_thread_note_in_collab_end_event`
- `cd codex-rs && cargo test -p codex-core spawn_agent_preserves_thread_note_in_result_and_session_source`
- `cd codex-rs && cargo test -p codex-core build_settings_update_items_ignores_thread_note_changes_for_model_context`
- `cd codex-rs && cargo test -p codex-core build_settings_update_items_ignores_thread_note_clear_for_model_context`
- `cd codex-rs && cargo test -p codex-core build_initial_context_omits_thread_note_from_developer_context`
- `cd codex-rs && cargo test -p codex-core find_thread_note_by_id_prefers_latest_entry`
- `cd codex-rs && cargo test -p codex-core find_thread_note_by_id_returns_none_after_clear`
- `cd codex-rs && cargo test -p codex-core resume_agent_restores_thread_note_from_index`
- `cd codex-rs && cargo test -p codex-tui multi_agents`

## Runtime Checklist

Use this checklist against a live Codex runtime when validating the end-to-end behavior of
`thread_note`.

### 1. Spawn without a note

1. Spawn an agent without `thread_note`.
2. Confirm that the `spawn_agent` result does not report a note.
3. Confirm that TUI does not render a `Note:` line in the `Spawned ...` transcript entry.
4. Confirm that initial model context for that thread does not contain note metadata.

### 2. Spawn with plain-text note input

1. Spawn an agent with plain-text `thread_note`, for example `Repository researcher`.
2. Confirm that the tool result returns the normalized value:
   `Назначение: Repository researcher | Компетенции:`.
3. Confirm that the `Spawned ...` transcript entry renders the normalized note, not the raw
   plain-text input.
4. Confirm that initial model context for the spawned thread still does not contain note text.

### 3. Update competencies without changing purpose

1. Starting from a thread that already has a note, call `set_thread_note` with the same
   `Назначение:` and a richer `Компетенции:` section.
2. Confirm that the returned note preserves the original `Назначение:`.
3. Send a new prompt to the agent.
4. Confirm that the next `Sent input to ...` transcript entry shows the updated note snapshot.
5. Confirm that the note change does not create a model-facing context update item.

### 4. Update purpose when specialization really changes

1. Call `set_thread_note` with a different `Назначение:` and a matching `Компетенции:` section.
2. Confirm that the returned value uses canonical formatting.
3. Confirm that transcript entries after the update use the new note snapshot.
4. Confirm that the model context still omits note text after the update.

### 5. Close and resume

1. Spawn an agent with a non-empty note.
2. Optionally update the competencies so the note is not just the initial seed.
3. Close the agent thread.
4. Resume the same thread.
5. Confirm that:
   - the resumed thread still has the same note;
   - TUI continues to show the note for subsequent collaboration events;
   - the resumed model context still omits note text.

### 6. Persistence and unloaded-thread paths

1. With a thread note set, stop using the thread so that its metadata must be read from persisted
   state rather than only from live runtime memory.
2. Resume the thread after it has been unloaded or restarted.
3. Confirm that the same note is restored from persisted metadata.
4. Confirm that persisted note restoration does not reintroduce the note into developer context.

### 7. Negative checks

1. Set the note to an empty or whitespace-only value.
2. Confirm that the note is cleared.
3. Confirm that TUI no longer renders `Note:` for later events from that thread.
4. Confirm that no model-facing clear/update item is emitted for note removal.
5. Confirm that one agent does not automatically answer using another agent's note unless that note
   was explicitly surfaced through some other orchestration context.

## Coverage Notes

- `protocol` tests protect wire compatibility for the note-bearing payloads.
- focused `core` tests protect note propagation, persistence, and the absence of model-context
  injection.
- `tui` snapshot coverage protects the user-visible `Note:` rows.
- A full workspace `cargo test` remains a separate, explicit decision because `core` and `protocol`
  changes make it comparatively expensive.
