# Agent Role-Level Tool Selection Design

## Design Position

The first runtime slice is implemented as “role-applied config contributes an effective tool-selection policy that the native tool planner enforces before any projection is built.” Role templates do not own a separate tool list.

Rejected starting points:

- TUI-only filtering, because it does not affect model-visible specs, `tool_search`, Code Mode or dispatch.
- A parallel role-profile registry, because `Config.agent_roles` and role config files already own role source of truth.
- A separate MCP-only filter, because Codex tools include core, hosted, extension, dynamic, collaboration and Code Mode surfaces.
- Late dispatch-only deny checks, because blocked tools would still be discoverable and model-visible.

## Current Native Pipeline

Role source:

- `codex-rs/config/src/config_toml.rs::AgentsToml` declares user roles as flattened `roles: BTreeMap<String, AgentRoleToml>`.
- `codex-rs/config/src/config_toml.rs::AgentRoleToml` currently contains `description`, `config_file` and `nickname_candidates`, not generic tool selection.
- `codex-rs/core/src/config/agent_roles.rs::parse_agent_role_file_contents` parses role files, removes `name`, `description` and `nickname_candidates`, and leaves the remaining TOML as a config layer.
- `codex-rs/core/src/agent/role.rs::apply_role_to_config` applies the named role by loading its config file and rebuilding `Config`.

Tool planning:

- `codex-rs/core/src/session/turn.rs::built_tools` gathers all MCP tools, computes `build_mcp_tool_exposure`, and passes direct/deferred MCP tools, discoverable tools, extension executors and dynamic tools into `ToolRouterParams`.
- `codex-rs/core/src/tools/spec_plan.rs::build_tool_specs_and_registry` runs `add_tool_sources`, `append_tool_search_executor`, `prepend_code_mode_executors`, then `build_model_visible_specs_and_registry`.
- `codex-rs/core/src/tools/spec_plan.rs::append_tool_search_executor` indexes runtimes where `ToolExposure::Deferred`.
- `codex-rs/core/src/tools/spec_plan.rs::build_code_mode_executors` builds nested Code Mode tools from current executors.
- `codex-rs/core/src/tools/spec_plan.rs::build_model_visible_specs_and_registry` emits model-visible specs and constructs `ToolRegistry::from_tools`.

MCP naming:

- `codex-rs/codex-mcp/src/tools.rs::ToolInfo` preserves raw MCP server/tool identity while also storing `callable_namespace` and `callable_name`.
- `ToolInfo::canonical_tool_name` returns the model-visible namespaced `ToolName`.
- Existing `ToolFilter` filters raw MCP `tool.name` inside a single MCP server config; it is not a generic role-level filter.

## Implemented Native Contract

V1 introduced `ConfigToml.tool_selection` and effective `Config.tool_selection`. Production-ready v2 keeps the same native owner and adds `denied_tools`:

```toml
[tool_selection]
allowed_tools = ["update_plan", "codex_app/lookup"]
denied_tools = ["mcp__untrusted/shell"]
```

Semantics:

- source lives in native config loaded from role-applied TOML;
- old role files without `[tool_selection]` deserialize and behave unchanged;
- policy is available through `TurnContext.config` for each child thread turn;
- enforcement matches canonical runtime `ToolName`: `name` for plain tools and `namespace/name` for namespaced tools;
- allow then deny: `allowed_tools` narrows the candidate set when present, then `denied_tools` removes entries from the effective set; deny wins on conflicts;
- malformed entries and duplicates fail fast during config load/config rebuild;
- syntactically valid but stale runtime ids are harmless no-access entries; focused tests prove they do not create visible or registered tools; user-facing diagnostics are derived from current `PlannedTools` and emitted through the existing warning event path.

Deny is deliberately implemented in the same config/spec-plan path, not as MCP-only filtering, app-server state or TUI state. It is subtractive only: it cannot make a tool available if native source assembly, provider/model support, permissions, environment gating or MCP exposure removed it earlier.

## Source-Of-Truth Candidates

### Candidate A: New Generic Config Field

Role file contributes `[tool_selection] allowed_tools = [...]` and/or `denied_tools = [...]`. The config loader merges it into `Config`, and `TurnContext` carries the effective policy into tool planning.

This is the implemented v1 direction. The field intentionally stays outside `ToolsToml` because that section currently owns feature toggles, not generic tool-planning policy.

### Candidate B: Extend `AgentRoleToml`

Role declarations get tool-selection fields next to `description`, `config_file` and `nickname_candidates`.

