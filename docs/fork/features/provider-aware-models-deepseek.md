# Provider-aware models and DeepSeek

## Feature passport

- Code name: `provider-aware-models-deepseek`
- Status: draft-перенос на `feature/140/provider-model-catalog`; реализация находится в dirty worktree до финальной проверки и commit.
- Goal: сделать выбор моделей provider-aware и добавить DeepSeek как Chat Completions-compatible provider без нарушения upstream OpenAI behavior.
- Scope in: provider catalog, config/schema, app-server `model/list`, app-server `thread/settings/update`, TUI `/model`, runtime transport selection, Chat Completions SSE/client, feature/project docs.
- Scope out: parallel config, duplicated provider/model enums, one-off UI switches, hardcoded provider/model lists вне native source of truth, миграции БД, изменение sandbox/permissions/MCP semantics.

## User contract

Пользователь выбирает связку `(model_provider, model)`, а не только `model`. В TUI `/model` список может включать модели configured providers, включая DeepSeek, и выбор DeepSeek отправляет provider и model в app-server одним settings update. При сохранении выбора в user config записываются `model_provider`, `model` и, если выбрано, `model_reasoning_effort`.

DeepSeek появляется как built-in provider `deepseek` с base URL `https://api.deepseek.com`, auth env key `DEEPSEEK_API_KEY` и wire API `chat_completions`. Первая итерация поставляет static catalog `deepseek-v4-pro` и `deepseek-v4-flash`; обе модели видимы в picker, используют text-only input, shell command tool, function/namespace-function tools и reasoning efforts low/medium/high/xhigh. Freeform `apply_patch`, hosted web search и hosted image generation для DeepSeek намеренно не advertised, пока Chat Completions adapter не поддерживает совместимое представление.

Default behavior сохранён: обычный app-server `model/list` без `includeConfiguredProviders` продолжает возвращать каталог активного thread/provider через прежний `ThreadManager` path. Provider-aware каталог включается только явным `includeConfiguredProviders: true`; TUI bootstrap использует этот флаг, чтобы `/model` видел не только active provider.

Ошибки не скрываются silent fallback. Неизвестный `model_provider` в `thread/settings/update` отклоняется controlled `ConstraintError::InvalidValue`. Chat Completions transport fail-fast отклоняет unsupported image input, unsupported Responses-only tools и image/encrypted content внутри `function_call_output` вместо неявного downgrade или lossy drop.

## Branches and commits

| Branch | Baseline | Commit/state | Date | Subject | Роль |
| --- | --- | --- | --- | --- | --- |
| `feature/140/provider-model-catalog` | `fork/140` / `rust-v0.140.0` lineage | `draft/uncommitted` | 2026-06-17 | Provider-aware models and DeepSeek first iteration | Текущий перенос: source of truth, protocol/schema, app-server, TUI, Chat Completions transport и docs. |
| `fork/130` | `rust-v0.130.0` | `c9ff8c1e40` | 2026-05-16 | `feat(deepseek): add chat completions provider` | Historical reference: DeepSeek provider, Chat Completions transport, config/schema/thread config/tool docs. |
| `chrome_plugin` | `rust-v0.130.0` lineage | `c9ff8c1e40` | 2026-05-16 | `feat(deepseek): add chat completions provider` | Та же provider-aware возможность в chrome-plugin lineage. |

## Integration and compatibility notes

Native source of truth остаётся в `ModelProviderInfo`/`WireApi`, configured `Config.model_providers`, `Config.model_provider_id`, `ModelPreset`, app-server protocol model schema и runtime `SessionConfiguration`. DeepSeek добавлен как built-in provider в `built_in_model_providers`, а его model catalog подключён через `create_model_provider(...).models_manager(...)`, а не через TUI/app-server hardcoded list.

`WireApi::ChatCompletions`, сериализуемый как `chat_completions`, в этой итерации означает DeepSeek-compatible Chat Completions transport: request body включает DeepSeek-specific `thinking` и mapping reasoning effort. Это не объявляется generic provider-neutral Chat Completions контрактом; для других providers перед расширением нужны provider-specific request options. Legacy `wire_api = "chat"` по-прежнему rejected с существующей diagnostic, чтобы не вернуть удалённый upstream path. Responses API для OpenAI и других existing providers остаётся default.

Runtime switching использует native `thread/settings/update`: protocol получил optional `modelProvider`, core `ThreadSettingsOverrides` получил `model_provider`, а session apply меняет provider/model atomically in runtime config. Old sessions and persisted data remain readable because new app-server/model fields default to `openai` where they deserialize from older payloads, and missing persisted provider continues to follow existing fallback config behavior.

