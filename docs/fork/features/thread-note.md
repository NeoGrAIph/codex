# Feature: thread-note

## Feature Passport

- Code name: `thread-note`
- Status: implemented for `fork/130`
- Goal: attach human-readable purpose/competency metadata to agent threads.
- Scope in: `spawn_agent.thread_note`, `set_thread_note` for current thread or visible sub-agent,
  persistence, app-server read/list/history/live notifications, TUI display, and model-tool
  inspection through `list_agents`/legacy `wait_agent`.
- Scope out: prompt injection, persona/policy storage, cwd metadata, role-template state, new stable app-server mutation RPC.

## User Contract

- `thread_note` is thread-owned metadata.
- It can be set at spawn time or updated with `set_thread_note`.
- `thread_note_competencies` is a model-tool input convenience for competencies. It is folded into
  the canonical stored `thread_note` string and is not exposed as a separate persisted/app-server
  field.
- `set_thread_note.target` is optional. Omitted target updates the current thread. In legacy
  multi-agent mode the target is a visible agent thread id. In MultiAgentV2 the target may be a
  visible agent id or canonical task name.
- Absent spawn input means no note. `null`, empty, or whitespace-only input clears the note.
- The normalized note length limit is 500 characters. Over-limit input fails with a controlled tool error; it is not silently truncated.
- Plain text normalizes to `Назначение: <text> | Компетенции:`.
- Plain `thread_note` plus `thread_note_competencies` normalizes to
  `Назначение: <thread_note> | Компетенции: <thread_note_competencies>`.
- Canonical structured input `Назначение: ... | Компетенции: ...` preserves both parts with normalized spacing.
- Providing non-empty competencies both in canonical `thread_note` and in
  `thread_note_competencies` fails with a controlled tool error.
- Structured input clears the note when both parts are empty after trimming.
- Notes never become developer instructions, user instructions, environment context, tool hints, or IAC prompts.
- Competencies must be supplied through note metadata fields, not through `message`; `message` is
  the spawned agent's task prompt.
- Notes survive resume, rollout replay, and restart-safe app-server read/list.
- `spawn_agent` model-facing output never returns note, including when `hide_spawn_agent_metadata` is false.
- `hide_spawn_agent_metadata = true` does not remove the `thread_note` input field and does not block persistence; it only keeps spawn output metadata hidden.
- `set_thread_note` model-facing output is exactly
  `{ "target": string, "thread_note": string | null }` after normalization.
- `list_agents` is available in legacy and MultiAgentV2 modes and returns each visible agent's
  `thread_note: string | null`.
- Legacy `wait_agent` returns additive `agent_metadata` keyed like `status`; each value includes
  `thread_note: string | null`.

## Integration And Compatibility

- `thread_note` remains separate from persona, policy, role/template selection, cwd, model, reasoning effort, and runtime permissions.
- `SubAgentSource::ThreadSpawn` is not a current-note surface. It intentionally does not expose
  `thread_note`; clients must use `Thread.threadNote`, `CollabAgentState.threadNote`,
  `list_agents`, legacy `wait_agent`, or `thread/note/updated`.
- The source of truth is rollout-backed thread metadata plus a file-backed latest-value index under
  `codex_home` (for example `~/.codex/thread_note_index.jsonl`), matching the historical fork
  approach from
  [`35187c0529`](https://github.com/NeoGrAIph/codex/commit/35187c0529b8f0797cd9460c714167c06d60f24b).
  Creation-time `SessionMeta.thread_note` is a snapshot; later updates/clears are persisted as
  rollout metadata and appended to the index.
- `TurnContextItem` is not canonical storage and must not gain a note field.
- Add `Thread.threadNote: string | null` for app-server read/list/resume/fork visibility in the
  current fork server. Older servers may omit the field.
- Add `CollabAgentState.threadNote: string | null` for live/history collab state in the current
  fork server. Older servers may omit the field.
- Add `thread/note/updated` app-server notification so live note updates can reach inactive and
  active TUI/app-server clients in event order.
- Missing `threadNote` means older server/unknown.
- `threadNote: null` means no note or cleared note.
- `threadNote: "..."` means replace cached value.
- No new stable app-server mutation method in v1.
- Regenerate app-server schemas when protocol payloads change.
- App-server payload additions use the current v2 required-nullable response shape; clients that
  need cross-version compatibility should tolerate older servers omitting the field.

## Verification Matrix

| Surface | Required coverage |
| --- | --- |
| Normalizer | absent, null, plain, structured, empty, whitespace, both-parts-empty, length limit |
| Tools | v1/v2 spawn initial note, v2 unknown-field rejection, set/update/clear current and visible target note, hidden metadata output, legacy/v2 `list_agents` and legacy `wait_agent` note visibility |
| Persistence | `SessionMeta` initial note, update event/item, file-backed latest-value index, restart replay |
| App-server | `Thread.threadNote` read/list/resume/fork, `CollabAgentState.threadNote`, `ThreadNoteUpdatedNotification`, schema artifacts |
| TUI | picker/history rendering, null clears stale note |
| Safety | note absent from all model-facing context paths and not mixed with persona/policy/cwd |

## Doc Changelog

- 2026-05-12: Initial `fork/130` contract.
- 2026-05-12: Expanded v1 contract for normalization, storage, app-server compatibility, hidden metadata, and negative exposure gates.
- 2026-05-12: Replaced mandatory state DB projection with historical fork-compatible
  file-backed `codex_home` index persistence.
- 2026-05-14: Implemented current-release adaptation with required-nullable app-server fields,
  canonical `thread_note` tool input, and legacy `note` alias for `set_thread_note`.
- 2026-05-14: Extended `set_thread_note` to update visible targets, exposed notes to
  `list_agents` and legacy `wait_agent`, and added live app-server/TUI note update propagation.
- 2026-05-14: Made `list_agents` available in legacy collab mode with the same output contract as
  MultiAgentV2.
- 2026-05-14: Removed public `SubAgentSource::ThreadSpawn.thread_note` projection so current note
  state has one client-facing source of truth.
- 2026-05-15: Added `thread_note_competencies` tool input to avoid putting competencies in spawned
  agent prompts while preserving the single persisted `thread_note` contract.
