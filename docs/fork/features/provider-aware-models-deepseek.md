# Provider-aware models and DeepSeek

## Feature passport

- Code name: `provider-aware-models-deepseek`
- Status: первая итерация переноса реализована в `fork/140` commit `65bf81b8cd` (`feat(models): add provider-aware DeepSeek support`); provider-management итерация `/model-providers` добавлена как следующий `fork/140` slice.
- Goal: сделать выбор моделей provider-aware и добавить DeepSeek как Chat Completions-compatible provider без нарушения upstream OpenAI behavior.
- Scope in: provider catalog, config/schema, app-server `model/list`, app-server `thread/settings/update`, app-server `modelProvider/*`, TUI `/model`, TUI `/model-providers`, runtime transport selection, managed provider API keys, Chat Completions SSE/client, feature/project docs.
- Scope out: parallel config, duplicated provider/model enums, one-off UI switches, hardcoded provider/model lists вне native source of truth, миграции БД, изменение sandbox/permissions/MCP semantics.

## User contract

Пользователь выбирает связку `(model_provider, model)`, а не только `model`. В TUI `/model` список может включать модели configured providers, включая DeepSeek, и выбор DeepSeek отправляет provider и model в app-server одним settings update. Если несколько providers возвращают одинаковый model slug, all-models picker показывает provider id рядом с повторяющимся slug, чтобы пользователь видел разные provider entries, а не визуальные дубли. При сохранении выбора в user config записываются `model_provider`, `model` и, если выбрано, `model_reasoning_effort`.

Пользователь управляет дополнительными источниками моделей через TUI slash command `/model-providers` с aliases `/providers` и `/model_provider`. OpenAI считается native provider: он включён по умолчанию, не показывается в управляемом списке, не может быть отключён через `modelProvider/config/write`, а его auth управляется native account login, не `modelProvider/auth/write/remove`. В списке управляемых providers действие enable/disable сделано как в `/plugins`: Space переключает видимость provider в `/model`, Enter открывает detail-уровень provider. На detail-уровне можно сделать provider default in user config, сохранить managed API key или удалить managed API key; техническая информация provider показывается как контекст выбранного действия. `Set as default provider` сохраняет provider вместе с совместимой default model из native model manager, чтобы не получить пару вроде `model_provider = "deepseek"` плюс OpenAI model slug. Active/default provider нельзя скрыть через этот flow, включая combined request `setActive=true` and `enabledInPicker=false`; если пользователь вручную записал конфликтующий config, active provider всё равно остаётся видимым в picker, чтобы не заблокировать текущий выбор.

API keys не пишутся в plaintext config. `modelProvider/auth/write` сохраняет ключ в `codex-secrets` как local encrypted secret, scoped globally, с именем provider env key (`DEEPSEEK_API_KEY`) или derived `MODEL_PROVIDER_<ID>_API_KEY` для providers без env key. Config loader материализует managed key в provider auth только если env key отсутствует; если env key задан, он остаётся приоритетным native source. TUI sensitive prompt маскирует ввод, `SaveModelProviderApiKey` не пишется в session event log, and debug formatting for the TUI event/protocol params redacts the secret.

DeepSeek появляется как built-in provider `deepseek` с base URL `https://api.deepseek.com`, auth env key `DEEPSEEK_API_KEY` и wire API `chat_completions`. Первая итерация поставляет static catalog `deepseek-v4-pro` и `deepseek-v4-flash`; обе модели видимы в picker, используют text-only input, shell command tool, function/namespace-function tools и reasoning efforts low/medium/high/xhigh. Freeform `apply_patch`, hosted web search и hosted image generation для DeepSeek намеренно не advertised, пока Chat Completions adapter не поддерживает совместимое представление.

Default behavior сохранён: обычный app-server `model/list` без `includeConfiguredProviders` продолжает возвращать каталог активного thread/provider через прежний `ThreadManager` path. Provider-aware каталог включается только явным `includeConfiguredProviders: true`; TUI bootstrap использует этот флаг, чтобы `/model` видел не только active provider. `disabled_model_providers` влияет только на provider-aware picker/catalog projection и запрещён в project-local config, чтобы проект не мог скрыть user-level providers.

