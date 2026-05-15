# Project: agents-overlay

## Status

Implemented for `fork/130` on top of `rust-v0.130.0`.

Baseline context:

- Release tag: `rust-v0.130.0`
- Baseline commit: `58573da43ab697e8b79f152c53df4b42230395a8`
- Research package: `docs/fork/research/0.130.0/agents-overlay.md`

## Canonical Links

- Feature contract: `../../features/agents-overlay.md`
- Release research: `../../research/0.130.0/agents-overlay.md`
- Native switch path: upstream 0.130 `select_agent_thread(...)`; `agent-switch-viewport` is
  deferred / obsoleted for this release.
- Cwd decision: `../subagent-cwd/README.md`
- Thread-note decision: `../thread-note/README.md`
- Role-template decision: `../agent-role-templates/README.md`

## Goal

Add a full-screen TUI `A G E N T S` overlay that inspects known subagents, shows fork/118-style
actions/details, connects through the existing thread selection path, and can submit a confirmed
agent `Shutdown`. Keep `/agent` as the lightweight picker and keep app-server/Codex app behavior
stable.

## Implementation Map

- Overlay enum: `codex-rs/tui/src/pager_overlay.rs::Overlay`.
- Event routing: `codex-rs/tui/src/app.rs` and `codex-rs/tui/src/app_backtrack.rs`.
- Key entry points: `codex-rs/tui/src/app/input.rs`.
- Projection module: new `codex-rs/tui/src/agents_overlay.rs`.
- Thread selection: existing `select_agent_thread(...)` /
  `select_agent_thread_and_discard_side(...)`.
- Existing metadata helpers: `agent_navigation.rs`, `loaded_threads.rs`, and `multi_agents.rs`.
- App-server data: existing `Thread`, `ThreadStatus`, and `ThreadItem::CollabAgentToolCall`
  payloads.
- Snapshots: overlay states and narrow terminal layouts.

## Locked Decisions

- TUI-only in v1.
- `/agent` remains the lightweight picker.
- `Close` is implemented only as confirmed `Shutdown`; never use `thread/unsubscribe`.
- `Connect` reuses existing selection helpers.
- Persona is displayed when present/non-default; template policy is not displayed.
- Plan data is inspect-only and compact: show progress/active step or explanation fallback, never a
  full always-visible step list.
- Effective cwd is shown only from existing `Thread.cwd` / session cwd surfaces.
- `threadNote` is optional secondary metadata and never affects identity, ordering, or selection.
- `AgentNavigationState` is not the projection source of truth; it is only an ordering/label cache.
- `find_loaded_subagent_threads_for_primary(...)` remains useful for picker backfill, but the
  overlay needs a richer parser that preserves `Thread.source`, depth/path, cwd, and status.
- Activation uses the `Ctrl-T` cycle: main -> transcript -> `A G E N T S` -> main.

## Current 0.130 Facts

- `Overlay` currently has only `Transcript` and `Static` variants.
- `App.overlay` is `Option<Overlay>`; there is no overlay stack.
- `App::run` sends all overlay events through `handle_backtrack_overlay_event(...)`, whose `Esc`
  behavior is transcript/backtrack-specific.
- `/agent` and `/multi-agents` send `AppEvent::OpenAgentPicker`.
- `thread/loaded/list` returns ids; the TUI then reads each thread with `thread/read(false)` during
  backfill.
- `loaded_threads.rs::LoadedSubagentThread` stores only id, nickname, and role.

## Implementation Order

1. Add projection model and pure parser tests.
2. Add `Overlay::Agents` and kind-aware overlay routing.
3. Add suspended transcript state and keep transcript updates flowing while agents overlay is open.
4. Add inspect/details rendering and snapshots.
5. Wire `Connect` through existing selection helpers after closing the overlay stack.
