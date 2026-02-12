# SubAgentsWindow (SAW): окно sub-agents и цикл `Ctrl+T`

## Кодовое имя

`SubAgentsWindow` (сокращение: `SAW`) — кодовое имя fork-функции, которая добавляет окно `/ A G E N T S /` (summary по sub-agents) и включает его в цикл `Ctrl+T`.

## Паспорт feature

- Кодовое имя: `SubAgentsWindow (SAW)`.
- Статус: `implemented` (локально проверено; есть известные version-related падения snapshot/user-agent тестов, не связанные с логикой SAW).
- Goal: единый цикл `Ctrl+T` для `Transcript -> SubAgentsWindow (SAW) -> Close` с корректным возвратом терминала в inline-screen.
- Scope in: `codex-rs/tui`.
- Scope out: API/протоколы/безопасность/конфиги.
- API impact: отсутствует.
- Security/policy impact: отсутствует.
- Config/flags: не добавлялись.
- Базовый пакет исследований релиза: `docs/research/0.99/README.md`.
- Связанные исследования:
  - `docs/research/0.99/BASELINE_AND_BRANCH_STATE.md`
  - `docs/research/0.99/MIN_DIFF_IMPLEMENTATION_STRATEGY.md`
  - `docs/research/0.99/QUALITY_GATES_AND_CHECKS.md`
  - `docs/research/0.99/TUI_WINDOWS_AND_OVERLAYS.md`

## Назначение

`SubAgentsWindow (SAW)` вводит единый пользовательский цикл для быстрого просмотра контекста сессии и состояния заспавненных агентов без выхода из текущей TUI-сессии.

Цель:

- Ускорить переключение между историей диалога и обзором агентной активности.
- Уменьшить количество разных хоткеев/режимов для операторов, работающих с форком и мульти-агентными сценариями.
- Сохранить минимальный diff и не менять API/протоколы.

## Пользовательское поведение

Нажатие `Ctrl+T` теперь образует цикл из трёх состояний:

1. `None -> / T R A N S C R I P T /`
2. `/ T R A N S C R I P T / -> / A G E N T S /` (`SAW`)
3. `/ A G E N T S /` (`SAW`) `-> None`

При переходе `3` терминал возвращается в обычный режим корректно: без повторного входа в alt-screen и без визуального эффекта "перемотки" окна.

Что видит пользователь в `/ A G E N T S /`:

- дерево sub-agent тредов (иерархия по `parent_thread_id` с отступами depth);
- для каждого агента первая строка:
  - `Role`;
  - `Model`;
  - `Reasoning`;
  - `Spawned` (elapsed с момента spawn в формате `fmt_elapsed_compact`);
- для каждого агента вторая строка:
  - `thread_id`;
  - статус (`pending init`, `running`, `completed`, `errored`, `shutdown`, `not found`);
  - `Context left` (процент или `—`).
- для каждого агента третья строка:
  - mini status-indicator в стиле `Working`-строки;
  - спиннер + activity label (`Working`/`Completed`/`Errored`/`Shutdown`/`Not found`);
  - elapsed с момента смены статуса в формате `fmt_elapsed_compact` (например `1m 04s`, `1h 00m 00s`).
- для каждого агента четвертая строка:
  - `Last tool` (например `shell`, `apply_patch`, `web_search`, `server.tool`, `view_image` или `—`).
  - `Last tool detail` (опционально): следующей строкой показывается короткая “подпись” последнего вызова инструмента.
    - `shell`: конкретная команда (как в exec overlay; `strip_bash_lc_and_escape`).
    - `apply_patch`: агрегат A/M/D + кол-во файлов + первые пути.
    - `web_search`: `search/open/find` с нормализованным query/url (без query/fragment для url).
    - `server.tool` (MCP) и `DynamicToolCallRequest`: аргументы в компактном JSON-формате с простым редактированием секретов по ключам (например `token`, `authorization`) и агрессивным truncation.
    - `view_image`: путь (в виде `~/...`, если под `$HOME`).
  - для остальных инструментов сейчас выводится только лейбл инструмента (без дополнительной строки с деталями).
