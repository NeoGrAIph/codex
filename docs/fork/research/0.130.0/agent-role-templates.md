# Исследование нативной реализации Agent Role Templates для rust-v0.130.0

## Базовая точка релиза

- Release/tag: `rust-v0.130.0`.
- Dereferenced commit: `58573da43ab697e8b79f152c53df4b42230395a8`.
- Проверка локальной базы: `git show -s --format=%H%n%D%n%ci rust-v0.130.0^{}` вернул этот commit, refs `HEAD -> fork/130`, `tag: rust-v0.130.0`, `fork/130-upstream`, дата `2026-05-08 14:57:54 -0700`.
- Исследование ориентировано на upstream-shaped adaptation поверх 0.130, а не на literal backport исторического markdown runtime.

## Базовое описание

`agent-role-templates` добавляет markdown-backed role templates и personas для spawned agents.

Каноничное решение для porting на 0.130: native TOML role pipeline остается primary. Markdown templates - augmentation layer поверх native roles, а не replacement для upstream role resolution или config layering.

## Текущее состояние 0.130

0.130 уже имеет зрелый native role pipeline:

- Role loading сосредоточен в `codex-rs/core/src/config/agent_roles.rs`.
- Role application сосредоточен в `codex-rs/core/src/agent/role.rs`.
- Native roles backed by TOML через `[agents.<role>]` и standalone `.codex/agents/*.toml`.
- Role metadata включает только `description`, `config_file` и `nickname_candidates`; `agent_persona` и policy не представлены в `AgentRoleConfig`.
- Standalone role files могут содержать обычные `ConfigToml` fields: `developer_instructions`, `model`, `model_reasoning_effort`, `sandbox_mode`, profile/provider settings.
- `apply_role_to_config()` вставляет role как high-precedence `SessionFlags` config layer, сохраняя current profile/provider, если role явно не владеет этими fields.
- Spawn order в v1/v2 сейчас такой: parent runtime snapshot -> explicit `model`/`reasoning_effort` request -> `apply_role_to_config()` -> runtime permission/cwd overrides -> depth overrides. Поэтому native TOML role-owned `model`/`model_reasoning_effort` фактически сильнее explicit spawn args и документируется в tool description как locked settings.

Spawn integration:

- v1 handler: `multi_agents/spawn.rs`
- v2 handler: `multi_agents_v2/spawn.rs`
- tool schema: `multi_agents_spec.rs`
- existing args включают `agent_type`, `model` и `reasoning_effort`; v2 также имеет `task_name` и `fork_turns`
- `agent_persona` не существует
- v2 `SpawnAgentArgs` использует `#[serde(deny_unknown_fields)]`, поэтому `agent_persona` нужно добавлять одновременно в schema и args, иначе модельные calls будут отклоняться.
- full-history forked agents reject role/model/reasoning overrides и наследуют parent settings; persona/template overrides должны следовать тому же правилу, если не будет явно задокументирован иной contract.

Persistence/projection:

- Canonical sub-agent metadata - `SessionSource::SubAgent(SubAgentSource::ThreadSpawn { ... })`.
- Existing persisted/projected fields: `agent_nickname`, `agent_role` и `agent_path`.
- SQLite, thread-store, TUI и app-server project nickname/role/path, но не persona или policy.
- `AgentControl::resume_agent_from_rollout()` восстанавливает descendant tree по `thread_spawn_edges`, а stale descendant source может быть rehydrated from edge data; новое persona metadata нельзя терять в этом пути.

Существующая markdown-adjacent upstream support:

- `codex-rs/external-agent-migration` мигрирует external `.claude/agents/*.md` в native `.codex/agents/*.toml`.
- Это import-only, не runtime markdown template loading. Миграция читает markdown frontmatter/body и пишет TOML с `developer_instructions`, опционально `model_reasoning_effort` и `sandbox_mode`.

## Gap analysis для fork feature

Что уже покрыто upstream 0.130:

- Native TOML role discovery, validation, config layering и spawn tool role description.
- Built-in roles (`default`, `explorer`, `worker`) и user-defined TOML roles.
- Role-specific nickname candidates.
- Spawn/resume/tree lifecycle через `SubAgentSource::ThreadSpawn`, `thread_spawn_edges` и `AgentControl`.
- MCP tool allow/deny на server config уровне через `enabled_tools`/`disabled_tools`, включая model-visible filtering и dispatch-time block.

