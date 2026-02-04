# Multi-agent notes

> Owner: <team/owner> | Scope: fork/colab-agents | Audience: devs
> Status: active | Last reviewed: 2026-02-04 | Related: docs/fork/colab-agents.md

## Категории и критерии выбора

Выбирай категорию по доминирующему аспекту проблемы:

- **Bug** — функциональность ведёт себя не так, как описано в спецификации или ожиданиях пользователя (например, регрессия, некорректный ответ API).
- **Incident** — проблема уже затронула prod/stage и требует расследования последствий (простой сервиса, деградация SLA, аварийная операция).
- **Security** — обнаружено нарушение политики безопасности или уязвимость, которую может использовать злоумышленник (отсутствие аутентификации, утечка данных, риск эскалации привилегий).
- **Performance** — система не удовлетворяет требованиям по времени ответа, пропускной способности или ресурсопотреблению (рост задержек, узкое место в ресурсах, неэффективные алгоритмы).
- **Reliability** — поведение компонента нестабильно или приводит к сбоям/неопределённым состояниям без явного нарушения функциональности (тесты, редкие сбои, сложность восстановления).
- **Documentation** — недостаточно информации в руководствах или спецификациях, мешающее эксплуатации или разработке (устаревшие инструкции, отсутствие схем).
- **Process/DX** — неэффективность в инструментах разработки, CI/CD, рабочих процессах команды (долгие сборки, несогласованные процедуры).
- **Enhancement** — улучшение существующего поведения или добавление нового функционала без критичной необходимости (фич-запросы, UX-полировки).

Если ситуация попадает сразу в несколько категорий, выбери основную по наиболее критичному аспекту и перечисли дополнительные в поле «Теги».

## Уровни важности

- **Critical** — требуется немедленное реагирование; риск серьёзных последствий или нарушение регуляторных/контрактных обязательств.
- **High** — существенно влияет на функциональность или пользователей, но допускает короткую задержку перед устранением.
- **Medium** — влияние ограничено, существуют обходные пути или воздействие минимально.
- **Low** — низкий риск и влияние; может быть запланировано на будущее окно или совмещено с другими задачами.

## Шаблон записи

`Проблемы и риски`/`Улучшения`

```md
### <Краткое имя задачи>
- **Категория:** <выбери из списка выше>
- **Важность:** <Critical | High | Medium | Low>
- **Обнаружено:** <дата>
- **Контекст:** <где замечено: сервис/окружение/версия>
- **Описание:**
  - *Фактическое поведение:* <что происходит сейчас>
  - *Ожидаемое поведение:* <как должно работать>
- **Шаги воспроизведения:**
  1. <минимально достаточные шаги>
  2. <...>
- **Последствия/риск:** <какого масштаба влияние, кого затронуло>
- **Предложенное решение:** <гипотеза исправления или направления анализа>
- **Связанные материалы:** <ссылки на тикеты, PR, ADR, документацию>
- **Теги:** <ключевые слова через запятую для поиска и аналитики>
- **Статус:** <Open | In Progress | Blocked | Resolved> (при закрытии указать ссылку на фиксацию)
```

## Проблемы и риски

### Spawned Explorer Lacks Repo Read Tools
- **Категория:** Process/DX
- **Важность:** High
- **Обнаружено:** 2026-02-04
- **Контекст:** `spawn_agent` → `agent_type=explorer` (локальная сессия Codex CLI)
- **Описание:**
  - *Фактическое поведение:* заспавненный `explorer` сообщает, что не имеет доступа к ФС/инструментам чтения и не может ориентироваться в репозитории.
  - *Ожидаемое поведение:* `explorer` должен иметь read-only инструменты (`read_file`, `list_dir`, `grep_files`) и уметь выдавать точные “файл → что там”.
- **Шаги воспроизведения:**
  1. Вызвать `spawn_agent` с `agent_type=explorer` и задачей “найди в коде X”.
  2. Получить ответ, что доступов/инструментов нет.
- **Последствия/риск:** multi-agent распараллеливание “ломается” на базовом сценарии (поиск/навигация), теряем основную выгоду от `explorer`.
- **Предложенное решение:** подтвердить источник “tool-less explorer” и сделать поведение детерминированным.
  Выявленная причина: при включённых `RemoteModels` (по умолчанию) метаданные модели берутся из remote‑описания (кэш `~/.codex/models_cache.json`), где для `gpt-5.2-codex` `experimental_supported_tools` оказался пустым. Из‑за отсутствия слияния с локальными дефолтами `read_file/list_dir/grep_files` не регистрировались в tool registry → allowlist агента указывал на несуществующие инструменты.
  Исправление:
  1) Добавить backfill `experimental_supported_tools` из локальных дефолтов при пустом remote‑описании.
  2) Разрешить allowlist `shell_command`/`shell` для `UnifiedExec`, чтобы `exec_command`/`write_stdin` не отфильтровывались.
