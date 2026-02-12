# TUI окна и overlay: поведение и инварианты

## Контекст

Этот документ фиксирует поведение окон TUI для сценариев, где фича затрагивает hotkeys/overlay и alt-screen lifecycle.

Цель: не допускать UX-регрессий при переключении окон и держать минимальный diff относительно upstream-логики.

## 1) Основные окна и состояния

Для цикла `SubAgentsWindow (SAW)` (`Ctrl+T`) используются состояния:

1. `None` (обычный inline viewport)
2. `Overlay::Transcript` (`/ T R A N S C R I P T /`)
3. `Overlay::Static` с заголовком `A G E N T S` (`/ A G E N T S /`)

Ключевые точки:

- маршрутизация шага цикла: `codex-rs/tui/src/app.rs` (`ctrl_t_overlay_action`, `handle_ctrl_t_key`);
- распознавание overlay-типа: `codex-rs/tui/src/pager_overlay.rs` (`Overlay::is_transcript`, `Overlay::is_agents`).

## 2) Роутинг событий окна

Нужно учитывать два пути обработки клавиш:

1. обычный путь: `handle_key_event` в `codex-rs/tui/src/app.rs`;
2. путь активного overlay/backtrack: `handle_backtrack_overlay_event` в `codex-rs/tui/src/app_backtrack.rs`.

Если overlay открыт, переходы по `Ctrl+T` должны быть доступны и в overlay-пути, иначе цикл застрянет на первом шаге.

## 3) Alt-screen lifecycle (критичный инвариант)

Терминальные примитивы:

- `enter_alt_screen`: `codex-rs/tui/src/tui.rs`
- `leave_alt_screen`: `codex-rs/tui/src/tui.rs`

Инвариант:

- внутри одного overlay-сеанса нельзя делать повторный `enter_alt_screen()` без проверки состояния;
- перед входом использовать guard `if !tui.is_alt_screen_active()`.

Причина: `enter_alt_screen()` сохраняет `alt_saved_viewport`. Повторный вход в уже активном alt-screen перезаписывает точку восстановления viewport.

## 4) Буфер history при открытом overlay

При `overlay.is_some()` новые history lines не вставляются сразу в terminal scrollback, а буферизуются в `deferred_history_lines`.

- буферизация: `codex-rs/tui/src/app.rs` (ветка `AppEvent::InsertHistoryCell`);
- flush на закрытии overlay: `close_transcript_overlay` в `codex-rs/tui/src/app_backtrack.rs`.

Это нативная логика transcript overlay и она должна сохраняться, если задача явно не требует другого UX.

## 5) Зафиксированная регрессия и фактический fix

Наблюдавшийся симптом:

- при цикле `Ctrl+T`: `Transcript -> Agents -> close`, окно терминала визуально "перематывалось".

Подтвержденный root cause:

- в `open_agents_overlay()` был безусловный `tui.enter_alt_screen()`;
- из-за повторного входа в alt-screen портился restore-point viewport;
- при `leave_alt_screen()` терминал возвращался не в исходное состояние inline viewport.

Финальный минимальный fix:

- в `open_agents_overlay()` использовать guard:
  - `if !tui.is_alt_screen_active() { let _ = tui.enter_alt_screen(); }`
- нативный `close_transcript_overlay()` не менять.

## 6) Практический checklist для будущих TUI-фич

Перед merge любой фичи с новыми окнами/overlay:

1. Проверить, кто владеет переходами (`handle_key_event` и overlay-routing).
2. Проверить, что `enter_alt_screen()` не вызывается повторно в активном alt-screen.
3. Проверить поведение закрытия окна на третьем/последнем шаге хоткея.
4. Убедиться, что semantics `deferred_history_lines` и flush не сломаны.
5. Зафиксировать поведение в `docs/features/<feature>.md` и в release research пакете.

## 7) Что считать признаком корректной работы

1. После закрытия overlay терминал возвращается в ожидаемый inline viewport.
2. Нет визуального "rewind/перемотки" экрана в сценарии закрытия окна.
3. Нативный transcript UX (включая flush отложенной history) не регрессирует.
