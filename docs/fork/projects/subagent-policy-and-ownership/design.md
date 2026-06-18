# Subagent policy and ownership design

## Canonical state

Ownership is derived from canonical agent paths, not from a new ownership table. The current caller path comes from `SessionSource::get_agent_path()` or the author thread's stored sub-agent source metadata, and the target path comes from live `AgentMetadata` or target thread source metadata.

The first policy-control-plane slice adds a durable action-policy substrate, not a new enforcement rule. The native carrier is `SubAgentSource::ThreadSpawn.action_policy`: a versioned snapshot computed during spawn from the effective child configuration after role application. In v1 the only runtime mode is `default`, which preserves current behavior. The snapshot exists so future `read_only`, `allow_list` and `deny_list` semantics have one persisted source to extend, instead of adding TUI-only state or a parallel policy registry.

## Data flow

`interrupt_agent` resolves the target through the existing agent resolver, ensures the target is a non-root spawned agent, rejects self-interrupt, then checks that root may interrupt any non-root target while non-root agents may only interrupt descendants of their own path.

Workbench/app-server `close one` follows the same ownership shape through `ThreadManager::close_agent_from_workbench`: it resolves author and target thread metadata, rejects self/root/pathless targets, checks that the target path is a descendant of the author path, then delegates the lifecycle mutation to native `AgentControl::close_agent`. The workbench TUI performs the same preflight for UX, but server-side path ownership is the authority.

Action-policy snapshot flow: role-applied native config -> effective child `Config` -> `SubAgentActionPolicySnapshot` -> `SubAgentSource::ThreadSpawn` -> rollout/session metadata -> app-server thread projection. The snapshot is projection-only in this slice. It does not change TUI action availability, tool planning, permissions, sandboxing or MCP behavior.

## Invariants

- Root keeps existing orchestration capability.
- A sub-agent can interrupt a descendant target inside its own canonical `AgentPath` subtree.
- A sub-agent can close a descendant target inside its own canonical `AgentPath` subtree only through the app-server/workbench close-one path.
- A sub-agent cannot use an absolute `/root/sibling` target to control a sibling subtree.
- A sub-agent cannot interrupt itself; existing diagnostic remains unchanged.
- A sub-agent cannot close itself, root, a pathless thread or a sibling subtree through workbench close.
- The guard uses existing paths and thread IDs and does not add a duplicated ownership model.

## Tradeoffs

This stage covers the active MAv2 interrupt control path and the TUI/app-server workbench close-one path. It deliberately does not claim full policy enforcement, role-template policy metadata or V1 legacy close-tool parity until role/policy metadata is propagated through the native tool and app-server surfaces.

The first action-policy snapshot deliberately does not expose `read_only`, `allow_list` or `deny_list` user settings. Those names look like security policy, so they must wait until the fork defines stable action ids, inheritance/precedence and server-side denial behavior. Adding them before enforcement would create a silent security footgun.
