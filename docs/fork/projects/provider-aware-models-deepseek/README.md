# Provider-aware models and DeepSeek project dossier

## Status

Current status: draft implementation in `feature/140/provider-model-catalog` dirty worktree, 2026-06-17. Historical reference commit: `c9ff8c1e40` from `fork/130` / `chrome_plugin`, subject `feat(deepseek): add chat completions provider`, dated 2026-05-16.

## Canonical links

- Feature contract: `docs/fork/features/provider-aware-models-deepseek.md`
- Design: `docs/fork/projects/provider-aware-models-deepseek/design.md`
- Verification: `docs/fork/projects/provider-aware-models-deepseek/verification.md`
- Historical source reference: `fork/130` commit `c9ff8c1e40`

## Implementation map

- Provider/config source of truth: `codex-rs/model-provider-info/src/lib.rs`, `codex-rs/core/config.schema.json`
- DeepSeek static model catalog: `codex-rs/model-provider/src/deepseek.rs`, `codex-rs/model-provider/src/provider.rs`
- Chat Completions transport: `codex-rs/codex-api/src/common.rs`, `codex-rs/codex-api/src/endpoint/chat_completions.rs`, `codex-rs/codex-api/src/sse/chat_completions.rs`, `codex-rs/core/src/client.rs`, `codex-rs/tools/src/tool_spec.rs`
- App-server protocol/catalog/settings projection: `codex-rs/app-server-protocol/src/protocol/v2/model.rs`, `codex-rs/app-server-protocol/src/protocol/v2/thread.rs`, `codex-rs/app-server/src/request_processors/catalog_processor.rs`, `codex-rs/app-server/src/request_processors/turn_processor.rs`
- Runtime settings path: `codex-rs/protocol/src/protocol.rs`, `codex-rs/core/src/codex_thread.rs`, `codex-rs/core/src/session/handlers.rs`, `codex-rs/core/src/session/session.rs`, `codex-rs/core/src/thread_manager.rs`
- TUI picker/persistence: `codex-rs/tui/src/app_server_session.rs`, `codex-rs/tui/src/chatwidget/model_popups.rs`, `codex-rs/tui/src/app/event_dispatch.rs`, `codex-rs/tui/src/app/thread_settings.rs`, `codex-rs/tui/src/config_update.rs`

## Current behavior summary

The active provider/model pair is selected through native settings. App-server clients may request `model/list` with `includeConfiguredProviders: true` to receive models tagged with `modelProvider`. The TUI uses that catalog for `/model`, sends provider and model together to `thread/settings/update`, and persists the same pair into config.

DeepSeek is a built-in configured provider named `deepseek`; its first iteration model catalog is static, so it can be displayed without probing the DeepSeek API. Actual inference requires `DEEPSEEK_API_KEY` and uses the new DeepSeek-compatible Chat Completions transport. DeepSeek does not advertise freeform `apply_patch` in this iteration; ordinary function tools remain available.
