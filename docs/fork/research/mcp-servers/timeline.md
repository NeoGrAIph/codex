# Эволюция внешних MCP servers

Статус исследования: покрыты stable-релизы от первого внешнего MCP server commit `147a940449839b116b220b7e7d016d2a2890c134` до текущего локального `rust-v0.144.1`. Основной timeline строится по stable-contained commits; alpha/side-branch commits не включены в основной поток без отдельного указания.

## Поколения

- Initial stdio MCP: `[mcp_servers]` в config, запуск stdio server, tool-call routing, startup tolerance and timeouts.
- RMCP/streamable HTTP/OAuth: переход на `rmcp`, streamable HTTP transport, OAuth credentials, headers, scopes, resource listing/read и tool filtering.
- App-server/TUI projection: `/mcp`, startup status, app-server MCP status/resource/tool APIs, elicitations and approval UI.
- Apps/plugins/deferred tools: host-owned Apps MCP, plugin MCP declarations, tool search/deferred exposure and connector policy.
- Runtime/environment isolation: thread/runtime-scoped MCP config, executor-backed stdio, explicit environments, permission profiles and shutdown/process cleanup.
- Catalog/extensions/selected plugins: `codex-mcp` catalog, extension contributions, selected executor plugin MCPs and thread-scoped activation.
- Согласованность step-pinned runtime: точные config/manager/environment на model step, один tool snapshot на sampling request, учитывающее availability переиспользование runtime и маршрутизация elicitations через refresh.

## `rust-v0.2.0`

- First containing stable for external MCP server support: `rust-v0.2.0`.

### Initial configured MCP servers (`#829`, `#854`)

- Коммиты: `147a940449` (`feat: support mcp_servers in config.toml (#829)`), `27198bfe11` (`fix: make McpConnectionManager tolerant of MCPs that fail to start (#854)`).
- Developer info: adds `[mcp_servers]` config, first `McpConnectionManager`, stdio server launch and MCP tool-call routing from model tool calls into configured servers; then makes startup tolerant when a server fails.
- Why: commit body for `#829` explicitly describes external MCP servers in Claude Desktop/Cursor style and calls out `mcp_connection_manager.rs` plus `mcp_tool_call.rs` as the main implementation paths.
- User info: Codex can use external tools such as a weather MCP server from config; one broken optional server no longer prevents the whole session from continuing.
- Compatibility/risks: the first implementation launched all configured servers eagerly, had limited recovery, and ran MCP server code outside the Codex sandbox boundary described in the commit body.
- Evidence: `EVID-002-initial-config`.

## `rust-v0.31.0`-`rust-v0.40.0`

### Startup and tool-call timeouts (`#3182`, `#3959`)

- Коммиты: `6efb52e545` (`feat(mcp): per-server startup timeout (#3182)`), `19f46439ae` (`timeouts for mcp tool calls (#3959)`).
- Developer info: separates startup timeout from tool-call timeout so slow initialization and long-running tool execution can be controlled independently.
- Why: path evidence touches MCP config/client code and stable containment maps the commits to `rust-v0.31.0` and `rust-v0.40.0`.
- User info: slow MCP servers fail with bounded startup behavior; individual tools also have bounded execution time.
- Compatibility/risks: timeout fields are config contract; future changes must update schema/docs and keep app-server/status diagnostics aligned.
- Evidence: `EVID-031-040-timeouts`.

## `rust-v0.42.0`-`rust-v0.48.0`

### RMCP, streamable HTTP, OAuth, resources and filters (`#4252`, `#4317`, `#4517`, `#5239`, `#5367`, `#5529`)

- Коммиты: `e555a36c6a` (`[MCP] Introduce an experimental official rust sdk based mcp client (#4252)`), `3a1be084f9` (`[MCP] Add experimental support for streamable HTTP MCP servers (#4317)`), `1d17ca1fa3` (`[MCP] Add support for MCP Oauth credentials (#4517)`), `40fba1bb4c` (`[MCP] Add support for resources (#5239)`), `740b4a95f4` (`[MCP] Add configuration options to enable or disable specific tools (#5367)`), `4cd6b01494` (`[MCP] Remove the legacy stdio client in favor of rmcp (#5529)`).
- Developer info: external MCP moves from the initial custom client toward RMCP-backed stdio and streamable HTTP, adds OAuth credential handling, resources/resource templates, and per-server tool allow/deny controls.
- Why: stable containment places these commits between `rust-v0.42.0` and `rust-v0.48.0`, and diffs touch `rmcp-client`, config parsing and MCP resource/tool paths.
- User info: Codex can connect to local stdio and HTTP MCP servers, log in to OAuth-backed servers, list/read MCP resources and hide specific server tools.
- Compatibility/risks: tool names, OAuth state and resource shapes become durable surfaces; hidden/disabled tools must be removed before model exposure, not only filtered in UI.
- Evidence: `EVID-042-048-rmcp-http-oauth-resources`.

