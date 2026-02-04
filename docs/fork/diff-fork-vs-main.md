# Diff fork/colab-agents vs main (v0.95.0)
> Owner: <team/owner> | Scope: fork vs main | Audience: devs
> Status: active | Last reviewed: 2026-02-04 | Related: docs/fork/colab-agents.md, docs/fork/upstream-main-commits.md
## Цель и методология
Проанализировать все расхождения между `fork/colab-agents` и `main` (v0.95.0),
дать понятные комментарии и рекомендации по синергии с upstream.
В анализе используется сравнение `git diff main...fork/colab-agents`.

## Сводка
- Изменённых файлов: **787**
- Коммитов только в fork: **91**
- Коммитов только в main: **75**
- Основные темы расхождений: форк‑логика multi‑agents + collab,
  большой блок forward‑портов из upstream, генерация протокольных схем.

## Executive summary

- **Ключевая fork‑ценность**: агентский реестр (YAML+Markdown), интеграция реестра в `collab`/`spawn_agent`,
  единый флаг `fn_multi_agents`, вспомогательные tools `list_agents/read_agent`, allow/deny‑пересечения.
- **Upstream‑forward‑портов больше, чем fork‑каста**: основная масса diff — это переносы upstream‑коммитов
  (особенно `app-server-protocol` schemas/fixtures, rmcp‑миграция, TUI улучшения).
- **Наиболее рискованные зоны конфликтов**: `codex-rs/core`, `codex-rs/tui`, `app-server-protocol`.
- **Главный риск**: возможный дрейф версии (держать fork и main на одной версии) и ручной дрейф схем.
- **Рекомендация**: держать fork‑функциональность тонким слоем над upstream,
  любые upstream‑правки затягивать целыми коммитами и перегенерировать схемы.

## Fork‑специфичные файлы (ядро функциональности)

Это список **основных** файлов, которые формируют уникальную функциональность fork. Эти изменения
желательно удерживать даже при максимальной каноничности к upstream.

### Агентский реестр и инструменты
- `codex-rs/core/src/agent/registry.rs` — загрузка YAML‑агентов + правила валидации.
- `codex-rs/core/src/agent/mod.rs` — подключение реестра и экспорт API.
- `codex-rs/core/src/agent/role.rs` — сохранён legacy‑слой для совместимости.
- `codex-rs/core/templates/agents/codex_*.md` — встроенные агентские шаблоны.
- `codex-rs/core/src/tools/handlers/agents.rs` — `list_agents`/`read_agent`.
- `codex-rs/core/src/tools/spec/agent_tools.rs` — спецификация инструментов агента.
- `codex-rs/core/src/tool_allowlist.rs` — пересечение allow/deny на уровне агента.
- `codex-rs/core/src/tools/handlers/collab.rs` — `spawn_agent` через реестр (agent_type/agent_name).
- `codex-rs/core/src/tools/spec.rs` — расширение `spawn_agent` schema + allow/deny фильтрация tools.
- `codex-rs/core/src/turn_metadata.rs` — fork‑расширения метаданных для agent/collab.

### Feature‑флаги и конфигурация
- `codex-rs/core/src/features.rs` — флаг `fn_multi_agents`.
- `codex-rs/core/src/features/legacy.rs` — legacy‑алиас `collab`.
- `codex-rs/core/src/config/mod.rs` — `tool_allowlist`/`tool_denylist` + связка с реестром.
- `codex-rs/core/config.schema.json` — схемы конфигурации fork‑фичей.

### Fork‑брендинг и локальные сборки
- `codex-rs/tui/src/cli.rs`
- `codex-rs/tui/src/history_cell.rs`
- `codex-rs/tui/src/status/card.rs`
- `codex-rs/tui/src/update_prompt.rs`
- `codex-rs/tui/src/chatwidget.rs` + снапшоты
- `scripts/codex-fork-build.sh`

### Документация форка
- `docs/fork/colab-agents.md`
- `docs/fork/upstream-main-commits.md`
- `docs/fork/commit-03fcd12e7.md`
- `docs/config.md` (раздел про agents registry)
- `codex-rs/AGENTS.md` (процесс интеграции upstream)

## План синхронизации с upstream (высокоуровнево)

1) **Базовая ветка**: считать `main` (v0.95.0) источником канона.
2) **Форк‑слой**: удерживать fork‑фичи как минимальный слой поверх upstream.
3) **Процесс**: commit‑by‑commit, как закреплено в `AGENTS.md`.
4) **Генерация**: любые изменения в `app-server-protocol` должны сопровождаться
   генерацией fixtures/схем, без ручного редактирования JSON/TS файлов.
5) **Версионность**: синхронизировать `workspace.version` с фактическим base‑релизом.
6) **Тесты**: минимум `cargo test -p codex-core` + `cargo test -p codex-tui` при затронутом UI.
7) **Результат**: fork остаётся каноничным, но с сохранённой multi‑agent функциональностью.

## Легенда статусов
- `A` = added, `M` = modified, `D` = deleted

## Анализ по категориям

### Репозиторные метаданные и процесс
Что это: шаблоны issue/CI, корневые инструкции, базовые сборочные файлы.
Почему разошлось: fork подтянул несколько upstream‑правок по шаблонам/CI и добавил собственный workflow‑раздел в `AGENTS.md`.
Риск: низкий. Это мета‑изменения, они не ломают runtime.
Рекомендация: держать в fork (полезно для процесса). В `main` переносить только вместе с обновлением upstream, иначе держать неизменным.

Файлы:
- `.github/ISSUE_TEMPLATE/2-bug-report.yml` (M)
- `.github/ISSUE_TEMPLATE/4-feature-request.yml` (M)
- `.github/workflows/issue-labeler.yml` (M)
- `.github/workflows/rust-ci.yml` (M)
- `.gitignore` (M)
- `AGENTS.md` (M)
- `defs.bzl` (M)
- `justfile` (M)

