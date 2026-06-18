# Agent role templates

## Feature passport

- Code name: `agent-role-templates`
- Status: native TUI-итерация расширена для `fork/140`; markdown/persona перенос остаётся отдельным будущим этапом.
- Goal: дать sub-agents осмысленные роли и инструкции через native role templates, чтобы пользователь мог видеть, создавать, проверять и применять специализации без ручного повторения роли в каждом prompt.
- Scope in: native TOML role catalog, built-in/configured role projection, parser-validated TOML draft wizard, TUI list/detail/create/open-file flow, source/provenance states, current-session runtime in-use evidence, spawn-agent role selection through existing `agent_type`.
- Scope out: markdown role loader, persona manifest, OpenClaude markdown/frontmatter profiles, persistent color/memory metadata without native source of truth, role-level tool selection editor; runtime limits и cwd описываются отдельно.

## Как работает для пользователя

Пользователь открывает `/agent-roles` или `/agents`, видит built-in и user templates, создаёт новый user template через предзаполненный native TOML draft, при необходимости открывает этот TOML-файл для редактирования и затем просит Codex запустить sub-agent с `agent_type` равным имени роли. Built-in роли остаются read-only; user roles управляются через существующий native config directory.

## Branches and commits

| Branch | Baseline | Commit | Date | Subject | Роль |
| --- | --- | --- | --- | --- | --- |
| `fork/saw` | `rust-v0.99.0` | `a15d4adc2c` | 2026-02-13 | `[SA][SAW] finalize sub-agent templates, guards, and UI integration` | Первичная связка templates с sub-agent workflow. |
| `fork/101` | `rust-v0.101.0` | `9f82deac0f` | 2026-02-15 | `feat(sa): complete Sub-Agents fork contract (templates, thread_note, runtime listing, TUI/SAW polish)` | Полный fork contract templates/runtime listing. |
| `fork/106` | `rust-v0.106.0` | `1e5e63ff26` | 2026-02-28 | `docs: add agent manifest template and authoring instructions` | Authoring guidance для templates. |
| `fork/106` | `rust-v0.106.0` | `c402866871` | 2026-02-28 | `feat: add agent role templates and thread persona metadata` | Runtime/templates/persona metadata. |
| `fork/107` | `rust-v0.107.0` | `4469b595c8` | 2026-02-28 | `feat: add agent role templates and thread persona metadata` | Перенос на 0.107. |
| `fork/111` | `rust-v0.111.0` | `7a977ec76b` | 2026-03-06 | `feat(agents): add role templates and thread persona foundation` | Обновленный foundation для templates/persona. |
| `fork/118` | `rust-v0.118.0` | `a233952c1c` | 2026-04-03 | `Implement agent role templates across core, state, app-server, and TUI` | Сквозная интеграция core/state/app-server/TUI. |
| `fork/118` | `rust-v0.118.0` | `2e6af0d4a2` | 2026-04-04 | `Support model instructions in markdown agent roles` | Markdown role templates получают model instructions. |
| `fork/130` | `rust-v0.130.0` | `783ebf1fac` | 2026-05-14 | `feat(agents): add markdown role templates` | Перенос markdown role templates на 0.130 lineage. |
| `chrome_plugin` | `rust-v0.130.0` lineage | `783ebf1fac` | 2026-05-14 | `feat(agents): add markdown role templates` | Та же возможность в chrome-plugin lineage. |

## Implementation notes

Затрагивались template files, core agent control/registry, protocol/thread metadata, app-server projection, TUI rendering и docs. В поздних переносах это стало cross-surface feature, а не только набором markdown-файлов. В `fork/130` native TOML roles остаются primary, а markdown roles дают defaults для `model`/`reasoning_effort`; full-history fork rejects role/persona/model/reasoning/cwd overrides.

## Native coverage in rust-v0.140.0 / fork/140

