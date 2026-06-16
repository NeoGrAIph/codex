# Agent role templates

## Feature passport

- Code name: `agent-role-templates`
- Status: переносимая fork-возможность.
- Goal: дать sub-agents осмысленные роли и инструкции через markdown templates/persona metadata.
- Scope in: built-in role templates, manifest/authoring docs, persona/thread metadata, model instructions in markdown roles.
- Scope out: runtime limits и cwd; они описаны отдельно.

## Как работает для пользователя

Пользователь или orchestrator запускает sub-agent с понятной специализацией: explorer, worker, reviewer и т.п. Польза: agents стартуют с нужной ролью и не требуют каждый раз ручного описания роли.

## Branches and commits

| Branch | Baseline | Commit | Date | Subject | Роль |
| --- | --- | --- | --- | --- | --- |
| `fork/saw` | `rust-v0.99.0` | `a15d4adc2c` | 2026-02-13 | `[SA][SAW] finalize sub-agent templates, guards, and UI integration` | Первичная связка templates с sub-agent workflow. |
| `fork/101` | `rust-v0.101.0` | `9f82deac0f` | 2026-02-15 | `feat(sa): complete Sub-Agents fork contract (templates, thread_note, runtime listing, TUI/SAW polish)` | Полный fork contract templates/runtime listing. |
| `fork/106` | `rust-v0.106.0` | `1e5e63ff26` | 2026-02-28 | `docs: add agent manifest template and authoring instructions` | Authoring guidance для templates. |
| `fork/106` | `rust-v0.106.0` | `c402866871` | 2026-02-28 | `feat: add agent role templates and thread persona metadata` | Runtime/templates/persona metadata. |
| `fork/107` | `rust-v0.107.0` | `4469b595c8` | 2026-02-28 | `feat: add agent role templates and thread persona metadata` | Перенос на 0.107. |
| `fork/111` | `rust-v0.111.0` | `7a977ec76b` | 2026-03-06 | `feat(agents): add role templates and thread persona foundation` | Обновленный foundation для templates/persona. |
| `fork/118` | `rust-v0.118.0` | `a233952c1c` | 2026-04-03 | `Implement agent role templates across core, state, app-server, and TUI` | Сквозная интеграция core/state/app-server/TUI. |
| `fork/118` | `rust-v0.118.0` | `2e6af0d4a2` | 2026-04-04 | `Support model instructions in markdown agent roles` | Markdown role templates получают model instructions. |
| `fork/130` | `rust-v0.130.0` | `783ebf1fac` | 2026-05-14 | `feat(agents): add markdown role templates` | Перенос markdown role templates на 0.130 lineage. |
| `chrome_plugin` | `rust-v0.130.0` lineage | `783ebf1fac` | 2026-05-14 | `feat(agents): add markdown role templates` | Та же возможность в chrome-plugin lineage. |

## Implementation notes

Затрагивались template files, core agent control/registry, protocol/thread metadata, app-server projection, TUI rendering и docs. В поздних переносах это стало cross-surface feature, а не только набором markdown-файлов. В `fork/130` native TOML roles остаются primary, а markdown roles дают defaults для `model`/`reasoning_effort`; full-history fork rejects role/persona/model/reasoning/cwd overrides.

## Native coverage in rust-v0.140.0

Status: `partial`. Native release имеет role system: `apply_role_to_config`, `resolve_role_config`, built-ins `default`, `explorer`, `worker`, TOML configs, `AgentRoleConfig`, `spawn_agent.agent_type`, persisted role/nickname/path metadata and TUI labels. Не хватает fork markdown role templates/persona manifest contract, native markdown role loader, reviewer-style built-ins and markdown model-instruction template behavior.

## Porting/current-state notes

При переносе в новую ветку нужно проверять current upstream skills/agents/model instructions path. Fork templates должны использовать native template/resource loading, а не отдельный обходной loader.