- **Связанные материалы:** `docs/fork/colab-agents.md`
- **Теги:** spawn_agent, explorer, tools, dx, registry, allowlist
- **Статус:** Resolved (backfill tool‑метаданных + allowlist alias для `UnifiedExec`)

### Explorer всё ещё без файловых инструментов в реальной сессии
- **Категория:** Process/DX
- **Важность:** High
- **Обнаружено:** 2026-02-04
- **Контекст:** `spawn_agent` → `agent_type=explorer` при разборе `/home/neograiph/repo/AGENTS/custom-agent-with-skills`
- **Описание:**
  - *Фактическое поведение:* заспавненный `explorer` пишет, что доступен только `web.run`; нет `read_file`/`list_dir`/`grep_files` и shell, поэтому не может читать локальные файлы.
  - *Ожидаемое поведение:* `explorer` должен получать read‑only инструменты и уметь ориентироваться в локальном репозитории.
- **Шаги воспроизведения:**
  1. Вызвать `spawn_agent` с `agent_type=explorer` и задачей на локальный репозиторий.
  2. Получить ответ, что доступов нет.
- **Последствия/риск:** фактическое распараллеливание по навигации в коде недоступно; теряем эффективность в базовом сценарии.
- **Предложенное решение:** проверить назначение tool allowlist при `spawn_agent` и согласованность с `list_agents`, а также учесть `UnifiedExec` (alias `shell_command` → `exec_command`/`write_stdin`).
- **Связанные материалы:** `docs/fork/colab-agents.md`, текущая сессия (2026‑02‑04)
- **Теги:** spawn_agent, explorer, tools, dx, regression
- **Статус:** Resolved (fix in `codex-rs/core/src/models_manager/manager.rs` + alias в `codex-rs/core/src/tools/spec.rs`)

### RemoteModels обнуляет experimental_supported_tools для explorer
- **Категория:** Process/DX
- **Важность:** High
- **Обнаружено:** 2026-02-04
- **Контекст:** code-review `codex-rs` (multi-agent, `gpt-5.2-codex`, `RemoteModels`)
- **Описание:**
  - *Фактическое поведение:* при включённом `RemoteModels` модель берётся из remote списка без слияния с локальными дефолтами; если `experimental_supported_tools` пустой, `read_file/list_dir/grep_files` не регистрируются → explorer остаётся без файловых инструментов.
  - *Ожидаемое поведение:* локальные дефолты (из `models.json`/`model_info.rs`) должны дополнять remote‑метаданные хотя бы для `experimental_supported_tools`.
- **Шаги воспроизведения:**
  1. Включить `remote_models` (по умолчанию true).
  2. Получить remote‑описание `gpt-5.2-codex` с пустым `experimental_supported_tools`.
  3. Вызвать `spawn_agent` для `explorer` и убедиться, что `read_file/list_dir/grep_files` недоступны.
- **Последствия/риск:** базовый multi‑agent сценарий “поиск по репо” ломается без явной диагностики.
- **Предложенное решение:** объединять remote‑метаданные с локальными дефолтами для `experimental_supported_tools`, или добавить явный лог/алерт при пустом списке.
- **Связанные материалы:** `codex-rs/core/src/models_manager/manager.rs`, `codex-rs/core/src/tools/spec.rs`, `codex-rs/core/src/models_manager/model_info.rs`
- **Теги:** remote_models, experimental_supported_tools, explorer, tools
- **Статус:** Resolved (fix in `codex-rs/core/src/models_manager/manager.rs`)

### UnifiedExec теряет инструменты из‑за allowlist `shell_command`
- **Категория:** Process/DX
- **Важность:** Medium
- **Обнаружено:** 2026-02-04
- **Контекст:** `spawn_agent` → `agent_type=explorer`, `unified_exec=true`, allowlist в `~/.codex/agents/codex_explorer.md`
- **Описание:**
  - *Фактическое поведение:* allowlist содержит `shell_command`, но реальные инструменты — `exec_command`/`write_stdin`, поэтому фильтр удаляет их и агент остаётся без shell‑инструментов.
  - *Ожидаемое поведение:* `shell_command` (и `shell`) должны считаться алиасом для `exec_command`/`write_stdin` при `UnifiedExec`.
- **Шаги воспроизведения:**
  1. Включить `unified_exec=true`.
  2. Заспавнить `explorer` с allowlist, где есть `shell_command`, но нет `exec_command`.
  3. Убедиться, что `exec_command`/`write_stdin` отфильтрованы.
- **Последствия/риск:** агенты остаются без shell‑инструментов даже при корректном allowlist.
- **Предложенное решение:** добавить маппинг алиасов allowlist → `UnifiedExec` + предупреждать в логе, если фильтр удалил все инструменты.
- **Связанные материалы:** `codex-rs/core/src/tools/spec.rs`
- **Теги:** unified_exec, allowlist, shell_command, tools
- **Статус:** Resolved (alias + warn‑лог в `codex-rs/core/src/tools/spec.rs`)

