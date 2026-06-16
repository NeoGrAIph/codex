# Provider-aware models and DeepSeek verification

## Required checks

| Command | Purpose | Current result |
| --- | --- | --- |
| `just write-config-schema` | Regenerate config schema for `wire_api = "chat_completions"` and built-in provider metadata. | Passed during draft verification on 2026-06-17. |
| `just write-app-server-schema` | Regenerate app-server JSON/TypeScript schemas for `includeConfiguredProviders`, `modelProvider`, and `thread/settings/update.modelProvider`. | Passed during draft verification on 2026-06-17. |
| `just test -p codex-model-provider-info` | Validate provider info parsing and DeepSeek built-in provider contract. | Passed: 23 tests. |
| `just test -p codex-model-provider deepseek_catalog_does_not_advertise_freeform_apply_patch deepseek_provider_disables_unsupported_hosted_tools` | Validate provider factory/model manager integration and DeepSeek static catalog/tool capability contract. | Passed: 2 tests. |
| `just test -p codex-api` | Validate Chat Completions request/SSE adapter. | Passed: 125 tests. |
| `just test -p codex-api chat_completions_rejects` | Validate unsupported Chat Completions history items and unsupported structured function-call outputs fail request construction. | Passed: 4 tests. |
| `just test -p codex-tools` | Validate Chat Completions tool conversion. | Passed: 86 tests. |
| `just test -p codex-app-server-protocol` | Validate protocol serialization and generated schema fixtures. | Passed: 231 tests. |
| `just test -p codex-app-server list_models` | Validate default/provider-aware app-server catalog surfaces and inactive-provider cache isolation. | Passed: 7 tests. |
| `just test -p codex-app-server thread_settings_update` | Validate app-server settings update compatibility. | Passed: 6 tests. |
| `just test -p codex-core session_settings_model_provider_update provider_and_model_change_uses_chat_completions_next_turn provider_change_before_first_turn_discards_startup_prewarm` | Validate runtime settings/session provider switch, fail-fast unknown provider behavior, next-turn Chat Completions transport and provider-scoped startup prewarm. | Passed: 4 tests. |
| `just test -p codex-tui model_reasoning_selection_popup_applies_custom_effort single_reasoning_option_skips_selection` | Validate TUI picker actions and provider-aware persistence event path. | Passed: 2 tests. |
| `just fmt` | Apply repository formatting. | Passed on 2026-06-17. |
| `just fix -p codex-core -p codex-model-provider -p codex-app-server -p codex-models-manager` | Scoped Clippy fix/check for touched Rust packages. | Passed on 2026-06-17. |
| `git diff --check` | Whitespace check. | Passed on 2026-06-17. |

## Scenario matrix

| Scenario | Expected result | Evidence path |
| --- | --- | --- |
| Default app-server `model/list` without `includeConfiguredProviders` | Uses the existing active thread/provider catalog and preserves old output behavior except compatible `modelProvider` defaults. | `app-server/tests/suite/v2/model_list.rs` default list tests. |
| Provider-aware app-server `model/list` | Returns configured/static provider catalogs tagged with `modelProvider`; DeepSeek appears as `deepseek`. | `list_models_can_include_configured_provider_catalogs`. |
| Inactive provider cache isolation | A model seeded into the active OpenAI global cache is not exposed as an inactive configured provider model. | `list_models_does_not_reuse_active_provider_cache_for_inactive_provider`. |
| TUI `/model` selection for DeepSeek | Sends `UpdateModelSelection`/`PersistProviderModelSelection`, then `thread/settings/update` with provider and model. | `codex-rs/tui/src/chatwidget/model_popups.rs`, `app/event_dispatch.rs`, `app/thread_settings.rs`; `just test -p codex-tui`. |
| Runtime settings update with known provider | Updates active session provider/model and next turn uses the provider's wire API. | `session_settings_model_provider_update`; `provider_and_model_change_uses_chat_completions_next_turn`. |
| Provider switch before first turn with startup prewarm | OpenAI websocket startup prewarm is prepared, provider switches to DeepSeek before first user turn, and first request goes to `/chat/completions` rather than reusing the OpenAI prewarm. | `provider_change_before_first_turn_discards_startup_prewarm`. |
| Runtime settings update with unknown provider | Fails with `ConstraintError::InvalidValue` and does not silently fallback. | `core/src/session/session.rs`; `just test -p codex-core`. |
| DeepSeek request transport | `WireApi::ChatCompletions` posts to `/chat/completions`, includes DeepSeek-specific `thinking`/reasoning fields and maps SSE deltas to existing response events. | `provider_and_model_change_uses_chat_completions_next_turn`; `codex-api` tests. |
| DeepSeek tool catalog | DeepSeek static models do not advertise freeform `apply_patch`; provider capabilities disable hosted web/image tools; ordinary function tools still build for Chat Completions. | `deepseek_catalog_does_not_advertise_freeform_apply_patch`; `deepseek_provider_disables_unsupported_hosted_tools`; `provider_and_model_change_uses_chat_completions_next_turn`. |
| Unsupported Chat Completions inputs | Images and unsupported Responses-only tools are rejected before request construction. | `codex-api`/`codex-tools` tests and core request builder. |
| Unsupported Chat Completions history | Persisted/history items without chat-compatible projection and structured `function_call_output` image/encrypted content are rejected with `InvalidRequest` instead of being silently dropped. | `chat_completions_rejects_local_shell_history_item`; `chat_completions_rejects_unknown_history_item`; `chat_completions_rejects_image_function_call_output_content`; `chat_completions_rejects_encrypted_function_call_output_content`. |
| Old config/session compatibility | Missing provider fields default/fallback through existing OpenAI/config snapshot behavior; no migration required. | Protocol serde defaults, session config path and app-server protocol tests. |

## Known coverage gaps

- No live DeepSeek API call is part of local verification; local tests validate request building, SSE parsing and provider selection against a mock `/chat/completions` server without external credentials.
- The first iteration does not implement a dynamic DeepSeek remote model list. Static catalog updates must be maintained in `codex-rs/model-provider/src/deepseek.rs`.
- `wire_api = "chat_completions"` is DeepSeek-compatible rather than generic provider-neutral Chat Completions until provider-specific request options are introduced.
- Full workspace `just test` is not required for this draft unless reviewers request broader coverage; focused crate tests cover touched native surfaces.
- Full `just test -p codex-core` and full `just test -p codex-app-server` were attempted once while another worktree was running a heavy `codex-core` nextest job. They produced unrelated temp/git trust failures and app-server initialize timeouts, then were interrupted. After the external run ended, the focused provider/model app-server and core tests passed.
