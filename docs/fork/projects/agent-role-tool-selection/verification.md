# Agent Role-Level Tool Selection Verification

## Verification Scope

This document tracks the implemented v1 runtime slice for role-level tool selection plus core runtime catalog/effective-policy diagnostics, the read-only role detail/effective-state projection, the loaded-thread app-server catalog read contract, the `/agent-roles` current-thread runtime catalog reference, parser-validated native TOML draft authoring, catalog-assisted create flow and existing user-role allowlist editing. Production-ready v2 extends the same native path with `denied_tools`: deny is enforced after allow and wins on conflicts. The current scope is native config parsing, role config application, planner enforcement before `tool_search`/Code Mode/registry construction, runtime stale-id diagnostics/catalog projection, generated app-server protocol artifacts, displaying declared allowlist/denylist entries in the role template detail surface, showing loaded-thread catalog entries as read-only evidence, proving that the role-template TOML wizard can create or update a role file containing `[tool_selection] allowed_tools` either manually or from selected current-thread catalog ids, and exposing external whole-policy app-server writes for discovered user role TOML files.

## Implemented Checks

| Surface | Required check | Candidate command |
| --- | --- | --- |
| Config parse/schema | Valid `[tool_selection] allowed_tools` and `denied_tools` parse into canonical `ToolName`; malformed/duplicate entries fail fast; generated schema includes both fields. | `just write-config-schema`; `just test -p codex-core tool_selection`. |
| Role application | Role `config_file` applies policy through `apply_role_to_config`, not via `AgentRoleToml` or TUI metadata. | `just test -p codex-core tool_selection`. |
| Tool planning | Filter runs after source assembly and before `tool_search`, Code Mode and registry construction; a second pass keeps synthetic Code Mode tools strict. | `just test -p codex-core tool_selection`. |
| Model-visible specs | Blocked direct tools are absent from model-visible specs. | `tool_selection_allowlist_filters_visible_specs_and_registry`. |
| Dispatch registry | Blocked direct tools are absent from `ToolRegistry`, so native unsupported-tool dispatch path remains authoritative. | `tool_selection_allowlist_filters_visible_specs_and_registry`. |
| Dispatch rejection | A blocked direct tool call returns native unsupported-tool diagnostics and does not execute the handler. | `tool_selection_blocked_tool_dispatch_uses_native_unsupported_path`. |
| Denylist conflict | A tool listed in both `allowed_tools` and `denied_tools` is denied; deny wins without changing permission/sandbox semantics. | `tool_selection_denylist_wins_over_allowlist`. |
| Denylist default path | With only `denied_tools`, unrelated native tools remain available and denied tools are removed from model-visible specs and dispatch registry. | `tool_selection_denylist_filters_visible_specs_and_registry`. |
| Deferred `tool_search` | Blocked deferred extension tools are absent before `tool_search` is appended. | `tool_selection_filters_deferred_tools_before_tool_search`. |
| MCP direct/deferred | Direct MCP tools use canonical namespaced `ToolName`; deferred MCP tools require both the deferred tool and `tool_search` in the allowlist. | `tool_selection_filters_direct_and_deferred_mcp_tools`. |
| Code Mode | Nested tool remains registered only when allowed; `exec`/`wait` must also be explicitly allowlisted to expose Code Mode. | `tool_selection_filters_code_mode_projection_strictly`. |
| Hosted tools | Hosted specs are filtered by plain hosted tool name. | `tool_selection_filters_hosted_specs`. |
| Default upstream path | Missing policy keeps native tools unrestricted. | `tool_selection_missing_policy_keeps_native_tools_unrestricted`. |
| Unknown/stale ids | Syntactically valid unmatched ids do not create model-visible or registered tools. | `tool_selection_unknown_entries_do_not_create_tools`. |
| Direct app-server MCP execution | `mcpServer/tool/call` resolves the loaded thread's canonical MCP tool name through native MCP inventory and enforces `allowed_tools`/`denied_tools` before executing; policy denial uses JSON-RPC invalid request `-32600`, not internal error. | `mcp_server_tool_call_respects_tool_selection_denylist`; `mcp_server_tool_call_respects_tool_selection_allowlist`; `mcp_server_tool_call_allows_canonical_tool_selection_entry`. |
| Stale-id diagnostics | Unmatched ids are derived from current `PlannedTools`, include hosted/namespaced formatting, and emit one bounded existing warning per turn. | `tool_selection_stale_entries_are_derived_from_available_inventory`; `built_tools_warns_once_for_unavailable_tool_selection_entries`. |
| Runtime catalog projection | Tool identities are derived from a planner-owned inventory snapshot and include direct, deferred, MCP, hosted, hidden, `tool_search` and Code Mode synthetic tools with selected state and exposure metadata for the current policy, without a TUI hardcoded registry. Denied tools project as `selected=false`. | `tool_selection_catalog_projects_effective_runtime_tools`; `tool_selection_catalog_marks_unselected_runtime_discovered_tools`; `tool_selection_catalog_marks_denied_tools_unselected`; `tool_selection_missing_policy_keeps_native_tools_unrestricted`; `tool_selection_unknown_entries_do_not_create_tools`. |
| App-server catalog read | `agentRole/toolSelectionCatalog/read` reads the loaded thread's native catalog projection through `CodexThread::tool_selection_catalog`, not a global or config-only registry. | `read_tool_selection_catalog_uses_loaded_thread_policy`; `serialize_agent_role_tool_selection_catalog_read`; `schema_fixtures_match_generated`. |
| App-server whole-policy write | Experimental `agentRole/toolSelection/set` rewrites only `[tool_selection]` in an existing discovered user role TOML file, rejects external `config_file` roles, distinguishes strict empty allowlist from null/absent section removal, validates through native parser and requires loaded config refresh before success. Stable checked-in schema fixtures do not claim this experimental method. | `agent_role_tool_selection_set_updates_user_role_file`; `agent_role_tool_selection_set_rejects_external_config_file`; `agent_role_tool_selection_set_requires_experimental_api_capability`; `serialize_agent_role_tool_selection_set`. |
| Synthetic effective diagnostics | A selected synthetic tool that is present in the inventory catalog but absent from the enforced final plan is reported as unmatched. | `tool_selection_warns_when_selected_tool_search_has_no_effective_index`. |
| Native availability boundary | Allowlisting an environment-backed tool does not bypass native availability gating when no environment exists. | `tool_selection_allowlist_does_not_enable_unavailable_tools`. |
| Restart/config reload | A fresh config load from the same `$CODEX_HOME` can resolve the role and reapply the same tool-selection policy through `apply_role_to_config`. | `agent_role_tool_selection_survives_config_restart_reload`. |
| Cold resume | A resumed thread reloads the same `$CODEX_HOME/config.toml` and keeps the effective tool-selection policy in the resumed request tools. | `resume_retains_tool_selection_policy_in_resumed_turn`. |
| Source role changed or removed after spawn | A path-backed sub-agent keeps its original effective `allowed_tools`/`denied_tools` after cold resume and retry, using `ThreadSpawn.tool_selection` rather than the current source role TOML. | `resume_agent_from_rollout_uses_edge_tool_selection_when_source_config_changed`; `thread_manager_agent_retry_spawns_fresh_sibling_from_initial_task`. |
| Role templates regression | Existing role template listing/create tests still pass with the new ConfigToml field, and role detail projection includes `denied_tools` provenance. | `just test -p codex-core agent_role_templates`; `list_agent_role_templates_reports_model_provider_locks`. |
| TUI role detail projection | Declared `[tool_selection] allowed_tools` and `denied_tools` appear in the role template selected detail with read-only effective-state copy, without adding a parallel editor/profile schema. | `just test -p codex-tui agent_role_templates_popup_snapshot`; `role_template_detail_renders_allowlist_and_denylist_separately`. |
| TUI runtime catalog reference | `/agent-roles` renders `agentRole/toolSelectionCatalog/read` output as current-thread tool-id evidence with exposure labels and unmatched entries, and states that it is not a role-specific preview. | `just test -p codex-tui agent_role_templates_popup_runtime_catalog_snapshot`. |
| Role-template parser validation | Invalid or duplicate `[tool_selection] allowed_tools` entries in a role TOML file are reported by native role template listing and draft creation before spawn/config rebuild. | `just test -p codex-core agent_role_templates`. |
| Native TOML draft authoring | The `/agent-roles` create flow can write a parser-validated `$CODEX_HOME/agents/<role>.toml` draft that includes `[tool_selection] allowed_tools`. | `create_user_agent_role_template_from_draft_writes_valid_native_toml`; `agent_role_template_create_from_draft_updates_session_catalog`. |
| Catalog-assisted create | A loaded current-thread catalog can open a searchable picker, grouped by native exposure class, and selected ids seed an editable native TOML draft. | `starter_agent_role_template_draft_with_allowed_tools_writes_native_tool_selection`; `agent_role_template_tool_selection_picker_submits_selected_tools`; `agent_role_template_create_prompt_with_allowed_tools_submits_native_toml_draft`; `agent_role_templates_popup_runtime_catalog_snapshot`. |
| Existing user-role allow/deny edit | A valid discovered `$CODEX_HOME/agents/*.toml` user role can open searchable pickers with its current allowlist or denylist preselected, then submit an editable update draft that overwrites the same native role file. Deny editing preserves the existing allowlist. | `user_agent_role_template_update_draft_rewrites_native_tool_selection`; `user_agent_role_template_update_draft_rewrites_native_denied_tools`; `agent_role_template_tool_selection_picker_for_role_submits_selected_tools`; `agent_role_template_denied_tool_selection_picker_for_role_submits_selected_tools`; `agent_role_template_edit_prompt_with_allowed_tools_submits_native_toml_draft`; `agent_role_template_edit_prompt_with_denied_tools_submits_native_toml_draft`; `agent_role_template_update_from_draft_updates_session_catalog`; `agent_role_templates_popup_runtime_catalog_snapshot`. |