Ошибки не скрываются silent fallback. Неизвестный `model_provider` в `thread/settings/update` отклоняется controlled `ConstraintError::InvalidValue`. Chat Completions transport fail-fast отклоняет unsupported image input, unsupported Responses-only tools и image/encrypted content внутри `function_call_output` вместо неявного downgrade или lossy drop.

## UX reference

OpenClaude `v0.19.0` используется как ориентир качества для дальнейшего развития provider-management UX, см. `~/repo/AGENTS/openclaude/docs/ui-snapshots/provider-settings-onboarding.snap`. Полезные для нашего `/model-providers` принципы: provider setup должен быть пошаговым, secrets не должны показываться после ввода, persistent profile/config writes должны быть явными и scoped, settings panels должны оставаться плотными и рабочими, а unsupported usage/capability surfaces должны показывать честный fallback row вместо пустого или вводящего в заблуждение состояния.

Текущая итерация не переносит OpenClaude profile wizard буквально. Для `fork/140` сохраняется native Codex path: provider state берётся из `modelProvider/list`, enable/disable делается row toggle через `SelectionToggle`, Enter открывает detail-level actions, API keys пишутся через `codex-secrets`, а `/model` наследует состояние через provider-aware `model/list`.

## Branches and commits

| Branch | Baseline | Commit/state | Date | Subject | Роль |
| --- | --- | --- | --- | --- | --- |
| `fork/140` | `rust-v0.140.0` | `65bf81b8cd` | 2026-06-17 | `feat(models): add provider-aware DeepSeek support` | Первая итерация переноса: source of truth, protocol/schema, app-server, TUI, Chat Completions transport и docs. |
| `feature/140/provider-model-catalog` | `fork/140` / `rust-v0.140.0` lineage | `65bf81b8cd` | 2026-06-17 | `feat(models): add provider-aware DeepSeek support` | Исходная feature branch первой итерации, влита в `fork/140`. |
| `fork/130` | `rust-v0.130.0` | `c9ff8c1e40` | 2026-05-16 | `feat(deepseek): add chat completions provider` | Historical reference: DeepSeek provider, Chat Completions transport, config/schema/thread config/tool docs. |
| `chrome_plugin` | `rust-v0.130.0` lineage | `c9ff8c1e40` | 2026-05-16 | `feat(deepseek): add chat completions provider` | Та же provider-aware возможность в chrome-plugin lineage. |

## Integration and compatibility notes

Native source of truth остаётся в `ModelProviderInfo`/`WireApi`, configured `Config.model_providers`, `Config.model_provider_id`, `ModelPreset`, app-server protocol model schema и runtime `SessionConfiguration`. DeepSeek добавлен как built-in provider в `built_in_model_providers`, а его model catalog подключён через `create_model_provider(...).models_manager(...)`, а не через TUI/app-server hardcoded list.

`WireApi::ChatCompletions`, сериализуемый как `chat_completions`, uses Chat Completions transport with provider-specific request options. DeepSeek получает `thinking` и mapping reasoning effort только через DeepSeek option set; generic Chat Completions option set не отправляет эти DeepSeek-only поля. Это всё ещё не означает, что любой provider можно безопасно включить одним `wire_api`: для нового provider нужны отдельные model catalog, auth, capability and request-option contract. Legacy `wire_api = "chat"` по-прежнему rejected с существующей diagnostic, чтобы не вернуть удалённый upstream path. Responses API для OpenAI и других existing providers остаётся default.

Runtime switching использует native `thread/settings/update`: protocol получил optional `modelProvider`, core `ThreadSettingsOverrides` получил `model_provider`, а session apply меняет provider/model atomically in runtime config. Old sessions and persisted data remain readable because new app-server/model fields default to `openai` where they deserialize from older payloads, and missing persisted provider continues to follow existing fallback config behavior.

Provider management использует native app-server protocol и config writer: `modelProvider/list` читает `Config.model_providers`, `Config.model_provider_id`, `Config.disabled_model_providers` и provider-owned model managers, но исключает native OpenAI из управляемой projection; `modelProvider/config/write` записывает `model_provider`, compatible `model` and `disabled_model_providers` через `ConfigBatchWriteParams`, при этом прямое отключение `openai`, управление OpenAI auth and combined active+hidden requests отклоняются controlled invalid request; `modelProvider/auth/write`/`modelProvider/auth/remove` работают через `codex-secrets`, а не через TOML. После mutation TUI обновляет provider list и provider-aware model catalog через app-server, чтобы `/model` наследовал изменения без отдельного hardcoded state.

