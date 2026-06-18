# Subagent policy and ownership

## Feature passport

- Code name: `subagent-policy-and-ownership`
- Status: переносимая security/ownership возможность.
- Goal: не дать sub-agent обходить allow/deny policies, trust boundaries и ownership своего subtree.
- Scope in: current `fork/140` MAv2 interrupt ownership guard, workbench/app-server close-one ownership guard, canonical `AgentPath` subtree checks, root/non-root/self/cross-subtree diagnostics and focused ownership tests. Historical/future scope still includes durable policy metadata propagation, MCP allowlist/denylist, trust gating and read-only `.agents`, but those surfaces are not claimed by the first `fork/140` slices.
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

Status: `partial`. Native release имеет protected metadata paths (`.git`, `.agents`, `.codex`), workspace-write protected subpaths, MCP `ToolFilter` with `enabled_tools`/`disabled_tools`, requirements/identity filtering for MCP servers, root-tree scoped `AgentControl`, subagent metadata and close handler. Текущий `fork/140` добавляет path-based ownership для MAv2 `interrupt_agent` и для workbench/app-server `close one` через `ThreadManager::close_agent_from_workbench`. Не хватает durable policy metadata propagation, role-template `read_only`/`allow_list`/`deny_list` as subagent contract, V1 legacy close tool parity and app-server protocol/schema fields for policy metadata state.

## Porting/current-state notes

Это security-sensitive функционал. При переносе нужно обязательно проверять both positive and negative cases: разрешенный sub-agent action работает, запрещенный action fail-fast с диагностикой.

## Fork/140 implementation status

Первые `fork/140` slices не вводят новую policy metadata model и не меняют MCP allow/deny enforcement. Они добавляют native ownership guard для MAv2 `interrupt_agent` и workbench/app-server `close one`: root может управлять non-root agents, а sub-agent может interrupt/close только targets внутри своего canonical `agent_path` subtree; descendant target разрешён, а sibling/root/self/pathless/root обходы fail-fast. Durable role-template policy metadata, app-server schema fields for policy metadata и V1 close-agent legacy parity остаются gap для следующего этапа.

## Doc changelog

- 2026-06-17: Зафиксирован fork/140 first iteration: MAv2 cross-subtree interrupt denial через `AgentControl` metadata/`SessionSource`, без parallel ownership store.
- 2026-06-18: Focused MAv2 ownership verification passed for cross-subtree, root target and self-target interrupt denial.
- 2026-06-18: Добавлен positive descendant ownership test: non-root agent can interrupt a descendant inside its own `AgentPath` subtree while sibling/root/self targets remain denied.
- 2026-06-18: Workbench/app-server `close one` добавлен в ownership scope: `ThreadManager::close_agent_from_workbench` enforces author/target `AgentPath` descendant ownership before delegating to native `AgentControl::close_agent`.
