# Subagent CWD Project

This dossier describes how `subagent-cwd` is implemented in the current `fork/118` workstream and
where to look when the feature changes.

Canonical references:

- Feature contract: [../../features/subagent-cwd.md](../../features/subagent-cwd.md)
- Release research: [../../research/0.118.0/subagent-cwd.md](../../research/0.118.0/subagent-cwd.md)

Current status:

- `spawn_agent` accepts an optional child `cwd`
- the child cwd is treated as a new session root, not a shell-only override
- the effective child cwd must be visible in tool output, collab events, and replay/history surfaces
- policy B allows any valid on-disk target path

Implementation surfaces:

- `protocol`: tool schema and spawn-event payloads
- `core`: child config rebuild, agent spawn flow, runtime inheritance
- `app-server-protocol`: thread-history projection and collab state persistence
- `tui`: spawned-agent transcript rendering
- `docs/fork`: feature contract, project dossier, and release research

Read this folder in order:

1. [design.md](design.md) for canonical state, data flow, and invariants
2. [verification.md](verification.md) for scenario coverage and validation commands
