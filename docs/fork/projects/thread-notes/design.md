# Thread notes design

## Canonical state

Thread notes are metadata, not model context. The canonical spawned-agent value lives in `SubAgentSource::ThreadSpawn.thread_note` and is copied into `SessionMeta.thread_note` for rollout persistence.

## Data flow

`spawn_agent.thread_note` is trimmed and normalized; empty values become `None`, values above 500 characters fail-fast. `thread_spawn_source` attaches the note to `SessionSource`. `AgentControl::prepare_thread_spawn` carries the note into live `AgentMetadata`. `list_agents` and `wait_agent` both project visible-agent metadata from `AgentControl::list_agents`; `wait_agent` adds that snapshot to its result without reading mailbox content or final answers. The rollout recorder writes `SessionMeta.thread_note`, and thread-store metadata sync/read models preserve it where the existing metadata path has a field. For post-spawn app-server updates, `thread/metadata/update.threadNote` uses the same shared normalization and updates rollout `SessionMeta.thread_note` plus nested `SessionSource::SubAgent(ThreadSpawn.thread_note)` through the native thread metadata update path; after a successful set/clear, app-server emits `thread/note/updated` with the normalized nullable note. For model-visible updates, MAv2 `set_thread_note` resolves a visible spawned sub-agent target, rejects root/non-sub-agent targets, applies the same normalization, writes through `AgentControl::set_thread_note` and `ThreadMetadataPatch`, then refreshes live `AgentMetadata` so `list_agents` and `wait_agent` immediately project the changed note. App-server `Thread.threadNote` is a read projection derived from that metadata, not a storage source. App-server `CollabAgentState.threadNote` is also a projection: spawn/send/wait/close/resume collab events carry the current note alongside existing status metadata so live notifications and replayed thread history use the same state shape. The TUI `/agent` workbench reads `Thread.threadNote` with fallback to app-server `Thread.source` hydration and renders the note as a bounded selected-detail recovery anchor; live `thread/note/updated` updates the same cache entry without fetching the thread again.

## Invariants

- Missing note in old sessions deserializes as `None`.
- Notes are not injected into prompts, developer instructions, shell environment, or tool context.
- Live list projection returns note as nullable `thread_note`.
- MAv2 `wait_agent` returns visible-agent snapshot metadata with nullable `thread_note`, but not mailbox/final-answer content.
- MAv2 `set_thread_note` may update only spawned sub-agent threads visible through the current agent tree; root and non-sub-agent threads are controlled errors.
- App-server thread read/list/resume/update responses return note as nullable `threadNote` projection.
- App-server collab tool-call notifications/history may include nullable `CollabAgentState.threadNote`; omitted values from old JSONL records remain `None`.
- App-server `thread/note/updated` is emitted only after a successful `thread/metadata/update.threadNote` set/clear and carries the normalized nullable note.
- App-server post-spawn updates must preserve omit/clear/set semantics: omitted `threadNote` leaves note unchanged, `null` or blank clears it, and a non-empty string up to 500 characters replaces it.
- TUI workbench detail may render the note, but it must treat missing note as an empty state and must not infer or synthesize one from transcript text.
- No sqlite migration is required for this iteration.

## Tradeoffs

The sqlite-backed metadata index does not gain a column in this stage, so rollout/session metadata and serialized `SessionSource` remain the durable source for restart-safe note reconstruction. Post-spawn app-server editing reuses `thread/metadata/update` instead of a dedicated app-server endpoint; model-visible MAv2 editing uses `set_thread_note` as a tool façade over the same metadata patch path. `Thread.threadNote`, `CollabAgentState.threadNote`, `thread/note/updated` and `wait_agent.agents[*].thread_note` are projections over the same metadata. The current TUI rendering is a projection over existing thread source metadata, not a separate note store.
