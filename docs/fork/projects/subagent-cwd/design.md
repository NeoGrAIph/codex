# Subagent CWD Design

## Canonical state

`subagent-cwd` is a child-session bootstrap feature.

The canonical value is the resolved child session cwd chosen at spawn time. It is not just a
command working directory and it is not a display-only field.

For spawned sub-agents:

1. The parent turn provides the base context.
2. `spawn_agent.cwd`, when present, selects the child session root.
3. Relative child paths resolve against the parent `turn.cwd`.
4. The child config is rebuilt from that target cwd.
5. Runtime inheritance is layered on top of the rebuilt config.
6. The resolved cwd is persisted into the spawned thread metadata and replay surfaces.

## Data flow

1. The model calls `spawn_agent` with `message` or `items`, plus optional `cwd`.
2. The handler normalizes the requested cwd and resolves it to an absolute path.
3. The core rebuilds effective config for the target cwd using the same cwd-sensitive layering used for ordinary session startup.
4. Runtime-only state is then copied from the parent turn into the child config, including model, reasoning, sandbox, approvals, shell-environment policy, and role-specific overrides.
5. `AgentControl` spawns the child thread with that rebuilt config.
6. The resulting thread snapshot is emitted back through tool output, collab events, and thread-history projections.

## Key invariants

- Omitting `cwd` preserves current behavior.
- Relative `cwd` resolution always uses the parent turn cwd as the base.
- A valid child cwd must produce a valid child config or the spawn fails.
- The implementation must not silently fall back to the parent cwd.
- The child session cwd must be visible everywhere the child thread identity is surfaced.
- Policy B is intentional: the child may start outside the parent workspace if the requested path is valid.

## Intentional tradeoffs

- The feature prefers correctness over a shallow override because cwd participates in config layering,
  trust, skills, plugins, sandboxing, and history semantics.
- The result/event/history surfaces carry the effective cwd so parent agents and UI clients do not
  need to infer it indirectly.
- No repo-bound restriction is imposed in this feature; that would be a different policy and a
  different contract.
