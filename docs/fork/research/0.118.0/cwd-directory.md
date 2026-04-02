# `cwd` / `Directory` research for `rust-v0.118.0`

- Baseline tag: `rust-v0.118.0`
- Baseline commit: `b630ce9a4e754d35a1f33e4366ba638d18626142`
- Research target: the `Directory` surface in the unified TUI status card and the wider `cwd` data flow that feeds it
- Scope: `tui`, `core`, `protocol`, `rollout`, `app-server`, and the app-server transport surfaces now hosted inside `tui`
- Research state: current `fork/118` `HEAD` only; no WIP or stashed changes are part of this document

## Executive Summary

- `cwd` is not a cosmetic UI-only field. It remains a session/root concept that flows through config resolution, turn context, persistence, sandbox derivation, command execution, skill/plugin loading, and resume/list APIs.
- In `fork/118`, the old split between native `tui` and `tui_app_server` is gone for these surfaces. The same `codex-rs/tui` crate now owns local UI, app-server transport bootstrapping, remote/embedded thread semantics, picker behavior, trust onboarding, and session logging.
- The status card `Directory` row still renders from `config.cwd`, while footer/status surfaces still prefer runtime `current_cwd` and only fall back to `config.cwd`.
- `cwd` now also directly shapes markdown and transcript rendering. Proposed-plan cells, reasoning summaries, streamed markdown, diff summaries, and similar history surfaces either snapshot session `cwd` or derive cwd-aware display strings so local file links and path labels stay stable after the live state advances.
- The repository still intentionally preserves multiple `cwd` forms:
  - `config.cwd`: effective session workspace root from config building or resume/fork config rebuild
  - `current_cwd`: latest runtime/session cwd advertised to the widget via `SessionConfigured`
  - `TurnContext.cwd`: per-turn cwd used by tools and persisted in rollout
  - command cwd: process working directory for exec/shell operations
- In remote app-server mode, the client still intentionally does not treat the local process cwd as authoritative server state for thread bootstrap and picker defaults. That is why default same-cwd filtering and embedded `cwd` overrides are disabled there, even though per-turn `turn/start` requests still carry an explicit cwd.

## Terminology

### `config.cwd`

The effective working root inside `Config`. This is the value produced by `ConfigBuilder`, passed into new sessions, used for project config layer discovery, and displayed in the status card.

Primary code pointers:

- `codex-rs/core/src/config/mod.rs`
  - `ConfigBuilder::build`
  - final `Config { cwd: resolved_cwd, .. }`
- `codex-rs/tui/src/status/card.rs`
  - `("workdir", config.cwd.display().to_string())`
  - `directory: config.cwd.to_path_buf()`

### `current_cwd`

Widget-local runtime cwd. This is updated from `SessionConfiguredEvent.cwd` and used by footer status surfaces and git-branch refresh logic.

Primary code pointers:

- `codex-rs/tui/src/chatwidget.rs`
  - `on_session_configured`
  - `self.current_cwd = Some(event.cwd.clone())`
- `codex-rs/tui/src/chatwidget/status_surfaces.rs`
  - `status_line_cwd`
  - `sync_status_line_branch_state`

### `TurnContext.cwd`

Per-turn cwd used by the runtime and tools. This is the durable answer to "what cwd was active when this turn ran?"

Primary code pointers:

- `codex-rs/protocol/src/protocol.rs`
  - `Op::UserTurn { cwd, .. }`
  - `Op::OverrideTurnContext { cwd: Option<PathBuf>, .. }`
  - `TurnContextItem { cwd, .. }`
- `codex-rs/core/src/codex.rs`
  - turn-context construction

### Command cwd

The actual process cwd passed to exec/shell operations. This can match the session cwd, but it is still a distinct concept because sandbox policy calculations may be rooted in the session config cwd.

Primary code pointers:

- `codex-rs/core/src/tools/handlers/shell.rs`
- `codex-rs/app-server/src/command_exec.rs`
- `codex-rs/protocol/src/protocol.rs`
  - `ExecCommandBeginEvent.cwd`
  - `ExecCommandEndEvent.cwd`

### Rendering cwd

Several TUI transcript surfaces preserve cwd-aware rendering context so markdown file links and path labels stay consistent even if rendering happens later or after state changes.

