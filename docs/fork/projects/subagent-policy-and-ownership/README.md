# Subagent policy and ownership project

## Status

`fork/140` first ownership slices implemented path-based MAv2 interrupt ownership and workbench/app-server close-one ownership. Durable policy metadata is not implemented yet.

## Canonical links

- Feature contract: `docs/fork/features/subagent-policy-and-ownership.md`
- Shared architecture research: `docs/fork/research/multi-agents/architecture.md`
- Implementation: `codex-rs/core/src/tools/handlers/multi_agents_v2/interrupt_agent.rs`, `codex-rs/core/src/thread_manager.rs`, `codex-rs/core/src/agent/control.rs`, `codex-rs/app-server/src/request_processors/thread_processor.rs`, `codex-rs/tui/src/app/session_lifecycle.rs`

## Implementation map

- Source of truth: `AgentMetadata.agent_path` in the root-scoped `AgentControl` registry plus current caller `SessionSource` / author thread source metadata.
- Producer: spawned thread registration through `AgentControl::prepare_thread_spawn`.
- Consumers: MAv2 `interrupt_agent` target resolution and ownership guard; app-server/TUI workbench `close one` through `ThreadManager::close_agent_from_workbench`.
- Intentionally unaffected: MCP tool allow/deny policy, role-template policy fields, V1 close/resume tools and app-server policy metadata schema.

## Current gaps

Role-template `read_only`/`allow_list`/`deny_list`, durable policy metadata propagation, app-server policy projection and V1 legacy close ownership parity remain follow-up work. Current positive coverage includes non-root interrupt of a descendant target inside the caller subtree and workbench/app-server close of an owned path-backed descendant.
