# Interactive collaboration mode project

## Status

`interactive-collaboration-mode` is implemented locally on `fork/140` as an upstream-shaped extension of the native collaboration-mode source of truth. The fork adds `ModeKind::Interactive`, generated app-server schema value `interactive`, a builtin preset, `/interactive`, TUI-visible mode projection, footer indication and `request_user_input` availability for Interactive mode.

## Canonical links

| Artifact | Purpose |
| --- | --- |
| `docs/fork/features/interactive-collaboration-mode.md` | Feature contract, historical commits and current native coverage. |
| `docs/fork/research/multi-agents/architecture.md` | Shared collaboration-mode and multi-agent architecture map. |
| `codex-rs/protocol/src/config_types.rs` | Native `ModeKind` and TUI-visible mode source of truth. |
| `codex-rs/tui/src/collaboration_modes.rs` | TUI filtering and mode cycling projection. |
| `codex-rs/app-server-protocol/src/protocol/v2/collaboration_mode.rs` | App-server collaboration-mode list contract. |
| `codex-rs/app-server-protocol/schema/typescript/ModeKind.ts` and `schema/json/ClientRequest.json` | Generated schema evidence for wire value `interactive`. |

## Current decision

The selected implementation path is a real upstream-shaped `ModeKind::Interactive`. The feature must remain additive and must not become a parallel TUI/runtime switch. Future edits should keep all projections synchronized: protocol/schema, presets, TUI mode list/cycle/footer/slash handling, tool availability and docs.

## Implementation surfaces

- Source of truth: `ModeKind::Interactive`, `CollaborationMode`, `CollaborationModeMask`, `TUI_VISIBLE_COLLABORATION_MODES`.
- Presets: `codex-rs/models-manager/src/collaboration_mode_presets.rs` adds the Interactive preset and developer instructions.
- App-server protocol: `collaborationMode/list`, `thread/settings/update.collaborationMode`, `turn/start.collaborationMode` and generated schema value `interactive`.
- TUI projection: `codex-rs/tui/src/collaboration_modes.rs`, `/interactive` dispatch, footer `CollaborationModeIndicator::Interactive`, mode list/cycle filtering, focused tests and snapshots.
- Core/tool behavior: `request_user_input` is available in `Interactive` and `Plan`; Default remains gated by `DefaultModeRequestUserInput`.

## Current gaps

- Interactive mode currently changes prompt/tool-availability posture only; it does not change MAv2 lifecycle, workbench actions or sub-agent runtime ownership.
- There is no OpenClaude-style task/workflow storage behind Interactive mode.
- Broader end-to-end manual runtime testing with the rebuilt installed fork binary remains release/manual acceptance outside the focused local dirty-worktree tests.