## Acceptance Criteria

Current v1 acceptance:

- There is exactly one effective role tool-selection source of truth: `ConfigToml`/`Config`/`TurnContext.config`.
- Old roles without `[tool_selection]` behave as before.
- `denied_tools` is subtractive only and wins over `allowed_tools` on conflicts.
- Blocked direct, deferred, hosted and Code Mode-projected tools are absent from the relevant projections.
- Allowing a tool does not widen permission/sandbox/network/approval/MCP policy because policy is purely subtractive over the tool plan; focused tests prove unknown ids and environment-unavailable tools do not become available.
- Blocked direct dispatch uses the existing unsupported-tool error path.
- Blocked external `mcpServer/tool/call` requests fail with JSON-RPC invalid request `-32600` before the MCP manager executes the raw server/tool call; canonical allowlisted MCP calls still succeed.
- Generated config schema is updated.
- App-server read protocol is generated and schema-tested for `agentRole/toolSelectionCatalog/read`.
- App-server write protocol is covered by Rust protocol serialization and experimental capability gating for `agentRole/toolSelection/set`; default checked-in stable schema fixtures intentionally do not include this experimental method.
- External writes target only discovered user role files and do not create a parallel config registry.
- Fresh config restart/reload keeps the role-applied tool-selection policy.
- Cold thread resume through the same `$CODEX_HOME/config.toml` keeps the effective tool-selection policy.

