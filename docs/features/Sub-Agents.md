# Sub-Agents: spawn и наблюдение sub-agent тредов

Этот документ описывает весь добавленный и незакомментированный функционал, связанный с sub-agent тредами в форке (baseline: `0.99`).

Детальные спецификации отдельных частей:
- окно `/ A G E N T S /` и цикл `Ctrl+T`: `docs/features/saw.md`
- template-backed роли/личности и дефолты спавна: `docs/features/agent_templates.md`

## 1) Passport

- Статус: `implemented`
- Scope in:
  - `codex-rs/core` (spawn_agent schema/handler, template parsing, лимиты)
  - `codex-rs/protocol` (metadata источника spawned threads)
  - `codex-rs/tui` (отображение дерева и статусов)
- Scope out:
  - отдельный публичный API для реестра ролей/личностей (вне `spawn_agent` schema)
  - отдельный tool для “list agents” (используется нативный tool hint в schema)
- Security/policy impact:
  - добавлено ограничение `close_agent` для sub-agent: закрывать можно только в пределах собственного subtree;
  - добавлено per-`agent_type` ограничение tools через `allow_list`/`deny_list`.
- Базовый пакет исследований релиза: `docs/research/0.99/README.md`

## 2) Цели

1. Сделать `spawn_agent` “самоописываемым” через schema hints:
   - роли (agent_type) и их назначение,
   - личности (agent_name) и их назначение,
   - дефолтные параметры модели/размышления.
2. Сохранить минимальный diff относительно upstream:
   - не добавлять отдельные реестры/конфиги, если можно использовать шаблоны в `templates/agents`.
3. Дать оператору быстрый обзор sub-agent активности внутри TUI:
   - без смены контекста и без разрушения inline-screen терминала.

## 3) User Contract

### 3.1) Spawning sub-agents (`spawn_agent`)

`spawn_agent` принимает (в дополнение к существующим полям) следующие опциональные параметры:
- `agent_type`: строка роли (built-in или template-backed из `templates/agents/<agent_type>.md`).
- `agent_name`: личность (вариант) внутри `agent_type` (если шаблон определяет варианты).
- `model`: override модели для spawned agent.
- `reasoning_effort`: override уровня размышления для spawned agent.

Приоритеты выбора параметров spawned agent:
1. explicit overrides из `spawn_agent` (`model`, `reasoning_effort`)
2. дефолты из YAML-frontmatter `templates/agents/<agent_type>.md`
3. текущая нативная логика наследования из turn/config (как было раньше)

Tool policy per `agent_type`:
- template frontmatter может задавать `allow_list` и/или `deny_list` (списки имён tools).
- policy применяется только к spawned agent этого `agent_type`:
  - сначала allow-list (если задан),
  - затем deny-list.
- policy переносится в `SessionSource::SubAgent::ThreadSpawn` metadata и применяется при сборке `ToolRouter`.

Выбор base instructions:
- если передан `agent_name`, должен существовать одноимённый block в шаблоне;
- если `agent_name` не передан:
  - используется дефолтный block (текст до первого marker), если он непустой;
  - иначе, если в шаблоне ровно один `agent_name` block, используется он;
  - иначе `spawn_agent` возвращает ошибку “requires agent_name selection”.

### 3.2) Template-backed роли и личности (`templates/agents/*.md`)

Источник truth для roles/variants: `codex-rs/core/templates/agents/*.md` (compile-time templates).

Формат:
- опциональный YAML frontmatter между `--- ... ---` с полями:
  - `description` (назначение роли),
  - `model`, `reasoning_effort` (дефолты),
  - `agent_names[]` (список личностей с описаниями).
- опциональные blocks в body, размеченные маркерами:
  - `<!-- agent_name: <name> -->`

Валидация (для защиты от рассинхронизации):
- если YAML содержит `agent_names`, то blocks в body должны быть 1:1 с ними (без лишних/пропущенных).

### 3.3) Лимиты и guard-rails

- Максимальная глубина каскадного спавна тредов: `MAX_THREAD_SPAWN_DEPTH = 2`.
  - при превышении `spawn_agent` отклоняется сообщением “Agent depth limit reached. Solve the task yourself.”
- Максимальное число активных sub-agent тредов по умолчанию: `agents.max_threads = 12` (если не задано в конфиге).
- `close_agent` выполняет каскадное завершение: при закрытии родителя последовательно закрываются все его потомки (`ThreadSpawn`) и только затем сам родитель.
- Каскад встроен в `AgentControl::shutdown_agent`, поэтому одинаково работает для любых call-sites завершения (не только `close_agent`).
- Для вызовов `close_agent` из sub-agent действует subtree guard:
  - можно закрыть себя,
  - можно закрыть descendants,
  - нельзя закрыть sibling/другие ветки.

