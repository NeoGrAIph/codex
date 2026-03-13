# Thread Note

## Goal

`thread_note` is an optional metadata field for a thread. It is distinct from the thread name and
is intended to preserve the stable narrow specialization, intended role, or relevant competencies
of a specific agent thread.

Canonical format:

`Назначение: <seed> | Компетенции: <list>`

Implementation dossier:

- [docs/fork/projects/thread-note/README.md](../projects/thread-note/README.md)

## Transcript Behavior

When a collaboration transcript entry references a thread with a non-empty note, TUI renders a
`Note:` line before the prompt preview.

### Spawned agent

For:

```text
• Spawned <agent> [role]
```

TUI renders:

```text
  └ Note: <thread note>
    <prompt preview>
```

The `Note:` line is omitted when `new_thread_note` is absent or whitespace-only.

### Sent input to agent

For:

```text
• Sent input to <agent> [role]
```

TUI renders:

```text
  └ Note: <thread note snapshot>
    <prompt preview>
```

The `Note:` line is omitted when `receiver_thread_note` is absent or whitespace-only.

## Snapshot Semantics

Transcript rendering uses the note carried by the event payload:

- `CollabAgentSpawnEndEvent.new_thread_note`
- `CollabAgentInteractionEndEvent.receiver_thread_note`

This is a snapshot of thread metadata at the time the event was emitted. TUI does not perform a
live lookup during replay.

## Metadata-only Contract

`thread_note` is runtime metadata, not a model instruction.

- Codex does not inject `thread_note` into developer context.
- Codex does not inject `thread_note` into `environment_context` XML.
- Changing or clearing the note does not emit a prompt-level update for the model.
- The note remains available to orchestration, transcript rendering, restart/resume recovery, and
  other metadata surfaces that explicitly carry it.

This is intentional: note content may describe specialization or competencies, but it must not
become an implicit task instruction or persona override for the current model turn.

## Adjacent Metadata

- `agent_role` defines a broad reusable role class.
- `agent_persona` and role templates shape prompt/policy/style.
- `agent_nickname` gives the thread a human-friendly display identity.
- `thread_note` captures the narrower specialization and competencies of this specific thread.

## Recommended Usage

Recommended `thread_note` values:

- `Назначение: Repository researcher | Компетенции: структура репозитория; AGENTS.md`
- `Назначение: Feature documentation specialist | Компетенции: docs/fork/features; feature contracts`
- `Назначение: Test runner for this task | Компетенции: cargo test; targeted crate checks`
- `Назначение: Rust TUI snapshot reviewer | Компетенции: insta snapshots; ratatui transcript output`

When the thread learns more, update only `Компетенции:` and preserve `Назначение:`.

For persisted threads, `set_thread_note` writes the normalized note into an append-only
`thread_note_index.jsonl` under `CODEX_HOME` before updating live session state. On restart/resume,
Codex restores the note from that index first, then falls back to the existing session snapshot,
and finally to the note embedded in `ThreadSpawn` source metadata. This contract intentionally does
not expose `thread_note` through app-server thread read/list APIs in the current fork state.
