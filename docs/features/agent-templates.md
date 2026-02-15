# agent_templates

## Feature passport

- Code name: `agent_templates`.
- Status: `implemented`.
- Goal: конфигурировать роли/персоны для `spawn_agent` через шаблоны в репозитории вместо жёстко прошитых инструкций.
- Scope in: загрузка, парсинг и выбор template инструкций; применяемые defaults `model/reasoning_effort`; sandbox override `read_only`; tool policy `allow_list/deny_list`.
- Scope out: изменения SAW overlay и TUI-рендера.
- API impact: добавлен tool `list_agents`; `spawn_agent` поддерживает `agent_name`, `model`, `reasoning_effort`, `thread_note`.
- Security impact: template policy (`allow_list/deny_list`) ограничивает набор инструментов для дочерних тредов; template `read_only: true` принудительно переводит spawned thread в `SandboxPolicy::ReadOnly`; при отсутствии `allow_list` дочерний тред получает полный набор доступных tools.

## User contract

- Шаблон роли загружается из первого найденного источника в порядке приоритета:
- project scope: `.codex/.agents/<agent_type>.md` при обходе от текущего `cwd` вверх до ближайшего `.git` (nearest-first);
- user scope: `~/.codex/.agents/<agent_type>.md`;
- embedded fallback: `codex-rs/core/templates/agents/<agent_type>.md`.
- При старте runtime выполняется seed `~/.codex/.agents`: если в каталоге нет ни одного файла, совпадающего по имени с embedded шаблонами, embedded набор копируется в `~/.codex/.agents` как baseline.
- Для template-backed роли `agent_type` должен соответствовать `snake_case` или `kebab-case` (`[a-z0-9_-]+` после нормализации/trim), иначе возвращается `invalid agent_type`.
- Поддерживается YAML frontmatter:
- `description`, `model`, `reasoning_effort`, `read_only`, `agent_names`, `allow_list`, `deny_list`.
- Поддерживаются named instructions блоки:
- `<!-- agent_name: <name> -->`.
- Выбор инструкций:
- если передан `agent_name` -> берётся соответствующий named block;
- иначе берётся default block, а если он пуст и named block ровно один -> берётся он;
- иначе возвращается ошибка выбора персоны.
- Применение model/reasoning defaults:
- `spawn_agent` overrides имеют высший приоритет;
- затем применяются defaults из выбранного `agent_name` (если в `agent_names` указаны `model`/`reasoning_effort`);
- затем defaults уровня `agent_type` (frontmatter root);
- затем inherited turn config.
- Если в шаблоне задано `read_only: true`, spawned thread принудительно получает `SandboxPolicy::ReadOnly`.
- Если `read_only` не задан, используется унаследованный sandbox policy текущего turn.
- Tool policy по умолчанию для шаблонных ролей:
- политика ограничений применяется только если в шаблоне задан `allow_list` и/или `deny_list`;
- элементы `allow_list` / `deny_list` поддерживают exact match и glob-паттерны (`*`, `?`); regex не поддерживается;
- matching регистрозависимый (case-sensitive), при конфликте `deny_list` имеет приоритет над `allow_list`;
- в поставляемых шаблонах `orchestrator` не задаёт `allow_list` (получает полный toolset, включая `spawn_agent`);
- в поставляемых шаблонах `worker` и `explorer` задают `allow_list` без `spawn_agent`, но с `update_plan`, `list_agents` и `list_active_agents`.
- Контракт `list_agents`:
- по умолчанию (`expanded=false`) ответ содержит только `agents: []` без `count/model/reasoning_effort/prompt`;
- для каждого `agent_type` в выдаче обязательны `agent_type`, `description`, `allow_list`, `deny_list`;
- `agent_names` выводится только если у роли есть персоны; если персон нет — поле не выводится;
- для каждой персоны в `agent_names` обязательны `name` и `description`.
- Опции `list_agents`:
- `agent_type` (опционально): фильтр по конкретной роли;
- `expanded` (опционально, default `false`): включает расширенные поля.
- `agent_type` валидируется как `snake_case | kebab-case`; невалидное значение возвращает `invalid agent_type`.
- если `agent_type` валиден, но шаблон отсутствует во всех источниках, возвращается `missing agent template: <agent_type>`.
- При `expanded=true`:
- на уровне роли добавляются `model`, `reasoning_effort`, `default_prompt`;
- в `agent_names[]` добавляются `model`, `reasoning_effort`, `prompt`.
- Строгие инварианты шаблонов для `list_agents`:
- YAML `description` обязателен для каждого `agent_type`;
- default prompt-блок (текст после YAML и до первого `<!-- agent_name: ... -->`) обязателен;
- при нарушении контракта `list_agents` возвращает ошибку.

