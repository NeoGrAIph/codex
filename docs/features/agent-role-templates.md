# Agent Role Templates

## Feature passport

- Code name: `Agent Role Templates`.
- Status: `implemented`.
- Goal: добавить template augmentation поверх upstream role pipeline `0.111.0`, не превращая templates в отдельный registry ролей.
- Scope in:
- `codex-rs/core/src/agent/role_templates.rs`
- `codex-rs/core/templates/agent_roles/*.md`
- `codex-rs/core/src/tools/handlers/multi_agents.rs`
- `codex-rs/core/src/tools/spec.rs`
- Scope out:
- enforcement `allow_list` / `deny_list`;
- SAW и TUI;
- новые app-server RPC или TUI-specific projections.

## Problem statement

В `rust-v0.111.0` уже есть ingress selector роли (`agent_type`) и canonical runtime metadata роли (`agent_role`), но нет fork-level слоя, который:

- задаёт persona-specific system prompt;
- задаёт role/persona defaults для `model` и `reasoning_effort`;
- формализует role capability metadata (`allow_list` / `deny_list`);
- разрешает эти значения одинаково на spawn/resume/app-server путях.

Без отдельного template layer fork-specific persona/policy логика расползается по runtime коду и теряет воспроизводимость.

## Contract

Role templates загружаются только как augmentation layer поверх уже существующей роли.

Источник роли:

- built-in role registry;
- `config.agent_roles`.

Template сам по себе новую роль не создаёт.

### Template lookup

Role templates ищутся в таком порядке:

1. ближайший `./.codex/.agents/*.md` при подъёме от `cwd` к repo root;
2. `~/.codex/.agents/*.md`;
3. embedded fallback templates.

Embedded fallback templates существуют только для ролей:

- `default`
- `explorer`
- `worker`

Поиск role stem:

- case-insensitive;
- `-` и `_` эквивалентны;
- неоднозначное совпадение приводит к fail-fast ошибке.

### Template schema

Template использует YAML frontmatter и prompt blocks.

Role-level metadata:

- `description`
- `read_only`
- `model`
- `reasoning_effort`
- `allow_list`
- `deny_list`

Persona-level metadata:

- `name`
- `description`
- `model`
- `reasoning_effort`

Prompt blocks объявляются как:

```md
<!-- agent_nickname: default -->
... prompt ...
```

`agent_names` обязателен и должен включать `default`.

### Validation rules

Template считается невалидным, если:

- отсутствует YAML frontmatter;
- отсутствует `agent_names`;
- отсутствует persona `default`;
- у persona нет непустого `description`;
- у persona нет prompt block;
- в файле есть prompt text вне marker blocks;
- во frontmatter есть неизвестные поля.

## Runtime resolution

`spawn_agent` остаётся upstream-first:

1. роль выбирается через `agent_type`;
2. применяется upstream `apply_role_to_config`;
3. загружается template для той же роли;
4. persona выбирается через `agent_persona`, иначе `default`;
5. template augmentation применяется только к domain-specific settings.

Template augmentation в этой стадии влияет только на:

- `developer_instructions`
- `model`
- `reasoning_effort`
- `read_only`
- `allow_list`
- `deny_list`

Template augmentation не имеет права в этой feature менять:

- approval;
- cwd;
- другие runtime-owned overrides.

## Resolution order

Порядок разрешения значений:

- `model`: explicit spawn arg -> persona template -> role template -> inherited runtime model -> catalog default;
- `reasoning_effort`: explicit spawn arg -> persona template -> role template -> inherited runtime reasoning.

Persona prompt добавляется как финальный child-specific layer:

- если `developer_instructions` уже существуют, prompt дописывается через разделитель;
- если их нет, prompt становится новым значением.

`read_only` применяется как tighten-only post-runtime override:

- сначала child получает обычные runtime-owned overrides текущего turn;
- затем template может ужесточить sandbox до `read_only`;
- template не имеет права ослабить уже более строгие sandbox constraints.

## Policy metadata

`allow_list` / `deny_list` в этой feature:

- парсятся;
- нормализуются (`trim`, drop empty, sort, dedupe);
- прикрепляются к `ThreadSpawn`.

Синтаксис фиксируется уже сейчас:

- case-insensitive match;
- `*` — любое число символов;
- `?` — один символ;
- строка без wildcard — exact name.

Enforcement policy intentionally не входит в эту feature и документируется отдельным follow-up commit.

## Discovery metadata

`spawn_agent` discovery text показывает template metadata без prompt leakage:

- role `description`;
- `read_only`;
- список `agent_persona` с `name` и `description`.

Prompt text не включается в schema/discovery output и разрешается только во время spawn-time template resolution.

## Compatibility guarantees

- Роль остаётся canonical через `agent_role`; legacy alias `agent_type` продолжает приниматься там, где это уже было частью контракта.
- Явный `agent_persona` без template для выбранной роли приводит к ошибке spawn-time resolution.
- Отсутствие template не меняет поведение роли относительно upstream `0.111.0`.

## Validation matrix

- parser tests для template schema и prompt blocks;
- resolution tests для precedence project -> user -> embedded;
- spawn tests для `default` persona, explicit persona и template-free role;
- runtime tests, что template augmentation не переписывает runtime-owned overrides;
- runtime tests, что `read_only` применяется после runtime overrides и только ужесточает sandbox;
- spawn-time validation tests для template `model` / `reasoning_effort`;
- protocol/state/app-server tests, что `allow_list` / `deny_list` остаются metadata-only на этой стадии.
