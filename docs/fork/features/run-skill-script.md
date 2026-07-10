# run_skill_script tool

## Feature passport

- Code name: `run-skill-script`
- Status: экспериментальная fork-возможность из `fork/multi-agent`.
- Goal: дать agents инструмент запуска skill scripts через unified exec path.
- Scope in: tool spec/handler, templates allowlist, unified exec integration.
- Scope out: общая система skills и plugin installation.

## Как работает для пользователя

Agent может вызвать специализированный tool для запуска script, поставляемого skill, через unified exec. Польза: повышается эффективность и качество работы agent, так как skill может не только описывать действия, но и выполнять подготовленный helper.

## Branches and commits

| Branch | Baseline | Commit | Date | Subject | Роль |
| --- | --- | --- | --- | --- | --- |
| `fork/multi-agent` | `fork/colab-agents` lineage | `521c2a9978` | 2026-02-09 | `core: add run_skill_script tool; bump wait default` | Добавлен handler/spec и template wiring для `run_skill_script`. |
| `fork/multi-agent` | `fork/colab-agents` lineage | `ab35fe2fc1` | 2026-02-09 | `core: expose run_skill_script under unified_exec` | Tool выведен через unified exec. |

## Implementation notes

Diff затрагивал tool handler/spec, app-server schema artifacts, core templates and tests. В subject одного commit также есть wait tuning; это учтено отдельно в `agent-runtime-limits`.

## Native coverage in rust-v0.140.0

Status: `partial`. Native release имеет skill metadata/resource surfaces, `skills.list`, `skills.read`, `SkillMetadata`, `SkillDependencies`, implicit skill invocation detection and ordinary `exec_command`/shell execution. Не хватает отдельного model-visible `run_skill_script` tool, handler/spec/template allowlist and permission binding by skill identity; scripts сейчас запускаются как обычные команды через стандартный exec/shell path.

## Porting/current-state notes

Перед переносом нужно проверить current upstream skills/plugin architecture. Если есть native tool/resource execution surface для skills, fork должен использовать его.
