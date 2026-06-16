# Spawn agent cwd verification

## Required checks

- `cargo check -p codex-core -p codex-thread-store -p codex-protocol -p codex-rollout`: compile affected source crates.
- `cargo check --tests -p codex-core`: compile core tests and fixture updates.
- `just fmt`: format changed Rust code from `codex-rs`.
- `just test -p codex-core spawn_agent_tool_v2_requires_task_name_and_lists_visible_models spawn_agent_tool_v1_keeps_legacy_fork_context_field`: schema visibility tests.
- `just test -p codex-core multi_agent_v2_spawn_applies_cwd_and_thread_note_without_widening_permissions multi_agent_v2_spawn_rejects_cwd_outside_workspace_roots multi_agent_v2_spawn_rejects_nonexistent_cwd_inside_workspace_roots`: focused cwd positive/negative tests.
- `git diff --check`: whitespace check.

## Scenario matrix

- Positive cwd: spawn MAv2 child inside parent workspace root; assert child `TurnEnvironmentSelections.legacy_fallback_cwd`, workspace roots, and permission profile.
- Schema: MAv2 exposes `cwd`; V1 does not.
- Negative cwd: spawn MAv2 child outside workspace roots; assert fail-fast model error and no silent fallback to parent cwd.
- Negative missing cwd: spawn MAv2 child with nonexistent cwd inside workspace roots; assert fail-fast model error before spawn.
- Regression: existing relative `send_message` path resolution still works after spawn.

## Known gaps

No app-server/TUI/manual resume verification yet; those surfaces are intentionally unaffected in this iteration.
