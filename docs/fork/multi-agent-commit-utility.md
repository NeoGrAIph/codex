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
| 49dd67a26 | feat: land unified_exec | high | high | medium | tbd | Требует детального анализа: смена дефолтных инструментов на non‑Windows, влияние на ожидания `shell*` в форке, совместимость маршрутизации/allowlist, влияние на параллелизм и тестовые ожидания. |
| 0efd33f7f | Update tests to stop using sse_completed fixture | tbd | tbd | tbd | tbd | tbd |
| 583e5d4f4 | Migrate state DB path helpers to versioned filename | tbd | tbd | tbd | tbd | tbd |
| df000da91 | Add a codex.rate_limits event for websockets | tbd | tbd | tbd | tbd | tbd |
| aab60a55f | nit: cleaning | tbd | tbd | tbd | tbd | tbd |
| 61aecdde6 | fix: make sure file exist in find_thread_path_by_id_str_in_subdir | tbd | tbd | tbd | tbd | tbd |
| 38f6c6b11 | chore: simplify user message detection | tbd | tbd | tbd | tbd | tbd |
| 1eb21e279 | Requirements: add source to constrained requirement values | tbd | tbd | tbd | tbd | tbd |
| 3d8deeea4 | fix: single transaction for dyn tools injection | tbd | tbd | tbd | tbd | tbd |
| 100eb6e6f | Prefer state DB thread listings before filesystem | tbd | tbd | tbd | tbd | tbd |
| 8f17b37d0 | fix(core) Request Rule guidance tweak | tbd | tbd | tbd | tbd | tbd |
| 968c02947 | fix(core) updated request_rule guidance | tbd | tbd | tbd | tbd | tbd |
| 56ebfff1a | Move metadata calculation out of client | tbd | tbd | tbd | tbd | tbd |
| 38a47700b | Add thread/compact v2 | tbd | tbd | tbd | tbd | tbd |
| fcaed4cb8 | feat: log websocket timing into runtime metrics | tbd | tbd | tbd | tbd | tbd |
| a9eb766f3 | tui: make Esc clear request_user_input notes while notes are shown | tbd | tbd | tbd | tbd | tbd |

## Шаблон для новых релизов

| Commit | Summary | Usefulness | Impact | Risk | Decision | Notes |
|---|---|---|---|---|---|---|
| <sha> | <summary> | tbd | tbd | tbd | tbd | tbd |
