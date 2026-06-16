# Agent runtime limits project

## Status

`fork/140` first iteration implemented as native config default tuning.

## Canonical links

- Feature contract: `docs/fork/features/agent-runtime-limits.md`
- Shared architecture research: `docs/fork/research/multi-agents/architecture.md`
- Implementation: `codex-rs/core/src/config/mod.rs`

## Implementation map

- Source of truth: `Config` defaults and `MultiAgentV2Config`.
- Producers: default config construction and feature config merge.
- Consumers: `AgentExecutionLimiter`, spawn depth checks, `wait_agent` timeout options, tool usage hints derived from config.
- Intentionally unaffected: user config overrides, hard max wait timeout, sandbox/permissions.

## Current gaps

No dynamic runtime override or UI display change is included; this is only default tuning.