Startup prewarm is provider-scoped. Если пользователь меняет provider через `thread/settings/update` до первого regular turn, startup websocket prewarm, подготовленный для стартового OpenAI provider, снимается и не может быть использован DeepSeek turn. `run_turn` дополнительно discards any prewarmed client session whose provider does not match the turn context.

## Propagation matrix

| Surface | Source of truth / producer | Consumer / projection | Изменение |
| --- | --- | --- | --- |
| Provider catalog | `model-provider-info::built_in_model_providers`, `WireApi` | config loader/schema, provider factory | Добавлены `deepseek` и `chat_completions`; schema regenerated. |
| Model catalog | `model-provider::ConfiguredModelProvider::models_manager` | app-server `model/list`, TUI `/model` | DeepSeek получает static catalog без freeform apply_patch; `ModelPreset.model_provider` помечает provider. |
| Provider capabilities | `ModelProvider::capabilities` | tool planning | DeepSeek отключает hosted web search и image generation для Chat Completions request compatibility. |
| Provider enablement | `ConfigToml.disabled_model_providers`, `Config.disabled_model_providers` | app-server `modelProvider/list`, provider-aware `model/list`, TUI `/model` and `/model-providers` | Inactive disabled providers скрываются из provider-aware picker; active provider остаётся видимым; active+hidden combined writes fail; native OpenAI исключён из управляемого списка, не может быть отключён и остаётся доступным даже при stale `disabled_model_providers = ["openai"]`; project-local config не может задавать этот список. |
| Provider auth | env vars, `ModelProviderInfo.env_key`, `codex-secrets` global secret | config loader, provider auth, app-server `modelProvider/list`, TUI `/model-providers` | Env key остаётся приоритетным runtime source; managed key используется без plaintext TOML, когда env key отсутствует; auth status проецируется в UI/API; OpenAI auth is native account login and rejected by `modelProvider/auth/write/remove`; managed key write/remove refreshes loaded thread runtime config through the native app-server config path; Debug output redacts submitted API keys. |
| Remote model cache | `OpenAiModelsManager` cache path | active/inactive remote provider catalogs | OpenAI сохраняет legacy `models_cache.json`; non-OpenAI remote providers используют scoped `models_cache/<provider-scope>.json`, чтобы inactive provider не читал active provider cache. |
| App-server catalog | `ModelListParams.includeConfiguredProviders`, `Model.modelProvider` | clients and TUI bootstrap | Default list unchanged; provider-aware list returns models tagged by provider. |
| App-server provider management | `modelProvider/list`, `modelProvider/config/write`, `modelProvider/auth/write`, `modelProvider/auth/remove` | TUI `/model-providers`, experimental app-server clients | Управление providers идёт через typed Rust protocol and native config/secrets managers. The default non-experimental schema fixture writer filters experimental `modelProvider/*` request methods and their experimental-only dependency DTOs; Rust wire serialization is covered by protocol tests. |
| Runtime settings | `ThreadSettingsUpdateParams.modelProvider`, `ThreadSettingsOverrides.model_provider` | core session config, next turn transport | Atomic provider+model switch; unknown provider fails fast; app-server rejects `modelProvider` without `model` to avoid incompatible provider/model pairs. |
| TUI picker | app-server `model/list` response | `/model` actions, config persistence | Selection sends and persists provider+model+effort; duplicate model slugs from different providers are disambiguated by provider id in the picker. |
| TUI provider manager | app-server `modelProvider/list` response | `/model-providers` row toggle, detail-level actions, sensitive prompt, model catalog refresh | UI показывает управляемые non-OpenAI provider state; Space переключает picker visibility через `SelectionToggle`, Enter открывает detail-уровень для default/API-key actions, после mutation `/model` catalog обновляется через app-server. |
| Transport | `ModelProviderInfo.wire_api` | `ModelClientSession::stream` | `Responses` path unchanged; DeepSeek-compatible `ChatCompletions` uses `/chat/completions` SSE adapter. |
| Tools | native `ToolSpec` | request builder for Chat Completions | Function/namespace function tools are converted; unsupported tools fail before request. |
| History projection | `ResponseItem` request history | Chat Completions request builder | Message/reasoning/function-call history is converted; unsupported persisted/history item types and unsupported structured `function_call_output` content fail with controlled `InvalidRequest` instead of being dropped. |
| Startup prewarm | session startup websocket prewarm | first regular turn | Provider-changing settings updates abort startup prewarm; turn execution rejects provider-mismatched prewarm sessions. |
| Persistence/resume | existing thread config snapshot/session metadata | thread read/resume/start paths | No new storage; provider id travels through existing config snapshot and settings update paths. |

