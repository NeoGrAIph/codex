# Agent role templates design

## Цель итерации

Цель текущей итерации — дать пользователю рабочий TUI management surface для native Codex agent roles без изменения upstream behavior. Реализация использует уже существующие TOML roles, built-ins и `spawn_agent.agent_type`; markdown/persona перенос не смешивается с этой итерацией.

## Native source of truth

Единственный source of truth для runtime roles остаётся `Config.agent_roles` вместе с built-in role configs из `codex-rs/core/src/agent/role.rs`. Новый модуль `codex-rs/core/src/agent_role_templates.rs` не содержит собственного списка ролей и не добавляет profile format: он строит read-only projection поверх existing configs и создаёт starter TOML через existing parser contract.

User templates живут в `$CODEX_HOME/agents/*.toml`. Built-in roles остаются read-only и берутся из existing built-in TOML content. Конфликт имён проверяется через `resolve_role_config`, поэтому user role не может молча затереть built-in или уже загруженную role.

## Integration map

| Surface | Решение |
| --- | --- |
| Role/template/persona source | `Config.agent_roles` + built-ins; persona manifest пока не переносился. |
| Config/profile surface | Existing `$CODEX_HOME/agents/*.toml`; отдельный `profiles`/`templates` config не добавлен. |
| Spawn tool spec/handlers | Existing `spawn_agent.agent_type` остаётся selector; применение роли остаётся в `apply_role_to_config`/`resolve_role_config`. |
| Runtime state | Runtime получает role defaults через existing config path; новая TUI projection не создаёт one-off runtime state. |
| App-server projection | Wire protocol не менялся. Текущая итерация не требует нового request/response shape. |
| TUI projection | `/agent-roles` и alias `/agents` открывают catalog/create flow; `/agent` и `/subagents` сохраняют прежний active-agent/window behavior. |
| Persistence/resume | Created templates persist as TOML files; existing sub-agent session metadata с role string остаётся совместимым. |
| Tool/model/policy defaults | Role TOML может задавать developer instructions, model, reasoning, service tier, tools/skills через existing flattened config. Permission/sandbox/env не расширялись. |
| Intentionally unaffected | App-server schema, mailbox/thread-store, permission profiles, sandbox/env selection, active sub-agent picker, existing session resume format. |

## Data flow

`/agent-roles` резолвится slash-command parser в `SlashCommand::AgentRoles`, затем `chatwidget/slash_dispatch.rs` отправляет `AppEvent::OpenAgentRoleTemplates`. `app/event_dispatch.rs` вызывает `ChatWidget::open_agent_role_templates_popup`, а popup строит список через `list_agent_role_templates(&self.config)`.

Create flow использует `CustomPromptView`: prompt отдаёт raw name в `AppEvent::CreateAgentRoleTemplate`, TUI вызывает `create_user_agent_role_template(&self.config, raw_name)`, core helper нормализует имя, проверяет existing roles, создаёт `$CODEX_HOME/agents/<role>.toml` через `create_new`, валидирует generated TOML тем же parser path и добавляет созданную role в TUI catalog для текущего management view.

Selection flow для spawn не менялся: пользователь или orchestrator просит Codex запустить sub-agent с `agent_type` равным имени template. Existing spawn handler применяет role defaults через native config. Новосозданный файл гарантированно загружается новыми сессиями из `$CODEX_HOME/agents/`; hot reload уже запущенного core thread не добавлялся в этой итерации.

## UX decisions

OpenClaude reference повлиял на форму, а не на код: экран строится как scan-first list с detail panel, source labels `[user]`/`[built-in]`, отдельной create action и понятной подсказкой, где лежит editable template. В Codex TUI это выражено через existing `SelectionView` и `CustomPromptView`, без отдельной landing page или standalone editor.

Built-in роли read-only, потому что native source не предполагает их редактирование. User role `Enter` открывает TOML file через existing open-url event с `file://`, чтобы не вводить новый inline editor и не дублировать config authoring surface.

## Compatibility notes

Старые сессии остаются читаемыми: session metadata хранит role as string, а missing/new TOML fields уже имеют поведение existing config loader. Конфликты имён fail fast в UI вместо silent fallback. Generated starter file использует existing TOML schema и проходит parse check до записи.

Главный текущий gap: создание role во время уже открытой app-server/core thread не является полноценным hot reload tool schema/config. Для гарантированного spawn с новой role нужно использовать новую сессию или runtime path, который заново загрузит `$CODEX_HOME/agents/*.toml`. Следующий этап должен либо добавить native config reload extension point, либо явно задокументировать session boundary как продуктовый contract.
