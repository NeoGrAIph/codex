# Evidence: external MCP servers

Файл фиксирует проверочные команды и решения triage для исследования внешних MCP servers. Основной язык - русский; commit subjects, PR ids, API/type/tool names сохраняются как в upstream.

## Общий scope и refs

- Scope in: Codex как MCP client/runtime для внешних MCP servers, включая `[mcp_servers]`, stdio/streamable HTTP, OAuth/auth, resources/templates, tool exposure, approvals/elicitations, app-server/TUI projections, plugins/apps/extensions and environment routing.
- Scope out: `codex-rs/mcp-server` как Codex-as-MCP-server binary/API. Коммиты вроде `21cd953dbd` (`feat: introduce mcp-server crate (#792)`), `2b72d05c5e` (`feat: make Codex available as a tool when running it as an MCP server (#811)`) и `497c5396c0` (`feat: add mcp subcommand to CLI to run Codex as an MCP server (#934)`) намеренно не включены в основной timeline.
- Начальный external-MCP commit: `147a940449839b116b220b7e7d016d2a2890c134`.
- Первый stable tag, содержащий начальный commit: `rust-v0.2.0`.
- Исследованный literal range: `rust-v0.142.5^{}..rust-v0.145.0^{}` (`26de83050b20f7e0ee211b9739e52ae00ce8032a..25af12f7e61572b0bc18ddb1008be543b91519b0`). Выражение `A..B` означает множество commits, достижимых из `B`, но не достижимых из `A`; оно не утверждает, что `A` является предком `B` или что stable refs образуют последовательную ancestry-цепочку.
- Промежуточные stable refs: `rust-v0.143.0^{}` = `c4d748f586a84a3ed5b6aceb82e9a1db4abb1cda`, `rust-v0.144.0^{}` = `767822446c7a594caa19609ca435281a9ec67e0d`, `rust-v0.144.1^{}` = `44918ea10c0f99151c6710411b4322c2f5c96bea`, `rust-v0.144.2^{}` = `a6645b6b8a656360fa16fb7e1c6721d0697d3d6a`, `rust-v0.144.3^{}` = `78ad6e6bfd1d3b6a209acd3ef82172a96b25179c`, `rust-v0.144.4^{}` = `8c68d4c87dc54d38861f5114e920c3de2efa5876`, `rust-v0.145.0^{}` = `25af12f7e61572b0bc18ddb1008be543b91519b0`.
- Boundary `rust-v0.144.4^{}` -> `rust-v0.145.0^{}`: left/right `7/342`, merge-base `3380969a29134630d56feb6218e8e8dcc5e8196d`. Left-only: `7ef1728763`, `9d47eb221d`, `77c42a202a`, `32649bc5e6`, `8a4d35a1e1`, `d82b7e5d4c`, `8c68d4c87d`. Отдельные patch-line изменения имеют reworked/content-equivalent реализации в `rust-v0.145.0`, включая `dc23c7bcc8`, `22781d4001`, `8cf9a1b1f8` и `60b9b551c1`; это не полный one-to-one mapping всех left-only commits.
- Верхняя stable граница локального исследования: `rust-v0.145.0` = `25af12f7e61572b0bc18ddb1008be543b91519b0`.

### Topology stable boundaries

- `rust-v0.142.5^{}` -> `rust-v0.143.0^{}`: `git rev-list --left-right --count A...B` даёт `6 258`; merge-base `27f22b54aef4d7e5eb6c564e969c961c74605461`. Шесть left-only commits линии `0.142.5` не входят в literal range `A..B`: release commit `26de83050b`, backport WebSocket trace `e019402a9e`, revert `07f7032383`, восстановление V1 delegation `3b8b60a583`, Bedrock models `a2325b50ff` и Ultra reasoning `aedb8f345a`. Они учтены как disposition release-line divergence, а не как изменения, которые нужно переносить в секцию `0.143.0`.
- `rust-v0.143.0^{}` -> `rust-v0.144.0^{}`: left/right `1 80`; merge-base `6afcf26d5d76c2f88b9096caa758931ffa673745`. Единственный left-only commit — release commit `c4d748f586`; он не входит в `rust-v0.143.0^{}..rust-v0.144.0^{}`.
- `rust-v0.144.0^{}` -> `rust-v0.144.1^{}`: left/right `1 4`; merge-base `3380969a29134630d56feb6218e8e8dcc5e8196d`. Единственный left-only commit — release commit `767822446c`; он не входит в `rust-v0.144.0^{}..rust-v0.144.1^{}`.
- `rust-v0.144.1^{}` -> `rust-v0.144.2^{}`: left/right `1 2`; merge-base `77c42a202aa7e89189c1126021dcc054b1795e1d`. Left-only — release commit `44918ea10c`; right-only — Guardian rollback `32649bc5e6` и release commit `a6645b6b8a`.
- `rust-v0.144.2^{}` -> `rust-v0.144.3^{}`: left/right `1 2`; merge-base `32649bc5e6591ad8ea0b7b8ce073df447565ec7c`. Left-only — release commit `a6645b6b8a`; right-only — Advanced Reasoning `8a4d35a1e1` и release commit `78ad6e6bfd`.
- `rust-v0.144.3^{}` -> `rust-v0.144.4^{}`: left/right `1 2`; merge-base `8a4d35a1e100efc2c64b72515668a84da663f067`. Left-only — release commit `78ad6e6bfd`; right-only — Guardian model-catalog policy `d82b7e5d4c` и release commit `8c68d4c87d`.
- Следовательно, release sections ниже группируют right-only commits по первой stable boundary, которая их содержит, и отдельно фиксируют left-only disposition; названия диапазонов не следует читать как линейный переход от parent к child.

