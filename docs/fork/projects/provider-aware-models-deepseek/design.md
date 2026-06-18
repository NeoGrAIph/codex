# Provider-aware models and DeepSeek design

## Canonical state

The canonical runtime state is the pair `(model_provider_id, model)`. `model_provider_id` names an entry in `Config.model_providers`; `model` names a `ModelPreset` provided by that provider's model manager. The default provider remains `openai`, and old protocol/model payloads that do not include provider information deserialize as OpenAI where a default is required for compatibility.

Provider management adds two adjacent canonical states without replacing the runtime pair: `Config.disabled_model_providers` stores provider ids hidden from provider-aware pickers, and `codex-secrets` stores managed provider API keys. Provider API keys are resolved env-first through `ModelProviderInfo.env_key`; managed secrets are materialized into provider auth only when the env key is absent.

## Native propagation path

1. `model-provider-info` defines provider identity and wire API. `built_in_model_providers` produces the built-in OpenAI, Amazon Bedrock, OSS providers and now DeepSeek.
2. Config loading merges built-ins and user configured providers into `Config.model_providers`, sets `Config.model_provider_id`, and exposes the active `Config.model_provider`.
3. `model-provider` creates the provider-specific `ModelsManager`. DeepSeek uses a static catalog through the same manager factory entrypoint as other providers.
4. OpenAI-compatible remote managers keep the legacy global cache only for OpenAI. Non-OpenAI remote providers use provider-scoped cache paths, so inactive provider catalog projection cannot reuse an active provider's `models_cache.json`.
5. app-server `modelProvider/list` reads configured providers, active/default provider id, `disabled_model_providers`, auth status and offline/static model counts through the same provider model manager path used by `model/list`; native `openai` is omitted from this management projection.
6. app-server `modelProvider/config/write` writes `model_provider`, a compatible default `model` and `disabled_model_providers` through `ConfigBatchWriteParams`, while rejecting direct attempts to disable native `openai` and rejecting combined active+hidden writes; `modelProvider/auth/write` and `modelProvider/auth/remove` write managed keys through `codex-secrets` for configurable providers and reject native OpenAI auth changes.
7. app-server `model/list` keeps the upstream/default `ThreadManager` catalog path unless the caller sets `includeConfiguredProviders: true`. In provider-aware mode it iterates configured providers, skips disabled inactive providers except native OpenAI, tags each `ModelPreset` with provider id, and returns app-server `Model.modelProvider`.
8. TUI `/model` reads app-server models, keeps provider id on `ModelPreset`, and emits provider-aware selection events.
9. TUI `/model-providers` reads app-server provider state, hides native OpenAI from management, uses `SelectionToggle` for `/plugins`-style Space enable/disable on rows, opens a detail level on Enter for default/auth actions, then refreshes the provider-aware model catalog so `/model` inherits the new state.
10. TUI sends `thread/settings/update` with `modelProvider` and `model`; core validates the provider against configured providers and updates the session config used by the next turn.
11. `ModelClientSession::stream` selects Responses or Chat Completions from `ModelProviderInfo.wire_api` at the transport boundary. Chat Completions request construction carries explicit provider options: DeepSeek receives `thinking` and mapped `reasoning_effort`, while the generic option set omits those DeepSeek-only fields.

## Data flow

Model selection data flow: `ModelProviderInfo` -> `Config.model_providers` -> `ConfiguredModelProvider::models_manager` -> `ModelPreset { model_provider, model, ... }` -> app-server `Model { modelProvider, model, ... }` -> TUI selection -> `ThreadSettingsUpdateParams { modelProvider, model, ... }` -> `ThreadSettingsOverrides` -> `SessionSettingsUpdate` -> `SessionConfiguration` -> next turn `ModelClientSession`.

Provider management data flow: `Config.disabled_model_providers` + `Config.model_provider_id` + `codex-secrets` + provider env vars -> app-server `ModelProvider { active, enabledInPicker, authStatus, ... }` for configurable non-OpenAI providers -> TUI `/model-providers` Space toggle or detail action -> app-server config/secrets mutation -> TUI refreshes `modelProvider/list` and provider-aware `model/list`.

## Invariants