This is useful for catalog display, but unsafe as the only source because declarations are not the effective turn config. If selected, it must compile into the same `Config`/`TurnContext` policy as Candidate A.

### Candidate C: ToolRouter Parameter Only

`ToolRouterParams` gets a policy and `spec_plan` filters `PlannedTools`.

This is necessary near enforcement, but not sufficient as source of truth. The policy still needs to originate from native config after role application.

## Enforcement Point

The filter runs in `build_tool_specs_and_registry` after `add_tool_sources(&context, &mut planned_tools)` and before `append_tool_search_executor(&context, &mut planned_tools)`. It also runs again after `prepend_code_mode_executors(&context, &mut planned_tools)` so synthetic Code Mode tools (`exec`/`wait`) cannot bypass a strict allow/deny policy.

That placement is required because:

- `tool_search` is appended from deferred runtimes already present in `planned_tools`;
- Code Mode executors are prepended from the current runtime list;
- model-visible specs and `ToolRegistry` are both derived from the same `planned_tools` after those steps;
- dispatch of a blocked tool is safely handled by absence from `ToolRegistry`, using the existing unsupported-tool path.

Do not filter only `model_visible_specs`; that leaves dispatch and Code Mode bypasses. Do not filter only `ToolRegistry`; that leaves model-visible and `tool_search` leaks. Do not filter only MCP exposure; that ignores core, extension, dynamic, hosted and collaboration tools.

## Tool Identity Model

V1 chooses canonical runtime `ToolName`:

- plain core tool: TOML `name` maps to `ToolName { namespace: None, name }`;
- namespaced tool: TOML `namespace/name` maps to `ToolName { namespace: Some(...), name }`;
- MCP model-visible tool: `ToolInfo::canonical_tool_name()`;
- MCP raw identity is not accepted in v1;
- Code Mode projection names are not config identity, but `exec` and `wait` are plain tools and must be explicitly allowlisted if Code Mode execution should be available;
- dynamic/extension tools use `ToolExecutor::tool_name()`.

Future TUI authoring may display MCP raw server/tool labels, but it must resolve them back to canonical `ToolName` before writing config.

## Permissions And Security Boundary

Tool selection is a subtractive capability policy. It cannot grant permissions and cannot make a tool safer than its runtime boundary. Existing owners remain authoritative:

- `PermissionProfile` owns filesystem/network/sandbox policy.
- Approval policy owns ask/auto behavior.
- MCP manager owns server startup, OAuth, approval mode, elicitation and raw protocol calls.
- Hooks/Guardian/tool lifecycle own review and telemetry.
- Provider/model metadata owns `ToolMode`, hosted tool support and transport-specific capabilities.

The effective rule is intersection: selected-by-role and allowed-by-security-boundaries must both be true. If a tool appears in both `allowed_tools` and `denied_tools`, it is denied.

## Compatibility And Migration

Old role files without tool-selection fields keep full native tool availability. Malformed entries and duplicates fail fast when config is loaded or when a child thread rebuilds config from a role file.

Current validation checks syntax and duplicates during config load/config rebuild for both `allowed_tools` and `denied_tools`. For dynamic, extension and MCP deferred tools, fail-fast inventory validation remains intentionally absent because runtime inventory is not available at plain config parse time. This does not grant access; unmatched entries simply match no runtime tool, `tool_selection_unknown_entries_do_not_create_tools` covers allowlist no-access behavior, and `built_tools_warns_once_for_unavailable_tool_selection_entries` covers the observable warning once the native runtime catalog exists.

Runtime diagnostics are projection-only:

- `spec_plan` computes `ToolSelectionDiagnostics.catalog_entries` from a planner-owned inventory snapshot before enforcement removes blocked tools, including direct, deferred, MCP, hosted, hidden dispatch-only, `tool_search` and Code Mode synthetic tool identities plus selected state and bounded exposure metadata for the current policy. `selected=false` means either absent from `allowed_tools` when allowlist is present or present in `denied_tools`.
- `spec_plan` computes `ToolSelectionDiagnostics.unmatched_allowed_tools` from the enforced `PlannedTools` after source assembly, the first filter, deferred `tool_search` append and Code Mode synthetic executor prepend, so selected synthetic tools that cannot exist in the effective plan still warn.
- `ToolRouter` carries that projection alongside model-visible specs and `ToolRegistry`.
- `built_tools` emits a bounded existing `EventMsg::Warning` once per turn using a `TurnContext` latch.
- `CodexThread::tool_selection_catalog` builds the same native `ToolRouter` without emitting the warning side effect, then projects `catalog_entries` and `unmatched_allowed_tools` for a loaded thread.
- Experimental app-server method `agentRole/toolSelectionCatalog/read` exposes this loaded-thread projection through generated protocol schema/TypeScript. It requires `threadId`; invalid or unloaded threads fail fast instead of returning a global/config-only approximation.
- Experimental app-server method `agentRole/toolSelection/set` is the external write/management surface. It is a whole-policy replacement over an existing discovered user role file under `$CODEX_HOME/agents/*.toml`: built-in roles, shadowed built-ins without a user file and external `config_file` roles are rejected. The endpoint rewrites only `[tool_selection]`, validates the resulting TOML through the native role parser, and then must complete the existing config reload/loaded-thread refresh path before returning success. This deliberately does not use `config/batchWrite`, because the source of truth is the role file, not user `config.toml`.
- New path-backed sub-agent spawns persist the effective role-applied tool-selection policy in `SessionSource::SubAgent(ThreadSpawn.tool_selection)`. This is intentionally downstream of `apply_role_to_config`: role TOML remains the source for creating a child config, while thread-spawn metadata becomes the durable effective-policy snapshot for that already-created child. Resume and retry restore this snapshot before the child builds future turns, so editing or deleting the role file after spawn cannot silently widen or replace the existing child thread's tool boundary.
- Direct app-server method `mcpServer/tool/call` is an execution surface, not only a read projection. It enters core through `CodexThread::call_mcp_tool` and `Session::call_tool`, where the loaded thread's effective `Config.tool_selection` is checked before `McpConnectionManager::call_tool`. The check resolves raw MCP `server`/`tool` through `McpConnectionManager::list_all_tools()` and uses `ToolInfo::canonical_tool_name()`, preserving native MCP naming behavior.
- Diagnostics do not feed back into policy, do not create a second registry and do not turn unmatched ids into grants. The catalog entries are a read-only projection for UI evidence and native TOML authoring; they are not an authoring source of truth.

## Remaining Design Work

- Catalog-assisted TUI authoring is implemented as a transient picker over the loaded-thread read-only catalog followed by the existing native TOML draft/file path. For new roles, the picker seeds a starter draft. For existing valid user roles discovered under `$CODEX_HOME/agents/*.toml`, the picker preselects the role's current allowlist and prepares an editable update draft for the same file. The loaded-thread catalog can show and seed canonical tool identities, selected state and exposure metadata as evidence, but it must not become a role-specific preview or separate editor registry. External `config_file` roles remain on the open-file path until a separate ownership/write contract exists.
- App-server write/management API intentionally guarantees new spawns and next-turn/reload visibility only. It does not rewrite an already materialized role-applied session layer inside an in-flight child turn. Existing child threads keep their own `ThreadSpawn.tool_selection` snapshot until a separate live-mutation contract is designed.

## Retention Contract

The durable snapshot shape mirrors user-facing TOML/API names rather than serializing `ToolName` directly: `allowed_tools` and `denied_tools` are stored as canonical `name` or `namespace/name` strings plus a version and source marker. Core converts between this protocol snapshot and `Config.tool_selection` through the same native grammar used by `[tool_selection]`.

Producer path:

- V1/V2 `spawn_agent` builds the child `Config`, applies role config through `apply_role_to_config`, then derives a `SubAgentToolSelectionSnapshot` from the effective `Config.tool_selection`.
- `thread_spawn_source` stores that snapshot in `SessionSource::SubAgent(ThreadSpawn.tool_selection)` before the thread is created.
- `AgentControl::prepare_thread_spawn` preserves an explicit snapshot, and does not synthesize one for unrestricted roles.

Consumer path:

- Cold resume reads `stored_thread.source.get_subagent_tool_selection()`, applies that snapshot to the child `Config`, and then passes it through `prepare_thread_spawn`, so stale state-db metadata cannot erase the rollout snapshot.
- `ensure_v2_agent_loaded` applies the snapshot before reloading a stored V2 agent, so lazy Agent Window loading uses the same retained policy as full tree resume.
- Retry copies the target thread's `ThreadSpawn.tool_selection` into the fresh sibling source and uses the target config snapshot, so retry inherits the same tool boundary without reading the current role file.

Compatibility:

- Old sessions without `ThreadSpawn.tool_selection` stay readable and keep the previous reload behavior.
- Missing or malformed source role files after spawn do not affect child threads that already have a snapshot.
- The snapshot is not a grant. It still flows into `Config.tool_selection`, so all native `spec_plan`, MCP direct-call, permission, sandbox and approval boundaries remain authoritative.