Status: `partial, expanded`. Native release уже имел role system: `apply_role_to_config`, `resolve_role_config`, built-ins `default`, `explorer`, `worker`, TOML configs, `AgentRoleConfig`, `spawn_agent.agent_type`, persisted role/nickname/path metadata and TUI labels. В `feature/140/agent-role-templates` добавлен native projection/catalog для этих ролей, TUI управление templates без нового config format, basic active/shadowed source states для user roles, которые перекрывают built-in roles, parser-validated TOML draft wizard, automatic post-wizard reload through the existing `config/batchWrite reload_user_config` path, next-foreground-turn `spawn_agent.agent_type` schema refresh evidence after native reload, read-only detail projection для declared `[tool_selection] allowed_tools`, current-thread runtime catalog reference from `agentRole/toolSelectionCatalog/read`, current-session runtime in-use summary from existing TUI thread metadata, typed availability/session-boundary state for user, built-in and shadowed built-in roles, native config-refresh reload of `Config.agent_roles`, config-file origin projection for `$CODEX_HOME/agents`, external `config_file` and built-in bundle roles, bounded declaration origin projection from `ConfigLayerStack.origins()`, typed bounded field provenance for config-layer fields, role-file metadata, effective role-file runtime fields with per-field inherited/overridden detail where the native role loader proves it, discovered user files and built-in bundle files, and loader-derived inherited/overridden provenance for role metadata fields (`description`, `config_file`, `nickname_candidates`). Не хватает fork markdown role templates/persona manifest contract, native markdown role loader, reviewer-style built-ins and markdown model-instruction template behavior.

## OpenClaude-inspired target

OpenClaude `AgentsMenu`, `AgentDetail`, `AgentEditor`, `CreateAgentWizard` and `ToolSelector` используются как UX reference, но не как формат данных. Codex role templates должны оставаться TOML-native:

- source of truth: `Config.agent_roles` plus built-ins, not a separate profile registry;
- persistence: `$CODEX_HOME/agents/*.toml` and existing `ConfigToml` parser path;
- binding: `spawn_agent.agent_type`, `apply_role_to_config`, `resolve_role_config`;
- authoring: wizard opens a native TOML draft with `name`, `description`, `nickname_candidates`, `developer_instructions` and a commented `[tool_selection] allowed_tools` hint; submitted draft must pass existing role-file parser validation before writing `$CODEX_HOME/agents/<role>.toml`;
- source states/detail: basic `user active`, `built-in active` and `built-in shadowed` are implemented from native role provenance; selected detail shows source, status, config file, config origin, editable native TOML fields, parser validation status for user role files, declaration-field summary, declaration config-layer origins where available, TOML field summary, bounded field-source provenance, loader-derived inherited/overridden role metadata provenance, file-level selected `config_file` detail for effective runtime TOML fields where available, nicknames, current-session runtime in-use summary, role locks, declared tool-selection allowlist, current-thread runtime catalog reference when available, runtime binding and current-session availability boundary. Richer effective/configured/editable/partial/ignored/disabled/new-session-only states must be based on native `ConfigLayerStack`/loader/runtime evidence, not OpenClaude `allAgents` semantics;
- hot reload: same-thread availability remains explicitly bounded through typed projection state; native `reload_user_config_layer` / `refresh_runtime_config` reload `Config.agent_roles`, the next foreground turn rebuilds `spawn_agent.agent_type` schema from the refreshed role catalog, and the TUI wizard calls existing app-server `reload_user_config` after successful create instead of adding a new role-template endpoint.

OpenClaude-only concepts are intentionally excluded until separately contracted: markdown/frontmatter profile files, `.openclaude/agents`, persistent color, memory prompt injection and generic `tools`/`disallowedTools` fields.

## Native integration map

