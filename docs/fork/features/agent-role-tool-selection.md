# Agent role-level tool selection

## Feature Passport

- Code name: `agent-role-tool-selection`
- Status: first runtime slice plus runtime catalog/effective-policy diagnostics, read-only role detail/effective-state projection, thread-scoped app-server catalog read contract, `/agent-roles` current-thread catalog reference, catalog-assisted create flow and existing user-role tool allowlist editing implemented in `fork/140`; manual native TOML authoring remains available through the role-template wizard draft, while external app-server write/management API is deferred.
- Goal: дать sub-agent role template нативный способ сузить доступный набор tools, не создавая параллельный config format, tool registry, MCP filter или TUI-only allowlist.
- Scope in: role-applied config contract, effective tool-selection policy, validation/diagnostics, `ToolRouter`/`spec_plan` filtering, `ToolRegistry` dispatch shape, deferred `tool_search`, Code Mode nested tools, MCP direct/deferred tools, extension/dynamic tools, TUI authoring requirements and verification gates.
- Scope out: изменение permission profiles, sandbox/network/cwd semantics, MCP approval policy, provider/model routing, OpenClaude markdown `tools`/`disallowedTools` persistence, hardcoded TUI buckets, silent fallback for invalid static tools.

## Current Status

Первый runtime slice реализован как native config policy: role config file может задать `[tool_selection] allowed_tools = [...]`, `ConfigToml` парсит поле в effective `Config.tool_selection`, а `TurnContext.config` передаёт policy в `codex-rs/core/src/tools/spec_plan.rs`.

V1 intentionally allowlist-only. Отсутствующая секция сохраняет upstream all-tools behavior. Allowlist является subtractive capability boundary: она удаляет tools из model-visible specs, deferred discovery, Code Mode projection и `ToolRegistry`, но не расширяет permissions, sandbox, approval, cwd, MCP или provider/model capabilities.

Реализованный grammar: `name` для plain tools и `namespace/name` для namespaced runtime `ToolName`. Парсер trimming-aware, запрещает blank entries, malformed `namespace/name` and duplicates. Runtime inventory diagnostics реализованы в tool planning: синтаксически корректный id, который не совпал с доступным tool в текущем turn, не даёт доступ ни к одному tool, не создаёт phantom capability и один раз за turn выводится как existing `EventMsg::Warning`. Runtime catalog projection также живёт в planner diagnostics: `ToolSelectionDiagnostics.catalog_entries` derived from planner-owned `PlannedTools`, with selected state and bounded exposure metadata for the current policy. `CodexThread::tool_selection_catalog` and experimental app-server method `agentRole/toolSelectionCatalog/read` expose that projection for a loaded thread without a TUI hardcoded registry.

## User Contract

Пользователь создаёт или редактирует role template и задаёт набор tools, доступных sub-agent, spawned через `spawn_agent.agent_type`. Role без tool-selection fields сохраняет текущий all-native-tools behavior. Role с tool-selection policy сужает capability только для spawned child thread, где эта role применена.

Blocked tool не должен появляться в model-visible specs, deferred discovery, Code Mode nested tool list или executable `ToolRegistry`. Если модель всё же вызывает blocked tool по старому context или handcrafted payload, runtime должен вернуть controlled unsupported-tool diagnostic из native dispatch path, а не выполнять tool.

Tool selection не является permission grant. Если role разрешает shell-like tool, его исполнение всё равно ограничено текущими `PermissionProfile`, approval policy, sandbox, environment/cwd rules и network policy. Если permission profile запрещает действие, role-level selection не может его разрешить.

## Source Of Truth Decision

V1 использует role-applied config layer field, а не `AgentRoleToml` declaration metadata и не TUI-only state. Source of truth:

1. `codex-rs/config/src/config_toml.rs::ConfigToml.tool_selection`
2. `codex-rs/config/src/config_toml.rs::ToolSelectionToml.allowed_tools`
3. `codex-rs/core/src/config/mod.rs::ToolSelectionConfig`
4. `TurnContext.config.tool_selection`
5. `codex-rs/core/src/tools/spec_plan.rs::apply_tool_selection_policy`
6. `codex-rs/core/src/tools/router.rs::ToolSelectionDiagnostics`

Rejected paths remain rejected: `AgentRoleToml` as standalone owner would create a parallel runtime path; `ToolRouterParams` as source of truth would bypass native config/resume; TUI-only filtering would not protect `tool_search`, Code Mode or dispatch. Diagnostics are projection-only and are derived from `PlannedTools`; they are not a second policy source.