## `rust-v0.59.0`-`rust-v0.67.0`

### Non-blocking startup, elicitations and in-session login (`#6334`, `#6947`, `#7751`)

- Коммиты: `03ffe4d595` (`core/tui: non-blocking MCP startup (#6334)`), `7561a6aaf0` (`support MCP elicitations (#6947)`), `893f5261eb` (`feat: support mcp in-session login (#7751)`).
- Developer info: MCP startup becomes non-blocking in the TUI/core path, MCP server elicitations enter the interaction model, and login can happen during an existing session.
- Why: commits are stable-contained in `rust-v0.59.0`, `rust-v0.64.0` and `rust-v0.67.0`; diffs touch core/TUI startup and MCP auth/elicitation flow.
- User info: Codex no longer freezes user work while MCP servers boot; servers can ask the client/user for required input and OAuth can be initiated without restarting Codex.
- Compatibility/risks: startup state is user-visible and asynchronous; elicitations must preserve turn/request correlation and avoid silently auto-accepting privileged input.
- Evidence: `EVID-059-067-startup-elicitation-login`.

## `rust-v0.81.0`-`rust-v0.99.0`

### Requirements, approvals and stdio cleanup (`#9101`, `#10200`, `#10710`)

- Коммиты: `2651980bdf` (`Restrict MCP servers from requirements.toml (#9101)`), `34f89b12d0` (`MCP tool call approval (simplified version) (#10200)`), `82c981cafc` (`Process-group cleanup for stdio MCP servers to prevent orphan process storms (#10710)`).
- Developer info: requirements layers can disable MCP servers, MCP tool calls get explicit approval handling, and stdio processes are cleaned up as process groups.
- Why: evidence commands confirm first containing stable tags `rust-v0.81.0`, `rust-v0.93.0` and `rust-v0.99.0`; current config still carries `disabled_reason`, approval modes and stdio runtime cleanup tests.
- User info: managed/project requirements can restrict available external servers; destructive or untrusted MCP tools can prompt for approval; stopped sessions should not leave process storms behind.
- Compatibility/risks: requirements must disable by source-of-truth config/catalog rather than by UI hiding; stdio teardown is part of the runtime safety contract.
- Evidence: `EVID-081-099-requirements-approvals-cleanup`.

## `rust-v0.111.0`-`rust-v0.119.0`

### App-server elicitations, `codex-mcp` extraction, Apps MCP and deferred custom tools (`#13425`, `#15919`, `#16082`, `#16944`, `#17043`)

- Коммиты: `926b2f19e8` (`feat(app-server): support mcp elicitations in v2 api (#13425)`), `59b68f5519` (`Extract MCP into codex-mcp crate (#15919)`), `5fe9ef06ce` (`[mcp] Support MCP Apps part 1. (#16082)`), `d7f99b0fa6` (`[mcp] Expand tool search to custom MCPs. (#16944)`), `7b6486a145` (`[mcp] Support server-driven elicitations (#17043)`).
- Developer info: app-server gains MCP elicitation protocol, MCP runtime is extracted into `codex-mcp`, host-owned Apps MCP starts integrating with the same runtime, and custom MCP tools become discoverable through tool search/deferred exposure.
- Why: stable containment maps these to `rust-v0.111.0` and `rust-v0.119.0`; current HEAD still has `codex-rs/codex-mcp`, app-server MCP elicitation types and deferred MCP exposure logic.
- User info: app clients can render/respond to MCP elicitations; large MCP tool inventories can stay out of the model context until discovered; Apps and custom MCP servers share more runtime plumbing.
- Compatibility/risks: app-server and TUI must use typed elicitation requests, and deferred tools still need exact routing back to the original MCP server/tool names.
- Evidence: `EVID-111-119-appserver-codex-mcp-apps`.

