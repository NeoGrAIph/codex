# Agent window UI snapshots TOP 3

Этот каталог хранит read-only visual snapshots окна агентов для будущего переноса. Предыдущие одиночные alias/source snapshots заменены на TOP 3: `agent-window-tui-snapshot.rs` был ошибочным Rust source artifact, а `agent-window-ui-snapshot.snap` дублировал текущий `top-1-selected-inspect.snap`.

## TOP 3

| Rank | Local artifact | Source ref / commit / path | Что показывает | Почему выбран | Польза для переноса |
| --- | --- | --- | --- | --- | --- |
| 1 | `top-1-selected-inspect.snap` | `fork/107` / `39400fe84c79b18bd0318e5e725b24adea142ce0` / `codex-rs/tui/src/snapshots/codex_tui__agents_overlay__tests__agents_overlay_selected_with_inspect_enabled.snap` | Дерево агентов, выбранный worker, status/model/reasoning/context/workdir, last tool, plan, action menu и раскрытый `Inspect` block с request text. Blob: `d6f9d7c7641cd07ff569c1c64bc8e6ff44eb51e4`. | Самый информативный одиночный visual snapshot: одновременно видны summary, selection, actions и inspect-details. | Основной эталон будущего Agent window: показывает layout, hierarchy, selected state, action menu labels и форму detailed inspection. |
| 2 | `top-2-confirm-close.snap` | `fork/107` / `39400fe84c79b18bd0318e5e725b24adea142ce0` / `codex-rs/tui/src/snapshots/codex_tui__agents_overlay__tests__agents_overlay_confirm_close.snap` | Тот же multi-agent context, выбранный worker и destructive confirmation state `Confirm close` с thread id и `No`/`Yes`. Blob: `f8eb0dad991e6f25aa6929646d61e2237daea803`. | Лучший snapshot критического lifecycle состояния: закрытие агента и keyboard-selected confirmation. | Нужен для переноса close/shutdown UX, focus rules, confirm wording и проверки, что destructive action scoped to selected thread. |
| 3 | `top-3-running-plan.snap` | `fork/106` / `62ecf70c26` / `codex-rs/tui/src/snapshots/codex_tui__agents_overlay__tests__agents_overlay_running_with_plan.snap`; тот же blob есть в `fork/107`. | Базовый running summary без action menu: orchestrator + worker, status timing, model/reasoning, context left, last tool and plan block. Blob: `ae1d55b19e1b29699984b513f03a50c22b47978e`. | Лучший compact baseline snapshot: меньше интерактивного шума, зато хорошо видно основное информационное наполнение окна. | Нужен как regression baseline для пассивного overview режима: что видно до открытия action/inspect states. |

## Поиск по refs

Проверенные refs: `fork/saw`, `fork/101`, `fork/106`, `fork/107`, `fork/118`, `fork/130`, `chrome_plugin`.

Результат сравнения:

- `fork/saw` и `fork/101`: есть TUI/SAW код, но не найдено релевантных сохранённых visual `.snap` artifacts по заданным именам.
- `fork/106`: первые `agents_overlay` snapshots: empty state и running-with-plan, плюс `multi_agents` transcript snapshot.
- `fork/107`: самый полный набор visual states для full-screen overlay: empty, running, selected actions, inspect, selected inspect, confirm close.
- `fork/118`, `fork/130`, `chrome_plugin`: найдены `multi_agents` transcript snapshots (`collab_agent_transcript`, `collab_resume_interrupted`), но они показывают transcript/chat cells, а не окно `A G E N T S`; поэтому не вошли в TOP 3 для Agent window.

## Решение по UX reference

`top-1-selected-inspect.snap` принят как лучший пользовательский reference из найденных примеров. Он наиболее полно показывает полезную форму Agent window: дерево agents, выбранный agent, статусы, model/reasoning, context/workdir, last tool, plan, action menu и раскрытый `Inspect` block.

