# Архитектурная концепция внешних MCP servers

Статус документа: нормализованная карта нативных source-of-truth и extension points для Codex как MCP client/runtime на stable `rust-v0.144.1`. Историческая хронология покрыта до `rust-v0.144.1` в `timeline.md`; проверочные команды и доказательства по коммитам приведены в `evidence.md`.

## Термины

- External MCP server: сервер, который Codex запускает или вызывает как клиент MCP. Источник может быть пользовательский `[mcp_servers]`, plugin declaration, selected executor plugin, extension contribution, built-in/compatibility registration, remote/local marketplace plugin or host-owned Apps MCP.
- `codex-rs/mcp-server`: отдельный контур, где Codex сам выступает MCP server для внешних clients. Этот документ не исследует его как основной runtime и упоминает только для разграничения терминов.
- Stdio MCP: локальный или environment-owned process transport с `command`, `args`, `env`, `env_vars`, optional `cwd`.
- Streamable HTTP MCP: remote HTTP transport with `url`, optional bearer token env var, static/env HTTP headers and OAuth login state.
- Apps MCP / `codex_apps`: host-owned MCP surface for ChatGPT Apps/connectors. It shares runtime and tool exposure machinery but has connector policy and auth rules.
- Selected plugin MCP: MCP declaration discovered from a selected executor plugin/capability root and activated only for the owning thread.
- MCP catalog: resolved source-of-truth for MCP server registrations after config, plugin, selected plugin, compatibility and extension contributions have been merged by precedence.

## Текущая карта source-of-truth

