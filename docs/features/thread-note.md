# Thread Note

## Feature passport

- Code name: `Thread Note`.
- Status: `implemented`.
- Goal: добавить устойчивую user-facing заметку треда (`thread_note`) как часть metadata-контракта для обычных и spawned threads.
- Scope in:
- `codex-rs/protocol/src/protocol.rs`
- `codex-rs/core/src/{codex.rs,codex_thread.rs,agent/control.rs,tools/handlers/multi_agents.rs,tools/spec.rs,util.rs}`
- `codex-rs/core/src/rollout/{metadata.rs,policy.rs,recorder.rs}`
- `codex-rs/state/src/{extract.rs,model/thread_metadata.rs,runtime/threads.rs,runtime/memories.rs}`
- `codex-rs/app-server-protocol/src/protocol/{common,v2}.rs`
- `codex-rs/app-server/src/{codex_message_processor.rs,bespoke_event_handling.rs}`
- `codex-rs/tui/src/chatwidget.rs`
- Scope out:
- `thread_note_index.jsonl` / `session_index` как source of truth;
- `CODEX_THREAD_NOTE`;
- developer-instructions injection `Thread note: ...`;
- shell/js_repl/unified-exec/env plumbing;
- SAW, overlays и subtree management.

## User contract

- `spawn_agent` принимает опциональный `thread_note`.
- `thread_note` нормализуется (`trim`, пустое или пробельное значение -> `None`).
- `spawn_agent` возвращает `thread_note` в JSON-результате вместе с `agent_id` и `nickname`.
- Добавлен новый collab tool `set_thread_note`:
- вход: `id`, `note?`;
- пустое/пробельное значение очищает заметку;
- выход: `{ "thread_note": <normalized|null> }`.
- `wait`, `CollabAgentStatusEntry`, `CollabAgentRef`, `CollabAgentSpawnEndEvent` возвращают `thread_note`.

## Protocol contract

- Добавлен `Op::SetThreadNote { note: Option<String> }`.
- Добавлен `EventMsg::ThreadNoteUpdated(ThreadNoteUpdatedEvent)`.
- `ThreadNoteUpdatedEvent` содержит `{ thread_id, thread_note }`.
- `SessionConfiguredEvent` расширен полем `thread_note: Option<String>`.
- `SessionMeta` расширен полем `thread_note: Option<String>`.
- `SubAgentSource::ThreadSpawn` содержит `thread_note: Option<String>`.
- `CollabAgentRef`, `CollabAgentStatusEntry`, `CollabAgentSpawnEndEvent` содержат `thread_note`.

Совместимость:

- все новые поля опциональные;
- старые payload без `thread_note` продолжают читаться как `None`.

## Persistence model

- Каноничный source of truth: `SessionMeta.thread_note` + persisted `ThreadMetadata.thread_note`.
- Для spawned threads `source.thread_spawn.thread_note` хранит initial/source metadata и поддерживается согласованным с top-level note.
- `ThreadMetadata` расширен полем `thread_note`.
- Добавлена SQLite migration `0020_threads_thread_note.sql`.
- Состояние note сохраняется двумя путями:
- initial value через `SessionMeta.thread_note`;
- последующие изменения через `ThreadNoteUpdatedEvent`.
- Отдельный индекс `thread_note_index.jsonl` не используется.

## Core behavior

- `normalize_thread_note(note: Option<&str>) -> Option<String>`:
- делает `trim`;
- пустое/пробельное значение очищает note.
- `spawn_agent`:
- нормализует `thread_note`;
- записывает его в `SubAgentSource::ThreadSpawn`;
- копирует note в child `SessionConfiguration.thread_note`.
- `set_thread_note`:
- требует persistence-enabled session;
- обновляет `SessionConfiguration.thread_note`;
- для spawned threads синхронизирует note и в `SessionSource::SubAgent(SubAgentSource::ThreadSpawn { .. })`;
- эмитит `ThreadNoteUpdated`.
- При resume/fork note восстанавливается сначала из persisted `ThreadMetadata.thread_note`, затем из rollout/session source fallback.

## App-server contract

- Добавлен RPC `thread/note/set`:
- request: `ThreadSetNoteParams { threadId, note? }`;
- response: `ThreadSetNoteResponse {}`.
- Добавлена notification `thread/note/updated` (`ThreadNoteUpdatedNotification`).
- `Thread` (v2) расширен полем `thread_note`.
- `thread/list`, `thread/read`, `thread/resume`, `thread/fork`, `thread/unarchive` возвращают `thread_note`.
- Для spawned threads top-level `thread_note` согласуется с nested `source.thread_spawn.thread_note`.
- Для unloaded persisted thread note обновляется через state DB, без reread отдельного индекса.

## TUI minimum

- `SessionConfigured` seed'ит `thread_note` в локальное состояние `ChatWidget`.
- `ThreadNoteUpdated` обновляет локальный note-state.
- Новая metadata не меняет существующий overlay/UI flow и не добавляет отдельный note UI в этом коммите.

## Verification matrix

- `protocol`:
- roundtrip optional `thread_note` в `ThreadSpawn`, `SessionMeta`, `SessionConfiguredEvent`;
- serialization/deserialization `ThreadNoteUpdatedEvent`.
- `core`:
- тест `normalize_thread_note`;
- `spawn_agent` propagates `thread_note`;
- `set_thread_note`: invalid id, missing agent, update, clear;
- `wait`/collab status payloads preserve note.
- `state`:
- extraction from `SessionMeta.thread_note`;
- extraction from `ThreadNoteUpdatedEvent`;
- sqlite roundtrip for `thread_note`;
- legacy rows without column return `None`.
- `app-server`:
- loaded and unloaded `thread/note/set` paths;
- `thread/list/read/resume/fork/unarchive` preserve note.
- `tui`:
- `SessionConfigured` seeds note;
- `ThreadNoteUpdated` updates note state;
- helper/session snapshot paths do not drop note.

## Doc changelog

- 2026-03-07: Documented contract-first implementation of `thread_note` on top of the existing metadata/state pipeline, without `session_index` and without runtime side effects.
