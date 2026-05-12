# subagent-cwd Verification

## Scenarios

| Surface | Scenario | Expected result |
| --- | --- | --- |
| Tool schema | Legacy v1 spawn schema | Optional `cwd` input is present; output schema unchanged |
| Tool schema | MultiAgentV2 spawn schema | Optional `cwd` input is present even when metadata fields are hidden; output schema unchanged |
| Omitted cwd | Spawn without `cwd` | Existing parent cwd and inherited environments are preserved |
| Relative cwd | Spawn with `cwd: "child"` | Resolved against parent `turn.cwd`, not process cwd |
| Relative cwd | Spawn with `cwd: "../sibling"` | Accepted when resolved directory exists, even outside parent workspace |
| Absolute cwd | Spawn with existing absolute directory outside workspace | Accepted and used as child session root |
| Failure | Empty `cwd` | Controlled error before child thread creation |
| Failure | Nonexistent `cwd` | Controlled error before child thread creation |
| Failure | File path `cwd` | Controlled error before child thread creation |
| Failure | Inaccessible/stat-failing `cwd` | Controlled error before child thread creation |
| Failure | Child config rebuild fails | Controlled error with no fallback to parent cwd |
| Failure | Current permission profile cannot project to legacy sandbox policy | Controlled error with no fallback to parent cwd or child config permissions |
| Forking | Legacy explicit cwd with `fork_context: true` | `cwd cannot be combined with fork_context in this release.` |
| Forking | MultiAgentV2 explicit cwd with omitted `fork_turns` | Controlled error because omitted defaults to `all` |
| Forking | MultiAgentV2 explicit cwd with `fork_turns: "all"` or positive integer | Controlled fork conflict error |
| Forking | MultiAgentV2 explicit cwd with `fork_turns: "none"` | Accepted when cwd is valid |
| Runtime | Child first turn | `Config.cwd`, `TurnContext.cwd`, and effective session cwd equal resolved child cwd |
| Runtime | Permission profile | Current legacy-compatible permission intent is rebased to child cwd; child cwd is not auto-trusted |
| Runtime | Workspace-write | Current workspace-write allows child cwd writes and not parent cwd writes |
| Runtime | Read-only | Current read-only remains read-only even if child config allows workspace-write |
| Runtime | Parent ACL | Explicit cwd does not copy parent concrete runtime permission profile as raw ACL |
| Runtime | Shell snapshot | Child with explicit cwd does not inherit parent cwd-bound shell snapshot |
| Runtime | Exec policy | Child with explicit cwd does not inherit parent cwd-bound exec policy as concrete state |
| Config | Child project config | Loaded from child cwd, not parent cwd |
| Config | Role config | Applied after child cwd config rebuild |
| Environment selection | Explicit cwd | Parent environment cwd is not inherited; default selection cwd equals child cwd |
| Environment selection | Omitted cwd | Existing inherited `turn.environments.to_selections()` behavior is unchanged |
| Persistence | New child thread | `SessionConfiguredEvent.cwd`, `SessionMeta.cwd`, and `ThreadPersistenceMetadata.cwd` store child cwd |
| Resume | Child resume after restart | Child restores stored cwd |
| Resume | Missing stored cwd | Resume fails fast and does not start child from caller cwd |
| Resume | Grandchild resume after restart | Each descendant restores its own stored cwd |
| Compatibility | App-server clients | Existing `Thread.cwd` surfaces expose child cwd; no schema changes are required |

## Commands

```bash
cd codex-rs
just fmt
cargo test -p codex-core multi_agents
cargo test -p codex-core agent::control
cargo test -p codex-core thread_manager
cargo test -p codex-core config
cargo test -p codex-core shell_snapshot
cargo test -p codex-tools
just fix -p codex-core
```

Run `cargo test -p codex-app-server-protocol` and `just write-app-server-schema` only if
app-server protocol is touched, which v1 should avoid.

Do not run or update TUI snapshots for v1 unless a later change adds user-visible TUI behavior.

## Coverage Gaps

- Forked history with explicit cwd is intentionally unsupported in v1.
- Remote environment re-rooting is not implemented in v1; explicit cwd uses default environment
  selection construction.
- `ThreadItem::CollabAgentToolCall` does not include cwd in v1. App-server observation relies on
  existing `Thread.cwd` surfaces.
- The model-facing `spawn_agent` result does not echo effective cwd in v1 to avoid absolute path
  disclosure.
- Current focused implementation coverage validates fork conflicts, file-path rejection, relative
  cwd runtime propagation, environment cwd, current workspace-write rebase to child cwd, current
  read-only preservation, parent permission-profile raw ACL non-inheritance, parent exec-policy
  manager non-inheritance, child workspace-write policy shape, parent cwd non-writeability,
  successful resume from stored child cwd, and resume fail-fast for missing stored cwd. Additional
  outside-workspace, inaccessible-path, role-config-from-child-cwd, shell snapshot
  non-inheritance, and explicit grandchild-cwd resume scenarios remain follow-up coverage items.

## Regression Focus

- Existing spawn behavior without `cwd` must not change.
- Existing app-server stable protocol shape must not change.
- Role/model/reasoning override behavior must remain compatible with current spawn tests.
- Shell snapshot and exec policy inheritance must remain unchanged for omitted `cwd`.
- Resume of older rollouts without stored cwd must keep legacy fallback behavior.