### Скрипты форка
Что это: локальный helper для сборки форка.
Риск: низкий, но относится только к fork.
Рекомендация: в `fork/colab-agents` оставить как tracked файл; в `main` — игнорировать, чтобы не смешивать форк‑инструменты с каноничной веткой.

Файлы:
- `scripts/codex-fork-build.sh` (A)

### Документация форка
Что это: описания форк‑изменений и процесс интеграции upstream.
Риск: низкий.
Рекомендация: оставить в fork. В `main` не переносить (это не каноничный upstream‑док).

Файлы:
- `docs/fork/colab-agents.md` (A)
- `docs/fork/commit-03fcd12e7.md` (A)
- `docs/fork/upstream-main-commits.md` (A)

### Продуктовая документация (не‑fork)
Что это: общие docs, часть из которых — upstream‑обновления, часть — fork‑расширения (например, `docs/config.md` про `fn_multi_agents`).
Риск: средний (док может расходиться с фактическим поведением при дальнейшем ребейзе).
Рекомендация: держать `docs/config.md` синхронным с реальным поведением форка; остальные изменения лучше подтягивать из upstream при обновлении базы.

Файлы:
- `docs/config.md` (M)
- `docs/contributing.md` (M)
- `docs/tui-chat-composer.md` (M)

### codex-rs: корневые файлы workspace
Что это: `Cargo.toml` (workspace версии/члены), `Cargo.lock`, `codex-rs/AGENTS.md`.
Почему разошлось: fork содержит forward‑ports из upstream (удаление `mcp-types`, добавление `codex-experimental-api-macros`, новые зависимости) и собственный процессный файл.
Риск: средний — влияет на сборку и состав крейтов.
Рекомендация: если цель — каноничность, выравнивать по upstream и обновлять форк на его базе; версию workspace держать консистентно с реальным релизом/тегом (сейчас main и fork выровнены на v0.95.0).

Файлы:
- `codex-rs/AGENTS.md` (A)
- `codex-rs/Cargo.lock` (M)
- `codex-rs/Cargo.toml` (M)

### app-server-protocol: схемы и fixtures (генерация)
Что это: массово добавленные JSON/TS схемы + tooling для их генерации. Это forward‑port upstream‑коммита о vendor‑fixtures.
Риск: средний — большие диффы и вероятность дрейфа при ручных правках.
Рекомендация: не править вручную. При обновлениях выполнять генерацию (например, `just write-app-server-schema`) и синхронизировать с upstream. Для `main` — переносить целиком вместе с upstream‑коммитом, иначе нечастично.

