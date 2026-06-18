# Agent Role-Level Tool Selection Verification

## Verification Scope

This document tracks the implemented v1 runtime slice for role-level tool selection plus core runtime catalog/effective-policy diagnostics, the read-only role detail/effective-state projection, the loaded-thread app-server catalog read contract, the `/agent-roles` current-thread runtime catalog reference, parser-validated native TOML draft authoring, catalog-assisted create flow and existing user-role allowlist editing. The current scope is native config parsing, role config application, planner enforcement before `tool_search`/Code Mode/registry construction, runtime stale-id diagnostics/catalog projection, generated app-server protocol artifacts, displaying declared allowlist entries in the role template detail surface, showing loaded-thread catalog entries as read-only evidence, and proving that the role-template TOML wizard can create or update a role file containing `[tool_selection] allowed_tools` either manually or from selected current-thread catalog ids.

External app-server write/management API is not part of this slice.

## Implemented Checks

| Surface | Required check | Candidate command |
| --- | --- | --- |
| Config parse/schema | Valid `[tool_selection] allowed_tools` parses into canonical `ToolName`; malformed/duplicate entries fail fast; generated schema includes the new field. | `just write-config-schema`; `just test -p codex-core tool_selection`. |
| Role application | Role `config_file` applies policy through `apply_role_to_config`, not via `AgentRoleToml` or TUI metadata. | `just test -p codex-core tool_selection`. |
| Tool planning | Filter runs after source assembly and before `tool_search`, Code Mode and registry construction; a second pass keeps synthetic Code Mode tools strict. | `just test -p codex-core tool_selection`. |
| Model-visible specs | Blocked direct tools are absent from model-visible specs. | `tool_selection_allowlist_filters_visible_specs_and_registry`. |
| Dispatch registry | Blocked direct tools are absent from `ToolRegistry`, so native unsupported-tool dispatch path remains authoritative. | `tool_selection_allowlist_filters_visible_specs_and_registry`. |
| Dispatch rejection | A blocked direct tool call returns native unsupported-tool diagnostics and does not execute the handler. | `tool_selection_blocked_tool_dispatch_uses_native_unsupported_path`. |
| Deferred `tool_search` | Blocked deferred extension tools are absent before `tool_search` is appended. | `tool_selection_filters_deferred_tools_before_tool_search`. |
| MCP direct/deferred | Direct MCP tools use canonical namespaced `ToolName`; deferred MCP tools require both the deferred tool and `tool_search` in the allowlist. | `tool_selection_filters_direct_and_deferred_mcp_tools`. |
| Code Mode | Nested tool remains registered only when allowed; `exec`/`wait` must also be explicitly allowlisted to expose Code Mode. | `tool_selection_filters_code_mode_projection_strictly`. |
| Hosted tools | Hosted specs are filtered by plain hosted tool name. | `tool_selection_filters_hosted_specs`. |
| Default upstream path | Missing policy keeps native tools unrestricted. | `tool_selection_missing_policy_keeps_native_tools_unrestricted`. |
| Unknown/stale ids | Syntactically valid unmatched ids do not create model-visible or registered tools. | `tool_selection_unknown_entries_do_not_create_tools`. |
| Stale-id diagnostics | Unmatched ids are derived from current `PlannedTools`, include hosted/namespaced formatting, and emit one bounded existing warning per turn. | `tool_selection_stale_entries_are_derived_from_available_inventory`; `built_tools_warns_once_for_unavailable_tool_selection_entries`. |
| Runtime catalog projection | Tool identities are derived from a planner-owned inventory snapshot and include direct, deferred, MCP, hosted, hidden, `tool_search` and Code Mode synthetic tools with selected state and exposure metadata for the current policy, without a TUI hardcoded registry. | `tool_selection_catalog_projects_effective_runtime_tools`; `tool_selection_catalog_marks_unselected_runtime_discovered_tools`; `tool_selection_missing_policy_keeps_native_tools_unrestricted`; `tool_selection_unknown_entries_do_not_create_tools`. |
| App-server catalog read | `agentRole/toolSelectionCatalog/read` reads the loaded thread's native catalog projection through `CodexThread::tool_selection_catalog`, not a global or config-only registry. | `read_tool_selection_catalog_uses_loaded_thread_policy`; `serialize_agent_role_tool_selection_catalog_read`; `schema_fixtures_match_generated`. |
| Synthetic effective diagnostics | A selected synthetic tool that is present in the inventory catalog but absent from the enforced final plan is reported as unmatched. | `tool_selection_warns_when_selected_tool_search_has_no_effective_index`. |
| Native availability boundary | Allowlisting an environment-backed tool does not bypass native availability gating when no environment exists. | `tool_selection_allowlist_does_not_enable_unavailable_tools`. |
| Restart/config reload | A fresh config load from the same `$CODEX_HOME` can resolve the role and reapply the same tool-selection policy through `apply_role_to_config`. | `agent_role_tool_selection_survives_config_restart_reload`. |
| Cold resume | A resumed thread reloads the same `$CODEX_HOME/config.toml` and keeps the effective tool-selection policy in the resumed request tools. | `resume_retains_tool_selection_policy_in_resumed_turn`. |
| Role templates regression | Existing role template listing/create tests still pass with the new ConfigToml field. | `just test -p codex-core agent_role_templates`. |
| TUI role detail projection | Declared `[tool_selection] allowed_tools` appears in the role template selected detail with read-only effective-state copy, without adding a parallel editor/profile schema. | `just test -p codex-tui agent_role_templates_popup_snapshot`. |
| TUI runtime catalog reference | `/agent-roles` renders `agentRole/toolSelectionCatalog/read` output as current-thread tool-id evidence with exposure labels and unmatched entries, and states that it is not a role-specific preview. | `just test -p codex-tui agent_role_templates_popup_runtime_catalog_snapshot`. |
| Role-template parser validation | Invalid or duplicate `[tool_selection] allowed_tools` entries in a role TOML file are reported by native role template listing and draft creation before spawn/config rebuild. | `just test -p codex-core agent_role_templates`. |
| Native TOML draft authoring | The `/agent-roles` create flow can write a parser-validated `$CODEX_HOME/agents/<role>.toml` draft that includes `[tool_selection] allowed_tools`. | `create_user_agent_role_template_from_draft_writes_valid_native_toml`; `agent_role_template_create_from_draft_updates_session_catalog`. |
| Catalog-assisted create | A loaded current-thread catalog can open a searchable picker, grouped by native exposure class, and selected ids seed an editable native TOML draft. | `starter_agent_role_template_draft_with_allowed_tools_writes_native_tool_selection`; `agent_role_template_tool_selection_picker_submits_selected_tools`; `agent_role_template_create_prompt_with_allowed_tools_submits_native_toml_draft`; `agent_role_templates_popup_runtime_catalog_snapshot`. |
| Existing user-role allowlist edit | A valid discovered `$CODEX_HOME/agents/*.toml` user role can open a searchable picker with its current allowlist preselected, then submit an editable update draft that overwrites the same native role file. | `user_agent_role_template_update_draft_rewrites_native_tool_selection`; `agent_role_template_tool_selection_picker_for_role_submits_selected_tools`; `agent_role_template_edit_prompt_with_allowed_tools_submits_native_toml_draft`; `agent_role_template_update_from_draft_updates_session_catalog`; `agent_role_templates_popup_runtime_catalog_snapshot`. |

