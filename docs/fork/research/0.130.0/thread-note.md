# Исследование нативной реализации Thread Note для rust-v0.130.0

## Baseline

- Release tag: `rust-v0.130.0`.
- Dereferenced baseline commit: `58573da43ab697e8b79f152c53df4b42230395a8`.
- Baseline commit date: `2026-05-08 14:57:54 -0700`.
- Проверено локально через `git rev-parse rust-v0.130.0^{}` и `git show -s --format=%H%n%ci%n%s rust-v0.130.0^{}`.
- Target research package: `docs/fork/research/0.130.0/`.

Historical fork reference:
[`35187c0529`](https://github.com/NeoGrAIph/codex/commit/35187c0529b8f0797cd9460c714167c06d60f24b)
(`fork/118`, `feat(thread-note): add metadata-only thread note support`). This commit adds
file-backed `thread_note` persistence through `codex-rs/rollout/src/session_index.rs` without
adding a `codex-rs/state/migrations` thread-note migration.

## Базовое описание

`thread_note` - это fork-native thread-owned metadata для multi-agent threads. Он делает назначение и компетенции thread видимыми для людей и orchestration surfaces, но не должен менять model-facing контекст, инструкции или routing semantics.

Каноничные инварианты:

- `thread_note` - metadata, а не prompt material. Его нельзя внедрять в developer instructions, user instructions, `EnvironmentContext`, tool hints, settings updates, `InterAgentCommunication` prompt или hidden task prompts.
- Пустой ввод или ввод только из whitespace очищает note.
- Plain text нормализуется в `Назначение: <text> | Компетенции:`.
- `thread_note_competencies` является input-only удобством для tools и сворачивается в единый
  persisted `thread_note`; отдельного persisted/app-server поля для competencies нет.
- Структурированный ввод `Назначение: ... | Компетенции: ...` сохраняет смысл, нормализуя пробелы.
- Note должен переживать spawn, resume, rollout replay, thread-history reconstruction и restart-safe app-server list/read, если note экспонируется в `Thread`.
- Старые клиенты должны оставаться совместимыми, если они не знают об этом поле.

В `0.130` layout app-server protocol модульный. Старый путь `app-server-protocol/src/protocol/v2.rs` устарел; релевантные файлы находятся в `codex-rs/app-server-protocol/src/protocol/v2/`, а event/history mapping - в `protocol/event_mapping.rs` и `protocol/thread_history.rs`.

## Текущее состояние 0.130

`rg -n "thread_note|threadNote|ThreadNote|set_thread_note" codex-rs` не нашел реализации. В baseline нет `thread_note`, `threadNote`, `ThreadNote` или `set_thread_note`.

Существующая metadata архитектура:

- `codex-rs/protocol/src/protocol.rs` содержит `SessionSource`, `SubAgentSource::ThreadSpawn`, `SessionMeta`, `TurnContextItem` и collab events. `SubAgentSource::ThreadSpawn` сейчас хранит `parent_thread_id`, `depth`, `agent_path`, `agent_nickname`, `agent_role`. `SessionMeta` дублирует `agent_nickname`, `agent_role`, `agent_path` как top-level rollout metadata. `TurnContextItem` хранит model/runtime context и не должен становиться source of truth для note.
- `codex-rs/core/src/tools/handlers/multi_agents_common.rs::thread_spawn_source` строит `SessionSource::SubAgent(SubAgentSource::ThreadSpawn { ... })`; сейчас note туда не передается.
- `codex-rs/core/src/tools/handlers/multi_agents/spawn.rs` реализует v1 `spawn_agent`. Его `SpawnAgentArgs` не использует `deny_unknown_fields`, но schema все равно должна быть расширена в `multi_agents_spec.rs`.
- `codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs` реализует v2 `spawn_agent`. Его `SpawnAgentArgs` помечен `#[serde(deny_unknown_fields)]`, поэтому `thread_note` нужно добавить и в Rust args, и в `create_spawn_agent_tool_v2` schema.
- `codex-rs/core/src/agent/registry.rs::AgentMetadata` хранит `agent_id`, `agent_path`, `agent_nickname`, `agent_role`, `last_task_message`, но не note.
- `codex-rs/rollout/src/recorder.rs`, `codex-rs/rollout/src/metadata.rs`, `codex-rs/rollout/src/list.rs`, `codex-rs/state/src/extract.rs`, `codex-rs/state/src/model/thread_metadata.rs`, `codex-rs/state/src/runtime/threads.rs` уже переносят `agent_nickname`/`agent_role`/`agent_path` между rollout и SQLite metadata. Для `thread_note` локальные fork-ветки показывают более простой путь без migration: rollout metadata плюс append-only latest-value index under `codex_home`.
- `codex-rs/app-server-protocol/src/protocol/v2/thread_data.rs::Thread` экспонирует `source`, `thread_source`, `agent_nickname`, `agent_role`, `git_info`, `name`, но не note.
- `codex-rs/app-server-protocol/src/protocol/v2/thread.rs::ThreadMetadataUpdateParams` сейчас patch-ит только `git_info`; добавлять туда note в первом native port не нужно без отдельного stable-vs-experimental решения.
- `codex-rs/app-server-protocol/src/protocol/v2/item.rs::CollabAgentState` содержит только `status` и `message`; `event_mapping.rs` и `thread_history.rs` реконструируют `ThreadItem::CollabAgentToolCall` из collab events без note snapshots.
- TUI multi-agent navigation/rendering (`codex-rs/tui/src/multi_agents.rs`, `codex-rs/tui/src/app/thread_routing.rs`, `codex-rs/tui/src/app/session_lifecycle.rs`) кэширует `agent_nickname` и `agent_role`, но не note.

## Gap analysis

Относительно fork contract отсутствуют:

- Normalizer для plain/structured/clear semantics.
- `spawn_agent` v1/v2 input schema и runtime plumbing для initial child `thread_note`.
- Durable source of truth для note после `set_thread_note`. Одного `SessionMeta.thread_note` достаточно только для creation-time note; runtime update/clear должен писаться отдельным persisted metadata event или эквивалентным rollout item и append-only index entry.
- Runtime metadata field в `AgentMetadata`/agent navigation snapshots, чтобы live UI не ждал restart/re-read из rollout.
- Rollout extraction/list plus file-backed `codex_home` latest-value index для restart-safe app-server
  `thread/list` и `thread/read`.
- App-server `Thread.threadNote` и `CollabAgentState.threadNote` payload mapping, если note должен быть видим Codex app / external clients.
- TUI rendering и stale-cache clearing для explicit `null`.
- Tests, доказывающие отсутствие note в model-facing contexts.

Не рекомендуется использовать `TurnContextItem` как canonical storage для note. Он тесно связан с model-visible turn context (`user_instructions`, `developer_instructions`, environment/runtime fields) и уже используется context/replay кодом. Если note попадет туда, future code легко начнет воспринимать его как часть model context. Более безопасная форма: `SessionMeta.thread_note` для initial metadata плюс отдельное `EventMsg`/metadata rollout item для `set_thread_note` update/clear, затем `state::extract` применяет latest value к `ThreadMetadata`.

## Направление нативной реализации

Реализовать note через существующую metadata architecture, без side channel и без prompt injection.

- Добавить protocol-owned normalizer, который возвращает `Option<String>`, где `None` означает очищенный note. Он должен быть shared между `spawn_agent`, `set_thread_note`, app-server mapping и tests.
- Добавить `thread_note` в durable metadata:
  - `SessionMeta.thread_note` как creation-time snapshot.
  - Dedicated persisted update path для `set_thread_note`, например новый `EventMsg::ThreadNoteUpdated` или scoped rollout metadata item. Он должен поддерживать `Some(normalized)` и explicit clear.
  - Append-only latest-value index under `codex_home`, e.g. `thread_note_index.jsonl`, so restart-safe
    read/list does not require a state DB migration in v1.
- Для sub-agent spawn добавить optional `thread_note` и input-only `thread_note_competencies` в
  v1/v2 `spawn_agent`; v2 обязательно синхронизировать с `#[serde(deny_unknown_fields)]` args и
  schema.
- `SubAgentSource::ThreadSpawn.thread_note` допустим только как creation-time projection, если нужен symmetry с `agent_nickname`/`agent_role`. Он не должен быть единственным source of truth, потому что note принадлежит thread, а не только spawn source, и может меняться после создания.
- Добавить `set_thread_note` как core model tool для текущего thread и видимых sub-agent threads.
  Tool result: `{ "target": string, "thread_note": string | null }`. Его model-visible часть
  ограничена самим tool call/result; note не должен попадать в instructions/environment/history
  prompt material.
- Экспонировать note в orchestration inspection surfaces: `list_agents.thread_note` и legacy
  `wait_agent.agent_metadata[*].thread_note`, чтобы orchestrator мог видеть актуальную
  ответственность/компетенции агента без чтения UI-only данных.
- Передавать note snapshots через core collab events там, где history/UI нужен стабильный display: spawn end, send-input end, wait/resume/close end при наличии receiver state. Explicit `null` должен очищать stale cached values.
- Маппить note snapshots в app-server collab history как required-nullable `threadNote` в `CollabAgentState` для текущего fork server, если UI/history должен показывать per-agent note рядом со status.
- Расширить TUI metadata cache и rendering: non-empty notes показывать компактно, explicit `null` от текущего fork server очищает stale cache, absent field от older server не меняет старое cached значение.

Не добавлять note в model-visible contexts.

## Risky integration points / source-of-truth files

- Protocol source: `codex-rs/protocol/src/protocol.rs` (`SubAgentSource`, `SessionMeta`, collab events, possible note update event).
- Spawn tools: `codex-rs/core/src/tools/handlers/multi_agents/spawn.rs`, `codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs`, `codex-rs/core/src/tools/handlers/multi_agents_common.rs`, `codex-rs/core/src/tools/handlers/multi_agents_spec.rs`.
- Runtime agent metadata: `codex-rs/core/src/agent/registry.rs`, `codex-rs/core/src/agent/control.rs`.
- Rollout and metadata extraction: `codex-rs/rollout/src/recorder.rs`, `codex-rs/rollout/src/metadata.rs`, `codex-rs/rollout/src/list.rs`, `codex-rs/state/src/extract.rs`.
- State DB: not required for v1 `thread_note` persistence when app-server read/list hydrate from rollout metadata plus the file-backed index.
- App-server protocol: `codex-rs/app-server-protocol/src/protocol/v2/thread_data.rs`, `codex-rs/app-server-protocol/src/protocol/v2/item.rs`, `codex-rs/app-server-protocol/src/protocol/v2/thread.rs`, `codex-rs/app-server-protocol/src/protocol/event_mapping.rs`, `codex-rs/app-server-protocol/src/protocol/thread_history.rs`.
- App-server processors: `codex-rs/app-server/src/request_processors/thread_summary.rs`, `codex-rs/app-server/src/request_processors/thread_processor.rs`.
- Generated protocol artifacts, if app-server payloads change: `codex-rs/app-server-protocol/schema/json/**`, `codex-rs/app-server-protocol/schema/typescript/**`.
- TUI surfaces: `codex-rs/tui/src/multi_agents.rs`, `codex-rs/tui/src/app/thread_routing.rs`, `codex-rs/tui/src/app/session_lifecycle.rs`, `codex-rs/tui/src/app/loaded_threads.rs`, `codex-rs/tui/src/chatwidget/tests/app_server.rs`.

## Совместимость Codex App / App-server

Совместимость должна быть additive и non-breaking.

- Не добавлять новый stable app-server mutation method в первом native port. Если client-side edit понадобится позже, рассмотреть extension существующего `thread/metadata/update` только с отдельным compatibility decision; safer default - experimental gate.
- `Thread.threadNote: string | null` и `CollabAgentState.threadNote: string | null` добавляются как
  additive v2 response fields в текущем fork server. Это соответствует текущему app-server правилу
  required-nullable response fields. Older servers may omit these fields, so cross-version clients
  should tolerate absence.
- `threadNote: null` означает no note / cleared note; string означает current note.
- Старые Codex app clients обычно игнорируют неизвестные fields, но strict JSON validators или
  generated clients могут считать stable schema contract жестким. Поэтому изменение
  `Thread`/`CollabAgentState` требует schema regeneration и compatibility note.
- Новые app-server mutation methods, если когда-либо понадобятся, должны сначала быть experimental и иметь matrix: old app + new server, new app + old server, mixed schema versions.

Рекомендуемая классификация:

- Core model tool/schema: fork-native stable внутри этого fork.
- App-server payload metadata: stable additive required-nullable fields in the current fork server,
  with older-server omission handled as compatibility behavior.
- New app-server methods: только experimental, вне scope первого native port.
- TUI display: fork-native stable.

## Release-specific verification notes

Минимальный `fork/130` implementation gate:

- Normalizer unit tests: plain text, structured input, whitespace clear, repeated separators, leading/trailing spaces.
- v1/v2 `spawn_agent` schema tests и handler tests для `thread_note` и
  `thread_note_competencies`; v2 должен reject unknown fields кроме добавленных note fields.
- `set_thread_note` update/clear tests: persisted event/item, live metadata update, tool result `{ "thread_note": ... }`.
- Resume/rollout replay tests: initial spawn note и later update/clear переживают reload.
- Negative tests: note отсутствует в developer instructions, user instructions, `EnvironmentContext`, `TurnContextItem`-derived context update, `InterAgentCommunication` prompt и tool hints.
- App-server mapping tests: `Thread.threadNote` read/list/resume/fork при добавлении поля;
  `CollabAgentState.threadNote` в `event_mapping.rs` и `thread_history.rs`; live
  `ThreadNoteUpdatedNotification` propagation.
- File-backed index tests: append, latest-wins lookup, clear via `None`, missing/corrupt-line
  tolerance, and restart-safe app-server hydration from rollout/index.
- TUI snapshots: note rendering, older-server absent note не меняет output, current-server explicit
  `null` очищает stale note.
- Generated schema/TypeScript regeneration, если меняются app-server protocol payloads.

Research verification performed for this document:

- `rg -n "thread_note|threadNote|ThreadNote|set_thread_note" codex-rs`
- `rg -n "SubAgentSource|CollabAgentState" codex-rs`
- `rg --files codex-rs/app-server-protocol/src/protocol`
- `sed` inspections for `protocol.rs`, `multi_agents_common.rs`, v1/v2 `spawn.rs`, `multi_agents_spec.rs`, `thread_data.rs`, `thread.rs`, `item.rs`, `event_mapping.rs`, `thread_history.rs`, state metadata, rollout metadata, and TUI multi-agent cache/rendering.

## Открытые риски и assumptions

- Historical fork semantics for exact normalization strings (`Назначение`, `Компетенции`) should be cross-checked against the previous fork implementation before coding. This document treats the current contract text as intended behavior.
- `hide_spawn_agent_metadata`: safer default - hide note from model-facing spawn result when metadata is hidden, but still persist it internally if supplied. Это требует явного test.
- Historical fork branches show `Thread.threadNote` can be restart-safe without SQLite migration by
  using rollout metadata plus `~/.codex/thread_note_index.jsonl`. The `fork/130` implementation
  should preserve that approach unless current app-server read/list code proves a DB-only path is
  unavoidable.
- Stable app-server payload additions are additive JSON-wise, but can still affect strict clients and generated schemas. Document schema version compatibility in the implementation PR.
- Главный regression risk - случайное model exposure. Tests должны assert absence from every model-facing input path, not just absence from developer instructions.