Primary code pointers:

- `codex-rs/tui/src/history_cell.rs`
  - `new_proposed_plan`
  - `new_reasoning_summary_block`
  - `new_view_image_tool_call`
- `codex-rs/tui/src/markdown_stream.rs`
  - `MarkdownStreamCollector::new`
- `codex-rs/tui/src/streaming/mod.rs`
  - `StreamState::new`
- `codex-rs/tui/src/streaming/controller.rs`
  - `StreamController::new`

## Source-of-Truth Chain

### 1. `cwd` enters through CLI/config building

The unified TUI CLI still exposes `-C` / `--cd` as "use the specified directory as its working root." CLI merging passes that into the interactive config path, and `ConfigBuilder::build` resolves the chosen cwd to an absolute path. If no explicit cwd is provided, it falls back to `current_dir()`.

Primary code pointers:

- `codex-rs/tui/src/cli.rs`
- `codex-rs/cli/src/main.rs`
  - `merge_interactive_cli_flags`
  - `interactive.cwd = Some(cwd)`
- `codex-rs/core/src/config/mod.rs`
  - `let cwd_override = harness_overrides.cwd.as_deref().or(fallback_cwd.as_deref());`
  - `AbsolutePathBuf::relative_to_current_dir(path)?`
  - `AbsolutePathBuf::current_dir()?`

### 2. `Config.cwd` becomes session cwd

New sessions copy `config.cwd` into session configuration and then emit it to the UI as part of `SessionConfiguredEvent`.

Primary code pointers:

- `codex-rs/core/src/codex.rs`
  - `SessionConfiguration { cwd: config.cwd.clone(), .. }`
  - `EventMsg::SessionConfigured(SessionConfiguredEvent { cwd: session_configuration.cwd.to_path_buf(), .. })`
- `codex-rs/protocol/src/protocol.rs`
  - `SessionConfiguredEvent.cwd`

### 3. Turn execution persists the current cwd

The runtime snapshots cwd into each turn context. This matters because a session can later diverge from the initial session meta cwd without rewriting the rollout head.

Primary code pointers:

- `codex-rs/core/src/codex.rs`
  - turn-context construction
  - rollout persistence of `TurnContextItem`
- `codex-rs/protocol/src/protocol.rs`
  - `TurnContextItem.cwd`

### 4. Rollout and state DB persist cwd in multiple places

- Initial session-level cwd is written into `SessionMeta.cwd`
- Latest turn-level cwd is written into `TurnContextItem.cwd`
- Thread metadata in the state DB can also carry cwd for fast resume/list flows

This layering is intentional, but the sources do not have identical freshness semantics:

- `SessionMeta.cwd` is the initial session cwd
- `TurnContext.cwd` can be newer after a directory change
- state DB `ThreadMetadata.cwd` is useful for fast lookup, but it is not a guaranteed mirror of the latest turn-level cwd

That is why the TUI recovery path reads latest turn context directly instead of assuming the state DB always carries the freshest cwd.

Primary code pointers:

- `codex-rs/rollout/src/recorder.rs`
  - `write_session_meta`
- `codex-rs/rollout/src/metadata.rs`
  - `builder_from_session_meta`
  - `builder.cwd = session_meta.meta.cwd.clone()`
- `codex-rs/tui/src/lib.rs`
  - `read_session_cwd`
  - `read_latest_turn_context`
- `codex-rs/app-server/src/codex_message_processor.rs`
  - `read_history_cwd_from_state_db`
- `codex-rs/protocol/src/protocol.rs`
  - `SessionMeta.cwd`
  - `InitialHistory::session_cwd`

## UI and Rendering Surfaces

### Status card `Directory`

The `/status` card reads `config.cwd` and renders it as `Directory`. This is not derived from `current_cwd`.

Primary code pointers:

- `codex-rs/tui/src/status/card.rs`
  - `let directory_value = format_directory_display(&self.directory, Some(value_width));`
  - `formatter.line("Directory", vec![Span::from(directory_value)])`

### Session header `directory:`

The startup/session info cell still renders the same concept in the header box via `SessionHeaderHistoryCell`.

Primary code pointers:

- `codex-rs/tui/src/history_cell.rs`
  - `SessionHeaderHistoryCell`
  - `format_directory_inner`

