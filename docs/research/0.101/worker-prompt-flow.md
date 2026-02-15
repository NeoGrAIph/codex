# Worker Prompt Flow: upstream `0.101.0` vs fork

## Release baseline
- Upstream release tag: `rust-v0.101.0`
- Upstream release commit: `cf5f1868bc3df74739fc8e06e5c2e93728c5d8f9`

## Цель
Зафиксировать сквозной путь формирования `base_instructions` для `spawn_agent` с ролью `worker`, включая:
- entrypoints;
- порядок применения конфигурации и приоритеты;
- конкретные точки присваивания/перезаписи `base_instructions`.

## Общий скелет пайплайна (и upstream, и fork)
1. `spawn_agent` строит `Config` дочернего треда.
2. `AgentControl::spawn_agent` передаёт этот `Config` в `ThreadManager`.
3. `ThreadManager::spawn_new_thread_with_source` создаёт новый тред с `InitialHistory::New`.
4. `Codex::spawn` финализирует `session_configuration.base_instructions` по приоритетам:
   1) `config.base_instructions`; 2) `session_meta.base_instructions` из истории; 3) model default.

Ключевые downstream точки:
- `codex-rs/core/src/agent/control.rs:43` (`AgentControl::spawn_agent`)
- `codex-rs/core/src/thread_manager.rs:428` (`spawn_new_thread_with_source`, `InitialHistory::New` на `codex-rs/core/src/thread_manager.rs:436`)
- `codex-rs/core/src/codex.rs:331` (`Codex::spawn` приоритеты)

---

## Upstream `0.101.0`: формирование `base_instructions` для `worker`

### 1) Вход и аргументы
- `spawn_agent` принимает `agent_type: Option<AgentRole>`.
- Точка входа: `rust-v0.101.0:codex-rs/core/src/tools/handlers/collab.rs:91` (`spawn::handle`), аргументы на `:99`.
- Схема инструмента: только `message|items|agent_type` (`rust-v0.101.0:codex-rs/core/src/tools/spec.rs:514`).

### 2) Первичное присваивание `base_instructions`
- В `build_agent_spawn_config` делается жёсткое наследование от родительской сессии:
  - `config.base_instructions = Some(base_instructions.text.clone())`
  - `rust-v0.101.0:codex-rs/core/src/tools/handlers/collab.rs:789`
- Источник значения: `session.get_base_instructions()` в `spawn::handle` (`:139`).

### 3) Попытка role-level override
- После наследования вызывается `agent_role.apply_to_config(&mut config)` (`:144`).
- Для `AgentRole::Worker` в upstream нет `base_instructions` override (строка закомментирована), поэтому перезаписи не происходит:
  - `rust-v0.101.0:codex-rs/core/src/agent/role.rs:80`
  - `rust-v0.101.0:codex-rs/core/src/agent/role.rs:112` (перезапись была бы только при `Some(...)`).

### 4) Финальный приоритет в `Codex::spawn`
- Новый тред стартует с `InitialHistory::New`, поэтому history fallback для него пустой.
- В `Codex::spawn` побеждает уже выставленный `config.base_instructions`:
  - `rust-v0.101.0:codex-rs/core/src/codex.rs:331`.
- Итог: у `worker` в upstream effective `base_instructions` = унаследованный prompt родителя.

### 5) Откуда берётся родительский prompt (контекст upstream)
- Перед этим `Config.base_instructions` может прийти из overrides/CLI.
- Иначе подхватывается `model_instructions_file` из profile/global config:
  - `rust-v0.101.0:codex-rs/core/src/config/mod.rs:1625`.
- Если `Config.base_instructions` пуст, уже в `Codex::spawn` срабатывают fallback'и history/model (`:331`).

---

## Текущий fork: формирование `base_instructions` для `worker`

### 1) Вход и расширенные аргументы
- Точка входа перенесена в `codex-rs/core/src/tools/handlers/collab/spawn.rs:28`.
- Аргументы: `agent_type`, `agent_name`, `model`, `reasoning_effort` (`:13`).
- Схема инструмента расширена (`codex-rs/core/src/tools/spec.rs:533`).

### 2) Первичное наследование (как в upstream)
- Базовый шаг не изменён: `build_agent_spawn_config` сначала копирует prompt родителя:
  - `codex-rs/core/src/tools/handlers/collab.rs:840`
  - присваивание на `codex-rs/core/src/tools/handlers/collab.rs:846`.

