# Subagent CWD Verification

## Core scenarios

- Spawn without `cwd` keeps the current child session directory behavior.
- Spawn with a relative `cwd` resolves from the parent `turn.cwd`.
- Spawn with an absolute `cwd` outside the parent workspace succeeds under policy B.
- The child session config uses the target cwd for cwd-sensitive config loading.
- A bad target path fails fast and does not create a child thread under the parent cwd instead.
- The effective cwd is present in tool output, collab events, and replay/history surfaces.

## Validation commands

- `cd codex-rs && cargo test -p codex-tools`
- `cd codex-rs && cargo test -p codex-core multi_agents`
- `cd codex-rs && cargo test -p codex-app-server-protocol`
- `cd codex-rs && cargo test -p codex-tui multi_agents`
- `cd codex-rs && just write-app-server-schema`

## Manual checks

- Inspect a spawned child thread and confirm the recorded cwd matches the requested target.
- Confirm TUI spawn rows show the effective child cwd when available.
- Confirm replay/history uses the persisted cwd rather than a live lookup.

## Known gaps

- This feature does not change `thread/start`, `thread/resume`, or `thread/fork`.
- This feature does not create directories automatically.
- The contract assumes cwd resolution and validation reuse the existing absolute-path rules.
