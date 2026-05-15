# Feature: agent-role-templates

## Feature Passport

- Code name: `agent-role-templates`
- Status: implemented for `fork/130`
- Baseline: `rust-v0.130.0` / `58573da43ab697e8b79f152c53df4b42230395a8`
- Goal: add markdown-backed personas and template-scoped policy on top of native TOML agent roles.
- Scope in: markdown template discovery, `agent_persona`, persona instruction append, template defaults, template-only roles, session-scoped tool policy, additive app-server persona projection.
- Scope out: replacing native TOML roles, SQLite persona projection, policy projection to TUI, returning persona/policy/cwd in model-facing spawn output.

## User Contract

- Native TOML roles remain primary. Markdown templates augment native roles or define template-only roles only when no native TOML role exists.
- `spawn_agent.agent_type` is the upstream-compatible role selector. Use values such as
  `orchestrator`, not a combined `role:persona` string.
- `spawn_agent.agent_persona` is optional and selects a persona inside the chosen `agent_type`.
  Omitted or blank selects `default`; when supplied, use persona names such as `default`.
- Every selected template must define a valid `default` persona.
- Invalid selected templates, missing `default`, unknown personas, duplicate persona blocks, unknown frontmatter keys, and stray markdown outside persona blocks fail with controlled diagnostics.
- Markdown `model` and `reasoning_effort` are defaults only. Explicit spawn args beat markdown defaults, and native TOML role-owned model/reasoning beats both.
- Full-history fork rejects `agent_type`, `agent_persona`, `model`, `reasoning_effort`, and `cwd`.
- Explicit `spawn_agent.cwd` from `subagent-cwd` rebuilds the child config before native role and markdown template resolution.
- Template discovery runs from child cwd, not parent cwd.
- Explicit `cwd` passes `environments: None` to `ThreadManager`; omitted `cwd` preserves inherited environment selections.
- Template-only `agent_type` values are persisted as child `agent_role` metadata so app-server clients can display the selected role label.
- Persona, policy, and effective cwd are not returned in model-facing `spawn_agent` output.

Canonical markdown template spawn example:

```json
{
  "message": "coordinate this task",
  "agent_type": "orchestrator",
  "agent_persona": "default"
}
```

## Template Contract

Templates use strict YAML frontmatter and marker-delimited persona blocks compatible with `fork/118`:

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

- Required frontmatter: `description`, `agent_names`.
- Optional frontmatter: `model`, `reasoning_effort`, `model_instructions_file`, `read_only`, `allow_list`, `deny_list`.
- Required persona: `default`.
- Persona names use lowercase ASCII letters, digits, `_`, and `-`.
- `model_instructions_file` resolves relative to the markdown template file.
- Persona text appends to child developer instructions; it does not replace system/base instructions.
- `model_instructions_file` overrides child base instructions for the spawned session.

## Discovery And Matching

Discovery order, highest priority first:

1. child project `.codex/.agents/*.md`
2. user `~/.codex/.agents/*.md`

Matching rules:

- Exact template `name` wins before canonical fallback.
- Canonical fallback normalizes only `-` and `_`.
- Ambiguous fallback fails with a controlled diagnostic.
- `agents/*.md` is not a supported markdown template path; native TOML roles continue using the existing `.codex/agents/*.toml` surfaces.
- Template-only roles appear in the same model-visible `Available roles` list as native TOML roles,
  but are still weaker than native TOML roles.
- Markdown templates with the same name as a native TOML role do not create duplicate role entries;
  the native role description remains primary and markdown contributes persona/read-only metadata
  inside that role entry.

## Integration And Compatibility

- Parser/registry should live near native role code, e.g. `codex-rs/core/src/agent/role_templates.rs`.
- The native TOML path remains `codex-rs/core/src/config/agent_roles.rs` plus `codex-rs/core/src/agent/role.rs`.
- Tool policy is session-scoped and enforced server-side. It must not mutate global MCP server config.
- `allow_list` and `deny_list` use `wildmatch` masks with `*` and `?`; deny wins.
- Policy filters model-visible specs and blocks dispatch for builtin, dynamic, MCP, and deferred/tool-search surfaces.
- Effective template persona is stored on the child session source as optional JSON metadata. Tool policy is runtime-only. Older sessions omit persona and remain readable.
- App-server v2 adds stable optional `Thread.agentPersona` and optional `agent_persona` in the existing `Thread.source` projection. Tool policy remains runtime-only and is not exposed to app-server clients. No SQLite column is added.
- `agent_persona` is a model-facing `spawn_agent` parameter, not an app-server client request.
- `thread-note` remains separate display metadata; templates must not write note content.
- TUI may display persona when it is already available from app-server metadata, but must not
  require or display template policy in v1.

## Verification Matrix

| Surface | Required coverage |
| --- | --- |
| Parser | strict frontmatter, default persona, duplicate/unknown/stray content failures |
| Discovery | project/user `.agents/*.md` order, exact match, canonical fallback, ambiguity |
| Spawn | explicit args, TOML precedence, template defaults, cwd-rooted discovery |
| Forking | full-history rejects persona/template/model/reasoning/cwd overrides |
| Policy | read-only tightening, allow/deny filtering, dispatch blocking |
| Persistence | optional source fields are backward compatible with old rollouts |
| Compatibility | additive app-server schema, no SQLite migration or TUI policy projection |

## Doc Changelog

- 2026-05-13: Implemented `fork/130` contract using `fork/118` markdown syntax, wildcard tool policy, and additive app-server persona projection.
- 2026-05-13: Clarified that template-only `agent_type` selections persist `agent_role` for app-server client role labels.
- 2026-05-13: Clarified upstream-compatible role/persona spawn form: role via `agent_type`,
  persona via optional `agent_persona`.
- 2026-05-13: Aligned markdown template role discovery output with native `Available roles`
  formatting and native-role precedence.
- 2026-05-15: Clarified that `agents-overlay` may render available persona labels while template
  policy remains server-side only.
- 2026-05-12: Expanded `fork/130` contract with child-cwd ordering, exact markdown format, policy enforcement, persistence, and app-server/TUI boundaries.
- 2026-05-12: Initial `fork/130` contract.