### Footer status line `current-dir`

The footer status line uses `status_line_cwd()`, which resolves to:

1. `current_cwd` if present
2. otherwise `config.cwd`

That means the footer is intentionally more runtime-aware than the status card.

Primary code pointers:

- `codex-rs/tui/src/chatwidget/status_surfaces.rs`
  - `status_line_cwd`
  - `StatusLineItem::CurrentDir`

### Footer and terminal title project naming

The status line and terminal title now treat `cwd` as a key into project-root inference, not just a raw directory string.

- git root wins when available
- otherwise the first `Project` layer returned from the current config-layer stack can provide the project label; with the current ordering, that means the outermost/root-most project layer rather than the nearest-to-cwd one
- only then does the UI fall back to cwd basename or formatted directory text

Primary code pointers:

- `codex-rs/tui/src/chatwidget/status_surfaces.rs`
  - `status_line_project_root_for_cwd`
  - `status_line_project_root_name`
  - `terminal_title_project_name`

### Session picker and same-cwd filtering

The unified session picker filters by same cwd unless `show_all` is enabled. The match is path-based, not string-prefix based.

Primary code pointers:

- `codex-rs/tui/src/resume_picker.rs`
  - `filter_cwd = if show_all || is_remote { None } else { std::env::current_dir().ok() }`
  - `paths_match`
  - `row_matches_filter`

Remote mode explicitly disables default cwd filtering because the local process cwd is not authoritative for a remote server.

### Display formatting rules

The main path formatting helper is still `format_directory_display()`. When the directory is under the user's home, it is rendered as `~` or `~/...`. When the display is too wide, it is center-truncated.

Primary code pointers:

- `codex-rs/tui/src/status/helpers.rs`
  - `format_directory_display`

The session header does not reuse that helper directly. `SessionHeaderHistoryCell::format_directory_inner` duplicates similar tilde/truncation behavior, so there is no single TUI formatting source of truth for directory labels.

Primary code pointers:

- `codex-rs/tui/src/history_cell.rs`
  - `SessionHeaderHistoryCell::format_directory_inner`

The shared `display_path_for()` helper is more contextual than the status card/footer:

- inside current cwd: relative path
- same repo but outside cwd: cwd-relative display, which can include `../...`
- otherwise: home-relative display if possible

Primary code pointers:

- `codex-rs/tui/src/diff_render.rs`
  - `display_path_for`
- `codex-rs/tui/src/resume_picker.rs`
- `codex-rs/tui/src/history_cell.rs`

### Markdown, streaming, and transcript rendering

`cwd` is now a first-class rendering input across the transcript pipeline. This is one of the largest practical expansions versus the older split-TUI mental model.

- proposed-plan cells snapshot session cwd before markdown rendering
- reasoning summary cells snapshot session cwd so file links match the live stream
- streaming markdown collectors keep a stable cwd for the full stream lifecycle
- viewed-image and diff-related history cells use cwd-aware path display helpers, but not every such cell stores cwd itself

Primary code pointers:

- `codex-rs/tui/src/history_cell.rs`
  - `new_proposed_plan`
  - `ProposedPlanCell`
  - `ReasoningSummaryCell`
  - `new_patch_event`
  - `new_view_image_tool_call`
- `codex-rs/tui/src/markdown_stream.rs`
  - `MarkdownStreamCollector { cwd, .. }`
  - `append_markdown(..., Some(self.cwd.as_path()), ..)`
- `codex-rs/tui/src/streaming/mod.rs`
  - `StreamState::new(width, cwd)`
- `codex-rs/tui/src/streaming/controller.rs`
  - `StreamController::new(width, cwd)`

### Approval and diff overlays

`cwd` is also a user-visible rendering input in approval flows. Apply-patch approvals carry cwd into the overlay model, and diff summaries use it to display file paths relative to the active workspace when possible.

Primary code pointers:

- `codex-rs/tui/src/bottom_pane/approval_overlay.rs`
  - `ApprovalRequest::ApplyPatch { cwd, .. }`
- `codex-rs/tui/src/diff_render.rs`
  - `DiffSummary { cwd, .. }`
  - `display_path_for`

### Trust onboarding

The trust screen lives in the unified TUI and renders the current cwd directly. It still resolves the repo root for trust decisions when possible.

