# Thread Note on rust-v0.114.0

## Baseline

- Upstream baseline: `rust-v0.114.0`
- Fork branch: `fork/114`

## Gap Summary

The release already contains collaboration transcript entries for spawned agents and follow-up
messages, but it does not expose an optional note line in those transcript rows. `Spawned ...`
already carries note-capable metadata in the fork workstream, while `Sent input to ...` needs an
explicit note snapshot on the interaction end event so replay stays stable.

The release also does not persist `thread_note` across restart/resume, and it does not expose an
optional `Note:` line for sent-input transcript rows. The fork adaptation needs those behaviors,
but note must remain metadata-only rather than model-visible prompt text.

## Upstream-shaped adaptation

The adaptation for `fork/114` keeps the existing transcript structure and extends only the payload
and current-session context surfaces needed by the fork:

- add `receiver_thread_note` to `CollabAgentInteractionEndEvent`
- preserve note metadata when spawning or rehydrating thread-spawn agents
- render `Note:` conditionally in TUI before the prompt preview
- normalize note values to `Назначение: ... | Компетенции: ...`
- persist note through an append-only `thread_note_index.jsonl` so restart/resume restores the
  latest value without reintroducing sqlite note metadata
- keep app-server note mutation/read surfaces out of scope instead of re-expanding the fork API

This intentionally does not reuse the older `fork/107` model-visible channel for note semantics.
That older path made note part of prompt-visible session guidance. In `fork/114`, `thread_note`
stays as orchestration/runtime metadata because prompt-visible note text can become accidental
behavioral instruction when the actual user prompt is empty, technical, or underspecified.

## Verification

- `cargo check --tests -p codex-protocol -p codex-core -p codex-tui`
- `cargo test -p codex-protocol`
- `cargo test -p codex-core send_input_records_receiver_thread_note_in_collab_end_event`
- `cargo test -p codex-core spawn_agent_preserves_thread_note_in_result_and_session_source`
- `cargo test -p codex-core build_settings_update_items_ignores_thread_note_changes_for_model_context`
- `cargo test -p codex-core build_initial_context_omits_thread_note_from_developer_context`
- `cargo test -p codex-tui multi_agents`