Startup prewarm is provider-scoped. Если пользователь меняет provider через `thread/settings/update` до первого regular turn, startup websocket prewarm, подготовленный для стартового OpenAI provider, снимается и не может быть использован DeepSeek turn. `run_turn` дополнительно discards any prewarmed client session whose provider does not match the turn context.

## Propagation matrix

| Surface | Source of truth / producer | Consumer / projection | Изменение |
| --- | --- | --- | --- |
| Provider catalog | `model-provider-info::built_in_model_providers`, `WireApi` | config loader/schema, provider factory | Добавлены `deepseek` и `chat_completions`; schema regenerated. |
| Model catalog | `model-provider::ConfiguredModelProvider::models_manager` | app-server `model/list`, TUI `/model` | DeepSeek получает static catalog без freeform apply_patch; `ModelPreset.model_provider` помечает provider. |
| Provider capabilities | `ModelProvider::capabilities` | tool planning | DeepSeek отключает hosted web search и image generation для Chat Completions request compatibility. |
| Remote model cache | `OpenAiModelsManager` cache path | active/inactive remote provider catalogs | OpenAI сохраняет legacy `models_cache.json`; non-OpenAI remote providers используют scoped `models_cache/<provider-scope>.json`, чтобы inactive provider не читал active provider cache. |
| App-server catalog | `ModelListParams.includeConfiguredProviders`, `Model.modelProvider` | clients and TUI bootstrap | Default list unchanged; provider-aware list returns models tagged by provider. |
| Runtime settings | `ThreadSettingsUpdateParams.modelProvider`, `ThreadSettingsOverrides.model_provider` | core session config, next turn transport | Atomic provider+model switch; unknown provider fails fast. |
| TUI picker | app-server `model/list` response | `/model` actions, config persistence | Selection sends and persists provider+model+effort. |
| Transport | `ModelProviderInfo.wire_api` | `ModelClientSession::stream` | `Responses` path unchanged; DeepSeek-compatible `ChatCompletions` uses `/chat/completions` SSE adapter. |
| Tools | native `ToolSpec` | request builder for Chat Completions | Function/namespace function tools are converted; unsupported tools fail before request. |
| History projection | `ResponseItem` request history | Chat Completions request builder | Message/reasoning/function-call history is converted; unsupported persisted/history item types and unsupported structured `function_call_output` content fail with controlled `InvalidRequest` instead of being dropped. |
| Startup prewarm | session startup websocket prewarm | first regular turn | Provider-changing settings updates abort startup prewarm; turn execution rejects provider-mismatched prewarm sessions. |
| Persistence/resume | existing thread config snapshot/session metadata | thread read/resume/start paths | No new storage; provider id travels through existing config snapshot and settings update paths. |

## Intentionally unaffected surfaces

Permissions, sandbox policies, MCP tool registry, rollout database schema, external-agent migration, multi-agent lineage, app-server direct thread lifecycle semantics and OpenAI Responses transport behavior are intentionally unchanged in this iteration.

## Verification matrix

| Check | Surface |
| --- | --- |
| `just write-config-schema` | config/schema accepts `chat_completions` and built-in provider metadata. |
| `just write-app-server-schema` | protocol/TypeScript/JSON schema includes `includeConfiguredProviders`, `modelProvider`, settings update provider field. |
| `just test -p codex-model-provider-info` | provider info deserialization and DeepSeek built-in provider contract. |
| `just test -p codex-model-provider` | provider factory uses DeepSeek static catalog through native manager. |
| `just test -p codex-api` | Chat Completions request/SSE conversion. |
| `just test -p codex-tools` | Chat Completions tool JSON conversion. |
| `just test -p codex-app-server-protocol` | generated schema fixtures and protocol serialization. |
| `just test -p codex-app-server list_models` | app-server `model/list` provider projection, configured-provider catalog and negative cache isolation. |
| `just test -p codex-core provider_and_model_change_uses_chat_completions_next_turn` | runtime settings update proves the next turn posts DeepSeek provider/model to `/chat/completions`. |
| `just test -p codex-core provider_change_before_first_turn_discards_startup_prewarm` | startup OpenAI websocket prewarm is not reused after switching to DeepSeek before the first turn. |
| `just test -p codex-api chat_completions_rejects` | unsupported history items and unsupported structured `function_call_output` content fail request construction instead of being silently dropped. |
| `just test -p codex-tui` | `/model` picker actions, persisted config edit and app-server settings update projection. |
| `just fmt`; `git diff --check` | formatting and whitespace hygiene. |

## Doc changelog

- 2026-06-17: зафиксирован draft-перенос первой итерации на `feature/140/provider-model-catalog`, включая user contract, native propagation matrix, compatibility notes и verification matrix.
