# subagent-cwd Design

## Canonical State

The resolved child cwd is the child session root. It must be reflected in child `Config.cwd`,
first-turn context, environment selections, session metadata, and resume config.

The model-facing tool result is not canonical state. App-server `Thread.cwd`, core
`SessionConfiguredEvent.cwd`, persisted `SessionMeta.cwd`, and `ThreadPersistenceMetadata.cwd` are
the observation surfaces for effective cwd.

## Data Flow

1. Parse optional `spawn_agent.cwd` in both v1 and MultiAgentV2 handlers.
2. Compute fork mode before spawn:
   - legacy `fork_context: true` is forked history;
   - MultiAgentV2 omitted `fork_turns` defaults to `all`;
   - explicit `cwd` is accepted only when fork mode is `None`.
3. Resolve cwd:
   - omitted -> parent `turn.cwd`;
   - relative -> parent `turn.cwd` joined with the raw relative path;
   - absolute -> raw absolute path;
   - normalize for the native platform without canonicalizing away the logical path.
4. Validate explicit cwd:
   - empty path is invalid;
   - path must be absolute after resolution;
   - path must exist;
   - path must be a directory;
   - stat/access errors are controlled model-facing errors.
5. Rebuild child config layers for resolved cwd:
   - load user/project cwd-scoped layers from the child cwd;
   - preserve session layers intentionally;
   - preserve runtime-owned model/provider/reasoning, base instructions, developer instructions,
     compact prompt, approval policy, shell environment policy, and sandbox executable;
   - preserve runtime model/session state, but use child cwd config/role permission state instead
     of copying the parent concrete `turn.permission_profile()`.
   - do not carry parent shell snapshot or cwd-bound exec policy as concrete state when child cwd
     differs; rebuild them for the child cwd or omit them so session startup computes child-owned
     state.
6. Apply requested model/reasoning overrides.
7. Apply role config after child cwd rebuild.
8. Apply runtime policy overrides that do not reset `config.cwd` to parent `turn.cwd`.
9. Apply spawn depth/thread overrides.
10. Spawn:
    - omitted cwd -> keep current `Some(turn.environments.to_selections())`;
    - explicit cwd -> pass `environments: None`.
11. `ThreadManagerState::spawn_new_thread_with_source()` builds default environments from
    `config.cwd` when environments are `None`.
12. Session startup persists child cwd through session metadata.
13. Descendant resume reads each stored child cwd and rebuilds that child's config from it.

## Invariants

- Omitted `cwd` preserves current upstream behavior.
- Explicit cwd failure never falls back to parent cwd.
- Child cwd may be outside parent workspace.
- Child cwd is not auto-trusted.
- Explicit child cwd does not copy parent concrete runtime ACL.
- Parent environment cwd is not inherited for explicit cwd.
- Parent shell snapshot and cwd-bound exec policy are not inherited for explicit cwd.
- Stored cwd resume failure is a hard error, not a fallback to caller cwd.
- Explicit cwd never mutates `thread/start`, `thread/resume`, or `thread/fork` request semantics.
- Effective cwd is not duplicated into model-facing `SpawnAgentResult`.
- Hidden metadata mode does not hide the `cwd` input field.

## Source-Level Decisions

- Add `cwd: Option<PathBuf>` to both `SpawnAgentArgs` structs. MultiAgentV2 must keep
  `#[serde(deny_unknown_fields)]`.
- Add `cwd` to both v1 and v2 spawn input schemas. Do not add `cwd` to output schemas.
- Add a shared resolver/validator in the multi-agent spawn path, not in app-server request
  processors.
- Split or replace `apply_spawn_agent_runtime_overrides()` for explicit cwd so it cannot assign
  `config.cwd = turn.cwd.clone()`.
- Audit `AgentControl` shell snapshot and exec policy inheritance. If child cwd differs from the
  parent turn cwd, do not pass parent cwd-derived snapshot/policy into the child thread.
- Add a child-cwd config rebuild helper in `Config` or adjacent config code. Do not call
  `Config::rebuild_preserving_session_layers()` naively with the parent config, because it passes
  `self.cwd` into final `ConfigOverrides.cwd`.
- In `AgentControl::resume_agent_from_rollout()`, avoid using one `config.clone()` for all
  descendants. Each resumed child must rebuild from its persisted session cwd when available.

## Tradeoffs

- Forked history plus explicit cwd is deferred because history placement semantics are ambiguous.
- The model does not receive effective cwd in output to avoid absolute path disclosure.
- No app-server changes are needed because existing `Thread.cwd` surfaces expose effective cwd.
- Re-rooting inherited environment selections is deferred. Passing `environments: None` is simpler
  and uses the existing `ThreadManager` source of truth for child-rooted defaults.

## Failure Modes

- `cwd` empty: controlled model-facing error.
- `cwd` cannot normalize to an absolute path: controlled model-facing error.
- resolved `cwd` does not exist: controlled model-facing error before any child thread is created.
- `cwd` points to a file: controlled model-facing error before any child thread is created.
- `cwd` cannot be stat-ed: controlled model-facing error before any child thread is created.
- child config rebuild fails: controlled model-facing error, no fallback to parent cwd.
- stored child cwd cannot be restored during resume: controlled error with no fallback to caller cwd.
- shell snapshot or exec policy cannot be rebuilt for child cwd: controlled error if required by
  the selected runtime path; otherwise omit inherited state and let session startup recompute it.
- role config fails after child cwd rebuild: preserve existing role failure behavior.
- explicit cwd with forked history: controlled model-facing error.

## App-server Compatibility

No app-server protocol or schema change is part of v1. Existing app-server clients can observe the
effective child cwd through `Thread.cwd` on `thread/started`, `thread/read`, and `thread/list` once
core starts the child thread with the correct `config.cwd`.

Do not change these in v1:

- `codex-rs/app-server-protocol/src/protocol/v2/thread_data.rs::Thread`
- `codex-rs/app-server-protocol/src/protocol/v2/item.rs::ThreadItem::CollabAgentToolCall`
- `codex-rs/protocol/src/protocol.rs::CollabAgentSpawnBeginEvent`
- `codex-rs/protocol/src/protocol.rs::CollabAgentSpawnEndEvent`
- generated app-server schema artifacts

## Handoff Notes

- `agent-role-templates`: template resolution must happen after child cwd config rebuild, using the
  child cwd as the project/config source of truth.
- `thread-note`: notes should rely on thread/session metadata for cwd; do not infer cwd from
  collab tool-call rows.
- TUI overlay/viewport: v1 has no UI work. Future display should read cwd from session/thread
  state, not from spawn tool output.
