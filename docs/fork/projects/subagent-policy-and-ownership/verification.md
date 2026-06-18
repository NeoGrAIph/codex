# Subagent policy and ownership verification

## Required checks

- `cargo check -p codex-core -p codex-thread-store -p codex-protocol -p codex-rollout`
- `cargo check --tests -p codex-core`
- `just fmt`
- `just test -p codex-core multi_agent_v2_interrupt_agent_allows_descendant_target multi_agent_v2_interrupt_agent_rejects_cross_subtree_target multi_agent_v2_interrupt_agent_rejects_root_target_and_id multi_agent_v2_interrupt_agent_rejects_self_target_by_id multi_agent_v2_interrupt_agent_rejects_self_target_by_task_name`
- `just test -p codex-core thread_manager_agent_close_closes_path_backed_descendant`
- `just test -p codex-tui agent_picker_close_preflight_enforces_status_and_path_ownership`
- `git diff --check`

## Scenario matrix

- Positive descendant: child `/root/worker` interrupts `/root/worker/child`; `Op::Interrupt` is sent to the descendant and the call succeeds.
- Negative cross-subtree: child `/root/worker` tries to interrupt `/root/sibling`; fail-fast error and no interrupt op is sent.
- Negative root target: existing root target denial remains intact.
- Negative self target: existing self-interrupt denial remains intact.
- Positive close descendant: workbench/app-server close one can close a path-backed spawned descendant and delegates shutdown to native `AgentControl::close_agent`.
- Negative close boundaries: main/root/self/pathless/already-closed/outside-subtree targets are denied before TUI enablement and rechecked server-side by `ThreadManager::close_agent_from_workbench`.

## Known gaps

No MCP allow/deny policy verification in this iteration because policy metadata propagation is not changed. V1 legacy close-tool parity is not claimed; close ownership coverage applies to the workbench/app-server close-one path.

## Verification log

- 2026-06-18: `just test -p codex-core multi_agent_v2_interrupt_agent_rejects_cross_subtree_target multi_agent_v2_interrupt_agent_rejects_root_target_and_id multi_agent_v2_interrupt_agent_rejects_self_target_by_id` прошёл 3/3. Cross-subtree, root target and self-target interrupt denials are confirmed through native MAv2 handler tests.
- 2026-06-18: Added `multi_agent_v2_interrupt_agent_allows_descendant_target` to cover the positive subtree case: a non-root caller can interrupt a descendant target and the captured op goes to the descendant thread. Spark test-runner rerun passed: 5/5 focused ownership tests, 0 failed, and scoped `git diff --check` passed.
- 2026-06-18: Spark test-runner current-state rerun confirmed the complete first-slice ownership matrix: `just test -p codex-core multi_agent_v2_interrupt_agent_allows_descendant_target multi_agent_v2_interrupt_agent_rejects_cross_subtree_target multi_agent_v2_interrupt_agent_rejects_root_target_and_id multi_agent_v2_interrupt_agent_rejects_self_target_by_id multi_agent_v2_interrupt_agent_rejects_self_target_by_task_name` passed 5/5, and scoped `git diff --check` passed.
- 2026-06-18: Workbench/app-server close-one ownership added to this dossier. Kepler rerun from the close-one slice passed `just test -p codex-core thread_manager_agent_close_closes_path_backed_descendant ...` and `just test -p codex-tui agent_picker_close_preflight_enforces_status_and_path_ownership` as part of the focused close-one verification, with no pending snapshots and scoped `git diff --check`.
