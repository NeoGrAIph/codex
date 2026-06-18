# Interactive collaboration mode verification

## Current verification state

`fork/140` now has a local runtime implementation of separate Interactive mode through native collaboration-mode surfaces. Current verification covers the enum/source of truth, generated schema value, presets, TUI slash/footer projection and `request_user_input` availability. The latest focused integration smoke reran the protocol, models-manager, tools, core, TUI and app-server-protocol checks after the broader dirty feature set had changed; rebuilt installed-binary manual smoke is release acceptance, not a blocker for the current dirty-worktree implementation goal.

## Required checks before any implementation

| Surface | Required verification |
| --- | --- |
| Protocol/schema | `just test -p codex-app-server-protocol serialize_list_collaboration_modes` and schema fixture checks after any enum/request/schema change. |
| Mode source of truth | Tests around `ModeKind::is_tui_visible`, `TUI_VISIBLE_COLLABORATION_MODES` and any new mode serialization. |
| Presets | `just test -p codex-models-manager collaboration_mode` or the closest preset test after changing built-in presets. |
| TUI slash/footer | Focused `codex-tui` plan/collaboration tests and snapshots covering command visibility, `/interactive`, mode cycle and footer indicator. |
| Core/tool behavior | Focused `codex-core` and `codex-tools` tests for `request_user_input` availability, collaboration-mode developer instructions and Plan/update-plan restrictions. |
| Compatibility | Old session/config deserialization with missing mode fields and controlled behavior for unknown/unsupported mode values. |
| Docs | `git diff --check -- docs/fork/features/interactive-collaboration-mode.md docs/fork/projects/interactive-collaboration-mode`. |

## Scenario matrix

| Scenario | Expected result |
| --- | --- |
| Current `fork/140` TUI-visible modes | `Default`, `Interactive` and `Plan` are visible in the normal TUI cycle. |
| `/plan` command | Switches to native Plan mode through existing collaboration-mode flow. |
| `/interactive` command | Switches to native Interactive mode through existing collaboration-mode flow and shows `Interactive mode` footer state. |
| Hidden `PairProgramming` / `Execute` modes | They are not exposed by `codex-rs/tui/src/collaboration_modes.rs` because `ModeKind::is_tui_visible` returns false. |
| `request_user_input` in Interactive | Available by default alongside Plan mode; Default remains unavailable unless the Default-mode feature flag is enabled. |
| App-server schema | Generated `ModeKind` schema includes `interactive` so separately distributed clients see the wire value. |

## Known gaps

- Full manual TUI smoke with a rebuilt installed fork binary remains a release/manual acceptance check outside the local dirty-worktree verification scope.
- Interactive mode does not add lifecycle/workbench behavior; sub-agent runtime changes belong to `subagent-workbench` and MAv2 docs/tests.
- Compatibility policy for older app-server clients that do not know `interactive` remains a release/distribution concern, not a local schema-test gap.

## Verification log

- 2026-06-18: dossier created after feature-dossier audit found `docs/fork/features/interactive-collaboration-mode.md` without a matching `docs/fork/projects/interactive-collaboration-mode/` package. Current source map verified from `ModeKind`, `collaboration_modes.rs`, `collaborationMode/list` and Plan-mode TUI surfaces; no runtime code changed.
- 2026-06-18: `ModeKind::Interactive` implemented locally and schema regenerated. Focused checks passed: `just test -p codex-protocol tui_visible_collaboration_modes_match_mode_kind_visibility`; `just test -p codex-models-manager collaboration_mode`; `just test -p codex-tools request_user_input_modes_follow_default_mode_feature`; `just test -p codex-core request_user_input_unavailable_messages_respect_default_mode_feature_flag request_user_input_tool_description_mentions_available_modes`; `just test -p codex-tui interactive_slash_command_switches_to_interactive_mode collaboration_mode_commands_are_hidden_when_disabled status_line_model_with_reasoning_interactive_mode_footer_snapshot`; `just test -p codex-app-server-protocol serialize_list_collaboration_modes typescript_schema_fixtures_match_generated json_schema_fixtures_match_generated`; `cargo insta pending-snapshots --workspace-root .`; `git diff --check`.
- 2026-06-18: Focused integration smoke rerun after later feature changes passed through Kepler: protocol visible-mode test, models-manager presets, tools `request_user_input` gating, core request-user-input availability/description, TUI `/interactive` slash/footer snapshots, app-server-protocol collaboration/schema fixtures, no pending snapshots and scoped `git diff --check`. A transient TUI compile gap from the new `AgentRoleConfig.runtime_config_sources` test fixture field was fixed before the successful rerun.
