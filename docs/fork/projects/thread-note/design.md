# thread-note Design

## Canonical State

The source of truth is rollout-backed thread metadata:

- `SessionMeta.thread_note: Option<String>` stores only the creation-time value for a thread.
- A persisted note update event/item stores every later update and clear operation. The default
  implementation shape is `EventMsg::ThreadNoteUpdated { thread_id, thread_note, updated_at_ms }`.
- A file-backed latest-value index under `codex_home` stores the most recent value for each thread
  without requiring a state DB migration. The historical fork used an append-only
  `thread_note_index.jsonl` where the newest matching entry wins and `None` clears stale values.
  Reference:
  [`35187c0529`](https://github.com/NeoGrAIph/codex/commit/35187c0529b8f0797cd9460c714167c06d60f24b)
  in `fork/118`.
- Runtime agent/TUI caches mirror the latest value for live display.

`SubAgentSource::ThreadSpawn` must not expose `thread_note` as a public/source projection. A note
belongs to the thread and can change after spawn, so current clients must read it from
`Thread.threadNote`, `CollabAgentState.threadNote`, `list_agents`, legacy `wait_agent`, or
`thread/note/updated`.

`TurnContextItem` must not store note. It carries model/runtime context such as instructions,
environment parameters, model, personality, sandbox, approval policy, and collaboration mode.
Adding note there would create an unnecessary model-exposure risk.

## Normalization

- Absent, `null`, empty, or whitespace-only input clears the note.
- Plain text becomes `Назначение: <text> | Компетенции:`.
- `thread_note` plus `thread_note_competencies` becomes
  `Назначение: <thread_note> | Компетенции: <thread_note_competencies>`.
- Canonical `Назначение: ... | Компетенции: ...` input preserves both parts with normalized spaces.
- Both structured parts empty means clear.
- Providing non-empty competencies both in canonical `thread_note` and in
  `thread_note_competencies` is rejected with a controlled tool error.
- Internal whitespace runs are collapsed to single spaces after trimming.
- Non-canonical labels, Markdown, JSON, and alternate language labels are treated as plain text in
  v1.
- Maximum normalized length is 500 characters; over-limit input returns a controlled tool error.

Recommended examples:

| Input | Normalized value |
| --- | --- |
| absent spawn arg | `None` |
| `null` | `None` |
| `   ` | `None` |
| `проверить миграции` | `Назначение: проверить миграции | Компетенции:` |
| `thread_note=проверить`, `thread_note_competencies=tests` | `Назначение: проверить | Компетенции: tests` |
| `Назначение: аудит   схем | Компетенции: app-server, TUI` | `Назначение: аудит схем | Компетенции: app-server, TUI` |
| `Назначение: | Компетенции:` | `None` |

The normalizer should be protocol-owned or otherwise shared so `spawn_agent`, `set_thread_note`,
app-server mapping, and tests cannot drift.

## Public Tool Surface

`spawn_agent.thread_note`:

- Add optional nullable `thread_note` to legacy v1 `spawn_agent`.
- Add optional nullable `thread_note_competencies` to legacy v1 `spawn_agent`.
- Add optional nullable `thread_note` to MultiAgentV2 `spawn_agent`; update both
  `#[serde(deny_unknown_fields)]` args and `create_spawn_agent_tool_v2` schema.
- Add optional nullable `thread_note_competencies` to MultiAgentV2 `spawn_agent`; it is folded into
  the canonical stored `thread_note`.
- Omitted field means no note for the child.
- `null`, empty, or whitespace-only field means clear/no note for the child.
- `spawn_agent` model-facing output never returns note. This remains true when
  `hide_spawn_agent_metadata` is false.
- `hide_spawn_agent_metadata = true` keeps note out of the spawn result, but it does not remove the
  input field and does not block server-side persistence.

`set_thread_note`:

- Add a core model tool that mutates the current thread's note or a visible sub-agent note.
- Args are `{ "target"?: string, "thread_note": string | null }`.
- Optional `thread_note_competencies: string | null` can be supplied with plain `thread_note` and is
  folded into the canonical stored note.
- Omitted `target` updates the current thread.
- Legacy multi-agent mode resolves `target` as a visible agent thread id. MultiAgentV2 resolves
  `target` as a visible agent id or canonical task name.
- Tool output is exactly `{ "target": string, "thread_note": string | null }` after normalization.
- The tool output is the only model-facing place where the note value is intentionally returned.

Inspection tools:

- `list_agents` is available in legacy and MultiAgentV2 modes and returns
  `thread_note: string | null` for each visible agent.
- Legacy `wait_agent` returns additive `agent_metadata` keyed like `status`; each entry currently
  contains `thread_note: string | null`.
- `spawn_agent` output still does not return note.

## Data Flow

Spawn:

1. Normalize `spawn_agent.thread_note`.
2. Pass the normalized value through spawn runtime metadata without adding it to prompts,
   instructions, `message`, or `SubAgentSource`.
3. Store normalized value in child `SessionMeta.thread_note`.
4. Register the value in runtime `AgentMetadata`/thread snapshots for live parent/TUI display.
5. Persist rollout metadata and append the latest value to the `codex_home` note index.
6. Surface it through `Thread.threadNote` and collab snapshots when app-server visibility is needed.

Update/clear:

1. `set_thread_note` normalizes input.
2. Resolve the optional target to the current thread or a visible sub-agent.
3. Runtime thread metadata updates immediately.
4. Persist update event/item through the normal session event pipeline.
5. Append the latest value or explicit clear to the file-backed note index.
6. App-server/TUI receive string or explicit `null` through `thread/note/updated`.

Resume/replay:

1. Load rollout.
2. Apply `SessionMeta.thread_note`.
3. Apply later note update events/items in rollout order.
4. If rollout metadata is unavailable or legacy data needs recovery, read the latest value from the
   file-backed note index.
5. Project the final value into runtime metadata, app-server `Thread`, and collab history.

## Invariants

- Note never enters developer/user instructions, environment context, tool hints, IAC prompts, or
  settings updates.
- App-server current-server `null`/string semantics remain distinct; older-server absence is
  tolerated only as cross-version compatibility.
- Note is secondary display metadata and never identity.
- Note is not persona, policy, role/template selection, cwd, model, reasoning effort, permission
  profile, or environment selection.
- `SessionMeta.thread_note` alone is not sufficient for update/clear; implementation must include a
  persisted update path.

## App-server Contract

V1 includes app-server visibility because the feature is useful only if humans and orchestration
surfaces can inspect the note.

Add these protocol v2 fields:

- `Thread.threadNote: string | null` in `protocol/v2/thread_data.rs`.
- `CollabAgentState.threadNote: string | null` in `protocol/v2/item.rs`.
- `ThreadNoteUpdatedNotification` exposed as `thread/note/updated`.

App-server read/list/resume/fork paths currently build `Thread` from `StoredThread` metadata. Since
the v1 plan does not add `thread_note` to `StoredThread`/state DB, request processors must hydrate
`Thread.threadNote` from rollout metadata plus the `codex_home` note index after loading stored
threads and before serializing protocol responses.

Serialization semantics:

- For current fork server responses, use required-nullable fields to match the app-server v2
  payload rules. Clients that need cross-version compatibility should tolerate older servers
  omitting the fields.
- For `CollabAgentState.threadNote`, `null` means no note or cleared note; string means replace
  cached value. Missing field means older server/unknown.
- Do not add a stable app-server mutation method in v1; mutation remains model-tool owned.
- Do not extend persona/policy/cwd app-server schema as part of this feature.

## Tradeoffs

- v1 adds app-server fields because visibility is part of the feature.
- v1 does not add a stable app-server mutation RPC; mutation is model-tool owned.
- A state DB migration is intentionally not required for v1. `Thread.threadNote` remains
  restart-safe when read/list/resume hydrate from rollout metadata plus the `codex_home` index,
  matching the local historical fork implementation. If a future upstream app-server path becomes
  DB-only, that can be evaluated as a separate compatibility decision.
- The TUI treats note as secondary metadata. Agent label and identity continue to come from
  nickname, role, task path, and thread id.

## TUI Handoff

- Extend `AgentPickerThreadEntry` and TUI `AgentMetadata` with note state separate from nickname and
  role.
- Consume `Thread.threadNote` from loaded/read/list thread data and `CollabAgentState.threadNote`
  from live/history collab items.
- Consume `ThreadNoteUpdatedNotification` for active and inactive thread caches.
- Missing field from an older server leaves cached note unchanged.
- `null` from the current fork server clears cached note immediately, including for inactive agents.
- String replaces cached note.
- Render note as a dim secondary line or compact detail line in picker/history surfaces. It should
  wrap or truncate predictably and must not resize fixed-format controls on focus/hover.
- Do not derive note from task name, persona, policy, cwd, or role template data.
