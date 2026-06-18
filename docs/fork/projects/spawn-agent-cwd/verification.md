# Spawn agent cwd verification

## Required checks

- `cargo check -p codex-core -p codex-thread-store -p codex-protocol -p codex-rollout`: compile affected source crates.
- `cargo check --tests -p codex-core`: compile core tests and fixture updates.
- `just fmt`: format changed Rust code from `codex-rs`.
- `just test -p codex-core spawn_agent_tool_v2_requires_task_name_and_lists_visible_models spawn_agent_tool_v1_keeps_legacy_fork_context_field`: schema visibility tests.
- `just test -p codex-core multi_agent_v2_spawn_applies_cwd_and_thread_note_without_widening_permissions multi_agent_v2_spawn_rejects_cwd_outside_workspace_roots multi_agent_v2_spawn_rejects_nonexistent_cwd_inside_workspace_roots multi_agent_v2_spawn_rejects_file_cwd_inside_workspace_roots multi_agent_v2_spawn_rejects_symlink_cwd_escape`: focused cwd positive/negative tests.
- `git diff --check`: whitespace check.

## Scenario matrix

- Positive cwd: spawn MAv2 child inside parent workspace root; assert child `TurnEnvironmentSelections.legacy_fallback_cwd`, workspace roots, and permission profile.
- Schema: MAv2 exposes `cwd`; V1 does not.
- Negative cwd: spawn MAv2 child outside workspace roots; assert fail-fast model error and no silent fallback to parent cwd.
- Negative missing cwd: spawn MAv2 child with nonexistent cwd inside workspace roots; assert fail-fast model error before spawn.
- Negative file cwd: spawn MAv2 child with an existing file inside workspace roots; assert fail-fast model error before spawn.
- Negative symlink escape: spawn MAv2 child with cwd that is lexically inside the workspace but canonically resolves outside workspace roots; assert fail-fast model error before spawn.
- Regression: existing relative `send_message` path resolution still works after spawn.

## Known gaps

No app-server/TUI/manual resume verification yet; those surfaces are intentionally unaffected in this iteration.

## Verification log

- 2026-06-18: `just test -p codex-core spawn_agent_tool_v2_requires_task_name_and_lists_visible_models spawn_agent_tool_v1_keeps_legacy_fork_context_field multi_agent_v2_spawn_applies_cwd_and_thread_note_without_widening_permissions multi_agent_v2_spawn_rejects_cwd_outside_workspace_roots multi_agent_v2_spawn_rejects_nonexistent_cwd_inside_workspace_roots multi_agent_v2_spawn_rejects_thread_note_over_500_chars` прошёл 6/6. Positive cwd, schema visibility and invalid/missing cwd fail-fast behavior confirmed through native MAv2/core tests.
- 2026-06-18: `just fmt`; `just test -p codex-core multi_agent_v2_spawn_applies_cwd_and_thread_note_without_widening_permissions multi_agent_v2_spawn_rejects_cwd_outside_workspace_roots multi_agent_v2_spawn_rejects_nonexistent_cwd_inside_workspace_roots multi_agent_v2_spawn_rejects_symlink_cwd_escape` прошёл 4/4. Canonical symlink-escape guard confirmed without changing the existing outside/missing cwd fail-fast behavior.
- 2026-06-18: added explicit regular-file cwd regression test after audit gap review; `just fmt` and `just test -p codex-core multi_agent_v2_spawn_applies_cwd_and_thread_note_without_widening_permissions multi_agent_v2_spawn_rejects_cwd_outside_workspace_roots multi_agent_v2_spawn_rejects_nonexistent_cwd_inside_workspace_roots multi_agent_v2_spawn_rejects_file_cwd_inside_workspace_roots multi_agent_v2_spawn_rejects_symlink_cwd_escape` passed 5/5.
- 2026-06-18: Spark test-runner current-state rerun confirmed schema and cwd coverage together: `just test -p codex-core spawn_agent_tool_v2_requires_task_name_and_lists_visible_models spawn_agent_tool_v1_keeps_legacy_fork_context_field multi_agent_v2_spawn_applies_cwd_and_thread_note_without_widening_permissions multi_agent_v2_spawn_rejects_cwd_outside_workspace_roots multi_agent_v2_spawn_rejects_nonexistent_cwd_inside_workspace_roots multi_agent_v2_spawn_rejects_file_cwd_inside_workspace_roots multi_agent_v2_spawn_rejects_symlink_cwd_escape` passed 7/7, and scoped `git diff --check` passed.
