# Subagent policy and ownership verification

## Required checks

- `cargo check -p codex-core -p codex-thread-store -p codex-protocol -p codex-rollout`
- `cargo check --tests -p codex-core`
- `just fmt`
- `just test -p codex-core multi_agent_v2_interrupt_agent_rejects_cross_subtree_target multi_agent_v2_interrupt_agent_rejects_root_target_and_id multi_agent_v2_interrupt_agent_rejects_self_target_by_id`
- `git diff --check`

## Scenario matrix

- Negative cross-subtree: child `/root/worker` tries to interrupt `/root/sibling`; fail-fast error and no interrupt op is sent.
- Negative root target: existing root target denial remains intact.
- Negative self target: existing self-interrupt denial remains intact.

## Known gaps

No MCP allow/deny policy verification in this iteration because policy metadata propagation is not changed.