## Current Known Gaps

- Stronger persisted policy retention is implemented and covered for new path-backed sub-agents through `ThreadSpawn.tool_selection`; older sessions without the snapshot keep the previous reload behavior.
- Direct sandbox/approval negative remains a possible future test if this feature starts changing security-policy surfaces; blocked-tool dispatch and native availability no-grant behavior are covered.
- External app-server write/management surfaces are limited to discovered user role files. External `config_file` roles remain on the open-file path until ownership/write semantics are defined, and live mutation of already materialized in-flight child role layers remains outside the current contract.

## Verification Log

2026-06-18:

```bash
just fmt
just test -p codex-core tool_selection
just write-config-schema
just test -p codex-core agent_role_templates
just test -p codex-app-server read_tool_selection_catalog_uses_loaded_thread_policy
git diff --check -- codex-rs/config/src/config_toml.rs codex-rs/core/src/config/mod.rs codex-rs/core/src/config/config_tests.rs codex-rs/core/src/tools/spec_plan.rs codex-rs/core/src/tools/spec_plan_tests.rs codex-rs/core/config.schema.json docs/fork/features/agent-role-tool-selection.md docs/fork/projects/agent-role-tool-selection/design.md docs/fork/projects/agent-role-tool-selection/verification.md docs/fork/features/subagent-policy-and-ownership.md docs/fork/projects/subagent-policy-and-ownership/design.md docs/fork/projects/fork-140-production-ready
```

