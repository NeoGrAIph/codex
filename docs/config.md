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

When the `collab` feature flag is enabled, Codex can discover agent definitions from `agents/` folders alongside each config layer:

- Project: `.codex/agents` (next to `.codex/config.toml`)
- User: `~/.codex/agents`
- System: the folder containing the system config file

Each agent file is a Markdown file with YAML frontmatter and an instructions body:

```md
---
name: "code-architect"
description: "Architecture guidance and review."
model: "gpt-5.2-codex"
reasoning_effort: "medium"
color: "cyan"
tools: ["read_file", "apply_patch"]
---

You are a senior architect...
```

`model` must be a valid model slug (no alias mapping is applied). `reasoning_effort` is optional and should be one of: `minimal`, `low`, `medium`, `high`, `xhigh`.

`tools` can be a list or a comma-separated string. If present, it acts as an allowlist
that is intersected with any existing tool allowlist (it never expands beyond the
current allowlist), and denylist rules still apply. Use `*` to leave tool access
unchanged.

## JSON Schema

The generated JSON Schema for `config.toml` lives at `codex-rs/core/config.schema.json`.

## Notices

Codex stores "do not show again" flags for some UI prompts under the `[notice]` table.

Ctrl+C/Ctrl+D quitting uses a ~1 second double-press hint (`ctrl + c again to quit`).