## Implementation map

- Parser/cache:
- `codex-rs/core/src/agent/role_templates.rs`
- Handler integration:
- `codex-rs/core/src/tools/handlers/collab/spawn.rs`
- `codex-rs/core/src/tools/handlers/list_agents.rs`
- Tool schema hint:
- `codex-rs/core/src/tools/spec.rs`
- Build dependency tracking:
- `codex-rs/core/build.rs`

## Verification matrix

- `cd codex-rs && cargo test -p codex-core --lib`
- Unit tests in `codex-rs/core/src/agent/role_templates.rs`:
- invalid stem rejection;
- frontmatter parse and named blocks;
- validation mismatch between `agent_names` and blocks;
- strict `list_agents` validation for required `description` and default prompt.
- Tool registration/schema checks in `codex-rs/core/src/tools/spec.rs` for `list_agents`.

## Runbook: Add New Agent Role With Personas

1. Decide role contract and boundaries.
- Define `agent_type` name (file stem in `.codex/.agents/<agent_type>.md` или `core/templates/agents/<agent_type>.md`).
- Use `snake_case` or `kebab-case` for `agent_type` (`[a-z0-9_-]+`) to satisfy runtime validation.
- Define whether role is allowed to orchestrate (`spawn_agent` in `allow_list`) or execution-only.
- Define minimal safe tool policy for the role (`allow_list` + optional `deny_list`).

2. Create template file.
- Path options:
- project-local override: `.codex/.agents/<agent_type>.md`;
- user-level override: `~/.codex/.agents/<agent_type>.md`;
- embedded default: `codex-rs/core/templates/agents/<agent_type>.md`.
- Required YAML fields for this fork contract:
- `description` at role level is mandatory.
- `agent_names` is optional; omit field when role has no personas.
- If `agent_names` is present, each entry must contain `name` and `description`.
- Optional defaults:
- role level: `model`, `reasoning_effort`;
- persona level: `model`, `reasoning_effort`.
- Optional sandbox mode:
- `read_only: true` for inspection/research roles that must not write to disk or access network.
- Tool policy fields:
- `allow_list` and `deny_list` are optional but recommended.
- values support exact names and glob masks (`*`, `?`); regex syntax is not interpreted.
- If `allow_list` is omitted, the spawned thread keeps the full toolset available in runtime config.

3. Add default and named prompt blocks.
- Default prompt block is required and must be placed after YAML and before first persona block marker.
- Persona blocks must use exact markers:
- `<!-- agent_name: <name> -->`.
- For every `agent_names[].name`, a matching persona block must exist.
- Do not add unnamed extra persona blocks not declared in YAML.

4. Use this minimal template skeleton.
```md
---
description: Use `auditor` for compliance checks and risk triage.
model: gpt-5-codex
reasoning_effort: medium
read_only: true
agent_names:
  - name: strict
    description: Conservative validation with explicit risk callouts.
    model: gpt-5-codex
    reasoning_effort: high
  - name: fast
    description: Quick scan with concise findings.
    model: gpt-5.3-codex-spark
    reasoning_effort: medium
allow_list:
  - exec_command
  - write_stdin
  - update_plan
  - list_agents
  - list_active_agents
  - wait
  - send_input
  - close_agent
deny_list:
  - apply_patch
---
Use `auditor` for policy and contract checks.
Return findings first, then assumptions.

<!-- agent_name: strict -->
Prioritize completeness and defensive checks.
Reject ambiguous conclusions.

<!-- agent_name: fast -->
Prioritize speed and high-signal output.
Skip low-risk edge cases unless explicitly requested.
```

