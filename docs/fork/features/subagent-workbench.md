# Subagent Workbench / agents overlay

## Feature passport

- Code name: `subagent-workbench`
- Status: переносимая fork-возможность; реализовывалась в нескольких release-ветках.
- Goal: дать пользователю TUI-поверхность для наблюдения и переключения между sub-agents, вместо работы только через текстовые tool calls.
- Scope in: SAW/agents overlay, summary текущих agents, preview prompt/context, connect/switch action.
- Scope out: сама модель исполнения sub-agent задач; она описана в соседних feature docs.

## Как работает для пользователя

Пользователь видит TUI-окно со списком sub-agents, их статусами и контекстом. Польза: легче управлять параллельной работой и понимать, кто чем занят.

## Branches and commits

| Branch | Baseline | Commit | Date | Subject | Роль |
| --- | --- | --- | --- | --- | --- |
| `fork/saw` | `rust-v0.99.0` | `4eab565e69` | 2026-02-12 | `feat(tui): SubAgentsWindow (SAW) Ctrl+T agents summary (baseline 0.99)` | Первый SAW overlay в TUI. |
| `fork/saw` | `rust-v0.99.0` | `084d3d0f0f` | 2026-02-12 | `docs(diff): add SAW.patch snapshot (baseline 0.99)` | Evidence/snapshot для SAW diff. |
| `fork/saw` | `rust-v0.99.0` | `a15d4adc2c` | 2026-02-13 | `[SA][SAW] finalize sub-agent templates, guards, and UI integration` | Финальная интеграция SAW с templates/guards/UI. |
| `fork/colab-agents`, `fork/multi-agent` | `rust-v0.98.0` audit lineage | `91669ef30c` | 2026-02-05 | `feat: push sub-agent status and add Agents overlay` | Runtime начинает отправлять sub-agent status в parent TUI, где появляется Agents overlay. |
| `fork/101` | `rust-v0.101.0` | `122ea4f1bf` | 2026-02-14 | `feat(saw): stabilize fork contracts and AGENTS overlay for upstream 0.101.0` | Перенос SAW/AGENTS overlay на 0.101. |
| `fork/101` | `rust-v0.101.0` | `3a115403c3` | 2026-02-14 | `tui(saw): show prompt preview in AGENTS overlay and reorder context line` | Улучшение preview/context в overlay. |
| `fork/101` | `rust-v0.101.0` | `9f82deac0f` | 2026-02-15 | `feat(sa): complete Sub-Agents fork contract (templates, thread_note, runtime listing, TUI/SAW polish)` | Сведение SAW с sub-agent contract. |
| `fork/106` | `rust-v0.106.0` | `62ecf70c26` | 2026-03-01 | `Add SAW AGENTS overlay and update feature docs` | Повторный перенос overlay. |
| `fork/106` | `rust-v0.106.0` | `f9f4e52d38` | 2026-03-02 | `feat(saw): finalize interactive agents window and align feature docs` | Финализация interactive agents window. |
| `fork/107` | `rust-v0.107.0` | `50603a166b` | 2026-03-02 | `feat(saw): finalize interactive agents window and align feature docs` | Перенос на ветку 0.107. |
| `fork/107` | `rust-v0.107.0` | `39400fe84c` | 2026-03-06 | `feat(saw): add connect action for agent threads` | Добавлен connect action для agent threads. |
| `fork/118` | `rust-v0.118.0` | `722b52cdf7` | 2026-04-04 | `Add agents overlay and restore resume thread notes` | Перенос overlay и связка с thread notes. |
| `fork/130` | `rust-v0.130.0` | `34d3de7362` | 2026-05-15 | `Add agents overlay TUI feature` | Новый перенос agents overlay в более позднюю TUI архитектуру. |
| `chrome_plugin` | `rust-v0.130.0` lineage | `34d3de7362` | 2026-05-15 | `Add agents overlay TUI feature` | Та же возможность присутствует в chrome-plugin lineage. |

## Implementation notes

Затрагивались TUI modules для overlay/window, routing/switching state, app event dispatch и docs. В поздних переносах возможность разнесена между `agents_overlay`, app state/routing и feature/project docs.

## Native coverage in rust-v0.140.0

Status: `partial`. Native release уже имеет `/agent`, `/subagents`, `AppEvent::OpenAgentPicker`, `AppEvent::SelectAgentThread`, agent ordering/switch labels, `agent_status_feed` и protocol event `SubAgentActivityEvent`. Не хватает исторического SAW/Ctrl+T workbench с полным agents summary, prompt/context preview и explicit connect/switch UX из fork docs.

## Porting/current-state notes

В `fork/140` нет закоммиченного переноса этой возможности поверх `fork/140-upstream`. Если переносить дальше, нужно проверять current native multi-agent v2 surfaces и не дублировать upstream agents UI, если он уже появился нативно.
