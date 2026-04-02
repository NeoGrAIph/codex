# Thread Note

## Feature passport

- Code name: `thread-note`
- Status: `implemented` in current `fork/118` staged state
- Goal: preserve a stable metadata-only note for a thread so collaboration tooling, transcript
  rendering, and restart/resume flows can keep the thread's narrow specialization visible without
  turning that metadata into prompt instructions
- Scope in:
  - `codex-rs/protocol/src/thread_note.rs`
  - `codex-rs/protocol/src/protocol.rs`
  - `codex-rs/core/src/codex.rs`
  - `codex-rs/core/src/tools/handlers/multi_agents/{spawn,send_input,set_thread_note}.rs`
  - `codex-rs/rollout/src/{recorder,session_index}.rs`
  - `codex-rs/app-server-protocol/src/protocol/{v2,thread_history}.rs`
  - `codex-rs/app-server/src/bespoke_event_handling.rs`
  - `codex-rs/tui/src/{chatwidget,multi_agents}.rs`
- Scope out:
  - broad prompt/persona shaping
  - task-local status text
  - top-level app-server thread metadata surfaces
  - a new fork-specific RPC dedicated to note mutation

## User contract

- `spawn_agent` accepts optional `thread_note`.
- Plain-text note input is normalized to the canonical one-line format:
  `Назначение: <text> | Компетенции:`
- Structured input in the same two-section format is preserved and spacing-normalized.
- Empty or whitespace-only input clears the note.
- `spawn_agent` returns the normalized `thread_note` together with the new agent id and nickname.
- `set_thread_note` updates or clears the current agent thread note and returns the normalized value
  as `{ "thread_note": <string|null> }`.
- `thread_note` is distinct from:
  - `thread_name`: display title for the thread
  - `agent_role`: broad reusable role class
  - `agent_nickname`: human-friendly display identity
- Recommended semantics:
  - `Назначение:` is the stable role of the specific thread
  - `Компетенции:` is the evolving specialization/skill list for that thread

## Transcript and runtime behavior

- TUI renders `Note:` only when the relevant collaboration event carries a non-empty note snapshot.
- `Spawned ...` rows render from `CollabAgentSpawnEndEvent.new_thread_note`.
- `Sent input to ...` rows render from `CollabAgentInteractionEndEvent.receiver_thread_note`.
- Transcript replay uses event snapshots or cached collaboration metadata. It does not perform a
  live note lookup during replay.
- App-server v2 collaboration history surfaces currently expose `thread_note` inside
  `CollabAgentState` so downstream history reconstruction can preserve the same metadata.

## Metadata-only contract

- `thread_note` is runtime metadata, not a model instruction.
- Codex must not inject `thread_note` into developer instructions.
- Codex must not inject `thread_note` into `environment_context`.
- Changing or clearing the note must not create a model-facing settings update item.
- The note is allowed to appear in:
  - spawn/send-input collaboration events
  - session and rollout metadata
  - restart/resume recovery logic
  - transcript rendering
  - app-server collaboration state used for thread history reconstruction

## Persistence and recovery

- Active sessions keep the live note in session configuration.
- Spawned threads persist the initial note in `SessionMeta.thread_note` and in nested
  `SessionSource::SubAgent(SubAgentSource::ThreadSpawn { thread_note, .. })`.
- Turn-level persistence carries `thread_note` in `TurnContextItem`.
- Restart/resume recovery prefers:
  1. latest persisted `TurnContext.thread_note` from rollout
  2. `SessionMeta.thread_note`
  3. legacy fallback from `thread_note_index.jsonl`
  4. embedded `ThreadSpawn.thread_note`
- `thread_note_index.jsonl` remains a legacy append-only fallback, not the primary source of truth
  for the current fork state.

## Integration and compatibility notes

- The fork intentionally diverges from older fork revisions that treated `thread_note` as
  prompt-visible guidance.
- The fork also diverges from older docs that documented `thread/note/set` or
  `thread/note/updated`; the current staged `fork/118` contract does not expose a dedicated
  app-server RPC for note mutation.
- All note-bearing protocol fields are optional for wire compatibility.
- Older payloads without `thread_note` remain valid and deserialize as absent metadata.
- Current app-server top-level `Thread` payloads do not advertise `thread_note`; note visibility is
  limited to collaboration/history state and runtime metadata surfaces listed above.

## Verification matrix

- `spawn_agent` without note does not render `Note:` and does not synthesize a value.
- `spawn_agent` with plain text normalizes to canonical `Назначение: ... | Компетенции:`.
- `set_thread_note` updates the current thread note and returns normalized output.
- `set_thread_note` clears the note on empty/whitespace input.
- `CollabAgentSpawnEndEvent` and `CollabAgentInteractionEndEvent` preserve note snapshots.
- TUI transcript rows render `Note:` only when snapshot metadata is present.
- Restart/resume restores note from rollout-backed persistence before legacy fallback.
- Model-facing context remains unchanged when note is set or cleared.
- App-server collaboration history reconstruction preserves `thread_note` in `CollabAgentState`.

## Doc changelog

- 2026-04-02: Re-established the `thread-note` fork contract under `docs/fork/*` for `fork/118`
  and aligned it with the current staged metadata-only, rollout-backed behavior.
