# DeepSeek Chat Completions

## Feature Passport

- Code name: `deepseek-chat-completions`
- Status: implemented
- Goal: allow Codex CLI to run DeepSeek models directly through DeepSeek's OpenAI-compatible Chat
  Completions API.
- Scope in: built-in `deepseek` provider, `chat_completions` wire API, DeepSeek model catalog,
  function-tool streaming, and DeepSeek `reasoning_content` round trips.
- Scope out: native Anthropic/Gemini APIs, automatic cross-provider fallback, dynamic provider
  plugins, and Responses WebSocket support for DeepSeek.

## User Contract

- Users can set `model_provider = "deepseek"` and select `deepseek-v4-pro` or
  `deepseek-v4-flash`.
- Codex reads the DeepSeek bearer token from `DEEPSEEK_API_KEY`.
- Minimal user configuration:

  ```toml
  model_provider = "deepseek"
  model = "deepseek-v4-pro"
  ```

- Users must provide credentials before starting Codex:

  ```bash
  export DEEPSEEK_API_KEY=...
  ```

- DeepSeek requests use `https://api.deepseek.com/chat/completions`.
- DeepSeek runs through `wire_api = "chat_completions"`; OpenAI/Codex behavior remains
  `wire_api = "responses"`.
- The built-in `deepseek` provider sets `wire_api = "chat_completions"` automatically. Users only
  need to specify `wire_api` when defining a custom provider.
- DeepSeek supports text input, function tools, streaming, and thinking mode.
- Image input, hosted web search, image generation, namespace tools, and Responses WebSocket are not
  exposed for the built-in DeepSeek provider.
- There is no silent provider fallback. Missing credentials, unsupported content, and unsupported
  tool shapes fail with controlled diagnostics.
- DeepSeek `reasoning_content` emitted before tool calls is stored in transcript history and passed
  back on follow-up Chat Completions requests.

## Integration And Compatibility Notes

- The implementation is fork-specific but upstream-shaped: it extends the existing
  `ModelProviderInfo`, `ModelProvider`, and `WireApi` surfaces instead of adding an external shim.
- Existing `responses` providers keep their request shape, stream parser, WebSocket path, and auth
  behavior.
- Remote thread config has a new `WIRE_API_CHAT_COMPLETIONS` value so persisted provider metadata
  can represent the new wire API.
- Older sessions that do not contain `chat_completions` metadata continue to load as `responses`
  sessions or fail on unknown provider data using existing validation paths.

## Verification Matrix

- `cargo test -p codex-api`: Chat request construction and Chat Completions SSE parsing.
- `cargo test -p codex-tools`: Chat Completions tool schema conversion.
- `cargo test -p codex-model-provider-info`: built-in DeepSeek provider and wire API parsing.
- `cargo test -p codex-model-provider`: DeepSeek model catalog/capability integration.
- `cargo test -p codex-core`: stream routing compilation and existing core behavior.
- `just write-config-schema`: update the config schema for `chat_completions`.

## Doc Changelog

- 2026-05-16: Initial contract for direct DeepSeek Chat Completions support.