### 3.4) Наблюдение sub-agents в TUI (`Ctrl+T` / SAW)

В TUI добавлен цикл `Ctrl+T`:
1. `None -> / T R A N S C R I P T /`
2. `/ T R A N S C R I P T / -> / A G E N T S /` (summary sub-agents)
3. `/ A G E N T S / -> None`

Ключевые свойства контракта:
- корректный возврат терминала в inline-screen при закрытии (без “перемотки”);
- дерево sub-agent тредов по `parent_thread_id` с отступами depth;
- вывод status/working-индикатора, `Context left`, и “Last tool” (и опционально detail).
- при завершении turn у неактивного spawned sub-agent отправляется desktop notification
  (через текущие `notifications` настройки для `agent-turn-complete`).

## 4) Implementation Map

### 4.1) Core (templates + spawn)

- `codex-rs/core/src/agent/role_templates.rs`
  - парсинг `templates/agents/*.md` (frontmatter + blocks)
  - `list_summaries()` (для подсказок в schema `spawn_agent`)
  - `get_parsed()` (выбор инструкций/дефолтов при спавне)
- `codex-rs/core/templates/agents/*.md`
  - source-of-truth для ролей, личностей и дефолтов
- `codex-rs/core/src/tools/spec.rs`
  - расширение schema `spawn_agent` (`agent_name`, `model`, `reasoning_effort`)
  - подсказка по доступным `agent_type`/`agent_name` и их `description`
  - применение per-agent policy фильтрации tools (`allow_list`/`deny_list`)
- `codex-rs/core/src/codex.rs`
  - применение policy из `SessionSource::SubAgent::ThreadSpawn` при сборке `ToolsConfig`
- `codex-rs/core/src/tools/handlers/collab.rs`
  - роутинг tool calls, guards по глубине/лимитам, subtree guard для `close_agent`
- `codex-rs/core/src/tools/handlers/collab/spawn.rs`
  - реализация `spawn_agent` (выбор инструкций + применение приоритетов)
  - перенос `allow_list`/`deny_list` в source spawned thread
- `codex-rs/core/src/agent/control.rs`
  - поиск descendants по `ThreadSpawn` source
  - каскадный `shutdown_agent`
  - `is_descendant_of` для subtree guard
- `codex-rs/core/src/thread_manager.rs`
  - `list_threads()` для обхода дерева thread-spawn

Fork markers:
- `[SA] COMMIT OPEN/CLOSE` — template-backed логика spawn_agent
- `[SA] COMMIT OPEN/CLOSE` — tool policy / cascade terminate / subtree guard

### 4.2) Protocol (metadata spawned thread)

- `codex-rs/protocol/src/protocol.rs`
  - `SubAgentSource::ThreadSpawn` расширен: `agent_type: Option<String>`, `agent_name: Option<String>` (backward compatible через `#[serde(default)]`)

Fork markers:
- `SAW COMMIT OPEN/CLOSE` — сохранение `agent_type/agent_name` в source для отображения в TUI

### 4.3) TUI (SAW)

- `codex-rs/tui/src/app.rs`
  - state-machine `Ctrl+T` (Transcript -> Agents -> Close)
  - форматирование роли для SAW: `agent_type/agent_name` (если оба известны)
  - app-loop notify для completion неактивных spawned sub-agent тредов
- `codex-rs/tui/src/app_event.rs`
  - внутренний `AppEvent` для передачи completion-notify из фоновых listener-ов в app-loop
- `codex-rs/tui/src/agents_overlay.rs`
  - построение дерева sub-agent тредов и строк summary

### 4.4) Config/guards (limits)

- `codex-rs/core/src/agent/guards.rs`
  - `MAX_THREAD_SPAWN_DEPTH = 2`
- `codex-rs/core/src/config/mod.rs`
  - дефолт `DEFAULT_AGENT_MAX_THREADS = Some(12)`

## 5) Verification Matrix

Минимум (core/protocol):
```bash
cd codex-rs
just fmt
just fix -p codex-core
cargo test -p codex-protocol
cargo test -p codex-core --lib
```

TUI (SAW):
```bash
cd codex-rs
cargo test -p codex-tui ctrl_t_overlay_action_cycles_transcript_and_agents_overlays
```

## 6) Doc Changelog

- `2026-02-12`: добавлен документ `Sub-Agents.md` как “umbrella” для sub-agent функционала (SAW + SA + limits).
- `2026-02-12`: зафиксированы изменения: per-agent `allow_list`/`deny_list`, каскад завершения в `AgentControl::shutdown_agent`, subtree guard для `close_agent`.
- `2026-02-12`: добавлены notify-события completion для неактивных spawned sub-agent тредов (с уважением `notifications` policy).
