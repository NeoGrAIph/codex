# Project: subagent-cwd

## Status

Implemented for `fork/130` on top of `rust-v0.130.0`.

## Canonical Links

- Feature contract: `../../features/subagent-cwd.md`
- Release research: `../../research/0.130.0/subagent-cwd.md`
- Design: `design.md`
- Verification: `verification.md`

## Goal

Add optional `cwd` to model-facing `spawn_agent` so a child agent can start from an explicit
session root while preserving upstream behavior when omitted.

## Implementation Map

- Tool args/schema:
  - `codex-rs/core/src/tools/handlers/multi_agents/spawn.rs`
  - `codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs`
  - `codex-rs/core/src/tools/handlers/multi_agents_spec.rs`
- Shared spawn config and cwd resolution:
  - `codex-rs/core/src/tools/handlers/multi_agents_common.rs`
  - `codex-rs/core/src/config/mod.rs`
  - `codex-rs/utils/path-utils/src/lib.rs`
  - `codex-rs/utils/absolute-path/src/lib.rs`
- Runtime orchestration:
  - `codex-rs/core/src/agent/control.rs`
  - `codex-rs/core/src/thread_manager.rs`
  - `codex-rs/core/src/environment_selection.rs`
  - `codex-rs/core/src/shell_snapshot.rs`
- Persistence/resume:
  - `codex-rs/core/src/session/session.rs`
  - `codex-rs/protocol/src/protocol.rs`
  - `codex-rs/thread-store/src/types.rs`
- App-server observation-only compatibility:
  - `codex-rs/app-server-protocol/src/protocol/v2/thread_data.rs`
  - `codex-rs/app-server-protocol/src/protocol/v2/thread.rs`

## Locked Decisions

- Explicit `cwd` is rejected with forked history in v1.
- Effective `cwd` is not returned in model-facing spawn output.
- Hidden metadata mode keeps `cwd` available as an input field.
- Explicit `cwd` rebuilds child config before role config is applied.
- Explicit `cwd` preserves runtime policy selections and rebases the current legacy-compatible
  permission intent to the child cwd instead of copying parent concrete ACLs.
- Explicit `cwd` uses `environments: None` so environment selections are child-rooted.
- Explicit `cwd` does not reuse parent cwd-bound shell snapshot or exec policy as concrete child
  state.
- No app-server schema change in v1.

## Implementation Notes

- MultiAgentV2 defaults omitted `fork_turns` to `all`, so explicit `cwd` must require
  `fork_turns: "none"`.
- The resolver must resolve relative paths against parent `turn.cwd`, not against process cwd.
- `AbsolutePathBuf` is not enough for validation because it does not guarantee that the path exists
  or is a directory.
- `Config::rebuild_preserving_session_layers()` is useful reference code, but it preserves
  `self.cwd`; implement a child-cwd variant or seed it with the resolved child cwd deliberately.
- Descendant resume must rebuild each child from its stored session cwd instead of reusing the root
  resume config for the whole spawn tree.
- Shell snapshot and exec policy inheritance must be audited in `AgentControl`; different child cwd
  means parent cwd-bound snapshot/policy is stale unless rebuilt.

## Non-Goals

- Do not add cwd to `ThreadItem::CollabAgentToolCall`.
- Do not change `CollabAgentSpawnBeginEvent` or `CollabAgentSpawnEndEvent`.
- Do not regenerate app-server schemas for v1.
- Do not add TUI display or viewport behavior for v1.
