# fork/colab-agents

> Owner: <team/owner> | Scope: fork/colab-agents | Audience: devs
> Status: active | Last reviewed: 2026-02-05 | Related: codex-rs/AGENTS.md

## Контекст

Этот документ фиксирует мотивацию и ключевые решения, принятые в ветке `fork/colab-agents`,
а также перечисляет форк-изменения относительно upstream.

## Мотивация

- Уйти от жёстко прошитого `AgentRole` и дать возможность описывать агентов декларативно
  (repo/user/system), чтобы поддерживать командные и проектные вариации без перекомпиляции.
- Согласовать инструменты multi-agent режима (collab) с реальными агентскими профилями,
  обеспечив проверяемую конфигурацию и более понятный UX для `spawn_agent`.
- Сделать поведение forking/экспериментальных функций управляемым через feature-флаги.
- Отделить fork-бинарь от upstream и упростить локальные сборки/тесты.

## Принятые решения

1) **Единый реестр агентов (YAML + Markdown)**
- Источник правды: Markdown-файлы с YAML-frontmatter.
- Поддержка вариантов `agent_persons` (agent_name) для A/B-инструкций.
- Приоритет загрузки: проект → пользователь → система.
- Наследование `reasoning_effort` от базовой конфигурации, если не задано в агенте.
- Приоритет параметров: `agent_name` overrides → `agent_type` значения.

2) **Интеграция агентского реестра в collab-инструменты**
- `spawn_agent` резолвит агентский профиль через реестр.
- Используются агентские параметры модели/разрешений при создании дочернего потока.

3) **Инструменты и спецификация**
- Спецификация tool-ов для multi-agent генерируется из реестра.
- Добавлен отдельный обработчик для перечисления агентов.

4) **Feature-флаги**
- `fn_multi_agents` — единый экспериментальный флаг включения fork multi-agent (реестр + collab tools).
- `collab` оставлен как legacy-алиас для совместимости.
- `collaboration_modes` и `personality` остаются отдельными флагами (upstream-поведение).

5) **System footer для системного промпта**
- Базовый footer хранится в `core/templates/collab/codex_system_footer.md`.
- Override порядок: `.codex/collab/codex_system_footer.md` (проект) → `~/.codex/collab/codex_system_footer.md` (пользователь) → шаблон.
- При первом запуске footer seed’ится в `~/.codex/collab/` (без перезаписи существующего).

6) **Fork-идентификация и локальные сборки**
- Явная маркировка версии в CLI/TUI префиксом `FN`.
- Скрипт `scripts/codex-fork-build.sh` пишет `.codex-build-hash` для контроля актуальности бинаря.

## Перечень изменений и аргументация

### Агентский реестр
- `codex-rs/core/src/agent/registry.rs`
  - Новый реестр агентов: загрузка/валидация YAML, применение профиля к `Config`.
  - **Почему:** убрать жёсткую привязку к `AgentRole`, дать декларативные профили.
- `codex-rs/core/templates/agents/codex_*.md`
  - Шаблоны встроенных агентов + seeding при первом запуске.
  - **Почему:** единый базовый набор, чтобы форк работал «из коробки».
- `codex-rs/core/src/agent/role.rs`
  - Сохранён legacy-слой ради совместимости.
  - **Почему:** не ломать существующий API/контракты сразу.

### Collab-инструменты и спецификация
- `codex-rs/core/src/tools/handlers/collab.rs`
  - `spawn_agent` переключён на реестр и агентские YAML-профили.
  - `spawn_agent` поддерживает опциональные override `model`/`reasoning_effort` с наивысшим приоритетом.
    - При наличии списка моделей — проверяется валидность `model` перед спавном.
  - `agent_list` возвращает список активных агентов (с опциональным включением self/завершённых).
  - **Почему:** согласовать runtime с декларативной моделью агентов.
- `codex-rs/core/src/tools/handlers/agents.rs`
  - Новый handler для перечисления агентов.
  - **Почему:** прозрачность доступных профилей в run-time.
- `codex-rs/core/src/tools/spec.rs` + `codex-rs/core/src/tools/spec/agent_tools.rs`
  - Динамическая спецификация для агентских tools.
  - **Почему:** корректные описания в tool schema и согласование с реестром.
- `codex-rs/core/src/codex.rs`
  - System footer для системного промпта + seeding в `~/.codex/collab/`.
  - **Почему:** единая точка для org/проектных обязательных инструкций без изменения основных шаблонов.
- `codex-rs/core/templates/collab/codex_system_footer.md`
  - Базовый footer (подлежит override).
- `codex-rs/core/src/tool_allowlist.rs`
  - Утилиты пересечения allowlist при применении агентских профилей.
  - **Почему:** агент не должен расширять доступ к инструментам поверх базовой политики.

