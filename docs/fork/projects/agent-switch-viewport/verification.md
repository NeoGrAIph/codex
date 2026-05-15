# agent-switch-viewport Verification

## Scenarios

| Scenario | Expected result |
| --- | --- |
| Clean idle cache | Restores transcript without replaying completed turns |
| Missing cache | Uses existing full replay path |
| Dirty cache | Falls back to full replay |
| Running/in-progress state | Falls back to full replay |
| Any buffered snapshot event | Falls back to full replay in V1 |
| Stream-time tail during capture | Does not cache active transcript |
| Width change | Re-renders through resize/reflow path |
| Row cap | Matches current replay rendering |
| Draft composer | Restored by runtime-only snapshot path |
| Queued input | Restored by runtime-only snapshot path without unintended autosend |
| Collaboration mode | Restored by runtime-only snapshot path |
| Closed/replay-only clean cache | Restores safely and keeps existing informational message |
| Restore render error | Clears partial restore and falls back to full replay |
| Switch entry points | `/agent`, hotkeys, approval, side connect share path |
| Overlay/backtrack | Use restored `transcript_cells` |
| App-server compatibility | No schema/generated artifact diff |

## Focused Test Areas

- Pure `ThreadVisualState` eligibility and invalidation tests.
- `select_agent_thread(...)` tests that assert restore and replay share one branch.
- Existing `replay_thread_snapshot_*` tests after helper extraction.
- Resize/reflow row-cap tests using cached cells.
- Transcript overlay/backtrack tests after restore.
- Side-thread discard/connect tests proving cache cleanup and shared switch behavior.

## Commands

```bash
cd codex-rs
cargo test -p codex-tui thread_visual_state
cargo test -p codex-tui replay_thread_snapshot
cargo test -p codex-tui thread_switch_replay_buffer
cargo test -p codex-tui resize_reflow
cargo test -p codex-tui transcript_overlay
```

Before finalizing implementation, also run:

```bash
cd codex-rs
cargo test -p codex-tui
cargo insta pending-snapshots -p codex-tui
just fmt
just fix -p codex-tui
```

No app-server protocol tests or schema generation are expected for this feature unless the
implementation accidentally touches shared protocol crates, which would be outside the V1 contract.

## Coverage Gaps

- Live-tail preservation across inactive switches is out of scope for v1.
- Memory limits for cached transcript cells may need tuning after real usage.
- The V1 conservative buffered-event fallback may leave safe replay optimizations for a later
  iteration.
