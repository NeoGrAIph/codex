# Interactive collaboration mode design

## Native source of truth

The native collaboration-mode source of truth is not a TUI-only flag. It is shared across protocol, models manager, core context and TUI:

| Layer | Native owner | Current behavior |
| --- | --- | --- |
| Mode identity | `codex-rs/protocol/src/config_types.rs::ModeKind` | Defines `Default`, `Interactive`, `Plan`, `PairProgramming` and `Execute`; `Default`, `Interactive` and `Plan` are TUI-visible through `ModeKind::is_tui_visible` and `TUI_VISIBLE_COLLABORATION_MODES`. |
| Presets | `codex-rs/models-manager/src/collaboration_mode_presets.rs` | Builds native collaboration presets; `interactive_preset()` provides Interactive developer instructions without changing model or reasoning effort. |
| App-server protocol | `codex-rs/app-server-protocol/src/protocol/v2/collaboration_mode.rs`, `protocol/common.rs` and generated schema fixtures | Exposes `interactive` as a collaboration-mode wire/schema value through the existing collaboration-mode settings paths. |
| TUI mode list | `codex-rs/tui/src/collaboration_modes.rs` | Filters presets through `ModeKind::is_tui_visible`; `interactive_mask()` reuses the same mask lookup path as Plan. |
| TUI UX | `/interactive`, Shift+Tab, footer `CollaborationModeIndicator::Interactive` | Shows and switches Interactive mode using native TUI projection points. |
| Core behavior | collaboration-mode instructions, `request_user_input` spec, mode-specific tool availability | `request_user_input` is available in `Interactive` and `Plan`; Default remains gated by `DefaultModeRequestUserInput`. |

## Porting rule

A direct port of historical `Interactive` mode is unsafe if it only adds a slash command or footer label. The selected `fork/140` port updates the native collaboration-mode source of truth and its projections: enum/wire/schema, presets, TUI visibility/cycle/footer/slash handling, `request_user_input` availability and docs.

## Candidate implementation paths

| Path | Required native changes | Tradeoff |
| --- | --- | --- |
| New `ModeKind::Interactive` | Implemented locally: enum/wire/schema value, preset, model-manager entry, app-server schema/serialization tests, TUI visibility/cycle/footer/slash command, request-user-input availability rules and docs. | Clear user-facing mode; carries protocol/config compatibility responsibility. |
| Plan/default composition | Rejected for current `fork/140` implementation. | Lower compatibility risk, but did not satisfy the historical fork contract of an explicit interactive mode indicator. |

## Invariants

- Do not add a TUI-only mode that app-server/core cannot represent.
- Do not add or reorder protocol enum values without schema generation and compatibility notes.
- Do not silently alias `Interactive` to `Default` unless the feature contract explicitly documents that behavior and the UI makes it clear.
- Do not treat MAv2 sub-agent runtime as controlled by `collaborationMode`; collaboration mode is a settings/prompt/tool-availability surface, not the sole switch for multi-agent execution.
- Old sessions with missing or unknown collaboration mode data must remain readable through existing defaults or controlled diagnostics.

## Relationship to OpenClaude-inspired work

OpenClaude-style interactive choices are useful as UX reference only. In Codex, user questions already have native `request_user_input` semantics whose availability is mode-dependent. Interactive mode should improve the conditions under which structured user choices are native and visible; it must not introduce OpenClaude-style task storage, workflow files or a separate prompt/runtime control plane.
