# Agent role templates

## Статус

Первая итерация для `feature/140/agent-role-templates` реализует native UX управления TOML role templates: TUI показывает built-in и user roles, создаёт role template через parser-validated native TOML draft в `$CODEX_HOME/agents/*.toml`, открывает user template file для редактирования, показывает selected detail с source/status/config/config-origin/declaration-origin/bounded field provenance/loader-derived metadata inheritance/per-field runtime TOML source detail/locks/runtime binding/runtime in-use evidence и оставляет spawn selection на существующем `spawn_agent.agent_type`.

OpenClaude-inspired следующий этап должен улучшить этот же TOML-native путь: более удобные staged controls поверх уже реализованного TOML draft wizard. Текущий detail уже показывает config-file origin проекцию (`$CODEX_HOME/agents`, external `config_file` или built-in bundle), config-layer origins для declaration fields через `ConfigLayerStack.origins()`, bounded field provenance для role-file metadata/effective runtime fields/discovered files/built-in bundle files, per-field runtime TOML inherited/overridden detail from native `AgentRoleConfig.runtime_config_sources`, loader-derived inherited/overridden details for role metadata fields, typed availability/session-boundary state, native `Config.agent_roles` reload через config refresh, next-foreground-turn `spawn_agent.agent_type` schema refresh после native reload, automatic wizard-triggered reload через existing app-server config reload and runtime in-use summary по существующей TUI thread metadata (`agent_role`, status, current viewed thread). Markdown role templates, persona manifest and persistent color/memory metadata не входят в эту feature без отдельного native contract. Role-level tool selection вынесен в `docs/fork/features/agent-role-tool-selection.md`: runtime allowlist, read-only detail projection, parser-validated TOML draft authoring, catalog-assisted create and discovered user-role allowlist editing уже есть, а будущим срезом остаётся external app-server write/management API, если внешним клиентам понадобится менять эту policy.

Эта итерация намеренно не меняет app-server protocol и не добавляет параллельный profile registry.

## Канонические ссылки

| Документ | Назначение |
| --- | --- |
| `docs/fork/features/agent-role-templates.md` | Feature contract, historical refs и coverage по `fork/140`. |
| `docs/fork/research/multi-agents/architecture.md` | Native multi-agent architecture: spawn tool, registry, runtime state, persistence, TUI/app-server projection. |
| `docs/fork/research/agent-window/README.md` | Visual TUI references для agent windows/overlay states. |
| `docs/fork/projects/agent-role-templates/design.md` | Integration map и data flow текущей реализации. |
| `docs/fork/projects/agent-role-templates/verification.md` | Verification matrix, команды и known gaps. |
| `docs/fork/features/agent-role-tool-selection.md` | Отдельный контракт для role-level tool filtering, чтобы не смешивать UX ролей с permissions/tool runtime. |

## Implementation surfaces

| Surface | Файлы |
| --- | --- |
| Core projection/create helper | `codex-rs/core/src/agent_role_templates.rs`, `codex-rs/core/src/agent_role_templates_tests.rs`, `codex-rs/core/src/agent/role.rs`, `codex-rs/core/src/lib.rs` |
| TUI command/event flow | `codex-rs/tui/src/slash_command.rs`, `codex-rs/tui/src/chatwidget/slash_dispatch.rs`, `codex-rs/tui/src/app_event.rs`, `codex-rs/tui/src/app/event_dispatch.rs` |
| TUI UX | `codex-rs/tui/src/chatwidget/agent_role_templates.rs`, `codex-rs/tui/src/chatwidget.rs`, `codex-rs/tui/src/chatwidget/tests/popups_and_settings.rs`, `codex-rs/tui/src/chatwidget/snapshots/codex_tui__chatwidget__tests__agent_role_templates_popup.snap` |
| App-server client bridge | `codex-rs/app-server-client/src/lib.rs` |

## Пользовательский flow

Пользователь вводит `/agent-roles` или `/agents`, выбирает `Create new template`, редактирует предзаполненный TOML draft с `name`, `description`, `nickname_candidates` and `developer_instructions`, получает `$CODEX_HOME/agents/<name>.toml`, при необходимости продолжает редактировать defaults в этом файле и затем просит Codex запустить sub-agent с `agent_type` равным имени роли. Built-in роли можно инспектировать, но не редактировать через этот экран.

Следующий OpenClaude-inspired UX может добавить пошаговые controls для model/provider/reasoning/service-tier поверх того же TOML draft contract и показывать richer in-use state только когда для этого есть native evidence из config/runtime.