## Базовые команды проверки

```bash
git show --quiet --format='%H%x09%cI%x09%s' <sha>
git show --stat --name-only --oneline <sha>
git show --no-color --patch <sha> -- <selected-paths>
git tag --contains <sha> | rg '^rust-v0\.[0-9]+\.0$' | sort -V | head -1
git log rust-v0.142.0 --reverse --date=short --format='%h %cd %s' --grep='MCP' --grep='mcp' --all-match
git log rust-v0.142.0 --reverse --date=short --format='%h %cd %s' -- <mcp-paths>
git log rust-v0.141.0^{}..rust-v0.142.0^{} --reverse --date=short --format='%h %cd %s' --grep='MCP|mcp|plugin|Plugin|connector|Apps|tool search|elicitation|OAuth|catalog' --extended-regexp
git log rust-v0.142.0^{}..rust-v0.142.5^{} --reverse --date=short --format='%h %cd %s' --grep='MCP|mcp|plugin|Plugin|connector|Apps|tool search|elicitation|OAuth|catalog' --extended-regexp
git log rust-v0.142.5^{}..rust-v0.144.4^{} --reverse --date=short --format='%H%x09%ad%x09%s'
git log rust-v0.142.5^{}..rust-v0.144.4^{} --reverse --date=short --format='%H%x09%ad%x09%s' -- <mcp-paths>
git merge-base --is-ancestor <sha> rust-v0.143.0^{}
git merge-base --is-ancestor <sha> rust-v0.144.0^{}
git merge-base --is-ancestor <sha> rust-v0.144.1^{}
git merge-base --is-ancestor <sha> rust-v0.144.4^{}
```

Release range проверен командами:

```bash
git tag --contains 147a940449839b116b220b7e7d016d2a2890c134 | rg '^rust-v0\.[0-9]+\.0$' | sort -V | head -10
git tag -l 'rust-v0.*.0' | rg '^rust-v0\.[0-9]+\.0$' | sort -V | tail -10
```

Набор source paths для targeted history scan. Включены исторические пути ранней реализации, даже если они уже удалены или перенесены в current HEAD; current source-of-truth отдельно указан в `architecture.md`.

```bash
git log rust-v0.142.0 --reverse --date=short --format='%h %cd %s' -- \
  codex-rs/codex-mcp \
  codex-rs/rmcp-client \
  codex-rs/core/src/mcp.rs \
  codex-rs/core/src/mcp_tool_call.rs \
  codex-rs/core/src/mcp_connection_manager.rs \
  codex-rs/core/src/mcp_server_config.rs \
  codex-rs/core/src/tools/handlers/mcp.rs \
  codex-rs/core/src/tools/handlers/mcp_resource.rs \
  codex-rs/protocol/src/mcp.rs \
  codex-rs/config/src/mcp_types.rs \
  codex-rs/app-server/src/request_processors/mcp_processor.rs \
  codex-rs/tui/src/chatwidget/mcp_startup.rs \
  codex-rs/tui/src/history_cell/mcp.rs \
  codex-rs/ext/extension-api/src/contributors/mcp.rs \
  codex-rs/ext/mcp/src
```

## Release triage evidence

### `EVID-002-initial-config`

- Included commits: `147a940449839b116b220b7e7d016d2a2890c134`, `27198bfe11`.
- First containing stable tag: `rust-v0.2.0` for both entries.
- Commands: `git show --quiet --format='%H %cI %s' 147a940449839b116b220b7e7d016d2a2890c134`, `git show --stat --name-only --oneline 147a940449839b116b220b7e7d016d2a2890c134`, `git tag --contains 147a940449839b116b220b7e7d016d2a2890c134 | rg '^rust-v0\.[0-9]+\.0$' | sort -V | head -1`.
- Evidence notes: commit body for `#829` names `mcp_connection_manager.rs` and `mcp_tool_call.rs` as primary implementation and states that all configured MCP servers were initially launched on each Codex run.

