# Provider-aware models and DeepSeek design

## Canonical state

The canonical state is the runtime pair `(model_provider_id, model)`. `model_provider_id` names an entry in `Config.model_providers`; `model` names a `ModelPreset` provided by that provider's model manager. The default provider remains `openai`, and old protocol/model payloads that do not include provider information deserialize as OpenAI where a default is required for compatibility.

## Native propagation path

1. `model-provider-info` defines provider identity and wire API. `built_in_model_providers` produces the built-in OpenAI, Amazon Bedrock, OSS providers and now DeepSeek.
2. Config loading merges built-ins and user configured providers into `Config.model_providers`, sets `Config.model_provider_id`, and exposes the active `Config.model_provider`.
3. `model-provider` creates the provider-specific `ModelsManager`. DeepSeek uses a static catalog through the same manager factory entrypoint as other providers.
4. OpenAI-compatible remote managers keep the legacy global cache only for OpenAI. Non-OpenAI remote providers use provider-scoped cache paths, so inactive provider catalog projection cannot reuse an active provider's `models_cache.json`.
5. app-server `model/list` keeps the upstream/default `ThreadManager` catalog path unless the caller sets `includeConfiguredProviders: true`. In provider-aware mode it iterates configured providers, tags each `ModelPreset` with provider id, and returns app-server `Model.modelProvider`.
6. TUI `/model` reads app-server models, keeps provider id on `ModelPreset`, and emits provider-aware selection events.
7. TUI sends `thread/settings/update` with `modelProvider` and `model`; core validates the provider against configured providers and updates the session config used by the next turn.
8. `ModelClientSession::stream` selects Responses or DeepSeek-compatible Chat Completions from `ModelProviderInfo.wire_api` at the transport boundary.

## Data flow

`ModelProviderInfo` -> `Config.model_providers` -> `ConfiguredModelProvider::models_manager` -> `ModelPreset { model_provider, model, ... }` -> app-server `Model { modelProvider, model, ... }` -> TUI selection -> `ThreadSettingsUpdateParams { modelProvider, model, ... }` -> `ThreadSettingsOverrides` -> `SessionSettingsUpdate` -> `SessionConfiguration` -> next turn `ModelClientSession`.

## Invariants

- There is one provider/model source of truth: `Config.model_providers` plus provider-owned `ModelsManager`; UI and app-server do not own provider/model lists.
- `model_provider` and `model` are updated together when the picker changes model provider.
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

The Chat Completions adapter converts the existing Responses-shaped internal input into DeepSeek-compatible chat messages only at the transport boundary. This keeps upstream session/history/context construction unchanged and limits divergence to the provider wire adapter. Because the request includes DeepSeek-specific `thinking` and reasoning-effort mapping, `wire_api = "chat_completions"` is not yet a generic provider-neutral Chat Completions contract.

For historical context, the adapter converts messages, reasoning content, function calls and function call outputs. Function call outputs are accepted when they are plain text or `content_items` containing only `input_text`; `input_image` and `encrypted_content` fail fast with `InvalidRequest`. Persisted items without a chat-compatible representation, such as local shell calls, tool search calls, custom tools, web/image calls, compaction markers and unknown items, fail fast rather than being silently omitted.

## Compatibility behavior

Existing user configs without `model_provider` continue to default to OpenAI. Existing app-server clients that do not send `includeConfiguredProviders` receive the same active-provider catalog as before. Existing model payloads without `modelProvider` deserialize as OpenAI where protocol compatibility requires a concrete provider id.

Older persisted sessions do not need migration. If a session has no new provider fields, resume/thread summaries follow the existing fallback provider behavior from the config snapshot/session metadata path. If a user selects DeepSeek, subsequent turns use `chat_completions`; existing OpenAI turns still use `responses`.
