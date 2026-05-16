# DeepSeek Chat Completions Verification

## Focused Commands

```bash
cd codex-rs
cargo test -p codex-api
cargo test -p codex-tools
cargo test -p codex-model-provider-info
cargo test -p codex-model-provider
cargo test -p codex-config
cargo test -p codex-core
just write-config-schema
just fmt
just fix -p codex-api
just fix -p codex-tools
just fix -p codex-model-provider-info
just fix -p codex-model-provider
just fix -p codex-config
just fix -p codex-core
```

## Scenarios

- DeepSeek provider parses from built-in provider catalog.
- `wire_api = "chat_completions"` parses and serializes in config and remote thread config.
- Chat Completions request history includes assistant `reasoning_content` before tool calls.
- Chat Completions SSE parser reconstructs reasoning, tool call arguments, usage, and turn
  completion.
- Unsupported image input fails before a DeepSeek HTTP request is sent.
- Existing Responses providers still compile and use the old transport.

## Latest Local Evidence

- `cargo test -q -p codex-api`: passed.
- `cargo test -q -p codex-tools`: passed.
- `cargo test -q -p codex-model-provider-info -p codex-model-provider`: passed.
- `cargo test -q -p codex-config`: passed.
- `just write-config-schema`: passed.
- `just fmt`: passed.
- `just fix -p codex-api`: passed.
- `just fix -p codex-tools`: passed.
- `just fix -p codex-model-provider-info`: passed.
- `just fix -p codex-model-provider`: passed.
- `just fix -p codex-config`: passed.
- `just fix -p codex-core`: passed.
- `cargo test -q -p codex-core`: failed in the current workspace on existing environment/fixture
  mismatches, including `/tmp` git-context assumptions, config fixture drift, and tool-spec
  expectation drift. The failure is not a live DeepSeek transport check.

## Known Gaps

- No live DeepSeek integration test is run locally because it requires a real `DEEPSEEK_API_KEY`.
- No generic Chat Completions provider catalog is exposed yet; `deepseek` is the first built-in
  provider using the transport.