## Intentionally unaffected surfaces

Permissions, sandbox policies, MCP tool registry, rollout database schema, external-agent migration, multi-agent lineage, app-server direct thread lifecycle semantics and OpenAI Responses transport behavior are intentionally unchanged in this iteration. `/model-providers` меняет default provider in user config, но не переключает текущий running thread model; current-thread provider/model still changes через `/model` and `thread/settings/update`.

## Compatibility notes

`/model-providers` requires a TUI and embedded app-server from the same fork build because it calls experimental `modelProvider/*` methods. If an older app-server responds with `unknown variant modelProvider/list` or method-not-found, the TUI must report a controlled restart diagnostic instead of silently falling back to a partial provider manager. This is a mixed-runtime compatibility boundary, not a provider/config fallback.

## Backlog

### 2026-06-17 - DeepSeek rejects `developer` role in Chat Completions payload

При выбранной модели `deepseek/deepseek-v4-flash` с reasoning `high` следующий обычный turn может завершиться ошибкой provider/API validation: `messages[1].role: unknown variant developer`, expected `system`, `user`, `assistant`, `tool`, `latest_reminder`. Перед ошибкой может появляться предупреждение о сокращении skill descriptions до 2% skills context budget, но оно пока не считается root cause без проверки request body.

Причина подтверждена локально: Chat Completions adapter переносил `ResponseItem::Message { role: "developer" }` в `messages[]` без provider-compatible role normalization, тогда как DeepSeek-compatible endpoint не принимает роль `developer`. Fix сделан на transport boundary в `codex-rs/codex-api/src/common.rs`: `developer` context messages проецируются как `system` messages для Chat Completions, supported roles сохраняются, unknown message roles fail fast через `InvalidRequest`. TUI/model picker workaround не добавлялся; OpenAI Responses path не менялся.

## Verification matrix

