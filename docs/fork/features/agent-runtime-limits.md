# Agent runtime limits

## Feature passport

- Code name: `agent-runtime-limits`
- Status: первая итерация перенесена на `fork/140` и локально проверена.
- Goal: сделать multi-agent workflow практичнее за счет больших лимитов глубины spawn, количества threads и ожидания результатов.
- Scope in: `agents.max_threads`, spawn depth, `wait_agent` default/max timeout.
- Scope out: security enforcement и cwd.

## Как работает для пользователя

Пользователь может вести более крупные multi-agent задачи: запускать больше agents, давать им больше времени на ответ и строить более глубокие деревья подзадач. Польза: меньше преждевременных ограничений в сложных multi-agent workflow.

## Branches and commits

| Branch | Baseline | Commit | Date | Subject | Роль |
| --- | --- | --- | --- | --- | --- |
| `fork/colab-agents`, `fork/multi-agent` | `rust-v0.98.0` audit lineage | `200d9937e1` | 2026-02-05 | `feat(core): allow two-level agent spawn` | Разрешен второй уровень spawn вместо strict leaf-only agents. |
| `fork/multi-agent` | `fork/colab-agents` lineage | `595b3ccfb4` | 2026-02-07 | `core: raise default agents.max_threads to 12` | Увеличен default max threads. |
| `fork/multi-agent` | `fork/colab-agents` lineage | `521c2a9978` | 2026-02-09 | `core: add run_skill_script tool; bump wait default` | Увеличен wait default вместе с tool addition. |
| `fork/106` | `rust-v0.106.0` | `180489b76c` | 2026-03-01 | `feat(wait): increase default and max timeout windows` | Увеличены wait timeout windows. |
| `fork/106` | `rust-v0.106.0` | `3d23e9ffaf` | 2026-03-01 | `feat(sa): enforce subtree close ownership and bump spawn depth default` | Увеличен spawn depth default. |
| `fork/107` | `rust-v0.107.0` | `593b997715` | 2026-03-01 | `feat(wait): increase default and max timeout windows` | Перенос wait changes. |
| `fork/107` | `rust-v0.107.0` | `d4bd1f85f4` | 2026-03-01 | `feat(sa): enforce subtree close ownership and bump spawn depth default` | Перенос depth changes. |
| `fork/114` | `rust-v0.114.0` | `8fc6e5029d` | 2026-03-13 | `Увеличены default wait timeout и лимит sub-agent threads` | Русскоязычный перенос tuning на 0.114. |
| `fork/116` | `rust-v0.116.0` | `c71d2aedac` | 2026-03-13 | `Увеличены default wait timeout и лимит sub-agent threads` | Перенос tuning в 0.116 lineage. |
| `fork/130` | `rust-v0.130.0` | `837d978bd6` | 2026-05-14 | `Adjust wait_agent default timeout` | Настройка default timeout. |
| `fork/130` | `rust-v0.130.0` | `45b0b43f4a` | 2026-05-14 | `feat(agents): raise default spawn depth` | Увеличение default spawn depth. |
| `chrome_plugin` | `rust-v0.130.0` lineage | `837d978bd6` | 2026-05-14 | `Adjust wait_agent default timeout` | Та же настройка в chrome-plugin lineage. |
| `chrome_plugin` | `rust-v0.130.0` lineage | `45b0b43f4a` | 2026-05-14 | `feat(agents): raise default spawn depth` | Та же настройка в chrome-plugin lineage. |

## Implementation notes

Затрагивались core config/defaults, collab/multi-agent handlers, tests and templates. Некоторые коммиты совмещают tuning с security ownership changes; в этом doc учитывается только часть про лимиты.

## Native coverage in rust-v0.140.0

Status: native upstream coverage was `partial` before the fork overlay. Native release имел defaults/config for `DEFAULT_AGENT_MAX_THREADS = Some(6)`, `DEFAULT_AGENT_MAX_DEPTH = 1`, `DEFAULT_MULTI_AGENT_V2_MAX_CONCURRENT_THREADS_PER_SESSION = 4`, wait min/default/max `10_000/30_000/3_600_000`, `MultiAgentV2Config`, v2 wait options and tests. Fork/140 intentionally overlays exact tuning through native defaults: legacy `agents.max_threads = 12`, v2 default concurrency is root plus 12 spawned agents, and default spawn depth is 2.

## Porting/current-state notes

Новые лимиты должны быть согласованы с текущей моделью sub-agent lifecycle и системным лимитом активных agents. Если upstream уже изменил лимиты, fork должен документировать только intentional divergence.

## Fork/140 implementation status

Первая итерация задаёт fork defaults в native config constants: `DEFAULT_AGENT_MAX_THREADS = Some(12)`, `DEFAULT_MULTI_AGENT_V2_MAX_CONCURRENT_THREADS_PER_SESSION = 13` (root плюс 12 spawned agents), `DEFAULT_AGENT_MAX_DEPTH = 2`, `DEFAULT_MULTI_AGENT_V2_DEFAULT_WAIT_TIMEOUT_MS = 300_000`. Max wait window остаётся upstream hard max `3_600_000`, config overrides продолжают работать через существующий `MultiAgentV2Config`, а `wait_agent` получает эти значения через native `spec_plan` projection в model-visible schema.

## Doc changelog

- 2026-06-17: Зафиксированы fork/140 runtime limits: 12 spawned agents, MAv2 concurrency 13 including root, spawn depth 2, default wait 300s.
- 2026-06-18: Исправлен stale config test expectation для root-inclusive MAv2 cap; `multi_agent_v2_default_session_thread_cap_counts_root` and `multi_agent_v2_runtime_defaults_use_fork_limits` подтверждают 12 spawned agents, depth 2 and default wait 300s.
- 2026-06-18: Добавлен runtime limiter regression `spawn_agent_v2_default_limit_allows_twelve_spawned_agents`: default MAv2 cap accepts 12 spawned agents and rejects the 13th with `AgentLimitReached { max_threads: 12 }`.
- 2026-06-18: Уточнён статус feature doc: upstream native coverage remains partial relative to fork needs, but the `fork/140` first iteration is implemented through native config defaults and covered by focused runtime tests.
- 2026-06-18: Добавлен schema projection regression `multi_agent_v2_wait_agent_schema_uses_configured_fork_timeout_defaults`: model-visible `wait_agent.timeout_ms` description now proves the native `spec_plan` path advertises default wait `300000` ms.
