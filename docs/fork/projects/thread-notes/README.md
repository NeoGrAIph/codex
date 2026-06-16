# Thread notes project

## Status

`fork/140` first iteration implemented as metadata-only spawn-time note support for MAv2.

## Canonical links

- Feature contract: `docs/fork/features/thread-notes.md`
- Shared architecture research: `docs/fork/research/multi-agents/architecture.md`
- Implementation: `codex-rs/protocol/src/protocol.rs`, `codex-rs/core/src/agent/control.rs`, `codex-rs/thread-store/src/types.rs`, `codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs`

## Implementation map

- Source of truth: `SessionSource::SubAgent(SubAgentSource::ThreadSpawn { thread_note })` and rollout `SessionMeta.thread_note`.
- Producers: MAv2 `spawn_agent.thread_note`, normalized to a non-empty string up to 500 characters.
- Consumers: child session source, live `AgentMetadata`, MAv2 `list_agents`, thread-store read model/patch.
- Intentionally unaffected: model/developer/environment context, app-server `threadNote`, TUI rendering, `set_thread_note`, wait-agent output, sqlite schema.

## Current gaps

Notes are visible through live list projection and persisted in rollout/session metadata, but not yet editable after spawn and not projected through app-server/TUI.
