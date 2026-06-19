# Production-ready design contract

## Guiding rule

Every deferred feature must become a native Codex contract. The implementation must extend the existing source of truth for the capability it changes, then let existing projections inherit the new behavior. If a native extension point is insufficient, add the smallest upstream-shaped extension point and document why it is required.

## Source-of-truth map

| Capability | Current native sources | Production-ready requirement |
| --- | --- | --- |
| Agent identity and ownership | `SessionSource::SubAgent`, `SubAgentSource::ThreadSpawn`, `AgentPath`, `ThreadManager`, `AgentControl` | All mutating workbench actions and policy checks use the same root/self/descendant/sibling/pathless ownership rules server-side. |
| Agent visibility | `AgentNavigationState`, thread metadata/projections, app-server thread list/read surfaces | `dismiss` uses rollout `SessionMeta.agent_hidden`, thread-store `ThreadMetadataPatch.agent_hidden`, app-server `Thread.agentHidden` and Agent Window filtering; it does not reuse `close` or `archive` semantics. |
| Agent lifecycle | MAv2 `spawn_agent`, `send_message`, `followup_task`, `interrupt_agent`, native `AgentControl::close_agent` | `retry` and `stop all` need explicit contracts for inherited cwd, role, provider/model, tool policy, sandbox, failure behavior and confirmation. |
| Role definitions | Built-ins, `$CODEX_HOME/agents/*.toml`, `ConfigToml`, `AgentRoleConfig`, role parser/loader | Markdown/frontmatter/persona inputs compile into validated native role config; runtime does not read a parallel profile format. Built-in `reviewer` is a normal bundled role and applies read-only defaults through the same native config path. |
| Role tool policy | `[tool_selection]`, `Config.agent_roles`, `spec_plan`, `ToolRegistry`, MCP direct/deferred exposure | Add native allow/deny semantics and external management without widening permissions or bypassing planner-owned filtering. |
| Sub-agent action policy | `SubAgentActionPolicySnapshot`, `SessionSource`, tool/runtime handlers, user role TOML | Current slices implement `read_only`, workbench action allow/deny, trust gating, read-only `.agents` enforcement and `agentRole/actionPolicy/set` over native config/runtime paths. MCP allow/deny belongs to the generic role `[tool_selection]` contract and is managed/enforced there, not through a second action-policy or MCP-only registry. |
| Protocol/schema | app-server protocol v2, generated JSON/TypeScript schema | New external write/management APIs must be typed, gated where experimental, schema-generated and compatibility-documented. |
| TUI | slash dispatch, `AgentNavigationState`, workbench read model, role template popups | UI actions are projections of runtime contracts, not the source of truth. Snapshots must cover compact, empty, running, waiting, error and confirmation states. |

## Slice order

1. Define and implement the policy enforcement substrate before adding new lifecycle buttons. The policy slices now prove `read_only`, action allow/deny, project role trust gating and read-only `.agents` metadata through native server/runtime checks.
2. Extend role tool-selection semantics from allowlist-only to a typed allow/deny policy in native config and `spec_plan`, including MCP direct/deferred behavior.
3. Add app-server write/management API only after the native role/tool policy exists and has parser/config tests. `agentRole/toolSelection/set` and `agentRole/actionPolicy/set` now follow this rule by writing native discovered user role TOML instead of snapshots or parallel registries.
4. Add role markdown/frontmatter/persona import as compile-to-native-TOML authoring. Runtime role loading remains native TOML/config. The current slices also add built-in `reviewer` as a bundled config-backed role, not a runtime enum.
5. Add workbench visibility/lifecycle actions: dismiss, retry and stop all are implemented through native thread metadata/session source, app-server `agent/*` methods and `AgentControl` close/spawn paths. Connect/switch copy and repeated Ctrl+T from transcript overlay use existing native picker paths. Bounded role/profile hydration is implemented from native thread/session snapshots (`Model`, `Reasoning`, `Service tier`, `Tool policy`, `Action policy`); remaining workbench work should focus only on explicit new fields or targeted UX polish without adding a second lifecycle state.
6. Run cross-feature acceptance against resume/restart and mixed surfaces.

## Open design questions

- What exact footer/help copy should describe the repeated `Ctrl+T` transcript-to-Agent-Window behavior, if any, without obscuring that first-press `Ctrl+T` remains native transcript?
- What is the recovery UX for dismissed agents beyond native thread history/read/resume paths? First slice intentionally has no undismiss UI.
- Does `read_only` block all mutating tools, only file/process/network tools, or role/action-specific categories? This must be encoded in native tool/action policy, not TUI copy.
- Which remaining role tool-selection retention cases need additional snapshot/origin metadata when the source role file is deleted or changed? Action policy already uses durable effective snapshots for spawned agents while management writes affect future spawns/config refresh boundaries.