## `rust-v0.122.0`-`rust-v0.124.0`

### Deferred code-mode tools, environments, threadless resources, hooks and permission profiles (`#17287`, `#18085`, `#18212`, `#18292`, `#18385`, `#18286`)

- Коммиты: `9c6d038622` (`[code mode] defer mcp tools from exec description (#17287)`), `b4be3617f9` (`[1/8] Add MCP server environment config (#18085)`), `996aa23e4c` (`[5/6] Wire executor-backed MCP stdio (#18212)`), `1132ef887c` (`Make MCP resource read threadless (#18292)`), `305825abd9` (`Support MCP tools in hooks (#18385)`), `ff22982d75` (`mcp: include permission profiles in sandbox state (#18286)`).
- Developer info: MCP tools can be deferred from code-mode descriptions, MCP servers gain environment placement, stdio can be launched through executor environments, app-server can read resources without a loaded thread, hooks can observe MCP tools, and sandbox state includes permission profiles.
- Why: stable tags place these commits in `rust-v0.122.0` through `rust-v0.124.0`; current source paths show `McpRuntimeContext`, threadless app-server resource reads, hook payload support and `SandboxState.permission_profile`.
- User info: external servers can be tied to execution environments, resources can be inspected from app-server status/resource APIs, and MCP tool/hook/permission behavior aligns better with active session policy.
- Compatibility/risks: remote stdio requires explicit absolute `cwd`; threadless resource reads lack turn-selected environment and must use config-level fallback rules only.
- Evidence: `EVID-122-124-environments-resources-hooks`.

## `rust-v0.126.0`-`rust-v0.135.0`

### External import, turn items, built-in runtime servers, explicit environments and naming manager (`#19949`, `#20677`, `#21356`, `#23583`, `#21576`)

- Коммиты: `cb8b1bbcd6` (`Support detect and import MCP, Subagents, hooks, commands from external (#19949)`), `c8c30d9d75` (`[codex] Emit MCP tool calls as turn items (#20677)`), `b2268999fe` (`feat: make built-in MCPs first-class runtime servers (#21356)`), `298e5cfce1` (`Route MCP servers through explicit environments (#23583)`), `ff7513cd83` (`Move MCP tool naming mode into manager (#21576)`).
- Developer info: external-agent migration can import MCP configs, MCP tool calls become protocol/turn items, built-in MCPs use the same runtime server path, server environment routing becomes explicit, and naming/prefix behavior moves into the manager.
- Why: stable containment spans `rust-v0.126.0` through `rust-v0.135.0`; current architecture has `McpConfig.mcp_server_catalog`, protocol MCP tool items and manager-owned `prefix_mcp_tool_names`.
- User info: imported external agent configs can bring MCP declarations into Codex, clients can render MCP calls as first-class history items, and built-in/custom/plugin MCPs converge on common runtime behavior.
- Compatibility/risks: migration must reject mixed/ambiguous transport configs; clients should render MCP turn items rather than scraping transcript text; tool naming compatibility must remain manager-owned.
- Evidence: `EVID-126-135-import-items-runtime`.

## `rust-v0.140.0`-`rust-v0.141.0`

### Catalog, thread-scoped contributions and selected executor plugin MCPs (`#27634`, `#27670`, `#27870`, `#27893`)

- Коммиты: `4a5a676499` (`Resolve MCP server registrations through a catalog (#27634)`), `693082f3c4` (`Make MCP server contributions thread-scoped (#27670)`), `b3c423e475` (`Discover stdio MCP servers from selected executor plugins (#27870)`), `c8c78b63a7` (`Activate selected executor plugin MCPs in app-server (#27893)`).
- Developer info: MCP declarations resolve through a catalog with source/precedence, extension contributions can be thread-scoped, selected executor plugins can contribute stdio MCP declarations, and app-server activates those only for selected thread capability roots.
- Why: commits are stable-contained in `rust-v0.140.0` and `rust-v0.141.0`; current source paths include `codex-mcp/src/catalog.rs`, `ext/extension-api/src/contributors/mcp.rs`, `ext/mcp/src/executor_plugin.rs` and app-server selected capability root wiring.
- User info: plugin/executor MCP tools can be available only to the thread that selected that plugin/root; unrelated threads do not inherit those tools globally.
- Compatibility/risks: selected plugin MCPs must not be registered globally; precedence and disabled registrations must be resolved in the catalog before connection startup.
- Evidence: `EVID-140-141-catalog-selected-plugins`.