Results: `tool_selection` 26/26 passed, including denylist-only, deny-wins-over-allow, denied catalog projection and MCP direct deny coverage. `agent_role_templates` 19/19 passed. App-server loaded-thread catalog read 1/1 passed. `write-config-schema` updated `denied_tools` in the generated config schema. `git diff --check` passed for the touched slice.

### 2026-06-18 - Role-template denylist projection and read-only permission check

Commands:

```bash
just test -p codex-core apply_role_sets_native_read_only_permission_profile list_agent_role_templates_reports_model_provider_locks
just test -p codex-tui agent_role_template_update_from_draft_updates_session_catalog role_template_detail_renders_allowlist_and_denylist_separately
cargo insta pending-snapshots --workspace-root .
```

Results: delegated spark test-runner PASS. Core focused run passed 2/2, proving role-applied `default_permissions = ":read-only"` resolves to native `PermissionProfile::read_only()` and role-template projection includes `tool_selection.denied_tools`. TUI focused run passed 2/2, proving selected detail renders allowlist/denylist separately and existing role update still writes native TOML. No pending snapshots.

### 2026-06-18 - Direct app-server MCP execution policy gate

Commands:

```bash
just fmt
just test -p codex-app-server mcp_server_tool_call_respects_tool_selection_denylist mcp_server_tool_call_respects_tool_selection_allowlist mcp_server_tool_call_allows_canonical_tool_selection_entry
just test -p codex-core tool_selection_filters_direct_and_deferred_mcp_tools
cargo insta pending-snapshots --workspace-root .
git diff --check -- codex-rs/core/src/config/mod.rs codex-rs/core/src/tools/spec_plan.rs codex-rs/core/src/codex_thread.rs codex-rs/core/src/session/mcp.rs codex-rs/app-server/tests/suite/v2/mcp_tool.rs docs/fork/features/agent-role-tool-selection.md docs/fork/projects/agent-role-tool-selection/design.md docs/fork/projects/agent-role-tool-selection/verification.md docs/fork/features/subagent-policy-and-ownership.md docs/fork/projects/subagent-policy-and-ownership/design.md docs/fork/projects/subagent-policy-and-ownership/verification.md docs/fork/projects/fork-140-production-ready/README.md docs/fork/projects/fork-140-production-ready/verification.md
```

Results: delegated spark test-runner PASS after fixing the diagnostic formatter to use TOML-style `namespace/name`. App-server direct MCP execution tests passed 3/3, proving `mcpServer/tool/call` respects `tool_selection.denied_tools`, respects restrictive `allowed_tools`, and still allows the canonical `mcp__tool_server/echo_tool` entry. Core MCP planner regression `tool_selection_filters_direct_and_deferred_mcp_tools` passed 1/1. No pending snapshots; scoped `git diff --check` passed.

### 2026-06-19 - External app-server whole-policy write API

Commands:

```bash
just fmt
just test -p codex-core user_agent_role_template_update_tool_selection_rewrites_allowlist_and_denylist user_agent_role_template_update_tool_selection_removes_section_for_null_policy user_agent_role_template_update_draft_rewrites_native_tool_selection
just test -p codex-app-server-protocol serialize_agent_role_tool_selection_set serialize_agent_role_tool_selection_catalog_read
just write-app-server-schema
just test -p codex-app-server agent_role_tool_selection_set_updates_user_role_file agent_role_tool_selection_set_rejects_external_config_file read_tool_selection_catalog_uses_loaded_thread_policy
just test -p codex-app-server-protocol schema_fixtures_match_generated
just test -p codex-app-server agent_role_tool_selection_set_requires_experimental_api_capability
```

