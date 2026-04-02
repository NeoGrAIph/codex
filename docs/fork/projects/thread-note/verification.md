# Thread Note Verification

## Scenarios

The feature is healthy when these scenarios remain true:

- spawning without `thread_note` does not create a note value and does not render `Note:`
- spawning with plain-text note input returns canonical formatting
- structured note input preserves canonical sections with normalized spacing
- `set_thread_note` updates and clears note values deterministically
- spawned and sent-input transcript rows show `Note:` only when the event snapshot carries it
- transcript replay stays stable after later note updates
- restart/resume restores note from rollout-backed persistence before falling back to the legacy
  note index
- note changes do not inject model-facing context
- app-server collaboration history reconstruction preserves thread note in `CollabAgentState`
- top-level app-server thread payloads continue not to expose `thread_note`

## Validation commands

- `cd codex-rs && cargo test -p codex-protocol`
- `cd codex-rs && cargo test -p codex-rollout`
- `cd codex-rs && cargo test -p codex-core multi_agents`
- `cd codex-rs && cargo test -p codex-tui multi_agents`

## Focused checks

- `spawn_agent` with `"thread_note": "Repository researcher"` returns
  `Назначение: Repository researcher | Компетенции:`
- `set_thread_note` with empty input returns `{ "thread_note": null }`
- resumed spawned threads keep the last persisted note
- `CollabAgentInteractionEndEvent.receiver_thread_note` is reflected in transcript rendering
- app-server history reducers populate `CollabAgentState.thread_note` for collaboration rows

## Runtime checklist

1. Spawn an agent without a note.
   Expected: no `Note:` line in the `Spawned ...` row.

2. Spawn an agent with a plain-text note.
   Expected: normalized one-line note is returned and rendered.

3. Update the note through `set_thread_note`.
   Expected: returned note is normalized and later `Sent input to ...` rows use the updated
   snapshot.

4. Clear the note.
   Expected: future collaboration rows stop rendering `Note:`.

5. Restart or resume the thread.
   Expected: note is restored from persisted metadata without becoming model-visible context.

## Coverage notes

- `codex-protocol` protects normalization and note-bearing event/session shapes
- `codex-rollout` protects session meta persistence and legacy note-index fallback
- `codex-core` protects spawn/update propagation and resume behavior
- `codex-tui` protects the user-visible transcript rows
- a full workspace test run remains a separate decision because this feature touches several crates
