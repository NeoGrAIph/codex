# thread-note research for rust-v0.116.0

- Baseline tag: `rust-v0.116.0`
- Baseline commit: `38771c9082535aa16b4c4d0395d3532f32f656ff`
- Porting mode: upstream-shaped adaptation

## Gap Analysis

- `0.116.0` already has split multi-agent handlers, richer turn context, app-server metadata merging, and current spawn model/reasoning overrides.
- `0.116.0` does not have any `thread_note` surface in protocol, core, TUI, or app-server.
- Public app-server v2 currently reused core sub-agent source too directly, so adding `thread_note` in core would leak the field unless the boundary was tightened.

## Conflict-Prone Files

- `codex-rs/protocol/src/protocol.rs`
- `codex-rs/core/src/codex.rs`
- `codex-rs/core/src/tools/handlers/multi_agents/*`
- `codex-rs/tui/src/multi_agents.rs`
- `codex-rs/app-server-protocol/src/protocol/v2.rs`
- `codex-rs/app-server/src/codex_message_processor.rs`

## Adaptation Notes

- Reused current split handler layout instead of restoring the historical monolithic collab handler.
- Preserved `spawn_agent` model and reasoning overrides from `0.116.0`.
- Kept `thread_note` out of model-facing context by only extending `TurnContextItem` metadata and avoiding `EnvironmentContext` changes.
- Replaced app-server passthrough of core `SubAgentSource` with an app-server-owned mapping so fork-only metadata stays internal.

## Release-Specific Verification

- Compile all touched crates together because protocol/core/TUI/app-server boundaries all changed.
- Recheck `multi_agents` snapshots because note rendering is user-visible.
