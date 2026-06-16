# Subagent policy and ownership project

## Status

`fork/140` first iteration implemented for MAv2 interrupt ownership. Durable policy metadata is not implemented yet.

## Canonical links

- Feature contract: `docs/fork/features/subagent-policy-and-ownership.md`
- Shared architecture research: `docs/fork/research/multi-agents/architecture.md`
- Implementation: `codex-rs/core/src/tools/handlers/multi_agents_v2/interrupt_agent.rs`, `codex-rs/core/src/agent/control.rs`

## Implementation map

- Source of truth: `AgentMetadata.agent_path` in the root-scoped `AgentControl` registry plus current caller `SessionSource`.
- Producer: spawned thread registration through `AgentControl::prepare_thread_spawn`.
- Consumers: MAv2 `interrupt_agent` target resolution and ownership guard.
- Intentionally unaffected: MCP tool allow/deny policy, role-template policy fields, V1 close/resume tools, app-server schema.

## Current gaps

Role-template `read_only`/`allow_list`/`deny_list`, durable policy metadata propagation, app-server policy projection, and V1 close ownership parity remain follow-up work.