Что отсутствует для `agent-role-templates`:

- Runtime discovery/parsing `.md` role templates.
- `agent_persona` как model-facing `spawn_agent` parameter.
- Persona selection, default persona, validation of named personas и developer-instruction append semantics.
- Template defaults for model/reasoning that are weaker than explicit spawn args and weaker than native TOML locked settings.
- Template-scoped tool policy for built-in, dynamic, MCP and tool-search/deferred surfaces.
- Optional persona metadata on spawned-agent session source; policy remains runtime-only.
- App-server/Codex app compatibility story for new metadata fields.

Intentional divergence from historical behavior:

- Native TOML role pipeline is primary. Markdown may augment a resolved TOML role or define a template-only role only when no native TOML role exists and the markdown contract is valid.
- Markdown templates use `.codex/.agents/*.md` only. The `agents/*.md` path is intentionally unsupported so markdown templates do not collide with 0.130 native `.codex/agents/*.toml` roles.

## Направление нативной реализации

Реализовать markdown templates как fork-owned augmentation layer.

- Добавить parser/registry рядом с native role code, например `core/src/agent/role_templates.rs`.
- Не расширять `ConfigToml` для markdown-only semantics без необходимости. Если все же добавляются TOML/schema fields, нужен `just write-config-schema` и минимальный generated diff.
- Discover only:
  - project `.codex/.agents/*.md`
  - user `~/.codex/.agents/*.md`
- Сначала resolve native TOML role через existing `resolve_role_config()` / `apply_role_to_config()` path, затем apply markdown augmentation.
- Разрешать template-only roles только когда native TOML role не существует и markdown template валиден; такие роли должны появляться в spawn tool description вместе с TOML roles.
- Для exact role name использовать exact markdown match before `-` / `_` canonical fallback. Ambiguous fallback должен fail controlled diagnostic, а не silently pick one.

Recommended application order for non-fork spawn:

1. Build child config from parent turn with `build_agent_spawn_config()`.
2. Apply explicit spawn `model`/`reasoning_effort` overrides.
3. Apply native TOML role via `apply_role_to_config()`.
4. Apply markdown persona augmentation to developer/base instructions.
5. Apply markdown `model`/`reasoning_effort` defaults only when explicit spawn args are absent and native TOML role did not own those fields.
6. Re-apply runtime overrides that must remain parent-owned (`cwd`, shell env, current approval baseline).
7. Tighten permissions/tool policy after the final runtime refresh so markdown `read_only`/policy cannot be relaxed.

Template semantics:

- Strict YAML frontmatter плюс marker-delimited persona blocks.
- `default` persona обязательна.
- Unknown keys, missing blocks, duplicate blocks и stray markdown invalid.
- `agent_persona` выбирает named persona; omitted означает `default`.
- Persona text append к child developer instructions и не replace base/system instructions.
- `model_instructions_file` резолвится относительно markdown file и задает child `Config.base_instructions`.
- Template `model` и `reasoning_effort` - только defaults; explicit spawn args сильнее markdown defaults.
- Native TOML role-owned model/reasoning остается stronger than markdown defaults, если contract явно не меняется.
- Template `description` должен использоваться в spawn tool role description для template-only roles и как augmentation note для TOML-backed roles, не замещая native TOML `description`.

Policy:

- `read_only` может только ужесточать runtime permissions.
- `read_only` должен опираться на canonical `PermissionProfile`/runtime permission pipeline, а не на legacy-only `sandbox_mode` mutation.
- `allow_list` и `deny_list` должны filter model-visible tool specs и block dispatch.
- `deny_list` wins over `allow_list`.
- Policy остается runtime-only в `Config` for v1; resume does not expose or reconstruct it from
  public source metadata.
- Policy должна покрывать все tool surfaces, а не только MCP:
  - builtin/function/freeform/local shell tools from `codex-rs/core/src/tools/spec.rs`, `spec_plan.rs` и `router.rs`;
  - deferred/tool-search surfaces from `tool_search_entry.rs` and `ToolRouter::model_visible_specs()`;
  - dynamic tools from `codex-rs/core/src/tools/handlers/dynamic.rs`;
  - MCP tools from `codex-rs/codex-mcp/src/tools.rs` and `connection_manager.rs`.
