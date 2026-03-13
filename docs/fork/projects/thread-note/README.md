# Thread Note Project

This dossier describes how `thread_note` is implemented in `fork/114` and where to look when the
feature evolves.

Contract and release context:

- Feature contract: [docs/fork/features/thread-note.md](../../features/thread-note.md)
- Release research: [docs/fork/research/0.114.0/thread-note.md](../../research/0.114.0/thread-note.md)

Current status:

- transcript rows render `Note:` when the event payload carries a non-empty note snapshot;
- `thread_note` is metadata-only and is not exposed to the model through developer context or
  `environment_context` XML;
- note survives restart/resume via an append-only `thread_note_index.jsonl`;
- semantically, the note describes the stable narrow specialization or competencies of this
  specific thread, not the broad `agent_role`, prompt persona, temporary status, or current task.
- stored form is a single canonical line: `Назначение: ... | Компетенции: ...`.
- active threads still use the live session snapshot first; restart/resume recovery reads the note
  index before any embedded `ThreadSpawn` metadata.
- app-server note mutation/read surfaces are intentionally out of scope in this release
  adaptation.

Implementation surfaces:

- `protocol`: note fields on collaboration events and turn context payloads;
- `core`: note propagation across spawn, session metadata, and restart-safe persistence;
- `core/rollout`: append-only note index used for restart-safe persistence;
- `tui`: transcript rendering for spawned and sent-input collaboration rows.

Read this folder in order:

1. [design.md](design.md) for architecture, data flow, and invariants.
2. [verification.md](verification.md) for the regression matrix and validation commands.
