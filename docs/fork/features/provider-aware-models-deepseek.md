# Provider-aware models and DeepSeek

## Feature passport

- Code name: `provider-aware-models-deepseek`
- Status: реализовано в исторической fork lineage, не перенесено коммитом на текущую `fork/140`.
- Goal: сделать модели provider-aware и добавить DeepSeek через Chat Completions-compatible provider.
- Scope in: provider catalog, config/schema, thread config, model listing, runtime switching, Chat Completions SSE/client.
- Scope out: любые hardcoded per-client model lists без native catalog propagation.

## Как работает для пользователя

Пользователь или agent выбирает не только model, но и корректный provider. Польза: модели разных providers, включая DeepSeek через Chat Completions, могут отображаться и использоваться без путаницы transport/API.

## Branches and commits

| Branch | Baseline | Commit | Date | Subject | Роль |
| --- | --- | --- | --- | --- | --- |
| `fork/130` | `rust-v0.130.0` | `c9ff8c1e40` | 2026-05-16 | `feat(deepseek): add chat completions provider` | DeepSeek provider, Chat Completions transport, config/schema/thread config/tool docs. |
| `chrome_plugin` | `rust-v0.130.0` lineage | `c9ff8c1e40` | 2026-05-16 | `feat(deepseek): add chat completions provider` | Та же provider-aware возможность в chrome-plugin lineage. |

## Implementation notes

Commit затрагивал `codex-api`, `model-provider`, `model-provider-info`, config schema/thread config proto, core client, tool specs/tests и fork feature/project/research docs.

## Native coverage in rust-v0.140.0

Status: `partial`. Native release поддерживает configured `model_providers`, `ModelProviderInfo`, `merge_configured_model_providers`, `Config.model_provider_id`, thread/session provider state, `ThreadStartParams.model_provider`, `ThreadSettings.model_provider`, thread list filtering by providers and OpenAI-style `/models` helper. Не хватает built-in DeepSeek, Chat Completions-compatible `WireApi`, provider-aware `model/list` with `includeConfiguredProviders`/provider field, and atomic `(model_provider, model)` switch via `thread/settings/update`.

## Porting/current-state notes

Memory/evidence по предыдущей работе указывает, что риск находился в stale app-server `model/list` path и atomic `(model_provider, model)` runtime switch. При переносе нужно проверять свежий `model/list`, `thread/settings/update`, `thread/read`, `thread/resume` и next-turn transport.