| Surface | Source of truth / behavior |
| --- | --- |
| Source of truth | `Config.agent_roles` плюс built-ins из `codex-rs/core/src/agent/role.rs`; новый `agent_role_templates` только проецирует этот источник и создаёт user TOML-файл. |
| Config/profile surface | `$CODEX_HOME/agents/*.toml` и существующий flattened `AgentRoleConfig`; TOML draft wizard использует existing parser path; отдельный profile registry не добавляется. |
| Spawn tool spec/handlers | `spawn_agent.agent_type` остаётся native selector; schema/list продолжает строиться из role config через существующий multi-agent path. |
| Runtime state | `apply_role_to_config` и `resolve_role_config` остаются местом применения role defaults; native config refresh/reload пересобирает `Config.agent_roles` из текущего `ConfigLayerStack`; next foreground turn получает refreshed role catalog in `spawn_agent.agent_type` schema через обычный `ToolRouter::from_turn_context`; после successful wizard create TUI вызывает existing app-server config reload path; отдельная role enum/list не хранится. |
| TUI projection | `/agent-roles` и alias `/agents` открывают list/detail/create flow; create flow открывает native TOML draft prompt; selected user role detail показывает config path, config origin, editable native fields, declaration-field summary, declaration config-layer origins where available, TOML field summary, bounded field-source provenance with inherited/overridden metadata details where native loader proves them and file-level selected `config_file` detail for effective runtime TOML fields, parser validation status, current-session runtime in-use summary, model/provider/reasoning/developer-instructions lock summary, declared `[tool_selection] allowed_tools`, current-thread runtime catalog reference when available, declared permission/sandbox defaults, secret-safe MCP/hooks/skills/apps subsystem summaries, `spawn_agent.agent_type` binding and typed session-boundary availability state; `/agent` и `/subagents` продолжают управлять active sub-agent windows. |
| App-server protocol | Не менялся: role templates остаются config/runtime capability, wire contract не расширен. |
| Persistence/resume | Новые role files сохраняются как `$CODEX_HOME/agents/*.toml`; existing session metadata с `agent_role` остаётся строковым и backward-compatible. |
| Tool/model/policy defaults | Модель, provider, reasoning, service tier, developer instructions и `[tool_selection] allowed_tools` могут наследоваться через native role config при spawn. Role detail показывает configured allowlist read-only; editor/authoring остаётся в отдельной `agent-role-tool-selection` feature. |
| Permission/sandbox defaults | `approval_policy`, `sandbox_mode`, `default_permissions`, `[sandbox_workspace_write]` and `[permissions]` остаются native config-layer fields. Role detail показывает только declared summary; effective runtime permission profile продолжает рассчитываться штатным config/runtime path. |
| Subsystem defaults | `[mcp_servers]`, `[hooks]`, `[skills.config]` and `[apps]` остаются native config-layer fields. Role detail показывает только counts/presence summary без команд, env, secrets, hook body, tool args или app payload. |
| Intentionally unaffected | Effective permission calculation, sandbox/env selection, mailbox/thread-store format, app-server schema, existing `/agent` sub-agent navigation. |

## Porting/current-state notes

При переносе в новую ветку нужно проверять current upstream skills/agents/model instructions path. Fork templates должны использовать native template/resource loading, а не отдельный обходной loader.

## Verification matrix

| Surface | Проверка |
| --- | --- |
| Core role template projection/create | `just test -p codex-core agent_role_templates` |
| TUI source-state popup snapshot | `just test -p codex-tui agent_role_templates_popup_snapshot agent_role_templates_popup_runtime_catalog_snapshot` |
| TUI slash/popup/create broader focused tests | `just test -p codex-tui agent_role` |
| Snapshot acceptance | `cargo insta show tui/src/chatwidget/snapshots/codex_tui__chatwidget__tests__agent_role_templates_popup.snap.new` и `cargo insta accept --snapshot 'tui/src/chatwidget/snapshots/codex_tui__chatwidget__tests__agent_role_templates_popup.snap'` при intentional UI diff. |
| Formatting | `just fmt` в `codex-rs` |
| Diff hygiene | `git diff --check` |

## Doc changelog