Файлы:
- `codex-rs/app-server-protocol/BUILD.bazel` (M)
- `codex-rs/app-server-protocol/Cargo.toml` (M)
- `codex-rs/app-server-protocol/schema/json/ApplyPatchApprovalParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/ApplyPatchApprovalResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/ChatgptAuthTokensRefreshParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/ChatgptAuthTokensRefreshResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/ClientNotification.json` (A)
- `codex-rs/app-server-protocol/schema/json/ClientRequest.json` (A)
- `codex-rs/app-server-protocol/schema/json/CommandExecutionRequestApprovalParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/CommandExecutionRequestApprovalResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/DynamicToolCallParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/DynamicToolCallResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/EventMsg.json` (A)
- `codex-rs/app-server-protocol/schema/json/ExecCommandApprovalParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/ExecCommandApprovalResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/FileChangeRequestApprovalParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/FileChangeRequestApprovalResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/FuzzyFileSearchParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/FuzzyFileSearchResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/JSONRPCError.json` (A)
- `codex-rs/app-server-protocol/schema/json/JSONRPCErrorError.json` (A)
- `codex-rs/app-server-protocol/schema/json/JSONRPCMessage.json` (A)
- `codex-rs/app-server-protocol/schema/json/JSONRPCNotification.json` (A)
- `codex-rs/app-server-protocol/schema/json/JSONRPCRequest.json` (A)
- `codex-rs/app-server-protocol/schema/json/JSONRPCResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/RequestId.json` (A)
- `codex-rs/app-server-protocol/schema/json/ServerNotification.json` (A)
- `codex-rs/app-server-protocol/schema/json/ServerRequest.json` (A)
- `codex-rs/app-server-protocol/schema/json/ToolRequestUserInputParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/ToolRequestUserInputResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/codex_app_server_protocol.schemas.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/AddConversationListenerParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/AddConversationSubscriptionResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/ArchiveConversationParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/ArchiveConversationResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/AuthStatusChangeNotification.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/CancelLoginChatGptParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/CancelLoginChatGptResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/ExecOneOffCommandParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/ExecOneOffCommandResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/ForkConversationParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/ForkConversationResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/GetAuthStatusParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/GetAuthStatusResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/GetConversationSummaryParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/GetConversationSummaryResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/GetUserAgentResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/GetUserSavedConfigResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/GitDiffToRemoteParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/GitDiffToRemoteResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/InitializeParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/InitializeResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/InterruptConversationParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/InterruptConversationResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/ListConversationsParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/ListConversationsResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/LoginApiKeyParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/LoginApiKeyResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/LoginChatGptCompleteNotification.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/LoginChatGptResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/LogoutChatGptResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/NewConversationParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/NewConversationResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/RemoveConversationListenerParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/RemoveConversationSubscriptionResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/ResumeConversationParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/ResumeConversationResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/SendUserMessageParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/SendUserMessageResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/SendUserTurnParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/SendUserTurnResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/SessionConfiguredNotification.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/SetDefaultModelParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/SetDefaultModelResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v1/UserInfoResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/AccountLoginCompletedNotification.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/AccountRateLimitsUpdatedNotification.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/AccountUpdatedNotification.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/AgentMessageDeltaNotification.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/AppsListParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/AppsListResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/CancelLoginAccountParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/CancelLoginAccountResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/CommandExecParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/CommandExecResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/CommandExecutionOutputDeltaNotification.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ConfigBatchWriteParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ConfigReadParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ConfigReadResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ConfigRequirementsReadResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ConfigValueWriteParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ConfigWarningNotification.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ConfigWriteResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ContextCompactedNotification.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/DeprecationNoticeNotification.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ErrorNotification.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/FeedbackUploadParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/FeedbackUploadResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/FileChangeOutputDeltaNotification.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/GetAccountParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/GetAccountRateLimitsResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/GetAccountResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ItemCompletedNotification.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ItemStartedNotification.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ListMcpServerStatusParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ListMcpServerStatusResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/LoginAccountParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/LoginAccountResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/LogoutAccountResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/McpServerOauthLoginCompletedNotification.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/McpServerOauthLoginParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/McpServerOauthLoginResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/McpServerRefreshResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/McpToolCallProgressNotification.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ModelListParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ModelListResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/PlanDeltaNotification.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/RawResponseItemCompletedNotification.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ReasoningSummaryPartAddedNotification.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ReasoningSummaryTextDeltaNotification.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ReasoningTextDeltaNotification.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ReviewStartParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ReviewStartResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/SkillsConfigWriteParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/SkillsConfigWriteResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/SkillsListParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/SkillsListResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/TerminalInteractionNotification.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ThreadArchiveParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ThreadArchiveResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ThreadForkParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ThreadForkResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ThreadListParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ThreadListResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ThreadLoadedListParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ThreadLoadedListResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ThreadNameUpdatedNotification.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ThreadReadParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ThreadReadResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ThreadResumeParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ThreadResumeResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ThreadRollbackParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ThreadRollbackResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ThreadSetNameParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ThreadSetNameResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ThreadStartParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ThreadStartResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ThreadStartedNotification.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ThreadTokenUsageUpdatedNotification.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ThreadUnarchiveParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/ThreadUnarchiveResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/TurnCompletedNotification.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/TurnDiffUpdatedNotification.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/TurnInterruptParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/TurnInterruptResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/TurnPlanUpdatedNotification.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/TurnStartParams.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/TurnStartResponse.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/TurnStartedNotification.json` (A)
- `codex-rs/app-server-protocol/schema/json/v2/WindowsWorldWritableWarningNotification.json` (A)
- `codex-rs/app-server-protocol/schema/typescript/AbsolutePathBuf.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/AddConversationListenerParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/AddConversationSubscriptionResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/AgentMessageContent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/AgentMessageContentDeltaEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/AgentMessageDeltaEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/AgentMessageEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/AgentMessageItem.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/AgentReasoningDeltaEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/AgentReasoningEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/AgentReasoningRawContentDeltaEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/AgentReasoningRawContentEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/AgentReasoningSectionBreakEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/AgentStatus.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ApplyPatchApprovalParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ApplyPatchApprovalRequestEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ApplyPatchApprovalResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ArchiveConversationParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ArchiveConversationResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/AskForApproval.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/AuthMode.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/AuthStatusChangeNotification.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/BackgroundEventEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ByteRange.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/CallToolResult.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/CancelLoginChatGptParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/CancelLoginChatGptResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ClientInfo.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ClientNotification.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ClientRequest.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/CodexErrorInfo.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/CollabAgentInteractionBeginEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/CollabAgentInteractionEndEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/CollabAgentSpawnBeginEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/CollabAgentSpawnEndEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/CollabCloseBeginEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/CollabCloseEndEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/CollabWaitingBeginEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/CollabWaitingEndEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/CollaborationMode.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/CollaborationModeMask.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ContentItem.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ContextCompactedEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ContextCompactionItem.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ConversationGitInfo.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ConversationSummary.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/CreditsSnapshot.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/CustomPrompt.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/DeprecationNoticeEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/DynamicToolCallRequest.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ElicitationRequestEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ErrorEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/EventMsg.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ExecApprovalRequestEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ExecCommandApprovalParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ExecCommandApprovalResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ExecCommandBeginEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ExecCommandEndEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ExecCommandOutputDeltaEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ExecCommandSource.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ExecOneOffCommandParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ExecOneOffCommandResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ExecOutputStream.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ExecPolicyAmendment.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ExitedReviewModeEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/FileChange.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ForcedLoginMethod.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ForkConversationParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ForkConversationResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/FunctionCallOutputContentItem.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/FunctionCallOutputPayload.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/FuzzyFileSearchParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/FuzzyFileSearchResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/FuzzyFileSearchResult.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/GetAuthStatusParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/GetAuthStatusResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/GetConversationSummaryParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/GetConversationSummaryResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/GetHistoryEntryResponseEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/GetUserAgentResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/GetUserSavedConfigResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/GhostCommit.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/GitDiffToRemoteParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/GitDiffToRemoteResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/GitSha.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/HistoryEntry.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/InitializeCapabilities.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/InitializeParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/InitializeResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/InputItem.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/InputModality.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/InterruptConversationParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/InterruptConversationResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ItemCompletedEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ItemStartedEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ListConversationsParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ListConversationsResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ListCustomPromptsResponseEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ListSkillsResponseEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/LocalShellAction.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/LocalShellExecAction.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/LocalShellStatus.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/LoginApiKeyParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/LoginApiKeyResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/LoginChatGptCompleteNotification.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/LoginChatGptResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/LogoutChatGptResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/McpAuthStatus.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/McpInvocation.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/McpListToolsResponseEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/McpStartupCompleteEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/McpStartupFailure.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/McpStartupStatus.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/McpStartupUpdateEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/McpToolCallBeginEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/McpToolCallEndEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/MessagePhase.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ModeKind.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/NetworkAccess.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/NewConversationParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/NewConversationResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ParsedCommand.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/PatchApplyBeginEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/PatchApplyEndEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/Personality.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/PlanDeltaEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/PlanItem.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/PlanItemArg.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/PlanType.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/Profile.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/RateLimitSnapshot.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/RateLimitWindow.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/RawResponseItemEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ReasoningContentDeltaEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ReasoningEffort.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ReasoningItem.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ReasoningItemContent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ReasoningItemReasoningSummary.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ReasoningRawContentDeltaEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ReasoningSummary.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/RemoveConversationListenerParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/RemoveConversationSubscriptionResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/RequestId.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/RequestUserInputEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/RequestUserInputQuestion.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/RequestUserInputQuestionOption.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/Resource.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ResourceTemplate.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ResponseItem.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ResumeConversationParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ResumeConversationResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ReviewCodeLocation.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ReviewDecision.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ReviewFinding.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ReviewLineRange.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ReviewOutputEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ReviewRequest.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ReviewTarget.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/SandboxMode.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/SandboxPolicy.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/SandboxSettings.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/SendUserMessageParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/SendUserMessageResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/SendUserTurnParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/SendUserTurnResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ServerNotification.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ServerRequest.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/SessionConfiguredEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/SessionConfiguredNotification.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/SessionSource.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/SetDefaultModelParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/SetDefaultModelResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/Settings.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/SkillDependencies.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/SkillErrorInfo.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/SkillInterface.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/SkillMetadata.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/SkillScope.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/SkillToolDependency.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/SkillsListEntry.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/StepStatus.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/StreamErrorEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/SubAgentSource.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/TerminalInteractionEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/TextElement.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ThreadId.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ThreadNameUpdatedEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ThreadRolledBackEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/TokenCountEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/TokenUsage.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/TokenUsageInfo.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/Tool.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/Tools.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/TurnAbortReason.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/TurnAbortedEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/TurnCompleteEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/TurnDiffEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/TurnItem.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/TurnStartedEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/UndoCompletedEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/UndoStartedEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/UpdatePlanArgs.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/UserInfoResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/UserInput.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/UserMessageEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/UserMessageItem.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/UserSavedConfig.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/Verbosity.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/ViewImageToolCallEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/WarningEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/WebSearchAction.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/WebSearchBeginEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/WebSearchEndEvent.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/WebSearchItem.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/WebSearchMode.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/index.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/serde_json/JsonValue.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/Account.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/AccountLoginCompletedNotification.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/AccountRateLimitsUpdatedNotification.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/AccountUpdatedNotification.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/AgentMessageDeltaNotification.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/AnalyticsConfig.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/AppInfo.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/AppsListParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/AppsListResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/AskForApproval.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ByteRange.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/CancelLoginAccountParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/CancelLoginAccountResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/CancelLoginAccountStatus.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ChatgptAuthTokensRefreshParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ChatgptAuthTokensRefreshReason.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ChatgptAuthTokensRefreshResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/CodexErrorInfo.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/CollabAgentState.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/CollabAgentStatus.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/CollabAgentTool.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/CollabAgentToolCallStatus.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/CommandAction.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/CommandExecParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/CommandExecResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/CommandExecutionApprovalDecision.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/CommandExecutionOutputDeltaNotification.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/CommandExecutionRequestApprovalParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/CommandExecutionRequestApprovalResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/CommandExecutionStatus.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/Config.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ConfigBatchWriteParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ConfigEdit.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ConfigLayer.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ConfigLayerMetadata.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ConfigLayerSource.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ConfigReadParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ConfigReadResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ConfigRequirements.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ConfigRequirementsReadResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ConfigValueWriteParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ConfigWarningNotification.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ConfigWriteResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ContextCompactedNotification.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/CreditsSnapshot.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/DeprecationNoticeNotification.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/DynamicToolCallParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/DynamicToolCallResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/DynamicToolSpec.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ErrorNotification.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ExecPolicyAmendment.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/FeedbackUploadParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/FeedbackUploadResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/FileChangeApprovalDecision.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/FileChangeOutputDeltaNotification.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/FileChangeRequestApprovalParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/FileChangeRequestApprovalResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/FileUpdateChange.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/GetAccountParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/GetAccountRateLimitsResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/GetAccountResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/GitInfo.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ItemCompletedNotification.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ItemStartedNotification.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ListMcpServerStatusParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ListMcpServerStatusResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/LoginAccountParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/LoginAccountResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/LogoutAccountResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/McpAuthStatus.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/McpServerOauthLoginCompletedNotification.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/McpServerOauthLoginParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/McpServerOauthLoginResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/McpServerRefreshResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/McpServerStatus.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/McpToolCallError.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/McpToolCallProgressNotification.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/McpToolCallResult.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/McpToolCallStatus.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/MergeStrategy.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/Model.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ModelListParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ModelListResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/NetworkAccess.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/OverriddenMetadata.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/PatchApplyStatus.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/PatchChangeKind.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/PlanDeltaNotification.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ProfileV2.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/RateLimitSnapshot.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/RateLimitWindow.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/RawResponseItemCompletedNotification.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ReasoningEffortOption.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ReasoningSummaryPartAddedNotification.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ReasoningSummaryTextDeltaNotification.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ReasoningTextDeltaNotification.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ResidencyRequirement.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ReviewDelivery.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ReviewStartParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ReviewStartResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ReviewTarget.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/SandboxMode.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/SandboxPolicy.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/SandboxWorkspaceWrite.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/SessionSource.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/SkillDependencies.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/SkillErrorInfo.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/SkillInterface.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/SkillMetadata.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/SkillScope.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/SkillToolDependency.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/SkillsConfigWriteParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/SkillsConfigWriteResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/SkillsListEntry.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/SkillsListParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/SkillsListResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/TerminalInteractionNotification.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/TextElement.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/TextPosition.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/TextRange.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/Thread.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ThreadArchiveParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ThreadArchiveResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ThreadForkParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ThreadForkResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ThreadItem.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ThreadListParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ThreadListResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ThreadLoadedListParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ThreadLoadedListResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ThreadNameUpdatedNotification.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ThreadReadParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ThreadReadResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ThreadResumeParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ThreadResumeResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ThreadRollbackParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ThreadRollbackResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ThreadSetNameParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ThreadSetNameResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ThreadSortKey.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ThreadSourceKind.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ThreadStartParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ThreadStartResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ThreadStartedNotification.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ThreadTokenUsage.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ThreadTokenUsageUpdatedNotification.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ThreadUnarchiveParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ThreadUnarchiveResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/TokenUsageBreakdown.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ToolRequestUserInputAnswer.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ToolRequestUserInputOption.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ToolRequestUserInputParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ToolRequestUserInputQuestion.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ToolRequestUserInputResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/ToolsV2.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/Turn.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/TurnCompletedNotification.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/TurnDiffUpdatedNotification.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/TurnError.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/TurnInterruptParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/TurnInterruptResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/TurnPlanStep.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/TurnPlanStepStatus.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/TurnPlanUpdatedNotification.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/TurnStartParams.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/TurnStartResponse.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/TurnStartedNotification.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/TurnStatus.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/UserInput.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/WindowsWorldWritableWarningNotification.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/WriteStatus.ts` (A)
- `codex-rs/app-server-protocol/schema/typescript/v2/index.ts` (A)
- `codex-rs/app-server-protocol/src/bin/write_schema_fixtures.rs` (A)
- `codex-rs/app-server-protocol/src/experimental_api.rs` (A)
- `codex-rs/app-server-protocol/src/export.rs` (M)
- `codex-rs/app-server-protocol/src/lib.rs` (M)
- `codex-rs/app-server-protocol/src/protocol/common.rs` (M)
- `codex-rs/app-server-protocol/src/protocol/v1.rs` (M)
- `codex-rs/app-server-protocol/src/protocol/v2.rs` (M)
- `codex-rs/app-server-protocol/src/schema_fixtures.rs` (A)
- `codex-rs/app-server-protocol/tests/schema_fixtures.rs` (A)