### `EVID-031-040-timeouts`

- Included commits: `6efb52e545` (`rust-v0.31.0`), `19f46439ae` (`rust-v0.40.0`).
- Commands: `git show --stat --name-only --oneline 6efb52e545 19f46439ae`, plus stable containment via `git tag --contains`.
- Evidence notes: subjects and path diffs identify separate startup and tool-call timeout work for external MCP server lifecycle.

### `EVID-042-048-rmcp-http-oauth-resources`

- Included commits: `e555a36c6a` (`rust-v0.42.0`), `3a1be084f9` (`rust-v0.43.0`), `1d17ca1fa3` (`rust-v0.45.0`), `40fba1bb4c` (`rust-v0.47.0`), `740b4a95f4` (`rust-v0.48.0`), `4cd6b01494` (`rust-v0.48.0`).
- Commands: `git show --stat --name-only --oneline <sha>` for each commit and targeted patches under `codex-rs/rmcp-client`, `codex-rs/config/src/mcp_types.rs`, `codex-rs/core/src/mcp_tool_call.rs`, and MCP resource handlers where present.
- Evidence notes: this group establishes the transition from custom stdio-only behavior to RMCP-backed stdio/HTTP, OAuth credentials, resource listing/read and tool allow/deny configuration.

### `EVID-059-067-startup-elicitation-login`

- Included commits: `03ffe4d595` (`rust-v0.59.0`), `7561a6aaf0` (`rust-v0.64.0`), `893f5261eb` (`rust-v0.67.0`).
- Commands: `git show --stat --name-only --oneline 03ffe4d595 7561a6aaf0 893f5261eb`, plus stable containment checks.
- Evidence notes: diffs show startup status moving out of blocking startup, MCP elicitation support entering client/runtime paths, and login becoming available during an active session.

### `EVID-081-099-requirements-approvals-cleanup`

- Included commits: `2651980bdf` (`rust-v0.81.0`), `34f89b12d0` (`rust-v0.93.0`), `82c981cafc` (`rust-v0.99.0`).
- Commands: `git show --stat --name-only --oneline 2651980bdf 34f89b12d0 82c981cafc`.
- Evidence notes: current config still exposes `McpServerDisabledReason`, per-server/per-tool approval modes and stdio process cleanup tests; this group is security/runtime boundary evidence rather than UI-only evidence.

### `EVID-111-119-appserver-codex-mcp-apps`

- Included commits: `926b2f19e8` (`rust-v0.111.0`), `59b68f5519` (`rust-v0.119.0`), `5fe9ef06ce` (`rust-v0.119.0`), `d7f99b0fa6` (`rust-v0.119.0`), `7b6486a145` (`rust-v0.119.0`).
- Commands: `git show --stat --name-only --oneline 926b2f19e8 59b68f5519 5fe9ef06ce d7f99b0fa6 7b6486a145`.
- Evidence notes: current HEAD confirms this as durable architecture: `codex-rs/codex-mcp`, app-server v2 MCP elicitation types, Apps MCP support and `McpToolExposure` deferred tool behavior remain present.

### `EVID-122-124-environments-resources-hooks`

- Included commits: `9c6d038622` (`rust-v0.122.0`), `b4be3617f9` (`rust-v0.122.0`), `996aa23e4c` (`rust-v0.122.0`), `1132ef887c` (`rust-v0.123.0`), `305825abd9` (`rust-v0.124.0`), `ff22982d75` (`rust-v0.124.0`).
- Commands: `git show --stat --name-only --oneline <sha>` and targeted source inspection of `codex-rs/codex-mcp/src/runtime.rs`, `codex-rs/app-server/src/request_processors/mcp_processor.rs`, `codex-rs/ext/extension-api/src/contributors/mcp.rs`, `codex-rs/core/src/tools/handlers/mcp.rs`.
- Evidence notes: current `McpRuntimeContext` validates local/remote environment placement; app-server threadless resource reads explicitly document their fallback rules; hooks and sandbox state include MCP-specific metadata.

### `EVID-126-135-import-items-runtime`

- Included commits: `cb8b1bbcd6` (`rust-v0.126.0`), `c8c30d9d75` (`rust-v0.129.0`), `b2268999fe` (`rust-v0.130.0`), `298e5cfce1` (`rust-v0.134.0`), `ff7513cd83` (`rust-v0.135.0`).
- Commands: `git show --stat --name-only --oneline cb8b1bbcd6 c8c30d9d75 b2268999fe 298e5cfce1 ff7513cd83`.
- Evidence notes: external-agent import includes MCP config detection/import; MCP tool calls become first-class turn items; built-in MCPs and naming mode move into common runtime/manager paths.

### `EVID-140-141-catalog-selected-plugins`

