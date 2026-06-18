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
| Runtime state | Runtime получает role defaults через existing config path; native `reload_user_config_layer` / `refresh_runtime_config` пересобирает `Config.agent_roles` из текущего `ConfigLayerStack`; после successful wizard create TUI вызывает existing app-server config reload path; новая TUI projection не создаёт one-off runtime state. |
| App-server projection | Wire protocol не менялся. Текущая итерация не требует нового request/response shape. |
| TUI projection | `/agent-roles` и alias `/agents` открывают catalog/create flow; create flow использует native TOML draft prompt; selected detail показывает declaration-field summary, TOML field summary, bounded field provenance, loader-derived inherited/overridden metadata provenance, file-level selected `config_file` detail for effective runtime TOML fields, typed availability/session-boundary state, runtime in-use summary from existing TUI thread metadata and current-thread tool catalog reference when a loaded-thread app-server read is available; `/agent` и `/subagents` сохраняют прежний active-agent/window behavior. |
| Persistence/resume | Created templates persist as TOML files; existing sub-agent session metadata с role string остаётся совместимым. |
| Tool/model/policy defaults | Role TOML может задавать developer instructions, model/provider/reasoning/service tier and `[tool_selection] allowed_tools` through existing flattened config. Permission/sandbox/env не расширялись. Role detail показывает declared tool-selection allowlist read-only; parser-validated TOML draft authoring, catalog-assisted create and discovered user-role allowlist editing покрываются через `agent-role-tool-selection`, а external app-server write/management API остаётся отдельным будущим contract. |
| Intentionally unaffected | App-server schema, mailbox/thread-store, permission profiles, sandbox/env selection, active sub-agent picker, existing session resume format. |

## Richer detail contract

Agent Role detail is a read-only projection over `Config.agent_roles`, built-ins and native role `ConfigToml` layers. It is not a second role schema and must not copy OpenClaude markdown/frontmatter fields into Codex role declarations when a native Codex source already exists.

## Field ownership matrix

| Field or group | Native owner | Fork decision |
| --- | --- | --- |
| `name`, source, built-in/user state, config path | Role declaration/projection | Keep in role template projection and selected detail. |
| `description` / OpenClaude `whenToUse` | Role declaration | Keep as native role metadata used by spawn guidance and TUI detail. |
| `nickname_candidates` | Role declaration | Keep as Codex-specific role metadata and show in detail. |
| `developer_instructions` | Role `ConfigToml` layer | Show as developer-instructions presence/summary; do not relabel it as OpenClaude `systemPrompt`. |
| `instructions`, `model_instructions_file` | General config layer | Treat as existing Codex prompt/config semantics, not role-profile metadata. |
| `model`, `model_provider`, `model_reasoning_effort`, `model_reasoning_summary`, `model_verbosity`, `service_tier` | Role `ConfigToml` layer and model/provider runtime | Show declared locks/effective contribution where native evidence exists; do not create a role-local model registry. |
| `approval_policy`, `default_permissions`, `[permissions]`, `sandbox_mode`, `[sandbox_workspace_write]` | Native permission/sandbox config | Show declared summary only; effective permission calculation remains native runtime behavior. |
| `[mcp_servers]`, `[hooks]`, `[skills.config]`, `[apps]` | Native subsystem configs | Show secret-safe counts/presence only; never expose command bodies, env values, secrets, hook payloads or app payloads. |
| OpenClaude `tools` / `disallowedTools` | No role-local Codex source | Do not add these fields to `AgentRoleToml`; use `agent-role-tool-selection` and native tool planning if a capability boundary is needed. |
| OpenClaude `permissionMode`, `color`, `memory`, `background`, `isolation`, `maxTurns`, `initialPrompt` | Absent or belongs to another Codex runtime contract | Keep out of role templates until a native source of truth and feature contract exist. |

## Anti-parallel-config rules

