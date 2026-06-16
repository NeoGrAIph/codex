# Thread notes design

## Canonical state

Thread notes are metadata, not model context. The canonical spawned-agent value lives in `SubAgentSource::ThreadSpawn.thread_note` and is copied into `SessionMeta.thread_note` for rollout persistence.

## Data flow

`spawn_agent.thread_note` is trimmed and normalized; empty values become `None`, values above 500 characters fail-fast. `thread_spawn_source` attaches the note to `SessionSource`. `AgentControl::prepare_thread_spawn` carries the note into live `AgentMetadata`. The rollout recorder writes `SessionMeta.thread_note`, and thread-store metadata sync/read models preserve it where the existing metadata path has a field.

## Invariants

- Missing note in old sessions deserializes as `None`.
- Notes are not injected into prompts, developer instructions, shell environment, or tool context.
- Live list projection returns note as nullable `thread_note`.
- No sqlite migration is required for this iteration.

## Tradeoffs

The sqlite-backed metadata index does not gain a column in this stage, so rollout/session metadata is the durable source for restart-safe note reconstruction. App-server and TUI surfaces remain documented follow-up work.
