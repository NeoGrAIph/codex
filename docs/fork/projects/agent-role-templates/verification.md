# Agent role templates verification

## Matrix

| Проверка | Команда | Результат |
| --- | --- | --- |
| Rust formatting | `just fmt` из `codex-rs` | Пройдено после code changes; повторить перед финальной сдачей после docs не требуется для Markdown, но команда уже выполнялась для изменённого Rust. |
| Core helper tests | `just test -p codex-core agent_role_templates` из `codex-rs` | Пройдено: catalog merge, create valid role file, built-in conflict rejection, name normalization. |
| TUI focused tests | `just test -p codex-tui agent_role` из `codex-rs` | Пройдено: `/agents` alias, popup snapshot, create flow updates session catalog. |
| Snapshot review | `cargo insta show tui/src/chatwidget/snapshots/codex_tui__chatwidget__tests__agent_role_templates_popup.snap.new` и `cargo insta accept --snapshot 'tui/src/chatwidget/snapshots/codex_tui__chatwidget__tests__agent_role_templates_popup.snap'` | Применено только к новому Agent Role Templates snapshot. |
| Full TUI suite | `just test -p codex-tui` из `codex-rs` | Запуск выполнен, но suite упал на 30 unrelated failures в status/title/history/IPC surfaces; многие snapshot diffs показывали version drift `v0.0.0 -> v0.140.0`. Сгенерированные `.snap.new` отклонены через `cargo insta reject` и не приняты в change set. |
| Pending snapshots | `cargo insta pending-snapshots` из `codex-rs` | Пройдено: `No pending snapshots.` |
| Diff hygiene | `git diff --check` | Пройдено. |

## Coverage

Core tests подтверждают, что TUI не получает отдельный role registry: список строится из `Config.agent_roles` и built-ins, create flow пишет валидный TOML в native `$CODEX_HOME/agents/`, а имена built-in roles блокируются. TUI tests подтверждают, что `/agents` теперь ведёт в management surface, popup визуально показывает create/user/built-in rows и создание добавляет роль в текущий management catalog.

## Known gaps

Markdown/persona loader, richer authoring wizard, inline editor, reviewer-style built-ins and hot reload of already-running core/app-server thread config не входят в текущую итерацию. Самый важный runtime gap — newly-created role file гарантированно подхватывается новыми сессиями, но текущая уже открытая thread может не обновить `spawn_agent.agent_type` schema без native reload path.
