# Agent Role Templates (`.agents/*.md`)

## Summary

This feature extends `spawn_agent` role selection with YAML-backed role/persona templates loaded from:

1. nearest `./.codex/.agents/*.md` up the current project tree,
2. `~/.codex/.agents/*.md`,
3. embedded fallback templates.

No new listing tool is introduced. Discovery remains in `spawn_agent` schema text.

## Loading And Precedence

Templates are resolved by role stem (`<role>.md`) with first-wins precedence:

1. nearest project template directory (`./.codex/.agents`) walking upward from current cwd up to repo root,
2. user templates (`~/.codex/.agents`),
3. embedded fallback templates (`default`, `explorer`, `worker`, `awaiter`).

If the same role stem exists in multiple sources, the first source in this order is used.

Role names are normalized case-insensitively. Canonical matching also treats `-` and `_` as equivalent.
If canonical matching resolves to multiple templates, spawn fails with an ambiguity error.

## Template Contract

Each role file is a Markdown document with YAML frontmatter and persona prompt blocks.

Role file names must resolve to a valid role stem containing only lowercase ASCII letters, digits,
`_`, or `-`.

### Required YAML fields

- `description` (role description)
- `agent_names` (non-empty array)
- `agent_names` must include `name: default`
- every `agent_names[].description` is required and non-empty

### Optional YAML fields

- role-level:
  - `read_only`
  - `model`
  - `reasoning_effort`
  - `allow_list`
  - `deny_list`
- persona-level (`agent_names[]`):
  - `model`
  - `reasoning_effort`

### Prompt blocks

Prompt text must be defined only with marker blocks:

```md
<!-- agent_nickname: default -->
...
<!-- agent_nickname: reviewer -->
...
```

Rules:

- each YAML nickname must have a matching prompt block,
- prompt blocks cannot reference undeclared nicknames,
- duplicate nickname blocks are invalid,
- prompt text outside nickname blocks is invalid.
- unknown YAML fields are invalid (`deny_unknown_fields` on frontmatter entries).

## Spawn Behavior

`spawn_agent` accepts:

- `agent_type`
- `agent_nickname`
- `model`
- `reasoning_effort`

Role selection behavior:

- role config is applied when the role is declared in built-in/user `agent_roles`,
- template-only roles are allowed even without declared role config,
- if neither declared role nor template can resolve the requested `agent_type`, spawn fails.

Persona selection behavior:

- if `agent_nickname` is omitted, `default` is used,
- if provided, it must exist in the resolved template,
- for roles without template metadata, only `default` is accepted.

Template runtime settings are applied during spawn:

- selected persona prompt becomes base instructions,
- `model` and `reasoning_effort` are applied (persona overrides role-level defaults),
- `read_only: true` enforces read-only sandbox policy,
- `allow_list` and `deny_list` are normalized (trimmed, deduplicated, sorted) and propagated via
  `ThreadSpawn` metadata.

Explicit runtime overrides from tool arguments have highest priority:

- explicit `model` overrides template and inherited model,
- explicit `reasoning_effort` is validated against the resolved model capabilities.

Validation behavior:

- unknown `agent_nickname` for a template role fails with
  `unknown agent_nickname '<name>' for agent_type '<role>'`,
- unsupported alias key `agent_name` fails with
  `unsupported key \`agent_name\`; use \`agent_nickname\``,
- unknown/extra argument keys are rejected because spawn args use strict parsing
  (`#[serde(deny_unknown_fields)]`).

## Schema Discovery Output

`spawn_agent` schema metadata for each role includes only:

- role `description`,
- `read_only`,
- `agent_nickname[]` entries (`name`, `description`).

Prompt text is intentionally excluded from schema discovery.

If template metadata fails to load when the tool schema text is built, discovery includes a warning
and roles without metadata fall back to a synthetic `default` nickname entry in the description.
