# fork/140 cross-feature acceptance verification

## Required checks

- `just test -p codex-core tool_selection_filters_direct_and_deferred_mcp_tools provider_and_model_change_uses_chat_completions_next_turn agent_role_template_current_model_draft_writes_native_model_defaults`
- `just test -p codex-tui agent_role_template_create_prompt_from_current_model_submits_native_toml_draft agent_role_templates_popup_snapshot`
- `just test -p codex-app-server mcp_server_refresh_exposes_added_tools_on_next_turn`
- `cargo insta pending-snapshots --workspace-root .`
- `git diff --check`
- 12-agent runtime smoke: spawn 12 read-only sub-agents, require every agent to report `fork/140` and clean `git status --short`.

## Verification log

| Date | Command | Result | Coverage |
| --- | --- | --- | --- |
| 2026-06-18 | `just test -p codex-core tool_selection_filters_direct_and_deferred_mcp_tools provider_and_model_change_uses_chat_completions_next_turn agent_role_template_current_model_draft_writes_native_model_defaults` | passed | Core cross-feature checks for MCP tool-selection enforcement, provider switch next-turn behavior and role TOML draft model/provider seeding. |
| 2026-06-18 | `just test -p codex-tui agent_role_template_create_prompt_from_current_model_submits_native_toml_draft agent_role_templates_popup_snapshot` | passed | TUI role authoring flow and snapshot stability. |
| 2026-06-18 | `just test -p codex-app-server mcp_server_refresh_exposes_added_tools_on_next_turn` | passed | App-server MCP refresh to next-turn tool projection. |
| 2026-06-18 | `cargo insta pending-snapshots --workspace-root .` | passed: no pending snapshots | No pending TUI snapshot artifacts. |
| 2026-06-18 | `git diff --check` | passed | Whitespace and patch hygiene. |
| 2026-06-18 | 12 read-only sub-agents: Jason, Mendel, Descartes, Kuhn, Parfit, Herschel, Lovelace, Godel, James, Popper, Boyle, Darwin | passed: 12/12 | Practical runtime smoke: all agents started concurrently enough to satisfy the fork limit expectation and each reported branch `fork/140` with a clean worktree. |

## Remaining accepted gaps

- `subagent-workbench` still defers dismiss-only visibility, retry and stop-all until each has a native runtime contract, server-side enforcement and snapshots.
- `mcp-on-demand-discovery` does not add model-visible MCP create/refresh; it verifies existing app-server refresh instead.
- `agent-role-tool-selection` still defers external app-server write/management API for role policy; native TOML authoring remains the current contract.
- Full manual TUI smoke with a rebuilt installed fork binary remains release acceptance, not a local branch merge gate.