- Контракт конфигурации: `codex-rs/config/src/mcp_types.rs` владеет `McpServerConfig`, `RawMcpServerConfig`, `McpServerTransportConfig`, per-tool config, `McpServerAuth::{OAuth, ChatGpt}`, OAuth config, `AppToolApproval::{Auto, Prompt, Writes, Approve}`, полями `enabled`, `required`, `supports_parallel_tool_calls`, таймаутами запуска/вызова, списками разрешённых/запрещённых tools и представлением stdio `cwd` как `LegacyAppPathString`.
- Загрузка конфигурации и requirements: `codex-rs/core/src/config/mod.rs` владеет runtime-значением `Config.mcp_servers`, фильтрацией requirements, plugin MCP requirements, построением каталога и диагностикой неподдерживаемого plaintext `bearer_token`. `codex-rs/config/src/mcp_requirements.rs` владеет managed matchers для exact/command/URL, включая упорядоченное сопоставление списка аргументов точной длины и проверку regex по полному значению. `codex-rs/config/src/mcp_edit.rs` и `codex-rs/cli/src/mcp_cmd.rs` владеют командами редактирования конфигурации.
- Registration/catalog: `codex-rs/codex-mcp/src/catalog.rs` owns `McpServerRegistration`, `McpServerSource`, precedence and name-veto behavior; `codex-rs/codex-mcp/src/plugin_config.rs` parses plugin MCP declarations; `codex-rs/core-plugins/src/manifest.rs`, `loader.rs`, `marketplace.rs` and `store.rs` own plugin manifest formats, manifest path lists, marketplace fallback and root local marketplace handling before MCP declarations enter the catalog; `codex-rs/ext/extension-api/src/contributors/mcp.rs` defines extension-owned add/replace/remove contributions; `codex-rs/ext/mcp/src/executor_plugin.rs` discovers selected executor plugin MCP declarations.
- Владение runtime/client: `codex-rs/codex-mcp/src/mcp/mod.rs` владеет `McpConfig`, сборкой effective servers, снимками auth status, чтением resources и Apps MCP helpers; `codex-rs/codex-mcp/src/connection_manager.rs` владеет запущенными clients, startup status, агрегацией tools/resources/templates, маршрутизацией вызовов и управлением elicitation requests; `codex-rs/codex-mcp/src/runtime.rs` владеет разрешением environment и sandbox state, причём sandbox metadata ограничена environment конкретного сервера. `codex-rs/core/src/session/mcp_runtime.rs` владеет `McpRuntimeSnapshot`: точными MCP config, connection manager, runtime context и доступностью selected environments, зафиксированными для одного model step.
- Реализация транспорта: `codex-rs/rmcp-client/src/rmcp_client.rs`, `stdio_server_launcher.rs`, `executor_process_transport.rs`, `http_client_adapter.rs`, `oauth.rs`, `oauth/store_lock.rs`, `auth_status.rs` и `perform_oauth_login.rs` владеют RMCP stdio/HTTP transport, protected-resource OAuth discovery, хранением OAuth, ограниченной межпроцессной блокировкой aggregate stores File/Secrets, обновлением/восстановлением token и очисткой processes. HTTP MCP и OAuth traffic выбранного executor plugin используют HTTP client владеющего executor, а не host-local bootstrap path.
- Видимая модели экспозиция tools: `codex-rs/core/src/session/step_context.rs`, `codex-rs/core/src/mcp_tool_exposure.rs`, `codex-rs/core/src/tools/handlers/mcp.rs`, `codex-rs/core/src/mcp_tool_call.rs`, `codex-rs/core/src/mcp_openai_file.rs`, `codex-rs/tools/src/mcp_tool.rs` и `codex-rs/tools/src/tool_config.rs` владеют MCP tool snapshot на один sampling request, отложенной через tool search экспозицией по умолчанию при её поддержке, прямым fallback при отсутствии search, нормализацией namespace, загрузкой файлов через environment filesystem, hook payloads, approval review и выполнением tool calls.
- Resources/templates: `codex-rs/protocol/src/mcp.rs` owns internal MCP resource/template/content types; `codex-rs/core/src/tools/handlers/mcp_resource.rs` and sibling handlers expose `list_mcp_resources`, `list_mcp_resource_templates` and `read_mcp_resource` to the model.
- App-server API: `codex-rs/app-server-protocol/src/protocol/v2/mcp.rs` владеет v2 wire shapes для status/list, resource read, tool call, refresh, OAuth login, типизированной startup failure `reauthenticationRequired` и elicitations, включая расширенные формы `openai/form`; OAuth login выбранного executor принимает thread context, чтобы discovery, callback и token exchange использовали правильный runtime. `codex-rs/app-server/src/request_processors/mcp_processor.rs` владеет обработкой requests; `codex-rs/app-server/src/mcp_refresh.rs` владеет очередью reload/refresh. Проекции MCP tool-call items могут включать доверенный Apps/MCP app identity context.
- Координация elicitations: `codex-rs/core/src/elicitation.rs` владеет считающим session-scoped `ElicitationService`; созданные core и server MCP elicitations регистрируются в нём, поэтому результаты code-mode/tools и учёт timeout остаются приостановленными до завершения всех активных requests. `codex-mcp` хранит общий для thread elicitation router с opaque tokens, чтобы response после замены runtime всё ещё достигал точного pending request.
- Auth hosted Apps: `codex-rs/model-provider/src/auth.rs` владеет привязанным к identity `AuthManagerAuthProvider`; только зарезервированная host-owned регистрация `codex_apps` использует обновление token той же account identity на каждом request, а пользовательские/прямые MCP servers сохраняют свой настроенный auth path.
- TUI projections: `codex-rs/tui/src/chatwidget/mcp_startup.rs` renders startup state; `codex-rs/tui/src/history_cell/mcp.rs` renders inventory, tool calls and outputs; `codex-rs/tui/src/bottom_pane/mcp_server_elicitation.rs` renders server elicitation forms; slash command handling routes `/mcp` through the app-server/session surfaces.
- Diagnostics and trace: `codex-rs/rollout-trace/src/mcp.rs`, MCP tool spans/metrics and app-server/TUI history items are evidence/projection surfaces. Runtime behavior still belongs in config/catalog/connection manager/tool handlers.

## Lifecycle flows

### Config and registration

1. Config layers deserialize `[mcp_servers.<name>]` into `McpServerConfig`; invalid transport combinations fail during config parsing.
2. Requirements and managed config can disable or constrain servers, preserving `disabled_reason` for diagnostics.
3. Plugin, selected plugin, compatibility and extension declarations become `McpServerRegistration` entries after plugin manifests, path lists and marketplace fallback are resolved.
4. The catalog resolves precedence and disabled name-veto behavior before any connection startup.

### Startup and refresh

1. Session/thread runtime builds `McpConfig` from current config, auth, plugins and extension contributions.
2. `McpRuntimeContext` разрешает environment каждого server. Локальный stdio использует host-native правила cwd/fallback; remote stdio требует явный `cwd`, но принимает absolute paths в формате целевого environment и преобразует их в `PathUri` при запуске через executor; HTTP и OAuth выбранного executor plugin используют сеть владеющего executor. Sandbox metadata ограничена разрешённым environment server и может консервативно использоваться `codex sandbox` только при явном permission profile.
3. Каждый model step проецирует MCP/connector declarations выбранных plugins через capability roots, готовые именно для этого шага. Изменение входной availability перестраивает manager только при фактическом изменении winning MCP servers, ownership, connectors или доступности явно привязанного environment.
4. `McpConnectionManager` запускает включённые servers, публикует per-server startup status и хранит metadata: origin, required flag, approval defaults и поддержку parallel calls. `McpRuntimeSnapshot` привязывает этот manager/config/runtime context к шагу, который объявил его tools.
5. App-server `config/mcpServer/reload` ставит strict refresh в очередь для загруженных threads. Refresh атомарно публикует новый runtime; старые snapshots остаются валидными для in-flight work, а заменённые managers/processes отменяются после освобождения последнего runtime handle, без изменения общего глобального списка tools.