- окно `/ A G E N T S /` обновляется в реальном времени, пока открыто:
  - при включённых анимациях — с каденсом `TARGET_FRAME_INTERVAL`;
  - при выключённых анимациях — с каденсом `1s`.
- цветовая семантика:
  - `running` — cyan, `completed` — green, `errored`/`not found` — red, `pending init`/`shutdown` — dim;
  - `Context left`: `<15%` — red, `<30%` — magenta, иначе default; суффикс `left` — dim.
  - activity label в третьей строке: `completed` — green, `errored`/`not found` — red, `pending init`/`shutdown` — dim; для активных (`pending init`/`running`) используется спиннер и shimmer-стиль (если анимации включены).

Если заспавненных sub-агентов нет, отображается `No active sub-agent threads.`.

## Scope и ограничения

- Scope in: только `codex-rs/tui`.
- API impact: отсутствует.
- Security/policy impact: отсутствует.
- Конфиги и флаги: не добавлялись.
- Поведение существующих несвязанных overlay (например, diff/static overlays) не меняется.

## Логика реализации

### 1) Маркер типа overlay и распознавание состояния

Файл: `codex-rs/tui/src/pager_overlay.rs`

Добавлено:

- константа заголовка агентного окна: `AGENTS_OVERLAY_TITLE = "A G E N T S"`;
- методы:
  - `Overlay::is_transcript()`
  - `Overlay::is_agents()`

Это нужно, чтобы роутинг `Ctrl+T` принимал решение по текущему типу overlay без изменения существующей архитектуры `Overlay::{Transcript, Static}`.

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
- открыт любой другой static overlay -> не вмешиваться (`None`).

### 3) Построение содержимого `/ A G E N T S /`

Файл: `codex-rs/tui/src/app.rs`

Добавлены:

- `available_thread_ids()` — единый метод prune/sort для `thread_event_channels`;
- `open_agents_overlay()` — сбор данных для summary;
- вынесенный модуль `codex-rs/tui/src/agents_overlay.rs` — построение дерева и форматирование строк summary.

Алгоритм `open_agents_overlay()`:

1. Получить актуальные `thread_id` через `available_thread_ids()`.
2. Для каждого треда:
   - взять snapshot из `ThreadEventStore`;
   - взять `config_snapshot` треда и оставить только `SessionSource::SubAgent(_)`;
   - построить иерархию по `SubAgentSource::ThreadSpawn { parent_thread_id, .. }`;
   - вычислить `Context left` из последнего `EventMsg::TokenCount` (`last_token_usage` + `model_context_window`);
   - взять статус из последнего состояния `ThreadEventStore` и время его смены.
3. Сформировать 4-5 строк на агента:
   - строка 1: `Role/Model/Reasoning/Spawned`;
   - строка 2: `thread_id/Status/Context left`;
   - строка 3: `Working`-совместимый mini status-indicator + elapsed.
   - строка 4: `Last tool`.
   - строка 5 (опционально): `Last tool detail` (если доступно для данного события).
4. Сформировать список строк для `Overlay::new_static_with_lines(..., "A G E N T S")`.
5. Сбросить backtrack-preview состояние и показать overlay в alt-screen, но вызывать `enter_alt_screen()` только если alt-screen ещё не активен (`if !tui.is_alt_screen_active()`).
6. Пока overlay активен, на каждом `TuiEvent::Draw` выполняется `refresh_agents_overlay_if_active()`:
   - пересчёт строк summary;
   - обновление содержимого static-overlay без его закрытия;
   - планирование следующего redraw по каденсу анимаций.

### 3.1) Корректный возврат терминала (root cause и fix)

Файлы: `codex-rs/tui/src/app.rs`, `codex-rs/tui/src/tui.rs`

Root cause в ранней реализации SAW:

- на шаге `/ T R A N S C R I P T / -> / A G E N T S /` выполнялся повторный `tui.enter_alt_screen()`;
- `enter_alt_screen()` сохраняет `alt_saved_viewport`, и повторный вызов в уже активном alt-screen перезаписывал точку восстановления viewport;
- на третьем `Ctrl+T` (`/ A G E N T S / -> None`) нативный `leave_alt_screen()` восстанавливал уже не исходный inline viewport, что визуально проявлялось как "перемотка" окна терминала.

