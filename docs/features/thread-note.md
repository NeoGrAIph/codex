# Thread Note

## Feature passport

- Code name: `Thread Note`.
- Status: `implemented` (рабочий набор правок сейчас поднят из `stash`).
- Goal: добавить устойчивую user-facing заметку треда (`thread_note`) как часть контекста сабагентов и runtime metadata.
- Scope in:
- `codex-rs/protocol/src/protocol.rs`
- `codex-rs/core/src/codex.rs`
- `codex-rs/core/src/agent/control.rs`
- `codex-rs/core/src/tools/handlers/multi_agents.rs`
- `codex-rs/core/src/rollout/session_index.rs`
- `codex-rs/core/src/exec_env.rs`
- `codex-rs/app-server-protocol/src/protocol/{common,v2}.rs`
- `codex-rs/app-server/src/{codex_message_processor,bespoke_event_handling}.rs`
- `codex-rs/tui/src/{app,chatwidget,multi_agents}.rs`
- Scope out:
- изменение модели ownership/subtree;
- изменение контракта `allow_list/deny_list` (документировано отдельно в `threadspawn-contract.md`).

## User contract

- `spawn_agent` принимает опциональный `thread_note`.
- `thread_note` нормализуется (trim, пустое/пробелы -> `None`).
- `spawn_agent` возвращает `thread_note` в JSON-результате вместе с `agent_id` и `nickname`.
- Добавлен новый collab tool `set_thread_note`:
- вход: `id`, `note?`;
- пустое/пробельное значение очищает заметку;
- выход: `{ "thread_note": <normalized|null> }`.
- `wait` и `CollabAgentStatusEntry` сохраняют и возвращают `thread_note` для каждого агента.

## Protocol contract

- Добавлен `Op::SetThreadNote { note: Option<String> }`.
- Добавлен `EventMsg::ThreadNoteUpdated(ThreadNoteUpdatedEvent)`.
- `SessionConfiguredEvent` расширен полем `thread_note: Option<String>`.
- `SubAgentSource::ThreadSpawn` содержит `thread_note: Option<String>`.
- `CollabAgentRef`, `CollabAgentStatusEntry`, `CollabAgentSpawnEndEvent` содержат `thread_note`-поля.

Совместимость:

- поля опциональные, с безопасным default при десериализации;
- старые payload без `thread_note` остаются валидными.

## Persistence model

- Добавлен append-only индекс `thread_note_index.jsonl`.
- Введены API в `session_index`:
- `append_thread_note`
- `find_thread_note_by_id`
- `find_thread_notes_by_ids`
- Семантика разрешения состояния: последняя запись побеждает; `None` (или пустая строка после trim) очищает заметку.

## Core behavior

- `handlers::set_thread_note`:
- нормализует вход;
- записывает в `thread_note_index.jsonl`;
- обновляет `session_configuration.thread_note`;
- эмитит `ThreadNoteUpdated`.
- Если persistence недоступен, операция возвращает `Error` event и не делает частичных обновлений.
- При инициализации/возобновлении сессии `thread_note` восстанавливается по приоритету:
- note из индекса;
- note из текущей session config;
- note из `session_source` (`ThreadSpawn.thread_note`).
- `thread_note` добавляется в developer instructions как строка `Thread note: ...`.

## Environment propagation

- `create_env(...)` расширен параметром `thread_note`.
- Добавлен env var `CODEX_THREAD_NOTE`.
- `thread_note` прокинут в shell/js-repl/user-shell/unified-exec пути.

## App-server contract

- Добавлен RPC `thread/note/set`:
- request: `ThreadSetNoteParams { threadId, note? }`;
- response: `ThreadSetNoteResponse {}`.
- Добавлена notification `thread/note/updated` (`ThreadNoteUpdatedNotification`).
- `Thread` (v2) расширен полем `thread_note`.
- `thread/list`, `thread/read`, `thread/resume`, `thread/fork`, `thread/unarchive` подхватывают `thread_note` из индекса.
- При merge метаданных spawned-треда сохраняется `thread_note` вместе с nickname/role.

## TUI and processors

- TUI bootstrap (`SessionConfigured`) теперь инициализирует `thread_note`.
- `ChatWidget` обрабатывает `ThreadNoteUpdated` и обновляет локальное состояние note.
- `mcp-server` и `exec` процессоры корректно игнорируют/пропускают новое metadata-событие без регрессий.

## Verification matrix in diff

- `core/session_index`:
- тесты на latest-wins и clear-семантику для note.
- `core/util`:
- тест нормализации `normalize_thread_note`.
- `core/multi_agents`:
- тесты `spawn_agent` с note;
- тесты `set_thread_note` (invalid id, missing agent, update+clear).
- `protocol`:
- roundtrip/fixture проверки `thread_note` в `ThreadSpawn`.
- `app-server`/`tui`:
- фикстуры и компиляционные обновления под новые поля/события.

## Operational notes

- После восстановления этого stash в дереве есть не только feature-правки, но и auto-generated schema/type файлы, а также `*.snap.new` snapshot-артефакты TUI; перед коммитом их нужно разобрать отдельным шагом.

## Doc changelog

- 2026-02-27: Initial feature document based on recovered stash diff (`thread_note` contract, persistence, API, and UI propagation).