### Tool exposure and calls

1. Шаг фиксирует один `McpRuntimeSnapshot`, а sampling request лениво фиксирует один список tools из его manager. Поэтому tool planning, app/world-state context и execution видят одно MCP state, даже если refresh параллельно публикует более новый runtime.
2. `McpConnectionManager` нормализует canonical/callable names, сохраняя raw server/tool names для фактического MCP call.
3. `McpToolExposure` откладывает effective MCP tools через tool search, когда он поддерживается, и использует прямую видимую модели экспозицию только как fallback при недоступности search. Apps MCP tools дополнительно фильтруются по connector policy и доступности app.
4. `McpHandler` маршрутизирует model tool calls через `handle_mcp_tool_call`, сохраняя hook payloads, approval behavior, telemetry tags и plugin provenance. В режиме `writes` approval пропускают только tools с явной аннотацией `readOnlyHint = true`; повторное использование session/persistent approval в этом режиме отключено.
5. Результаты tools становятся protocol/turn items и history cells TUI/app-server, а не строками, доступными только в transcript.

### Resources and app-server operations

1. Model tools can list resources/templates across all configured servers or by server, then read a specific resource by server and URI.
2. App-server `mcpServerStatus/list` can fetch tools, auth status, server info, resources and templates with optional thread context and pagination.
3. App-server `mcpServer/resource/read` supports thread-bound reads through loaded thread runtime, or threadless reads from latest config with config-level environment fallback.
4. App-server `mcpServer/tool/call` requires `threadId` and injects thread metadata into `_meta` before routing to the thread's configured MCP runtime.

### Auth, approvals and elicitations

1. Streamable HTTP servers can use bearer token env vars or OAuth credentials; plaintext `bearer_token` is rejected with targeted diagnostics.
2. `mcpServer/oauth/login` разрешает configured/requested/discovered scopes, учитывает настройки OAuth client id/resource/callback и отправляет completion notification. Для HTTP servers выбранного executor параметр `threadId` выбирает runtime, executor которого выполняет OAuth discovery, token exchange и refresh.
3. MCP tool approval can come from global approval policy, server-level default, per-tool override, Apps connector policy or approval reviewer integration.
4. Server-driven elicitations проходят через нейтральные protocol types до app-server wire conversion, включая расширенные формы `openai/form`. Считающий session-scoped service приостанавливает доставку результатов и учёт active time до завершения всех outstanding elicitations; общий для thread response router сохраняет доставку при MCP runtime refresh.
5. Apps MCP identity and app-id filtering belong in `codex-mcp`/connector policy paths; trusted app identity should be projected on tool-call items rather than inferred from raw tool names.

### Plugins, extensions and thread scope

1. Plugin MCP declarations are parsed from supported plugin manifest shapes into normal MCP configs and attributed for UI/tool provenance.
2. Selected executor plugin MCP declarations are discovered from the selected environment-owned capability root and activated only for the thread that selected that root.
3. Extension MCP contributions add/replace/remove registrations through contributor APIs and must remain scoped to the runtime context they were resolved for.
4. Thread-scoped MCP contributions must not leak startup status, tools, resources or plugin provenance into unrelated threads.

## Developer guidance for fork changes

