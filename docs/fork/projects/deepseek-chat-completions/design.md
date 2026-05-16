# DeepSeek Chat Completions Design

## Canonical State

- `ModelProviderInfo.wire_api` selects the transport. `responses` remains the default; DeepSeek uses
  `chat_completions`.
- The built-in provider id is `deepseek`; the display name is `DeepSeek`.
- DeepSeek credentials are read from `DEEPSEEK_API_KEY` through the existing provider auth path.
- DeepSeek model metadata is a static catalog with `deepseek-v4-pro` and `deepseek-v4-flash`.

## Data Flow

- Core builds a provider-neutral prompt as existing `ResponseItem` history.
- For `chat_completions`, core converts tool specs to Chat Completions function tools and calls
  `ChatCompletionsApiRequest::new`.
- `codex-api` converts `ResponseItem` history to Chat `messages`, including `reasoning_content`
  from previous DeepSeek reasoning items.
- The Chat SSE parser accumulates `reasoning_content`, text, tool call deltas, and usage, then emits
  existing `ResponseEvent` values.
- Session handling remains unchanged after the adapter emits `ResponseItem::Reasoning`,
  `ResponseItem::Message`, `ResponseItem::FunctionCall`, and `ResponseEvent::Completed`.

## Invariants

- OpenAI Responses request/stream code is not used for DeepSeek.
- DeepSeek `reasoning_content` is never silently discarded before a tool follow-up.
- Provider capabilities hide unsupported hosted tools instead of exposing tools the provider cannot
  execute.
- Fallback between providers is not automatic.

## Tradeoffs

- The first adapter supports DeepSeek directly rather than adding a generic Anthropic-style shim.
- The adapter is intentionally text/function-tool focused; non-text multimodal support is rejected
  until a provider contract and tests exist.
- DeepSeek model metadata is static to avoid relying on a `/models` response that does not provide
  Codex-specific capabilities.

