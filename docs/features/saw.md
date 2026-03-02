# SubAgentsWindow (SAW)

## Feature passport

- Code name: `SubAgentsWindow` (`SAW`).
- Status: `implemented`.
- Scope in:
- `codex-rs/tui/src/app.rs`
- `codex-rs/tui/src/app_backtrack.rs`
- `codex-rs/tui/src/pager_overlay.rs`
- `codex-rs/tui/src/agents_overlay.rs`
- Scope out:
- app-server/protocol wire contracts;
- new MCP tools or API methods.

## Goal

Сделать единый цикл `Ctrl+T` для обзора истории и состояния sub-agent тредов:

1. `None -> / T R A N S C R I P T /`
2. `/ T R A N S C R I P T / -> / A G E N T S /`
3. `/ A G E N T S / -> None`

Без изменения публичных API и без регресса поведения других static overlays.

## User-visible behavior

`/ A G E N T S /` рендерит иерархию sub-agent тредов (по `parent_thread_id`) и по каждому треду:

- строка 1: статус (`Working/Completed/Errored/...`), elapsed, `Role`, `Model`, `Reasoning`, `Spawned`;
- строка 2: `Context left` + `thread_id`;
- строка 3: `Last tool` и `status_detail` в формате `Last tool ... | status_detail`;
- строка 4 (опционально): detail по последнему инструменту;
- секция `Plan:` (опционально) при наличии `update_plan`.

Особенности:

- Prompt preview не показывается;
- разделитель `|` в строке `Last tool | status_detail` имеет стабильную колонку (уменьшает "прыгание");
- `status_detail` всегда рендерится (fallback `—`);
- если sub-agent тредов нет, показывается `No active sub-agent threads.`

### Inspect mode

Inspect-блок внутри SAW показывает только данные, которых нет в основных строках карточки:

- `Directory`: директория треда (`config_snapshot.cwd`, отображается как `~/...` при возможности);
- `Request`: полный текст последнего `EventMsg::UserMessage.message` из event snapshot треда.

Правила рендера:

- `Request` не truncates;
- multiline-запрос рендерится построчно с wrap по ширине overlay;
- если непустой `UserMessage` в snapshot не найден, показывается fallback `—`.

### Interactive SAW

SAW работает в трёх режимах:

1. `Browse` (по умолчанию) — навигация по агентам.
2. `Actions` — меню действий для выбранного агента.
3. `Confirm` — подтверждение закрытия агента (`Yes/No`) внутри SAW.

Keymap в `Browse`:

- `↑/k` и `↓/j`: bounded-выбор агента без карусели;
  - верхняя позиция (`None`) находится над первым агентом и не показывает курсор;
  - `↓` из верхней позиции выбирает первого агента;
  - `↑` из первого агента возвращает в верхнюю позицию `None`;
  - `↓` на последнем агенте и `↑` на верхней позиции не меняют выбор;
- `Enter`: открыть меню `Actions` для выбранного агента (в `None` — no-op);
- `I`: переключить inspect для выбранного агента (в `None` — no-op);
- `PgUp/PgDn/Home/End/Space/Ctrl+f/Ctrl+b/Ctrl+u/Ctrl+d`: pager-scroll содержимого;
- `Ctrl+T`: закрыть SAW (и продолжить цикл `Ctrl+T`).
- нижняя строка hints для SAW: `enter to actions`, `I toggle inspect`, `ctrl+t to cycle`, `q to quit`.

Keymap в `Actions`:

- `↑/k` и `↓/j`: выбор пункта меню;
- `Enter`:
  - `Inspect` / `Disable Inspect` — включить/выключить inspect-блок для выбранного агента;
  - `Close` — открыть inline confirm `Yes/No` для закрытия выбранного агента;
- `Esc`: закрыть меню.

Keymap в `Confirm`:

- `↑/k` и `↓/j`: выбор `No/Yes`;
- `Enter`:
  - `Yes` — выполнить закрытие subtree выбранного агента;
  - `No` — закрыть confirm и вернуться в `Actions`;
- `Esc`: закрыть confirm и вернуться в `Actions`.

Ограничения v2:

- SAW предоставляет одно mutating-действие: `Close` (каскадный shutdown subtree);
- отдельный single-thread close без cascade не поддерживается.

## Data aggregation rules

SAW использует snapshot событий треда и runtime метаданные `ThreadEventStore`:

- `latest_status` + `status_changed_at`;
- `active_reasoning_summary`;
- `active_plan_update`;
- `created_at`.

Правила reset:

- на `TurnStarted` reasoning summary очищается;
- на `TurnStarted` план очищается только если прошлый plan полностью `completed`;
- незавершенный план переносится в следующий task.

## Runtime refresh behavior

Пока `/ A G E N T S /` открыт, содержимое обновляется в draw-cycle:

- при включенных анимациях: с каденсом `TARGET_FRAME_INTERVAL`;
- при выключенных анимациях: раз в `1s`.

Дополнительно:

- refresh SAW сохраняет текущий `scroll_offset` static-overlay (без "прыжка" в начало);
- выбор агента сохраняется по `thread_id` при пересборке списка;
- если выбранный/inspect-тред исчезает, состояние аккуратно re-clamp/recover.

## Internal design notes

- SAW-рендер вынесен в отдельный модуль `agents_overlay.rs`.
- `Overlay` дополнен предикатами `is_transcript()` / `is_agents()` и safe-update методом
  `replace_static_lines_preserve_scroll_if_title(...)`.
- В `app_backtrack` есть ранний перехват `Ctrl+T`, чтобы цикл работал одинаково и в overlay-routing.
- Вход в alt-screen для SAW защищен guard-условием (`enter` только если alt-screen не активен).

## Validation

Минимальная проверка:

- `cargo test -p codex-tui ctrl_t_overlay_action_cycles_transcript_and_agents_overlays`
- `cargo test -p codex-tui`

## Related features

- `docs/features/threadspawn-contract.md`
- `codex-rs/docs/features/agent-role-templates.md`
- `codex-rs/docs/features/threadspawn-agent-persona.md`