### core: логика агента, конфиг, инструменты, модели (mixed)
Что это: самая крупная зона пересечения fork‑логики и upstream‑изменений.
Fork‑специфика: агентский реестр (YAML+Markdown), `fn_multi_agents`, интеграция реестра в collab‑tools, allow/deny‑пересечения.
Upstream‑часть: многочисленные улучшения core (skills, модельные метаданные, compaction, git info, rollout и т.д.).
Риск: высокий — частые конфликты при ребейзе.
Рекомендация: держать fork‑добавки, но максимально накладывать их поверх upstream‑поведения (canonical‑first). Любые upstream‑обновления сюда интегрировать аккуратно и с тестами.

Файлы:
- `codex-rs/core/Cargo.toml` (M)
- `codex-rs/core/build.rs` (A)
- `codex-rs/core/config.schema.json` (M)
- `codex-rs/core/src/agent/mod.rs` (M)
- `codex-rs/core/src/agent/registry.rs` (A)
- `codex-rs/core/src/agent/role.rs` (M)
- `codex-rs/core/src/client.rs` (M)
- `codex-rs/core/src/codex.rs` (M)
- `codex-rs/core/src/command_safety/is_dangerous_command.rs` (M)
- `codex-rs/core/src/command_safety/is_safe_command.rs` (M)
- `codex-rs/core/src/compact.rs` (M)
- `codex-rs/core/src/compact_remote.rs` (M)
- `codex-rs/core/src/config/mod.rs` (M)
- `codex-rs/core/src/context_manager/history.rs` (M)
- `codex-rs/core/src/context_manager/history_tests.rs` (M)
- `codex-rs/core/src/context_manager/mod.rs` (M)
- `codex-rs/core/src/default_client.rs` (M)
- `codex-rs/core/src/environment_context.rs` (M)
- `codex-rs/core/src/event_mapping.rs` (M)
- `codex-rs/core/src/exec_policy.rs` (M)
- `codex-rs/core/src/features.rs` (M)
- `codex-rs/core/src/features/legacy.rs` (M)
- `codex-rs/core/src/git_info.rs` (M)
- `codex-rs/core/src/instructions/user_instructions.rs` (M)
- `codex-rs/core/src/lib.rs` (M)
- `codex-rs/core/src/mcp/mod.rs` (M)
- `codex-rs/core/src/mcp_connection_manager.rs` (M)
- `codex-rs/core/src/mcp_tool_call.rs` (M)
- `codex-rs/core/src/models_manager/cache.rs` (M)
- `codex-rs/core/src/models_manager/manager.rs` (M)
- `codex-rs/core/src/models_manager/mod.rs` (M)
- `codex-rs/core/src/models_manager/model_info.rs` (M)
- `codex-rs/core/src/models_manager/model_presets.rs` (M)
- `codex-rs/core/src/rollout/list.rs` (M)
- `codex-rs/core/src/rollout/metadata.rs` (M)
- `codex-rs/core/src/rollout/session_index.rs` (M)
- `codex-rs/core/src/rollout/tests.rs` (M)
- `codex-rs/core/src/rollout/truncation.rs` (M)
- `codex-rs/core/src/session_prefix.rs` (M)
- `codex-rs/core/src/shell_snapshot.rs` (M)
- `codex-rs/core/src/skills/loader.rs` (M)
- `codex-rs/core/src/skills/system.rs` (M)
- `codex-rs/core/src/state_db.rs` (M)
- `codex-rs/core/src/stream_events_utils.rs` (M)
- `codex-rs/core/src/tasks/mod.rs` (M)
- `codex-rs/core/src/tasks/review.rs` (M)
- `codex-rs/core/src/tasks/user_shell.rs` (M)
- `codex-rs/core/src/thread_manager.rs` (M)
- `codex-rs/core/src/tool_allowlist.rs` (A)
- `codex-rs/core/src/tools/context.rs` (M)
- `codex-rs/core/src/tools/handlers/agents.rs` (A)
- `codex-rs/core/src/tools/handlers/collab.rs` (M)
- `codex-rs/core/src/tools/handlers/mcp_resource.rs` (M)
- `codex-rs/core/src/tools/handlers/mod.rs` (M)
- `codex-rs/core/src/tools/handlers/plan.rs` (M)
- `codex-rs/core/src/tools/registry.rs` (M)
- `codex-rs/core/src/tools/router.rs` (M)
- `codex-rs/core/src/tools/spec.rs` (M)
- `codex-rs/core/src/tools/spec/agent_tools.rs` (A)
- `codex-rs/core/src/turn_metadata.rs` (A)
- `codex-rs/core/src/user_shell_command.rs` (M)
- `codex-rs/core/src/windows_sandbox.rs` (M)
- `codex-rs/core/templates/agents/codex_architect.md` (A)
- `codex-rs/core/templates/agents/codex_bug-hunter.md` (A)
- `codex-rs/core/templates/agents/codex_explorer.md` (A)
- `codex-rs/core/templates/agents/codex_orchestrator.md` (A)
- `codex-rs/core/templates/agents/codex_reviewer.md` (A)
- `codex-rs/core/templates/agents/codex_worker.md` (A)
- `codex-rs/core/templates/agents/orchestrator.md` (M)
- `codex-rs/core/templates/collaboration_mode/plan.md` (M)
- `codex-rs/core/templates/model_instructions/gpt-5.2-codex_instructions_template.md` (M)
- `codex-rs/core/tests/chat_completions_payload.rs` (M)
- `codex-rs/core/tests/chat_completions_sse.rs` (M)
- `codex-rs/core/tests/common/lib.rs` (M)
- `codex-rs/core/tests/responses_headers.rs` (M)
- `codex-rs/core/tests/suite/client.rs` (M)
- `codex-rs/core/tests/suite/client_websockets.rs` (M)
- `codex-rs/core/tests/suite/collaboration_instructions.rs` (M)
- `codex-rs/core/tests/suite/compact.rs` (M)
- `codex-rs/core/tests/suite/compact_remote.rs` (M)
- `codex-rs/core/tests/suite/deprecation_notice.rs` (M)
- `codex-rs/core/tests/suite/image_rollout.rs` (M)
- `codex-rs/core/tests/suite/list_models.rs` (M)
- `codex-rs/core/tests/suite/models_cache_ttl.rs` (M)
- `codex-rs/core/tests/suite/override_updates.rs` (M)
- `codex-rs/core/tests/suite/permissions_messages.rs` (M)
- `codex-rs/core/tests/suite/personality.rs` (M)
- `codex-rs/core/tests/suite/prompt_caching.rs` (M)
- `codex-rs/core/tests/suite/remote_models.rs` (M)
- `codex-rs/core/tests/suite/review.rs` (M)
- `codex-rs/core/tests/suite/rmcp_client.rs` (M)
- `codex-rs/core/tests/suite/sqlite_state.rs` (M)

