# Release audit report: 0.98 (rust-v0.98.0 vs fork/colab-agents)

> Owner: <team/owner> | Scope: fork/colab-agents upgrade audit | Audience: devs
> Status: draft | Last reviewed: 2026-02-06 | Related: `docs/fork/native-first-audit.md`

## Executive summary

_TBD after zone triage._

## Baseline and scope

- Base: `rust-v0.98.0` (commit `82464689c…`)
- Fork baseline: `fork/colab-agents` (commit `33aa00c7f…`)
- Diff artifacts:
  - `docs/fork/release-audit/0.98/DIFF_BASELINE.md`
  - `docs/fork/release-audit/0.98/DIFF_INDEX.md`
  - `docs/fork/release-audit/0.98/diff/*.patch`
- Native-first cards: `docs/fork/native-first-audit.md`

## Key risks and regressions

### P0 (blockers)

_TBD_

### P1 (should-do)

_TBD_

### P2 (nice-to-have)

_TBD_

## Recommendations (native-first / defork)

_TBD_

## Manual verification plan (TUI)

Цель: минимальный ручной прогон ключевых сценариев до/после изменений, особенно вокруг multi-agent UX и naming sync.

### Setup

1. Build fork binary (локально).
2. Launch TUI in a clean workspace.
3. Ensure config/agents registry seeding runs (если применимо).

### Smoke flows

1. Basic chat: отправка сообщений, отмена/прерывание, history rendering.
2. Tool calls: shell, apply_patch (если доступно), file writes/reads.
3. Approval UX: request/deny/approve, history отображение решений.

### Multi-agent / collab flows

1. Toggle/enter overlays (Ctrl+N / Agents summary/details) при включённых флагах.
2. `spawn_agent`: успешный spawn, обработка ошибок (неизвестный agent/model), закрытие потоков.
3. Shutdown orchestration: корректное закрытие потомков, отсутствие “подвисших” overlay/alt-screen.

### Skills / registry flows

1. List agents: проверить перечень профилей, варианты `agent_names`, отображение `reasoning_effort`.
2. Remote skills (если включено): list/read/write, обработка сетевых ошибок.

### After naming sync (повторно)

После синхронизации нейминга/label-ов (и/или hotkeys) повторить разделы:

- Smoke flows
- Multi-agent / collab flows

Фокус: корректность названий/подсказок в UI, стабильность hotkeys, отсутствие регрессий в overlay lifecycle.

## Appendix

- Карточки: `docs/fork/native-first-audit.md`
- Индекс diff: `docs/fork/release-audit/0.98/DIFF_INDEX.md`

