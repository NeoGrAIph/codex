# Архитектура и точки расширения (для minimal diff)

## 1) Каркас системы (на уровне crate-ов)

1. `codex-rs/cli` — входная точка бинаря `codex`, роутинг сабкоманд и запуск `tui/exec/app-server`.
   - См.: `codex-rs/cli/src/main.rs`.
2. `codex-rs/core` — бизнес-логика, конфиг, тулы, оркестрация turn/thread, policy/sandbox.
   - См.: `codex-rs/core/src/lib.rs`.
3. `codex-rs/protocol` — внутренние/общие wire-типы.
   - См.: `codex-rs/protocol/src/*`.
4. `codex-rs/app-server-protocol` + `codex-rs/app-server` — внешнее JSON-RPC API для IDE/интеграций.
   - См.: `codex-rs/app-server-protocol/src/protocol/v2.rs`, `codex-rs/app-server/README.md`.
5. `codex-rs/tui` — UI-слой, привязка к core/config, рендер, hotkeys.
   - См.: `codex-rs/tui/src/lib.rs`.
6. `codex-rs/exec` — non-interactive сценарии (`codex exec` / review flow).

## 2) Слой конфигурации и ограничений

Критично для любой новой фичи, влияющей на runtime behavior:

1. Типы user config: `codex-rs/core/src/config/mod.rs` (`ConfigToml`, `Config`).
2. Порядок слоёв и trust/requirements enforcement:
   - `codex-rs/core/src/config_loader/mod.rs`.
3. Feature flags:
   - `codex-rs/core/src/features.rs`.

Ключевой факт: в коде уже есть сложная многоуровневая модель (`system/user/cwd/tree/repo/runtime + requirements`).
Для минимального diff расширять нужно существующие точки (feature/config fields), а не вводить параллельный механизм.

## 3) Tooling pipeline (куда добавлять новый tool)

Базовые точки:

1. Регистрация/описание tool specs:
   - `codex-rs/core/src/tools/spec.rs`
2. Реализация handler:
   - `codex-rs/core/src/tools/handlers/mod.rs`
   - конкретный модуль в `codex-rs/core/src/tools/handlers/*`
3. Если нужен wire/API экспортер:
   - `codex-rs/app-server-protocol/src/protocol/v2.rs`
   - с последующей регенерацией схем.

Правило минимального diff: добавить новый handler + подключение в существующую фабрику, не перерабатывая общий `tools/spec` pipeline.

## 4) Skills/Hooks как наименее конфликтные extension точки

1. Skills:
   - `codex-rs/core/src/skills/mod.rs`
   - `codex-rs/core/src/skills/loader.rs`
   - `codex-rs/core/src/skills/manager.rs`
2. Hooks:
   - `codex-rs/hooks/src/types.rs`
   - `codex-rs/hooks/src/registry.rs`

Если фича может быть реализована как skill/hook/обертка над существующими tool calls, это обычно дает наименьший конфликт с upstream.

## 5) API-правила для app-server (обязательно)

По правилам в `AGENTS.md` при изменении API:

1. Весь новый API surface добавляется в `v2`, не в `v1`.
2. Соблюдать naming и wire-shape (`*Params`, `*Response`, camelCase на wire, ts-rs exports).
3. Для optional полей в client->server `*Params` соблюдать `#[ts(optional = nullable)]`.
4. После изменения API обязательно:
   - обновить docs (`codex-rs/app-server/README.md` минимум),
   - регенерировать схемы `just write-app-server-schema`,
   - прогнать `cargo test -p codex-app-server-protocol`.

## 6) Где высокий риск merge-конфликтов

По истории и текущему diff-футпринту форка чаще всего конфликтуют:

1. `codex-rs/core/src/*`
2. `codex-rs/tui/src/*`
3. `codex-rs/app-server-protocol/src/*`
4. `codex-rs/app-server-protocol/schema/*` (generated)
5. `codex-rs/core/config.schema.json` (generated)

Следствие: планировать изменения так, чтобы не затрагивать одновременно `core + tui + app-server-protocol`, если можно разделить на этапы.

## 7) Практическая матрица: тип фичи -> минимальный набор файлов

### A. Новая команда/флаг CLI

1. `codex-rs/cli/src/main.rs`
2. при необходимости один модуль в `codex-rs/cli/src/*`

### B. Новый tool без внешнего API

1. `codex-rs/core/src/tools/handlers/<tool>.rs`
2. `codex-rs/core/src/tools/handlers/mod.rs`
3. минимальные правки в `codex-rs/core/src/tools/spec.rs`
4. тесты `codex-rs/core/tests/suite/*`

### C. Новый behavior под feature flag

1. `codex-rs/core/src/features.rs`
2. точечные изменения в runtime-модуле
3. тесты на default/off/on

### D. Изменение config.toml surface

1. `codex-rs/core/src/config/mod.rs` (+ при необходимости `types.rs`, `profile.rs`)
2. `just write-config-schema`
3. тесты precedence/validation

### E. Изменение app-server контракта

1. `codex-rs/app-server-protocol/src/protocol/common.rs` и/или `v2.rs`
2. `codex-rs/app-server/src/*`
3. `just write-app-server-schema`
4. тесты `codex-rs/app-server-protocol` + `codex-rs/app-server`

## 8) Что не стоит трогать без явной необходимости

1. `vendor/`, sandbox low-level, release workflows, Bazel/CI конфиги.
2. Большие generated артефакты вручную.
3. Широкие рефакторы в `core/src/lib.rs` и cross-cutting переименования.

