# Agent role templates

## Статус

Первая итерация для `feature/140/agent-role-templates` реализует native UX управления TOML role templates: TUI показывает built-in и user roles, создаёт starter template в `$CODEX_HOME/agents/*.toml`, открывает user template file для редактирования и оставляет spawn selection на существующем `spawn_agent.agent_type`.

Markdown role templates, persona manifest и более глубокая thread persona projection остаются следующим этапом переноса. Эта итерация намеренно не меняет app-server protocol и не добавляет параллельный profile registry.

## Канонические ссылки

| Документ | Назначение |
| --- | --- |
| `docs/fork/features/agent-role-templates.md` | Feature contract, historical refs и coverage по `fork/140`. |
| `docs/fork/research/multi-agents/architecture.md` | Native multi-agent architecture: spawn tool, registry, runtime state, persistence, TUI/app-server projection. |
| `docs/fork/research/agent-window/README.md` | Visual TUI references для agent windows/overlay states. |
| `docs/fork/projects/agent-role-templates/design.md` | Integration map и data flow текущей реализации. |
| `docs/fork/projects/agent-role-templates/verification.md` | Verification matrix, команды и known gaps. |

## Implementation surfaces

| Surface | Файлы |
| --- | --- |
| Core projection/create helper | `codex-rs/core/src/agent_role_templates.rs`, `codex-rs/core/src/agent_role_templates_tests.rs`, `codex-rs/core/src/agent/role.rs`, `codex-rs/core/src/lib.rs` |
| TUI command/event flow | `codex-rs/tui/src/slash_command.rs`, `codex-rs/tui/src/chatwidget/slash_dispatch.rs`, `codex-rs/tui/src/app_event.rs`, `codex-rs/tui/src/app/event_dispatch.rs` |
| TUI UX | `codex-rs/tui/src/chatwidget/agent_role_templates.rs`, `codex-rs/tui/src/chatwidget.rs`, `codex-rs/tui/src/chatwidget/tests/popups_and_settings.rs`, `codex-rs/tui/src/chatwidget/snapshots/codex_tui__chatwidget__tests__agent_role_templates_popup.snap` |
| App-server client bridge | `codex-rs/app-server-client/src/lib.rs` |

## Пользовательский flow

Пользователь вводит `/agent-roles` или `/agents`, выбирает `Create new template`, задаёт имя роли, получает `$CODEX_HOME/agents/<normalized-name>.toml`, редактирует описание/instructions/defaults в этом файле и затем просит Codex запустить sub-agent с `agent_type` равным имени роли. Built-in роли можно инспектировать, но не редактировать через этот экран.
