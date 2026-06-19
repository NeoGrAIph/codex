# Subagent policy and ownership

## Feature passport

- Code name: `subagent-policy-and-ownership`
- Status: переносимая security/ownership возможность.
- Goal: не дать sub-agent обходить allow/deny policies, trust boundaries и ownership своего subtree.
- Scope in: current `fork/140` MAv2 interrupt ownership guard, V1 legacy `close_agent` ownership guard, workbench/app-server ownership guard for mutating Agent Window actions, canonical `AgentPath` subtree checks, root/non-root/self/cross-subtree diagnostics, server-side `read_only` denial for workbench mutation, explicit workbench action allow/deny policy through persisted `SubAgentActionPolicySnapshot`, app-server `agentRole/actionPolicy/set` write/management API for writable user role TOML, role/project trust gating through native `ConfigLayerStack`, read-only protected `.agents` metadata paths through native filesystem sandbox policy, and focused ownership/security tests. MCP allow/deny is intentionally handled by the generic role `[tool_selection]` contract in `agent-role-tool-selection`, not by a separate sub-agent action policy registry.
- Scope out: пользовательские role templates и cwd, кроме мест пересечения.

## Как работает для пользователя

Sub-agent получает права в рамках контекста, из которого был создан. Польза: multi-agent работа безопаснее, потому что agent не должен использовать запрещенные tools, закрывать чужие branches или затрагивать не свой контекст.

## Branches and commits

| Branch | Baseline | Commit | Date | Subject | Роль |
| --- | --- | --- | --- | --- | --- |
| `fork/101` | `rust-v0.101.0` | `3ce86025b8` | 2026-02-18 | `fix(core): honor mcp allowlist in apps tool selection` | Apps tool selection учитывает MCP allowlist. |
| `fork/105` | `rust-v0.105.0` | `935e4739d5` | 2026-02-26 | `feat(threadspawn): harden protocol contract and metadata propagation` | База propagation для policy metadata. |
| `fork/106` | `rust-v0.106.0` | `a6350283d3` | 2026-02-27 | `feat(threadspawn): preserve policy metadata across runtime boundaries` | Policy metadata переносится через runtime boundaries. |
| `fork/106` | `rust-v0.106.0` | `c0143191f5` | 2026-02-27 | `fix(core): enforce ThreadSpawn MCP allow/deny policy in apps mode` | Enforcement allow/deny для ThreadSpawn в apps mode. |
| `fork/106` | `rust-v0.106.0` | `3d23e9ffaf` | 2026-03-01 | `feat(sa): enforce subtree close ownership and bump spawn depth default` | Ownership close subtree + depth default. |
| `fork/107` | `rust-v0.107.0` | `2311a1a7b5` | 2026-02-27 | `feat(threadspawn): preserve policy metadata across runtime boundaries` | Перенос metadata propagation. |
| `fork/107` | `rust-v0.107.0` | `be70eb5513` | 2026-02-27 | `fix(core): enforce ThreadSpawn MCP allow/deny policy in apps mode` | Перенос enforcement. |
| `fork/107` | `rust-v0.107.0` | `d4bd1f85f4` | 2026-03-01 | `feat(sa): enforce subtree close ownership and bump spawn depth default` | Перенос ownership. |
| `fork/111` | `rust-v0.111.0` | `0afcd81113` | 2026-03-07 | `fix(core): enforce threadspawn allow deny tool policy` | Enforcement в обновленной ветке. |
| `fork/111` | `rust-v0.111.0` | `eee6f8a54a` | 2026-03-09 | `feat(core): add subtree close ownership foundation` | Foundation для ownership. |
| `fork/130` | `rust-v0.130.0` | `783ebf1fac` | 2026-05-14 | `feat(agents): add markdown role templates` | Markdown templates несут `read_only`, `allow_list`, `deny_list`; policy runtime-only/server-side. |
| `fork/colab-agents` | `rust-v0.98.0` audit lineage | `73681a6e51` | 2026-02-06 | `fix(core): enforce trust and tool policy for subagents` | Trust/tool policy audit fix. |
| `fork/colab-agents` | `rust-v0.98.0` audit lineage | `2107dc4850` | 2026-02-06 | `fix(core): restore requirements provenance and fallback defaults` | Requirements provenance/defaults. |
| `fork/colab-agents` | `rust-v0.98.0` audit lineage | `5ab2538812` | 2026-02-06 | `fix(security): trust-gate project layers and rules` | Trust-gating project layers/rules. |
| `fork/colab-agents` | `rust-v0.98.0` audit lineage | `f731f3db0f` | 2026-02-06 | `protocol: make .agents read-only in sandbox` | `.agents` read-only в sandbox. |
| `fork/multi-agent` | `fork/colab-agents` lineage | `cbda41c980` | 2026-02-07 | `core: restrict close_agent to caller subtree` | Close ownership enforcement. |

## Implementation notes

Затрагивались core agent control/guards, tool policy, MCP/apps tool selection, sandbox protocol, requirements provenance, trust checks и tests.