- Included commits: `4a5a676499` (`rust-v0.140.0`), `693082f3c4` (`rust-v0.140.0`), `b3c423e475` (`rust-v0.141.0`), `c8c78b63a7` (`rust-v0.141.0`).
- Commands: `git show --stat --name-only --oneline 4a5a676499 693082f3c4 b3c423e475 c8c78b63a7`, `rg -n "McpServerRegistration|McpServerContribution|selected plugin|executor plugin" codex-rs/codex-mcp/src codex-rs/ext codex-rs/app-server/src`.
- Evidence notes: current source confirms catalog precedence, thread-scoped contribution APIs and selected executor plugin MCP activation in app-server/runtime paths.

### `EVID-142-mcp-files-env`

- Диапазон: `rust-v0.141.0^{}..rust-v0.142.0^{}`.
- Included commits: `7baf7e467e9bd8a34772c14a1fd5edbe9039bea1`; supporting environment/cwd context: `f8850cab1d0f192a799122ff96cb27061b9366eb`.
- Commands: `git show --stat --name-only --oneline 7baf7e467e f8850cab1d`, selected-path inspection under `codex-rs/core/src/mcp_openai_file.rs`, `codex-rs/codex-api/src/files.rs`, `codex-rs/app-server-protocol/src/protocol/v2/turn.rs` and generated turn/thread environment params.
- Evidence notes: file upload routing moved through environment-owned filesystem paths, with target-native cwd preservation as adjacent app-server environment support.

### `EVID-142-plugin-manifests`

- Диапазон: `rust-v0.141.0^{}..rust-v0.142.0^{}`.
- Included commits: `1883dedc0e3499c8f42e08835540319ad7131d77`, `e12dd73b7d5a2aa2b8d0933a2053e7eb5eba6fbb`, `a760b63f838db94369f91b829a366c76f4761107`, `772c5c51952a8bd279dbeeedfea69a6feb837a1d`.
- Commands: `git show --stat --name-only --oneline 1883dedc0e e12dd73b7d a760b63f83 772c5c5195`, selected-path inspection under `codex-rs/core-plugins/src/{loader,manifest,marketplace,store}.rs`, `codex-rs/plugin/src/manifest.rs` and `codex-rs/ext/mcp/src/executor_plugin/provider*.rs`.
- Evidence notes: diffs establish object-valued plugin MCP manifests, manifest path lists, root local marketplace plugins and marketplace manifest fallback as plugin declaration input shapes before MCP catalog registration.

### `EVID-142-mcp-runtime-security`

- Диапазон: `rust-v0.141.0^{}..rust-v0.142.0^{}`.
- Included commits: `790213ded0588d824e99f830f41f04f3d98196df`, `21a599fa56472a7cea8132c5a47a4374d4d5aa17`, `765309d5a611ea02be842ead0ab1a2828196fae9`, `29eb434bc5fd81f29540446f6989219736b09a80`, `4e6bc4226658b7bc2ba4e207506b51f8d6d3626f`, `81b000421dc795062019f3737db6fac3fda16aa4`.
- Commands: `git show --stat --name-only --oneline 790213ded0 21a599fa56 765309d5a6 29eb434bc5 4e6bc42266 81b000421d`, selected-path inspection under `codex-rs/codex-mcp`, `codex-rs/rmcp-client/src/auth_status.rs`, `codex-rs/core/src/mcp_tool_call.rs`, `codex-rs/core/src/tools/handlers/mcp_resource/*`, `codex-rs/app-server-protocol/src/protocol/v2/mcp.rs` and app-server MCP tests.
- Evidence notes: diffs scope sandbox metadata to the MCP server environment, add extended `openai/form` elicitations, project trusted MCP App identity onto tool-call items, remove hardcoded app-id filters, add protected-resource OAuth discovery and expose config toggles for orchestrator skills/MCP resources.
- Excluded near-misses: plugin analytics/cache/warmup/skills-only commits such as `7e735b59ce`, `a34da3b295`, `3959ab0ffc`, `2c7802e7cf`, `c73296a0f0`, `a52a3b5197`, `e83b7841b0`, `7e37354a58`, `38c96866f0`, `44dbae90eb`, `a72433d560`, `0318381762`, `0065c3a17d`, `64bdeed9f7`, `a57b268d61`, `c2fbf4247` and `38211a3ff8` were reviewed as adjacent plugin/skills/performance work but not selected because they do not directly change MCP registration, auth, tool/resource exposure, elicitation, sandbox or manifest parsing contracts.

### `EVID-142.2-tool-search-remote-stdio-cwd`

