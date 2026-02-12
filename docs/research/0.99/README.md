# Исследование для форка на базе rust-v0.99.0

Дата среза: 2026-02-12
База: `rust-v0.99.0` (`ec9f76ce4f854c7d4f3c78c9b1bacbe128df286e`)

## Зачем этот пакет

Этот пакет отвечает на вопрос: **какая информация нужна, чтобы внедрять новый функционал в форк корректно, синергично с upstream и с минимальным diff**.

## Короткий ответ: какая информация нужна обязательно

Ниже минимальный набор, без которого нельзя проектировать изменения качественно:

1. Product scope: что именно добавляем, а что явно не делаем.
2. Поверхности изменения: где фича должна работать (`tui`, `exec`, `app-server`, `sdk`, только локально/и API).
3. Совместимость: должны ли сохраняться текущие внешние контракты (JSON-RPC v2, config schema, wire behavior).
4. Security policy: меняются ли approvals/sandbox/exec-policy/requirements.
5. UX policy: нужны ли новые команды/горячие клавиши/режимы и насколько можно менять существующий UX.
6. Rollout policy: фича-флаг, дефолтное состояние, критерии включения/отката.
7. Verification policy: какие тесты/чек-листы считаются обязательными до merge.

## Что уже исследовано в репозитории

1. Базовая архитектура и границы crate-ов (`codex-rs/Cargo.toml`, `codex-rs/core/src/lib.rs`, `codex-rs/tui/src/lib.rs`, `codex-rs/app-server-protocol/src/protocol/v2.rs`).
2. Слои конфига и requirements enforcement (`codex-rs/core/src/config_loader/mod.rs`, `codex-rs/core/src/config/mod.rs`).
3. Точки расширения tools/skills/hooks (`codex-rs/core/src/tools/handlers/mod.rs`, `codex-rs/core/src/tools/spec.rs`, `codex-rs/core/src/skills/mod.rs`, `codex-rs/hooks/src/*`).
4. CI/release quality gates (`.github/workflows/rust-ci.yml`, `.github/workflows/rust-release.yml`, `justfile`).
5. Текущее состояние веток форка относительно `rust-v0.99.0`.
6. Поведение TUI окон/overlay и инварианты alt-screen lifecycle для hotkey-циклов (`Ctrl+T` и смежные сценарии).

## Что нужно от команды до начала реализации

См. `OPEN_QUESTIONS_CHECKLIST.md`. Если кратко: надо письменно зафиксировать API/UX/security границы фичи и определить ветку-носитель (новая от `rust-v0.99.0` или продолжение существующего большого patch stack).

## Состав пакета

1. `BASELINE_AND_BRANCH_STATE.md`
2. `ARCHITECTURE_AND_EXTENSION_POINTS.md`
3. `MIN_DIFF_IMPLEMENTATION_STRATEGY.md`
4. `QUALITY_GATES_AND_CHECKS.md`
5. `OPEN_QUESTIONS_CHECKLIST.md`
6. `TUI_WINDOWS_AND_OVERLAYS.md`
