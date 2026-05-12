# Исследование нативной реализации Subagent CWD для rust-v0.130.0

## Baseline release

- Upstream tag: `rust-v0.130.0`.
- Dereferenced release commit: `58573da43ab697e8b79f152c53df4b42230395a8`.
- Локальная проверка baseline: `git rev-parse rust-v0.130.0^{}`.
- Research package: `docs/fork/research/0.130.0/`.

## Базовое описание

`subagent-cwd` добавляет optional `cwd` в `spawn_agent`, чтобы child agent мог стартовать из собственного session root, вместо того чтобы всегда наследовать parent turn cwd.

Контракт:

- Отсутствующий `cwd` сохраняет текущее поведение.
- Relative `cwd` резолвится относительно parent `turn.cwd`.
- Absolute `cwd` используется напрямую.
- Применяется Policy B: child cwd может быть любой валидной on-disk directory, не только внутри parent workspace.
- Target directory не создается автоматически.
- Invalid paths или invalid child config construction fail fast без fallback к parent cwd.
- Resolved cwd - это child session root, а не command-only override.
- `thread/start`, `thread/resume` и `thread/fork` вне scope.

## Текущее состояние 0.130

`fork/130` не реализует explicit child cwd.

Проверенные integration points:

- `codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs::SpawnAgentArgs` не имеет `cwd` и использует `#[serde(deny_unknown_fields)]`.
- Legacy v1 `codex-rs/core/src/tools/handlers/multi_agents/spawn.rs::SpawnAgentArgs` тоже не имеет `cwd`; serde unknown fields не запрещены на struct level, но runtime tool schema сейчас не объявляет `cwd`.
- Tool schemas в `codex-rs/core/src/tools/handlers/multi_agents_spec.rs` не экспонируют `cwd` ни во input, ни в output.
- `codex-rs/core/src/tools/handlers/multi_agents_common.rs::build_agent_spawn_config()` идет через `build_agent_shared_config()` и клонирует parent `turn.config`.
- `apply_spawn_agent_runtime_overrides()` явно задает `config.cwd = turn.cwd.clone()`, что делает inherited cwd текущим upstream behavior.
- Оба handler-а (`multi_agents/spawn.rs` и `multi_agents_v2/spawn.rs`) вызывают `apply_role_to_config()` перед повторным `apply_spawn_agent_runtime_overrides()`, поэтому role reload не может сохранить alternative child cwd без изменения ordering/override contract.
- Оба handler-а передают `SpawnAgentOptions { environments: Some(turn.environments.to_selections()), ... }`; при explicit child cwd это напрямую переносит parent environment cwd.
- `codex-rs/core/src/thread_manager.rs` строит default environments из `config.cwd` только когда environments не переданы. В `ThreadManagerState::spawn_new_thread_with_source()` и `fork_thread_with_source()` `None` вызывает `default_thread_environment_selections(...)`.
- `codex-rs/core/src/environment_selection.rs::default_thread_environment_selections()` ставит `TurnEnvironmentSelection.cwd` равным переданному cwd, поэтому это правильный source of truth для default local/remote environment selection.
- `codex-rs/core/src/config/mod.rs::ConfigBuilder` умеет cwd-scoped config loading через `ConfigOverrides.cwd`/`fallback_cwd`, но relative cwd там резолвится относительно process current dir. Для `spawn_agent.cwd` relative path должен быть заранее resolved относительно parent `turn.cwd`.
- `codex-rs/utils/absolute-path/src/lib.rs::AbsolutePathBuf` гарантирует absolute normalized path, но не гарантирует существование path на filesystem. Нужна отдельная validation, что resolved cwd существует и является directory.

Существующие surfaces уже экспонируют thread cwd:

- `codex-rs/protocol/src/protocol.rs::SessionConfiguredEvent.cwd`
- `codex-rs/protocol/src/protocol.rs::SessionMeta.cwd`
- `codex-rs/thread-store/src/types.rs::ThreadPersistenceMetadata.cwd`
- app-server `codex-rs/app-server-protocol/src/protocol/v2/thread_data.rs::Thread.cwd`
- app-server `ThreadStartResponse.cwd`, `ThreadResumeResponse.cwd`, `ThreadListResponse.data[].cwd`, `ThreadReadResponse.thread.cwd`, `ThreadStartedNotification.thread.cwd`

App-server v2 находится в `codex-rs/app-server-protocol/src/protocol/v2/`, а не в устаревшем пути `v2.rs`.

## Gap analysis

