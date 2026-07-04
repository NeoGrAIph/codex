# Architecture: app-server `thread/*` RPC

Документ фиксирует current HEAD source of truth для app-server `thread/*` namespace после исследования `rust-v0.86.0`-`rust-v0.142.5`. История релизов находится в `timeline.md`, команды и triage decisions - в `evidence.md`.

## Scope

- In scope: app-server JSON-RPC methods/notifications under `thread/*`, related v2 protocol types, request processors, app-server README contract, generated schema/TypeScript fixtures, app-server suite tests and ThreadStore-backed persistence when it is the backend for those RPCs.
- Out of scope: internal session/runtime architecture when it is not exposed through `thread/*`; TUI-only navigation; multi-agent tool semantics except where they project into thread metadata such as `parentThreadId`.

## Current Source Of Truth

- Method and notification registry: `codex-rs/app-server-protocol/src/protocol/common.rs` maps request variants to method names such as `thread/start`, `thread/resume`, `thread/fork`, `thread/archive`, `thread/delete`, `thread/list`, `thread/read`, `thread/turns/list`, `thread/settings/update`, `thread/increment_elicitation`, `thread/decrement_elicitation`, `thread/approveGuardianDeniedAction`, `thread/realtime/*` and notification names such as `thread/started`, `thread/status/changed`, `thread/archived`, `thread/deleted`, `thread/name/updated`, `thread/goal/updated`, `thread/settings/updated`, `thread/tokenUsage/updated` and `thread/realtime/*`.
- v2 type definitions: `codex-rs/app-server-protocol/src/protocol/v2/thread.rs`, `thread_data.rs`, `turn.rs`, `item.rs`, `realtime.rs`, `permissions.rs` and `shared.rs` define request/response payloads, `Thread`, `ThreadStatus`, `ThreadSettings`, `MultiAgentMode` settings projections, paged turn views, item variants and related projections.
- Runtime request handling: `codex-rs/app-server/src/request_processors/thread_processor.rs` owns most thread request handlers; `thread_goal_processor.rs`, `thread_lifecycle.rs`, `thread_delete.rs`, `thread_summary.rs`, `thread_resume_redaction.rs` and `codex-rs/app-server/src/thread_state.rs` own specialized goal/lifecycle/delete/summary/redaction/live-state behavior.
- Persistence backend: `codex-rs/thread-store/src/*` owns local/in-memory store APIs, search, read/list/archive/unarchive/delete/update metadata, recency ordering, optional turn filters, session-id persistence and live-thread writing. App-server should consume ThreadStore rather than reintroducing rollout-file-specific reads for thread RPCs.
- Core bridge: `codex-rs/core/src/thread_manager.rs`, `codex-rs/core/src/session/*` and `codex-rs/rollout/src/*` are backend evidence for start/resume/fork/history/rollback behavior, but they are not the public app-server thread contract. Resume reconstruction can seed the in-memory world-state baseline from the latest `TurnContextItem`; legacy records reconstruct only the primary environment as `local` until live state re-establishes exact environment ids.
- Public documentation: `codex-rs/app-server/README.md` is the human-readable app-server contract for method semantics, examples, experimental gating and current unsupported surfaces.
- Generated artifacts: `codex-rs/app-server-protocol/src/export.rs` and generated schema/TypeScript fixtures must match any protocol shape change.
- Tests: `codex-rs/app-server/tests/suite/v2/thread_*.rs`, `realtime_conversation.rs`, `remote_thread_store.rs` and common helpers in `codex-rs/app-server/tests/common/test_app_server.rs` are the focused verification surface.

## Current RPC Surface