При переносе эту реализацию нельзя копировать буквально. Общая структура окна подходит как baseline, но `Actions` и `Inspect` требуют переработки:

- `Actions` стоит оформить как компактную, явно сфокусированную панель действий для выбранного agent; destructive action `Close` должен быть визуально отделён от обычных действий.
- `Inspect` block нужно сделать более сканируемым: разделить identity/runtime/task/last activity/context, поднять наверх информацию, нужную для решения “к какому agent перейти”, а вторичные поля опустить ниже.
- Выбранная строка agent и применяемые к ней actions должны быть визуально связаны, чтобы пользователь сразу понимал scope действия.
- Confirm/close flow лучше держать отдельным focused state, не смешивая destructive confirmation с обычным inspect/detail content.

## Как читать snapshots

Все TOP artifacts — обычные `insta` snapshots. Header:

```text
---
source: tui/src/agents_overlay.rs
expression: rendered
---
```

Всё после второго `---` — rendered terminal text. Быстрый просмотр UI без metadata:

```bash
awk 'BEGIN{n=0} /^---$/{n++; next} n>=2{print}' docs/fork/research/agent-window/top-1-selected-inspect.snap
awk 'BEGIN{n=0} /^---$/{n++; next} n>=2{print}' docs/fork/research/agent-window/top-2-confirm-close.snap
awk 'BEGIN{n=0} /^---$/{n++; next} n>=2{print}' docs/fork/research/agent-window/top-3-running-plan.snap
```

## Команды проверки

Команды, использованные для read-only поиска и сравнения:

```bash
for ref in fork/saw fork/101 fork/106 fork/107 fork/118 fork/130 chrome_plugin; do printf '\n== %s ==\n' "$ref"; git ls-tree -r "$ref" --name-only | rg -i '\.snap(\.new)?$' | rg -i 'agents_overlay|saw|SubAgentsWindow|agent_window|collab_agent|agent_transcript|multi_agents'; done
git log --oneline --decorate --all -- codex-rs/tui/src/snapshots/codex_tui__agents_overlay__tests__agents_overlay_running_with_plan.snap codex-rs/tui/src/snapshots/codex_tui__agents_overlay__tests__agents_overlay_confirm_close.snap codex-rs/tui/src/snapshots/codex_tui__agents_overlay__tests__agents_overlay_selected_with_inspect_enabled.snap
git show --name-status --oneline --decorate 39400fe84c -- codex-rs/tui/src/snapshots/codex_tui__agents_overlay__tests__agents_overlay_confirm_close.snap codex-rs/tui/src/snapshots/codex_tui__agents_overlay__tests__agents_overlay_selected_with_inspect_enabled.snap
git show --name-status --oneline --decorate 62ecf70c26 -- codex-rs/tui/src/snapshots/codex_tui__agents_overlay__tests__agents_overlay_running_with_plan.snap
git rev-parse fork/107:codex-rs/tui/src/snapshots/codex_tui__agents_overlay__tests__agents_overlay_selected_with_inspect_enabled.snap fork/107:codex-rs/tui/src/snapshots/codex_tui__agents_overlay__tests__agents_overlay_confirm_close.snap fork/106:codex-rs/tui/src/snapshots/codex_tui__agents_overlay__tests__agents_overlay_running_with_plan.snap
```

Local artifacts were copied from git objects:

```bash
git show fork/107:codex-rs/tui/src/snapshots/codex_tui__agents_overlay__tests__agents_overlay_selected_with_inspect_enabled.snap > docs/fork/research/agent-window/top-1-selected-inspect.snap
git show fork/107:codex-rs/tui/src/snapshots/codex_tui__agents_overlay__tests__agents_overlay_confirm_close.snap > docs/fork/research/agent-window/top-2-confirm-close.snap
git show fork/106:codex-rs/tui/src/snapshots/codex_tui__agents_overlay__tests__agents_overlay_running_with_plan.snap > docs/fork/research/agent-window/top-3-running-plan.snap
```
