# SubAgentsWindow (SAW): окно sub-agents и цикл `Ctrl+T`

## Кодовое имя

`SubAgentsWindow` (сокращение: `SAW`) — кодовое имя fork-функции, которая добавляет окно `/ A G E N T S /` (summary по sub-agents) и включает его в цикл `Ctrl+T`.

## Паспорт feature

- Кодовое имя: `SubAgentsWindow (SAW)`.
- Статус: `implemented` (есть рабочая реализация и локальные прогоны; известные legacy snapshot-предупреждения не относятся к логике SAW).
- Goal: единый цикл `Ctrl+T` для `Transcript -> SubAgentsWindow (SAW) -> Close` с корректным возвратом терминала в inline-screen.
- Scope in: `codex-rs/tui` (рендер AGENTS overlay, сбор summary из событий, маршрутизация `Ctrl+T`).
- Scope out: внешние API для SAW, изменения security/policy, отдельные RPC под overlay.
- API impact: отсутствует.
- Security/policy impact: отсутствует.
- Config/flags: новых флагов для SAW не добавлялось.
- Базовый пакет исследований релиза: `docs/research/0.99/README.md`.
- Связанные исследования:
  - `docs/research/0.99/BASELINE_AND_BRANCH_STATE.md`
  - `docs/research/0.99/MIN_DIFF_IMPLEMENTATION_STRATEGY.md`
  - `docs/research/0.99/QUALITY_GATES_AND_CHECKS.md`
  - `docs/research/0.99/TUI_WINDOWS_AND_OVERLAYS.md`

## Назначение

`SubAgentsWindow (SAW)` вводит единый пользовательский цикл для быстрого просмотра контекста сессии и состояния заспавненных агентов без выхода из текущей TUI-сессии.

Цели:

- Ускорить переключение между историей диалога и обзором агентной активности.
- Уменьшить количество разных хоткеев/режимов для операторов, работающих с форком и мульти-агентными сценариями.
- Сохранить минимальный diff и не менять публичные API/протоколы.

## Пользовательское поведение

Нажатие `Ctrl+T` образует цикл из трёх состояний:

1. `None -> / T R A N S C R I P T /`
2. `/ T R A N S C R I P T / -> / A G E N T S /` (`SAW`)
3. `/ A G E N T S /` (`SAW`) `-> None`

При переходе `3` терминал возвращается в обычный режим корректно: без повторного входа в alt-screen и без визуального эффекта "перемотки" окна.

Что видит пользователь в `/ A G E N T S /`:

- дерево sub-agent тредов (иерархия по `parent_thread_id` с отступами depth);
- для каждого агента первая строка:
  - индикатор активности (спиннер для активных, `•` для неактивных),
  - activity label (`Working`/`Completed`/`Errored`/`Shutdown`/`Not found`),
  - elapsed рядом со статусом: `(activity_elapsed)`,
  - `Role`, `Model`, `Reasoning`, `Spawned`;
- для каждого агента вторая строка:
  - `thread_id`,
  - `Context left` (процент или `—`);
- для каждого агента третья строка:
  - `Last tool: ...`,
  - разделитель `|` (dim),
  - `status_detail` (dim) при наличии;
- для каждого агента четвёртая строка (опционально):
  - `Last tool detail` (строка `└ ...`) при наличии.

`status_detail` в активном состоянии:

- приоритетно берётся из persisted reasoning summary;
- reasoning summary сохраняется до следующего summary или пустого `summary_text`;
- если summary нет, используется fallback на последний `BackgroundEvent`.

`Plan` секция под инструментами:

- если модель присылает `update_plan`, под tool-блоком рендерится `Plan:`;
- при наличии explanation добавляется `Note: ...`;
- шаги плана выводятся маркерами:
  - `pending -> [ ]`
  - `in_progress -> [>]`
  - `completed -> [x]`

Динамика обновления:

- окно `/ A G E N T S /` обновляется в реальном времени, пока открыто;
- при включённых анимациях — с каденсом `TARGET_FRAME_INTERVAL`;
- при выключённых анимациях — с каденсом `1s`.

Цветовая семантика:

- `running` — рабочий статус, `completed` — green, `errored`/`not found` — red, `pending init`/`shutdown` — dim;
- `Context left`: `<15%` — red, `<30%` — magenta, иначе default; суффикс `left` — dim;
- для активных label может отображаться с shimmer (если анимации включены).

Если заспавненных sub-агентов нет, отображается `No active sub-agent threads.`.

## Scope и ограничения

- Scope in: только `codex-rs/tui` и связанные fork-расширения collab/summary.
- API impact: отсутствует.
- Security/policy impact: отсутствует.
- Конфиги и флаги SAW: не добавлялись.
- Поведение несвязанных overlay (например, diff/static overlays) не меняется.

## Логика реализации

### 1) Маркер типа overlay и распознавание состояния

Файл: `codex-rs/tui/src/pager_overlay.rs`

Добавлено:

- константа заголовка агентного окна: `AGENTS_OVERLAY_TITLE = "A G E N T S"`;
- методы:
  - `Overlay::is_transcript()`
  - `Overlay::is_agents()`

Это нужно, чтобы роутинг `Ctrl+T` принимал решение по текущему типу overlay без изменения архитектуры `Overlay::{Transcript, Static}`.

### 2) Локальная state-machine на уровне `App`

Файл: `codex-rs/tui/src/app.rs`

Добавлено:

- enum `CtrlTOverlayAction`:
  - `OpenTranscript`
  - `OpenAgents`
  - `CloseAgents`
  - `None`
- вычисление шага цикла: `ctrl_t_overlay_action()`
- единая точка входа: `handle_ctrl_t_key()`

Решение по следующему шагу:

- нет overlay -> открыть transcript;
- открыт transcript -> открыть agents;
- открыт agents -> закрыть overlay (выход в обычный режим);
- открыт другой static overlay -> не вмешиваться (`None`).

### 3) Построение содержимого `/ A G E N T S /`

Файлы: `codex-rs/tui/src/app.rs`, `codex-rs/tui/src/agents_overlay.rs`

Добавлены/изменены:

- `available_thread_ids()` — единый метод prune/sort для `thread_event_channels`;
- `open_agents_overlay()` — сбор summary-данных;
- `refresh_agents_overlay_if_active()` — live refresh;
- выделенный модуль `agents_overlay.rs` для дерева и форматирования строк.

Сбор данных:

1. Актуальные `thread_id` через `available_thread_ids()`.
2. Для каждого треда:
   - snapshot из `ThreadEventStore`;
   - `config_snapshot` только для `SessionSource::SubAgent(_)`;
   - иерархия по `SubAgentSource::ThreadSpawn { parent_thread_id, .. }`;
   - `Context left` из последнего `EventMsg::TokenCount`;
   - status/`status_changed_at` из `ThreadEventStore`;
   - `last_tool`/`last_tool_detail` по последним tool-ивентам;
   - `status_detail` (reasoning summary -> background fallback);
   - `plan_update` из `EventMsg::PlanUpdate` с нормализацией.
3. Рендер строк AGENTS overlay.

### 3.1) Корректный возврат терминала (root cause и fix)

Файлы: `codex-rs/tui/src/app.rs`, `codex-rs/tui/src/tui.rs`

Root cause ранней реализации SAW:

- при переходе `/ T R A N S C R I P T / -> / A G E N T S /` вызывался повторный `tui.enter_alt_screen()`;
- повторный вход перезаписывал `alt_saved_viewport`;
- на `Close` восстанавливался неверный viewport (эффект "перемотки").

Fix:

- guard в `open_agents_overlay()`:
  - `if !tui.is_alt_screen_active() { let _ = tui.enter_alt_screen(); }`

Итог:

- сохранено upstream-поведение закрытия transcript overlay;
- устранён побочный эффект SAW-сценария.

### 4) Интеграция с потоком событий overlay

Файл: `codex-rs/tui/src/app_backtrack.rs`

Добавлен ранний перехват `Ctrl+T` в `handle_backtrack_overlay_event()`:

- если активен transcript/agents overlay, вызвать `handle_ctrl_t_key()` и завершить обработку.

Это критично для 2-го и 3-го шага цикла.

### 5) Замена старого прямого открытия transcript

Файл: `codex-rs/tui/src/app.rs`

В `handle_key_event()` ветка `Ctrl+T` вызывает `handle_ctrl_t_key()` вместо прямого `Overlay::new_transcript(...)`.

## Совместимость и минимальный diff

Принципы, сохранённые в реализации:

- не добавлялись новые публичные API SAW;
- не изменялись wire-структуры ради SAW-рендера;
- не менялась базовая модель `Overlay`;
- не менялось поведение несвязанных popup/static overlays;
- переиспользована текущая инфраструктура `thread_event_channels` и `ThreadManager`.

## Верификация

Локальные проверки по SAW:

- `cargo test -p codex-tui` — проходил (с legacy snapshot-format warnings, не относящимися к SAW-логике).
- `./scripts/full-test-report.sh` — используется как полный gate-скрипт для workspace.

## Операционные заметки

- `SAW` дополняет `/agent` picker, но не заменяет его.
- `/ A G E N T S /` — обзорное окно состояния; выбор агента для фокуса по-прежнему делается через existing picker.
- Summary-рендер дерева вынесен в `codex-rs/tui/src/agents_overlay.rs`.

## Файлы реализации

- `codex-rs/tui/src/pager_overlay.rs`
- `codex-rs/tui/src/app.rs`
- `codex-rs/tui/src/app_backtrack.rs`
- `codex-rs/tui/src/agents_overlay.rs`

## Changelog

- 2026-02-13: добавлены reasoning-summary persistence и отображение в SAW.
- 2026-02-13: добавлен `Plan` блок под tool summary (`[ ]/[>]/[x]`, `Note` при explanation).
- 2026-02-13: синхронизирован контракт строк AGENTS overlay (status+elapsed в первой строке, `Last tool | status_detail` в третьей).
