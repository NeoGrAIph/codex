# Thread notes verification

## Required checks

- `cargo check -p codex-core -p codex-thread-store -p codex-protocol -p codex-rollout`
- `cargo check --tests -p codex-core`
- `just fmt`
- `just test -p codex-core spawn_agent_tool_v2_requires_task_name_and_lists_visible_models spawn_agent_tool_v1_keeps_legacy_fork_context_field`
- `just test -p codex-core multi_agent_v2_spawn_applies_cwd_and_thread_note_without_widening_permissions multi_agent_v2_spawn_rejects_thread_note_over_500_chars`
- `just test -p codex-thread-store resume_history_restores_thread_note_from_session_metadata`
- `just test -p codex-app-server thread_metadata_update_patches_thread_note_through_source_metadata thread_metadata_update_rejects_thread_note_over_500_chars`
- `just test -p codex-tui agent_picker_thread_detail_uses_top_level_thread_note_and_clears_stale_note`
- `just test -p codex-core multi_agent_v2_wait_agent`
- `just test -p codex-core multi_agent_v2_set_thread_note`
- `just test -p codex-core set_thread_note_tool_requires_nullable_thread_note`
- `just test -p codex-app-server-protocol reconstructs_collab_spawn_end_item_with_model_metadata collab_resume_end_maps_to_item_completed_resume_agent`
- `just test -p codex-app-server-protocol thread_note_updated_notification_round_trips`
- `just test -p codex-exec collab_spawn_begin_and_end_emit_item_events`
- `just test -p codex-tui thread_note_updated_notification_updates_agent_picker_cache thread_note_updated_targets_thread`
- `git diff --check`

## Scenario matrix

- Positive note: spawn MAv2 child with `thread_note`, assert child `SessionSource::get_thread_note()` and `list_agents.thread_note`.
- Schema: MAv2 exposes `thread_note`; V1 does not.
- Negative note length: spawn rejects a note above 500 characters with a model-visible error.
- Resume metadata: `ThreadMetadataSync::for_resume` reconstructs `ThreadMetadataPatch.thread_note` from rollout `SessionMeta.thread_note`.
- App-server post-spawn update: `thread/metadata/update.threadNote` sets, normalizes and clears the note through native thread metadata update; the returned/read `Thread.source` projection contains the updated `ThreadSpawn.thread_note` and top-level `Thread.threadNote` mirrors the same metadata.
- Wait projection: MAv2 `wait_agent` returns a visible-agent snapshot with `thread_note`, using the same native projection as `list_agents` and without exposing mailbox or final-answer content.
- Model-visible update: MAv2 `set_thread_note` sets, normalizes and clears a visible spawned sub-agent note through native `ThreadMetadataPatch`, appends restart-safe rollout `SessionMeta.thread_note`, refreshes live `AgentMetadata`, and rejects root targets.
- Collab state projection: app-server spawn/send/wait/close/resume tool-call notifications and reconstructed thread history carry `CollabAgentState.threadNote` from existing event metadata without introducing a separate note store.
- Dedicated app-server notification: successful `thread/metadata/update.threadNote` emits `thread/note/updated` after the response, with normalized string on set and `null` on clear; TUI updates existing `/agent` cache from that notification without `thread/read`.
- TUI workbench: selected `/agent` detail shows `Note:` from top-level `Thread.threadNote` with fallback to `Thread.source`, renders no synthetic note when missing, and clears stale cached note when app-server returns `None`.
- Protocol schema: `just test -p codex-app-server-protocol` verifies generated app-server schema fixtures include `SubAgentSource.thread_note` and `Thread.threadNote`.
- Persistence: compile coverage verifies `SessionMeta`, thread-store patch/read model, and rollout recorder fields; live scenario verifies runtime propagation.

## Known gaps

The generated app-server protocol schema includes `SubAgentSource.thread_note` through shared source metadata, top-level `Thread.threadNote` as a read projection, `CollabAgentState.threadNote` for existing collab tool-call state, and `ThreadNoteUpdatedNotification` for app-server set/clear updates. `thread/metadata/update.threadNote` provides the native app-server update/clear contract, and MAv2 `set_thread_note` provides the model-visible update/clear contract for spawned sub-agents. TUI rendering is covered by the existing `/agent` workbench detail projection over `Thread.threadNote`/`Thread.source`, not by a separate TUI note store.

## Verification log