## Enforcement Contract

Единственная безопасная точка enforcement находится после добавления native tool sources и до производных projections. В текущем pipeline `codex-rs/core/src/tools/spec_plan.rs::build_tool_specs_and_registry` сначала строит отдельный planner-owned inventory snapshot for `ToolSelectionDiagnostics.catalog_entries`, then enforces the real `planned_tools`: `add_tool_sources`, первый `apply_tool_selection_policy`, `append_tool_search_executor`, `prepend_code_mode_executors`, `ToolSelectionDiagnostics.unmatched_allowed_tools` over the enforced plan, второй `apply_tool_selection_policy`, затем `build_model_visible_specs_and_registry`. Первый фильтр не даёт blocked deferred tools попасть в `tool_search`/Code Mode, второй фильтр не даёт synthetic tools обойти allowlist.

Implemented enforcement surfaces:

- `PlannedTools.runtimes`: удалить blocked runtimes до построения `ToolSearchHandler`, Code Mode executors and `ToolRegistry`.
- `PlannedTools.hosted_specs`: применить тот же policy к hosted specs, чтобы hosted-only tools не обходили runtime filtering.
- Deferred `tool_search`: `append_tool_search_executor` строит index из runtimes with `ToolExposure::Deferred`; blocked deferred runtimes must be absent before this step.
- Code Mode: `build_code_mode_executors` собирает nested tool specs from current executors; blocked runtimes must be absent before this step.
- `ToolRegistry`: `ToolRegistry::from_tools` must receive only allowed runtimes, so direct blocked dispatch naturally fails as unsupported.
- MCP direct/deferred tools: filtering must cover both `mcp_tools` and `deferred_mcp_tools` after native MCP exposure decision, using canonical model-visible `ToolName` and raw server/tool metadata consistently.
- Extension and dynamic tools: filtered by `ToolExecutor::tool_name()` after native source assembly. Stale syntactically valid entries are not included automatically, do not create tools and do not grant access; unmatched entries are reported through `ToolRouter::tool_selection_diagnostics` and existing `EventMsg::Warning` once per turn.

## Permissions Boundary

Role tool selection only removes capabilities from the effective tool set. It must not:

- modify `PermissionProfile` or `AdditionalPermissionProfile`;
- change approval policy or auto-approval behavior;
- widen filesystem, workspace root, sandbox, network or cwd access;
- change MCP approval mode, OAuth state, server startup, elicitation or resource access rules;
- bypass Guardian/hooks/tool lifecycle;
- change provider/model capability detection or `ToolMode`;
- treat Code Mode nested execution as a separate permission domain.

If a tool is allowed by role selection but denied by permission/sandbox/approval policy, the existing security boundary wins. If a tool is denied by role selection but allowed by permission policy, role selection wins for visibility and dispatch.

## OpenClaude Reference

OpenClaude is useful as UX evidence only. `ToolSelector` demonstrates bucket toggles, MCP server grouping, advanced individual tool selection, wildcard/all-tools semantics and stale-tool validation. Codex must not copy OpenClaude markdown/frontmatter persistence; the Codex contract must flow through native role config and `ToolRouter` planning.

Research reference: `docs/fork/research/0.140.0/openclaude-agent-ux-runtime.md` records that Codex has no generic role-level allowlist/denylist and that late filtering would bypass `tool_search`, Code Mode nested tools, hosted tools and `ToolRegistry`.

## Propagation Matrix

