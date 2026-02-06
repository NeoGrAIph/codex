# Native-first audit: rust-v0.98.0 vs fork/colab-agents

> Owner: <team/owner> | Scope: fork/colab-agents upgrade audit | Audience: devs
> Status: draft | Last reviewed: 2026-02-06 | Related: `docs/fork/release-audit/0.98/REPORT.md`

## Цель

- Зафиксировать отличия между upstream `rust-v0.98.0` (commit `82464689c…`) и форком `fork/colab-agents`
  (baseline commit `33aa00c7f…`).
- Сделать **native-first аудит**: найти места, где форк добавляет кастом, но в upstream уже есть равный/лучший
  нативный аналог (или появился более простой путь), и сформировать рекомендации по дефоркизации.

## Артефакты diff (источник правды)

- Baseline: `docs/fork/release-audit/0.98/DIFF_BASELINE.md`
- Index: `docs/fork/release-audit/0.98/DIFF_INDEX.md`
- Per-file patches: `docs/fork/release-audit/0.98/diff/*.patch`

Примечание: ветка `fork/colab-agents` двигается (в т.ч. коммиты аудита), но артефакты pinned к baseline commit
`33aa00c7f…`.

## Инвентарь (triage)

Категоризация берётся из `DIFF_INDEX.md` (эвристика для маршрутизации review; при необходимости можно уточнять).

| Category | Count | Notes |
|---|---:|---|
| core | 171 | По умолчанию: `codex-rs/**` вне явных special-case категорий |
| generated | 71 | Schema/config/lock и др. артефакты, не аудитим построчно |
| tui | 65 | `codex-rs/tui/**` и снапшоты |
| vendor | 50 | `codex-rs/vendor/**` (не аудитим построчно) |
| docs | 15 | `docs/**` + `AGENTS.md` |
| protocol | 8 | `codex-rs/protocol/**` + `codex-rs/app-server-protocol/src/**` (schema исключена) |
| ci | 7 | `.github/**`, `scripts/**` |
| linux-sandbox | 3 | `codex-rs/linux-sandbox/**` |

### Status breakdown

| Status | Count |
|---:|---:|
| M | 277 |
| D | 82 |
| A | 26 |
| R | 5 |

## Формат карточек

Определения:

- **Native-first assessment**: есть ли в upstream нативный эквивалент/замена fork-кастому (или более правильный
  API/паттерн), и насколько он покрывает fork use-case.
- **Priority**:
  - **P0**: блокер апгрейда/безопасности/данных или высокий риск регрессий; исправлять в первую очередь.
  - **P1**: важно для поддерживаемости/UX/снижения диффа, но не блокер.
  - **P2**: nice-to-have, можно отложить.

Правила:

- Не вставлять большие диффы сюда; ссылаться на `.patch` из `docs/fork/release-audit/0.98/diff/…`.
- `generated/vendor` не аудитим построчно: фиксируем только риски и правила регенерации.

## Карточки (native-first)

| Card | Area | Files (patch) | Fork delta (что изменили) | Native-first assessment | Recommendation | Priority | Risks/Notes |
|---|---|---|---|---|---|---|---|
| NF-000 | meta | _TBD_ | _TBD_ | _TBD_ | _TBD_ | P2 | Placeholder row; заменяется реальными карточками |