- 2026-06-18: `just test -p codex-core spawn_agent_tool_v2_requires_task_name_and_lists_visible_models spawn_agent_tool_v1_keeps_legacy_fork_context_field multi_agent_v2_spawn_applies_cwd_and_thread_note_without_widening_permissions multi_agent_v2_spawn_rejects_cwd_outside_workspace_roots multi_agent_v2_spawn_rejects_nonexistent_cwd_inside_workspace_roots multi_agent_v2_spawn_rejects_thread_note_over_500_chars` прошёл 6/6; positive note propagation and 500-character fail-fast validation confirmed alongside cwd spawn contract.
- 2026-06-18: `just test -p codex-thread-store resume_history_restores_thread_note_from_session_metadata` прошёл 1/1; restart/resume reconstruction from `SessionMeta.thread_note` into thread metadata patch confirmed.
- 2026-06-18: `just test -p codex-app-server-protocol typescript_schema_fixtures_match_generated json_schema_fixtures_match_generated` прошёл 2/2; generated schema fixtures remain in sync with shared `SubAgentSource.thread_note` projection.
- 2026-06-18: `just test -p codex-tui agent_picker_selected_description_includes_workbench_detail agent_picker_workbench_snapshot` прошёл в составе focused workbench checks; selected detail snapshot includes `Note: Investigate parser state` from native thread metadata hydration.
- 2026-06-18: после синхронизации thread-notes docs повторно выполнен `just test -p codex-tui agent_picker_selected_description_includes_workbench_detail agent_picker_workbench_snapshot`; результат 2/2 passed, confirming current `/agent` workbench note rendering.
- 2026-06-18: Spark test-runner confirmed `just test -p codex-app-server thread_metadata_update_patches_thread_note_through_source_metadata thread_metadata_update_rejects_thread_note_over_500_chars`; app-server post-spawn update/clear and over-500-character rejection passed.
- 2026-06-18: After `just write-app-server-schema`, the reused Spark test-runner confirmed `just test -p codex-app-server-protocol typescript_schema_fixtures_match_generated json_schema_fixtures_match_generated`; scoped `git diff --check` over code, docs and generated schema artifacts also passed.
- 2026-06-18: Added top-level app-server `Thread.threadNote` read projection without sqlite migration. Reused Kepler as the existing Spark test-runner; focused checks passed: app-server-protocol schema fixtures 2/2, app-server metadata update/read projection 2/2, TUI top-level-note/clear/detail tests 3/3, no pending snapshots, scoped `git diff --check`, and no migration/sql file changes.
- 2026-06-18: Added MAv2 `wait_agent` visible-agent snapshot projection for `thread_note` without exposing mailbox/final-answer content. Reused Kepler for focused checks: `just test -p codex-core multi_agent_v2_wait_agent` passed 11/11, `just test -p codex-core wait_agent_tool_v2_uses_timeout_only_summary_output list_agents_tool_includes_path_prefix_and_agent_fields` passed 2/2, scoped `git diff --check` passed, and stale wording search found no remaining wait-agent gap claim.
- 2026-06-18: Added MAv2 model-visible `set_thread_note` through native `ThreadMetadataPatch` and live `AgentMetadata` refresh. Reused Kepler for focused checks: `just test -p codex-core multi_agent_v2_set_thread_note` passed 2/2, `just test -p codex-core set_thread_note_tool_requires_nullable_thread_note` passed 1/1, `just test -p codex-thread-store sqlite_failures` passed 3/3, no pending snapshots and scoped `git diff --check` passed.
- 2026-06-18: Added `CollabAgentState.threadNote` through existing app-server collab tool-call notifications/history, with no sqlite migration and no dedicated note notification. Reused Kepler for focused checks: `just test -p codex-app-server-protocol collab_agent_state_maps_interrupted_status reconstructs_collab_spawn_end_item_with_model_metadata collab_resume_end_maps_to_item_completed_resume_agent`, `just test -p codex-exec collab_spawn_begin_and_end_emit_item_events`, `just test -p codex-exec turn_items_for_thread_returns_matching_turn_items session_configured_from_thread_response_preserves_parent_thread_id`, `just test -p codex-tui collab_spawn_end_shows_requested_model_and_effort`, schema generation, no pending snapshots and scoped `git diff --check`.
- 2026-06-18: Added dedicated app-server `thread/note/updated` notification emitted after successful `thread/metadata/update.threadNote` set/clear. TUI consumes it through existing thread notification routing and updates `/agent` cache without a follow-up `thread/read`. Reused Kepler for focused checks: `just test -p codex-app-server-protocol thread_note_updated_notification_round_trips`, app-server-protocol schema fixture tests, `just test -p codex-app-server thread_metadata_update_patches_thread_note_through_source_metadata thread_metadata_update_rejects_thread_note_over_500_chars`, `just test -p codex-tui thread_note_updated_notification_updates_agent_picker_cache thread_note_updated_targets_thread`, no pending snapshots and scoped `git diff --check`.
- 2026-06-18: Current-state audit confirmed docs still match the implemented metadata-only thread-note contract. Kepler reran the cross-surface smoke set: `just test -p codex-core multi_agent_v2_set_thread_note set_thread_note_tool_requires_nullable_thread_note multi_agent_v2_wait_agent`, `just test -p codex-app-server thread_metadata_update_patches_thread_note_through_source_metadata thread_metadata_update_rejects_thread_note_over_500_chars`, `just test -p codex-app-server-protocol thread_note_updated_notification_round_trips`, `just test -p codex-tui thread_note_updated_notification_updates_agent_picker_cache thread_note_updated_targets_thread agent_picker_thread_detail_uses_top_level_thread_note_and_clears_stale_note`, and scoped docs `git diff --check`; all passed.