| Surface | Native owner | Required behavior |
| --- | --- | --- |
| Role authoring | `$CODEX_HOME/agents/*.toml`, role files parsed by `parse_agent_role_file_contents` | Role tool-selection syntax is native TOML config syntax; old role files without the field keep all-tools default. |
| Config source | `ConfigToml` -> `Config` -> role-applied config layer | Implemented as `[tool_selection] allowed_tools = [...]`; effective policy exists in native config state, not in TUI-only metadata. |
| Spawn application | `spawn_agent.agent_type` -> `apply_role_to_config` | Covered by `agent_role_config_file_applies_tool_selection_allowlist`: role config file rebuilds child config with effective policy. |
| Turn state | `TurnContext` | Implemented through existing `TurnContext.config`; no separate `ToolRouterParams` source of truth. |
| MCP exposure | `build_mcp_tool_exposure`, `ToolInfo::canonical_tool_name`, MCP raw server/tool metadata | Direct and deferred MCP tools are filtered consistently; raw names remain available for protocol calls but model-visible names drive policy matching unless the final grammar says otherwise. |
| Tool planning | `spec_plan::add_tool_sources`, `PlannedTools`, `ToolExposure` | Filter after source assembly and before `tool_search`, Code Mode and registry projections. |
| Model-visible specs | `build_model_visible_specs_and_registry`, `create_tools_json_for_responses_api`, `create_tools_json_for_chat_completions_api` | Only allowed tools appear in request specs for both Responses and Chat Completions transports. |
| Deferred discovery | `ToolSearchHandler`, `ToolSearchInfo` | Blocked deferred tools do not index or appear through `tool_search`. |
| Code Mode | `CodeModeExecuteHandler`, nested `ToolSpec` collection | Blocked tools are absent from nested tool definitions and cannot be invoked through Code Mode runtime delegation. |
| Dispatch | `ToolRegistry::from_tools`, `dispatch_any_with_terminal_outcome` | Blocked tools are not registered; direct calls return native unsupported-tool diagnostics. Covered by dispatch-level test. |
| Runtime diagnostics | `ToolRouter::tool_selection_diagnostics`, `built_tools` warning path | Unmatched allowlist entries are computed from the enforced `PlannedTools`, formatted as `name` or `namespace/name`, emitted once per turn through existing `EventMsg::Warning`, and capped to avoid unbounded warning text. The same diagnostics project bounded planner-owned catalog entries from an inventory snapshot with current-policy selected state and exposure metadata. |
| App-server read contract | `CodexThread::tool_selection_catalog`, `agentRole/toolSelectionCatalog/read`, generated app-server schema/TS | Thread-scoped read method returns planner-owned catalog entries plus `unmatchedAllowedTools` for a loaded thread. It fails fast for invalid/missing `threadId` and does not build a global or config-only catalog. |
| TUI | Role template UI | Read-only role detail projection is implemented through `AgentRoleTemplateLocks.allowed_tool_names`; selected detail states that the configured list is a runtime allowlist, unmatched entries warn during tool planning and grant no access. `/agent-roles` consumes `agentRole/toolSelectionCatalog/read` when a loaded thread is available and renders the current-thread catalog as tool-id evidence, not as a role-specific preview. Authoring writes native TOML through the role-template draft path; the current catalog can seed a searchable bucketed create picker for a new role or a searchable edit picker for an existing valid `$CODEX_HOME/agents/*.toml` user role. Edit picker keeps existing selected tool ids that are absent from the current thread catalog as selected unavailable rows, so accepting the editor does not silently delete valid stale/future/workspace-specific allowlist entries. Both paths open an editable native TOML draft with `[tool_selection] allowed_tools` before writing. External app-server write/management APIs remain deferred. |
| Persistence/resume | Role file + session config rebuild | Missing field is backward-compatible. Syntax errors fail during config load/spawn config rebuild; fresh config restart reload and cold thread resume through the same `$CODEX_HOME/config.toml` are covered. Stronger retention after source config removal/change is not claimed. |
| Role template validation | `parse_agent_role_file_contents` reused by role loading, listing and draft creation | Invalid or duplicate `tool_selection.allowed_tools` entries fail through the native role-file parser path instead of being shown as valid and failing later during spawn/config rebuild. |

## Verification Matrix