- Add new MCP config fields in `codex-rs/config/src/mcp_types.rs`, update schema/tests and prove the exact runtime parser accepts the field before documenting it as usable.
- Prefer `codex-rs/codex-mcp/src/connection_manager.rs`, `catalog.rs`, `runtime.rs` and `mcp/mod.rs` for MCP runtime changes; avoid plumbing ad hoc server/tool maps through `codex-core`.
- Preserve one source of truth for registration: config/plugin/extension declarations should become catalog entries, then effective servers. Do not add hardcoded tool lists or one-off UI switches.
- Если изменение затрагивает видимые модели tools, вместе обновляй согласованность `McpRuntimeSnapshot`/`StepContext`, per-request tool snapshot, экспозицию через tool search по умолчанию, поведение прямого fallback, нормализацию namespace, tool search metadata, hook payloads и telemetry.
- If a change affects app-server behavior, update `app-server-protocol` v2 types, generated schema/fixtures and app-server README entries together.
- If a change affects TUI-visible startup, inventory, tool-call or elicitation rendering, update TUI tests/snapshots and keep thread-id routing explicit.
- Если изменение затрагивает sandbox, cwd, environment id, OAuth, bearer tokens, approvals или elicitations, добавляй положительные и отрицательные security tests; сохраняй auth boundaries для origin/account и сериализацию aggregate credential store, не добавляй silent fallback для небезопасной или неразбираемой config.
- For selected plugin or extension MCPs, validate thread scope: selected declarations should be visible only in the owning thread/runtime and refresh should preserve that boundary.

## Native propagation rule for fork work

Новая MCP capability должна проходить через существующий source-of-truth и projections:

- config field -> config/schema/tests -> catalog/effective server -> connection manager -> app-server/TUI diagnostics;
- plugin/extension declaration -> registration catalog -> effective server -> thread/runtime scoped startup -> tool/resource status;
- tool metadata -> MCP tool normalization -> `McpHandler` -> protocol item/app-server history -> TUI history cell;
- resource metadata -> protocol MCP resource types -> model resource tools -> app-server status/resource read;
- approval/elicitation behavior -> config/app policy -> connection manager/reviewer -> app-server request -> TUI bottom pane response;
- environment/cwd/permission state -> config `LegacyAppPathString`/`McpRuntimeContext`/sandbox state -> step-pinned `McpRuntimeSnapshot` -> RMCP local or executor transport -> teardown/recovery tests;
- selected plugin HTTP/OAuth -> URI-native selected root -> executor-owned plugin declaration -> thread/runtime projection -> executor HTTP client -> app-server login/status and end-to-end token-exchange tests;
- OAuth/auth -> `McpServerAuth` plus trusted-origin/account checks -> auth status/startup reason -> serialized credential store or host-owned dynamic provider -> app-server diagnostics/reconnect behavior.

Антипаттерны: parallel config, duplicated server registries, global plugin MCP activation, UI-only hiding, transcript scraping, direct app-server-to-plugin calls that bypass catalog/runtime resolution, and fallback to local cwd for named remote stdio servers.

## Compatibility notes and open questions

- `mcp__` tool prefixes and later manager-owned naming modes are compatibility-sensitive; raw server/tool names must remain available for actual MCP protocol calls.
- Supported model/provider combinations should receive MCP tools through `tool_search` by default; small MCP inventories being directly visible is now compatibility fallback behavior, not the primary contract.
- Remote stdio `cwd` may use the executor platform's absolute path syntax. Fork checks must not reject a Windows absolute cwd just because the host parser is POSIX, or the inverse.
- `enabled_tools` and `disabled_tools` are exposure filters within one server, not a universal permission model for built-in tools.
- `required = true` changes startup failure semantics and can make `codex exec` fail fast; UI-only warnings are not equivalent.
- Threadless app-server resource reads are useful for inventory, but they do not have turn-selected environment/cwd and must not pretend to inherit a thread runtime.
- Apps MCP uses host/auth/connector policy in addition to generic MCP rules; custom MCP servers should not inherit Apps-specific auth or route filters unless they are explicitly host-owned Apps registrations.
- MCP выбранного executor plugin поддерживают stdio и streamable HTTP declarations. HTTP connection, OAuth discovery/exchange/refresh и thread-bound login должны оставаться в сети владеющего executor; host-local auth bootstrap не является совместимым fallback.
- Model step обязан выполняться через тот же `McpRuntimeSnapshot`, tools которого он объявил. Чтение latest session manager во время вызова возвращает race с refresh и несовместимо с нативным lifecycle `0.143+`.
- `McpServerAuth::ChatGpt` не является общим переключателем перенаправления credentials: session auth может получить только доверенный first-party ChatGPT origin, configured authorization имеет приоритет, а зарезервированный dynamic provider `codex_apps` следует за token refresh только пока account/user/workspace identity не изменилась.
- Regex в managed MCP matchers сопоставляются с полным значением, а списки аргументов command имеют точную длину и порядок. Они ограничивают executable/arguments или direct URL, но не stdio `cwd`, `env` или `env_vars`.
- `codex-rs/mcp-server` may share names and app-server protocol concepts, but it is not the owner of external MCP server runtime behavior.
