# Subagent policy and ownership design

## Canonical state

Ownership is derived from canonical agent paths, not from a new ownership table. The current caller path comes from `SessionSource::get_agent_path()` or the author thread's stored sub-agent source metadata, and the target path comes from live `AgentMetadata` or target thread source metadata.

The first policy-control-plane slice adds a durable action-policy substrate. The native carrier is `SubAgentSource::ThreadSpawn.action_policy`: a versioned snapshot computed during spawn from the effective child configuration after role application. `source = config` means the snapshot came from non-role config, `source = role_applied_config` means the role-applied config supplied it, and `source = default` means no explicit action policy was configured. The snapshot exists so workbench `allowed_actions` and `denied_actions` have one persisted source instead of TUI-only state or a parallel policy registry.

`read_only` is enforced separately because Codex already has a native source of truth for it: the author thread's active `Config.permissions` profile. Agent Window mutating actions call into `ThreadManager`, and `ThreadManager` checks whether a non-root author thread carries the active `:read-only` permission profile before allowing message, follow-up, close, dismiss, retry or stop-all. Root/user orchestration keeps existing native behavior, and sandbox-only read-only execution without the active `:read-only` role/config selection is not reinterpreted as a sub-agent action policy. This keeps the rule server/runtime-side and prevents older or buggy TUI clients from bypassing it.

Repo-local role trust is not a new sub-agent policy table. It is inherited from native project config layering: `ConfigLayerStack` keeps disabled project layers for diagnostics, but `load_agent_roles` reads only enabled layers, so trusted project `.codex/agents/*.toml` files can contribute roles and untrusted/unknown project layers cannot widen permissions, tools or actions through role files. User-owned `$CODEX_HOME/agents/*.toml` and explicit user config `config_file` references remain user config, not project trust input.

Read-only `.agents` is also native sandbox state, not role runtime state. `FileSystemSandboxPolicy` treats top-level `.git`, `.agents` and `.codex` as protected metadata names under workspace-write policy and projects them as read-only carveouts where possible. Workbench policy must not reinterpret `.agents` as a writable role source.

## Data flow

MAv2 mutating tools resolve or create targets through the existing agent resolver and native `AgentPath` metadata. A real root session may act as root. A non-root `SessionSource::SubAgent` without `agent_path` is not promoted to root ownership: `spawn_agent`, `send_message`, `followup_task`, `interrupt_agent` and `set_thread_note` fail fast with a path-backed-author diagnostic before creating a new child path, queuing communication, interrupting or writing thread metadata. This preserves old/pathless sessions as readable/resumable history without letting them control sibling/root trees.

`interrupt_agent` resolves the target through the existing agent resolver, ensures the target is a non-root spawned agent, rejects self-interrupt, then checks that root may interrupt any non-root target while path-backed non-root agents may only interrupt descendants of their own path.

V1 legacy `close_agent` keeps the native `AgentControl::close_agent` mutation path but now performs the same ownership check before shutdown. Root/user callers keep existing orchestration behavior, including closing legacy pathless V1 children. Path-backed non-root callers can close only descendants of their own `AgentPath`; self, sibling/outside-subtree and pathless-author cases fail before `CollabCloseBeginEvent` and before `AgentControl::close_agent`.

Workbench/app-server mutating actions follow the same ownership shape through `ThreadManager`: each action resolves author and target thread metadata, rejects self/root/pathless targets where applicable, checks that the target path is a descendant of the author path, then delegates the native mutation. `close` and `stop all` delegate lifecycle mutation to native `AgentControl::close_agent`; `dismiss` writes the native thread visibility metadata; `retry` respawns from durable sub-agent metadata; message/follow-up use native inter-agent communication. The workbench TUI performs the same preflight for UX, but server-side path ownership and `read_only` denial are the authority.

Action-policy snapshot flow: role-applied native config -> effective child `Config` -> `SubAgentActionPolicySnapshot` -> `SubAgentSource::ThreadSpawn` -> rollout/session metadata -> app-server thread projection. Explicit workbench action allow/deny is sourced from the persisted snapshot, not from the live role file, so resume/restart and deleted/edited role files do not silently widen old agents. `read_only` workbench denial is intentionally not sourced from this snapshot; for non-root authors it is computed from the live author thread's active permission profile so role-applied permission preservation uses the same native path as command/file execution.