- Do not extend `AgentRoleToml` with fields already owned by `ConfigToml`, permissions, MCP, hooks, skills, apps, model providers or MAv2 runtime.
- Do not add OpenClaude-style `tools`, `disallowedTools`, `permissionMode`, `mcpServers`, persistent color or memory fields as role-local schema.
- Do not create a separate role/profile registry next to `Config.agent_roles` and built-ins.
- Do not parse TOML ad hoc for behavior. Narrow read-only summaries may inspect parsed role files, but runtime behavior must continue through the existing deserializer, config loader and `apply_role_to_config`.
- Detail copy must distinguish `declared`, `effective where proven`, `new-session only` and `unknown`; it must not imply hot reload or active usage without thread/runtime evidence.

## Data flow

`/agent-roles` резолвится slash-command parser в `SlashCommand::AgentRoles`, затем `chatwidget/slash_dispatch.rs` отправляет `AppEvent::OpenAgentRoleTemplates`. `app/event_dispatch.rs` запрашивает `agentRole/toolSelectionCatalog/read` для loaded thread when available, затем вызывает `ChatWidget::open_agent_role_templates_popup_with_runtime_catalog`, а popup строит список через `list_agent_role_templates(&self.config)` и показывает catalog read result только как current-thread tool-id reference.

Create flow использует `CustomPromptView`: prompt открывается с native TOML draft, включая короткую закомментированную подсказку про `[tool_selection] allowed_tools` для ручного native authoring, submit отдаёт draft в `AppEvent::CreateAgentRoleTemplateFromDraft`, TUI вызывает `create_user_agent_role_template_from_draft(&self.config, draft)`, core helper парсит draft через existing `parse_agent_role_file_contents`, требует normalized slug in `name`, проверяет existing roles, создаёт `$CODEX_HOME/agents/<role>.toml` через `create_new`, повторно валидирует TOML с финальным path label и добавляет созданную role в TUI catalog для текущего management view. `Create template from current model` остаётся тем же create flow: action только seed’ит draft текущими `Config.model`, `Config.model_provider_id`, `Config.model_reasoning_effort` и `Config.service_tier` как обычные flattened `ConfigToml` fields. Это не вызывает `UpdateModelSelection`, не меняет session/global model selection и не создаёт role-local provider catalog. После успешной записи dispatcher вызывает existing `AppServerSession::reload_user_config`, который отправляет `config/batchWrite` без edits с `reload_user_config: true`; app-server затем пересобирает loaded thread configs через native `refresh_runtime_config`. Старый helper `create_user_agent_role_template(raw_name)` сохранён как совместимый простой path и использует тот же write/validation helper.

Selection flow для spawn не менялся: пользователь или orchestrator просит Codex запустить sub-agent с `agent_type` равным имени template. Existing spawn handler применяет role defaults через native config. Новосозданный файл гарантированно загружается новыми сессиями из `$CODEX_HOME/agents/`; current loaded threads pick it up after a native config reload path (`reload_user_config_layer` / `refresh_runtime_config`) or restart, and the TUI wizard now triggers that reload explicitly after successful create.

## UX decisions

OpenClaude reference повлиял на форму, а не на код: экран строится как scan-first list с detail panel, source-state labels `[user active]`, `[built-in active]`, `[built-in shadowed]`, отдельной create action и понятной подсказкой, где лежит editable template. В Codex TUI это выражено через existing `SelectionView` и `CustomPromptView`, без отдельной landing page или standalone editor.

Basic active/shadowed states уже реализованы в native projection: `Config.agent_roles` entries идут первыми и остаются active; built-in roles не удаляются из списка, если user role имеет то же имя, а помечаются как shadowed. Это делает конфликт имён видимым пользователю без изменения `resolve_role_config` и `spawn_agent.agent_type`.

Selected detail теперь показывает native recovery anchors: source, status, config file, editable native profile fields, declaration-field summary from effective `AgentRoleConfig`, TOML field summary from the role file/bundled config, bounded field provenance, loader-derived inherited/overridden metadata provenance, file-level selected `config_file` inheritance/override detail for effective runtime TOML fields when the native loader proves it, parser validation status for user TOML files, nickname candidates, typed availability/session-boundary state, runtime in-use summary, role locks (`model`, `model_provider`, `model_reasoning_effort`, `service_tier`, `developer_instructions` presence), declared `[tool_selection] allowed_tools`, current-thread runtime catalog reference when available and runtime binding через `spawn_agent.agent_type`. Catalog reference is explicitly not a role-specific preview and does not create an editor registry. Runtime in-use summary is derived from `AgentNavigationState` rows and thread metadata `agent_role`; it counts known non-primary threads by status and can mark the current viewed thread, but it does not infer whether a historical thread used a built-in or user-shadowing definition because thread metadata stores only the `agent_type` string. Availability is a core projection state: user templates are new-session/reload-boundary scoped, active built-ins need no user file, and shadowed built-ins are not selected by `agent_type`. Это закрывает часть richer detail без нового config format.

