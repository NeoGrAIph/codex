# Feature: subagent-cwd

## Feature Passport

- Code name: `subagent-cwd`
- Status: implemented for `fork/130`
- Goal: let `spawn_agent` start a child agent from an explicit session root.
- Baseline: `rust-v0.130.0` / `58573da43ab697e8b79f152c53df4b42230395a8`.
- Scope in: model tool input, cwd resolution, validation, config rebuild, environment selection,
  shell snapshot / exec policy handling, persistence, and descendant resume.
- Scope out: `thread/start`, `thread/resume`, `thread/fork`, app-server schema changes,
  `ThreadItem::CollabAgentToolCall`, collab spawn events, Codex app client changes, and TUI UI.

## User Contract

- `spawn_agent.cwd` is an optional string path in both legacy v1 `spawn_agent` and
  MultiAgentV2 `spawn_agent`.
- Omitted `cwd` preserves upstream behavior: child agents inherit parent `turn.cwd`.
- Omit `cwd` by default. Set it only when the child agent must start in a different repository,
  worktree, or session root than the parent.
- Relative `cwd` resolves against parent `turn.cwd`.
- Absolute `cwd` is used directly.
- The resolved path may be outside the parent workspace.
- `..` path segments are allowed after normal path resolution.
- The resolved path must exist and be a directory.
- The directory is never created automatically.
- Invalid explicit `cwd` fails fast and never falls back to parent cwd.
- Explicit `cwd` is incompatible with forked history in v1:
  - legacy `fork_context: true` returns a controlled error;
  - MultiAgentV2 requires `fork_turns: "none"` when `cwd` is set because omitted
    `fork_turns` defaults to `all`.
- Effective cwd is not returned in model-facing `spawn_agent` output.
- Hidden metadata mode does not remove the `cwd` input field because `cwd` is a placement/runtime
  control, not agent metadata.

Critical model-facing error strings:

- Legacy fork conflict: `cwd cannot be combined with fork_context in this release.`
- MultiAgentV2 fork conflict: `cwd cannot be combined with forked history in this release; set fork_turns to "none" when using cwd.`
- Invalid path errors must include the `cwd` field name and distinguish empty path, missing path,
  not-a-directory, and stat/access failure. They must not say that Codex used the parent cwd.

Model-facing output remains unchanged:

- Legacy v1: `{ "agent_id": "...", "nickname": ... }`
- MultiAgentV2 with metadata: `{ "task_name": "...", "nickname": ... }`
- MultiAgentV2 hidden metadata: `{ "task_name": "..." }`

Canonical different-worktree spawn example:

```json
{
  "message": "coordinate this task",
  "task_name": "orchestrator_task",
  "cwd": "/home/neograiph/tasks",
  "fork_turns": "none"
}
```

## Integration And Compatibility

- No v1 app-server schema change.
- Existing `Thread.cwd` and session metadata expose effective cwd to app-server clients.
- Explicit cwd rebuilds child config before role config is applied.
- Explicit cwd preserves runtime model/session selections.
- Legacy-compatible current permission intent is rebased to the child cwd. For example, current
  workspace-write grants write access to the child cwd, not the parent cwd; current read-only
  remains read-only even if child config would otherwise allow workspace-write.
- Current permission profiles that cannot be safely projected through the legacy sandbox policy
  fail with a controlled error and do not fall back to parent cwd or child config permissions.
- Parent concrete runtime permission profiles are not copied as raw ACLs.
- Explicit cwd must not inherit parent shell snapshots or cwd-bound exec policy as concrete state.
  Shell snapshot and exec policy must either be rebuilt for the child cwd or intentionally omitted
  so the child session computes its own state.
- Explicit cwd passes `environments: None` so `ThreadManager` builds child-rooted environment
  selections.
- The feature must not auto-trust the child cwd.
- Resume must fail fast when a stored child cwd cannot be restored; it must not silently restart the
  child from the caller cwd.
- `SessionConfiguredEvent.cwd`, `SessionMeta.cwd`, `ThreadPersistenceMetadata.cwd`, and app-server
  `Thread.cwd` are the compatibility surfaces for observing the effective child cwd.
- Do not add cwd to `ThreadItem::CollabAgentToolCall` or app-server generated schemas in v1.

## Verification Matrix

| Surface | Required coverage |
| --- | --- |
| Tool schema | v1/v2 expose optional `cwd`; hidden metadata keeps input `cwd`; output unchanged |
| Resolution | omitted, relative to parent `turn.cwd`, absolute, `..`, outside-workspace |
| Failure | empty, nonexistent, file path, inaccessible path, config rebuild failure |
| Runtime | config cwd, turn cwd, sandbox/profile cwd, environment cwd |
| Runtime | current workspace-write rebases to child cwd and does not write parent cwd |
| Runtime | current read-only is preserved even when child config allows workspace-write |
| Runtime | shell snapshot and cwd-bound exec policy are not inherited from parent cwd |
| Config | child project config and role config load from child cwd |
| Forking | explicit cwd with forked history returns controlled error |
| Compatibility | no app-server schema or TUI changes for v1 |
| Resume | child and grandchild restore stored cwd after restart |

## Doc Changelog

- 2026-05-12: Clarified current permission intent rebasing for explicit cwd; no DB schema or OS
  permission changes are required; non-projectable current permission profiles fail fast.
- 2026-05-12: Clarified explicit cwd permission inheritance and resume fail-fast behavior.
- 2026-05-12: Implemented v1 core behavior with focused multi-agent coverage.
- 2026-05-12: Added shell snapshot and exec policy handling to the runtime contract.
- 2026-05-12: Clarified v1 fork incompatibility, output shape, hidden metadata behavior,
  app-server exclusions, and cwd-scoped config/runtime requirements.
- 2026-05-12: Initial `fork/130` contract.
