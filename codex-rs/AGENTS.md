# Codex-RS Fork

This is a fork of the original Codex CLI with an extended agent registry system.

## Fork Management

### Remotes
```bash
origin    → your fork
upstream  → https://github.com/openai/codex (original)
```

### Sync with upstream
```bash
git fetch upstream
git checkout main
git rebase upstream/main
# Resolve conflicts if any, then:
git push origin main --force-with-lease
```

### Adding new fork features
```bash
# 1. Start from fork branch
git checkout fork/colab-agents

# 2. Make changes, use fork markers for inline edits
# 3. Test
cargo test -p codex-core

# 4. Commit with "feat(fork):" prefix
git commit -m "feat(fork): description of change"

# 5. Push
git push origin fork/colab-agents
```

### Updating fork after upstream changes
```bash
# 1. Sync main with upstream
git checkout main
git fetch upstream
git rebase upstream/main
git push origin main --force-with-lease

# 2. Rebase fork branch onto updated main
git checkout fork/colab-agents
git rebase main

# 3. Resolve conflicts if any:
#    - Files with FORK markers: keep our changes
#    - New upstream files: accept upstream
#    - registry.rs, codex_*.md: always keep ours

# 4. After resolving conflicts
git rebase --continue
git push origin fork/colab-agents --force-with-lease
```

### Conflict resolution tips
- **Our files** (always keep): `registry.rs`, `tool_allowlist.rs`, `codex_*.md`, `AGENTS.md`
- **Modified files**: look for `// === FORK:` markers, keep those blocks
- **spec.rs, codex.rs, collab.rs**: merge carefully, keep fork logic
- Run tests after resolving: `cargo test -p codex-core`

### Fork markers in code
All fork-specific changes are marked with comments:
```rust
// === FORK: DESCRIPTION START ===
// ... fork code ...
// === FORK: DESCRIPTION END ===
```

Search for fork changes: `rg "FORK:" --type rust`

---

## Agent Registry System

### Overview
The agent registry replaces hard-coded `AgentRole` enum with dynamic YAML-based agent definitions loaded from `~/.codex/agents/*.md` files.

### Key Files

| File | Purpose |
|------|---------|
| `core/src/agent/registry.rs` | Main registry module (loading, parsing, apply_to_config) |
| `core/src/agent/role.rs` | Legacy enum (kept for backward compatibility) |
| `core/templates/agents/codex_*.md` | Built-in agent templates (seeded on first run) |
| `core/src/tools/handlers/collab.rs` | spawn_agent handler using registry |
| `core/src/tools/spec.rs` | Dynamic tool schema with agent descriptions |

### Agent Definition Format

```yaml
---
name: worker                    # Required: 3-64 chars, lowercase + hyphen
description: |                  # Required: shown in tool schema
  Use for execution and production work.
model: gpt-5.2-codex           # Required: model slug
reasoning_effort: medium        # Optional: low/medium/high/xhigh (inherits if omitted)
color: green                    # Required: blue/cyan/green/yellow/magenta/red
read_only: false               # Optional: sets SandboxPolicy::ReadOnly
tools:                         # Optional: allowlist (omit for all tools)
  - Read
  - Grep
tool_denylist:                 # Optional: denylist
  - spawn_agent
agent_names:                   # Optional: variants for A/B testing
  - name: strict
    description: Strict variant
  - name: lenient
    description: Lenient variant
---

System prompt content here.

<!-- agent_name: strict -->
Strict variant instructions.

<!-- agent_name: lenient -->
Lenient variant instructions.
```

### Built-in Agents

| Agent | Model | Reasoning | Special |
|-------|-------|-----------|---------|
| worker | gpt-5.2-codex | (inherit) | denylist: spawn_agent |
| explorer | gpt-5.2-codex | medium | read_only: true |
| reviewer | gpt-5.2-codex | medium | read_only: true, variants |
| architect | gpt-5.2-codex | high | — |
| bug-hunter | gpt-5.2-codex | high | read_only: true |
| orchestrator | gpt-5.2-codex | (inherit) | — |

### Priority (highest → lowest)
1. Repo: `.codex/agents/*.md` in project
2. User: `~/.codex/agents/*.md`
3. System: seeded `codex_*.md` files

### Seeding
On first run, `seed_builtin_agents()` copies `codex_*.md` templates to `~/.codex/agents/`. Seeding is skipped if any `codex_*.md` file already exists.

---

## Code Conventions

### Adding new agents
1. Create `core/templates/agents/codex_<name>.md` with YAML frontmatter
2. Add to `BUILTIN_AGENTS` array in `registry.rs`
3. Run `cargo test --package codex-core --lib -- registry`

### Modifying registry
- Keep changes minimal in existing files
- Use fork markers for inline changes
- New functionality in separate modules when possible

### Testing
```bash
# Registry tests
cargo test --package codex-core --lib -- registry

# Full core tests
cargo test -p codex-core

# Format and lint
just fmt
just fix -p codex-core
```

---

## Supported Models

From `core/models.json`, gpt-5.2-codex supports:
- `low` — fast responses
- `medium` — balanced (default)
- `high` — complex problems
- `xhigh` — extra reasoning

If `reasoning_effort` is omitted in agent YAML, it inherits from parent config.