Primary code pointers:

- `codex-rs/tui/src/onboarding/trust_directory.rs`
  - `TrustDirectoryWidget.cwd`
  - `resolve_root_git_project_for_trust(&self.cwd).unwrap_or_else(|| self.cwd.clone())`

### Session logging

The unified TUI can persist cwd into its session log header. This is operational metadata, not a source of truth for resume, but it is part of the observable cwd footprint.

Primary code pointers:

- `codex-rs/tui/src/session_log.rs`
  - `kind: "session_start"`
  - `"cwd": config.cwd`

## Runtime Usage

### Config layers and project discovery

`cwd` is a core input into config resolution and project-root discovery. It determines which project config layers are active and where project docs such as `AGENTS.md` are searched.

Primary code pointers:

- `codex-rs/core/src/config/mod.rs`
  - `resolved_cwd`
  - `get_active_project(resolved_cwd.as_path())`
- `codex-rs/core/src/project_doc.rs`
  - `discover_project_doc_paths(config)`

### Realtime startup context

`cwd` feeds the realtime startup context builder. It determines which recent-work group is treated as the current workspace, anchors the workspace scan, and therefore changes the prompt prelude injected into realtime sessions.

Primary code pointers:

- `codex-rs/core/src/realtime_context.rs`
  - `let cwd = config.cwd.clone()`
  - `build_recent_work_section(&cwd, ..)`
  - `build_workspace_section_with_user_root(&cwd, ..)`
- `codex-rs/core/src/realtime_conversation.rs`
  - `build_realtime_startup_context(..)`

### Sandbox and filesystem permissions

Sandbox readable and writable roots are derived relative to cwd. In workspace-write mode, cwd is always promoted into writable roots.

Primary code pointers:

- `codex-rs/protocol/src/protocol.rs`
  - `ReadOnlyAccess::get_readable_roots_with_cwd`
  - `SandboxPolicy::get_readable_roots_with_cwd`
  - `SandboxPolicy::get_writable_roots_with_cwd`
- `codex-rs/protocol/src/permissions.rs`
  - `from_legacy_sandbox_policy(..., cwd)`
  - `resolve_access_with_cwd`

This is why changing session cwd is not cosmetic. It directly changes what the sandbox treats as the workspace root.

### Command execution and shell tools

Shell and exec-like tools carry an explicit command cwd and emit it in execution events. The process spawn path uses that cwd directly.

Primary code pointers:

- `codex-rs/core/src/tools/handlers/shell.rs`
  - `ShellRequest { cwd: exec_params.cwd.clone(), .. }`
- `codex-rs/app-server/src/command_exec.rs`
  - `spawn_pty_process(..., cwd.as_path(), ..)`
  - `spawn_pipe_process(..., cwd.as_path(), ..)`

Important nuance:

- command process cwd is explicit per request
- app-server mismatch checks and some sandbox derivation still compare against the session/server config cwd

Primary code pointers:

- `codex-rs/app-server/src/codex_message_processor.rs`
  - config mismatch reporting for `requested_cwd`
  - `let sandbox_cwd = self.config.cwd.clone();`

### Guardian approvals

`cwd` is part of the approval context sent to guardian-style reviewers. Shell, exec-command, execve, and apply-patch approvals all serialize cwd, and command assessment payloads also include it.

Primary code pointers:

- `codex-rs/core/src/guardian/approval_request.rs`
  - `GuardianApprovalRequest::{Shell, ExecCommand, Execve, ApplyPatch}`
  - `CommandApprovalAction.cwd`
  - `command_assessment_action_value(..)`

### Shell snapshots

Interactive shell snapshotting preserves cwd as part of the snapshot identity and validates/replays the snapshot using that directory.

Primary code pointers:

- `codex-rs/core/src/shell_snapshot.rs`
  - `ShellSnapshot { path, cwd }`
  - `write_shell_snapshot(.., cwd)`
  - `validate_snapshot(.., session_cwd)`

### Skills, plugins, MCP config, and search roots

Skills are loaded relative to `config.cwd`, not the shell's ambient process cwd.

Primary code pointers:

- `codex-rs/core/src/skills.rs`
  - `SkillsLoadInput::new(config.cwd.clone().to_path_buf(), .. )`