### tui: интерфейс (mixed)
Что это: обновления UX (upstream) + fork‑брендинг версии `FN`.
Риск: средний — UI‑снапшоты и поведение тестов часто ломаются при ребейзах.
Рекомендация: сохранить fork‑маркер версии, но стараться оставаться в рамках upstream UX. Если upstream предложит официальный fork‑маркер, заменить на него.

Файлы:
- `codex-rs/tui/Cargo.toml` (M)
- `codex-rs/tui/src/app.rs` (M)
- `codex-rs/tui/src/bottom_pane/approval_overlay.rs` (M)
- `codex-rs/tui/src/bottom_pane/chat_composer.rs` (M)
- `codex-rs/tui/src/bottom_pane/experimental_features_view.rs` (M)
- `codex-rs/tui/src/bottom_pane/mod.rs` (M)
- `codex-rs/tui/src/bottom_pane/request_user_input/mod.rs` (M)
- `codex-rs/tui/src/bottom_pane/textarea.rs` (M)
- `codex-rs/tui/src/chatwidget.rs` (M)
- `codex-rs/tui/src/chatwidget/snapshots/codex_tui__chatwidget__tests__binary_size_ideal_response.snap` (M)
- `codex-rs/tui/src/chatwidget/snapshots/codex_tui__chatwidget__tests__experimental_features_popup.snap` (M)
- `codex-rs/tui/src/chatwidget/tests.rs` (M)
- `codex-rs/tui/src/cli.rs` (M)
- `codex-rs/tui/src/history_cell.rs` (M)
- `codex-rs/tui/src/lib.rs` (M)
- `codex-rs/tui/src/resume_picker.rs` (M)
- `codex-rs/tui/src/slash_command.rs` (M)
- `codex-rs/tui/src/snapshots/codex_tui__resume_picker__tests__resume_picker_thread_names.snap` (A)
- `codex-rs/tui/src/snapshots/codex_tui__update_prompt__tests__update_prompt_modal.snap` (M)
- `codex-rs/tui/src/status/card.rs` (M)
- `codex-rs/tui/src/status/snapshots/codex_tui__status__tests__status_snapshot_cached_limits_hide_credits_without_flag.snap` (M)
- `codex-rs/tui/src/status/snapshots/codex_tui__status__tests__status_snapshot_includes_credits_and_limits.snap` (M)
- `codex-rs/tui/src/status/snapshots/codex_tui__status__tests__status_snapshot_includes_forked_from.snap` (M)
- `codex-rs/tui/src/status/snapshots/codex_tui__status__tests__status_snapshot_includes_monthly_limit.snap` (M)
- `codex-rs/tui/src/status/snapshots/codex_tui__status__tests__status_snapshot_includes_reasoning_details.snap` (M)
- `codex-rs/tui/src/status/snapshots/codex_tui__status__tests__status_snapshot_shows_empty_limits_message.snap` (M)
- `codex-rs/tui/src/status/snapshots/codex_tui__status__tests__status_snapshot_shows_missing_limits_message.snap` (M)
- `codex-rs/tui/src/status/snapshots/codex_tui__status__tests__status_snapshot_shows_stale_limits_message.snap` (M)
- `codex-rs/tui/src/status/snapshots/codex_tui__status__tests__status_snapshot_truncates_in_narrow_terminal.snap` (M)
- `codex-rs/tui/src/tooltips.rs` (M)
- `codex-rs/tui/src/update_prompt.rs` (M)
- `codex-rs/tui/tooltips.txt` (M)