- There is one provider/model source of truth: `Config.model_providers` plus provider-owned `ModelsManager`; UI and app-server do not own provider/model lists.
- There is one picker visibility source of truth: `Config.disabled_model_providers`. It is denied in project-local config because provider visibility is a user-level preference and project-local config must not hide user providers.
- OpenAI is native, default-enabled and non-manageable in provider management. It is not returned by `modelProvider/list`, cannot be disabled through `modelProvider/config/write`, rejects `modelProvider/auth/write/remove`, and stale `disabled_model_providers` entries for `openai` do not hide OpenAI models from provider-aware `model/list`.
- There is one managed key source of truth: `codex-secrets` local encrypted storage. API keys are not persisted in `config.toml`, TUI settings state or session event log, and Debug formatting for the TUI event/protocol params redacts the submitted key. The transient `modelProvider/auth/write` protocol request necessarily contains `apiKey` while the request is handled; default non-experimental schema fixtures filter that experimental method and no plaintext key is stored by schema generation.
- Env key auth remains first-class native behavior. If `DEEPSEEK_API_KEY` is present, runtime auth uses it; managed key is used when the env key is absent.
- `model_provider` and `model` are updated together when the picker changes model provider. `modelProvider/config/write(setActive=true)` also persists a provider-compatible default model, so user config cannot become `deepseek` plus an OpenAI model slug through the provider manager.
- `/model` treats `(model_provider, model)` as the selectable identity. If two providers expose the same model slug, the all-models popup disambiguates the row label with provider id while preserving the original provider/model pair in the selection event.
- `/model-providers` may update the default provider in config, but current-thread runtime provider/model is still changed through `/model` and `thread/settings/update`.
- Active/default provider cannot be disabled through app-server provider management, including a single request that tries to set a provider active and hidden. If manually disabled in config, active provider remains visible in provider-aware `model/list` to preserve a recoverable picker state.
- Default OpenAI behavior remains available through the existing no-`includeConfiguredProviders` catalog path and `WireApi::Responses`.
- Unknown provider ids fail fast during settings apply.
- Chat Completions receives only supported request shapes. Unsupported images or Responses-only tools stop before network I/O.
- Chat Completions history projection is not lossy: unsupported persisted/history `ResponseItem` variants and unsupported structured `function_call_output` content stop request construction with `InvalidRequest`.
- DeepSeek static catalog does not advertise freeform `apply_patch` until Chat Completions has a compatible conversion path.
- DeepSeek provider capabilities disable hosted web search and image generation, because those native tool specs are not supported by the first Chat Completions adapter.
- Inactive providers must not be probed online just because a client asks for provider-aware catalog. Active provider keeps existing online-if-uncached catalog refresh; inactive providers use offline/static data.
- Inactive non-static providers must not reuse active-provider cache entries; cache identity is scoped for non-OpenAI remote managers.
- Startup websocket prewarm is provider-scoped. Provider-changing settings updates abort the pending startup prewarm, and turn execution rejects a prewarmed client session whose provider differs from the turn context.
- No database migration is required; provider state already exists in config/thread metadata paths.

## Implementation tradeoffs

The first iteration uses a static DeepSeek model catalog rather than calling DeepSeek's model list endpoint. This keeps `/model` deterministic, avoids API probing during picker bootstrap, and matches the current provider-manager pattern used for provider catalogs that are not OpenAI `/models` compatible.

`includeConfiguredProviders` is explicit on app-server `model/list` to avoid changing existing clients. TUI opts into the provider-aware path because `/model` is the user-facing surface that needs cross-provider discovery.

`disabled_model_providers` is a user config list rather than a provider-local field because picker visibility is not a provider capability; the same provider definition can be visible or hidden for a given user. The list is filtered in provider-aware projections only, so legacy `model/list` and active thread runtime behavior remain unchanged.

OpenClaude `v0.19.0` `ProviderManager` is recorded as a UX reference for future provider-management polish (`~/repo/AGENTS/openclaude/docs/ui-snapshots/provider-settings-onboarding.snap`). It demonstrates a stepwise provider setup flow, secret redaction after entry, explicit workspace-scoped profile writes, dense settings tabs and honest fallback rows for unsupported usage. The current fork iteration intentionally does not copy that wizard wholesale because Codex already has native app-server/config/secrets paths; instead, those qualities should guide future improvements around adding new providers, editing credentials and presenting unsupported usage/capability states.

Managed API keys reuse `codex-secrets` because it already provides local encrypted storage with an OS-keyring-protected passphrase. The feature does not introduce a provider-specific plaintext key file or a second auth registry. App-server integration tests set a debug/test-only fixed secrets keyring passphrase for the child app-server process so encrypted storage can be tested without depending on host `org.freedesktop.secrets`.

The Chat Completions adapter converts the existing Responses-shaped internal input into chat messages only at the transport boundary. This keeps upstream session/history/context construction unchanged and limits divergence to the provider wire adapter. Provider-specific request options live in the adapter contract: DeepSeek gets `thinking` and DeepSeek reasoning-effort mapping; generic Chat Completions request options intentionally omit those fields until a provider contract proves they are supported.

Developer/context messages are also normalized at this same boundary: `ResponseItem::Message { role: "developer" }` becomes a Chat Completions `system` message because the DeepSeek-compatible endpoint accepts `system` but rejects `developer`. Supported chat roles pass through unchanged, while unknown message roles fail fast with `InvalidRequest`. This preserves native Codex context construction and avoids TUI/model-picker workarounds.

For historical context, the adapter converts messages, reasoning content, function calls and function call outputs. Function call outputs are accepted when they are plain text or `content_items` containing only `input_text`; `input_image` and `encrypted_content` fail fast with `InvalidRequest`. Persisted items without a chat-compatible representation, such as local shell calls, tool search calls, custom tools, web/image calls, compaction markers and unknown items, fail fast rather than being silently omitted.

## Compatibility behavior

Existing user configs without `model_provider` continue to default to OpenAI. Existing user configs without `disabled_model_providers` behave as an empty disabled list. Existing app-server clients that do not send `includeConfiguredProviders` receive the same active-provider catalog as before. Existing model payloads without `modelProvider` deserialize as OpenAI where protocol compatibility requires a concrete provider id.

Older persisted sessions do not need migration. If a session has no new provider fields, resume/thread summaries follow the existing fallback provider behavior from the config snapshot/session metadata path. If a user selects DeepSeek, subsequent turns use `chat_completions`; existing OpenAI turns still use `responses`.

Managed provider keys are optional compatibility state. Removing a managed key does not remove or rewrite `DEEPSEEK_API_KEY`; if the env key exists, auth status becomes `EnvKeyPresent` and runtime continues to use native env auth. If neither env key nor managed key exists, auth status becomes `EnvKeyMissing` for env-key providers.