### Конфиг и совместимость
- `codex-rs/core/src/config/mod.rs`
  - Подключение агентского реестра на уровне конфигурации.
  - **Почему:** единый вход для project/user/system layers.
- `docs/config.md`
  - Документация YAML-агентов и правил allow/deny.
  - **Почему:** прозрачная настройка без чтения кода.

### Feature-флаги
- `codex-rs/core/src/features.rs`
  - Введён `fn_multi_agents` как единый экспериментальный флаг включения fork multi-agent.
  - `collab` переведён в legacy-алиас для совместимости.
  - **Почему:** единая точка включения fork-функциональности.

### Fork-брендинг и UX
- `codex-rs/tui/src/cli.rs`
- `codex-rs/tui/src/history_cell.rs`
- `codex-rs/tui/src/status/card.rs`
- `codex-rs/tui/src/update_prompt.rs`
- `codex-rs/tui/src/chatwidget.rs` + снапшоты
  - Префикс `FN` в отображаемой версии.
  - **Почему:** чёткое различение fork-бинаря.

### TUI: хоткеи и overlay-поведение (multi-agent)
- **Ctrl+T** — только transcript (upstream-канон). Нельзя переиспользовать под multi-agent.
- **Ctrl+N** — только multi-agent overlays при включённом `fn_multi_agents`/`collaboration_modes`.
  - Цикл начинается с `AgentsSummary` → `AgentsDetails` → close.
- **Alt-screen дисциплина:** не вызывать повторный `enter_alt_screen()` при уже активном overlay.
  - Иначе затирается сохранённый viewport и возврат из overlay ведёт себя некорректно
    (терминал не восстанавливается как после transcript).
- **Цель:** поведение возврата из Ctrl+N overlay должно быть каноничным (как у Ctrl+T),
   при этом не смешивать режимы и не ломать upstream-горячие клавиши.

### Рекомендованный порядок агентов
- `orchestrator` → `explorer` → `bug-hunter` → `reviewer: strict` → `architect` → `worker`.

### Рекомендованные модели и reasoning_effort
- `architect`: `model=gpt-5.2`, `reasoning_effort=high`
- `bug-hunter`: `model=gpt-5.2`, `reasoning_effort=high`
  - `agent_name: safe` → `gpt-5.2`, `high`
  - `agent_name: risky` → `gpt-5.2`, `medium`
- `reviewer`: `model=gpt-5.2`, `reasoning_effort=medium`
  - `agent_name: strict` → `gpt-5.2`, `high`
  - `agent_name: lenient` → `gpt-5.2`, `medium`
- `explorer`: `model=gpt-5.2`, `reasoning_effort=medium`
  - `agent_name: fast` → `gpt-5.2-codex`, `medium`
  - `agent_name: deep` → `gpt-5.2`, `high`
- `worker`: `model=gpt-5.2-codex`, `reasoning_effort=medium`
- `orchestrator`: `model=gpt-5.2-codex`, `reasoning_effort=high`
  - `agent_name: lean` → `gpt-5.2-codex`, `medium`
  - `agent_name: thorough` → `gpt-5.2-codex`, `high`

### Персоналити для gpt-5.2-codex
- Зафиксировано: `gpt-5.2-codex_pragmatic.md` (через `personality = "pragmatic"`).

### Локальные сборки
- `scripts/codex-fork-build.sh`
  - Запись build-hash для детекта устаревших бинарей.
  - **Почему:** снизить риск запуска старой сборки.

### Документация форка
- `codex-rs/AGENTS.md`
  - Процесс ведения форка и правила ребейза/разрешения конфликтов.
  - **Почему:** снизить риск ошибок при регулярном обновлении от upstream.

## Ограничения и допущения

- Реестр агентов не заменяет полностью legacy API, а сосуществует с ним.
- Функциональность collab остаётся экспериментальной и требует явного включения.
- Разрешения инструментов агента ограничиваются текущей allowlist/denylist политики.
- При включённом `remote_models` метаданные инструментов берутся из `codex-rs/core/models.json`.
  Если в модели отсутствуют `experimental_supported_tools`, инструменты не регистрируются,
  и агент с allowlist на них останется без доступа. Держим `models.json` и модельные профили
  в синхроне; фолбэк‑инструменты (например `shell_command`) оставляем как страховку.

## Что проверять после обновлений

- Конфликтные зоны: `core/src/codex.rs`, `core/src/features.rs`, `core/src/tools/spec.rs`,
  `core/src/tools/handlers/collab.rs`, TUI (`chatwidget.rs`, `history_cell.rs`).
- Регрессионные тесты для `codex-core` и `codex-tui`.