## Native coverage in rust-v0.140.0

Status: `partial`. Native release имеет protected metadata paths (`.git`, `.agents`, `.codex`), workspace-write protected subpaths, MCP `ToolFilter` with `enabled_tools`/`disabled_tools`, requirements/identity filtering for MCP servers, root-tree scoped `AgentControl`, subagent metadata and close handler. Текущий `fork/140` добавляет path-based ownership для MAv2 `spawn_agent`, `send_message`, `followup_task`, `interrupt_agent`, `set_thread_note`, V1 legacy `close_agent` and workbench/app-server mutating actions through native `AgentPath` metadata and `ThreadManager`/tool-handler guards; pathless non-root authors are denied instead of being treated as root owners. `feature/140/agent-policy-control-plane` добавляет durable `action_policy` snapshot в `SubAgentSource::ThreadSpawn`. Explicit action allow/deny semantics are scoped to workbench actions in this slice: `agent_message_send`, `agent_followup_send`, `agent_close`, `agent_dismiss`, `agent_retry`, `agent_stop_all`; deny wins over allow, and missing lists preserve default behavior. Role-level `read_only` is enforced through native role-applied config (`default_permissions = ":read-only"` -> active `:read-only` permission profile) and workbench mutation now checks a non-root author thread's active permission profile before message/follow-up/close/dismiss/retry/stop-all. `agentRole/actionPolicy/set` is implemented as an experimental app-server whole-policy write API for discovered user roles under `$CODEX_HOME/agents/*.toml`: it writes native `[subagent_action_policy]` fields, rejects built-ins and external `config_file` roles, validates through the native role parser, and must refresh loaded thread configs before returning success without mutating persisted snapshots of already spawned agents. Project-local role trust gating is inherited from native project config layers: untrusted/unknown `.codex` layers are disabled, `load_agent_roles` reads only enabled layers, and trusted project `.codex/agents/*.toml` roles are the only repo-local roles loaded. `.agents` is protected by native filesystem sandbox metadata rules, so workspace-write agents cannot write top-level `.agents` without an explicit compatible policy change. Root/user orchestration and ordinary sandbox-only read-only execution keep existing native behavior. Role-level MCP allow/deny management and enforcement are covered by the same native `[tool_selection]` policy used by `spec_plan`, `/agent-roles` catalog-assisted authoring, `agentRole/toolSelection/set` and direct external `mcpServer/tool/call`; the direct call path resolves `ToolInfo::canonical_tool_name()` through `McpConnectionManager` before execution.

## Porting/current-state notes

Это security-sensitive функционал. При переносе нужно обязательно проверять both positive and negative cases: разрешенный sub-agent action работает, запрещенный action fail-fast с диагностикой.

## Production-ready v2 policy direction

Первый production-ready enforcement substrate для tool/MCP allow/deny должен идти через native `[tool_selection]` role-applied config, а не через отдельный subagent policy registry. `allowed_tools` выбирает допустимые canonical `ToolName`, `denied_tools` удаляет entries после allow и выигрывает при конфликте. Enforcement authority остаётся `codex-rs/core/src/tools/spec_plan.rs`: filtered `PlannedTools` затем порождает model-visible specs, deferred `tool_search`, Code Mode projection и `ToolRegistry`.

`SubAgentActionPolicySnapshot` остаётся durable summary/projection for spawned sub-agent state. Он может фиксировать effective policy metadata для protocol/persistence/TUI, но не должен становиться отдельным enforcement source, который расходится с `Config`, `TurnContext`, `ToolRegistry`, permissions или sandbox.

App-server management для role action policy пишет тот же native source of truth, а не snapshot: `agentRole/actionPolicy/set` заменяет `[subagent_action_policy] allowed_actions` / `denied_actions` в discovered user role TOML under `$CODEX_HOME/agents`, а `null`/absent для обоих списков удаляет секцию. Existing spawned agents retain their persisted `SubAgentActionPolicySnapshot`; update affects future spawns and loaded thread config refresh boundaries, and reload failure is a returned app-server error rather than a warning-only fallback.

`read_only` реализуется через native permission/sandbox config path effective child thread, а не как UI-only flag: role config sets `default_permissions = ":read-only"` and the child config carries active `:read-only` permission profile metadata. Spawn handlers may refresh live approval/cwd/sandbox context after role application, but must not overwrite the role-narrowed permission profile. Workbench mutating actions must deny a read-only non-root author in `ThreadManager` before TUI/app-server success is possible; TUI preflight is only UX. `.agents` остаётся protected metadata/read-only sandbox concern и не становится runtime role source. Trust gating для repo-local roles использует native project layer trust: trusted `.codex/agents` роли могут сужать capabilities, а untrusted/unknown project layers не попадают в effective role catalog и не могут silent-widen permissions/tool access.