### 3) Built-in role слой
- Применяется `role.apply_to_config(&mut config)` (`codex-rs/core/src/tools/handlers/collab/spawn.rs:108`).
- Для `worker` это по-прежнему не даёт prompt override (`codex-rs/core/src/agent/role.rs:80`).

### 4) Новый template-слой (fork-расхождение)
- Для `agent_type != default` вызывается `role_templates::get_parsed(...)` (`codex-rs/core/src/tools/handlers/collab/spawn.rs:118`).
- Loader/parser шаблонов:
  - `codex-rs/core/src/agent/role_templates.rs:267`
  - встроенный bundle `templates/agents/*.md` на `:11`.
- Для `worker` используется `codex-rs/core/templates/agents/worker.md:1`.

### 5) Логика выбора prompt из шаблона
- Если задан `agent_name` → берётся соответствующий named-блок (`spawn.rs:131`).
- Если `agent_name` не задан и default не пустой → берётся default-блок (`spawn.rs:143`).
- Если default пустой и named-блок один → берётся он (`spawn.rs:146`).
- Иначе ошибка (`spawn.rs:155`).
- Важный нюанс: default + named не склеиваются; выбирается ровно один блок.

### 6) Перезапись `base_instructions` (ключевая точка fork)
- После выбора шаблонного текста выполняется:
  - `config.base_instructions = Some(selected_instructions)`
  - `codex-rs/core/src/tools/handlers/collab/spawn.rs:161`.
- Это **перезаписывает** ранее унаследованное значение из `build_agent_spawn_config`.

### 7) Дополнительные приоритеты и edge-cases
- При наличии template meta/model defaults они применяются только если нет явного override в вызове (`spawn.rs:173`, `:180`), затем явные `model/reasoning_effort` имеют высший приоритет (`spawn.rs:190`, `:193`).
- Для built-in роли без `agent_name`, если template отсутствует, включён fallback к upstream-поведению (без ошибки, prompt остаётся унаследованным): `spawn.rs:121`.
- Если YAML/frontmatter шаблона не парсится, fork специально деградирует в «сырое markdown как инструкции» (`role_templates.rs:250`), то есть prompt всё равно перезапишется.

### 8) Финализация в `Codex::spawn`
- Как и в upstream, финальный выбор делается в `codex-rs/core/src/codex.rs:331`.
- Для `worker` в текущем fork обычно уже стоит template-значение в `config.base_instructions`, поэтому оно и становится session-level `base_instructions`.

---

## Ключевые расхождения upstream vs fork
1. **Источник worker prompt**
   - Upstream: наследование от родителя (role `worker` runtime prompt не подменяет).
   - Fork: template-driven prompt из `templates/agents/worker.md`.
2. **Точки перезаписи**
   - Upstream: только наследование в `build_agent_spawn_config`, дальше для worker overwrite нет.
   - Fork: дополнительная обязательная перезапись на `spawn.rs:161` (при валидном `worker` template).
3. **Поведение по `agent_name`**
   - Upstream: отсутствует.
   - Fork: выбор между default и named prompt, плюс ошибки в неоднозначных случаях.
4. **Поведение при проблемах шаблона**
   - Upstream: template-слоя нет.
   - Fork: parse-failure fallback в raw markdown вместо отказа.
5. **Контракт spawn metadata**
   - Upstream: `ThreadSpawn { parent_thread_id, depth }`.
   - Fork: добавлены `agent_type/agent_name/allow_list/deny_list` (`codex-rs/protocol/src/protocol.rs:1786`).

## Ссылки на ключевые функции/файлы
- Upstream `0.101.0`:
  - `rust-v0.101.0:codex-rs/core/src/tools/handlers/collab.rs` (`spawn::handle`, `build_agent_spawn_config`, `thread_spawn_source`)
  - `rust-v0.101.0:codex-rs/core/src/agent/role.rs` (`AgentRole::profile`, `AgentRole::apply_to_config`)
  - `rust-v0.101.0:codex-rs/core/src/codex.rs` (`Codex::spawn`)
  - `rust-v0.101.0:codex-rs/core/src/config/mod.rs` (merge base instructions / `model_instructions_file`)
  - `rust-v0.101.0:codex-rs/core/src/tools/spec.rs` (`create_spawn_agent_tool`)
- Fork (текущее состояние ветки):
  - `codex-rs/core/src/tools/handlers/collab/spawn.rs`
  - `codex-rs/core/src/agent/role_templates.rs`
  - `codex-rs/core/templates/agents/worker.md`
  - `codex-rs/core/src/tools/handlers/collab.rs`
  - `codex-rs/core/src/codex.rs`
