# agent-role-templates Design

## Canonical State

Native TOML roles remain the canonical role config source. Markdown templates produce an effective
template state for a specific spawned child:

- role name
- persona name
- developer instruction append text
- optional base instruction override text
- effective session-scoped tool policy

Persist the selected persona on the child session source as optional JSON metadata. Keep tool
policy in runtime config only. Older sessions omit persona and remain readable.

## Parser And Registry

Place the parser/registry near native role application, e.g.
`codex-rs/core/src/agent/role_templates.rs`. Keep it separate from `ConfigToml`; markdown-only
semantics should not require config schema regeneration in v1.

The registry is built from the child config:

1. child project `.codex/.agents/*.md`
2. user `~/.codex/.agents/*.md`

The child project path is derived from `config.cwd` after `subagent-cwd` has rebuilt the child
config. Parent cwd must not be consulted for template discovery after explicit `spawn_agent.cwd`.

Matching rules:

- Exact template `name` wins.
- If exact match is absent, fallback may normalize only `-` and `_`.
- Ambiguous fallback fails with a model-facing diagnostic.
- Native TOML role match is stronger than markdown. Markdown with the same role name augments the
  resolved TOML role; markdown creates a template-only role only when no native TOML role exists.
- Spawn remains upstream-shaped: `agent_type` selects the role, while optional `agent_persona`
  selects only the persona inside that role. Do not use combined `role:persona` values as the
  canonical model-facing form.
- Model-visible spawn guidance uses one upstream-shaped `Available roles` list. Template-only
  markdown roles are appended after native roles; same-name markdown templates add persona/read-only
  lines inside the native role block instead of creating a second markdown-specific section.

## Markdown Format

Templates require strict YAML frontmatter followed by only marker-delimited persona blocks:

```md
---
description: "Reviews implementation plans and code changes."
agent_names:
  - name: default
    description: Default reviewer persona.
  - name: strict
    description: Stricter reviewer persona.
model: "gpt-5-codex"
reasoning_effort: "high"
model_instructions_file: "./instructions.md"
read_only: true
allow_list:
  - shell*
deny_list:
  - image_generation
---

<!-- agent_nickname: default -->
Default reviewer persona text.

<!-- agent_nickname: strict -->
Stricter reviewer persona text.
```

Validation:

- `description` and `agent_names` are required and non-empty.
- Allowed top-level keys are `description`, `agent_names`, `model`, `reasoning_effort`,
  `model_instructions_file`, `read_only`, `allow_list`, and `deny_list`.
- `default` persona is required.
- Persona names use lowercase ASCII letters, digits, `_`, and `-`.
- Empty persona body, duplicate persona name, unknown selected persona, and stray markdown outside
  persona blocks fail.
- `model_instructions_file` resolves relative to the markdown file.

Invalid templates found during discovery are ignored with startup/discovery diagnostics. Selecting
an invalid template, selecting an unknown persona, or hitting an ambiguous fallback fails the spawn
with a controlled model-facing diagnostic.

## Spawn Data Flow

Non-fork spawn order:

1. Parse `agent_type`, `agent_persona`, `model`, `reasoning_effort`, and optional `cwd`.
2. Build the initial child config from the parent turn runtime snapshot.
3. Resolve and validate explicit `cwd` per `subagent-cwd`.
4. If explicit `cwd` exists, rebuild the child config for that cwd before role/template lookup.
5. Apply native TOML role with `apply_role_to_config()` and record whether the role owns
   model/reasoning through `model`, `model_reasoning_effort`, or `profile`.
6. Re-apply runtime cwd, approval, shell environment, and permission state without resetting an
   explicit child cwd.
7. Build the markdown registry from the child cwd and resolve template/persona.
8. Append persona text to child `developer_instructions`.
9. Apply markdown `model`/`reasoning_effort` only while the child still has the inherited parent
   model/reasoning values.
10. Apply `model_instructions_file` as the child base instructions for the spawned session.
11. Apply explicit spawn `model`/`reasoning_effort` only when the native TOML role did not own
    those settings; explicit values beat markdown defaults.
12. Tighten `read_only` and install session-scoped tool policy after runtime refresh so template
    policy cannot be relaxed by parent runtime state.
13. Persist selected template metadata with the child session source.
14. Spawn with `environments: None` when explicit `cwd` was supplied; otherwise preserve inherited
    `turn.environments.to_selections()`.

Full-history fork:

- Reject `agent_type`, `agent_persona`, `model`, `reasoning_effort`, and explicit `cwd`.
- Do not resolve markdown templates.
- Inherit parent settings/history under existing full-history fork semantics.

Partial/no-history spawn:

- `fork_turns=none` and positive `fork_turns=N` may use persona/template overrides.
- Explicit `cwd` uses child-cwd discovery and `environments: None`.

## Policy

Template policy is session-scoped runtime state. It must not mutate `McpServerConfig` or plugin MCP
policy.

Semantics:

- `allow_list` absent means all tools are allowed unless denied.
- `allow_list` present means only matching model-visible names are allowed.
- `deny_list` always wins.
- Namespaced tools use the model-visible concatenated name.
- Matching uses `wildmatch` masks with `*` and `?`, matching `fork/118`.
- `read_only` can only tighten the effective `PermissionProfile`.

Implementation points:

- Filter `ToolRouter::model_visible_specs()`.
- Filter tool-search/deferred entries before they are exposed to the model.
- Filter dynamic tools consistently with direct tools.
- Enforce again in `ToolRouter::dispatch_tool_call_with_code_mode_result()` before registry
  dispatch, including MCP calls resolved through `resolve_mcp_tool_info()`.
- Blocked calls return a controlled tool error; they must not escalate to user approval.

## Persistence And Resume

`SubAgentSource::ThreadSpawn` carries optional `agent_persona` only. Tool policy is copied into
the spawned session runtime config and is not exposed through app-server protocol or public source
metadata. No SQLite migration or new columns are introduced.

Resume:

- Stored session metadata remains backward compatible when optional persona metadata is absent.
- Direct and subtree resume must not require markdown files to exist for old sessions to load.
- If persona metadata is restored for resumed descendants, the behavior must be covered by a
  focused resume test before the docs promise it as a guaranteed contract.

SQLite projection:

- No v1 migration for persona or policy.
- Add projection later only if a UI/query feature explicitly needs it.

## App-server And UI Compatibility

- App-server v2 exposes `Thread.agentPersona` as an optional stable field derived from
  `SessionSource::get_agent_persona()`.
- Existing app-server `Thread.cwd` remains the app/client observation path for child cwd.
- Codex app clients do not pass persona/policy; they are selected by the model through
  `spawn_agent`.
- Core enforces policy server-side, so old clients remain compatible.
- TUI and agents overlay must not require policy projection in v1.
- Thread notes are display metadata only and are not sourced from personas.

## Tradeoffs

- Persisting the selected persona in source metadata keeps app-server/resume projections stable
  without exposing runtime-only policy as public metadata.
- Template-only roles improve migration ergonomics but remain deliberately weaker than native TOML
  roles.
- Additive app-server projection gives Codex app persona visibility without requiring clients to
  send new request fields.
