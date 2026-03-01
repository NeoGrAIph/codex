# Close Agent Subtree Ownership And Cascade Shutdown

## Summary

This feature hardens `close_agent` semantics for hierarchical agents and makes shutdown behavior
consistent for thread-spawn subtrees.

It introduces two runtime guarantees:

- ownership checks for `close_agent` in sub-agent context (`self` or descendants only),
- cascade shutdown of descendant agents when closing a parent agent.

## Contract

## Ownership checks (`close_agent`)

The ownership check is applied only when the caller is a spawned sub-agent:

- `SessionSource::SubAgent(SubAgentSource::ThreadSpawn { .. })`.

In this context, `close_agent(id=...)` is allowed only when:

- `id` is the caller thread itself, or
- `id` is a descendant of the caller thread in the live `ThreadSpawn` tree.

If the target is outside the caller subtree (for example sibling or parent):

- the tool returns `RespondToModel("Not permitted to close agents outside your subtree.")`,
- `CollabCloseEndEvent` is emitted with `AgentStatus::Errored(...)`.

Outside sub-agent context (for example top-level/user thread), behavior is unchanged.

## Cascade shutdown

`AgentControl::shutdown_agent(agent_id)` now shuts down the whole `ThreadSpawn` subtree:

1. collect descendants for `agent_id`,
2. shut them down leaf-first (reverse traversal order),
3. shut down `agent_id`.

For each closed thread:

- `Op::Shutdown` is submitted,
- thread is removed from `ThreadManagerState`,
- spawned-thread reservation is released from `AgentControlState`.

## Tree discovery and traversal

Descendants are built from live thread metadata where:

- thread `session_source` is `SubAgent(ThreadSpawn { parent_thread_id, .. })`.

Traversal details:

- parent -> children map is built from active threads,
- each children list is sorted by `thread_id.to_string()` for deterministic ordering,
- DFS collects descendants recursively.

This logic is exposed via:

- `list_thread_spawn_descendants(parent_thread_id)`,
- `is_descendant_of(ancestor, candidate)`.

## Error handling

Cascade close is resilient to expected races in descendant shutdown:

- `ThreadNotFound(_)` and `InternalAgentDied` are ignored for descendants.

Other descendant errors:

- first non-race error is remembered and returned after parent close succeeds.

Parent close errors remain authoritative:

- if parent shutdown fails, that error is returned immediately.

## Backward compatibility

- No protocol schema changes are required.
- Existing close behavior for non-sub-agent callers remains intact.
- Ownership boundaries are stricter only for spawned sub-agents.

## Tests

Added integration tests for `multi_agents` handler:

- `close_agent_cascades_to_descendants`
- `close_agent_rejects_cross_subtree_shutdown_for_subagents`

Existing `close_agent_submits_shutdown_and_returns_status` remains valid.

## Related Features

- `docs/features/threadspawn-agent-persona.md`
- `docs/features/agent-role-templates.md`
