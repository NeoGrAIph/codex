# DeepSeek Chat Completions Research

## Baseline

- Upstream release baseline: `0.130.0`
- Local branch context: `fork/130`
- Relevant upstream architecture: `ModelProviderInfo`, `ModelProvider`, `codex-api` Responses
  transport, static model catalogs, and `ResponseItem` transcript history.

## Gap Analysis

- Upstream Codex supports user-defined model providers, but the accepted wire API is Responses-only.
- DeepSeek's direct API is OpenAI-compatible Chat Completions, not OpenAI Responses.
- DeepSeek thinking/tool-call sessions require `reasoning_content` to be passed back in assistant
  history after tool calls.
- Existing `ResponseItem::Reasoning` can carry this provider-specific reasoning state, so no new
  database or rollout migration is required.

## Risky Integration Points

- `WireApi` parsing and remote thread config persistence.
- `codex-api` request/stream abstractions, because Responses stream events are the current default.
- Tool schema conversion, because Responses tools and Chat Completions tools use different JSON
  shapes.
- Provider capabilities, because DeepSeek should not see namespace tools, hosted web search, image
  generation, or image input.

## Release-Specific Verification Notes

- `chat_completions` must be additive and not reinterpret legacy `wire_api = "chat"`.
- The built-in `deepseek` provider must not require OpenAI login or ChatGPT credentials.
- The implementation should not add external provider SDKs or dynamic plugin loading.

