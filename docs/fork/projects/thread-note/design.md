# Design

## Canonical State

- Runtime source of truth: `SessionConfiguration.thread_note`
- Persistence source of truth: append-only `thread_note_index.jsonl`
- Spawn metadata source of truth: `SessionSource::SubAgent(SubAgentSource::ThreadSpawn { thread_note, .. })`

## Data Flow

- `spawn_agent.thread_note` is normalized in protocol helpers before child spawn.
- `set_thread_note` appends the normalized value to `thread_note_index.jsonl`, then updates live session state.
- Session init restores `thread_note` from index first, then falls back to embedded `SessionSource`.
- Collaboration events carry note snapshots:
  - `CollabAgentSpawnEndEvent.new_thread_note`
  - `CollabAgentInteractionEndEvent.receiver_thread_note`
- TUI renders `Note:` from those event snapshots.

## Invariants

- `thread_note` is metadata-only and must not enter prompt/developer/environment context.
- Clearing a note is represented by `None`.
- Public app-server v2 payloads must not expose `thread_note`.

## Tradeoffs

- The app-server protocol now owns its sub-agent source mapping instead of transparently reusing core enums. This keeps fork-only metadata internal without forking public API contracts.
