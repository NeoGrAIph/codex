# План доработок fork/colab-agents

> Owner: <team/owner> | Scope: fork/colab-agents | Audience: devs
> Status: active | Last reviewed: 2026-02-04 | Related: docs/fork/diff-fork-vs-main.md, docs/fork/colab-agents.md, docs/fork/upstream-main-commits.md

## Цель

Сохранить fork‑ценность (multi‑agents + агентский реестр) и одновременно
минимизировать дрейф относительно upstream, следуя принципу **canonical‑first**.

## Объём (Scope)

Входит:
- `codex-rs/core` (agent registry, collab, allow/deny‑пересечения)
- `codex-rs/tui` (fork‑брендинг версии + UI‑совместимость)
- `codex-rs/app-server-protocol` (схемы/fixtures)
- процесс интеграции upstream‑коммитов (commit‑by‑commit)

Не входит:
- новые функциональные фичи вне fork‑ценности
- изменения без опоры на diff‑анализ

## Fork‑ядро (не теряем)

- Агентский реестр и YAML‑описания (`core/src/agent/registry.rs` + templates).
- Инструменты `list_agents`/`read_agent` и расширенный `spawn_agent`.
- `fn_multi_agents` как единый флаг включения fork‑мультиагентов.
- Пересечения allow/deny‑листов для агентов.
- Fork‑брендинг версии (`FN`).

## Принципы каноничности

1) **Upstream‑first**: поведение upstream — дефолт.
2) **Fork‑слой минимальный**: форк‑функции — как надстройка.
3) **Генерация без ручных правок**: любые schema/fixtures только через генерацию.
4) **Commit‑by‑commit**: интеграции делаются по одному коммиту с анализом/документацией.

## Этапы доработок

### Этап 1: База и версия
- Выравнять `workspace.version` с базовым релизом (main v0.95.0).
- Проверить, что fork‑брендинг версии остаётся актуальным.

### Этап 2: Core‑слой (fork‑ценность)
- Убедиться, что `fn_multi_agents` покрывает все fork‑фичи.
- Проверить, что allow/deny‑пересечения не расширяют доступ.
- Убедиться, что `spawn_agent` корректно использует `agent_type/agent_name`.

### Этап 3: TUI‑слой (Upstream Sync)
- Максимально синхронизировать базовый UX/поведение с upstream.
- Не смешивать fork‑UX изменения в этот этап.

### Этап 4: TUI‑слой (Fork UX: Multi‑agents / Collab)
- Изолировать fork‑UX, связанный с multi‑agents/collab режимами и overlays.
- Держать изменения минимальными и избегать касания не относящихся к fork‑ценности частей TUI.

### Этап 5: TUI‑слой (Верификация и снапшоты)
- Прогнать `cargo test -p codex-tui`.
- Если в этом этапе меняется UI-вывод/рендеринг: проверить снапшоты (`cargo insta pending-snapshots -p codex-tui`) и принять изменения осознанно.
- Ручной smoke-прогон ключевых TUI сценариев после merge.

### Этап 6: app-server-protocol
- Любые изменения переносить целиком.
- При несовпадениях — регенерировать fixtures/схемы.

### Этап 7: Документация
- `docs/config.md` всегда отражает реальные fork‑фичи.
- Документация форка остаётся в `docs/fork/*`.

## Минимальный тестовый контур

- `cargo test -p codex-core`
- `cargo test -p codex-tui` (если затронут UI/снапшоты; обязательно на этапах 3-5 при любых касаниях TUI)
- `just write-app-server-schema` при изменении протокольных схем

## Риски и меры снижения

- **Дрейф версии** → синхронизация `workspace.version` с базой.
- **Схемы/fixtures** → только генерация, не вручную.
- **Конфликты в core/tui** → решать в пользу upstream, fork‑слой как патч.

## Готовность (Done criteria)

- План зафиксирован и согласован.
- Fork‑ядро описано и защищено от потерь.
- Процесс интеграции upstream формализован.