5. Validate template invariants exposed by `list_agents`.
- `description` must be non-empty.
- Default prompt block must be non-empty.
- If `agent_names` is present, each persona must have:
- YAML declaration with `name` and `description`;
- matching `<!-- agent_name: ... -->` section.
- If role has no personas, do not emit `agent_names` in output; omit from YAML.

6. Confirm runtime behavior.
- `spawn_agent` selection:
- `agent_type=<file stem>`;
- optional `agent_name=<persona name>`.
- Config precedence:
- explicit `spawn_agent.model` / `spawn_agent.reasoning_effort`;
- persona defaults;
- role defaults;
- inherited turn config.
- Tool policy is applied from template to spawned thread.
- If `read_only: true`, spawned thread sandbox policy is set to `SandboxPolicy::ReadOnly` before model/reasoning override validation.

7. Run checks.
- `cd codex-rs && just fmt`
- `cd codex-rs && cargo test -p codex-core --lib`
- Optional focused checks for template tooling:
- `cargo test -p codex-core role_templates`
- `cargo test -p codex-core tools::spec::tests::list_agents_schema_supports_filter_and_expanded_options`

8. Smoke test via tools.
- Call `list_agents` and verify new role appears with expected `allow_list`/`deny_list`.
- Call `list_agents` with `agent_type` filter and `expanded=true`.
- Call `list_agents` with invalid `agent_type` to verify `invalid agent_type` error path.
- Spawn role without persona and with each persona.
- Verify resolved model/reasoning and tool availability on child thread.

9. Update feature docs.
- Update this file (`docs/features/agent_templates.md`) when:
- role contract changed;
- template invariants changed;
- list/spawn behavior changed.
- Update `docs/features/Sub-Agents.md` when ownership, close/shutdown, or lifecycle behavior changes.

10. Common failure modes.
- `agent_name requires a non-default agent_type`: persona used without explicit non-default role.
- `list_agents` fails on missing description/default prompt: template contract violation.
- `invalid agent_type "...": expected snake_case or kebab-case ...`: invalid role/filter name format.
- `missing agent template: <agent_type>`: requested role file does not exist in project/user/embedded sources.
- Persona declared in YAML but block missing (or inverse): template mismatch.
- Model override rejected in spawn: unknown model slug for active provider.
- Reasoning effort rejected: value unsupported by selected model.

## Doc changelog

- 2026-02-14: Added `list_agents` query options (`agent_type`, `expanded`) and extended response mode for focused role inspection.
- 2026-02-14: Added runbook for creating a new `agent_type` with multiple personas, including invariants, policy wiring, and verification steps.
- 2026-02-14: Added strict `list_agents` tool contract (no count/model/reasoning/prompt in output), optional `agent_names` field, and mandatory template invariants (`description` + default prompt).
- 2026-02-14: Added `list_agents` to worker/explorer allow-lists to make role catalog available to all template roles.
- 2026-02-14: Added `list_active_agents` to worker/explorer allow-lists for runtime visibility of spawned descendants.
- 2026-02-14: Restricted `spawn_agent` to orchestrator-only templates; added `update_plan` to worker/explorer allow-lists.
- 2026-02-14: Added `agent_name`-level template defaults for `model` and `reasoning_effort`.
- 2026-02-14: Added template-backed role/persona contract for `spawn_agent` and documented selection/precedence rules.
- 2026-02-14: Clarified `agent_type` validation/error paths, default full-toolset behavior when `allow_list` is omitted, and added `thread_note` to API impact.
- 2026-02-14: Added glob matching (`*`, `?`) for tool policy `allow_list`/`deny_list` with case-sensitive semantics and deny precedence.
- 2026-02-14: Added template `read_only` frontmatter flag mapped to `SandboxPolicy::ReadOnly` at spawn-time.
- 2026-02-14: Clarified `read_only` default behavior (inherit parent sandbox when omitted) and runbook guidance for inspection-only roles.
- 2026-02-14: Added runtime template discovery from `.codex/.agents` (project), `~/.codex/.agents` (user), embedded fallback, plus seed-on-empty behavior and snake/kebab `agent_type` resolution.