- Диапазон: `rust-v0.142.0^{}..rust-v0.142.5^{}`.
- Release verification: `git show -s --format='%H %cI %s' rust-v0.142.2^{}`, `git rev-parse --short rust-v0.142.0^{}`, `git rev-parse --short rust-v0.142.5^{}`, `git rev-list --count rust-v0.142.0^{}..rust-v0.142.5^{}`.
- Included commits: `c53b1dae09db40902c59f6a0d57d0dcc334926db`, `67009bc53ffc684eaf235c6186df890d35259240`; supporting context: `11fab432be5a873fea97688ef67e8af896f5ea8c`.
- Commands: `git show --stat --name-only --oneline c53b1dae09 67009bc53f 11fab432be`, selected-path inspection under `codex-rs/core/src/mcp_tool_exposure.rs`, `codex-rs/core/src/tools/spec_plan.rs`, `codex-rs/features/src/lib.rs`, `codex-rs/config/src/mcp_types.rs`, `codex-rs/codex-mcp/src/runtime.rs`, `codex-rs/rmcp-client/src/stdio_server_launcher.rs`, `codex-rs/rmcp-client/tests/foreign_stdio_cwd.rs` and `codex-rs/utils/path-uri/src/api_path_string.rs`.
- Evidence notes: `#29486` makes searched MCP tool flow default when supported and removes the old feature-gated/100-tool threshold behavior; `#29493` removes host-native absolute-path validation for remote stdio `cwd`, stores config cwd as `LegacyAppPathString`, and forwards foreign absolute paths to executor launch as `PathUri`.
- Excluded near-misses: `bf7148626b` remote plugin featured IDs, `2c351cb864` dark-mode plugin logo metadata and `fbd575ab4a` TUI plugin catalog coverage affect plugin UI/catalog surfaces but do not change external MCP registration/runtime/tool exposure contracts selected here. `rust-v0.142.5` WebSocket trace backport changes request logging, not MCP behavior/API.

### `EVID-143-runtime-snapshots-refresh-elicitations`

- Диапазон: `rust-v0.142.5^{}..rust-v0.143.0^{}`.
- Включённые коммиты: `8751fd3fcb8031d42b62670a6131872074635c9b` (`#29608`), `df1ee09ec50453da3976d239da6cb035403ff28f` (`#29724`), `ee9e0f6387b91cd14008ea59a8e4315b2a2fdacb` (`#30101`), `3095ea9c3d155bfc89197d2628eb818a55c2755d` (`#30093`), `fb8598df3ff05366a295c407c91f3d9316a1630c` (`#30127`), `6d2168f06ae275d5e1f73cabf935d2bcc8549998` (`#30148`), `84fe70c30e88a1828a510804341202bab629d507` (`#30627`).
- Команды: `git show --stat --name-only --oneline 8751fd3fcb df1ee09ec5 ee9e0f6387 3095ea9c3d fb8598df3f 6d2168f06a 84fe70c30e`; targeted patches в `codex-rs/core/src/session/{mcp,mcp_runtime,step_context}.rs`, `codex-rs/core/src/elicitation.rs`, `codex-rs/core/src/state/service.rs` и `codex-rs/codex-mcp/src/{connection_manager,elicitation}.rs`.
- Выводы: `#30101` устанавливает `McpRuntimeSnapshot` как точные config/manager/runtime context, зафиксированные `StepContext`; `#30093` проецирует declarations выбранного plugin через capability roots, готовые для этого шага; `#30148` не заменяет manager, если winning MCP/connector projection не изменилась. `#30127` добавляет общий для thread elicitation router с opaque token через замену runtime, а `#30627` — считающий session-owned `ElicitationService`. `#29608` является более ранним прямым исправлением очистки managers после refresh; текущий source сохраняет более сильный поздний invariant: in-flight step snapshots валидны до освобождения их последнего handle.

### `EVID-143-selected-http-oauth-paths`

- Диапазон: `rust-v0.142.5^{}..rust-v0.143.0^{}`.
- Включённые коммиты: `2e69966cd8fbb3f0265152af778dd5512c66d151` (`#28918`), `3e39e92f03431a83d004d9ecdb90d8c82fc24d02` (`#29628`), `6368937939dceb07b4a3c47c4448027d0d1a85a6` (`#28522`), `b215961a56b2553a4612a22a812747403c08a58b` (`#28529`); поддерживающий end-to-end test `c38b2e9ba69cb57d197c6e5ba78b5e52ae0870f9` (`#29656`) и prerequisite RMCP `bbe100689073815876d330e56a200217a96a9e35` (`#29634`).
- Команды: `git show --stat --name-only --oneline 2e69966cd8 3e39e92f03 6368937939 b215961a56 c38b2e9ba6 bbe1006890`; targeted patches в `codex-rs/ext/mcp/src/executor_plugin`, `codex-rs/codex-mcp/src/{plugin_config,runtime,mcp/auth}.rs`, `codex-rs/rmcp-client`, app-server v2 MCP protocol и `app-server/tests/suite/v2/executor_mcp.rs`.
- Выводы: selected roots и relative MCP cwd остаются `PathUri`-native для filesystem executor. Streamable HTTP declarations больше не отбрасываются; их HTTP traffic, OAuth discovery, dynamic registration, PKCE exchange и refresh используют владеющий executor. `mcpServer/oauth/login` получает optional `threadId`, чтобы app-server выбрал этот runtime, а `#29656` завершает executor-only token exchange, а не ограничивается проверкой discovery.