Results: core role-template helper tests passed 3/3, proving whole-policy allow/deny replacement, null-policy section removal and preservation of existing `denied_tools` when the legacy allowlist-only TUI helper rewrites a user role. Protocol serialization passed 2/2, generated app-server schema fixtures passed 2/2 for stable checked-in protocol surfaces, app-server integration passed 3/3 for positive discovered-user-role write, external `config_file` rejection and existing catalog read regression, and experimental capability gate passed 1/1. The write API is limited to `$CODEX_HOME/agents/*.toml` discovered user roles and refreshes loaded config after native parser validation; the default stable schema fixtures do not claim the experimental write method.

### 2026-06-19 staged allow/deny TUI authoring follow-up

Commands:

```bash
just test -p codex-core user_agent_role_template_update_draft_rewrites_native_denied_tools user_agent_role_template_update_draft_rewrites_native_tool_selection
just test -p codex-tui agent_role_template_denied_tool_selection_picker_for_role_submits_selected_tools agent_role_template_edit_prompt_with_denied_tools_submits_native_toml_draft agent_role_template_tool_selection_picker_for_role_submits_selected_tools agent_role_templates_popup_runtime_catalog_snapshot
cargo insta accept --snapshot 'tui/src/chatwidget/snapshots/codex_tui__chatwidget__tests__agent_role_templates_popup_runtime_catalog.snap'
cargo insta pending-snapshots --workspace-root .
```

Results: core focused run passed 2/2, proving the new denied-tools draft helper preserves the existing allowlist while replacing `denied_tools`. TUI focused run passed 4/4 after accepting the intentional runtime-catalog popup update, proving discovered user roles now expose separate `Edit allowed tools` and `Edit denied tools` staged controls over the same current-thread catalog evidence and parser-validated TOML draft path. `cargo insta pending-snapshots --workspace-root .` reported no pending snapshots.

### 2026-06-19 - Path-backed sub-agent tool-selection retention

Commands:

```bash
just fmt
just test -p codex-core thread_manager_agent_retry_spawns_fresh_sibling_from_initial_task resume_agent_from_rollout_uses_edge_tool_selection_when_source_config_changed
just write-app-server-schema
just test -p codex-app-server-protocol schema_fixtures_match_generated
cargo check --tests -p codex-protocol -p codex-thread-store -p codex-core -p codex-app-server -p codex-app-server-protocol
```

Results: `cargo check -p codex-core --tests` passed before the focused run. Core focused tests passed 2/2 after fixing the runtime path: retry now copies `ThreadSpawn.tool_selection` into the fresh sibling, and cold tree resume applies the persisted snapshot to the child config instead of using the current source config. `just write-app-server-schema` completed and app-server protocol schema fixtures passed 2/2, covering the generated protocol shape for `ThreadSpawn.tool_selection`. Multi-crate compile check passed after updating app-server/thread metadata fixtures that reconstruct `ThreadSpawn`.

### 2026-06-19 - Current-state retention rerun

Commands:

```bash
just test -p codex-core thread_manager_agent_retry_spawns_fresh_sibling_from_initial_task resume_agent_from_rollout_uses_edge_tool_selection_when_source_config_changed
```

Results: 2/2 passed in the current `fork/140` dirty-set. This confirms the retention requirement is already satisfied by `ThreadSpawn.tool_selection`: retry copies the effective snapshot to the fresh sibling, and cold resume applies the stored edge snapshot instead of widening to the current source role/config.

### 2026-06-19 - Direct MCP denial error contract

Commands:

```bash
just fmt
just test -p codex-app-server mcp_server_tool_call_respects_tool_selection_denylist mcp_server_tool_call_respects_tool_selection_allowlist mcp_server_tool_call_allows_canonical_tool_selection_entry
```

Results: 3/3 passed. The two negative tests now assert JSON-RPC error code `-32600` for `tool_selection.denied_tools` and restrictive `allowed_tools` denials, while the canonical allowlisted MCP tool still executes. This closes the audit gap where policy denial could be reported to external app-server clients as `internal_error`.

### 2026-06-19 - Agent-role write reload contract

Commands:

```bash
just fmt
just test -p codex-app-server agent_role_tool_selection_set_updates_user_role_file agent_role_tool_selection_set_rejects_external_config_file agent_role_tool_selection_set_requires_experimental_api_capability agent_role_action_policy_set_updates_user_role_file agent_role_action_policy_set_rejects_external_config_file agent_role_action_policy_set_requires_experimental_api_capability
just test -p codex-app-server-protocol serialize_agent_role_tool_selection_set serialize_agent_role_action_policy_set
```

Results: app-server 6/6 passed and app-server-protocol serialization 2/2 passed. The shared agent-role write helper now treats config reload and loaded-thread refresh as part of the success contract for both tool-selection and action-policy writes; reload failure is returned as a controlled app-server error instead of a warning-only fallback. The tool-selection positive write test also covers strict empty allowlist (`allowedTools=[]`) and null clearing (`allowedTools=null`, `deniedTools=null`) as distinct whole-policy outcomes. The experimental wire methods are covered by Rust protocol serialization; default stable schema fixtures intentionally exclude experimental methods.

### 2026-06-19 - Documentation consistency rerun for production-ready claims

Commands:

```bash
just test -p codex-core tool_selection_denylist_wins_over_allowlist tool_selection_denylist_filters_visible_specs_and_registry tool_selection_catalog_marks_denied_tools_unselected thread_manager_agent_retry_spawns_fresh_sibling_from_initial_task resume_agent_from_rollout_uses_edge_tool_selection_when_source_config_changed load_config_parses_subagent_action_policy load_config_rejects_duplicate_subagent_action_policy_entries thread_manager_workbench_action_policy_enforces_allow_and_deny
```

Results: 8/8 passed. This rerun backs the updated project README and production-ready verification wording: denylist behavior, persisted tool-selection retention, root-level action-policy parsing and explicit workbench action allow/deny are implemented and not merely planned.

Additional regression:

```bash
just test -p codex-core agent_role_templates
```

Results: 21/21 passed, covering the broader role-template listing, creation, validation, config-file origin and update paths after adding whole-policy tool-selection writes.

2026-06-18:

```bash
just fmt
just write-config-schema
just test -p codex-core tool_selection
just test -p codex-core agent_role_templates
just test -p codex-tui agent_role_templates_popup_snapshot
```

Results: `tool_selection` 13/13 passed; `agent_role_templates` 9/9 passed; `agent_role_templates_popup_snapshot` 1/1 passed.

2026-06-18:

```bash
just fmt
just test -p codex-core tool_selection built_tools_warns_once_for_unavailable_tool_selection_entries
```

Results: 15/15 passed. Added evidence for `ToolRouter` stale-id diagnostics and once-per-turn user-visible warning via existing `EventMsg::Warning`.

2026-06-18:

```bash
just fmt
just test -p codex-core resume_retains_tool_selection_policy_in_resumed_turn
```

2026-06-18:

```bash
just fmt
just test -p codex-tui agent_role_templates_popup_snapshot
```

Results: 1/1 passed after accepting the intentional snapshot line for read-only tool-selection effective-state copy.

Results: 1/1 passed. Added cold resume evidence: a thread resumed from rollout with the same `$CODEX_HOME/config.toml` still sends only the allowlisted `update_plan` tool in the resumed request.

2026-06-18:

```bash
just fmt
just test -p codex-core agent_role_templates
just test -p codex-core tool_selection
```

Results: `agent_role_templates` 13/13 passed; `tool_selection` 18/18 passed. Added role-template parser validation evidence for `[tool_selection] allowed_tools`: duplicate entries in file-backed role listing and blank entries in TOML draft creation now fail through the native role-file parser path before spawn/config rebuild.

2026-06-18:

```bash
just fmt
just test -p codex-core tool_selection
```

Results: `tool_selection` 21/21 passed. Added core runtime catalog projection evidence: `ToolSelectionDiagnostics.catalog_entries` is derived from a planner-owned inventory snapshot and covers direct, deferred, MCP, hosted, hidden, `tool_search` and Code Mode synthetic tool identities with selected state and exposure metadata for the current policy; unmatched diagnostics are derived from the enforced final plan.

2026-06-18:

```bash
just fmt
just write-app-server-schema
just test -p codex-app-server-protocol serialize_agent_role_tool_selection_catalog_read
just test -p codex-app-server-protocol schema_fixtures_match_generated
just test -p codex-core tool_selection
just test -p codex-app-server read_tool_selection_catalog_uses_loaded_thread_policy
```

