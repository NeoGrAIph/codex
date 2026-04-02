# `subagent-cwd` research for `rust-v0.118.0`

- Baseline tag: `rust-v0.118.0`
- Baseline commit: `b630ce9a4e754d35a1f33e4366ba638d18626142`
- Research target: a fork-specific `spawn_agent.cwd` contract that makes the child agent start from a separate session directory
- Scope: `protocol`, `core`, `app-server-protocol`, `tui`, and fork docs
- Research state: implemented `fork/118` feature contract for this release baseline

## Executive summary

- The upstream `rust-v0.118.0` baseline does not expose a child-session cwd override on `spawn_agent`.
- The fork contract chooses policy B: the child may start from any valid path on disk.
- The child cwd must be resolved before spawn and then carried through session bootstrap, runtime
  metadata, and replay/history.
- The feature is intentionally fail-fast: invalid target cwd should surface as a spawn error, not as a fallback to the parent cwd.

## Upstream gap

The upstream baseline only gives `spawn_agent` the parent turn context. It does not provide a
documented contract for selecting a different child session root at spawn time.

## Fork adaptation

The fork adapts the feature by:

- adding optional `spawn_agent.cwd`
- resolving relative child paths from the parent `turn.cwd`
- rebuilding effective child config from the requested cwd before spawn
- persisting the effective child cwd in the spawned thread and collab/history surfaces
- keeping the parent-session behavior unchanged when `cwd` is omitted

## Conflict-prone files

- `codex-rs/tools/src/agent_tool.rs`
- `codex-rs/core/src/tools/handlers/multi_agents/{spawn,multi_agents_common}.rs`
- `codex-rs/core/src/agent/control.rs`
- `codex-rs/core/src/thread_manager.rs`
- `codex-rs/protocol/src/protocol.rs`
- `codex-rs/app-server-protocol/src/protocol/{thread_history,v2}.rs`
- `codex-rs/tui/src/{multi_agents,app}.rs`

## Release-specific verification

- `cd codex-rs && cargo test -p codex-tools`
- `cd codex-rs && cargo test -p codex-core multi_agents`
- `cd codex-rs && cargo test -p codex-app-server-protocol`
- `cd codex-rs && cargo test -p codex-tui multi_agents`