Direct app-server MCP execution follows the same native policy bridge: `mcpServer/tool/call` must not bypass role-applied `[tool_selection]`. The enforcement belongs in core `Session::call_tool`, before `McpConnectionManager::call_tool`, and uses MCP inventory canonical names instead of a duplicated app-server allow/deny list.

## Fork/140 implementation status

Первые `fork/140` slices добавляют native ownership guard для MAv2 `interrupt_agent`, V1 legacy `close_agent` and workbench/app-server mutating actions: root может управлять non-root agents, а sub-agent может работать только с targets внутри своего canonical `agent_path` subtree; descendant target разрешён, а sibling/root/self/pathless обходы fail-fast. `feature/140/agent-policy-control-plane` добавляет persisted action-policy snapshot как extension point; role-template `read_only` uses native permission config application, survives post-role spawn runtime refresh and denies workbench mutation from read-only authors. Explicit workbench action allow/deny now uses the same persisted action-policy snapshot, and `agentRole/actionPolicy/set` provides a typed experimental app-server write path for the native user role TOML source. Project-local roles are trust-gated by native config layers, `.agents` stays read-only through native protected metadata sandbox rules, and MCP allow/deny stays on the native role `[tool_selection]` path instead of becoming a separate sub-agent policy engine.

## Doc changelog

- 2026-06-17: Зафиксирован fork/140 first iteration: MAv2 cross-subtree interrupt denial через `AgentControl` metadata/`SessionSource`, без parallel ownership store.
- 2026-06-18: Focused MAv2 ownership verification passed for cross-subtree, root target and self-target interrupt denial.
- 2026-06-18: Добавлен positive descendant ownership test: non-root agent can interrupt a descendant inside its own `AgentPath` subtree while sibling/root/self targets remain denied.
- 2026-06-18: Workbench/app-server `close one` добавлен в ownership scope: `ThreadManager::close_agent_from_workbench` enforces author/target `AgentPath` descendant ownership before delegating to native `AgentControl::close_agent`.
- 2026-06-18: `feature/140/agent-policy-control-plane` scope narrowed to durable projection-only `SubAgentActionPolicySnapshot` before any `read_only`/allow/deny enforcement, чтобы не создавать ложный security contract.
- 2026-06-18: `feature/140/agent-policy-control-plane` реализовал projection-only `SubAgentSource::ThreadSpawn.action_policy`, сохраняемый через spawn, resume/tree-resume, thread-note metadata update и app-server thread projections.
- 2026-06-18: Production-ready direction clarified: tool/MCP allow/deny enforcement belongs in native role-applied `[tool_selection]` and `spec_plan`; `SubAgentActionPolicySnapshot` remains persisted projection/summary, not an alternate enforcement engine.
- 2026-06-18: read-only role enforcement clarified and covered through native role-applied `default_permissions = ":read-only"` resolving to `PermissionProfile::read_only()`; no UI-only policy flag or alternate action-policy engine was added.
- 2026-06-18: read-only spawn regression closed: v1 and MAv2 spawn now refresh live runtime context after role application without overwriting the role-applied `PermissionProfile::read_only()`.
- 2026-06-18: direct app-server MCP execution closed as a policy bypass: `mcpServer/tool/call` now checks the loaded thread's effective `[tool_selection]` using `ToolInfo::canonical_tool_name()` before delegating to MCP execution.
- 2026-06-19: Workbench mutation `read_only` slice defined: `ThreadManager` must deny message/follow-up/close/dismiss/retry/stop-all when a non-root author thread carries the active native `:read-only` permission profile; this uses native `Config.permissions`, not `SubAgentActionPolicySnapshot` or TUI-only state.
- 2026-06-19: V1 legacy `close_agent` parity added: the tool now checks `AgentPath` ownership before calling `AgentControl::close_agent`; path-backed non-root authors can close descendants, but self and sibling targets fail before shutdown.
- 2026-06-19: Explicit workbench action allow/deny slice defined: role-applied config compiles into persisted `SubAgentActionPolicySnapshot`; non-root author checks use that snapshot in `ThreadManager`, with `denied_actions` winning over `allowed_actions` and missing lists preserving default behavior.
- 2026-06-19: Trust/read-only metadata slice verified: repo-local `.codex/agents/*.toml` roles load only from trusted project layers, untrusted/unknown project layers remain disabled, and native filesystem sandbox policy keeps `.agents` under protected metadata read-only carveouts.
- 2026-06-19: Added `agentRole/actionPolicy/set` as the experimental app-server write/management API for native user role `[subagent_action_policy]`: discovered `$CODEX_HOME/agents/*.toml` roles can replace or clear `allowed_actions`/`denied_actions`; built-ins and external `config_file` roles are rejected; persisted snapshots for existing agents are not mutated.
- 2026-06-19: Closed pathless non-root MAv2 ownership gap: `spawn_agent`, `send_message`, `followup_task`, `interrupt_agent` and `set_thread_note` no longer fall back to root ownership when `SubAgentSource::ThreadSpawn.agent_path` is absent on an old/pathless sub-agent source.