The app-server `skills/list` path also treats cwd as an explicit root and can fan out across multiple cwd values, including extra per-cwd user roots.

Primary code pointers:

- `codex-rs/app-server/src/codex_message_processor.rs`
  - `skills/list`
  - `cwds`
  - `per_cwd_extra_user_roots`
- `codex-rs/app-server/src/config_api.rs`
  - `fallback_cwd = params.cwd.as_ref().map(PathBuf::from)`
- `codex-rs/core/src/config/service.rs`

`skills/list` and plugin listing are not fully symmetric. When `skills/list` receives no `cwds`, it falls back to `self.config.cwd`. Plugin listing does not inject the same default cwd list and starts from an empty root list before manager-side expansion.

Plugins follow the same idea: repo-scoped plugin discovery and per-cwd marketplace logic depend on cwd roots rather than the caller's process state.

Primary code pointers:

- `codex-rs/app-server/src/codex_message_processor.rs`
  - plugin list/read/install/uninstall request handling
- `codex-rs/core/src/plugins/manager.rs`
  - plugin manifest rewrite of relative `"cwd"` to `plugin_root.join(cwd)`

MCP server config has its own `cwd` semantics: stdio transports can carry an explicit cwd, while streamable HTTP transports reject it.

Primary code pointers:

- `codex-rs/core/src/config/types.rs`
  - `RawMcpServerConfig.cwd`
  - `McpServerTransportConfig::Stdio { cwd, .. }`
  - `throw_if_set("streamable_http", "cwd", ..)`

The broader search-root story also includes the JS REPL runtime: bare package imports resolve through `CODEX_JS_REPL_NODE_MODULE_DIRS` first and then fall back to `cwd`.

### File search

File search itself takes explicit roots. `cwd` matters one level above the search engine, because callers choose roots, rebuild search directories, or refresh file search state using the active config cwd.

Primary code pointers:

- `codex-rs/file-search/src/lib.rs`
  - `run(..., roots, ..)`
- `codex-rs/tui/src/file_search.rs`
  - `update_search_dir`
- `codex-rs/tui/src/app.rs`
  - `FileSearchManager::new(config.cwd.to_path_buf(), ..)`
  - `file_search.update_search_dir(self.config.cwd.to_path_buf())`

## Resume, Fork, and App-Server Thread Semantics

### Recovering cwd for resume/fork in the TUI

The unified TUI recovers session cwd in this order:

1. state DB thread metadata
2. latest rollout turn context
3. session meta

This order exists because the latest turn context can reflect a more recent cwd than the initial session meta.

Primary code pointers:

- `codex-rs/tui/src/lib.rs`
  - `read_session_cwd`
  - `read_latest_turn_context`
- `codex-rs/protocol/src/protocol.rs`
  - `SessionMeta.cwd`

This TUI-specific path exists because latest `TurnContext.cwd` can be newer than the initial session meta.

### Recovering cwd for resume/fork in the app-server

The app-server does not use the same recovery chain as the TUI.

- fork recovery reads `state DB -> SessionMeta`
- resume recovery over `ThreadHistory` falls back to `SessionMeta` via `InitialHistory::session_cwd()`
- current server-side code does not read latest rollout `TurnContext.cwd` in these paths unless the caller explicitly overrides cwd

Primary code pointers:

- `codex-rs/app-server/src/codex_message_processor.rs`
  - `read_history_cwd_from_state_db`
- `codex-rs/protocol/src/protocol.rs`
  - `InitialHistory::session_cwd`
  - `session_cwd_from_items`

### Changed-cwd prompt

Resume and fork compare the caller's current cwd with the recovered session cwd. If they differ, the UI can prompt the user to continue with either the current cwd or the historical session cwd.

Primary code pointers:

- `codex-rs/tui/src/lib.rs`
  - `cwds_differ`
  - `resolve_cwd_for_resume_or_fork`

### App-server thread listing

The app-server API supports exact cwd filtering on `thread/list`. That is server-side semantic filtering against `summary.cwd`.

Primary code pointers:

- `codex-rs/app-server/src/codex_message_processor.rs`
  - `list_threads_common`
  - `summary.cwd == expected_cwd`

### App-server thread reads

`thread/read` uses persisted thread summary cwd, not the TUI-style latest-turn recovery chain.