| Scenario | Required evidence |
| --- | --- |
| No tool-selection policy | Covered by `tool_selection_missing_policy_keeps_native_tools_unrestricted`. |
| Static allowlist | Covered by `tool_selection_allowlist_filters_visible_specs_and_registry`. |
| Malformed/duplicate entries | Covered by `load_config_rejects_invalid_tool_selection_allowlist_entries`. |
| Role template validation | Covered by `list_agent_role_templates_reports_tool_selection_validation_errors` and `create_user_agent_role_template_from_draft_rejects_invalid_tool_selection`: invalid role TOML `allowed_tools` fails in role listing/create validation, not only at runtime config rebuild. |
| Deferred tools | Covered by `tool_selection_filters_deferred_tools_before_tool_search`. |
| Code Mode | Covered by `tool_selection_filters_code_mode_projection_strictly`. |
| Role config application | Covered by `agent_role_config_file_applies_tool_selection_allowlist`. |
| Hosted specs | Covered by `tool_selection_filters_hosted_specs`. |
| MCP direct and deferred | Covered by `tool_selection_filters_direct_and_deferred_mcp_tools`; deferred discovery requires both the deferred tool and `tool_search` to be allowlisted. |
| Unknown/stale ids | Covered by `tool_selection_unknown_entries_do_not_create_tools`: syntactically valid unmatched ids do not create visible or registered tools. |
| Runtime stale-id warning | Covered by `tool_selection_stale_entries_are_derived_from_available_inventory` and `built_tools_warns_once_for_unavailable_tool_selection_entries`: unmatched ids are derived from current inventory and emitted once as existing warning text. |
| Runtime catalog projection | Available tool identities are projected from the planner-owned inventory snapshot, including direct, deferred, MCP, hosted, hidden, `tool_search` and Code Mode synthetic tools; each entry carries whether the current allowlist selects it and its native exposure class. | Covered by `tool_selection_catalog_projects_effective_runtime_tools`, `tool_selection_catalog_marks_unselected_runtime_discovered_tools`, `tool_selection_missing_policy_keeps_native_tools_unrestricted` and `tool_selection_unknown_entries_do_not_create_tools`. |
| App-server runtime catalog read | Loaded-thread app-server request exposes the same core catalog projection and stale-entry diagnostics without a TUI hardcoded registry. | Covered by `read_tool_selection_catalog_uses_loaded_thread_policy`; generated schema consistency covered by `schema_fixtures_match_generated`; wire name covered by `serialize_agent_role_tool_selection_catalog_read`. |
| Synthetic tool effective diagnostics | Synthetic tools selected in the catalog but removed from the final enforced plan warn as unmatched instead of being treated as effectively available. | Covered by `tool_selection_warns_when_selected_tool_search_has_no_effective_index`. |
| Permissions/native availability | Covered by `tool_selection_allowlist_does_not_enable_unavailable_tools`: allowlisting an environment-backed tool does not make it available when native environment gating removes it. |
| Dispatch security boundary | Covered by `tool_selection_blocked_tool_dispatch_uses_native_unsupported_path`: a blocked direct tool call returns native `unsupported call: ...` and does not execute the handler. |
| Restart reload | Covered by `agent_role_tool_selection_survives_config_restart_reload`: a fresh config load from the same `$CODEX_HOME` reapplies role tool selection through the native role config path. |
| Cold thread resume | Covered by `resume_retains_tool_selection_policy_in_resumed_turn`: a resumed thread reloads the same `$CODEX_HOME/config.toml` and sends only the allowlisted tools in the resumed request. |
| TUI role detail | Read-only configured allowlist and effective-state copy are covered by `agent_role_templates_popup_snapshot`. Loaded current-thread runtime catalog reference is covered by `agent_role_templates_popup_runtime_catalog_snapshot`. |
| TUI authoring | Native TOML draft creation and update are implemented through the role-template wizard path; a catalog-assisted create picker can generate an editable draft from selected current-thread tool ids, and an existing valid `$CODEX_HOME/agents/*.toml` user role can preselect its current allowlist and update the same native TOML file. External app-server write/management APIs remain deferred until their ownership/write contract is defined. |

## Remaining Gaps

- Merge semantics documentation for multiple config layers beyond the current effective `ConfigToml` merge behavior.
- Stronger persisted policy retention after source config removal/change remains a future contract question. Fresh config restart reload and cold resume through the same `$CODEX_HOME/config.toml` are covered.
- Direct sandbox/approval negative around an allowed-but-security-denied tool, if this feature later changes security-policy surfaces. Current v1 does not alter those surfaces and has a dispatch-level blocked-tool test.
- App-server write/management API, if external clients need to change this policy outside the local native TOML role-file flow.
- Audit/logging: whether blocked direct calls should log a role-policy-specific reason or rely on existing unsupported-tool text.

## Next Implementation Gate

Do not add TUI authoring controls by inventing a parallel tool registry. The core runtime catalog projection exists in `ToolSelectionDiagnostics`, the app-server read path exists through `agentRole/toolSelectionCatalog/read`, and `/agent-roles` consumes that path only as current-thread tool-id evidence. Authoring must still write native `[tool_selection] allowed_tools` through parser-validated TOML and must not treat the current-thread catalog as a role-specific preview. Existing-role editing is limited to valid discovered user role files under `$CODEX_HOME/agents/*.toml`; external `config_file` roles should stay on the open-file path until a separate write ownership contract exists.

## Doc Changelog