- Input contract gap: `spawn_agent` не принимает `cwd`; v2 с `deny_unknown_fields` будет reject unknown `cwd`, а v1 проигнорирует unknown на deserialize path при несовпадении со schema expectations.
- Runtime source-of-truth gap: current child config всегда получает `turn.cwd` через `apply_spawn_agent_runtime_overrides()`.
- Config loading gap: shallow mutation `config.cwd` не перезагружает cwd-scoped project config, trust/project policy, plugins/MCP/hooks/skills/AGENTS-derived layers и relative-path-derived settings.
- Resolver gap: существующий config loader relative cwd резолвит от process cwd, а контракт требует parent `turn.cwd`.
- Validation gap: `AbsolutePathBuf` не проверяет on-disk directory, значит nonexistent/file cwd должны fail fast отдельной проверкой до spawn.
- Environment gap: inherited `turn.environments.to_selections()` может сохранить parent cwd даже при child `config.cwd`.
- Resume gap: `codex-rs/core/src/agent/control.rs::resume_agent_from_rollout()` сейчас прокидывает `config.clone()` потомкам; без rebuild из stored child cwd descendants могут resume с cwd root/parent thread.
- App-server gap: app-server already exposes effective thread cwd, но `ThreadItem::CollabAgentToolCall` и collab spawn events не несут cwd snapshot. Это не blocker для первой реализации, если compatibility contract не требует cwd прямо в tool-call history item.

## Направление нативной реализации

Не реализовывать это как shallow mutation `config.cwd`. Explicit child cwd требует rebuilding cwd-scoped config state.

Рекомендуемая реализация:

- Добавить `cwd: Option<PathBuf>` в v1 и v2 `SpawnAgentArgs`; в tool schemas это string field.
- Добавить `cwd` в `spawn_agent_output_schema_v1()` и обе ветки `spawn_agent_output_schema_v2(...)`, если контракт требует возвращать effective cwd в tool output. Если path disclosure нежелателен для hidden metadata mode, зафиксировать это отдельным compatibility decision и не полагаться на implicit omission.
- Добавить общий cwd resolver рядом с `multi_agents_common.rs` или в узком helper module:
  - omitted -> parent `turn.cwd`
  - relative -> parent `turn.cwd` joined with relative path
  - absolute -> as-is
  - normalize через platform-native path normalization
  - validate как absolute existing directory; не создавать directory автоматически
  - error message должен быть model-facing и конкретный: invalid path, not found или not directory
- Добавить child config builder для resolved cwd:
  - rebuild config layer stack для child cwd
  - preserve session/runtime-owned values: model/provider/reasoning, base/developer instructions, compact prompt, approvals, sandbox/permission selection, shell env policy и sandbox executable
  - preserve session layers intentionally, but load user/project cwd-scoped layers from child cwd
  - `Config::rebuild_preserving_session_layers()` is close but same-cwd-oriented: it uses `self.cwd` for final `ConfigOverrides.cwd`. Do not call it naively with parent config unless the helper is extended or the seed config already has resolved child cwd.
  - apply role config after child cwd rebuild, so `role.config_file` and project scoped role effects are evaluated against the child config state
  - apply explicit spawn overrides last
- Изменить runtime overrides так, чтобы explicit child cwd не перезаписывался обратно в parent `turn.cwd`. Практичный вариант: split helper на "runtime policy overrides" и "cwd override", либо передавать resolved cwd как параметр.
- Аккуратно обработать environments:
  - omitted `cwd` должен сохранить текущее inherited environment behavior: `Some(turn.environments.to_selections())`
  - explicit `cwd` не должен напрямую наследовать parent environment selections
  - preferred default для explicit cwd: передать `environments: None`, чтобы `ThreadManagerState` построил `default_thread_environment_selections(..., &config.cwd)`
  - если выбран re-root existing selections, тест должен доказать, что все локальные selection cwd указывают на resolved child cwd, а remote/disabled semantics не меняются случайно
- Не auto-trust child cwd. Trust по-прежнему контролируется config/project policy.
- Не использовать fallback к parent cwd после ошибки config rebuild или validation. Controlled error лучше, чем silent parent cwd.
- Добавить optional cwd в core collab spawn begin/end events только если TUI/history нужен этот snapshot.
- Resume descendants должны rebuild child config из сохраненного child cwd / `SessionMeta.cwd`; иначе restart может silently revert children к parent cwd.

## Risky source-of-truth files