### app-server: сервер и тесты (upstream forward‑ports)
Что это: forward‑ports upstream (experimental API flags, fixtures, исправления тестов).
Риск: средний — изменение протокола/схем.
Рекомендация: переносить вместе с соответствующими schema‑обновлениями; не дробить.

Файлы:
- `codex-rs/app-server/Cargo.toml` (M)
- `codex-rs/app-server/README.md` (M)
- `codex-rs/app-server/src/bespoke_event_handling.rs` (M)
- `codex-rs/app-server/src/codex_message_processor.rs` (M)
- `codex-rs/app-server/src/message_processor.rs` (M)
- `codex-rs/app-server/src/models.rs` (M)
- `codex-rs/app-server/tests/common/mcp_process.rs` (M)
- `codex-rs/app-server/tests/common/models_cache.rs` (M)
- `codex-rs/app-server/tests/common/rollout.rs` (M)
- `codex-rs/app-server/tests/suite/list_resume.rs` (M)
- `codex-rs/app-server/tests/suite/user_agent.rs` (M)
- `codex-rs/app-server/tests/suite/v2/compaction.rs` (M)
- `codex-rs/app-server/tests/suite/v2/experimental_api.rs` (A)
- `codex-rs/app-server/tests/suite/v2/mod.rs` (M)
- `codex-rs/app-server/tests/suite/v2/model_list.rs` (M)
- `codex-rs/app-server/tests/suite/v2/thread_list.rs` (M)
- `codex-rs/app-server/tests/suite/v2/thread_read.rs` (M)
- `codex-rs/app-server/tests/suite/v2/thread_resume.rs` (M)
- `codex-rs/app-server/tests/suite/v2/thread_unarchive.rs` (M)

