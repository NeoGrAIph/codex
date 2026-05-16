# DeepSeek Chat Completions Project

## Status

Implemented for Codex fork release baseline `0.130.0`.

## Canonical Links

- Feature contract: `docs/fork/features/deepseek-chat-completions.md`
- Design: `docs/fork/projects/deepseek-chat-completions/design.md`
- Verification: `docs/fork/projects/deepseek-chat-completions/verification.md`
- Research: `docs/fork/research/0.130.0/deepseek-chat-completions.md`

## Implementation Map

- Provider/config source of truth: `codex-rs/model-provider-info`
- Runtime provider capabilities and static model catalog: `codex-rs/model-provider`
- Chat Completions request/stream transport: `codex-rs/codex-api`
- Runtime routing: `codex-rs/core/src/client.rs`
- Tool schema conversion: `codex-rs/tools`