- Tool args and behavior: `codex-rs/core/src/tools/handlers/multi_agents/spawn.rs`, `codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs`.
- Shared spawn config/runtime state: `codex-rs/core/src/tools/handlers/multi_agents_common.rs`.
- Tool schemas/tests: `codex-rs/core/src/tools/handlers/multi_agents_spec.rs`, `codex-rs/core/src/tools/handlers/multi_agents_spec_tests.rs`, `codex-rs/core/src/tools/spec_plan_tests.rs`.
- Cwd-scoped config and validation helpers: `codex-rs/core/src/config/mod.rs`, `codex-rs/utils/absolute-path/src/lib.rs`, `codex-rs/utils/path-utils/src/lib.rs`.
- Role layering: `codex-rs/core/src/agent/role.rs`.
- Spawn/fork/resume orchestration: `codex-rs/core/src/agent/control.rs`, `codex-rs/core/src/thread_manager.rs`.
- Environment selections: `codex-rs/core/src/environment_selection.rs`.
- Persistence and resume metadata: `codex-rs/protocol/src/protocol.rs`, `codex-rs/thread-store/src/types.rs`, `codex-rs/core/src/session/session.rs`.
- App-server compatibility surfaces: `codex-rs/app-server-protocol/src/protocol/v2/thread_data.rs`, `codex-rs/app-server-protocol/src/protocol/v2/thread.rs`, `codex-rs/app-server-protocol/src/protocol/v2/item.rs`, `codex-rs/app-server-protocol/src/protocol/thread_history.rs`, `codex-rs/app-server/src/request_processors/thread_processor.rs`, `codex-rs/app-server/src/config_manager.rs`.

## Совместимость Codex App / App-server

Codex app не вызывает `spawn_agent` напрямую. `cwd` в model tool input/output не требует изменений Codex app.

Существующий app-server `Thread.cwd` уже экспонирует effective thread cwd через `thread/started`, `thread/read` и `thread/list`. Если child threads создаются с корректным `config.cwd`, Codex app clients могут наблюдать cwd через существующие stable surfaces.

Избегать app-server protocol changes для первой реализации:

- Не менять `thread/start`, `thread/resume` или `thread/fork`.
- Не добавлять `cwd` в `ThreadItem::CollabAgentToolCall`, если отдельное compatibility decision этого не требует.
- Если collab item cwd будет добавлен позже, он должен быть optional, а schema должна быть regenerated. Experimental gating может потребовать дополнительного nested field filtering support.
- Если меняются core `CollabAgentSpawnBeginEvent`/`CollabAgentSpawnEndEvent`, проверить app-server event mapping в `protocol/thread_history.rs` и `protocol/event_mapping.rs`; иначе app-server history может потерять новое поле или потребовать schema update.

## Необходимые тесты и артефакты

- Tool schema tests, доказывающие, что v1/v2 input включает optional `cwd`; output tests нужны только если `SpawnAgentResult` возвращает effective cwd.
- Core tests для omitted, relative, absolute, outside-workspace, nonexistent и file-path cwd.
- Tests, доказывающие, что child `config.cwd`, first turn `TurnContext.cwd`, sandbox/profile cwd и `TurnEnvironmentSelection.cwd` совпадают с resolved child cwd.
- Tests, доказывающие, что cwd-scoped project config/role config загружается из child cwd, а не parent cwd. Минимальный fixture: parent и child directories с разными project config/role-visible values.
- Tests, доказывающие, что explicit child cwd не auto-trust-ится и не получает parent trust/root write policy без явного config policy.
- Resume tests, доказывающие, что spawned descendants восстанавливаются из stored child cwd, включая descendant of descendant.
- Fork tests, если `fork_turns=all`/`fork_context` допускает `cwd`: full-history content should fork, but runtime/session root must be resolved child cwd.
- App-server tests только если есть app-server item/schema changes.
- Команды после реализации:
  - `cargo test -p codex-core multi_agents`
  - `cargo test -p codex-core agent::control`
  - `cargo test -p codex-core thread_manager`
  - `cargo test -p codex-tools`
  - `cargo test -p codex-app-server-protocol`, если protocol changes
  - `just write-app-server-schema`, если app-server protocol changes

## Открытые риски

- Нет готового existing helper, который rebuild config layer stack для другого cwd, сохраняя ровно правильные runtime/session layers. `rebuild_preserving_session_layers()` полезен как reference, но same-cwd assumption надо снять явно.
- Permission profile semantics cwd-sensitive и не должны деградировать.
- Environment selections могут silently keep parent cwd, если inherited напрямую.
- Role application ordering важен; applying role before child cwd rebuild может загрузить wrong project state.
- Full-history fork semantics требуют решения: разрешить `cwd` как runtime/session-root override при forked history или reject его вместе с other overrides. Решение должно быть documented и покрыто тестом.
- Effective cwd в model-visible `SpawnAgentResult` может раскрывать absolute local path; если это считается metadata hiding issue для v2 hidden mode, оставить observation через existing `Thread.cwd` app-server surfaces и зафиксировать omission.
