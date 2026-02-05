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

## Шаблон для новых релизов

| Commit | Summary | Usefulness | Impact | Risk | Decision | Notes |
|---|---|---|---|---|---|---|
| <sha> | <summary> | tbd | tbd | tbd | tbd | tbd |