### Flaky тайминг в `tool_parallelism::read_file_tools_run_in_parallel`
- **Категория:** Reliability
- **Важность:** Low
- **Обнаружено:** 2026-02-04
- **Контекст:** `cargo test --all-features` (локальная среда), `codex-rs/core/tests/suite/tool_parallelism.rs`
- **Описание:**
  - *Фактическое поведение:* тест упал по таймингу: `expected parallel execution to finish quickly, got 2.347s`.
  - *Ожидаемое поведение:* стабильное прохождение в средах с вариативной нагрузкой.
- **Шаги воспроизведения:**
  1. Запустить `cargo test --all-features`.
  2. Дождаться фейла `read_file_tools_run_in_parallel`.
- **Последствия/риск:** нестабильный CI/локальные прогоны при высокой нагрузке.
- **Предложенное решение:** смягчить тайм‑критерий и привязать его к warmup‑базе, чтобы учесть накладные расходы среды.
- **Связанные материалы:** `codex-rs/core/tests/suite/tool_parallelism.rs`
- **Теги:** tests, flake, timing, reliability
- **Статус:** Resolved (warmup‑база + headroom в `codex-rs/core/tests/suite/tool_parallelism.rs`)

### Flaky тайминг в `unified_exec_terminal_interaction_captures_delayed_output`
- **Категория:** Reliability
- **Важность:** Low
- **Обнаружено:** 2026-02-04
- **Контекст:** `cargo test -p codex-core --test all` (локальная среда), `codex-rs/core/tests/suite/unified_exec.rs`
- **Описание:**
  - *Фактическое поведение:* периодически падает `suite::unified_exec::unified_exec_terminal_interaction_captures_delayed_output` с `assertion failed: expected three terminal interactions`.
  - *Ожидаемое поведение:* стабильный проход вне зависимости от фоновой нагрузки.
- **Шаги воспроизведения:**
  1. Запустить `cargo test -p codex-core --test all`.
  2. Получить фейл в `unified_exec_terminal_interaction_captures_delayed_output`.
- **Последствия/риск:** нестабильные прогоны тестов по `codex-core`.
- **Предложенное решение:** проверить тайм‑пороги и ожидания в тесте, рассмотреть более устойчивый критерий/timeout.
- **Связанные материалы:** `codex-rs/core/tests/suite/unified_exec.rs`
- **Теги:** tests, flake, timing, unified_exec
- **Статус:** Open (test ignored; needs stabilization)

### Flaky тайминг в `remote_models_request_times_out_after_5s`
- **Категория:** Reliability
- **Важность:** Low
- **Обнаружено:** 2026-02-04
- **Контекст:** `cargo test -p codex-core --test all` (локальная среда), `codex-rs/core/tests/suite/remote_models.rs`
- **Описание:**
  - *Фактическое поведение:* тест падает по таймингу: `expected models call to time out before the delayed response; took 6.194s`.
  - *Ожидаемое поведение:* стабильный таймаут до 5s даже при шуме среды.
- **Шаги воспроизведения:**
  1. Запустить `cargo test -p codex-core --test all`.
  2. Дождаться фейла `remote_models_request_times_out_after_5s`.
- **Последствия/риск:** нестабильные прогоны `codex-core`.
- **Предложенное решение:** пересмотреть порог/механику ожидания в тесте (например, допустить небольшой буфер или использовать более устойчивый таймер).
- **Связанные материалы:** `codex-rs/core/tests/suite/remote_models.rs`
- **Теги:** tests, flake, timing, remote_models
- **Статус:** Open (test ignored; needs stabilization)

## Улучшения

### Нет локального AGENTS.md в custom-agent-with-skills
- **Категория:** Documentation
- **Важность:** Low
- **Обнаружено:** 2026-02-04
- **Контекст:** `/home/neograiph/repo/AGENTS/custom-agent-with-skills`
- **Описание:**
  - *Фактическое поведение:* в репозитории отсутствует `AGENTS.md`, поэтому нет локальных инструкций/ограничений.
  - *Ожидаемое поведение:* иметь короткий `AGENTS.md` с правилами репозитория (запуск `uv`, тесты, соглашения по навыкам).
- **Шаги воспроизведения:**
  1. Выполнить поиск `AGENTS.md` в репозитории.
- **Последствия/риск:** выше вероятность неверных допущений при работе с кодом/тестами.
- **Предложенное решение:** добавить минимальный `AGENTS.md` с локальными правилами и обязательными командами проверки.
- **Связанные материалы:** `README.md` в репозитории custom-agent-with-skills
- **Теги:** documentation, dx, agents
- **Статус:** Open