### mcp-server: адаптация под rmcp и approvals (upstream)
Что это: upstream‑изменения после миграции на rmcp и расширений approvals.
Риск: средний.
Рекомендация: держать в fork как часть upstream‑ветки; для main — только в составе обновления базы.

Файлы:
- `codex-rs/mcp-server/Cargo.toml` (M)
- `codex-rs/mcp-server/src/codex_tool_config.rs` (M)
- `codex-rs/mcp-server/src/codex_tool_runner.rs` (M)
- `codex-rs/mcp-server/src/error_code.rs` (D)
- `codex-rs/mcp-server/src/exec_approval.rs` (M)
- `codex-rs/mcp-server/src/lib.rs` (M)
- `codex-rs/mcp-server/src/message_processor.rs` (M)
- `codex-rs/mcp-server/src/outgoing_message.rs` (M)
- `codex-rs/mcp-server/src/patch_approval.rs` (M)
- `codex-rs/mcp-server/tests/common/Cargo.toml` (M)
- `codex-rs/mcp-server/tests/common/lib.rs` (M)
- `codex-rs/mcp-server/tests/common/mcp_process.rs` (M)
- `codex-rs/mcp-server/tests/suite/codex_tool.rs` (M)

### mcp-types: удаление legacy‑крейта (upstream)
Что это: удаление deprecated `mcp-types`.
Риск: низкий при наличии `rmcp`.
Рекомендация: держать удаление (это каноничное направление upstream).

Файлы:
- `codex-rs/mcp-types/BUILD.bazel` (D)
- `codex-rs/mcp-types/Cargo.toml` (D)
- `codex-rs/mcp-types/README.md` (D)
- `codex-rs/mcp-types/check_lib_rs.py` (D)
- `codex-rs/mcp-types/generate_mcp_types.py` (D)
- `codex-rs/mcp-types/schema/2025-03-26/schema.json` (D)
- `codex-rs/mcp-types/schema/2025-06-18/schema.json` (D)
- `codex-rs/mcp-types/src/lib.rs` (D)
- `codex-rs/mcp-types/tests/all.rs` (D)
- `codex-rs/mcp-types/tests/suite/initialize.rs` (D)
- `codex-rs/mcp-types/tests/suite/mod.rs` (D)
- `codex-rs/mcp-types/tests/suite/progress_notification.rs` (D)

### protocol: новые типы MCP + модели (upstream)
Что это: upstream‑изменения типов и моделей.
Риск: низкий‑средний (может требовать синхронизации с app‑server‑protocol).
Рекомендация: переносить вместе с протокольными схемами.

Файлы:
- `codex-rs/protocol/Cargo.toml` (M)
- `codex-rs/protocol/src/approvals.rs` (M)
- `codex-rs/protocol/src/lib.rs` (M)
- `codex-rs/protocol/src/mcp.rs` (A)
- `codex-rs/protocol/src/models.rs` (M)
- `codex-rs/protocol/src/openai_models.rs` (M)
- `codex-rs/protocol/src/protocol.rs` (M)

### codex-api: ответы/стримы (upstream)
Что это: upstream‑изменения в API‑клиенте.
Риск: средний.
Рекомендация: переносить вместе с обновлениями моделей/stream‑events.

