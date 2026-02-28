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

## Internal design notes

- SAW-рендер вынесен в отдельный модуль `agents_overlay.rs`.
- `Overlay` дополнен предикатами `is_transcript()` / `is_agents()` и safe-update методом
  `replace_static_lines_if_title(...)`.
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
