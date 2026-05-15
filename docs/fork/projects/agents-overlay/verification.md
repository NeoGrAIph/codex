# agents-overlay Verification

## Scenarios

| Scenario | Expected result |
| --- | --- |
| Empty projection | Empty overlay state |
| `thread/loaded/list` failure | Overlay opens with local rows and degraded-data state |
| Per-thread `thread/read` failure | Server-only row omitted or local fallback row retained |
| Flat agents | Rows sorted deterministically |
| Nested agents | Tree/depth preserved |
| Primary/current/side threads | Primary and side threads excluded; current subagent row marked `(current)` |
| Cwd display | Uses `Thread.cwd` |
| Thread note | Absent/no update, null/clear, string/replace semantics |
| Persona/policy | Persona label rendered when present/non-default; policy not required or rendered |
| Status precedence | Collab state, local runtime, thread status, closed, fallback |
| Inspect | Toggles detail state only |
| Plan summary | Hidden outside Inspect; summary from `TurnPlanUpdated`; explanation/legacy fallback; absent data hides `Plan:` |
| Connect | Closes overlay stack and uses existing selection path |
| Connect disappeared target | Existing selection path owns the user-facing error |
| Transcript suspension | Transcript restores after agents overlay closes |
| Suspended transcript updates | Committed cells arriving while agents is open appear after restore |
| Backtrack | Does not activate inside agents overlay |
| Close action | Confirmation required; Yes submits `Shutdown`; No/Esc do not submit; never uses `thread/unsubscribe` |
| App-server compatibility | No overlay-owned schema/generated artifact diff |

## Snapshot States

- Empty
- Degraded data
- Flat
- Nested
- Selected row
- Expanded inspect
- Action menu
- Close confirmation
- Running
- Completed
- Errored
- Shutdown/not-loaded
- With cwd
- With thread note
- Null-cleared note
- Persona label and no policy fields
- Plan active step / completed-only / explanation fallback / absent plan
- Narrow terminal

## Focused Test Areas

- Pure projection builder tests for flat, nested, missing metadata, and ordering fallback.
- `Thread.source` parser tests that preserve parent/depth/agent_path.
- Projection tests for primary/root and side thread exclusions plus current subagent marker.
- Thread-note merge tests for absent/null/string semantics.
- Plan summary tests for active step, completed-only, explanation fallback, and hidden empty state.
- Overlay routing tests proving `Esc` in agents does not enter transcript backtrack.
- Suspended transcript tests proving insert/consolidation updates reach the suspended transcript.
- Connect action tests proving no direct attach/resume/close/unsubscribe behavior in overlay.
- Close action tests proving confirmed `Shutdown` and cancelled close paths.

## Commands

```bash
cd codex-rs
cargo test -p codex-tui agents_overlay
cargo test -p codex-tui agent_navigation
cargo test -p codex-tui loaded_threads
cargo test -p codex-tui transcript_overlay
cargo test -p codex-tui
cargo insta pending-snapshots -p codex-tui
```

Before finalizing implementation, also run:

```bash
cd codex-rs
just fmt
just fix -p codex-tui
```

If `thread-note` protocol fields are implemented in the same change set, run the thread-note
protocol/app-server verification owned by that feature. Overlay-only work does not require schema
generation.

## Coverage Gaps

- Policy display requires a later projection contract.
- Persisted-but-not-loaded historical descendants require a later latency/product decision.
- Full rendered `insta` snapshots are still required before feature completion; current focused
  unit coverage exercises rendering helpers, action routing, and compact plan summaries.
- Periodic server polling is out of scope for v1; the overlay refreshes on open and from local
  updates while visible.