Финальный fix:

- в `open_agents_overlay()` добавлен guard:
  - `if !tui.is_alt_screen_active() { let _ = tui.enter_alt_screen(); }`
- нативная логика `close_transcript_overlay()` не менялась.

Итог:

- сохранено upstream-поведение закрытия transcript overlay;
- устранён побочный эффект SAW-сценария при цикле `Ctrl+T`.

### 4) Интеграция с существующим потоком событий overlay

Проблема: когда overlay уже открыт, события идут не через обычный `handle_key_event`, а через `handle_backtrack_overlay_event`.

Файл: `codex-rs/tui/src/app_backtrack.rs`

Добавлен ранний перехват `Ctrl+T` в `handle_backtrack_overlay_event()`:

- если активен transcript/agents overlay, вызвать `handle_ctrl_t_key()` и завершить обработку.

Это критично для 2-го и 3-го шага цикла.

### 5) Замена старого прямого открытия transcript

Файл: `codex-rs/tui/src/app.rs`

В `handle_key_event()` ветка `Ctrl+T` теперь вызывает `handle_ctrl_t_key()` вместо прямого `Overlay::new_transcript(...)`.

## Совместимость и минимальный diff

Принципы, сохранённые в реализации:

- не добавлялись новые публичные API;
- не изменялись wire-структуры и протоколы;
- не менялась базовая модель `Overlay`;
- не менялось поведение несвязанных popup/static overlays;
- переиспользована текущая инфраструктура `thread_event_channels` и `ThreadManager`.

## Верификация (актуальное состояние на 2026-02-12)

Файл: `codex-rs/tui/src/app.rs`

Добавлен тест:

- `ctrl_t_overlay_action_cycles_transcript_and_agents_overlays`

Он проверяет state-machine переходы:

- `None -> OpenTranscript`
- `Transcript -> OpenAgents`
- `Agents -> CloseAgents`
- `Other static overlay -> None`

Ручная проверка:

```bash
cd codex-rs && cargo build && cargo run --bin codex
```

Далее в TUI: `Ctrl+T` (Transcript) -> `Ctrl+T` (SAW) -> `Ctrl+T` (Close).

Результаты обязательных проверок:

- `just fmt` — passed.
- `just fix -p codex-tui` — passed.
- `cargo test -p codex-tui` — failed только на известных snapshot-тестах статуса (расхождение версии `v0.0.0` vs `v0.99.0`).
- `cargo test --all-features` — failed только на известном `app-server` тесте `user_agent` (расхождение версии `0.0.0` vs `0.99.0`).

## Операционные заметки

- `SAW` дополняет `/agent` picker, но не заменяет его.
- `/ A G E N T S /` — обзорное окно состояния; выбор агента для фокуса по-прежнему делается через существующий picker.
- Summary-рендер дерева и двухстрочного представления вынесен в `codex-rs/tui/src/agents_overlay.rs`.

## Файлы реализации

- `codex-rs/tui/src/pager_overlay.rs`
- `codex-rs/tui/src/app.rs`
- `codex-rs/tui/src/app_backtrack.rs`
- `codex-rs/tui/src/agents_overlay.rs`

## Резюме

`SubAgentsWindow (SAW)` стандартизирует управление двумя ключевыми обзорными окнами TUI по одному хоткею `Ctrl+T`, добавляет быстрый доступ к статусу spawned-агентов и делает это без изменения API и без широкого рефакторинга.

## Журнал изменений документа

- `2026-02-12`: добавлен паспорт feature, ссылки на research-пакет `0.99`, матрица верификации и фиксация known-failures; добавлена секция ручной проверки (`cargo build && cargo run --bin codex`); обновлена операционная часть под текущее вынесение рендера в `agents_overlay.rs`; переименовано кодовое имя фичи в `SubAgentsWindow (SAW)` (fork-маркеры в коде: `SAW COMMIT OPEN/CLOSE`); добавлена 4-я строка `Last tool` и 5-я опциональная строка `Last tool detail` (универсально для `shell/apply_patch/web_search/mcp/dynamic/view_image`, c truncation/redaction).
