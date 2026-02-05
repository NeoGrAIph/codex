# Configuration

For basic configuration instructions, see [this documentation](https://developers.openai.com/codex/config-basic).

For advanced configuration instructions, see [this documentation](https://developers.openai.com/codex/config-advanced).

For a full configuration reference, see [this documentation](https://developers.openai.com/codex/config-reference).

## Connecting to MCP servers

Codex can connect to MCP servers configured in `~/.codex/config.toml`. See the configuration reference for the latest MCP server options:

- https://developers.openai.com/codex/config-reference

## Apps (Connectors)

Use `$` in the composer to insert a ChatGPT connector; the popover lists accessible
apps. The `/apps` command lists available and installed apps. Connected apps appear first
and are labeled as connected; others are marked as can be installed.

## Notify

Codex can run a notification hook when the agent finishes a turn. See the configuration reference for the latest notification settings:

- https://developers.openai.com/codex/config-reference

## Agents registry (YAML)

When the `fn_multi_agents` feature flag is enabled, Codex can discover agent definitions from `agents/` folders alongside each config layer:

- Project: `.codex/agents` (next to `.codex/config.toml`)
- User: `~/.codex/agents`
- System: the folder containing the system config file

Each agent file is a Markdown file with YAML frontmatter and an instructions body:

```md
---
agent_type: "code-architect"
description: "Architecture guidance and review."
model: "gpt-5.2"
reasoning_effort: "high"
color: "cyan"
tools: ["read_file", "apply_patch"]
---

You are a senior architect...
```

`model` must be a valid model slug (no alias mapping is applied). `reasoning_effort` is optional and should be one of: `minimal`, `low`, `medium`, `high`, `xhigh`.

`tools` can be a list or a comma-separated string. If present, it acts as an allowlist
that is intersected with any existing tool allowlist (it never expands beyond the
current allowlist), and denylist rules still apply. Agent `tool_denylist` entries
are merged with any existing denylist rather than replacing it. Use `*` to leave tool access
unchanged.

### Agent name variants

You can declare `agent_persons` to provide named instruction variants (A/B) and optionally
override model and reasoning effort per variant:

```md
---
agent_type: "code-reviewer"
description: "Review code with optional strictness."
model: "gpt-5.2"
reasoning_effort: "medium"
color: "cyan"
agent_persons:
  - agent_name: strict
    description: "Strict review mode."
    model: "gpt-5.2"
    reasoning_effort: "high"
  - agent_name: lenient
    description: "Faster, higher-level review."
    model: "gpt-5.2"
    reasoning_effort: "medium"
---

Default reviewer instructions...

<!-- agent_name: strict -->
Strict instructions...

<!-- agent_name: lenient -->
Lenient instructions...
```

If `model` or `reasoning_effort` are specified under an `agent_name`, they take priority over
the top-level values for that variant. Omitted fields inherit from the top-level values.

Legacy fields `name` and `agent_names` are still accepted for compatibility, but must not be
mixed with `agent_type` or `agent_persons` in the same file.

## JSON Schema

The generated JSON Schema for `config.toml` lives at `codex-rs/core/config.schema.json`.

## Notices

Codex stores "do not show again" flags for some UI prompts under the `[notice]` table.

Ctrl+C/Ctrl+D quitting uses a ~1 second double-press hint (`ctrl + c again to quit`).
