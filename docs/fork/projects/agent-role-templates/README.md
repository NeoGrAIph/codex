# Agent role templates

## Статус

Первая итерация для `feature/140/agent-role-templates` реализует native UX управления TOML role templates: TUI показывает built-in и user roles, создаёт role template через parser-validated native TOML draft в `$CODEX_HOME/agents/*.toml`, редактирует existing user TOML через inline parser-validated editor, показывает selected detail с source/status/config/config-origin/declaration-origin/bounded field provenance/loader-derived metadata inheritance/per-field runtime TOML source detail/locks/runtime binding/runtime in-use evidence и оставляет spawn selection на существующем `spawn_agent.agent_type`. Built-in `reviewer` добавлен как обычная config-backed роль: она видна в том же catalog, shadowable user TOML с тем же именем, и получает read-only defaults через bundled `reviewer.toml`, а не через отдельный runtime enum.

OpenClaude-inspired следующий этап улучшает этот же TOML-native путь через более удобные staged controls поверх уже реализованного TOML draft wizard. Текущий detail уже показывает config-file origin проекцию (`$CODEX_HOME/agents`, external `config_file` или built-in bundle), config-layer origins для declaration fields через `ConfigLayerStack.origins()`, bounded field provenance для role-file metadata/effective runtime fields/discovered files/built-in bundle files, per-field runtime TOML inherited/overridden detail from native `AgentRoleConfig.runtime_config_sources`, loader-derived inherited/overridden details for role metadata fields, typed availability/session-boundary state, native `Config.agent_roles` reload через config refresh, next-foreground-turn `spawn_agent.agent_type` schema refresh после native reload, automatic wizard-triggered reload через existing app-server config reload and runtime in-use summary по существующей TUI thread metadata (`agent_role`, status, current viewed thread). Staged controls для `role-authoring-v2` now include `Create template from current model`, inline existing user-role TOML editing, existing user-role `Use current model defaults`, existing user-role `Use current model/provider`, existing-user-role `Use current reasoning`, existing-user-role `Clear model defaults`, catalog-assisted create from current tools and separate discovered-user-role edits for native `[tool_selection] allowed_tools` and `denied_tools`. Все эти actions открывают тот же parser-validated native TOML draft/update path, не меняют текущую модель сессии, не создают role-local model/tool registry и не добавляют app-server API. Markdown role templates, persona manifest and persistent color/memory metadata не входят в эту feature без отдельного native contract. Role-level tool selection вынесен в `docs/fork/features/agent-role-tool-selection.md`: runtime allow/deny enforcement, read-only detail projection, parser-validated TOML draft authoring, catalog-assisted create and discovered user-role allow/deny editing уже есть, а external app-server write/management API существует отдельным whole-policy contract для клиентов, которым нужно менять policy без TUI/TOML authoring.

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

Пользователь вводит `/agent-roles` или `/agents`, выбирает `Create new template` для базового TOML draft, `Create template from current model`, чтобы сразу перенести текущие model defaults в будущую роль, или `Create template from current tools`, чтобы seed'ить native `[tool_selection] allowed_tools` из current-thread runtime catalog. Для discovered user roles экран также показывает `Edit native TOML`, `Use current model defaults`, `Use current model/provider`, `Use current reasoning`, `Clear model defaults`, `Edit allowed tools` и `Edit denied tools`; каждое действие открывает editable TOML draft/update path для того же `$CODEX_HOME/agents/<role>.toml`, а tool actions сначала показывают searchable current-thread catalog picker. `Use current model/provider` меняет только flattened `model` и `model_provider`, сохраняя role-local `model_reasoning_effort`, `service_tier`, `[tool_selection]`, `[subagent_action_policy]` and other native tables. Built-in роли можно инспектировать, но не редактировать через этот экран.

Следующий OpenClaude-inspired UX может добавить пошаговые controls для model/provider/reasoning/service-tier поверх того же TOML draft contract и показывать richer in-use state только когда для этого есть native evidence из config/runtime.
