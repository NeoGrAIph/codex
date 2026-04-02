# Thread Note Project

This dossier describes how `thread_note` is implemented in the current `fork/118` workstream and
where to look when the feature changes.

Canonical references:

- Feature contract: [../../features/thread-note.md](../../features/thread-note.md)
- Release research: [../../research/0.118.0/thread-note.md](../../research/0.118.0/thread-note.md)

Current status:

- `thread_note` is metadata-only and remains out of model-facing prompt/context surfaces
- the canonical note format is a single line:
  `Назначение: ... | Компетенции: ...`
- spawned and send-input transcript rows render `Note:` from event snapshots or cached metadata
- restart/resume prefers rollout-backed persisted note state, then falls back to legacy note-index
  data when needed
- app-server v2 collaboration history state now carries `thread_note`, but top-level `Thread`
  payloads still do not expose it

Implementation surfaces:

- `protocol`: note normalization, note-bearing event fields, session and turn metadata
- `core`: spawn/update propagation, restart-safe restoration, tool handlers
- `rollout`: session meta, turn context persistence, legacy note-index fallback
- `app-server`: collab event projection and thread-history reconstruction
- `tui`: cached collab metadata and `Note:` transcript rendering

Read this folder in order:

1. [design.md](design.md) for architecture, source of truth, and invariants
2. [verification.md](verification.md) for scenarios and validation commands
