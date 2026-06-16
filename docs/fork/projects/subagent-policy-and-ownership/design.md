# Subagent policy and ownership design

## Canonical state

Ownership is derived from canonical agent paths, not from a new ownership table. The current caller path comes from `SessionSource::get_agent_path()` and the target path comes from live `AgentMetadata`.

## Data flow

`interrupt_agent` resolves the target through the existing agent resolver, ensures the target is a non-root spawned agent, rejects self-interrupt, then checks that root may interrupt any non-root target while non-root agents may only interrupt descendants of their own path.

## Invariants

- Root keeps existing orchestration capability.
- A sub-agent cannot use an absolute `/root/sibling` target to control a sibling subtree.
- A sub-agent cannot interrupt itself; existing diagnostic remains unchanged.
- The guard uses existing paths and thread IDs and does not add a duplicated ownership model.

## Tradeoffs

This stage covers the active MAv2 interrupt control path. It deliberately does not claim full policy enforcement or V1 close parity until role/policy metadata is propagated through the native tool and app-server surfaces.