- Existing MCP `enabled_tools`/`disabled_tools` is server-scoped. Template policy is spawn/session-scoped, so it should be layered at `ToolRouter`/dispatch boundary or carried into an equivalent per-turn filter; do not mutate global MCP server config for one sub-agent.

Persistence:

- Расширить canonical spawned-agent metadata только `agent_persona`; policy stays runtime-only to
  avoid expanding public source metadata.
- Добавить SQLite projection только для display/query fields, вероятно `agent_persona`; policy лучше не projecting unless needed for query/UI.
- Resume remains backward compatible when optional persona metadata is absent.
- Direct/subtree resume persona restoration must be backed by focused tests before being treated as
  a guaranteed contract; policy remains runtime-only and is not reconstructed from public metadata.
- Persisted policy reconstruction is deferred; v1 keeps policy as runtime-only child config state
  and documents that boundary instead of adding a hidden persisted-only shape.

## Risky integration points и source-of-truth files

Primary source of truth:

- `codex-rs/core/src/config/agent_roles.rs` - TOML role loading, validation, `.codex/agents/*.toml` discovery, required descriptions.
- `codex-rs/core/src/agent/role.rs` - native role application, config layer precedence, spawn tool role text.
- `codex-rs/config/src/config_toml.rs` - `ConfigToml`, `AgentsToml`, `AgentRoleToml`; change only if markdown semantics intentionally become config schema.

Spawn and lifecycle:

- `codex-rs/core/src/tools/handlers/multi_agents/spawn.rs`
- `codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs`
- `codex-rs/core/src/tools/handlers/multi_agents_spec.rs`
- `codex-rs/core/src/tools/handlers/multi_agents_common.rs`
- `codex-rs/core/src/agent/control.rs`

Persistence/resume/projection:

- `codex-rs/protocol/src/protocol.rs` (`SubAgentSource::ThreadSpawn`, `SessionMeta`, collab events).
- `codex-rs/state/src/model/thread_metadata.rs`
- `codex-rs/state/src/runtime/threads.rs`
- `codex-rs/thread-store/src/local/read_thread.rs`
- `codex-rs/thread-store/src/local/update_thread_metadata.rs`
- `codex-rs/app-server-protocol/schema/typescript/SubAgentSource.ts` and related generated app-server schema if protocol fields change.

Tool policy:

- `codex-rs/core/src/tools/router.rs`
- `codex-rs/core/src/tools/spec.rs`
- `codex-rs/core/src/tools/spec_plan.rs`
- `codex-rs/core/src/tools/tool_search_entry.rs`
- `codex-rs/core/src/tools/handlers/dynamic.rs`
- `codex-rs/codex-mcp/src/tools.rs`
- `codex-rs/codex-mcp/src/connection_manager.rs`
- `codex-rs/core-plugins/src/loader.rs` as prior art for plugin MCP policy mapping, not as session-scoped template policy storage.

Migration/compat prior art:

- `codex-rs/external-agent-migration/src/lib.rs` - markdown import into TOML; keep import-only behavior separate from runtime markdown templates.

## Совместимость Codex App / App-server

Feature должен работать в app-server sessions без изменений Codex app.

- `agent_persona` - model-facing `spawn_agent` parameter, а не новый app-server client request.
- Stable app-server clients должны продолжать работать, если они ignore persona.
- Избегать app-server protocol changes, если projection не нужна.
- Если projection нужна, использовать только optional fields и явно классифицировать stable vs experimental.
- Аккуратно менять `SubAgentSource::ThreadSpawn`, потому что app-server exposes core `SubAgentSource` через `SessionSource::SubAgent`.
- `SubAgentSource.ts` currently exposes only `parent_thread_id`, `depth`, `agent_path`, `agent_nickname`, `agent_role`; adding `agent_persona` или policy changes generated stable TypeScript unless field is explicitly hidden from TS.
- Если `agent_persona` нужен только runtime/model correctness, prefer persisted optional metadata that old clients can ignore. Если он нужен UI/listing, add optional schema field and update app-server compatibility matrix.
- Policy не должна требовать Codex app client enforcement. Core must enforce filtering/blocking server-side even when app client is old.
- Production Codex app clients не должны нуждаться в `experimentalApi` для core correctness.
- Avoid requiring Codex app to pass persona/policy in initial app-server requests; model-visible tool schema is enough for spawned-agent creation.

## Необходимые тесты и артефакты

