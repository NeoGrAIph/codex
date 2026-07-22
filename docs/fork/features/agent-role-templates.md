# Agent role templates

## Feature passport

- Code name: `agent-role-templates`
- Status: переносимая fork-возможность.
- Goal: дать sub-agents осмысленные роли и инструкции через markdown templates/persona metadata.
- Scope in: built-in role templates, manifest/authoring docs, persona/thread metadata, model instructions in markdown roles.
- Scope out: runtime limits и cwd; они описаны отдельно.

## Как работает для пользователя

Пользователь или orchestrator запускает sub-agent с понятной специализацией: explorer, worker, reviewer и т.п. Польза: agents стартуют с нужной ролью и не требуют каждый раз ручного описания роли.

## Branches and commits

| Branch | Baseline | Commit | Date | Subject | Роль |
| --- | --- | --- | --- | --- | --- |
| `fork/saw` | `rust-v0.99.0` | `a15d4adc2c` | 2026-02-13 | `[SA][SAW] finalize sub-agent templates, guards, and UI integration` | Первичная связка templates с sub-agent workflow. |
| `fork/101` | `rust-v0.101.0` | `9f82deac0f` | 2026-02-15 | `feat(sa): complete Sub-Agents fork contract (templates, thread_note, runtime listing, TUI/SAW polish)` | Полный fork contract templates/runtime listing. |
| `fork/106` | `rust-v0.106.0` | `1e5e63ff26` | 2026-02-28 | `docs: add agent manifest template and authoring instructions` | Authoring guidance для templates. |
| `fork/106` | `rust-v0.106.0` | `c402866871` | 2026-02-28 | `feat: add agent role templates and thread persona metadata` | Runtime/templates/persona metadata. |
| `fork/107` | `rust-v0.107.0` | `4469b595c8` | 2026-02-28 | `feat: add agent role templates and thread persona metadata` | Перенос на 0.107. |
| `fork/111` | `rust-v0.111.0` | `7a977ec76b` | 2026-03-06 | `feat(agents): add role templates and thread persona foundation` | Обновленный foundation для templates/persona. |
| `fork/118` | `rust-v0.118.0` | `a233952c1c` | 2026-04-03 | `Implement agent role templates across core, state, app-server, and TUI` | Сквозная интеграция core/state/app-server/TUI. |
| `fork/118` | `rust-v0.118.0` | `2e6af0d4a2` | 2026-04-04 | `Support model instructions in markdown agent roles` | Markdown role templates получают model instructions. |
| `fork/130` | `rust-v0.130.0` | `783ebf1fac` | 2026-05-14 | `feat(agents): add markdown role templates` | Перенос markdown role templates на 0.130 lineage. |
| `chrome_plugin` | `rust-v0.130.0` lineage | `783ebf1fac` | 2026-05-14 | `feat(agents): add markdown role templates` | Та же возможность в chrome-plugin lineage. |

## Upstream/native role-system milestones

| Commit | Date | Subject | Contract note |
| --- | --- | --- | --- |
| `05b960671d` | 2026-01-15 | `feat: add agent roles to collab tools (#9275)` | `spawn_agent.agent_type` появляется как preset selection for collab sub-agents. |
| `e47045c806` | 2026-02-16 | `feat: add customizable roles for multi-agents (#11917)` | User-defined roles become config-driven instead of only hardcoded presets. |
| `76283e6b4e` | 2026-02-17 | `feat: move agents config to main config (#11982)` | Role config moves into the normal `Config`/schema loader source of truth. |
| `a67660da2d` | 2026-03-11 | `Load agent metadata from role files (#14177)` | `agents/*.toml` role files become metadata-bearing authoring units discovered from config layers. |
| `4fa7d6f444` | 2026-03-12 | `Handle malformed agent role definitions nonfatally (#14488)` | Invalid role definitions are dropped with startup warnings instead of failing the whole runtime. |
| `d87d918716` | 2026-04-23 | `Resolve relative agent role config paths from layers (#19261)` | Relative `config_file` paths in `[agents.*]` are resolved against the declaring config layer. |

## Current TOML role-file format

Native TOML role files are config-layer files with three role metadata keys at the top level:

```toml
name = "reviewer"
description = "Use for focused review and risk analysis."
nickname_candidates = ["Ada", "Noether"]

developer_instructions = '''
Review the assigned change for correctness, regressions, and missing tests.
'''
model_reasoning_effort = "medium"
service_tier = "default"
```

- `name`: role name used by `spawn_agent.agent_type`; required for autodiscovered files under `agents/*.toml`, optional for files referenced by `[agents.<name>].config_file` because the config key is used as the hint.
- `description`: human-facing role description included in spawn tool guidance; required after metadata merge, and a higher-precedence role may inherit it from a lower-precedence role with the same name.
- `nickname_candidates`: optional non-empty list of unique ASCII nicknames; allowed characters are letters, digits, spaces, hyphens and underscores.
- `developer_instructions`: required for autodiscovered standalone role files so the file actually changes the spawned agent context; legacy split files referenced by `config_file` may omit it when the config declaration supplies metadata.
- Any remaining top-level keys are parsed as ordinary `ConfigToml` and become the role-specific config layer applied to the spawned child agent. Common valid examples are `model`, `model_provider`, `model_reasoning_effort`, `model_reasoning_summary`, `service_tier`, `sandbox_mode`, `approval_policy`, `default_permissions`, `developer_instructions`, `tools`, `features`, `mcp_servers`, `memories` and other current `ConfigToml` fields.
- Unknown top-level keys are not accepted because the parser uses `deny_unknown_fields` over metadata plus flattened `ConfigToml`. Tables such as `[tool_selection]` are invalid unless they exist in `ConfigToml`.
- Before applying the file as a config layer, `name`, `description` and `nickname_candidates` are removed from the TOML; they are metadata only, not runtime config keys.

Role files can be discovered automatically from `agents/*.toml` next to a config layer, for example `~/.codex/agents/worker.toml` or project `.codex/agents/reviewer.toml`. They can also be referenced explicitly:

```toml
[agents.reviewer]
description = "Review role"
config_file = "./agents/reviewer.toml"
nickname_candidates = ["Ada"]
```

Relative `config_file` paths resolve against the config layer that declares them.

## MCP limits in role TOML

Role TOML files can constrain configured MCP servers because `[mcp_servers.<name>]` is part of `ConfigToml` and the role file is applied as a config layer when the child agent is spawned. This is server-scoped MCP policy, not a universal per-role tool allowlist.

To disable an inherited MCP server for a role, the role file must provide a complete parseable MCP server table with `enabled = false`:

```toml
[mcp_servers."directus-syrm"]
url = "http://disabled.invalid/mcp"
enabled = false
```

The placeholder transport is required because standalone role files are parsed before they are merged with the base config, and an MCP server table without `command` or `url` is invalid. The placeholder must not contain real tokens or endpoint secrets; disabled servers are not expected to start.

`enabled_tools` and `disabled_tools` are tool filters within one MCP server only:

```toml
[mcp_servers."docs"]
enabled_tools = ["search", "read"]
disabled_tools = ["write"]
```

They do not make other MCP servers unavailable. To deny every MCP server for a role, add `enabled = false` tables for every server name currently configured in the inherited config. If a server must remain available, omit its disabling table so the role inherits that server from the base config rather than replacing its real transport with a placeholder. This does not deny built-in/core tools such as shell, patch, planning, image, or multi-agent collaboration tools; those still require permissions, runtime policy, or explicit role instructions.

## Implementation notes

Затрагивались template files, core agent control/registry, protocol/thread metadata, app-server projection, TUI rendering и docs. В поздних переносах это стало cross-surface feature, а не только набором markdown-файлов. В `fork/130` native TOML roles остаются primary, а markdown roles дают defaults для `model`/`reasoning_effort`; full-history fork rejects role/persona/model/reasoning/cwd overrides.

## Native coverage in rust-v0.140.0

Status: `partial`. Native release имеет role system: `apply_role_to_config`, `resolve_role_config`, built-ins `default`, `explorer`, `worker`, TOML configs, `AgentRoleConfig`, `spawn_agent.agent_type`, persisted role/nickname/path metadata and TUI labels. Не хватает fork markdown role templates/persona manifest contract, native markdown role loader, reviewer-style built-ins and markdown model-instruction template behavior.

## Porting/current-state notes

При переносе в новую ветку нужно проверять current upstream skills/agents/model instructions path. Fork templates должны использовать native template/resource loading, а не отдельный обходной loader.