Results: protocol serialization 1/1 passed; schema fixtures 2/2 passed; `tool_selection` 21/21 passed; app-server loaded-thread read contract 1/1 passed. Added app-server evidence that `agentRole/toolSelectionCatalog/read` returns selected `update_plan` and unmatched stale entries from the native loaded-thread policy.

2026-06-18:

```bash
just fmt
just test -p codex-core create_user_agent_role_template_from_draft_writes_valid_native_toml create_user_agent_role_template_from_draft_rejects_invalid_tool_selection
just test -p codex-tui agent_role_template_create_from_draft_updates_session_catalog agent_role_template_create_prompt_submits_native_toml_draft
git diff --check -- codex-rs/core/src/agent_role_templates_tests.rs codex-rs/tui/src/chatwidget/tests/popups_and_settings.rs docs/fork/features/agent-role-tool-selection.md docs/fork/projects/agent-role-tool-selection/verification.md docs/fork/research/0.140.0/openclaude-agent-ux-runtime-backlog.md docs/fork/projects/subagent-workbench/verification.md
```

Results: focused core role-template draft validation passed 2/2; focused TUI role-template create flow passed 2/2; diff check passed. Added evidence that parser-validated TOML draft authoring can create a role file with `[tool_selection] allowed_tools`; at that checkpoint catalog-driven editing was still deferred.

2026-06-18:

```bash
just fmt
just test -p codex-tui agent_role_templates_popup_snapshot agent_role_templates_popup_runtime_catalog_snapshot agent_role_template_create_prompt_snapshot agent_role_template_create_prompt_submits_native_toml_draft agent_role_template_create_from_draft_updates_session_catalog
```

Results: TUI role-template/runtime-catalog focused tests 5/5 passed after reviewing and accepting the intentional `agent_role_templates_popup` and `agent_role_templates_popup_runtime_catalog` snapshots with `cargo insta accept`. Added evidence that `/agent-roles` shows loaded-thread catalog entries only as current-thread tool-id reference and keeps TOML authoring on the native draft path.

2026-06-18:

```bash
just test -p codex-core tool_selection_warns_when_selected_tool_search_has_no_effective_index tool_selection_catalog_marks_unselected_runtime_discovered_tools tool_selection_catalog_projects_effective_runtime_tools tool_selection_stale_entries_are_derived_from_available_inventory
git diff --check -- codex-rs/core/src/tools/spec_plan.rs codex-rs/core/src/tools/spec_plan_tests.rs codex-rs/core/src/tools/router.rs docs/fork/features/agent-role-tool-selection.md docs/fork/projects/agent-role-tool-selection/README.md docs/fork/projects/agent-role-tool-selection/design.md docs/fork/projects/agent-role-tool-selection/verification.md
```

Results: delegated spark test-runner PASS. Reconfirmed that `ToolSelectionDiagnostics.catalog_entries` remains a read-only planner inventory snapshot, while `unmatched_allowed_tools` is derived from the enforced final plan and catches selected synthetic tools that cannot exist in the effective runtime plan.

2026-06-18:

```bash
just fmt
just test -p codex-core starter_agent_role_template_draft_with_allowed_tools_writes_native_tool_selection create_user_agent_role_template_from_draft_writes_valid_native_toml create_user_agent_role_template_from_draft_rejects_invalid_tool_selection
just test -p codex-tui agent_role_templates_popup_runtime_catalog_snapshot agent_role_template_create_prompt_with_allowed_tools_submits_native_toml_draft agent_role_template_tool_selection_picker_submits_selected_tools agent_role_template_create_prompt_submits_native_toml_draft agent_role_template_create_from_draft_updates_session_catalog
cargo insta accept --workspace-root . --snapshot 'tui/src/chatwidget/snapshots/codex_tui__chatwidget__tests__agent_role_templates_popup_runtime_catalog.snap'
```

Results: first focused run passed core 3/3 and TUI 4/5; the only TUI failure was the intentional `agent_role_templates_popup_runtime_catalog` snapshot update for the new `Create template from current tools` row. Snapshot was reviewed and accepted because it preserves the selected role detail and adds only the catalog-assisted create entry plus expected width wrapping.

