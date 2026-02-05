> Owner: fork maintainers | Scope: multi-agent behavior | Audience: dev  
> Status: draft | Last reviewed: 2026-02-05 | Related: docs/fork/upstream-main-commits.md

# Полезность апстрим‑коммитов для форка (multi‑agent)

Документ фиксирует, насколько каждый апстрим‑коммит полезен для форка в части мультиагентности, и какое решение принято (взять/пропустить/нужна адаптация).

## Легенда
- **Полезность**: `high` / `medium` / `low` / `none`
- **Влияние**: `high` / `medium` / `low` (на UX/поведение/операции в multi-agent)
- **Риск**: `high` / `medium` / `low` (регрессии, совместимость, поддержка)
- **Решение**: `adopt` / `skip` / `adapt` / `tbd`
- В **Notes** фиксируем вопросы и аспекты для всестороннего анализа, без директивных рекомендаций.

## rust-v0.96.0 (17 коммитов)

| Commit | Summary | Usefulness | Impact | Risk | Decision | Notes |
|---|---|---|---|---|---|---|
| 2572f96fa | New Features bundle (thread/compact v2, websocket rate limits, unified_exec non‑Windows, requirements source provenance) | none | low | low | skip | Meta release commit; only bumps version in `codex-rs/Cargo.toml`, no functional multi-agent changes. Revisit only with actual commits in range. |
| 49dd67a26 | feat: land unified_exec | high | high | medium | tbd | Требует детального анализа, в том числе: смена дефолтных инструментов на non‑Windows, влияние на ожидания `shell*` в форке, совместимость маршрутизации/allowlist, влияние на параллелизм и тестовые ожидания. |
| 0efd33f7f | Update tests to stop using sse_completed fixture | low | low | low | tbd | Требует детального анализа, в том числе: есть ли в форке собственные SSE‑фикстуры/хелперы поверх `completed_template.json`, есть ли расхождения в `core/tests/**`, и нужна ли адаптация новых `responses::ev_*` помощников для fork‑специфичных сценариев multi‑agent. |
| 583e5d4f4 | Migrate state DB path helpers to versioned filename | medium | medium | medium | tbd | Требует детального анализа, в том числе: аудит использования путей к state DB в форке и тулзах, необходимость миграции/сохранения legacy‑данных, сценарии отката версий и взаимодействие мультиагентных процессов с версионированием файла. |
| df000da91 | Add a codex.rate_limits event for websockets | high | medium | medium | tbd | Требует детального анализа, в том числе: формат и источник `codex.rate_limits` в форке, влияние на multi‑agent throttling/telemetry, совместимость с существующей обработкой `ResponseEvent` и возможные конфликты в websocket‑pipeline. |
| aab60a55f | nit: cleaning | low | low | low | tbd | Требует детального анализа, в том числе: совпадает ли в форке логика определения user events с апстрим‑контрактом (EventMsg::UserMessage vs ResponseItem::Message), и не ломают ли обновлённые тесты локальные изменения. |
| 61aecdde6 | fix: make sure file exist in find_thread_path_by_id_str_in_subdir | medium | low | low | tbd | Требует детального анализа, в том числе: есть ли в форке собственные изменения в `rollout/list.rs`, и нужны ли корректировки метрик/логирования для multi‑agent сценариев. |
| 38f6c6b11 | chore: simplify user message detection | medium | medium | medium | tbd | Требует детального анализа, в том числе: как форк сейчас формирует `has_user_event`/`title`, есть ли зависимость от `ResponseItem::Message` role=user в мультиагентных сценариях, и какой путь остаётся для извлечения пользовательского текста (например, `TurnContextItem` или другие источники). |
| 1eb21e279 | Requirements: add source to constrained requirement values | medium | medium | medium | tbd | Требует детального анализа, в том числе: где в форке используются `ConfigRequirements` и `Constrained<T>`, как выводится `/debug-config`, и нужно ли расширять fork‑специфичные требования/поля новым `ConstrainedWithSource`. |
| 3d8deeea4 | fix: single transaction for dyn tools injection | high | medium | low | tbd | Требует детального анализа, в том числе: есть ли в форке дополнительные операции внутри инъекции tools, и не приводит ли транзакция к конфликтам при параллельной записи/чтении в state DB. |
| 100eb6e6f | Prefer state DB thread listings before filesystem | high | medium | medium | tbd | Требует детального анализа, в том числе: совместимость с fork‑изменениями в `rollout/recorder.rs`, корректность метаданных `has_user_event/title` для multi‑agent UI, и поведение при несогласованности state DB и файловой системы. |
| 8f17b37d0 | fix(core) Request Rule guidance tweak | low | low | low | tbd | Требует детального анализа, в том числе: согласованность с форк‑политикой эскалации и локальными правилами `approval_policy` в мультиагентном режиме, а также влияние на частоту запросов approval в проде. |
| 968c02947 | fix(core) updated request_rule guidance | low | low | low | tbd | Требует детального анализа, в том числе: совпадение с fork‑специфичными правками в prompts/approval_policy и тем, как multi‑agent использует префикс‑разрешения. |
| 56ebfff1a | Move metadata calculation out of client | medium | medium | medium | tbd | Требует детального анализа, в том числе: влияние таймаута и фонового прогрева на стабильность headers в мультиагентных потоках, изменения API `ModelClient::new_session`, и возможные конфликты с форк‑логикой turn metadata/кэширования. |
| 38a47700b | Add thread/compact v2 | medium | medium | medium | tbd | Требует детального анализа, в том числе: совместимость с fork‑RPC и UI, необходимость обновления клиентов/SDK, и влияние на orchestration компакции в multi‑agent режиме. |
| fcaed4cb8 | feat: log websocket timing into runtime metrics | medium | medium | medium | tbd | Требует детального анализа, в том числе: есть ли в форке потребность в этих метриках для мультиагентных сравнений, как и где агрегируются runtime metrics по агентам/сессиям, и совместимость с текущим backend Responses API (поддержка `responsesapi.websocket_timing`). |
| a9eb766f3 | tui: make Esc clear request_user_input notes while notes are shown | low | low | low | tbd | Требует детального анализа, в том числе: есть ли в форке кастомный UX для `request_user_input` и нужно ли синхронизировать снапшоты/документацию TUI. |