## `rust-v0.142.0`

- Release commit: `3a76f3ac68c8949d1cac6ea769b6ec7b8953a415`
- Дата: `2026-06-22T23:36:01+02:00`
- Subject: release notes include plugin marketplace/recommendation work, MCP/plugin loading fixes, stdio MCP disconnect resilience, remote environment preservation and selected runtime polish. Direct external-MCP entries here are environment-filesystem file uploads, plugin MCP manifest/fallback handling and runtime security/auth/identity updates.

### Environment file uploads for MCP (`#27923`, supporting `#28146`)

- Коммиты: `7baf7e467e9bd8a34772c14a1fd5edbe9039bea1` (`[codex] Route MCP file uploads through environment filesystem (#27923)`), supporting context `f8850cab1d0f192a799122ff96cb27061b9366eb` (`app-server: preserve target-native environment cwd (#28146)`).
- Developer info: routes MCP/OpenAI file uploads through environment filesystem handling and preserves target-native environment cwd in app-server thread/turn parameters so remote/executor paths stay native.
- Why: diffs touch `codex-rs/core/src/mcp_openai_file.rs`, `codex-api/src/files.rs`, app-server protocol turn/thread environment params and openai-file MCP tests.
- User info: MCP-related file upload flows use the selected environment filesystem instead of assuming host-local paths.
- Compatibility/risks: file upload behavior is environment-sensitive; fork changes must not silently fall back to local cwd when a remote/executor environment owns the file.
- Evidence: `EVID-142-mcp-files-env`.

### Plugin MCP manifest formats and marketplace fallback (`#28580`, `#28790`, `#28771`, `#28789`)

- Коммиты: `1883dedc0e3499c8f42e08835540319ad7131d77` (`[codex] Support object-valued plugin MCP manifests (#28580)`), `e12dd73b7d5a2aa2b8d0933a2053e7eb5eba6fbb` (`[codex] Support plugin manifest path lists (#28790)`), `a760b63f838db94369f91b829a366c76f4761107` (`fix(plugins): support root local marketplace plugins (#28771)`), `772c5c51952a8bd279dbeeedfea69a6feb837a1d` (`[codex] Support marketplace plugin manifest fallback (#28789)`).
- Developer info: expands plugin manifest MCP declaration parsing to object-valued manifests and path lists, supports root local marketplace layouts and adds marketplace manifest fallback handling through `core-plugins` loader/marketplace/store plus executor-plugin MCP provider tests.
- Why: release notes call out root marketplace layouts and manifest fallback; diffs touch `core-plugins/src/{loader,manifest,marketplace,store}.rs`, `plugin/src/manifest.rs` and `ext/mcp/src/executor_plugin/provider*.rs`.
- User info: plugin-provided MCP declarations can be discovered from more marketplace/manifest layouts without manual restructuring.
- Compatibility/risks: plugin manifest parsing is a source-of-truth contract before catalog registration; fork MCP changes must keep object-valued and path-list forms compatible.
- Evidence: `EVID-142-plugin-manifests`.

### MCP runtime security, elicitations, Apps identity and toggles (`#28914`, `#27500`, `#27132`, `#28947`, `#29022`, `#28942`)