## Acceptance Criteria

Current v1 acceptance:

- There is exactly one effective role tool-selection source of truth: `ConfigToml`/`Config`/`TurnContext.config`.
- Old roles without `[tool_selection]` behave as before.
- Blocked direct, deferred, hosted and Code Mode-projected tools are absent from the relevant projections.
- Allowing a tool does not widen permission/sandbox/network/approval/MCP policy because policy is purely subtractive over the tool plan; focused tests prove unknown ids and environment-unavailable tools do not become available.
- Blocked direct dispatch uses the existing unsupported-tool error path.
- Generated config schema is updated.
- App-server read protocol is generated and schema-tested for `agentRole/toolSelectionCatalog/read`.
- Fresh config restart/reload keeps the role-applied tool-selection policy.
- Cold thread resume through the same `$CODEX_HOME/config.toml` keeps the effective tool-selection policy.

## Current Known Gaps

- Stronger persisted policy retention is not claimed: if a future contract requires resumed child threads to keep a role-applied tool policy after the source config file is removed or changed, that needs a new persistence/runtime design.
- Direct sandbox/approval negative remains a possible future test if this feature starts changing security-policy surfaces; blocked-tool dispatch and native availability no-grant behavior are covered.
- External app-server write/management surfaces are deferred. Core/app-server expose catalog diagnostics with selected state, `/agent-roles` consumes them as read-only current-thread evidence, and the existing native TOML draft write surface can create or update a discovered user role file with `[tool_selection] allowed_tools`. External `config_file` roles remain on the open-file path until ownership/write semantics are defined.

## Verification Log

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