### `EVID-143-auth-policy-sandbox-diagnostics`

- Диапазон: `rust-v0.142.5^{}..rust-v0.143.0^{}`.
- Включённые коммиты: `d2484697b1f9ce33d1d818ccad859ca3a4d721c6` (`#29358`), `4c0706e24a290f89c40d99b92912367c63fe273d` (`#29733`), `f8937b7d862e3d507fe22918e5daa35c7da993e9` (`#29924`), `db541f45536f18741b50c74b05904a2c5f628c96` (`#29648`), `a6d20ed29701bd8f291e94526154dcac90367d92` (`#29877`), `526f495f3a904e29ec1d07a927992c904c8a1f5c` (`#30257`), `b5866eebd631a62a1f3f29ac5e44752e38949c73` (`#29516`). Операционная диагностика: `cbcf1f8ca33ea6f683d11232ae485b32d23028c2` (`#28976`), `322e33512b2d38d38d705e2ef692a8aca50decac` (`#28630`), `cef5444a80ac5a94d435ab780fba5d6f433c504f` (`#29969`).
- Команды: `git show --stat --name-only --oneline d2484697b1 4c0706e24a f8937b7d86 db541f4553 a6d20ed297 526f495f3a b5866eebd6 cbcf1f8ca3 322e33512b cef5444a80`; targeted patches в `config/src/{mcp_types,mcp_requirements}.rs`, `codex-mcp/src/{connection_manager,runtime,mcp/auth}.rs`, `rmcp-client/src/{startup_error,auth_status}.rs`, `core/src/mcp_tool_call*`, app-server v2 MCP schemas и sandbox CLI tests.
- Выводы: `#29924` заменяет временный boolean на exhaustive `McpServerAuth::{OAuth, ChatGpt}` и ограничивает передачу ChatGPT credentials hardcoded trusted origin; configured authorization сохраняет приоритет. `#29648` добавляет managed matchers exact/prefix/full-regex без изменения same-name policy activation. `#29877` и `#30257` проводят типизированное startup recovery `reauthenticationRequired` через nested transport errors. `#29358` требует явный permission profile, когда `codex sandbox` использует MCP sandbox metadata. `#29516` намеренно ограничен allowlisted ChatGPT Cloudflare affinity cookies и не является общим хранением cookies для third-party MCP.

### `EVID-144-approvals-oauth-apps-consistency`

- Диапазон: `rust-v0.143.0^{}..rust-v0.144.0^{}`.
- Включённые коммиты: `6cf42cf16516ab3a125d853561ae3b8c77d6c13e` (`#30292`), `ff06ab7172f4a442e3cc0314b1178f8ea3c5ccac` (`#28772`), `f6e251c3ac6573a4d6eccddea6ad12587868485f` (`#30482`), `48cf58233190ce6f2aa6c3194d022c0e273e8fd6` (`#31612`), `bdaad6820cd884ea11787477f4c495e4de0a8be5` (`#31292`), `a09a7c41d8abbaec6543664bff04a81058192958` (`#31330`), `555aa79d5ab4c011196a1f9fd08c6e6673dd0907` (`#31486`).
- Команды: `git show --stat --name-only --oneline 6cf42cf165 ff06ab7172 f6e251c3ac 48cf582331 bdaad6820c a09a7c41d8 555aa79d5a`; targeted patches в `rmcp-client/src/oauth{,/store_lock}.rs`, `features/src/lib.rs`, `config/src/mcp_types.rs`, `core/src/{mcp_tool_call,mcp_openai_file}.rs`, `core/src/session/{step_context,world_state}.rs`, `codex-mcp/src/connection_manager.rs`, `model-provider/src/auth.rs` и generated config/app-server schemas.
- Выводы: aggregate OAuth stores File/Secrets получают ограниченную межпроцессную блокировку, поэтому конкурентные updates разных MCP servers не перезаписывают друг друга. Auth elicitation становится stable/default. `AppToolApproval::Writes` запрашивает approval, если нет `readOnlyHint = true`, и запрещает повторно используемые approvals. Ленивый per-step tool snapshot сохраняет согласованность Apps World State и построения router в одном sampling request. Hosted Apps file payloads больше не включают internal fields. Только зарезервированный `codex_apps` использует привязанный к identity provider `AuthManager`, который видит token refresh той же account на каждом request; прямые MCP registrations сохраняют configured auth.