- 2026-06-17: зафиксирована первая `fork/140` итерация: native catalog/create TUI для TOML role templates, без app-server protocol change и без markdown/persona loader.
- 2026-06-18: зафиксирован OpenClaude-inspired target для guided TOML wizard, richer detail, source/provenance states and explicit exclusion of markdown/frontmatter/color/memory/generic tool-selection without native contracts.
- 2026-06-18: реализованы basic active/shadowed source states для TOML role templates: user role с именем built-in помечается `user active`, соответствующая built-in role остаётся видимой как `built-in shadowed`.
- 2026-06-18: selected detail для user role расширен native contract подсказками: config file, nickname candidates, model/reasoning/developer-instructions locks, `spawn_agent.agent_type` binding and current-session reload boundary.
- 2026-06-18: selected detail and core projection now surface parser-validation diagnostics for file-backed user TOML roles and explicitly name editable native fields; no markdown/frontmatter/color/memory/tools profile fields were added.
- 2026-06-18: role detail lock summary now includes native `model_provider` from role TOML, so provider-aware roles can be inspected without a parallel profile schema.
- 2026-06-18: role detail now shows declared native permission/sandbox defaults (`approval_policy`, `sandbox_mode`, `default_permissions`, `[sandbox_workspace_write]`, `[permissions]`) without changing effective permission calculation.
- 2026-06-18: role detail now shows secret-safe subsystem summaries for declared `[mcp_servers]`, `[hooks]`, `[skills.config]` and `[apps]` without exposing commands, env, secrets or hook/app payloads.
- 2026-06-18: create flow now opens and validates a native TOML draft instead of accepting only a role name; generated files still live in `$CODEX_HOME/agents/*.toml`, the draft includes a commented `[tool_selection] allowed_tools` hint for native role-tool authoring, and no markdown/frontmatter profile format was added.
- 2026-06-18: role detail lock summary now includes declared `[tool_selection] allowed_tools`, so users can inspect role capability boundaries without a parallel role profile schema.
- 2026-06-18: role detail now includes first field-source summary: declaration fields from effective `AgentRoleConfig` and TOML fields from the native role file/bundled role config. Full `ConfigLayerStack` inherited/overridden provenance remains a separate gap.
- 2026-06-18: role detail now shows config-file origin for `$CODEX_HOME/agents`, external `config_file` and built-in bundle roles, so the user can distinguish native user templates from externally referenced files without a separate profile registry.
- 2026-06-18: role template projection now reports config-layer origins for declaration fields via `ConfigLayerStack.origins()`. This covers `agents.<role>.description`, `config_file` and `nickname_candidates` declarations; later loader trace adds inherited/overridden markers for those metadata fields only.
- 2026-06-18: OpenClaude Agent Detail fields classified into Codex-native declaration fields, role `ConfigToml` fields, external subsystem summaries and intentionally unsupported role-local fields; this keeps richer detail additive without importing a parallel profile schema.
- 2026-06-18: role template projection now exposes bounded field provenance as typed data and TUI detail copy: config-layer fields, role-file metadata, effective role-file runtime fields, discovered `$CODEX_HOME/agents` files and built-in bundle files.
- 2026-06-18: role detail now consumes `agentRole/toolSelectionCatalog/read` when a loaded thread exists and renders the current-thread runtime catalog as read-only tool-id evidence, explicitly not as a role-specific preview or editor registry.
- 2026-06-18: role detail now shows current-session runtime in-use evidence from existing TUI `AgentNavigationState` (`agent_role`, status and current viewed thread), without adding a role usage registry or claiming built-in/user definition source for historical threads.
- 2026-06-18: native role loader now tracks metadata-field provenance inside `AgentRoleConfig` for `description`, `config_file` and `nickname_candidates`, including inherited-from-lower-precedence and overrides-lower-precedence flags. `/agent-roles` consumes that trace through the existing `Field sources:` line; effective runtime TOML fields now also use native `AgentRoleConfig.runtime_config_sources` to show per-field inherited/overridden detail and to avoid claiming inheritance from shadowed role files.
- 2026-06-18: role template availability/session-boundary state is now typed in core projection: user templates are `NewSessionsOnly`, active built-ins are `BuiltIn`, and shadowed built-ins are `ShadowedBuiltIn`. TUI copy consumes this state instead of hardcoding hot-reload wording.
- 2026-06-18: native config refresh path now reloads `Config.agent_roles` from the current `ConfigLayerStack` through the existing role loader during `reload_user_config_layer` / `refresh_runtime_config`; the TUI wizard reuses that app-server reload path after successful create and does not add a parallel role registry or new role-template protocol endpoint.
- 2026-06-18: next-foreground-turn schema evidence added: `reload_user_config_layer_updates_spawn_agent_role_schema_for_next_turn` proves a role created in `$CODEX_HOME/agents` appears in model-visible `spawn_agent.agent_type` after native config reload and a new foreground turn.
- 2026-06-18: `role-authoring-v2` starts with a create-only staged action for current model defaults. `Create template from current model` seeds the existing native TOML draft with `model`, `model_provider`, `model_reasoning_effort` and `service_tier` from current `Config`, then uses the existing parser/write/reload path. It is intentionally not a role-local model picker or a provider registry.