Файлы:
- `codex-rs/codex-api/src/endpoint/chat.rs` (M)
- `codex-rs/codex-api/src/requests/chat.rs` (M)
- `codex-rs/codex-api/src/requests/responses.rs` (M)
- `codex-rs/codex-api/src/sse/chat.rs` (M)
- `codex-rs/codex-api/src/sse/responses.rs` (M)
- `codex-rs/codex-api/tests/clients.rs` (M)
- `codex-rs/codex-api/tests/models_integration.rs` (M)

### cli: macOS app launcher + обновления (upstream)
Что это: upstream‑фича `codex app` + небольшие правки.
Риск: низкий.
Рекомендация: можно оставить в fork; в main переносить при обновлении базы.

Файлы:
- `codex-rs/cli/Cargo.toml` (M)
- `codex-rs/cli/src/app_cmd.rs` (A)
- `codex-rs/cli/src/desktop_app/mac.rs` (A)
- `codex-rs/cli/src/desktop_app/mod.rs` (A)
- `codex-rs/cli/src/main.rs` (M)

### rmcp-client: адаптации (upstream)
Что это: upstream‑адаптация под rmcp.
Риск: низкий‑средний.
Рекомендация: держать синхронно с rmcp‑миграцией.

Файлы:
- `codex-rs/rmcp-client/Cargo.toml` (M)
- `codex-rs/rmcp-client/src/logging_client_handler.rs` (M)
- `codex-rs/rmcp-client/src/rmcp_client.rs` (M)
- `codex-rs/rmcp-client/src/utils.rs` (M)
- `codex-rs/rmcp-client/tests/resources.rs` (M)

### exec: события/вывод (upstream)
Что это: upstream‑изменения форматов событий.
Риск: низкий.
Рекомендация: переносить вместе с протокольными изменениями.

Файлы:
- `codex-rs/exec/Cargo.toml` (M)
- `codex-rs/exec/src/event_processor_with_human_output.rs` (M)
- `codex-rs/exec/src/exec_events.rs` (M)
- `codex-rs/exec/tests/event_processor_with_json_output.rs` (M)

### state: миграции и runtime (upstream)
Что это: upstream‑изменения и миграция `thread_dynamic_tools`.
Риск: средний (миграции).
Рекомендация: переносить только вместе с соответствующими code‑paths; обновлять миграции аккуратно.

Файлы:
- `codex-rs/state/Cargo.toml` (M)
- `codex-rs/state/migrations/0004_thread_dynamic_tools.sql` (A)
- `codex-rs/state/src/extract.rs` (M)
- `codex-rs/state/src/runtime.rs` (M)

### codex-experimental-api-macros: новый крейт (upstream)
Что это: upstream‑добавление крейта для экспериментального API.
Риск: низкий.
Рекомендация: держать как часть upstream‑набора.

Файлы:
- `codex-rs/codex-experimental-api-macros/BUILD.bazel` (A)
- `codex-rs/codex-experimental-api-macros/Cargo.toml` (A)
- `codex-rs/codex-experimental-api-macros/src/lib.rs` (A)

### execpolicy: правки allow‑rules (upstream)
Что это: upstream‑улучшения безопасных правил.
Риск: низкий.
Рекомендация: переносить вместе с тестами.

Файлы:
- `codex-rs/execpolicy/src/amend.rs` (M)
- `codex-rs/execpolicy/tests/basic.rs` (M)

### app-server-test-client: обновления (upstream)
Что это: upstream‑синхронизация тестового клиента.
Риск: низкий.
Рекомендация: держать в составе upstream‑пакета.

Файлы:
- `codex-rs/app-server-test-client/Cargo.lock` (M)
- `codex-rs/app-server-test-client/src/main.rs` (M)

### debug-client: небольшие правки (upstream)
Что это: upstream‑изменения клиента.
Риск: низкий.
Рекомендация: переносить вместе с прочими tool‑обновлениями.

Файлы:
- `codex-rs/debug-client/src/client.rs` (M)

### utils: cargo-bin helper (upstream)
Что это: upstream‑изменение для корректной работы с bin‑путями.
Риск: низкий.
Рекомендация: keep.

Файлы:
- `codex-rs/utils/cargo-bin/src/lib.rs` (M)

### Прочее (windows-sandbox-rs)
Что это: upstream‑правки в Windows sandbox‑подсистеме.
Риск: низкий‑средний.
Рекомендация: keep вместе с upstream обновлениями.

Файлы:
- `codex-rs/windows-sandbox-rs/src/setup_error.rs` (M)
- `codex-rs/windows-sandbox-rs/src/setup_orchestrator.rs` (M)

## Синергия и рекомендации по каноничности
1) **Сводить дрейф к upstream‑логике там, где нет fork‑ценности.**
   Всё, что помечено как upstream‑forward‑ports, лучше переносить целиком,
   чтобы не размножать полурасхождения.
2) **Fork‑ценность держать отдельным слоем.**
   Агентский реестр и `fn_multi_agents` — это ключевая функция форка.
   Рекомендуется держать её в виде тонкой надстройки над upstream‑сценариями.
3) **Ищем аналоги в upstream для замены кастома.**
   Сейчас ближайший функциональный аналог — upstream‑skills (`.agents/skills`).
   Возможная синергия: использовать единые пути/форматы,
   либо связать agent‑registry с системой skills, чтобы не плодить форматы.
4) **Генерируемые артефакты не править вручную.**
   Протокольные схемы и fixtures должны пересобираться из source‑описаний.

## Риски и долговые зоны
- `codex-rs/core` и `codex-rs/tui` — главные конфликтные зоны при ребейзе.
- `app-server-protocol` даёт гигантские диффы, которые сложно ревьюить вручную.
- Несовпадение версии workspace (если fork уйдёт от v0.95.0)
  может ввести в заблуждение при сборке/релизе.
