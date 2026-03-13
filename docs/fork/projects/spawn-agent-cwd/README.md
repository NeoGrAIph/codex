# Spawn Agent CWD Project

This dossier describes the `spawn-agent-cwd` fork feature for `fork/114`.

Canonical links:

- Feature contract: [docs/fork/features/spawn-agent-cwd.md](../../features/spawn-agent-cwd.md)
- Release research: [docs/fork/research/0.114.0/spawn-agent-cwd.md](../../research/0.114.0/spawn-agent-cwd.md)

Current status:

- `spawn_agent` accepts an optional `cwd` override;
- spawned threads still inherit parent runtime state by default;
- `cwd` override is validated fail-fast before spawn;
- override changes runtime config only and does not expand persisted thread source metadata.

Implementation surfaces:

- `core`: spawn args parsing, cwd resolution, runtime config override, tests;
- `tools spec`: `spawn_agent` schema/discovery text;
- `docs/fork`: contract, design notes, and verification checklist.

Read this folder in order:

1. [design.md](design.md)
2. [verification.md](verification.md)