## Реализация в fork/130

- Markdown runtime uses the historical `fork/118` frontmatter shape: `description`, `agent_names`,
  optional `model_instructions_file`, `model`, `reasoning_effort`, `read_only`, `allow_list`,
  and `deny_list`.
- Persona blocks use `<!-- agent_nickname: <name> -->` markers.
- `SubAgentSource::ThreadSpawn` now carries optional `agent_persona`; tool policy is kept in the
  spawned session runtime config and is not exposed as public source metadata. No SQLite migration
  or new DB column is introduced.
- App-server v2 exposes optional `Thread.agentPersona` as additive stable metadata for Codex app
  clients.
- Tool policy uses existing workspace `wildmatch` masks and is enforced in model-visible specs,
  tool-search entries, and dispatch.

Core/parser:

- Valid template loads with default and named persona.
- Malformed template skipped with warning for discovery, but selected invalid/unknown `agent_persona` fails controlled diagnostic.
- Template-only role can spawn when valid.
- Missing default или unknown persona fails.
- Omitted `agent_persona` uses default.
- Non-default persona appends developer instructions.
- `model_instructions_file` replaces child base instructions.
- Explicit spawn model/reasoning beats markdown defaults.
- Native TOML role-owned settings remain primary.
- Exact role name beats canonical fallback; ambiguous fallback fails.
- `.codex/.agents/*.md` is the only supported markdown template path.

Policy:

- `read_only` only tightens permissions.
- `allow_list` hides non-allowed tools.
- `deny_list` hides and blocks denied tools.
- Dispatch-time blocked tool returns controlled failure.
- MCP policy is session-scoped and does not mutate global `McpServerConfig`.
- Deferred/tool-search and dynamic tools are filtered consistently with direct model-visible specs.

Persistence/resume:

- Rollout/session metadata can carry optional persona through `SessionMeta.source`; policy remains
  runtime-only to avoid public source/schema exposure.
- No SQLite projection or migration is added for `agent_persona` or policy.
- Direct and subtree resume stay backward compatible when persona metadata is absent.
- Persona restoration for resumed descendants requires focused coverage before being treated as a
  guaranteed contract.
- Older rollouts without persona deserialize with `None` and keep current behavior.

App-server/TUI:

- `cargo test -p codex-app-server-protocol`, если schema changes.
- `just write-app-server-schema` для stable schema changes.
- `just write-app-server-schema --experimental` для experimental schema changes.
- `cargo test -p codex-app-server` for thread metadata / collab projection changes.
- TUI snapshots для persona labels, если rendered.

Suggested implementation verification commands:

- `cargo test -p codex-core agent_role`
- `cargo test -p codex-core multi_agents`
- `cargo test -p codex-state threads`
- `cargo test -p codex-thread-store`
- `cargo test -p codex-app-server-protocol` if `protocol.rs`/schema changes.
- `just write-config-schema` if `ConfigToml` or nested config types change.
- `just write-app-server-schema` or `just write-app-server-schema --experimental` if app-server protocol schema changes.
- `cargo test -p codex-tui` plus `cargo insta pending-snapshots -p codex-tui` if UI text/rendering changes.

## Открытые риски

- Historical templates ожидают, что explicit spawn args сильнее template defaults, тогда как native TOML role layers могут намеренно lock model/reasoning. Implementation должен keep TOML primary, делая markdown defaults weaker.
- Добавление fields в core `SubAgentSource::ThreadSpawn` может leak into app-server stable schema.
- Tool-policy filtering без dispatch enforcement недостаточен.
- `.codex/.agents` is intentionally kept as the markdown template path; `agents/*.md` remains unsupported to avoid conflict with native `.codex/agents/*.toml`.
- Runtime markdown templates не должны конфликтовать с external-agent migration, который writes TOML roles.
- Runtime-only policy does not survive a fresh resume in v1. This is documented as an intentional
  boundary so the feature can avoid adding policy to public source/app-server schema.
- `apply_spawn_agent_runtime_overrides()` сейчас reapplies parent runtime permissions after role application. Markdown `read_only` must be applied after this point or represented as a monotonic tightening that cannot be relaxed by later runtime refresh.
- Template-only roles need a spawn tool description source. Если registry lives outside `Config.agent_roles`, `spawn_tool_spec::build()` must merge it without making markdown look like native TOML in `ConfigToml`.