2026-06-18:

```bash
just test -p codex-core starter_agent_role_template_draft_with_allowed_tools_writes_native_tool_selection create_user_agent_role_template_from_draft_writes_valid_native_toml create_user_agent_role_template_from_draft_rejects_invalid_tool_selection
just test -p codex-tui agent_role_templates_popup_runtime_catalog_snapshot agent_role_template_create_prompt_with_allowed_tools_submits_native_toml_draft agent_role_template_tool_selection_picker_submits_selected_tools agent_role_template_create_prompt_submits_native_toml_draft agent_role_template_create_from_draft_updates_session_catalog
cargo insta pending-snapshots --workspace-root .
git diff --check -- codex-rs/core/src/agent_role_templates.rs codex-rs/core/src/agent_role_templates_tests.rs codex-rs/tui/src/app_event.rs codex-rs/tui/src/app/event_dispatch.rs codex-rs/tui/src/bottom_pane/mod.rs codex-rs/tui/src/chatwidget/agent_role_templates.rs codex-rs/tui/src/chatwidget/tests/popups_and_settings.rs codex-rs/tui/src/chatwidget/snapshots/codex_tui__chatwidget__tests__agent_role_templates_popup_runtime_catalog.snap docs/fork/features/agent-role-tool-selection.md docs/fork/projects/agent-role-tool-selection/README.md docs/fork/projects/agent-role-tool-selection/design.md docs/fork/projects/agent-role-tool-selection/verification.md docs/fork/research/0.140.0/openclaude-agent-ux-runtime-backlog.md
```

Results: reused Kepler as the persistent spark test runner. Final focused rerun passed core 3/3 and TUI 5/5, `cargo insta pending-snapshots --workspace-root .` reported no pending snapshots, and scoped `git diff --check` passed.

2026-06-18:

```bash
just fmt
cargo insta accept --workspace-root . --snapshot 'tui/src/chatwidget/snapshots/codex_tui__chatwidget__tests__agent_role_templates_popup_runtime_catalog.snap'
just test -p codex-core user_agent_role_template_update_draft_rewrites_native_tool_selection starter_agent_role_template_draft_with_allowed_tools_writes_native_tool_selection create_user_agent_role_template_from_draft_rejects_invalid_tool_selection
just test -p codex-tui agent_role_template_tool_selection_picker_for_role_submits_selected_tools agent_role_template_edit_prompt_with_allowed_tools_submits_native_toml_draft agent_role_template_update_from_draft_updates_session_catalog agent_role_templates_popup_runtime_catalog_snapshot agent_role_template_tool_selection_picker_submits_selected_tools
cargo insta pending-snapshots --workspace-root .
git diff --check -- codex-rs/core/src/agent_role_templates.rs codex-rs/core/src/agent_role_templates_tests.rs codex-rs/tui/src/app_event.rs codex-rs/tui/src/app/event_dispatch.rs codex-rs/tui/src/chatwidget/agent_role_templates.rs codex-rs/tui/src/chatwidget/tests/popups_and_settings.rs codex-rs/tui/src/chatwidget/snapshots/codex_tui__chatwidget__tests__agent_role_templates_popup_runtime_catalog.snap docs/fork/features/agent-role-tool-selection.md docs/fork/features/README.md docs/fork/projects/agent-role-templates/README.md docs/fork/projects/agent-role-templates/design.md docs/fork/projects/agent-role-templates/verification.md docs/fork/projects/agent-role-tool-selection/README.md docs/fork/projects/agent-role-tool-selection/design.md docs/fork/projects/agent-role-tool-selection/verification.md docs/fork/research/0.140.0/openclaude-agent-ux-runtime-backlog.md
```

Results: existing discovered user-role allowlist editing implemented and verified through the native TOML path. Reused Kepler as the persistent spark test runner: core update/create/validation tests passed 3/3; TUI picker/edit/update/snapshot tests passed 5/5 after reviewing and accepting the intentional snapshot row `Edit tools for explorer`; no pending snapshots; scoped `git diff --check` passed.
