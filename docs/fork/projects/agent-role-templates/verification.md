# agent-role-templates Verification

This plan validates the `fork/130` contract against `rust-v0.130.0`. It assumes native TOML roles
remain primary, `subagent-cwd` owns explicit child cwd semantics, and app-server projection stays
additive/backward compatible.

## Scenario Matrix

| Scenario | Expected result |
| --- | --- |
| Valid markdown format | Strict YAML frontmatter plus marker-delimited `default` and named personas loads successfully |
| Required fields | Missing or empty `description`, `agent_names`, or `default` fails with a controlled diagnostic |
| Unknown keys | Unknown top-level or policy frontmatter keys make the selected template invalid |
| Persona validation | Empty persona body, duplicate persona, bad persona name, or stray markdown outside blocks fails |
| Omitted persona | Missing or blank `spawn_agent.agent_persona` selects `default` |
| Unknown persona | Unknown `spawn_agent.agent_persona` fails without falling back to `default` |
| Instruction append | Selected persona appends to child developer instructions and does not replace system/base instructions |
| Instruction file | `model_instructions_file` resolves relative to the template path and applies as child base instructions |
| Exact role match | Exact markdown `name` beats canonical `-`/`_` fallback |
| Ambiguous fallback | Multiple canonical fallback matches fail with a controlled diagnostic |
| Discovery order | child `.codex/.agents/*.md`, then user `~/.codex/.agents/*.md` priority is enforced |
| Native TOML role exists | TOML role is primary; markdown with the same name only augments the single native `Available roles` entry |
| Template-only role | Valid markdown role appears in the same spawn `Available roles` guidance and spawns only when no native TOML role exists |
| Explicit cwd ordering | Explicit `cwd` rebuilds child config before TOML role and markdown template resolution |
| Child cwd discovery | Template discovery uses explicit child cwd, not parent cwd |
| Explicit cwd environments | Spawn with explicit cwd passes `environments: None` into `ThreadManager` |
| Omitted cwd | Upstream cwd and environment-selection inheritance remains unchanged |
| Explicit model/reasoning | Explicit spawn args beat markdown defaults |
| TOML-owned settings | Native TOML role-owned model/reasoning beat explicit args and markdown defaults |
| Runtime refresh | Post-role runtime refresh does not reset explicit child cwd or relax template `read_only` |
| Full-history fork | Rejects `agent_type`, `agent_persona`, `model`, `reasoning_effort`, and `cwd` |
| Partial/no-history spawn | Allows role/persona/template/cwd overrides when history fork is not full |
| Read-only policy | Only tightens effective `PermissionProfile`; it never relaxes parent/runtime policy |
| Allow list | Wildcard matches allowed tools, hides non-allowed tools, and blocks their dispatch |
| Deny list | Deny wins over allow and blocks dispatch with a controlled tool error |
| MCP policy | Session policy blocks MCP calls without mutating global `McpServerConfig` |
| Dynamic/deferred policy | Dynamic tools and tool-search/deferred entries are filtered consistently with direct tools |
| Resume compatibility | Missing persona metadata remains readable and policy remains runtime-only |
| Old rollout | Missing template state deserializes as absent and preserves upstream behavior |
| App-server compatibility | Stable schema changes are additive and expose `Thread.agentPersona` |
| TUI compatibility | No TUI persona display, snapshot churn, or agents-overlay dependency is introduced in v1 |
| Thread-note boundary | Persona text never becomes `thread_note`, and thread notes never become developer instructions |

## Focused Commands

Run after the implementation touches core spawn, role/template parsing, policy, or resume:

```bash
cd codex-rs
cargo test -p codex-core agent::role_templates::tests
cargo test -p codex-core spawn_tool_spec_build_with_templates
cargo test -p codex-core spawn_agent_applies_markdown_role_template_persona_and_policy
cargo test -p codex-core spawn_agent_applies_template_only_agent_type_and_records_role
cargo test -p codex-core spawn_agent_template_only_agent_type_omitted_persona_uses_default
cargo test -p codex-core model_visible_specs_honor_agent_tool_policy
cargo test -p codex-core dispatch_blocks_tools_denied_by_agent_tool_policy
cargo test -p codex-core tool_search_entries_honor_agent_tool_policy
```

Run state checks only if a state projection, migration, or thread runtime query path changes:

```bash
cargo test -p codex-state threads
```

Run app-server protocol checks only if implementation intentionally changes app-server protocol or
generated schema:

```bash
just write-app-server-schema
cargo test -p codex-app-server-protocol
```

Run app-server tests only if thread metadata, collab events, or app-server projections change:

```bash
cargo test -p codex-app-server
```

Run config schema generation only if `ConfigToml` or nested config types change, which v1 should
avoid:

```bash
just write-config-schema
```

Run TUI checks only if a later change adds visible persona/template UI:

```bash
cargo test -p codex-tui
cargo insta pending-snapshots -p codex-tui
```

## Minimum Unit Coverage

- Parser tests for valid templates, invalid frontmatter, invalid persona markers, required
  `default`, duplicate personas, unknown selected persona, and relative `model_instructions_file`.
- Registry tests for project/user `.agents/*.md` discovery order, exact match, canonical fallback, and
  ambiguous fallback failure.
- Spawn tests for TOML-primary precedence, unified `Available roles` guidance for template-only and
  same-name markdown roles, persona append, markdown defaults, explicit model/reasoning precedence,
  native TOML locked model/reasoning precedence, and explicit child-cwd discovery.
- Fork tests for full-history rejection of `agent_type`, `agent_persona`, `model`,
  `reasoning_effort`, and `cwd`, plus partial/no-history acceptance.
- Policy tests for model-visible filtering and dispatch-time blocking across direct, dynamic, MCP,
  and deferred/tool-search surfaces.
- Compatibility tests for optional source persona metadata and older rollout compatibility.

## Artifact Expectations

- App-server TypeScript/JSON schema diff is limited to optional `agentPersona` and optional
  `SubAgentSource::ThreadSpawn.agent_persona`.
- No SQLite migration in v1.
- No TUI snapshots in v1 unless a separate UI feature adds visible persona/template labels.
- No generated config schema diff unless implementation deliberately adds TOML-configured
  template fields.
- No model-facing `spawn_agent` output shape change for persona, policy, or effective cwd.

## Known Coverage Gaps

- Persona display in TUI is out of scope for v1.
- App-server raw policy display is out of scope for v1; policy remains runtime-only.
- SQLite persona/policy projection is out of scope for v1.
- Tool policy wildcard matching is implemented with `wildmatch` masks.
- Runtime markdown import from external agent markdown is out of scope; external migration remains
  import-only into native TOML roles.
- No source-level policy shape is exposed; clients must treat policy as server-side behavior.
