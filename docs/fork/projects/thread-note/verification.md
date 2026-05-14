# thread-note Verification

## Scenarios

| Scenario | Expected result |
| --- | --- |
| Absent spawn note | Child thread has no note and existing spawn behavior is unchanged |
| Plain note | Normalized to `Назначение: ... | Компетенции:` |
| Split purpose/competencies | `thread_note` + `thread_note_competencies` normalize into one stored note |
| Duplicate competencies | Canonical note with non-empty competencies plus `thread_note_competencies` is rejected |
| Structured note | Parts preserved with normalized spacing |
| Structured empty parts | Note cleared |
| Null/whitespace | Note cleared and persisted as explicit clear when this is an update |
| Over-length input | Controlled tool error; no silent truncation |
| Spawn with note | Child thread stores initial note |
| Legacy v1 spawn output | Output does not include note |
| MultiAgentV2 spawn output | Output does not include note and `deny_unknown_fields` remains enforced |
| Hidden metadata spawn | `thread_note` input persists internally; note is not returned in spawn output |
| Set note | Current thread runtime metadata updates and update event/item persists |
| Set visible target note | A visible sub-agent note updates by id, and by canonical task name in MultiAgentV2 |
| Unknown target note | Unknown or non-visible target returns a controlled model error |
| Clear note | Runtime metadata and file-backed index clear; app-server/TUI receive explicit `null` |
| List agents | Each visible agent includes `thread_note: string | null` |
| Legacy wait agent | `agent_metadata[target].thread_note` is returned without changing `status` semantics |
| Resume/replay | `SessionMeta.thread_note` plus later update/clear events restore latest note |
| Restart-safe read/list | `Thread.threadNote` comes from rollout metadata plus latest file-backed index value |
| App-server read/resume/fork | `Thread.threadNote` reflects latest note |
| Collab history | `CollabAgentState.threadNote` maps spawn/send/wait/resume/close snapshots |
| Older app-server absent field | Cached note remains unchanged for cross-version compatibility |
| Current app-server explicit null | Cached note is cleared |
| TUI string value | Cached note is replaced and rendered as secondary metadata |
| Model exposure | Note absent from all prompt/context paths |
| Feature isolation | Note is not mixed with persona, policy, role template, cwd, model, or permission state |

## Focused Test Surfaces

- Normalizer unit tests in the crate that owns the shared normalization function.
- `codex-core` handler tests for v1/v2 `spawn_agent.thread_note`,
  `spawn_agent.thread_note_competencies`, `set_thread_note` current/target
  update, target rejection, legacy/v2 `list_agents.thread_note`, legacy
  `wait_agent.agent_metadata`, and hidden metadata behavior.
- `codex-protocol` serialization tests for `SessionMeta.thread_note`, note update event/item, and
  app-server-facing payload conversions where applicable.
- `codex-rollout` recorder/list/metadata and session-index tests for creation snapshot, later
  updates, clear, latest-wins behavior, and restart-safe list/read hydration.
- `codex-app-server-protocol` tests for required-nullable `Thread.threadNote`,
  `CollabAgentState.threadNote`, `ThreadNoteUpdatedNotification`, event mapping, and
  thread-history reconstruction.
- `codex-app-server` request processor tests for read/list/resume/fork summaries, including
  hydration from rollout/index metadata after loading `StoredThread`.
- `codex-tui` cache and snapshot tests for picker/history rows, older-server absent unchanged, and
  current-server null clearing.
- Negative tests using structured request assertions that note is absent from developer
  instructions, user instructions, `EnvironmentContext`, `TurnContextItem`-derived context updates,
  `InterAgentCommunication` prompts, settings updates, and tool hints.

## Commands

```bash
cd ~/repo/AGENTS/codex-fork
cargo test --manifest-path codex-rs/Cargo.toml -p codex-protocol thread_note
cargo test --manifest-path codex-rs/Cargo.toml -p codex-rollout thread_note
cargo test --manifest-path codex-rs/Cargo.toml -p codex-core thread_note
cargo test --manifest-path codex-rs/Cargo.toml -p codex-app-server-protocol thread_note
cargo test --manifest-path codex-rs/Cargo.toml -p codex-app-server thread_note
cargo test --manifest-path codex-rs/Cargo.toml -p codex-tui multi_agents
just write-app-server-schema
git diff -- codex-rs/app-server-protocol/schema
```

If working from `codex-rs/`, equivalent scoped commands are:

```bash
cargo test -p codex-core thread_note
cargo test -p codex-app-server-protocol thread_note
cargo test -p codex-app-server thread_note
cargo test -p codex-tui multi_agents
```

Run `cargo test -p codex-tui` and accept required `insta` snapshots if user-visible TUI rendering
changes. Run `just fmt` after Rust edits and `just fix -p <crate>` for touched Rust crates per
`AGENTS.md`.

## Coverage Gaps

- Client-side note editing through app-server is out of scope.
- Codex app display work is limited to consuming optional fields; v1 does not add a stable app-server
  mutation RPC.
- Persona/policy/cwd projection remains owned by their separate features and must not be implemented
  as part of thread-note.