| Check | Surface |
| --- | --- |
| `just write-config-schema` | config/schema accepts `chat_completions`, built-in provider metadata and `disabled_model_providers`. |
| `just write-app-server-schema` | default non-experimental protocol/TypeScript/JSON schema includes stable provider fields such as `includeConfiguredProviders`, `modelProvider` and settings update provider field; experimental `modelProvider/*` request methods and experimental-only dependency DTOs are filtered from default standalone fixtures. |
| `just test -p codex-model-provider-info` | provider info deserialization and DeepSeek built-in provider contract. |
| `just test -p codex-model-provider` | provider factory uses DeepSeek static catalog through native manager. |
| `just test -p codex-api` | Chat Completions request/SSE conversion. |
| `just test -p codex-tools` | Chat Completions tool JSON conversion. |
| `just test -p codex-app-server-protocol` | generated schema fixtures and protocol serialization. |
| `just test -p codex-app-server list_models` | app-server `model/list` provider projection, configured-provider catalog and negative cache isolation. |
| `just test -p codex-app-server disabled_provider_is_hidden_from_provider_aware_model_list openai_provider_is_native_and_not_manageable stale_disabled_openai_config_does_not_hide_openai_models model_provider_auth_write_and_remove_updates_managed_key_status model_provider_env_key_takes_precedence_over_managed_key_status model_provider_set_active_writes_compatible_default_model model_provider_cannot_be_set_active_and_hidden` | provider enablement, native OpenAI non-manageability/auth guard, managed auth key propagation, env-key precedence, default provider/model atomicity and active+hidden rejection through app-server. |
| `just test -p codex-core provider_and_model_change_uses_chat_completions_next_turn` | runtime settings update proves the next turn posts DeepSeek provider/model to `/chat/completions`. |
| `just test -p codex-core provider_change_before_first_turn_discards_startup_prewarm` | startup OpenAI websocket prewarm is not reused after switching to DeepSeek before the first turn. |
| `just test -p codex-api chat_completions` | Chat Completions adapter maps `developer` messages to `system`, rejects unknown message roles and rejects unsupported history/items instead of silently dropping data. |
| Live DeepSeek smoke | Optional credentialed smoke with `DEEPSEEK_API_KEY`; on 2026-06-18 a minimal `deepseek-v4-flash` request returned HTTP 200 with response content redacted. |
| `just test -p codex-tui model_providers_popup_snapshot model_provider_detail_popup_snapshot model_provider_api_key_prompt_masks_input_snapshot model_provider_list_toggles_with_space_and_opens_details_with_enter model_provider_detail_actions_emit_expected_events model_provider_api_key_event_debug_is_redacted model_provider_aliases_open_model_providers model_provider_request_compat_detects_unsupported_errors` | `/model` picker actions, `/model-providers` row toggle/detail UX, slash aliases, detail action events, sensitive API-key prompt masking, secret-safe debug formatting, mixed-runtime restart diagnostic, persisted config edit and app-server settings update projection. |
| `just test -p codex-tui all_models_popup_disambiguates_duplicate_model_slugs_by_provider` | `/model` all-models picker keeps provider-specific entries selectable while disambiguating identical model slugs by provider id. |
| `just fmt`; `git diff --check` | formatting and whitespace hygiene. |

## Doc changelog

- 2026-06-17: первая итерация перенесена в `fork/140` commit `65bf81b8cd`, включая user contract, native propagation matrix, compatibility notes и verification matrix.
- 2026-06-17: добавлена текущая provider-management итерация `/model-providers`: enable/disable providers in picker, default provider write, managed API keys через `codex-secrets`, app-server `modelProvider/*` protocol и TUI sensitive prompt.
- 2026-06-17: уточнён native OpenAI contract: OpenAI не показывается в `/model-providers`, не отключается через `modelProvider/config/write`, stale disabled OpenAI не скрывает OpenAI models; enable/disable UI переведён на `/plugins`-style Space toggle с detail-уровнем по Enter.
- 2026-06-17: зафиксирован OpenClaude `ProviderManager` snapshot как UX-reference для будущих provider-management улучшений без буквального переноса profile wizard в текущую итерацию.
- 2026-06-17: добавлен backlog item по live-сбою DeepSeek `chat_completions` при попадании `developer` role в serialized messages.
- 2026-06-18: DeepSeek `developer` role backlog закрыт transport-boundary fix в Chat Completions adapter; добавлены regression tests и проверка `just test -p codex-api chat_completions`.
- 2026-06-18: provider-management guard hardening: OpenAI auth write/remove rejected, active+hidden provider write rejected, `setActive` writes compatible default model, `modelProvider/*` protocol serialization tests added, and TUI/protocol API-key debug output redacts secrets.
- 2026-06-18: добавлено dedicated TUI snapshot coverage для sensitive API-key prompt: введённый ключ маскируется и не появляется в rendered popup.
- 2026-06-18: `/model` all-models picker теперь disambiguates одинаковые model slugs по provider id; focused TUI test подтверждает рендер и сохранение provider context при выборе.
- 2026-06-18: schema/export contract уточнён после provider audit: default fixtures filter experimental provider-management request methods and experimental-only dependency DTOs from stable JSON/TypeScript bundles.
- 2026-06-18: добавлена mixed-runtime compatibility note для `/model-providers`: старый app-server без `modelProvider/*` должен давать controlled restart diagnostic, а не silent fallback или частичный provider manager.
- 2026-06-18: provider/model v2 добавил live DeepSeek smoke evidence and explicit provider-specific Chat Completions options: DeepSeek-only request fields are no longer unconditional for generic Chat Completions options.
- 2026-06-18: upstream-diff audit hardening added runtime guards: managed provider auth write/remove refreshes loaded thread runtime config, and `thread/settings/update` rejects provider-only updates without a matching `model`.
