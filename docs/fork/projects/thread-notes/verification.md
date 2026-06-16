# Thread notes verification

## Required checks

- `cargo check -p codex-core -p codex-thread-store -p codex-protocol -p codex-rollout`
- `cargo check --tests -p codex-core`
- `just fmt`
- `just test -p codex-core spawn_agent_tool_v2_requires_task_name_and_lists_visible_models spawn_agent_tool_v1_keeps_legacy_fork_context_field`
- `just test -p codex-core multi_agent_v2_spawn_applies_cwd_and_thread_note_without_widening_permissions multi_agent_v2_spawn_rejects_thread_note_over_500_chars`
- `just test -p codex-thread-store resume_history_restores_thread_note_from_session_metadata`
- `git diff --check`

## Scenario matrix

- Positive note: spawn MAv2 child with `thread_note`, assert child `SessionSource::get_thread_note()` and `list_agents.thread_note`.
- Schema: MAv2 exposes `thread_note`; V1 does not.
- Negative note length: spawn rejects a note above 500 characters with a model-visible error.
- Resume metadata: `ThreadMetadataSync::for_resume` reconstructs `ThreadMetadataPatch.thread_note` from rollout `SessionMeta.thread_note`.
- Persistence: compile coverage verifies `SessionMeta`, thread-store patch/read model, and rollout recorder fields; live scenario verifies runtime propagation.

## Known gaps

No app-server/TUI/wait-agent projection and no `set_thread_note` tool yet; those are projection gaps, not blockers for the metadata substrate verified here.