Built-in роли read-only, потому что native source не предполагает их редактирование. User role `Enter` открывает TOML file через existing open-url event с `file://`, чтобы не вводить новый inline editor и не дублировать config authoring surface.

## OpenClaude-inspired next stage

OpenClaude даёт полезную UX форму, но не переносимый data contract. Текущий Codex slice уже строится вокруг TOML-native wizard and richer detail:

- wizard writes only parser-validated `$CODEX_HOME/agents/*.toml` role files;
- current-model staged create seed writes only native flattened role TOML keys and remains create-only;
- generated drafts produce native TOML fields and fail before write on parser errors;
- detail rows already distinguish role declaration fields (`description`, `config_file`, `nickname_candidates`), declaration config-layer origins where `ConfigLayerStack.origins()` can prove them, native TOML file fields, config-file origin (`$CODEX_HOME/agents`, external `config_file`, built-in bundle), bounded field provenance, loader-derived inherited/overridden metadata provenance, file-level selected `config_file` detail for effective runtime TOML fields, parser validation status and a bounded role-lock summary (`developer_instructions`, model/provider/reasoning/service tier, declared tool-selection allowlist, current-thread runtime catalog reference, permission/sandbox summaries and secret-safe MCP/hooks/skills/apps summaries);
- richer source/provenance states must respect `ConfigLayerStack`, field-level inheritance and built-in fallback; текущий bounded field provenance intentionally reports only what native evidence proves: config-layer field origin, role-file metadata, effective role-file runtime fields, per-field runtime TOML inherited/overridden detail from `AgentRoleConfig.runtime_config_sources`, discovered user role files, built-in bundle files and loader trace for inherited/overridden role metadata fields. Текущий `shadowed by` слой покрывает только user-vs-built-in role name conflict, а OpenClaude `shadowed by` is not sufficient for Codex because an effective role can inherit fields across layers;
- availability state is intentionally explicit and conservative: `NewSessionsOnly` means the role file is available to new sessions and to already-loaded sessions only after native config reload/restart, `BuiltIn` means no user file is required, and `ShadowedBuiltIn` means the built-in definition is visible for inspection but not selected by `agent_type`;
- `active` in template catalog must not mean “selected profile”; in Codex live usage belongs to sub-agent thread metadata. The current in-use summary is rendered only from existing `AgentNavigationState` evidence and intentionally says when no known thread uses the role in the current TUI session.

OpenClaude-only markdown/frontmatter profile files, persistent color metadata, memory prompt injection and generic `tools`/`disallowedTools` fields remain out of scope until native Codex contracts exist.

## Compatibility notes

Старые сессии остаются читаемыми: session metadata хранит role as string, а missing/new TOML fields уже имеют поведение existing config loader. Конфликты имён fail fast в UI вместо silent fallback. Generated starter file использует existing TOML schema и проходит parse check до записи.

Главный текущий provenance gap: loader-derived trace покрывает только metadata fields in `AgentRoleConfig` (`description`, `config_file`, `nickname_candidates`). Runtime TOML fields (`model`, `permissions`, `mcp_servers`, hooks and similar) now expose only file-level selected `config_file` inheritance/override detail; полное объяснение того, какие конкретные runtime TOML fields были inherited или overridden across role files, потребует расширить native role-loading/apply-role model, потому что эти поля сейчас применяются как один selected role config layer at spawn. Runtime reload foundation теперь есть: `reload_user_config_layer` и `refresh_runtime_config` пересобирают `Config.agent_roles` из актуального `ConfigLayerStack`, а wizard create переиспользует existing app-server reload path. Same-active-turn model-visible tool schema mutation remains outside the current contract.
