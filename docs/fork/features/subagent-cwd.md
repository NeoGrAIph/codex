# Subagent CWD

## Feature passport

- Code name: `subagent-cwd`
- Status: `implemented` on `fork/118`
- Goal: allow `spawn_agent` to place a child agent in a separate session directory so the child runs with its own effective workspace root instead of always inheriting the parent turn directory
- Scope in:
  - `spawn_agent` tool input and output contract
  - sub-agent session bootstrap and config rebuild
  - collab spawn events and thread history projection
  - TUI rendering of spawned-agent metadata
  - fork documentation for the release baseline
- Scope out:
  - `thread/start`, `thread/resume`, and `thread/fork`
  - automatic directory creation
  - silent fallback to the parent `turn.cwd`
  - repo-only restrictions on the new child cwd

## User contract

- `spawn_agent` accepts an optional `cwd`.
- If `cwd` is omitted, the child agent keeps the current behavior and starts from the parent turn directory.
- If `cwd` is relative, it resolves against the parent `turn.cwd`.
- If `cwd` is absolute, it is used directly.
- Policy B applies: the child session may start in any valid path on disk, not only inside the current workspace.
- The resolved child cwd becomes the child session root, not just a command-level override.
- If the target cwd cannot be resolved or cannot produce a valid child config, `spawn_agent` fails fast and does not fall back to the parent cwd.
- The effective child cwd must be visible in:
  - the `spawn_agent` tool result
  - the collab spawn begin/end events
  - the spawned thread metadata and history replay surfaces

## Integration and compatibility notes

- Upstream behavior remains unchanged for callers that do not pass `cwd`.
- The feature is fork-specific because upstream `spawn_agent` does not expose an explicit child session-directory override.
- The child cwd is intentionally broader than the parent workspace boundary; this is a documented divergence, not a compatibility shim.
- The child cwd must be rebuilt as part of effective session config so cwd-scoped project config, trust, skills, and plugin discovery remain consistent with the chosen path.

## Verification matrix

- `cd codex-rs && cargo test -p codex-tools`
- `cd codex-rs && cargo test -p codex-core multi_agents`
- `cd codex-rs && cargo test -p codex-app-server-protocol`
- `cd codex-rs && cargo test -p codex-tui multi_agents`
- `cd codex-rs && just write-app-server-schema`

Each command validates one of the contract surfaces:

- `codex-tools` covers the `spawn_agent` schema and output shape.
- `codex-core` covers child config rebuild and agent-spawn behavior.
- `codex-app-server-protocol` covers wire types and thread-history projection.
- `codex-tui` covers transcript rendering and runtime event handling.
- `just write-app-server-schema` ensures generated protocol artifacts stay aligned.

## Doc changelog

- 2026-04-02: Implemented `spawn_agent.cwd` across tool schema, session bootstrap, replay/history, and TUI surfaces for `fork/118`.