### `EVID-144.1-no-mcp-delta`

- Диапазон: `rust-v0.144.0^{}..rust-v0.144.1^{}`.
- Команды: `git log --reverse --date=short --format='%H%x09%ad%x09%s' rust-v0.144.0^{}..rust-v0.144.1^{}`, `git diff --stat rust-v0.144.0^{}..rust-v0.144.1^{}`.
- Выводы: patch release содержит только parsing installer metadata и fallback/backport для code-mode host (`7ef1728763`, `9d47eb221d`, `77c42a202a`, release commit `44918ea10c`). Исходники MCP client/runtime/config/wire не изменены, поэтому для `rust-v0.144.1` зафиксирована явная секция без material MCP changes, а не выведенные по косвенным признакам feature entries.

### `EVID-144.2-144.4-no-mcp-delta`

- Диапазоны: `rust-v0.144.1^{}..rust-v0.144.2^{}`, `rust-v0.144.2^{}..rust-v0.144.3^{}`, `rust-v0.144.3^{}..rust-v0.144.4^{}`; каждая boundary имеет topology left/right `1/2`, где left-only — предыдущий release wrapper, а right-only — один functional commit и новый release wrapper.
- Release anchors: `a6645b6b8a656360fa16fb7e1c6721d0697d3d6a`, `78ad6e6bfd1d3b6a209acd3ef82172a96b25179c`, `8c68d4c87dc54d38861f5114e920c3de2efa5876`. Каждый меняет только версию в `codex-rs/Cargo.toml`.
- Проверенные functional commits: `32649bc5e6591ad8ea0b7b8ce073df447565ec7c` откатывает Guardian prompting/tool planning; `8a4d35a1e100efc2c64b72515668a84da663f067` меняет Advanced Reasoning UX и thread resume metadata; `d82b7e5d4c1c274bee0eb55f92ec12d017e78634` добавляет Guardian policy в model catalog.
- Targeted diff всех external-MCP source-of-truth paths — `codex-mcp`, `rmcp-client`, MCP config/requirements, session runtime snapshots, tool/resource handlers, OAuth, app-server MCP protocol/processors/refresh и TUI MCP projections — пуст.
- Guardian near-miss: rollback `32649bc5e6` меняет общий `effective_tool_mode`/`add_tool_sources`, но derived Guardian config по-прежнему очищает `mcp_servers` и отключает Apps/Plugins/Memories. Поэтому возврат к общему native tool planning не открывает внешний MCP runtime в Guardian session и не меняет MCP exposure contract.
- Вывод: архитектура external MCP client/runtime на `rust-v0.144.1` остаётся действующей на `rust-v0.144.4`; patch-релизы не добавляют material MCP feature, contract или runtime delta.

### Исключённые plugin/catalog near-misses в `0.143`-`0.144`

- Проверены, но исключены из прямой эволюции MCP: remote plugin catalog/default/npm/source-policy work `6509f3148a` (`#29375`), `9dbdb4e2c0` (`#29691`), `e428a12d22` (`#30297`), `d206a5d68f` (`#30981`), TUI catalog polish `a0d5fd772e` (`#26705`), namespace/skill guidance work `9c5be7e1d5` (`#30223`), `42156ba007` (`#31369`), `58b66d39e4` (`#31348`) и plugin install policy/logging `c71895f63b`, `1ee0e9a949`.
- Основание включения/исключения: эти commits могут влиять на обнаружение, установку или описание plugin, а обнаруженный plugin позднее может добавить MCP declarations, но проверенные diffs не меняют parsing MCP manifest, resolution registration/catalog, transport, OAuth, runtime projection, экспозицию tool/resource или MCP app-server wire contracts. Напротив, `#28522`, `#28529`, `#30093` и `#30148` включены, потому что используют selected plugin MCP declarations в реальном thread/runtime path.

## `rust-v0.144.4` source inspection