Action-policy management flow: external clients call experimental `agentRole/actionPolicy/set`; app-server resolves the current config, finds a discovered user role under `$CODEX_HOME/agents`, rejects built-ins and external `config_file` roles, rewrites native `[subagent_action_policy] allowed_actions` / `denied_actions` in that TOML file, validates the resulting file through the native role parser and must refresh loaded thread configs before returning success. The method is whole-policy replacement, not a partial patch. `allowed_actions = null` and `denied_actions = null` together remove the section. Existing spawned agents keep their persisted `SubAgentActionPolicySnapshot`; the write affects future spawns and next-turn config refresh boundaries.

Action ids are protocol-level enum values, not TUI-only strings: `agent_message_send`, `agent_followup_send`, `agent_close`, `agent_dismiss`, `agent_retry`, `agent_stop_all`. `allowed_actions = None` means the default action set is allowed; `allowed_actions = Some([...])` narrows to that set; `denied_actions` removes actions after allow and wins on conflicts. Missing fields in old sessions deserialize as default allow.

## Invariants

- Root keeps existing orchestration capability.
- A sub-agent can interrupt a descendant target inside its own canonical `AgentPath` subtree.
- A sub-agent can close a descendant target inside its own canonical `AgentPath` subtree through both the V1 legacy `close_agent` tool and the app-server/workbench close-one path.
- A sub-agent cannot use an absolute `/root/sibling` target to control a sibling subtree.
- A sub-agent cannot interrupt itself; existing diagnostic remains unchanged.
- A sub-agent cannot close itself, root, a pathless thread or a sibling subtree through workbench close.
- A non-root author with active `:read-only` permission profile cannot trigger Agent Window mutations: message, follow-up, close, dismiss, retry or stop-all fail before any target mutation.
- A non-root author whose persisted action-policy snapshot denies a workbench action cannot trigger that action even if TUI/app-server sends it.
- A non-root author with an `allowed_actions` list can trigger only listed workbench actions; `denied_actions` wins on conflicts.
- Project-local role files from untrusted or unknown project layers do not appear in the effective role catalog.
- Workspace-write sandbox policy blocks writes to top-level `.agents` protected metadata paths unless a future explicit contract deliberately changes that native policy.
- The guard uses existing paths and thread IDs and does not add a duplicated ownership model.

## Tradeoffs

This stage covers the active MAv2 interrupt control path, V1 legacy `close_agent` subtree ownership, TUI/app-server workbench mutating action boundaries, role/project trust gating and read-only metadata protection. MCP allow/deny is not implemented here as a second action-policy registry; it is covered by the native role `[tool_selection]` bridge once that policy is propagated through tool planning, `ToolRegistry` and direct app-server MCP execution.

The first action-policy snapshot deliberately starts with workbench actions only. It does not claim MCP/tool policy, command permissions or sandboxing. This avoids a broad security claim before the native consumers are wired. `read_only` is separate because it is already a native enforced permission profile; the workbench slice consumes that existing source of truth instead of creating new policy syntax.

## Production-ready v2 tool policy bridge

Tool/MCP allow/deny enforcement should not be added to `SubAgentActionPolicySnapshot` as a second runtime authority. The native bridge is role-applied `[tool_selection]`:

- `allowed_tools` narrows the tool set when present;
- `denied_tools` removes tools after allow and wins on conflicts;
- both fields parse through `ConfigToml`/`Config` into `TurnContext.config.tool_selection`;
- `spec_plan::apply_tool_selection_policy` filters `PlannedTools` before `tool_search`, Code Mode and `ToolRegistry` projections.

This makes tool policy automatically visible to every native consumer that derives from the planner: model-visible specs, deferred discovery, Code Mode nested tools, dispatch registry and loaded-thread app-server catalog. Workbench/TUI surfaces may display the effective summary, but mutating policy remains native TOML/config or a future app-server write API that writes the same native source.

Direct app-server MCP execution is also a native consumer of the same policy. `mcpServer/tool/call` reaches `Session::call_tool` and is checked against the loaded thread's effective `Config.tool_selection` before `McpConnectionManager::call_tool`; canonical MCP names come from `ToolInfo::canonical_tool_name()` in the native inventory, not from a second app-server registry or handcrafted `mcp__` naming.

`SubAgentActionPolicySnapshot` can summarize effective workbench action policy for persistence/protocol diagnostics, but it must not override `Config`, `TurnContext`, `ToolRegistry`, permissions, sandbox, MCP manager, Guardian or approval policy. `agentRole/actionPolicy/set` therefore writes native role TOML instead of editing snapshots. `read_only` follows the same rule: it compiles into native permission/sandbox state for the child thread through role-applied `default_permissions = ":read-only"` and active permission profile metadata before a TUI action or app-server API claims the agent is read-only. Spawn handlers may reapply live approval/cwd/sandbox context after role application, but that refresh must preserve the role-narrowed permission profile.