- Lifecycle: `thread/start`, `thread/resume`, `thread/fork`, `thread/unsubscribe`, `thread/archive`, `thread/unarchive`, `thread/delete`, `thread/compact/start`, `thread/rollback`.
- Read/list/search: `thread/list`, `thread/search`, `thread/loaded/list`, `thread/read`, `thread/turns/list`, `thread/turns/items/list`. Current docs state `thread/turns/items/list` has API shape but returns unsupported-method JSON-RPC error. Thread list/search ordering can use recency metadata when present, with compatibility for older rollout/state records.
- Metadata/settings: `thread/name/set`, `thread/metadata/update`, `thread/settings/update`, `thread/memoryMode/set`, `thread/goal/set`, `thread/goal/get`, `thread/goal/clear`, `thread/inject_items`. `ThreadSettings` retains deprecated multi-agent mode compatibility fields, but effective proactive MAv2 behavior is derived from `effort: "ultra"` rather than `multiAgentMode` request values.
- Elicitation/review helpers: `thread/increment_elicitation`, `thread/decrement_elicitation`, `thread/approveGuardianDeniedAction`.
- Execution/realtime helpers: `thread/shellCommand`, `thread/backgroundTerminals/clean`, `thread/backgroundTerminals/list`, `thread/backgroundTerminals/terminate`, `thread/realtime/start`, `thread/realtime/appendAudio`, `thread/realtime/appendText`, `thread/realtime/appendSpeech`, `thread/realtime/stop`, `thread/realtime/listVoices`.
- Notifications: lifecycle/status notifications include `thread/started`, `thread/status/changed`, `thread/archived`, `thread/deleted`, `thread/unarchived`, `thread/closed`, `thread/name/updated`, `thread/goal/updated`, `thread/goal/cleared`, `thread/settings/updated`, `thread/tokenUsage/updated`, `thread/compacted` and realtime notifications under `thread/realtime/*`.

## Key Invariants

- `thread/start`, `thread/resume` and `thread/fork` are the materialization entrypoints. Response payload and `thread/started`/subscription behavior must stay ordered so clients can safely render initial state and subsequent updates.
- `Thread` is the canonical app-server projection for user-facing thread metadata: id, session id, status, source/parent metadata, cwd/runtime roots, permissions/settings projections, name/git info, recency ordering metadata, ephemeral state and bounded turns/history views.
- List/read APIs must work without resuming a thread. Any storage mutation for unloaded threads should route through ThreadStore-backed paths.
- History payloads are bounded by default. Clients that need more history should use `thread/turns/list`, `initialTurnsPage`, incremental thread-history changes or explicit view/page controls instead of relying on full `thread.turns`.
- Durable Responses API history items should preserve `internal_chat_message_metadata_passthrough.turn_id` across persistence, resume/fork reconstruction, compaction and websocket incremental reuse. `compaction_trigger` remains a request control rather than a durable response item with metadata.
- Session ids are durable thread metadata across resume/read/list flows. Resume implementations should preserve existing session identity and lineage rather than minting unrelated session ids for the same stored thread.
- `thread/inject_items` must reject remote HTTP(S) image URLs at app-server ingress while keeping documented legacy resume/history compatibility and supported local/data image paths intact.
- Experimental method/field gates live in protocol annotations and initialize capabilities, not ad hoc request-processor checks only.
- Thread-scoped helper RPCs such as elicitation counters and Guardian approval must still route through protocol types, generated artifacts and app-server request processors; they should not become hidden side channels outside `thread/*`.
- Destructive operations (`thread/delete`, archive cascade) must use persisted lineage/store state to affect only documented descendants and emit notifications for each affected thread.
- Realtime and background terminal APIs are thread-scoped but should remain separate from ordinary `ThreadItem` history unless the protocol explicitly projects an item or notification. Realtime append/handoff controls live under `thread/realtime/*` and must stay aligned with app-server protocol capabilities and README examples.

## Verification Map

- Protocol shape: run app-server-protocol export/schema tests after type or method changes; check generated fixtures when request/response fields or experimental annotations change.
- Handler behavior: run focused app-server suite tests for touched methods, for example `thread_start`, `thread_resume`, `thread_fork`, `thread_list`, `thread_read`, `thread_settings_update`, `thread_search`, `thread_delete`, `thread_unsubscribe`, `thread_rollback` or `realtime_conversation`.
- Storage behavior: when read/list/history/archive/delete/metadata changes, include ThreadStore tests or app-server tests that exercise unloaded stored threads, not only loaded runtime threads.
- Compatibility behavior: for old rollouts/missing metadata, verify absence of optional fields remains readable and response projections use documented defaults such as `ThreadStatus::NotLoaded` for unloaded threads.
- Documentation: update `codex-rs/app-server/README.md` together with method semantics changes, and keep `docs/fork/research/thread/*` current through the update prompt when new stable releases add thread entries.