- Контракт конфигурации: `codex-rs/config/src/mcp_types.rs` определяет `McpServerConfig`, `McpServerAuth`, `AppToolApproval`, варианты transport, environment id, таймауты запуска/tools, approvals, tool filters и OAuth config; `codex-rs/config/src/mcp_requirements.rs` определяет managed MCP matchers.
- Runtime/catalog: `codex-rs/codex-mcp/src/catalog.rs`, `server.rs`, `mcp/mod.rs`, `runtime.rs`, `connection_manager.rs` владеют precedence registrations, effective servers, разрешением runtime environment и запущенными clients. `codex-rs/core/src/session/mcp_runtime.rs`, `mcp.rs` и `step_context.rs` владеют step-pinned runtime и tool snapshots на один sampling request.
- Tool/resource exposure: `codex-rs/core/src/tools/handlers/mcp.rs`, `codex-rs/core/src/mcp_tool_call.rs`, `codex-rs/core/src/mcp_tool_exposure.rs`, `codex-rs/core/src/tools/handlers/mcp_resource.rs`.
- Согласованность OAuth/elicitation: `codex-rs/rmcp-client/src/oauth/store_lock.rs`, `codex-rs/core/src/elicitation.rs`, `codex-rs/codex-mcp/src/elicitation.rs`, `codex-rs/model-provider/src/auth.rs`.
- App-server API: `codex-rs/app-server-protocol/src/protocol/v2/mcp.rs`, `codex-rs/app-server/src/request_processors/mcp_processor.rs`, `codex-rs/app-server/src/mcp_refresh.rs`.
- TUI and UI surfaces: `codex-rs/tui/src/chatwidget/mcp_startup.rs`, `codex-rs/tui/src/history_cell/mcp.rs`, `codex-rs/tui/src/bottom_pane/mcp_server_elicitation.rs`.
- Extension/plugin surfaces: `codex-rs/ext/extension-api/src/contributors/mcp.rs`, `codex-rs/ext/mcp/src/executor_plugin.rs`, `codex-rs/codex-mcp/src/plugin_config.rs`.

## `rust-v0.145.0` final-source inspection

- Runtime ownership split: `codex-rs/codex-mcp/src/runtime.rs` владеет thread-owned live `McpRuntime`; `codex-rs/core/src/session/mcp_runtime.rs` — request/step-pinned `McpRuntimeSnapshot`; `codex-rs/core/src/mcp.rs`/`McpManager` — process-scoped `McpToolCatalogCache`.
- Cache safety: `codex-rs/codex-mcp/src/tool_catalog_cache.rs` удаляет connection-specific instructions/annotations из cached entries; `codex-rs/codex-mcp/src/rmcp_client.rs` может вернуть cached catalog до завершения startup, а `connection_manager.rs` требует соответствующий live tool для `tool_info` и вызова.
- OAuth lifecycle: `codex-rs/rmcp-client/src/oauth/resolved_store.rs` владеет lifecycle pinning credential store, `oauth/refresh_lock.rs` — межпроцессной сериализацией refresh, `oauth/refresh_transaction.rs` — authoritative read-refresh-write transaction; `oauth/store_lock.rs` остаётся блокировкой aggregate File/Secrets stores.
- Проверка выполнена по final tree `rust-v0.145.0^{}` и тематическим diffs: `git show rust-v0.145.0^{}:<path>`, `git show --patch <sha> -- <path>` и `rg -n 'McpRuntime|McpRuntimeSnapshot|McpToolCatalogCache|ResolvedStore|refresh_transaction' <paths>`.

## Evidence gaps

- This pass did not inspect GitHub PR pages or external MCP specification pages; rationale is based on local commit subjects/bodies/diffs and current source.
- Compact timeline intentionally omits stable releases without material external-MCP changes; release coverage is provided through the command log and stable containment checks rather than no-change sections.

## `rust-v0.145.0`: тематические evidence groups

- `EVID-145-runtime-ownership-cache`: `2f7d89b141`, `1447cee36b`, `42c5d3c80d`, `3307ea8b63`, `1bbdb32789`, `79177c3e20`, `f24e695470`, `19940967bd`; thread-owned `McpRuntime`, snapshot/cache coherence, step-pinned runtime и availability-aware reuse.
- `EVID-145-startup-transport-oauth`: `6ced1ac5eb`, `cbdee7976b`, `6962a2ecae`, `8b2c84ddcc`, `2e156cbe31`, `44954d1b4b`; bounded remote stdio lines, pinned/serialized OAuth stores and refresh, startup timeout, non-blocking optional discovery и serialized stdin writes.
- `EVID-145-hosted-apps-files-routing`: `23a09eb3c3`, `b58952b0fa`, `0e19d5e908`, `8347b8de21`, `8604689ec5`, `2b486b4676`, `6bf4845b60`; Apps SKU/originator, HTTP-client file routing, schema-aware file payloads, Apps tool-cache injection, docs attribution и plugin-service endpoint.
- `EVID-145-content-wire-approval`: `800715d201`, `cbc83d961e`, `643de86a19`, `e52c35b000`; removal of template IDs, encrypted content preservation, audio outputs и approval rejection reasons.

Проверка новой boundary:

```bash
git rev-list --left-right --count 'rust-v0.144.4^{}...rust-v0.145.0^{}'
git merge-base 'rust-v0.144.4^{}' 'rust-v0.145.0^{}'
git log --no-merges --reverse --format='%H%x09%cI%x09%s' 'rust-v0.144.4^{}..rust-v0.145.0^{}' -- codex-rs/codex-mcp codex-rs/rmcp-client codex-rs/core/src/mcp codex-rs/app-server codex-rs/app-server-protocol codex-rs/tui codex-rs/ext
```