- 2026-06-18: rewritten as native Codex contract; implementation explicitly blocked until source-of-truth, identifier grammar and pre-projection enforcement design are settled.
- 2026-06-18: first runtime slice implemented locally: `[tool_selection] allowed_tools` flows through `ConfigToml`/`Config`/`TurnContext` into `spec_plan` before `tool_search`, Code Mode and registry construction; focused `codex-core` tests pass, including MCP direct/deferred coverage.
- 2026-06-18: role template detail now shows declared `[tool_selection] allowed_tools` read-only via native role TOML projection; the role-template wizard starter draft includes a commented native TOML example. At that checkpoint bucketed editing controls were still deferred.
- 2026-06-18: role-template TOML draft authoring coverage expanded: creation tests prove a parser-validated draft containing `[tool_selection] allowed_tools` is written to `$CODEX_HOME/agents/<role>.toml`. At that checkpoint catalog-driven picker/editor controls were still deferred.
- 2026-06-18: dispatch-level regression added: blocked tools are absent from `ToolRegistry` and direct dispatch returns native unsupported-tool diagnostics without executing the tool.
- 2026-06-18: stale-id diagnostics implemented through `ToolRouter::tool_selection_diagnostics` and existing `EventMsg::Warning`; focused `codex-core` tool-selection/session tests pass 15/15.
- 2026-06-18: cold resume coverage added through `resume_retains_tool_selection_policy_in_resumed_turn`; resumed request tools stay limited to the configured allowlist when using the same `$CODEX_HOME/config.toml`.
- 2026-06-18: existing user-role tool selection editing implemented through the native TOML path: `/agent-roles` offers `Edit tools for <role>` for valid `$CODEX_HOME/agents/*.toml` user roles when a runtime catalog is loaded, preselects the role's current allowlist in the searchable picker, opens an editable TOML draft, validates through the native role-file parser and overwrites the same role file before triggering native config reload.
- 2026-06-18: role template detail now includes read-only effective-state copy for configured tool selection: the list is a runtime allowlist, unmatched entries warn during native tool planning and grant no access. At that checkpoint authoring and runtime-discovered catalog display were still deferred.
- 2026-06-18: role-file parser validation now reuses the native `tool_selection.allowed_tools` validator, so invalid or duplicate allowlist entries are reported during role template listing and TOML draft creation before spawn/config rebuild. `just fmt`, `just test -p codex-core agent_role_templates` passed 13/13 and `just test -p codex-core tool_selection` passed 18/18.
- 2026-06-18: core planner diagnostics now include `ToolSelectionDiagnostics.catalog_entries`, a runtime catalog projection with selected state and exposure metadata derived from a planner-owned inventory snapshot rather than TUI hardcoded lists; unmatched diagnostics are checked against the enforced final plan. `just fmt` and `just test -p codex-core tool_selection` passed 21/21.
- 2026-06-18: added loaded-thread app-server read contract `agentRole/toolSelectionCatalog/read` backed by `CodexThread::tool_selection_catalog`, generated schema/TS artifacts and integration coverage. `just fmt`, `just write-app-server-schema`, `just test -p codex-app-server-protocol serialize_agent_role_tool_selection_catalog_read`, `just test -p codex-app-server-protocol schema_fixtures_match_generated`, `just test -p codex-core tool_selection` and `just test -p codex-app-server read_tool_selection_catalog_uses_loaded_thread_policy` passed.
- 2026-06-18: `/agent-roles` now calls the loaded-thread app-server catalog read path and renders the current-thread runtime catalog as read-only tool-id evidence, with explicit copy that it is not a role-specific preview. `just fmt` and `just test -p codex-tui agent_role_templates_popup_snapshot agent_role_templates_popup_runtime_catalog_snapshot agent_role_template_create_prompt_snapshot agent_role_template_create_prompt_submits_native_toml_draft agent_role_template_create_from_draft_updates_session_catalog` passed 5/5 after accepting the intentional snapshots through `cargo insta accept`.
- 2026-06-18: catalog-assisted create flow added: when a loaded runtime catalog is available, `/agent-roles` can open a searchable bucketed picker grouped by native `AgentRoleToolSelectionCatalogExposure`, then open the existing editable native TOML draft with selected `[tool_selection] allowed_tools`. The saved source remains `$CODEX_HOME/agents/*.toml`; at that checkpoint existing-role in-place editing and app-server write/management APIs were still deferred.
- 2026-06-18: audit fix: existing-role edit picker now preserves selected allowed tool ids that are not present in the current loaded-thread catalog by showing them as unavailable selected rows. This keeps valid future/deferred/workspace-specific allowlist entries unless the user explicitly removes them.
