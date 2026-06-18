# Thread notes project

## Status

`fork/140` first iteration implemented as metadata-only note support for MAv2, with spawn-time notes, post-spawn app-server update/clear through `thread/metadata/update.threadNote`, dedicated app-server `thread/note/updated`, model-visible MAv2 `set_thread_note`, app-server `Thread.threadNote` and `CollabAgentState.threadNote` read projections, and `/agent` workbench selected-detail rendering through existing thread source metadata.

## Canonical links

- Feature contract: `docs/fork/features/thread-notes.md`
- Shared architecture research: `docs/fork/research/multi-agents/architecture.md`
- Implementation: `codex-rs/protocol/src/protocol.rs`, `codex-rs/core/src/agent/control.rs`, `codex-rs/core/src/tools/handlers/multi_agents_v2/set_thread_note.rs`, `codex-rs/thread-store/src/types.rs`, `codex-rs/thread-store/src/local/update_thread_metadata.rs`, `codex-rs/app-server-protocol/src/protocol/v2/thread.rs`, `codex-rs/app-server/src/request_processors/thread_processor.rs`, `codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs`

## Implementation map

- Source of truth: `SessionSource::SubAgent(SubAgentSource::ThreadSpawn { thread_note })` and rollout `SessionMeta.thread_note`.
- Producers: MAv2 `spawn_agent.thread_note`, MAv2 `set_thread_note.thread_note`, and app-server `thread/metadata/update.threadNote`, normalized to a non-empty string up to 500 characters.
- Consumers: child session source, live `AgentMetadata`, MAv2 `list_agents`, MAv2 `wait_agent` visible-agent snapshot, thread-store read model/patch, app-server `Thread.threadNote`, app-server `CollabAgentState.threadNote` in collab tool-call notifications/history, app-server `thread/note/updated`, `/agent` workbench selected-detail `Note:` row.
- Intentionally unaffected: model/developer/environment context and sqlite schema. The dedicated app-server `thread/note/updated` notification is implemented as a projection of the existing metadata update path, not as a separate note store.

## Current coverage

Notes are visible through live list projection and `wait_agent` visible-agent snapshots, persisted in rollout/session metadata, editable/clearable after spawn through native app-server thread metadata update and model-visible MAv2 `set_thread_note`, exposed as top-level app-server `Thread.threadNote`, carried in `CollabAgentState.threadNote` for existing collab tool-call notifications/history, emitted as `thread/note/updated` after app-server set/clear, and rendered in the TUI workbench detail when app-server thread metadata exposes either `Thread.threadNote` or `SessionSource::SubAgent(ThreadSpawn.thread_note)`.