## rust-v0.98.0 (32 коммита)

| Commit | Summary | Usefulness | Impact | Risk | Decision | Notes |
|---|---|---|---|---|---|---|
| 7f2035761 | Stop client from being state carrier (#10595) | high | high | medium | adapt | Рефакторинг TurnContext/ModelClient, большая площадь изменений, возможны конфликты с форк‑правками. |
| 282f42c0c | Add option to approve and remember MCP/Apps tool usage (#10584) | medium | medium | medium | adapt | Сессионное авто‑одобрение codex_apps; в multi‑agent может распространяться между агентами. |
| 71e63f8d1 | fix: flaky test (#10644) | low | low | low | adopt | Стабилизация таймингового теста, изменения только в тестовом коде. |
| e9335374b | feat: add phase 1 mem client (#10629) | medium | medium | medium | adopt | Новый endpoint `/v1/memories/trace_summarize`, публичные типы и `core/memory_trace`; без fork‑guard. |
| 1b153a3d4 | Cloud Requirements: take precedence over MDM (#10633) | high | medium | low | adopt | Порядок слоёв: cloud перед MDM; macOS‑тест фиксирует приоритет. |
| 95269ce88 | Increase cloud req timeout (#10659) | low | low | low | adopt | Таймаут загрузки cloud‑требований 5s→15s; дольше блокирует старт. |
| ae4de43cc | feat(linux-sandbox): add bwrap support (#9938) | high | medium | medium | adopt | Фичефлаг bwrap, обновления CI/тестов, новый FFI‑путь при включении. |
| 7a253076f | Persist pending input user events (#10656) | medium | low | low | adopt | Pending input фиксируется как UserMessage, тест ждёт новое событие. |
| 4922b3e57 | feat: add phase 1 mem db (#10634) | medium | high | medium | adapt | Новая таблица thread_memory, миграция и API runtime; затрагивает state DB. |
| d589ee05b | Fix jitter in TUI apps/connectors picker (#10593) | medium | low | low | adopt | Новый ColumnWidthMode и стабильная колонка описаний; затронуты только apps/connectors, добавлены снапшоты. |
| acdbd8edc | [apps] Cache MCP actions from apps. (#10662) | medium | medium | medium | adopt | Кэш tool‑list на 1 час для CODEX_APPS_MCP_SERVER_NAME; возможна задержка обновлений до TTL. |
| 7c6d21a41 | Fix test_shell_command_interruption flake (#10649) | low | low | low | adopt | Стабилизация теста прерывания shell‑команд; продукт не меняется. |
| d452bb3ae | Add /debug-config slash command (#10642) | medium | low | low | adopt | Диагностический вывод стека слоёв и источников требований; без изменения поведения. |
| 7bcc55232 | Added support for live updates to skills (#10478) | high | medium | medium | adapt | Сброс кэша навыков и событие SkillsUpdateAvailable; AGENTS.md не отслеживается. |
| f9c38f531 | add none personality option (#10688) | medium | medium | low | adopt | Добавлен вариант personality=none, обновлены схема/доки/тесты, TUI подписи для None. |
| 5ea107a08 | feat(app-server, core): allow text + image content items for dynamic tool outputs (#10567) | medium | high | high | adapt | Меняет контракт dynamic_tools, payload FunctionCallOutput и схемы протокола; затронут core и тесты. |
| 224c9f768 | chore(app-server): document experimental API opt-in (#10667) | low | low | low | adopt | Документация README про opt‑in capabilities.experimentalApi, без поведенческих изменений. |
| 0e8d359da | Session-level model client (#10664) | medium | high | high | adapt | Крупный рефактор ModelClient/Session, новые аргументы на turn‑level, удалён TransportManager; риск конфликтов с форк‑правками core. |
| cddfd1e67 | feat(core): add configurable log_dir (#10678) | medium | low | low | adopt | Новый log_dir в ConfigToml/схеме и относительные пути для CLI overrides; обновлены docs. |
| 1f47e08d6 | Cloud Requirements: increase timeout and retries (#10631) | medium | medium | low | adopt | Ретраи с backoff при загрузке cloud requirements, backoff стал pub, добавлены тесты. |
| 73f32840c | chore(core) personality migration tests (#10650) | low | low | low | adopt | Дополнительные тесты миграции personality (meta‑only, explicit profile, идемпотентность) и app‑server сценарий. |
| d876f3b94 | fix(tui): restore working shimmer after preamble output (#10701) | medium | medium | medium | adapt | Правит видимость статуса между preamble и exec; есть снапшот; обновлён MODULE.bazel.lock. |
| 4ed8d74aa | fix: ensure status indicator present earlier in exec path (#10700) | medium | low | low | adopt | Гарантирует показ статуса при unified exec (включая Unknown); добавлены тесты/снапшот. |
| 1dc06b6ff | fix: ensure resume args precede image args (#10709) | high | medium | low | adopt | Меняет порядок `resume`/`--image`, предотвращая новый тред; добавлен тест SDK. |
| a05aadfa1 | chore(config) Default Personality Pragmatic (#10705) | medium | medium | low | adopt | Меняет дефолт личности на Pragmatic; обновлены core тесты и TUI снапшот. |
| e48297826 | fix(core) switching model appends model instructions (#10651) | high | medium | medium | adopt | При смене модели добавляет developer‑сообщение с инструкциями; меняет порядок update‑сообщений и сценарии кэширования. |
| 41b4962b0 | Sync collaboration mode naming across Default prompt, tools, and TUI (#10666) | medium | medium | low | adapt | Централизует display_name/видимость/availability режимов; затрагивает default prompt и TUI, возможна рассинхронизация при форк‑режимах. |
| cd5f49a61 | Make steer stable by default (#10690) | medium | medium | medium | adopt | Меняет дефолт ввода в TUI (Enter сразу отправляет, Tab ставит в очередь); в multi‑agent с параллельными задачами может сместить модель очереди. |
| dc7007bea | Fix remote compaction estimator/payload instruction small mismatch (#10692) | high | medium | medium | adopt | Синхронизирует оценку токенов и compact payload по base_instructions; снижает риск переполнения контекста в длинных multi‑agent сессиях. |
| 1e1146cd2 | Reload cloud requirements after user login (#10725) | low | low | low | adopt | Перезагружает cloud requirements после логина и синхронизирует residency; влияет на доступность функций/политик без рестарта. |
| fe8b474ac | fix(core,app-server) resume with different model (#10719) | high | medium | low | adopt | Одноразовая вставка developer‑инструкций при resume через pending_resume_previous_model; расширены тесты app‑server/core. |
| 82464689c | ## New Features - Steer mode is now stable and enabled by default, so `Enter` sends immediately during running tasks while `Tab` explicitly queues follow-up input. (#10690) | none | low | low | skip | Только bump версии в `codex-rs/Cargo.toml`; функциональных изменений нет. |

## Шаблон для новых релизов

| Commit | Summary | Usefulness | Impact | Risk | Decision | Notes |
|---|---|---|---|---|---|---|
| <sha> | <summary> | tbd | tbd | tbd | tbd | tbd |
