# Subagent policy and ownership verification

## Required checks

- `cargo check -p codex-core -p codex-thread-store -p codex-protocol -p codex-rollout`
- `cargo check --tests -p codex-core`
- `cargo check -p codex-app-server --tests`
- `cargo check -p codex-app-server-protocol --tests`
- `just write-app-server-schema`
- `just fmt`
- `just test -p codex-core multi_agent_v2_interrupt_agent_allows_descendant_target multi_agent_v2_interrupt_agent_rejects_cross_subtree_target multi_agent_v2_interrupt_agent_rejects_root_target_and_id multi_agent_v2_interrupt_agent_rejects_self_target_by_id multi_agent_v2_interrupt_agent_rejects_self_target_by_task_name`
- `just test -p codex-core thread_manager_agent_close_closes_path_backed_descendant`
- `just test -p codex-core spawn_agent_uses_explorer_role_and_preserves_approval_policy multi_agent_v2_spawn_applies_cwd_and_thread_note_without_widening_permissions`
- `just test -p codex-app-server summary_from_state_db_metadata_preserves_agent_nickname`
- `just test -p codex-tui agent_picker_close_preflight_enforces_status_and_path_ownership`
- `git diff --check`

## Scenario matrix

- Positive descendant: child `/root/worker` interrupts `/root/worker/child`; `Op::Interrupt` is sent to the descendant and the call succeeds.
- Negative cross-subtree: child `/root/worker` tries to interrupt `/root/sibling`; fail-fast error and no interrupt op is sent.
- Negative root target: existing root target denial remains intact.
- Negative self target: existing self-interrupt denial remains intact.
- Positive close descendant: workbench/app-server close one can close a path-backed spawned descendant and delegates shutdown to native `AgentControl::close_agent`.
- Negative close boundaries: main/root/self/pathless/already-closed/outside-subtree targets are denied before TUI enablement and rechecked server-side by `ThreadManager::close_agent_from_workbench`.
- Spawn policy snapshot: V1 and MAv2 spawn paths persist `SubAgentActionPolicySnapshot` in `SubAgentSource::ThreadSpawn`; role-based spawn marks `source = role_applied_config`, default/no-role spawn marks `source = default`.
- Metadata preservation: thread-note and app-server summary metadata updates preserve an existing action-policy snapshot instead of reconstructing `ThreadSpawn` without it.
- Protocol projection: app-server JSON and TypeScript schema fixtures include `SubAgentActionPolicySnapshot`, `SubAgentActionPolicyMode` and `SubAgentActionPolicySource`.

## Known gaps

No MCP allow/deny policy verification in this iteration because the new action-policy substrate is projection-only and does not add enforcement semantics. V1 legacy close-tool parity is not claimed; close ownership coverage applies to the workbench/app-server close-one path. `read_only`, allow-list and deny-list behavior remains intentionally unimplemented until stable action ids, precedence rules and server-side denial are defined.

## Verification log

- 2026-06-18: `just test -p codex-core multi_agent_v2_interrupt_agent_rejects_cross_subtree_target multi_agent_v2_interrupt_agent_rejects_root_target_and_id multi_agent_v2_interrupt_agent_rejects_self_target_by_id` прошёл 3/3. Cross-subtree, root target and self-target interrupt denials are confirmed through native MAv2 handler tests.
- 2026-06-18: Added `multi_agent_v2_interrupt_agent_allows_descendant_target` to cover the positive subtree case: a non-root caller can interrupt a descendant target and the captured op goes to the descendant thread. Spark test-runner rerun passed: 5/5 focused ownership tests, 0 failed, and scoped `git diff --check` passed.
- 2026-06-18: Spark test-runner current-state rerun confirmed the complete first-slice ownership matrix: `just test -p codex-core multi_agent_v2_interrupt_agent_allows_descendant_target multi_agent_v2_interrupt_agent_rejects_cross_subtree_target multi_agent_v2_interrupt_agent_rejects_root_target_and_id multi_agent_v2_interrupt_agent_rejects_self_target_by_id multi_agent_v2_interrupt_agent_rejects_self_target_by_task_name` passed 5/5, and scoped `git diff --check` passed.
- 2026-06-18: Workbench/app-server close-one ownership added to this dossier. Kepler rerun from the close-one slice passed `just test -p codex-core thread_manager_agent_close_closes_path_backed_descendant ...` and `just test -p codex-tui agent_picker_close_preflight_enforces_status_and_path_ownership` as part of the focused close-one verification, with no pending snapshots and scoped `git diff --check`.
- 2026-06-18: `feature/140/agent-policy-control-plane` substrate check: local `cargo check -p codex-core --tests` and `cargo check -p codex-app-server --tests` passed after adding `SubAgentSource::ThreadSpawn.action_policy`; `just write-app-server-schema` regenerated JSON/TypeScript schema artifacts for the new projection.
- 2026-06-18: Kepler test-runner passed `cargo check -p codex-protocol`, `cargo check -p codex-core --tests`, `cargo check -p codex-app-server --tests`, `cargo check -p codex-app-server-protocol --tests`, `just test -p codex-core spawn_agent_uses_explorer_role_and_preserves_approval_policy multi_agent_v2_spawn_applies_cwd_and_thread_note_without_widening_permissions` and scoped `git diff --check`; first app-server focused rerun exposed a test-only helper mismatch that was fixed by pattern matching the app-server protocol wrapper.
- 2026-06-18: Kepler rerun after the app-server test fix passed `cargo check -p codex-app-server --tests`, `just test -p codex-app-server summary_from_state_db_metadata_preserves_agent_nickname` and scoped `git diff --check`.
- 2026-06-18: Audit fix rerun passed `cargo check -p codex-core --tests`, `cargo check -p codex-app-server --tests`, `just test -p codex-core resume_agent_from_rollout_uses_edge_data_when_descendant_metadata_source_is_stale`, `just test -p codex-app-server thread_metadata_update_patches_thread_note_through_source_metadata summary_from_state_db_metadata_preserves_agent_nickname` and scoped `git diff --check`; this covers persisted action-policy retention through tree resume and app-server thread-note metadata updates.
- 2026-06-18: Peirce independent audit rerun returned `PASS_WITH_LOW`; the only remaining note was to include generated `SubAgentActionPolicy*.ts` schema files in the commit.