- DB-backed reads use `ThreadMetadata.cwd`
- rollout-backed reads use `SessionMeta.cwd`
- including turns in the response does not recompute `thread.cwd` from the latest `TurnContext.cwd`

Primary code pointers:

- `codex-rs/app-server/src/codex_message_processor.rs`
  - `thread_read`
  - `read_summary_from_state_db_by_thread_id`
  - `read_summary_from_rollout`
  - `thread_read_include_turns`

### Embedded vs remote app-server behavior

The unified TUI still deliberately splits behavior by transport mode:

- embedded/local mode: include cwd in thread start/resume/fork params
- remote mode: omit cwd override for thread start/resume/fork and disable local same-cwd assumptions in the picker
- both modes: `turn/start` still sends an explicit cwd for the actual user turn

Primary code pointers:

- `codex-rs/tui/src/app_server_session.rs`
  - `thread_start_params_from_config`
  - `thread_resume_params_from_config`
  - `thread_fork_params_from_config`
  - `thread_cwd_from_config`
  - embedded tests asserting `cwd`
  - remote tests asserting `cwd == None`

This remains one of the most important fork maintenance invariants around cwd semantics.

### Server-side config derivation

The app-server does not blindly trust every incoming cwd-bearing request as a pure display field.

- `thread/start` derives effective config from the incoming request cwd/overrides
- `thread/resume` and `thread/fork` derive config from requested overrides plus recovered history cwd
- mismatch reporting for requested-vs-active cwd is a separate concern and currently appears on the running-thread resume path, not as a general fork behavior

Primary code pointers:

- `codex-rs/app-server/src/codex_message_processor.rs`
  - `derive_config_from_params`
  - `derive_config_for_cwd`
  - `build_thread_config_overrides`
  - `load_latest_config`
  - `requested_cwd`

Command execution is a different case: request cwd controls the spawned process cwd, but sandbox derivation in the current code path still uses `self.config.cwd` rather than rebuilding config around the requested command cwd.

## Key Invariants and Gotchas

- `cwd` is a session/root concept first, and a display string second.
- `cwd` also appears as operational context in approvals, snapshots, trust prompts, realtime startup context, session logs, and markdown/transcript rendering.
- The status card and the footer are intentionally not identical:
  - status card reads `config.cwd`
  - footer reads `current_cwd` with fallback to `config.cwd`
- `SessionMeta.cwd` is not always the latest truth after directory changes; latest `TurnContext.cwd` can be newer.
- State DB thread metadata is useful for fast lookup, but should not be described as a guaranteed freshest cwd source.
- Command cwd and sandbox policy cwd are related but not guaranteed to be the same field in every code path.
- Remote app-server mode intentionally avoids treating the client's local cwd as authoritative server context for thread bootstrap and default filtering, but per-turn execution still carries cwd explicitly.
- In `fork/118`, any investigation that still starts from `tui_app_server/...` is working from an outdated module map. The transport-specific cwd logic now lives in `codex-rs/tui`.

## Practical Takeaways

- If the goal is to change the `Directory` row in `/status`, start in `tui/src/status/card.rs`, not in `chatwidget/status_surfaces.rs`.
- If the goal is to change the footer `current-dir` or terminal title `project`, start in `tui/src/chatwidget/status_surfaces.rs`.
- If the goal is to change how transcript markdown renders local file links, start in `tui/src/history_cell.rs`, `tui/src/markdown_stream.rs`, and `tui/src/streaming/`.
- If the goal is to change approval diff path presentation, start in `tui/src/bottom_pane/approval_overlay.rs` and `tui/src/diff_render.rs`.
- If the goal is to change resume/fork cwd behavior, start in `tui/src/lib.rs`, `tui/src/resume_picker.rs`, and `tui/src/app_server_session.rs`.
- If the goal is to change realtime workspace context injection, start in `core/src/realtime_context.rs`.
- If the goal is to change sandbox workspace semantics, start in `protocol/src/protocol.rs` and `protocol/src/permissions.rs`.
- If the goal is to change how repo-local docs, skills, or plugins are discovered, treat `config.cwd` as the primary root and audit the corresponding config rebuild path first.
- If the goal is to change app-server transport cwd semantics, start in `tui/src/app_server_session.rs` and `app-server/src/codex_message_processor.rs`.