- Коммиты: `790213ded0588d824e99f830f41f04f3d98196df` (`Scope MCP sandbox metadata to server environment (#28914)`), `21a599fa56472a7cea8132c5a47a4374d4d5aa17` (``Support `openai/form` extended form elicitations (#27500)``), `765309d5a611ea02be842ead0ab1a2828196fae9` (`Emit Trusted MCP App Identity on Tool-Call Items (#27132)`), `29eb434bc5fd81f29540446f6989219736b09a80` (`[codex] Remove hardcoded app ID filters (#28947)`), `4e6bc4226658b7bc2ba4e207506b51f8d6d3626f` (`[codex] Support protected resource OAuth discovery (#29022)`), `81b000421dc795062019f3737db6fac3fda16aa4` (`Add config toggles for orchestrator skills and MCP (#28942)`).
- Developer info: scopes MCP sandbox metadata to each server environment, extends typed MCP elicitations with `openai/form`, projects trusted MCP App identity on tool-call items, moves app-id filtering out of hardcoded lists, adds protected-resource OAuth discovery and config toggles around orchestrator skills/MCP resource tools.
- Why: diffs touch `codex-mcp/src/{connection_manager,runtime,server,codex_apps,mcp/mod}.rs`, `rmcp-client/src/auth_status.rs`, app-server protocol MCP schemas/tests, `core/src/mcp_tool_call.rs`, `core/src/tools/handlers/mcp_resource/*` and config/schema paths.
- User info: MCP runtime/auth/error surfaces become more explicit: app clients can render richer elicitations, Apps tool calls carry trusted identity context, protected-resource OAuth is discoverable and resource tools can be gated by config.
- Compatibility/risks: these are security/auth/projection contracts. Do not implement fork MCP behavior through raw app id string filters, global sandbox metadata or untyped elicitation payloads.
- Evidence: `EVID-142-mcp-runtime-security`.

## `rust-v0.142.2`

- Release commit: `390b0d254d658148751d0cca50ca41832c7894a1`
- Дата: `2026-06-24T23:36:23-07:00`
- Subject: patch release containing the default searched MCP tool flow and remote stdio cwd portability work selected for this MCP timeline.

### Default tool-search exposure and foreign remote stdio cwd (`#29486`, `#29493`)

- Коммиты: `c53b1dae09db40902c59f6a0d57d0dcc334926db` (`[codex] Use tool search for MCP tools by default (#29486)`), `67009bc53ffc684eaf235c6186df890d35259240` (`mcp: accept foreign absolute cwd for remote stdio (#29493)`); supporting context `11fab432be5a873fea97688ef67e8af896f5ea8c` (`path-uri: clarify host-native path conversion (#29501)`).
- Developer info: makes MCP tools deferred through `tool_search` whenever search and namespaced tools are supported, removing the feature-gated/100-tool threshold behavior while preserving direct exposure fallback for unsupported model/provider combinations. Remote stdio MCP `cwd` is now deserialized as `LegacyAppPathString` and converted to `PathUri` at executor launch, so target-platform absolute paths are not rejected by the orchestrator host parser.
- Why: commit bodies explicitly describe the intended searched-tool MCP flow and the remote cwd portability issue; diffs touch `core/src/mcp_tool_exposure.rs`, `core/src/tools/spec_plan.rs`, `features/src/lib.rs`, `config/src/mcp_types.rs`, `codex-mcp/src/runtime.rs`, `rmcp-client/src/stdio_server_launcher.rs` and path-uri helpers/tests.
- User info: supported setups discover MCP tools through `tool_search` first, and remote stdio MCP servers can use a cwd spelling native to the executor environment.
- Compatibility/risks: fork changes must not assume small MCP inventories are directly model-visible when search is available, and must not reject Windows-style absolute cwd on a POSIX host or POSIX-style cwd on a Windows host for remote stdio.
- Evidence: `EVID-142.2-tool-search-remote-stdio-cwd`.

## `rust-v0.143.0`

- Release commit: `c4d748f586a84a3ed5b6aceb82e9a1db4abb1cda`
- Дата: `2026-07-07T17:43:00-07:00`
- Release notes прямо называют MCP tool search по умолчанию, session auth для ChatGPT-hosted MCP и исправление MCP startup после отменённого review. Локальная проверка commits/source показывает более крупное архитектурное изменение: step-pinned runtimes, HTTP/OAuth выбранного executor и типизированные границы auth/policy.

### Step-pinned runtime, проекция availability и устойчивые к refresh elicitations (`#29608`, `#30101`, `#30093`, `#30127`, `#30148`, `#30627`)

- Коммиты: `8751fd3fcb8031d42b62670a6131872074635c9b`, `ee9e0f6387b91cd14008ea59a8e4315b2a2fdacb`, `3095ea9c3d155bfc89197d2628eb818a55c2755d`, `fb8598df3ff05366a295c407c91f3d9316a1630c`, `6d2168f06ae275d5e1f73cabf935d2bcc8549998`, `84fe70c30e88a1828a510804341202bab629d507`; нейтральная lower-layer elicitation boundary `df1ee09ec50453da3976d239da6cb035403ff28f` (`#29724`).
- Для разработчика: `McpRuntimeSnapshot` фиксирует точные MCP config, manager и runtime context для одного model step. MCP/connector declarations выбранного plugin проецируются через roots, готовые для этого шага; неизменившиеся effective projections переиспользуют live manager. Refresh публикует новый runtime, не меняя manager/tool set, уже объявленный in-flight step. Общий для thread router с opaque token сохраняет возможность ответить на старые MCP elicitations после замены, а считающий session-owned `ElicitationService` приостанавливает доставку результатов до завершения всех outstanding elicitations.
- Для пользователя: готовность environment и MCP refresh больше не приводят к исчезновению tool между объявлением и выполнением, ненужному перезапуску незатронутых stdio servers или зависанию пользовательского prompt на старом runtime.
- Совместимость/риски: все видимые модели consumers должны использовать snapshot шага, а не latest session-global manager. Runtime identity включает winning servers, ownership, connectors и явную environment availability; очистка refresh не должна завершать manager, который всё ещё удерживает in-flight step.
- Доказательства: `EVID-143-runtime-snapshots-refresh-elicitations`.

### Selected executor plugin HTTP MCP and OAuth (`#28918`, `#29628`, `#28522`, `#28529`, `#29656`)

- Коммиты: `2e69966cd8fbb3f0265152af778dd5512c66d151`, `3e39e92f03431a83d004d9ecdb90d8c82fc24d02`, `6368937939dceb07b4a3c47c4448027d0d1a85a6`, `b215961a56b2553a4612a22a812747403c08a58b`; поддерживающий end-to-end test token exchange `c38b2e9ba69cb57d197c6e5ba78b5e52ae0870f9` и prerequisite RMCP 1.8 `bbe100689073815876d330e56a200217a96a9e35`.
- Для разработчика: selected roots, manifest resources и relative MCP cwd остаются URI-native, пока выбранный executor не выполнит разрешение filesystem. Streamable HTTP declarations сохраняются вместе со stdio; connection traffic, OAuth discovery, dynamic registration, PKCE token exchange и refresh используют HTTP client владеющего executor. App-server OAuth login принимает thread context для выбора этого runtime.
- Для пользователя: выбранный remote plugin может предоставить HTTP MCP tools и завершить OAuth, даже если его server и token endpoints доступны только из environment executor.
- Совместимость/риски: нельзя проецировать executor paths на host app-server или неявно выполнять OAuth executor MCP через host-local client. Login/status выбранного server должны сохранять thread scope.
- Доказательства: `EVID-143-selected-http-oauth-paths`.

### Auth enum, managed policy, использование sandbox state и recovery diagnostics (`#29358`, `#29733`, `#29924`, `#29648`, `#29877`, `#30257`)

- Коммиты: `d2484697b1f9ce33d1d818ccad859ca3a4d721c6`, `4c0706e24a290f89c40d99b92912367c63fe273d`, `f8937b7d862e3d507fe22918e5daa35c7da993e9`, `db541f45536f18741b50c74b05904a2c5f628c96`, `a6d20ed29701bd8f291e94526154dcac90367d92`, `526f495f3a904e29ec1d07a927992c904c8a1f5c`; усиление hosted HTTP affinity `b5866eebd631a62a1f3f29ac5e44752e38949c73` (`#29516`).
- Для разработчика: HTTP auth становится явным `McpServerAuth::{OAuth, ChatGpt}` с приоритетом configured authorization и hardcoded trusted-origin boundary для ChatGPT session credentials. Managed requirements получают matchers executable/ordered arguments и URL с операциями exact/prefix/full-value-regex. `codex sandbox` может использовать MCP sandbox metadata только при явном permission profile. Истёкшие OAuth credentials без возможности refresh публикуют типизированный `reauthenticationRequired`, включая nested RMCP transport errors.
- Для пользователя: enterprise policy может сопоставлять ограниченные proxy commands/URLs, app clients могут предложить точное действие reconnect, а доверенные ChatGPT-hosted MCP endpoints могут использовать session auth без передачи ChatGPT credentials произвольным настроенным servers.
- Совместимость/риски: matcher имеет semantics exact-length/ordered и не ограничивает cwd/env; ChatGPT auth разрешён только для first-party origin; Cloudflare affinity cookies остаются строго allowlisted и не создают общий cookie jar для third-party MCP.
- Доказательства: `EVID-143-auth-policy-sandbox-diagnostics`.

### MCP operational attribution (`#28630`, `#28976`, `#29969`)

- Коммиты: `322e33512b2d38d38d705e2ef692a8aca50decac`, `cbcf1f8ca33ea6f683d11232ae485b32d23028c2`, `cef5444a80ac5a94d435ab780fba5d6f433c504f`.
- Для разработчика: startup spans идентифицируют server и фазу initialization; tool-result `isError` становится метрикой failed call; bounded/sanitized error codes и server attribution применяются одинаково без special case для Codex Apps.
- Для пользователя: прямых изменений feature toggle нет, но медленный startup и failing servers/tools можно атрибутировать без помещения недоверенного error text в labels метрик.
- Совместимость/риски: метрики должны сохранять ограниченные доверенные dimensions и не раскрывать raw server error text.
- Доказательства: `EVID-143-auth-policy-sandbox-diagnostics`.

## `rust-v0.144.0`

- Release commit: `767822446c7a594caa19609ca435281a9ec67e0d`
- Дата: `2026-07-09T08:59:13-07:00`
- Release notes прямо называют auth elicitation по умолчанию, app approval mode `writes` и auth refresh для долгоживущего `codex_apps`.

### Сериализация OAuth store, auth elicitation по умолчанию и approvals с учётом writes (`#30292`, `#28772`, `#30482`)

- Коммиты: `6cf42cf16516ab3a125d853561ae3b8c77d6c13e`, `ff06ab7172f4a442e3cc0314b1178f8ea3c5ccac`, `f6e251c3ac6573a4d6eccddea6ad12587868485f`.
- Для разработчика: aggregate OAuth credential stores File и Secrets получают ограниченные межпроцессные locks вокруг load/save/delete, чтобы concurrent servers не теряли updates. Auth elicitation становится stable и включается по умолчанию. `AppToolApproval::Writes` пропускает prompts только для `readOnlyHint = true`, запрашивает approval для tools без annotations и writing tools и отключает повторно используемые approval choices.
- Для пользователя: concurrent MCP logins реже перезаписывают credentials друг друга, authentication prompts работают без experimental opt-in, а reads можно разрешать автоматически с review для writes.
- Совместимость/риски: failure блокировки store не равна недоступности Secrets и не должна запускать небезопасный fallback. Отсутствующие tool annotations считаются writes в режиме `writes`.
- Доказательства: `EVID-144-approvals-oauth-apps-consistency`.

### Согласованность sampling, форма Apps file и долгоживущий hosted auth (`#31292`, `#31330`, `#31486`, supporting `#31612`)

- Коммиты: `bdaad6820cd884ea11787477f4c495e4de0a8be5`, `a09a7c41d8abbaec6543664bff04a81058192958`, `555aa79d5ab4c011196a1f9fd08c6e6673dd0907`; форматирование timeout для пользователя `48cf58233190ce6f2aa6c3194d022c0e273e8fd6`.
- Для разработчика: `StepContext` лениво переиспользует один MCP tool snapshot для Apps World State и построения tool-router в течение sampling request. Перезапись Apps file отправляет только документированный file reference из четырёх полей. Зарезервированный hosted path `codex_apps` использует provider поверх `AuthManager`, который читает обновлённые credentials той же identity на каждом request; прямые/пользовательские MCP registrations сохраняют существующий auth path.
- Для пользователя: один model request видит согласованный tool inventory, строгие Apps schemas больше не отклоняют internal upload fields, а долгоживущие hosted connector sessions восстанавливаются после refresh общего ChatGPT token.
- Совместимость/риски: dynamic auth должен оставаться зарезервированным и привязанным к account/user/workspace; switch account не даёт ambient auth до перестроения runtime state. Этот provider нельзя обобщать на произвольные MCP registrations.
- Доказательства: `EVID-144-approvals-oauth-apps-consistency`.

## `rust-v0.144.1`

- Release commit: `44918ea10c0f99151c6710411b4322c2f5c96bea`
- Дата: `2026-07-09T15:10:27-07:00`
- Существенных изменений external MCP нет.
- Почему: literal range `rust-v0.144.0^{}..rust-v0.144.1^{}` содержит только parsing installer metadata и backports установки/fallback code-mode host. Diff не затрагивает MCP config, client/runtime, OAuth, экспозицию tool/resource или app-server MCP wire paths. Отдельный left-only release commit `rust-v0.144.0` также не входит в этот range; topology и disposition зафиксированы в `evidence.md`.
- Доказательства: `EVID-144.1-no-mcp-delta`.
