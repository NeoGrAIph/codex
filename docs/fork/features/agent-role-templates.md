# Agent role templates

## Feature passport

- Code name: `agent-role-templates`
- Status: первая native TUI-итерация реализована для `fork/140`; markdown/persona перенос остаётся следующим этапом.
- Goal: дать sub-agents осмысленные роли и инструкции через native role templates, чтобы пользователь мог видеть, создавать и применять специализации без ручного повторения роли в каждом prompt.
- Scope in: native TOML role catalog, built-in/user role projection, starter role template creation, TUI list/detail/create/open-file flow, spawn-agent role selection through existing `agent_type`.
- Scope out: markdown role loader, persona manifest, runtime limits и cwd; они описываются и переносятся отдельно.

## Как работает для пользователя

Пользователь открывает `/agent-roles` или `/agents`, видит built-in и user templates, создаёт новый user template в `$CODEX_HOME/agents/<role>.toml`, при необходимости открывает этот TOML-файл для редактирования и затем просит Codex запустить sub-agent с `agent_type` равным имени роли. Built-in роли остаются read-only; user roles управляются через существующий native config directory.

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

## Implementation notes

Затрагивались template files, core agent control/registry, protocol/thread metadata, app-server projection, TUI rendering и docs. В поздних переносах это стало cross-surface feature, а не только набором markdown-файлов. В `fork/130` native TOML roles остаются primary, а markdown roles дают defaults для `model`/`reasoning_effort`; full-history fork rejects role/persona/model/reasoning/cwd overrides.

## Native coverage in rust-v0.140.0 / fork/140

Status: `partial, expanded`. Native release уже имел role system: `apply_role_to_config`, `resolve_role_config`, built-ins `default`, `explorer`, `worker`, TOML configs, `AgentRoleConfig`, `spawn_agent.agent_type`, persisted role/nickname/path metadata and TUI labels. В `feature/140/agent-role-templates` добавлен native projection/catalog для этих ролей и TUI управление templates без нового config format. Не хватает fork markdown role templates/persona manifest contract, native markdown role loader, reviewer-style built-ins and markdown model-instruction template behavior.

## Native integration map

| Surface | Source of truth / behavior |
| --- | --- |
| Source of truth | `Config.agent_roles` плюс built-ins из `codex-rs/core/src/agent/role.rs`; новый `agent_role_templates` только проецирует этот источник и создаёт user TOML-файл. |
| Config/profile surface | `$CODEX_HOME/agents/*.toml` и существующий flattened `AgentRoleConfig`; отдельный profile registry не добавляется. |
| Spawn tool spec/handlers | `spawn_agent.agent_type` остаётся native selector; schema/list продолжает строиться из role config через существующий multi-agent path. |
| Runtime state | `apply_role_to_config` и `resolve_role_config` остаются местом применения role defaults; TUI не хранит отдельную role enum/list. |
| TUI projection | `/agent-roles` и alias `/agents` открывают list/detail/create flow; `/agent` и `/subagents` продолжают управлять active sub-agent windows. |
| App-server protocol | Не менялся: role templates остаются config/runtime capability, wire contract не расширен. |
| Persistence/resume | Новые role files сохраняются как `$CODEX_HOME/agents/*.toml`; existing session metadata с `agent_role` остаётся строковым и backward-compatible. |
| Tool/model/policy defaults | Модель, reasoning, service tier, tools/skills и developer instructions наследуются из native role config при spawn; policy/env behavior не менялся. |
| Intentionally unaffected | Permission profiles, sandbox/env selection, mailbox/thread-store format, app-server schema, existing `/agent` sub-agent navigation. |

## Porting/current-state notes

При переносе в новую ветку нужно проверять current upstream skills/agents/model instructions path. Fork templates должны использовать native template/resource loading, а не отдельный обходной loader.

## Verification matrix

| Surface | Проверка |
| --- | --- |
| Core role template projection/create | `just test -p codex-core agent_role_templates` |
| TUI slash/popup/create snapshot | `just test -p codex-tui agent_role` |
| Snapshot acceptance | `cargo insta show tui/src/chatwidget/snapshots/codex_tui__chatwidget__tests__agent_role_templates_popup.snap.new` и `cargo insta accept --snapshot 'tui/src/chatwidget/snapshots/codex_tui__chatwidget__tests__agent_role_templates_popup.snap'` при intentional UI diff. |
| Formatting | `just fmt` в `codex-rs` |
| Diff hygiene | `git diff --check` |

## Doc changelog

- 2026-06-17: зафиксирована первая `fork/140` итерация: native catalog/create TUI для TOML role templates, без app-server protocol change и без markdown/persona loader.
