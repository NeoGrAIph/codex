# Эволюция multi-agent/collab/subagents

Статус исследования: покрыты и зафиксированы stable-релизы `rust-v0.81.0`-`rust-v0.142.5`. Основной timeline строится только по stable tags. Alpha tags используются только как дополнительное evidence, если это потребуется отдельно.

## Поколения

- Старый `collab` / sub-agent tools: ранний feature-gated runtime в `codex-core`, инструменты `spawn_agent`, `send_input`, `wait`, `close_agent`, позднее `resume_agent`, app-server/TUI projections и prompt guidance.
- Collaboration modes и Plan mode: режимы поверх базовой модели/effort/developer instructions, app-server `collaborationMode/list`, TUI переключение режимов и `/plan`.
- `multi_agent_v2`: path-based модель агентов, `AgentPath`, mailbox/message delivery, v2 handlers и новые имена tool surface.
- Model-only/current MAv2 runtime: v2 tools становятся model-only, конфигурируемые namespace/runtime metadata, encrypted payloads, `interrupt_agent`, path-based activity.
- App-server/protocol surfaces: `ThreadItem::CollabAgentToolCall`, `ThreadItem::SubAgentActivity`, `thread/settings/update.collaborationMode`, turn/start collaboration settings, generated schema/TS projections.

## `rust-v0.81.0`

- Release commit: `7e7f828e2ce5d31a61d577bcabb517cca342f8ad`
- Дата: `2026-01-14T09:42:55-08:00`
- Subject: `## New Features - Headless runs now switch to device-code login automatically so sign-in works without a browser. (#8756) - TUI lets you queue messages with Tab during active turns (including /review), shows queue hints, and streams exec output inline for smoother updates. (#9077, #9122, #9138, #9194) - WebSocket transport reuses connections and supports incremental append requests to reduce streaming overhead. (#9127, #9128) - Linux sandbox can mount paths read-only to better protect files from writes. (#9112) - Default API model moved to gpt-5.2-codex. (#9188)`

### `568b938c80a3454a3aa091b4ba20636662dea86b` - `feat: first pass on clb tool (#8930)`

- Дата: `2026-01-09T11:54:05Z`; PR: `#8930`; поколение: старый `collab` / sub-agent tools.
- Developer info: первый явный runtime/tool pass для collab: добавляет feature gate и tool plumbing в `codex-rs/core/src/features.rs`, `codex-rs/core/src/tools/spec.rs`, `codex-rs/core/src/tools/handlers/collab.rs` и интеграцию в core turn/tool path. Source-of-truth на этом этапе - `Feature::Collab` плюс tool spec/handler registration.
- Why: локально установимо из subject и diff как первичная реализация `clb`/collab tool; внешняя PR metadata не проверялась.
- User info: при включенном экспериментальном флаге модель получает первые инструменты управления sub-agent/collab workflow. Поверхность еще experimental/feature-gated.
- Compatibility/risks: ранний API использовал старую naming/model форму; позднее она менялась и не должна считаться current MAv2 contract.
- Evidence: `EVID-081-568b938c80`.

### `86f81ca010` - `feat: testing harness for collab 1 (#8983)`

- Дата: `2026-01-12T11:17:05Z`; PR: `#8983`; поколение: старый `collab` runtime.
- Developer info: добавляет/расширяет test harness и orchestration path для collab chain в `core/src/agent/control.rs`, `core/src/thread_manager.rs`, `core/src/tools/handlers/collab.rs`.
- Why: локально установимо из subject и diff: нужен проверяемый harness для раннего collab flow.
- User info: прямой новый UX не добавляет, но делает ранние `collab` tool flows проверяемыми и устойчивее.
- Compatibility/risks: тестовый слой закрепляет контракт раннего collab, который позже эволюционирует.
- Evidence: `EVID-081-86f81ca010`.

### `623707ab58` - `feat: add wait tool implementation for collab (#9088)`

- Дата: `2026-01-12T12:16:24Z`; PR: `#9088`; поколение: старый `collab` / waiting.
- Developer info: добавляет `wait` tool implementation для ожидания sub-agent/collab статусов; затронуты `core/src/agent/control.rs`, `core/src/agent/status.rs`, `core/src/codex*.rs`, `core/src/tools/handlers/collab.rs`, `protocol/src/protocol.rs`.
- Why: subject и diff показывают добавление missing coordination primitive: root-agent должен иметь native way ждать завершения/ответов sub-agent.
- User info: модель может не только spawn/send, но и ждать результат работы sub-agent вместо ручного polling/догадок.
- Compatibility/risks: вводит protocol/runtime event/status assumptions для ожидания; позднее wait semantics менялись, включая multiple IDs и mailbox.
- Evidence: `EVID-081-623707ab58`.

### `9659583559` - `feat: add close tool implementation for collab (#9090)`

- Дата: `2026-01-12T13:21:46Z`; PR: `#9090`; поколение: старый `collab` / lifecycle.
- Developer info: добавляет `close_agent`/close lifecycle через `core/src/agent/control.rs` и `core/src/tools/handlers/collab.rs`.
- Why: subject и diff показывают закрытие lifecycle gap: созданные collab agents должны управляемо завершаться.
- User info: модель получает способ закрыть/остановить агентную работу через native tool.
- Compatibility/risks: старое имя `close_agent` позже сосуществует с v2 и переименовывается в `interrupt_agent` для MAv2.
- Evidence: `EVID-081-9659583559`.

### `3b8d79ee11` - `chore: better error handling on collab tools (#9143)`

- Дата: `2026-01-13T13:56:11Z`; PR: `#9143`; поколение: старый `collab` stability.
- Developer info: улучшает error handling в `core/src/agent/control.rs`, `core/src/thread_manager.rs`, `core/src/tools/handlers/collab.rs`.
- Why: локально установимо из subject/diff: ранние collab tools нуждались в контролируемых ошибках вместо неясных failure modes.
- User info: ошибки tool calls по агентам становятся предсказуемее для модели и клиента.
- Compatibility/risks: стабилизирует ранний contract без добавления новой user-facing команды.
- Evidence: `EVID-081-3b8d79ee11`.

### `97f1f20edb` - `nit: collab send input cleaning (#9147)`

- Дата: `2026-01-13T15:15:41Z`; PR: `#9147`; поколение: старый `collab` input hygiene.
- Developer info: чистит обработку input для `send_input` в `core/src/tools/handlers/collab.rs`.
- Why: локально установимо из subject/diff; точная product-причина кроме input cleanup не установлена локально.
- User info: меньше риска передавать sub-agent некорректно оформленный prompt/input.
- Compatibility/risks: поведенческая стабилизация, не новый surface.
- Evidence: `EVID-081-97f1f20edb`.

### `acfd94f625c4f75d927c3debc533347bcd19da24` - `Add hierarchical agent prompt (#8996)`

- Дата: `2026-01-09T13:47:37-08:00`; PR: `#8996`; поколение: old sub-agent prompt semantics.
- Developer info: добавляет hierarchical/sub-agent instructions path в `core/src/features.rs`, `core/src/project_doc.rs`, `core/tests/suite/hierarchical_agents.rs`, `docs/agents_md.md`.
- Why: subject и diff показывают внедрение отдельного feature/config пути для child/sub-agent instructions.
- User info: подагенты начинают получать отдельный слой проектных инструкций, а не только root context.
- Compatibility/risks: feature key был переименован в следующем релизе; использовать раннее имя как current contract нельзя.
- Evidence: `EVID-081-acfd94f625`.

## `rust-v0.82.0`

- Release commit: `1324fffe5d620c8b49b0ffc6e6df5969f6bad639`
- Дата: `2026-01-14T15:42:33-08:00`
- Subject: `## New Features - Add a unified WebSearchMode setting (disabled/cached/live) across config, protocol v2, CLI/TUI, and the TypeScript SDK, superseding legacy web-search flags (#9216) - Improve startup UX by queuing input until session configuration/model selection is ready and showing a loading state in the TUIs (#9191) - Add experimental collaboration-tool prompting plus lifecycle events so clients can track tool start/finish (#9208, #9095) - Make tool execution more robust by falling back to a piped process when PTYs aren’t available (#8797)`

### `6a939ed7a46a0cba28b505fd404f1fc17fdfad24` - `feat: emit events around collab tools (#9095)`

- Дата: `2026-01-14T17:55:57Z`; PR: `#9095`; поколение: old collab protocol/runtime events.
- Developer info: добавляет `CollabAgent*Event`, `CollabWaiting*Event`, `CollabClose*Event` в `protocol/src/protocol.rs`, emission в `core/src/tools/handlers/collab.rs`, persistence policy в rollout и обработчики-заглушки/поверхности в exec, MCP server и TUI.
- Why: release note и diff прямо указывают на lifecycle events, чтобы клиенты могли отслеживать start/finish collab tools.
- User info: клиенты начинают видеть lifecycle collab tool calls, а не только финальный текстовый эффект.
- Compatibility/risks: новые protocol events требуют projection support; часть клиентов на этом этапе еще имела TODO handling.
- Evidence: `EVID-082-6a939ed7`.

### `3d322fa9d80cee853df81f3d43c7175210c990ec` - `feat: add collab prompt (#9208)`

- Дата: `2026-01-14T17:58:15Z`; PR: `#9208`; поколение: old collab prompting.
- Developer info: добавляет `core/templates/collab/experimental_prompt.md` и подключает его в `core/src/codex.rs` к base instructions при включенной `Feature::Collab`.
- Why: release note и diff показывают experimental collaboration-tool prompting.
- User info: модель получает встроенные инструкции, как использовать multi-agent/collab tools.
- Compatibility/risks: prompt experimental и позже перерабатывался/сокращался; не является current MAv2 usage hint.
- Evidence: `EVID-082-3d322fa9`.

### `e6d2ef432d6214ceef9ae12c512f3940284f6819` - feature rename to `child_agents_md` (`#9215`)

- Дата: `2026-01-14T11:14:24-08:00`; PR: `#9215`; поколение: sub-agent config/docs compatibility.
- Developer info: переименовывает feature key с `hierarchical_agents` на `child_agents_md`, обновляет `core/config.schema.json`, `core/src/features.rs`, `core/src/project_doc.rs`, tests и `docs/agents_md.md`.
- Why: subject/diff показывают rename; точная мотивация beyond naming clarity не установлена локально.
- User info: меняется конфигурационный ключ для child/sub-agent `AGENTS.md` behavior.
- Compatibility/risks: config compatibility point; при доработках форка нельзя опираться на старый feature key без миграционного поведения.
- Evidence: `EVID-082-e6d2ef43`.

## `rust-v0.83.0`

- Release commit: `5fe0cd19437ef03549ae8c12b9fb74364237378d`
- Дата: `2026-01-14T16:23:08-08:00`
- Subject: `## Bug Fixes - Improve SSE response streaming performance by avoiding redundant per-event cloning (#9238)`
- Результат triage: прямых multi-agent/collab/subagent изменений в диапазоне `rust-v0.82.0^{}..rust-v0.83.0^{}` не найдено. Рассмотренный SSE performance commit `3728db11...` исключен как общий streaming performance, не functional change темы.
- Evidence: `EVID-083-no-change`.

## `rust-v0.84.0`

- Release commit: `d67aa9eaa0c895b62926b6cf9e83f9535fd94d28`
- Дата: `2026-01-14T16:57:30-08:00`
- Subject: `## New Features - Extend the Rust protocol/types to include additional metadata on text elements, enabling richer client rendering and schema evolution (#9235)`
- Результат triage: прямых multi-agent/collab/subagent изменений в диапазоне `rust-v0.83.0^{}..rust-v0.84.0^{}` не найдено. Text element metadata `#9235` исключен как общий protocol/schema evolution без diff-связи с sub-agent control.
- Evidence: `EVID-084-no-change`.

## `rust-v0.85.0`

- Release commit: `4607330eff53e3ae39126afef76882042e14a03c`
- Дата: `2026-01-15T18:04:42Z`
- Subject: `## New Features - App-server v2 now emits collaboration tool calls as item events in the turn stream, so clients can render agent coordination in real time. (#9213) - Collaboration tools gained richer agent control: spawn_agent accepts an agent role preset, and send_input can optionally interrupt a running agent before delivering the message. (#9275, #9276) - The TUI now surfaces collab tool lifecycle events with status and prompt previews in the history view. (#9209) - /models metadata now includes upgrade migration markdown so clients can display richer guidance when suggesting model upgrades. (#9219)`

### `bad4c12b9da59f3d73e9c73e6602004412e0bbc6` - `feat: collab tools app-server event mapping (#9213)`

- Дата: `2026-01-15T09:03:26Z`; PR: `#9213`; поколение: app-server/protocol surfaces for old collab.
- Developer info: добавляет projection `EventMsg::Collab*` в app-server v2 item stream через `ThreadItem::CollabAgentToolCall`, `CollabAgentTool`, `CollabAgentToolCallStatus`, `CollabAgentStatus`, `CollabAgentState`; обновляет app-server docs/tests.
- Why: release note и diff указывают на необходимость рендерить agent coordination в real time.
- User info: app-server clients получают структурированные item events для `spawn_agent`, `send_input`, `wait`, `close_agent`.
- Compatibility/risks: клиенты должны понимать новый item kind; старый текстовый transcript не является достаточным projection.
- Evidence: `EVID-085-bad4c12b`.

### `05b960671dcd3ab062a1214b93447851ea432636` - `feat: add agent roles to collab tools (#9275)`

- Дата: `2026-01-15T13:33:52Z`; PR: `#9275`; поколение: old collab role presets.
- Developer info: добавляет `AgentRole`, `AgentProfile`, role config application и аргумент/schema `agent_type` для `spawn_agent`; затронуты `core/src/agent/role.rs`, `core/src/tools/handlers/collab.rs`, `core/src/tools/spec.rs`, `core/templates/agents/orchestrator.md`.
- Why: release note говорит, что `spawn_agent` принимает role preset; diff показывает перенос role-specific config в native spawn path.
- User info: модель может создавать подагента с ролью (`orchestrator`, `worker`, `default`) вместо универсального агента.
- Compatibility/risks: старые role presets не равны текущим `multi_agent_v2` role/runtime semantics.
- Evidence: `EVID-085-05b96067`.

### `faeb08c1e139590a1e01929d2bf289c2a6c47e2e` - `feat: add interrupt capabilities to send_input (#9276)`

- Дата: `2026-01-15T14:59:07Z`; PR: `#9276`; поколение: old collab runtime control.
- Developer info: добавляет параметр `interrupt` в `send_input`, вызывает `interrupt_agent` перед доставкой нового prompt, обновляет `core/src/tools/handlers/collab.rs`, `core/src/agent/control.rs`, `core/src/tools/spec.rs`.
- Why: release note говорит, что `send_input` can optionally interrupt a running agent; diff подтверждает параметр и runtime path.
- User info: root-agent может отправить follow-up с принудительным прерыванием текущей работы sub-agent.
- Compatibility/risks: later MAv2 separates interruption into `interrupt_agent`; не смешивать старый `send_input interrupt` с current v2 tool shape.
- Evidence: `EVID-085-faeb08c1`.

### `3fc487e0e0788045460df1abe21dc9de47f3b7f9` - `feat: basic tui for event emission (#9209)`

- Дата: `2026-01-15`; PR: `#9209`; поколение: TUI projection for old collab.
- Developer info: добавляет TUI обработку `EventMsg::Collab*`, модуль `tui/src/collab.rs` и rendering функций lifecycle/prompt previews.
- Why: release note и diff показывают user-visible TUI rendering collab lifecycle events.
- User info: TUI history показывает статусы collab tool lifecycle и prompt preview.
- Compatibility/risks: ранний renderer привязан к old collab event names; current TUI также имеет `SubAgentActivity`.
- Evidence: `EVID-085-3fc487e0`.

## `rust-v0.86.0`

- Release commit: `18057fdc30b10168ab18e8c1bb86c7a0a7772191`
- Дата: `2026-01-15T16:46:06-08:00`
- Subject: `## New Features - Skill metadata can now be defined in SKILL.toml (names, descriptions, icons, brand color, default prompt) and surfaced in the app server and TUI (#9125) - Clients can explicitly disable web search and signal eligibility via a header to align with server-side rollout controls (#9249)`

### `393a5a0311b5d19d1c4b0065cc4f952c059f14e4` - `chore: better orchestrator prompt (#9301)`

- Дата: `2026-01-15T18:11:43Z`; PR: `#9301`; поколение: old collab prompt guidance.
- Developer info: обновляет `codex-rs/core/templates/agents/orchestrator.md`, включая workflow по `spawn_agent`, `send_input`, `wait`, `close_agent` и правила координации.
- Why: subject/diff показывают улучшение orchestrator prompt; точная внешняя мотивация не установлена локально.
- User info: меняется guidance, по которому оркестратор выбирает и координирует multi-agent workflow.
- Compatibility/risks: prompt-level behavior может менять модельную стратегию без wire/API change; при форк-доработках такие изменения нужно тестировать через модельно-видимый context.
- Evidence: `EVID-086-393a5a03`.

## `rust-v0.87.0`

- Release commit: `8e8c884b7534d08b8cabccd26f37a8302b4bc4fd`
- Дата: `2026-01-16T16:04:05+01:00`
- Subject: `## New Features - User message metadata (text elements and byte ranges) now round-trips through protocol/app-server/core so UI annotations can survive history rebuilds. (#9331) - Collaboration wait calls can block on multiple IDs in one request, simplifying multi-thread coordination. (#9294) - User shell commands now run under the user snapshot so aliases and shell config are honored. (#9357) - The TUI now surfaces approval requests from spawned/unsubscribed threads. (#9232)`

### `c576756c81ee84036093e7abcb0f0660dd3c2914` - `feat: collab wait multiple IDs (#9294)`

- Дата: `2026-01-16T12:05:04+01:00`; PR: `#9294`; поколение: old collab runtime coordination.
- Developer info: расширяет `wait` так, чтобы один вызов мог ожидать несколько thread IDs; меняет handler/schema/status result в `core/src/tools/handlers/collab.rs`, `core/src/tools/spec.rs`, protocol events `CollabWaitingBeginEvent/EndEvent`, app-server mapping и TUI rendering.
- Why: release note и diff прямо указывают на multi-thread coordination.
- User info: модель может ждать группу sub-agents одним tool call и получать агрегированные статусы.
- Compatibility/risks: меняет shape результата `wait`; downstream projections должны поддерживать map статусов, а не один agent status.
- Evidence: `EVID-087-c576756c`.

### `f5b3e738fbaa742a38071e367c18b1626837c25e` - `feat: propagate approval request of unsubscribed threads (#9232)`

- Дата: `2026-01-16T11:23:01+01:00`; PR: `#9232`; поколение: old collab/TUI human approval routing.
- Developer info: добавляет routing approval requests от spawned/unsubscribed threads через TUI `AppEvent::ExternalApprovalRequest`, `external_approval_routes`, paused event handling и обратную доставку ответа.
- Why: release note говорит, что TUI surfaces approval requests from spawned/unsubscribed threads; diff подтверждает маршрутизацию.
- User info: пользователь видит approval prompts от подагентов и может отвечать из parent TUI.
- Compatibility/risks: approval routing становится cross-thread; при изменениях sandbox/permissions нужно проверять parent-child delivery, а не только root turn.
- Evidence: `EVID-087-f5b3e738`.

### `7905e99d03027f17dff5df50cef0fb601fc10f93` - `prompt collab (#9367)`

- Дата: `2026-01-16T15:12:41+01:00`; PR: `#9367`; поколение: old collab prompt guidance.
- Developer info: обновляет `core/templates/agents/orchestrator.md` по lifecycle worker/sub-agent coordination: мониторинг, ожидание, interrupt/close behavior.
- Why: diff меняет именно orchestrator prompt contract; deeper motivation не установлено локально.
- User info: меняется модельное поведение оркестратора при делегировании задач.
- Compatibility/risks: prompt-only change, но влияет на runtime outcome; для форка такие изменения требуют model-visible regression checks.
- Evidence: `EVID-087-7905e99d`.

## `rust-v0.88.0`

- Release commit: `149625a4f983e1b3c8530abef1c863689d98a1cc`
- Дата: `2026-01-21T10:43:17-08:00`
- Subject: `## New Features - Added collaboration modes and presets to streamline multi-agent workflows. (#9340, #9421) - Added device-code auth as a standalone fallback in headless environments. (#9333) - Introduced a request-user-input tool for explicit agent prompts. (#9472) - Enabled remote models and auto-enable WebSockets transport for smoother connectivity. (#9554, #9578)`

### `246f50655111fda3d4fa4f2032285d03464fe249` - `Introduce collaboration modes (#9340)`

- Дата: `2026-01-17T00:28:22Z`; PR: `#9340`; поколение: collaboration modes / Plan mode.
- Developer info: вводит `CollaborationMode` как structured session setting в `protocol/src/config_types.rs`, core session configuration/settings update path и mode kinds вместо разрозненного model/effort override.
- Why: subject/release note/diff показывают новый source-of-truth для режимов collaboration.
- User info: появляется концепция режимов `plan`, `pair_programming`, `execute`, `custom` как управляемая настройка workflow.
- Compatibility/risks: downstream должен читать режим как тип/структуру, а не выводить поведение только из model/effort.
- Evidence: `EVID-088-246f5065`.

### `146d54cede3c4ea38fbcacc89ab15a7e30d73b20` - `Add collaboration_mode override to turns (#9408)`

- Дата: `2026-01-16T21:51:25-08:00`; PR: `#9408`; поколение: app-server/protocol surface for collaboration modes.
- Developer info: добавляет per-turn override `collaboration_mode` в `TurnStartParams`, app-server request processing и core turn update path.
- Why: diff напрямую добавляет turn-level collaboration mode override.
- User info: client может менять collaboration mode на конкретном turn, не меняя глобальную сессию вручную.
- Compatibility/risks: mode override становится частью turn contract; persistence/resume должны сохранять effective state.
- Evidence: `EVID-088-146d54ce`.

### `1478a88eb0e655ede6efe86764a5bf9255ca7a13` - `Add collaboration developer instructions (#9424)`

- Дата: `2026-01-18T01:31:14Z`; PR: `#9424`; поколение: collaboration mode prompt/source-of-truth.
- Developer info: связывает collaboration mode с developer instructions через `build_collaboration_mode_update_item`, settings update items и `DeveloperInstructions`.
- Why: diff показывает перенос mode semantics в native context items, чтобы модель получала режим через developer instructions.
- User info: выбранный режим влияет на инструкции модели, а не только на UI label.
- Compatibility/risks: изменение режима должно обновлять model-visible context; нельзя менять только UI.
- Evidence: `EVID-088-1478a88e`.

### `8f0e0300d2ebdb8e743342d73b30a7ba615e9747` - `Expose collaboration presets (#9421)`

- Дата: `2026-01-17T12:32:50-08:00`; PR: `#9421`; поколение: app-server/protocol collaboration modes.
- Developer info: добавляет `collaborationMode/list`, `CollaborationModeListResponse`, builtin presets в models manager, app-server processor/tests.
- Why: release note и diff показывают exposing presets to clients.
- User info: app-server clients могут получить список встроенных режимов, а не hardcode'ить Plan/Execute.
- Compatibility/risks: новый режим должен появляться через preset source-of-truth, не через отдельные списки в UI.
- Evidence: `EVID-088-8f0e0300`.

### `f72f87fbeeea3663c99bf29478cf5765d4c806f6` - `Add collaboration modes test prompts (#9443)`

- Дата: `2026-01-18T11:39:08-08:00`; PR: `#9443`; поколение: collaboration mode prompts.
- Developer info: добавляет/фиксирует templates `core/templates/collaboration_mode/{plan,pair_programming,execute}.md`, schema и preset wiring.
- Why: diff подтверждает prompt assets and presets; точная продуктовая причина beyond test prompts не установлена локально.
- User info: режимы получают отдельные model-visible instructions.
- Compatibility/risks: prompt assets становятся частью behavior contract; generated/config schema меняется вместе с режимами.
- Evidence: `EVID-088-f72f87fb`.

### `31415ebfcf21e91b85a06effdc702a0b76ba3e46` - `Remove unused protocol collaboration mode prompts (#9463)`

- Дата: `2026-01-19T08:54:19-08:00`; PR: `#9463`; поколение: collaboration mode source-of-truth cleanup.
- Developer info: удаляет неиспользуемые protocol prompt artifacts и оставляет canonical prompt source в core templates.
- Why: subject/diff показывают консолидацию prompt source-of-truth.
- User info: direct UX не меняется, но уменьшается риск рассинхронизации prompts.
- Compatibility/risks: форк должен менять canonical core templates, а не удаленные protocol prompt copies.
- Evidence: `EVID-088-31415ebf`.

### `d544adf71a4bc4368059a84df3e56d8481279a1e` - `Feat: plan mode prompt update (#9495)`

- Дата: `2026-01-19T12:17:29-08:00`; PR: `#9495`; поколение: Plan mode.
- Developer info: обновляет `core/templates/collaboration_mode/plan.md`.
- Why: subject/diff показывают prompt-level behavior update; deeper motivation не установлено локально.
- User info: меняется то, как модель ведет планирование в Plan mode.
- Compatibility/risks: prompt-only but user-visible behavior change.
- Evidence: `EVID-088-d544adf7`.

### `675f165c56e0600d3bf4ccc6e798853c0d5fee60` - `fix(core) Preserve base_instructions in SessionMeta (#9427)`

- Дата: `2026-01-19T21:59:36-08:00`; PR: `#9427`; поколение: collaboration mode persistence/context continuity.
- Developer info: сохраняет `base_instructions` в `SessionMeta`/rollout path, затрагивая client/session metadata and collab handler context.
- Why: diff показывает persistence of base instructions; для collaboration modes это важно, потому что mode instructions должны переживать session continuity.
- User info: resume/continuation меньше теряет collaboration instructions.
- Compatibility/risks: persistence shape влияет на старые rollouts; missing fields должны иметь fallback behavior.
- Evidence: `EVID-088-675f165c`.

### `483239d861596a1917fe6d0c82487a92929870aa` - `chore: collab in experimental (#9525)`

- Дата: `2026-01-20T10:19:06Z`; PR: `#9525`; поколение: feature gating/discoverability.
- Developer info: обновляет `Feature::Collab` stage/description/announcement в `core/src/features.rs`.
- Why: subject/diff показывают gate metadata change.
- User info: collab явно представлен как experimental capability.
- Compatibility/risks: stage/gate metadata affects availability/discoverability; fork changes must keep feature gates coherent.
- Evidence: `EVID-088-483239d8`.

### `bf430ad9fee5583afab2bd6641407d67383be297` - `TUI: collaboration mode UX + always submit UserTurn when enabled (#9461)`

- Дата: `2026-01-19T09:32:04-08:00`; PR: `#9461`; поколение: TUI collaboration mode UX.
- Developer info: добавляет TUI `collaboration_modes.rs`, chatwidget state/rendering/tests and always-submit `UserTurn` path when collaboration mode is enabled.
- Why: subject/diff confirm TUI UX and turn submission behavior.
- User info: пользователь может выбирать/видеть режим, а turn routing становится согласованным с collaboration mode.
- Compatibility/risks: TUI must submit through native `UserTurn`/mode path, not custom side channel.
- Evidence: `EVID-088-bf430ad9`.

### `5ae6e70801110da6245ba900a8f07bee3e3d53f1` - `Tui: use collaboration mode instead of model and effort (#9507)`

- Дата: `2026-01-20T10:26:12-08:00`; PR: `#9507`; поколение: TUI collaboration mode source-of-truth.
- Developer info: переводит TUI state/status/settings на `CollaborationMode` как единый артефакт вместо отдельного model+effort pairing.
- Why: subject/diff confirm source-of-truth alignment.
- User info: UI меньше рассинхронизируется между режимом, моделью и effort.
- Compatibility/risks: mode projection должен обновлять model/effort через native mask/apply path.
- Evidence: `EVID-088-5ae6e708`.

### `57ec3a82778b1cc88a000016b79bd4732ff18a43` - `Feat: request user input tool (#9472)`

- Дата: `2026-01-19T10:17:30-08:00`; PR: `#9472`; поколение: collaboration modes / human-in-loop Plan mode.
- Developer info: добавляет `request_user_input` tool, event path, protocol types and app-server tests for explicit user questions.
- Why: release note says explicit agent prompts; diff shows native tool/protocol route.
- User info: модель может явно запросить решение/ввод пользователя в планирующих workflow.
- Compatibility/risks: tool availability must be gated by mode and UI support; otherwise model can request input without client projection.
- Evidence: `EVID-088-57ec3a82`.

### `0523a259c848c583b3960066160066312e43278c` - `Reject ask user question tool in Execute and Custom (#9560)`

- Дата: `2026-01-20T18:32:17-08:00`; PR: `#9560`; поколение: Plan mode tool gating.
- Developer info: добавляет runtime gate for request-user-input tool in `Execute`/`Custom`.
- Why: subject/diff show mode-specific rejection.
- User info: explicit user question tool ограничен подходящими режимами, снижая неожиданные interruptions.
- Compatibility/risks: при добавлении новых modes нужно проверить tool gating matrix.
- Evidence: `EVID-088-0523a259`.

### `5f55ed666b7551e030d0d0bf7576dd69909d512f` - `Add request-user-input overlay (#9585)`

- Дата: `2026-01-21T00:19:35-08:00`; PR: `#9585`; поколение: TUI projection for human-in-loop Plan mode.
- Developer info: добавляет TUI overlay в `tui/src/bottom_pane/request_user_input/*`, interrupt handling and docs.
- Why: diff projects request-user-input protocol into TUI.
- User info: пользователь видит запросы модели с вариантами/other и может ответить в интерфейсе.
- Compatibility/risks: protocol tool без UI overlay неполон для interactive clients.
- Evidence: `EVID-088-5f55ed66`.

### `f1c961d5f7a00033c5c668a8a6ddbe96d2004762` - `feat: max threads config (#9483)`

- Дата: `2026-01-21T09:39:11Z`; PR: `#9483`; поколение: runtime guardrails.
- Developer info: добавляет конфигурируемый лимит sub-agent threads через `core/src/config/mod.rs`, `core/config.schema.json`, `core/src/agent/guards.rs`, `core/src/agent/control.rs` и `core/src/thread_manager.rs`.
- Why: subject/diff показывают перенос hardcoded/implicit thread fan-out ограничения в native config/schema path.
- User info: администратор/профиль может ограничивать максимальное число sub-agent threads штатной конфигурацией.
- Compatibility/risks: все spawn-пути должны проходить через native guardrail; обход `agent/control` сломает лимит.
- Evidence: `EVID-088-f1c961d5`.

## `rust-v0.89.0`

- Release commit: `605d43719efa7f62160904d2dcecc46a64bcd32e`
- Дата: `2026-01-22T12:57:37-08:00`
- Subject: `## New Features - Added a /permissions command with a shorter approval set while keeping /approvals for compatibility. (#9561) - Added a /skill UI to enable or disable individual skills. (#9627) - Improved slash-command selection by prioritizing exact and prefix matches over fuzzy matches. (#9629) - App server now supports thread/read and can filter archived threads in thread/list. (#9569, #9571) - App server clients now support layered config.toml resolution and config/read can compute effective config from a given cwd. (#9510) - Release artifacts now include a stable URL for the published config schema. (#9572)`

### `fe641f759f1725fc32920b228a4b9a86f2d13966` - `Add collaboration_mode to TurnContextItem (#9583)`

- Дата: `2026-01-21T14:14:21-08:00`; PR: `#9583`; поколение: collaboration mode persistence/context.
- Developer info: добавляет `collaboration_mode` в `TurnContextItem`/compact/history path, чтобы context items знали effective mode.
- Why: diff directly persists mode in turn context.
- User info: resume/compaction/history rebuild лучше сохраняют mode state.
- Compatibility/risks: старые context items без поля должны иметь documented behavior.
- Evidence: `EVID-089-fe641f75`.

### `3fcb40245efbee1d9992686f1bdff243d6f9122a` - `Chore: update plan mode output in prompt (#9592)`

- Дата: `2026-01-21T14:12:18-08:00`; PR: `#9592`; поколение: Plan mode/request-user-input prompt contract.
- Developer info: обновляет `core/templates/collaboration_mode/plan.md` и answer shape для `request_user_input` (`answers`).
- Why: subject/diff show prompt/output contract update.
- User info: Plan mode ответы становятся менее неоднозначными и лучше парсятся.
- Compatibility/risks: prompt contract and parser schema must evolve together.
- Evidence: `EVID-089-3fcb4024`.

### `4210fb9e6cb3f50bb93d8fdfcb4494af27a36352` - `Modes label below textarea (#9645)`

- Дата: `2026-01-22T17:31:11Z`; PR: `#9645`; поколение: TUI collaboration mode projection.
- Developer info: добавляет `CollaborationModeIndicator` rendering below textarea in bottom pane/footer.
- Why: diff makes active mode visible in primary TUI surface.
- User info: пользователь видит текущий Plan/Coding/Execute-like mode и hint по переключению.
- Compatibility/risks: label must reflect actual `CollaborationMode`, not stale UI state.
- Evidence: `EVID-089-4210fb9e`.

## `rust-v0.90.0`

- Release commit: `b4e230f8de8f71d08f48c469443ed61a9f365af3`
- Дата: `2026-01-25T16:56:24+01:00`
- Subject: `## New Features - Added a network sandbox proxy with policy enforcement to better control outbound network access. (#8442) - Introduced the first phase of connectors support via the app server and MCP integration, including new config/docs updates. (#9667) - Shipped collaboration mode as beta in the TUI, with a clearer plan → execute handoff and simplified mode selection (Coding vs Plan). (#9690, #9712, #9802, #9834) - Added ephemeral threads and improved collaboration tool provenance metadata for spawned threads. (#9765, #9769) - WebSocket connections now support proxy configuration. (#9719)`

### `515ac2cd19a22151388d16c2d6c73e604cdbd3a0` - `feat: add thread spawn source for collab tools (#9769)`

- Дата: `2026-01-24T14:21:34Z`; PR: `#9769`; поколение: thread lineage/provenance for old collab.
- Developer info: добавляет `SubAgentSource`/`SessionSource` metadata for collab-spawned threads and wires it through collab handlers, agent control and thread manager.
- Why: release note mentions improved collaboration tool provenance metadata; diff adds native lineage fields.
- User info: spawned sub-agent threads получают traceable source, что улучшает resume/list/debug.
- Compatibility/risks: lineage must be persisted and projected; fork changes must not spawn untyped child threads.
- Evidence: `EVID-090-515ac2cd`.

### `83775f4df117b991e99db962171552fc056395e8` - `feat: ephemeral threads (#9765)`

- Дата: `2026-01-24T14:57:40Z`; PR: `#9765`; поколение: thread state/persistence.
- Developer info: добавляет ephemeral thread support in app-server protocol/thread manager paths, including optional thread path and changed archive/load behavior.
- Why: release note pairs ephemeral threads with collaboration provenance; diff is thread lifecycle/persistence work rather than collab-only.
- User info: internal/short-lived agent flows can avoid normal persistent rollout path where appropriate.
- Compatibility/risks: persistence/resume behavior diverges for ephemeral threads; fork features must document whether subagent state is durable.
- Evidence: `EVID-090-83775f4d`.

### `69cfc73dc6d9cfd7cf0ad4c47942dfcaeebee2ac` - `change collaboration mode to struct (#9793)`

- Дата: `2026-01-23T17:00:23-08:00`; PR: `#9793`; поколение: collaboration modes.
- Developer info: normalizes `CollaborationMode` as structured type with `ModeKind`, updating presets/request-user-input/TUI consumers.
- Why: subject/diff show contract normalization.
- User info: mode data becomes less ambiguous across app-server/TUI/core.
- Compatibility/risks: clients must follow structured wire type, not previous ad hoc representation.
- Evidence: `EVID-090-69cfc73d`.

### `58450ba2a1d7c0d2d5405ae5f1f99fcf09e1cfe4` - `Use collaboration mode masks without mutating base settings (#9806)`

- Дата: `2026-01-25T07:35:31Z`; PR: `#9806`; поколение: collaboration mode source-of-truth.
- Developer info: introduces/uses `CollaborationModeMask` so presets overlay base settings instead of mutating them; updates protocol response and TUI/app consumers.
- Why: subject/diff explicitly describes avoiding base settings mutation.
- User info: switching modes becomes safer: base settings survive Plan/Coding mode changes.
- Compatibility/risks: mode application must be mask-based; direct mutation can corrupt user settings.
- Evidence: `EVID-090-58450ba2`.

### `b3127e2eebcdc872cf43966649561b6d10aa8c56` - `Have a coding mode and only show coding and plan (#9802)`

- Дата: `2026-01-23T19:28:49-08:00`; PR: `#9802`; поколение: collaboration modes beta UX.
- Developer info: updates presets/templates/features to simplify mode set around Coding and Plan.
- Why: release note says simplified mode selection; diff updates presets/templates.
- User info: TUI mode selection becomes narrower and clearer.
- Compatibility/risks: removed/hidden modes must not remain hardcoded in clients.
- Evidence: `EVID-090-b3127e2e`.

### `f30f39b28b13e7048bfca7f7e8d4022e7840f443` - `feat: tui beta for collab (#9690)`

- Дата: `2026-01-23T13:57:59+01:00`; PR: `#9690`; поколение: TUI collaboration beta.
- Developer info: extends TUI app/chatwidget/collaboration mode state, popup/selection and beta rendering flows.
- Why: release note says collaboration mode shipped as beta in TUI.
- User info: collab mode becomes an exposed beta workflow in the TUI.
- Compatibility/risks: beta UI must remain driven by mode source-of-truth and feature gates.
- Evidence: `EVID-090-f30f39b2`.

### `0e79d239ed8d2802cacea364c023a72570afb415` - `TUI: prompt to implement plan and switch to Execute (#9712)`

- Дата: `2026-01-23T00:25:50Z`; PR: `#9712`; поколение: Plan -> Execute UX.
- Developer info: adds TUI `SubmitUserMessageWithMode`/plan implementation flow that submits implementation request with execute mode.
- Why: release note mentions clearer plan -> execute handoff.
- User info: после плана пользователь получает native flow для перехода к исполнению.
- Compatibility/risks: handoff must change actual collaboration mode, not just display text.
- Evidence: `EVID-090-0e79d239`.

### `d86bd20411522b7ecfd466aea7e384daac3e7fac` - `Change the prompt for planning and reasoning effort (#9733)`

- Дата: `2026-01-22T18:22:12-08:00`; PR: `#9733`; поколение: Plan mode prompt/config.
- Developer info: changes plan/execute reasoning effort and plan prompt.
- Why: subject/diff confirm prompt and reasoning settings change.
- User info: Plan mode behavior and effort defaults change.
- Compatibility/risks: preset effort is part of mode contract; app-server/TUI should project it through masks.
- Evidence: `EVID-090-d86bd204`.

### `f353d3d695260543aa85af508303345bb06ed159` - `prompt (#9777)`

- Дата: `2026-01-23T19:24:48Z`; PR: `#9777`; поколение: Plan mode prompt.
- Developer info: updates `core/templates/collaboration_mode/plan.md`.
- Why: diff is prompt-only; deeper motivation не установлено локально.
- User info: model planning behavior changes.
- Compatibility/risks: prompt-only change should still be treated as behavior change.
- Evidence: `EVID-090-f353d3d6`.

### `b332482eb1a183166d13bc8b58fbb2137fd3f446` - `Mark collab as beta (#9834)`

- Дата: `2026-01-25T11:13:21+01:00`; PR: `#9834`; поколение: feature gate/stability stage.
- Developer info: changes feature stage for collaboration/multi-agent feature to beta in `core/src/features.rs`.
- Why: release note says shipped as beta.
- User info: collab is communicated as beta rather than purely experimental.
- Compatibility/risks: beta does not mean stable API; docs must still mark old collab generation separately.
- Evidence: `EVID-090-b332482e`.

### `73b5274443cd3ef70ee8d30d707f8fdf805b7ad2` - `feat: cap number of agents (#9855)`

- Дата: `2026-01-25T14:57:22Z`; PR: `#9855`; поколение: runtime guardrails.
- Developer info: adds agent guardrails like spawn depth/count in `core/src/agent/guards.rs` and config default plumbing.
- Why: subject/diff show fan-out control.
- User info: runaway sub-agent spawning is limited by default.
- Compatibility/risks: resource limits must be enforced in native spawn path; bypassing it creates safety regressions.
- Evidence: `EVID-090-73b52744`.

## `rust-v0.91.0`

- Release commit: `3684bc646e0f1123e76a90f03cfb1111358f1aa1`
- Дата: `2026-01-25T17:52:47+01:00`
- Subject: `## Chores - Reduced the maximum allowed number of sub-agents by half to tighten resource usage and guardrails in agent fan-out behavior (#9861)`

### `8fea8f73d6c0dc77ffaf5e24d739ca348f30d5c0` - `chore: half max number of sub-agents (#9861)`

- Дата: `2026-01-25T17:51:55+01:00`; PR: `#9861`; поколение: runtime guardrails.
- Developer info: changes default `DEFAULT_AGENT_MAX_THREADS` from `12` to `6` in `core/src/config/mod.rs`.
- Why: release subject says reduced max sub-agents to tighten resource usage and guardrails.
- User info: default fan-out capacity is smaller; fewer concurrent/total sub-agent threads by default.
- Compatibility/risks: tests/docs relying on default max must not assume old value.
- Evidence: `EVID-091-8fea8f73`.

## `rust-v0.92.0`

- Release commit: `a09055074e082d70e8b92795b0cec1e969a6aaf9`
- Дата: `2026-01-27T10:32:31Z`
- Subject: `## New Features - API v2 threads can now inject dynamic tools at startup and route their calls/responses end-to-end through the server and core tool pipeline. (#9539) - Added filtering on the thread list in the app server to make large thread sets easier to browse. (#9897) - Introduced a thread/unarchive RPC to restore archived rollouts back into active sessions. (#9843) - MCP servers can now define OAuth scopes in config.toml, reducing the need to pass --scopes on each login. (#9647) - Multi-agent collaboration is more capable and safer, with an explorer role, better collab event mapping, and max-depth guardrails. (#9817, #9818, #9918, #9899) - Cached web_search is now the default client behavior. (#9974)`

### `375a5ef05116a3d289dc548402387f86e28851cc` - `fix: attempt to reduce high cpu usage when using collab (#9776)`

- Дата: `2026-01-26T10:07:25-08:00`; PR: `#9776`; поколение: old collab runtime stability.
- Developer info: clamps short `wait` timeouts via `MIN_WAIT_TIMEOUT_MS`, updates tests and tool description.
- Why: subject/diff identify high CPU from collab wait busy behavior.
- User info: collab wait consumes fewer resources and behaves more predictably.
- Compatibility/risks: wait timeout semantics changed; clients/tests should not depend on ultra-short polling.
- Evidence: `EVID-092-375a5ef0`.

### `3ba702c5b6eb53523d3ff644789e900fffa3533c` - `Feat: add isOther to question returned by request user input tool (#9890)`

- Дата: `2026-01-26T09:52:38-08:00`; PR: `#9890`; поколение: Plan mode / request_user_input protocol.
- Developer info: добавляет `isOther` в returned question path for `request_user_input`, затрагивая `protocol/src/request_user_input.rs`, `core/src/tools/spec.rs`, app-server v2 projection и TUI overlay.
- Why: subject/diff показывают explicit escape-option metadata для questions, чтобы UI/API не выводили `Other` из текста.
- User info: варианты ответа в Plan mode получают структурированный признак `isOther`; clients могут рендерить fallback option нативно.
- Compatibility/risks: все projections должны сохранять `isOther`; иначе Plan UX теряет escape path.
- Evidence: `EVID-092-3ba702c5`.

### `c66662c61bda8480e747cde951b15186e5aa3134` - `feat: rebase multi-agent tui on config_snapshot (#9818)`

- Дата: `2026-01-26T10:18:47Z`; PR: `#9818`; поколение: TUI subagent config projection.
- Developer info: uses `thread.config_snapshot()` to synthesize `SessionConfiguredEvent` for child/nested sessions in TUI.
- Why: subject/diff directly connect multi-agent TUI to config snapshot.
- User info: sub-agent threads display/configure with accurate model/provider/approval/sandbox/reasoning metadata.
- Compatibility/risks: UI should use config snapshot source-of-truth, not reconstruct settings ad hoc.
- Evidence: `EVID-092-c66662c6`.

### `3f338e4a6a70413dea864441f06f72becd49615c` - `feat: explorer collab (#9918)`

- Дата: `2026-01-26T16:21:42Z`; PR: `#9918`; поколение: old collab role model.
- Developer info: adds `AgentRole::Explorer`, role profile and config application in `core/src/agent/role.rs`.
- Why: release note mentions explorer role; diff adds enum/profile.
- User info: `spawn_agent` can create an Explorer role tuned for read-only/exploration style tasks.
- Compatibility/risks: role names are part of tool schema/user-visible behavior.
- Evidence: `EVID-092-3f338e4a`.

### `70d59593988d3af390527a6aadd6e801da5d6a33` - `feat: disable collab at max depth (#9899)`

- Дата: `2026-01-26T17:05:36Z`; PR: `#9899`; поколение: runtime guardrails.
- Developer info: disables `Feature::Collab`/spawn availability when max thread spawn depth is exceeded, through core spawn/config and collab handler path.
- Why: subject/diff show max-depth protection for collab.
- User info: deeply nested agents cannot keep spawning more agents past the configured depth.
- Compatibility/risks: feature availability becomes context-sensitive; UI/model prompt must not advertise unavailable collab tools.
- Evidence: `EVID-092-70d59593`.

### `47aa1f3b6affa3b29e7c94ba56bdecb299a1c095` - `Reject request_user_input outside Plan/Pair (#9955)`

- Дата: `2026-01-26T17:12:17-08:00`; PR: `#9955`; поколение: Plan mode tool gating.
- Developer info: обновляет `core/src/tools/handlers/request_user_input.rs` и regression tests, чтобы tool возвращал отказ вне разрешённых collaboration modes.
- Why: subject/diff прямо фиксируют runtime guard для mode-specific human input tool.
- User info: модель не может произвольно спрашивать пользователя через `request_user_input` в неподходящем режиме; ошибка становится контролируемой.
- Compatibility/risks: при добавлении новых modes нужно обновлять allow-list и tool description вместе.
- Evidence: `EVID-092-47aa1f3b`.

### `01d7f8095b54dc3b194def8a4d90de02c93f2fbf` - `feat: codex exec mapping of collab tools (#9817)`

- Дата: `2026-01-26T18:01:35Z`; PR: `#9817`; поколение: exec/app-server mapping for old collab.
- Developer info: maps collab lifecycle events into `codex exec` human/jsonl output via `RunningCollabToolCall`, `CollabToolCallItem`, `handle_collab_*` handlers and status formatting.
- Why: release note says better collab event mapping; diff adds exec projection.
- User info: `codex exec` users can observe collab tool lifecycle instead of losing it in raw events.
- Compatibility/risks: every new collab event/tool kind needs exec output mapping, not just TUI/app-server mapping.
- Evidence: `EVID-092-01d7f809`.

### Plan prompt stabilization series (`#9877`, `#9943`, `#9966`, `#9968`, `#9975`, `#9977`, `#9980`)

- Коммиты: `d27f2533a9b44f1c42437cf8e05fc5da84bcf779` (`Plan prompt (#9877)`), `159ff062813653996b507d51dd451566440d2ec0` (`plan prompt (#9943)`), `b7bba3614ef7a051f8e510092954367dfc85b9a0` (`plan prompt v7 (#9966)`), `b655a092bad1f1e96031d75ae9230dde7e796b64` (`Improve plan mode prompt (#9968)`), `28bd7db14aa600ed7b8b58bb7531932b9133b6f8` (`plan prompt (#9975)`), `cabb2085cc05655a96d6e66d21a70c1ee37d06dd` (`make plan prompt less detailed (#9977)`), `509ff1c643e0b45366fc5d77c78279ca29b71448` (`Fixing main and make plan mode reasoning effort medium (#9980)`).
- Даты: `2026-01-25T19:50:35-08:00` through `2026-01-26T22:30:24-08:00`; поколение: Plan mode prompt/config.
- Developer info: итеративно меняет `core/templates/collaboration_mode/plan.md`, а в `#9980` также preset reasoning effort in `core/src/models_manager/collaboration_mode_presets.rs`.
- Why: для `#9977` локально установлено стремление уменьшить перегруженность prompt из subject/diff; для `#9980` diff/subject фиксируют коррекцию reasoning effort. Для остальных точная причина не установлена локально.
- User info: Plan mode получает более стабильный model-visible protocol: меньше избыточных требований, уточнённые этапы планирования и другой default effort.
- Compatibility/risks: prompt-only серия меняет фактическое поведение модели; форк должен учитывать эти изменения как behavioral surface, даже без wire/API diff.
- Evidence: `EVID-092-plan-prompt-series`.

## `rust-v0.93.0`

- Release commit: `d86cf538f5e7d210ebb7a3493718aaaff40146da`
- Дата: `2026-01-30T23:17:24-07:00`
- Subject: release notes include Plan mode streaming/proposed plan handling and `/plan` TUI work; full release subject verified locally through tag commit metadata.

### `ec4a2d07e411d11f4de13b4e09aa7386c914df6d` - `Plan mode: stream proposed plans, emit plan items, and render in TUI (#9786)`

- Дата: `2026-01-30T18:59:30Z`; PR: `#9786`; поколение: collaboration modes / Plan mode.
- Developer info: добавляет first-class Plan streaming path: `<proposed_plan>` parsing, `ProposedPlanParser`, `PlanDeltaEvent`, `TurnItem::Plan`, v2 `ThreadItem::Plan`, `PlanDeltaNotification`, app-server handling, TUI history/rendering and tests.
- Why: commit message/diff establish intent to separate proposed plans from ordinary assistant text and make Plan mode a structured stream/item.
- User info: Plan mode shows proposed plans as dedicated streamed UI/history items, and implementation prompt appears from structured plan state rather than text heuristics.
- Compatibility/risks: clients must understand `PlanDelta`/`ThreadItem::Plan`; transcript-only rendering loses plan semantics.
- Evidence: `EVID-093-ec4a2d07`.

### `11958221a3354fe7941671cc2c4a24ffd73c29a7` - `tui: add feature-gated /plan slash command to switch to Plan mode (#10103)`

- Дата: `2026-01-29T16:40:43-08:00`; PR: `#10103`; поколение: TUI Plan mode UX.
- Developer info: adds `SlashCommand::Plan`, `plan_mask`, chatwidget dispatch and feature-gated command visibility through `Feature::CollaborationModes`.
- Why: subject/diff show a native TUI shortcut for switching into Plan mode.
- User info: `/plan` becomes a direct TUI entry point for Plan mode instead of requiring manual mode selection.
- Compatibility/risks: command visibility must follow feature gate; hardcoded slash-command lists can expose unavailable mode behavior.
- Evidence: `EVID-093-11958221`.

### `a0ccef9d5c44b7517243163ceb64be8223d5d8d5` - `Chore: plan mode do not include free form question and always include isOther (#10210)`

- Дата: `2026-01-30T01:19:24-08:00`; PR: `#10210`; поколение: Plan mode / request_user_input contract.
- Developer info: updates `request_user_input` handling/spec so questions require non-empty options and include `is_other` instead of relying on free-form question behavior.
- Why: subject/diff show Plan mode question contract was tightened around structured options and `isOther`.
- User info: Plan mode questions become structured-choice interactions with an explicit escape hatch.
- Compatibility/risks: UI/API projections must preserve `isOther`; free-form assumptions become stale.
- Evidence: `EVID-093-a0ccef9d`.

### Plan prompt/rendering iteration (`#10195`, `#10238`, `#10255`, `#10253`)

- Коммиты: `1ce722ed2efe69165b16b0af3501c613dbddef0c` (`plan mode: add TL;DR checkpoint and client behavior note (#10195)`), `9b29a48a09893cfdea7d7ea056918a9a011ccc01` (`Plan mode prompt (#10238)`), `b7351f7f53d07584bf292c1b61bb58c8aa839bb5` (`plan prompt (#10255)`), `83317ed4bff93b5cf98d3d3ec82dad66dd5b2207` (`Make plan highlight use popup grey background (#10253)`).
- Даты: `2026-01-30`; поколение: Plan mode prompt/TUI polish.
- Developer info: updates `core/templates/collaboration_mode/plan.md` and TUI style for proposed plan highlight.
- Why: beyond subject/diff, detailed product reason is не установлено локально.
- User info: Plan flow gets TL;DR/checkpoint wording refinements and clearer visual highlight for proposed plans.
- Compatibility/risks: prompt-only changes are behavior changes; TUI style must still render from structured plan items.
- Evidence: `EVID-093-plan-iteration`.

## `rust-v0.94.0`

- Release commit: `dce99bc2595e58acc4d88a324f3ccab05cf5cd7d`
- Дата: `2026-02-02T09:44:08-08:00`
- Subject: release notes include Plan mode enabled by default and related Plan mode refinements; full release subject verified locally through tag commit metadata.

### `30ed29a7b37e183f151ea04362bb22b496dc456e` - `enable plan mode (#10313)`

- Дата: `2026-02-01T00:58:17Z`; PR: `#10313`; поколение: collaboration modes / Plan mode stability.
- Developer info: changes `Feature::CollaborationModes` in `core/src/features.rs` from under-development/default-disabled to stable/default-enabled.
- Why: subject/diff establish the release transition from gated development mode to default availability.
- User info: Plan mode becomes available without explicitly enabling the under-development feature.
- Compatibility/risks: clients and prompts can now encounter Plan mode by default; stale feature-gate assumptions can hide available UX.
- Evidence: `EVID-094-30ed29a7`.

### Plan prompt stabilization after default enable (`#10308`, `#10329`)

- Коммиты: `2d6757430a05c4c619384b6f1d2f9c8847d4416e` (`plan mode prompt (#10308)`), `3dd9a37e0bb7af065eed668c2c6a4c96cec85320` (`Improve plan mode interaction rules (#10329)`).
- Даты: `2026-01-31`; поколение: Plan mode prompt.
- Developer info: updates `core/templates/collaboration_mode/plan.md` with interaction rules, allowed user-question path, plan structure and constraints.
- Why: subjects/diff establish prompt stabilization; detailed product rationale beyond that is не установлено локально.
- User info: Plan mode default behavior becomes more constrained and predictable after default enablement.
- Compatibility/risks: changing prompt constraints can alter model behavior without schema changes.
- Evidence: `EVID-094-plan-prompt`.

### `03fcd12e77fedf4fa327af27e2e476e1ebc5f651` - `Do not append items on override turn context (#10354)`

- Дата: `2026-02-01T18:51:26-08:00`; PR: `#10354`; поколение: collaboration mode state/source-of-truth.
- Developer info: stores full `CollaborationMode` in `TurnContext`, updates comparison of previous/current mode and regression tests in `collaboration_instructions.rs`.
- Why: subject/diff show duplicate settings items on override turn context were the bug being fixed.
- User info: mode changes do not create duplicate/false collaboration instructions between turns.
- Compatibility/risks: turn context is part of source-of-truth; custom mode changes must update it rather than appending ad hoc instructions.
- Evidence: `EVID-094-03fcd12e`.

### `974355cfddb2402803d06076e60d12a2bd46bf37` - `feat: vendor app-server protocol schema fixtures (#10371)`

- Дата: `2026-02-01T23:38:43-08:00`; PR: `#10371`; поколение: app-server/protocol surfaces.
- Developer info: vendors generated JSON/TypeScript schema fixtures containing `CollaborationMode*`, `PlanDeltaNotification`, `ThreadItem`, `TurnPlanUpdatedNotification`, `SubAgentSource` and related v2 types.
- Why: subject/diff establish schema fixture vendoring; the local reason is contract materialization for clients.
- User info: app-server clients can consume typed schema for Plan/collaboration surfaces instead of relying only on Rust sources.
- Compatibility/risks: schema fixtures must be regenerated when protocol shape changes.
- Evidence: `EVID-094-974355cf`.

## `rust-v0.95.0`

- Release commit: `12dbb76c812afaff8daf1f3a5acf1d5ef4a75cb1`
- Дата: `2026-02-03T20:07:30-08:00`
- Subject: release notes include `/plan` inline args/images and slash-command polish; full release subject verified locally through tag commit metadata.

### `9513f18bfe9edd9795f86c9645bdd023189a567c` - `chore: collab experimental (#10381)`

- Дата: `2026-02-02T10:57:44Z`; PR: `#10381`; поколение: old collab feature gating.
- Developer info: changes `Feature::Collab` metadata to `Stage::Experimental { name: "Sub-agents", ... }`, default disabled, with experimental UI text.
- Why: subject/diff show old collab/sub-agents surface being explicitly classified as experimental.
- User info: `/experimental` can show `Sub-agents`; users need opt-in/restart rather than assuming stable availability.
- Compatibility/risks: do not mix this experimental old collab gate with default-enabled Plan mode.
- Evidence: `EVID-095-9513f18b`.

### `3cc9122ee2596d937def5ed6fcd221dc446813b5` - `feat: experimental flags (#10231)`

- Дата: `2026-02-02T11:06:50Z`; PR: `#10231`; поколение: app-server/protocol experimental gating.
- Developer info: introduces `ExperimentalApi`, macro crate, `initialize.capabilities.experimentalApi`, app-server runtime rejection and schema filtering; marks experimental surfaces such as `collaborationMode/list` and `thread/start.dynamicTools`.
- Why: subject/diff show a unified experimental API gate and projection system.
- User info: app-server experimental methods/fields are unavailable unless clients explicitly opt in.
- Compatibility/risks: clients without experimental capability see filtered schema/errors; fork additions must mark experimental fields consistently.
- Evidence: `EVID-095-3cc9122e`.

### `3392c5af243821c85c3b816cbb0623a795a91385` - `Nicer highlighting of slash commands, /plan accepts prompt args and pasted images (#10269)`

- Дата: `2026-02-02T09:53:29-08:00`; PR: `#10269`; поколение: TUI Plan mode UX.
- Developer info: updates command parsing/submission via `CommandWithArgs`, `prepare_inline_args_submission`, `supports_inline_args` and text/image element handling.
- Why: subject/diff show `/plan` becoming a command with inline arguments and pasted media support.
- User info: `/plan <prompt>` can switch to Plan mode and submit initial prompt/images in one native TUI action.
- Compatibility/risks: slash command parsing must preserve text elements/images, not flatten them as plain text.
- Evidence: `EVID-095-3392c5af`.

### `d509df676b61d837d0c1dfb287ae53f1cce52578` - `Cleanup collaboration mode variants (#10404)`

- Дата: `2026-02-03T09:23:53-08:00`; PR: `#10404`; поколение: collaboration modes contract cleanup.
- Developer info: narrows public mode values to `default|plan`, keeps `PairProgramming`/`Execute` as legacy-compatible aliases and updates presets, config schema, app-server schema, TUI and `request_user_input` tests.
- Why: subject/diff establish public collaboration-mode contract cleanup.
- User info: users/clients see fewer modes and a clearer Default/Plan split; legacy names are coerced rather than exposed as first-class choices.
- Compatibility/risks: clients relying on old public mode names need compatibility handling.
- Evidence: `EVID-095-d509df67`.

### `998eb8f32be5ea0f29271d07d2fb812a69c6a3da` - `Improve Default mode prompt (less confusion with Plan mode) (#10545)`

- Дата: `2026-02-03T12:08:38-08:00`; PR: `#10545`; поколение: Plan mode tool gating/prompt.
- Developer info: centralizes `REQUEST_USER_INPUT_ALLOWED_MODES = [Plan]`, updates tool description/unavailable messages and Default prompt text.
- Why: subject/diff show confusion between Default and Plan mode was being reduced.
- User info: `request_user_input` is clearly unavailable in Default; incorrect calls produce a controlled message.
- Compatibility/risks: allowed-mode list and tool description must evolve together.
- Evidence: `EVID-095-998eb8f3`.

### `1096d6453c096a1860ba8e2595729cc69e2e312b` - `Fix plan implementation prompt reappearing after /agent thread switch (#10447)`

- Дата: `2026-02-02T17:40:05-08:00`; PR: `#10447`; поколение: TUI Plan/subagent UX.
- Developer info: updates TUI `saw_plan_item_this_turn` reset logic and tests replay/live completion behavior when switching `/agent` threads.
- Why: subject/diff establish fix for duplicated `Implement this plan?` prompt after agent-thread switch.
- User info: switching agent threads no longer replays stale plan implementation prompts.
- Compatibility/risks: Plan prompt rendering depends on both live and replay event ordering.
- Evidence: `EVID-095-1096d645`.

### `8f5edddf7121fdb676feac511c80da1b6d4dfb16` - `TUI: Render request_user_input results in history and simplify interrupt handling (#10064)`

- Дата: `2026-02-02T17:41:30-08:00`; PR: `#10064`; поколение: TUI request_user_input projection.
- Developer info: adds `RequestUserInputResultCell`, inserts history cells after `Op::UserInputAnswer` and simplifies Esc/Ctrl+C interrupt behavior.
- Why: subject/diff show transcript rendering and interrupt handling were the intended fixes.
- User info: Plan-mode answers are visible in history; interrupted overlays do not accidentally submit partial answers.
- Compatibility/risks: request/input transcript must be event-backed, not local overlay state only.
- Evidence: `EVID-095-8f5edddf`.

### `d9ad5c3c4959d2cabd57f9aaed75e96bb2c07c91` - `fix(app-server): fix approval events in review mode (#10416)`

- Дата: `2026-02-03T12:08:17-08:00`; PR: `#10416`; поколение: app-server/protocol sub-agent delegate flow.
- Developer info: fixes `codex_delegate.rs` approval event correlation so review-mode sub-agent delegate approval uses the actual sub-agent `call_id`; app-server tests assert `requestApproval.itemId`.
- Why: subject/diff establish incorrect approval event IDs in review mode.
- User info: review/multi-agent approval prompts correlate with the correct command execution item.
- Compatibility/risks: approval routing must preserve child/delegate call IDs; parent IDs are not interchangeable.
- Evidence: `EVID-095-d9ad5c3c`.

## `rust-v0.96.0`

- Release commit: `2572f96fafb3156f79523bfbe2bc84f13e382167`
- Дата: `2026-02-04T17:00:11Z`
- Subject: release notes mostly cover app-server/control/config/persistence work; direct Plan/collab change in this release is minor request-user-input UX.

### `a9eb766f33953d651a0f01670d893a4cf5da3763` - `tui: make Esc clear request_user_input notes while notes are shown (#10569)`

- Дата: `2026-02-03T16:17:06-08:00`; PR: `#10569`; поколение: TUI request_user_input UX.
- Developer info: updates `RequestUserInputOverlay` key handling so `Esc` clears notes and returns focus to options when notes UI is visible.
- Why: subject/diff establish the intended overlay behavior.
- User info: users can clear optional notes in Plan questions without aborting the interaction.
- Compatibility/risks: interrupt behavior and note-editing behavior must remain distinct in TUI tests.
- Evidence: `EVID-096-a9eb766f`.

## `rust-v0.97.0`

- Release commit: `4b415c72c9cb62dded0bb8eb5d4fcf9ddc90a8e3`
- Дата: `2026-02-04T20:26:51-08:00`
- Subject: `## New Features - Added a session-scoped “Allow and remember” option for MCP/App tool approvals, so repeated calls to the same tool can be auto-approved during the session. (#10584) - Added live skill update detection, so skill file changes are picked up without restarting. (#10478) - Added support for mixed text and image content in dynamic tool outputs for app-server integrations. (#10567) - Added a new `/debug-config` slash command in the TUI to inspect effective configuration. (#10642) - Introduced initial memory plumbing (API client + local persistence) to support thread memory summaries. (#10629, #10634) - Added configurable `log_dir` so logs can be redirected (including via `-c` overrides) more easily. (#10678)`

### `7f203576116de4002fcda5942a1e4248e18692ff` - `Stop client from being state carrier (#10595)`

- Дата: `2026-02-04T09:05:37-08:00`; PR: `#10595`; поколение: old collab runtime state / spawn source-of-truth.
- Developer info: moves session/turn data used by collab spawning out of `ModelClient` getters and into `TurnContext`: `session_source`, `config`, `model_info`, `provider`, `reasoning_effort`, `reasoning_summary` and related runtime handles. In `codex-rs/core/src/tools/handlers/collab.rs`, `spawn_agent` now reads `turn.session_source` for depth calculation and `build_agent_spawn_config` reads turn-context values instead of client state. The commit also updates tests such as `build_agent_spawn_config_uses_turn_context_values` and depth-limit checks.
- Why: commit message says the client should become session-wide and must stop carrying random state; diff confirms this was needed before/for a cleaner session-scoped client. The direct collab reason is locally established by the `collab.rs` diff: sub-agent spawn config must be derived from current turn context, not hidden client state.
- User info: no new command appears, but spawned agents inherit the correct current model/provider/reasoning/session-source context through the native spawn path. This reduces risk of stale state when collaboration settings change between turns.
- Compatibility/risks: fork changes must treat `TurnContext` as the source-of-truth for per-turn spawn behavior. Adding new per-turn multi-agent settings only to `ModelClient` or a UI layer would bypass old collab spawn propagation and create stale child-agent config.
- Evidence: `EVID-097-7f203576`.

### Demoted near-misses for `rust-v0.97.0`

- `282f42c0ce3c067a1a00c7590f76ef7bec47e207` (`Add option to approve and remember MCP/Apps tool usage (#10584)`) is session approval policy for MCP/Apps tools. It can affect sub-agent/tool-heavy sessions, but diff is limited to `mcp_tool_call.rs` approval memory and does not change collab/Plan/subagent runtime contract.
- `7a253076fef8058adb7964fdd521df80663cf39d` (`Persist pending input user events (#10656)`) is persistence of pending user input; useful workflow-continuity evidence, but not direct multi-agent functionality.
- `7bcc552325b95b1f17042c0e4f928ef971f26339` (`Added support for live updates to skills (#10478)`) is skills cache/file-watcher infrastructure, not sub-agent orchestration.
- `5ea107a08857e44001fe156a67b7d8536d5db66b` (`feat(app-server, core): allow text + image content items for dynamic tool outputs (#10567)`) is an app-server dynamic-tools wire change. It is important API evidence, including a breaking `DynamicToolCallResponse.output` to `contentItems` change, but it is dynamic-tool output plumbing rather than multi-agent/collab behavior.
- `0e8d359da900512129750cd6cce0a7799ddea28e` (`Session-level model client (#10664)`) completes broader client/session separation and is architecture evidence for turn-scoped `ModelClientSession`, but the direct collab spawn source-of-truth change is already captured in `#10595`.
- `e9335374b9ef43710e76a411caca5e3a0bdf139b` (`feat: add phase 1 mem client (#10629)`) and `4922b3e571d2da548f7e592707b4a1fac8fda09f` (`feat: add phase 1 mem db (#10634)`) introduce memory plumbing, not multi-agent/collab behavior.
- `d452bb3ae5b5e0f715bba3a44d7d30a51b5f28ae` (`Add /debug-config slash command (#10642)`) improves config diagnostics but does not change collaboration mode or sub-agent runtime semantics.
- `d589ee05b169af83f97d9603bcdf8eada77732c9` (`Fix jitter in TUI apps/connectors picker (#10593)`), `4ed8d74aab66f261fb08c4ad55fda936ff92e533` (`fix: ensure status indicator present earlier in exec path (#10700)`) and `d876f3b94fa76c9d169d032112592233ab5568f8` (`fix(tui): restore working shimmer after preamble output (#10701)`) are general TUI polish, not multi-agent/Plan timeline entries.

## `rust-v0.98.0`

- Release commit: `82464689ce0ba8a3b2065e73a8aa0cfdf2ad0625`
- Дата: `2026-02-05T08:12:44-08:00`
- Subject: `## New Features - Steer mode is now stable and enabled by default, so `Enter` sends immediately during running tasks while `Tab` explicitly queues follow-up input. (#10690)`

### `41b4962b0a7f5d73bb23d329ad9bb742545f6a2c` - `Sync collaboration mode naming across Default prompt, tools, and TUI (#10666)`

- Дата: `2026-02-04T23:03:28-08:00`; PR: `#10666`; поколение: collaboration modes / Plan naming cleanup.
- Developer info: synchronizes naming across `core/src/models_manager/collaboration_mode_presets.rs`, `core/src/tools/handlers/request_user_input.rs`, `core/templates/collaboration_mode/default.md`, `protocol/src/config_types.rs`, `tui/src/chatwidget.rs` and `tui/src/collaboration_modes.rs`.
- Why: subject/diff establish that Default/Plan naming drift existed across prompts, tools and TUI.
- User info: users see a more consistent Default/Plan naming model, while tools and TUI stop exposing conflicting mode labels.
- Compatibility/risks: fork additions to collaboration modes must update prompt/tool/TUI surfaces together; changing only presets creates naming drift.
- Evidence: `EVID-098-41b4962b`.

## `rust-v0.99.0`

- Release commit: `ec9f76ce4f854c7d4f3c78c9b1bacbe128df286e`
- Дата: `2026-02-11T11:54:41-08:00`
- Subject: `## New Features`

### `62605fa47102d72bb901dcbd0b2be5ec68ace8c9` - `Add resume_agent collab tool (#10903)`

- Дата: `2026-02-07T17:31:45+01:00`; PR: `#10903`; поколение: old collab lifecycle/resume.
- Developer info: adds `resume_agent` collab tool and corresponding protocol/app-server schema/events (`CollabResumeBeginEvent`, `CollabResumeEndEvent`, `CollabAgentTool` update) plus app-server docs/tests.
- Why: subject/diff show a missing lifecycle primitive: old collab needed native resume in addition to spawn/send/wait/close.
- User info: model can resume a previously paused/closed child thread through a native collab tool rather than manual thread switching.
- Compatibility/risks: old collab lifecycle now includes `resume_agent`; projections must map begin/end events, and later MAv2 should not assume only spawn/send/wait/close exist historically.
- Evidence: `EVID-099-62605fa4`.

### `87ccc5bbae759629d8a51525c977c56c978e0b09` - `feat: add connector capabilities to sub-agents (#11191)`

- Дата: `2026-02-10T11:53:01Z`; PR: `#11191`; поколение: old collab sub-agent capability propagation.
- Developer info: updates `core/src/agent/control.rs`, `core/src/tools/handlers/collab.rs` and `core/src/tools/spec.rs` so spawned sub-agents can receive connector capability context through the native spawn/spec path.
- Why: subject/diff show connector capabilities needed to be available to sub-agents, not only root sessions.
- User info: sub-agents can use connector-backed capabilities where policy/config permits, improving delegation beyond local-only tool work.
- Compatibility/risks: connector capability propagation must remain policy-aware; fork features should pass capabilities through native spawn config rather than duplicating connector lists.
- Evidence: `EVID-099-87ccc5bb`.

### `223fadc7605894638e01aa9f96c9cd1836c9ef85` - `Fix spawn_agent input type (#11304)`

- Дата: `2026-02-10T12:16:39Z`; PR: `#11304`; поколение: old collab spawn compatibility.
- Developer info: fixes the `spawn_agent` input type in startup/memory tool construction path so model-visible input schema matches the expected tool call shape.
- Why: subject/diff establish a schema/type mismatch fix.
- User info: `spawn_agent` calls are less likely to be rejected due to malformed argument type.
- Compatibility/risks: tool schema shape is part of the model contract; fixes must be reflected anywhere model-visible tools are assembled.
- Evidence: `EVID-099-223fadc7`.

### Plan/sub-agent UX and rollout notes (`#10457`, `#10748`, `#10928`, `#10921`, `#11175`, `#11173`, `#11230`)

- Коммиты: `040ecee7154556924e3b66b4b52253a51afc731f` (`Update explorer role default model (#10748)`), `b7ecd166a682de39ff1206e5c62afadbcbef72a2` (`Queue nudges while plan generating (#10457)`), `1751116ec62aebf9c242ad7ed00515386629f56e` (`chore(app-server): add experimental annotation to relevant fields (#10928)`), `f3f35526a8056e28a99ef57136ef548301eee9d2` (`Show left/right arrows to navigate in tui request_user_input (#10921)`), `13de7442965c2b59df46e21287585ba57d8de71d` (`fix: do not show closed agents in `/agent` (#11175)`), `284c03ceabe0fc52fdff8e24657d1fa4ddf5fcdb` (`chore: enable sub agents (#11173)`), `c2bfd1e473377e158cbdcb234dd62d2fc3ecf7ce` (`Revert "chore: enable sub agents" (#11230)`).
- Даты: `2026-02-05` through `2026-02-09`; поколение: Plan UX / old collab rollout control.
- Developer info: `#10457` queues user nudges while Plan is still generating; `#10928` annotates experimental app-server fields including collaboration/dynamic surfaces; `#11175` hides closed agents from `/agent`; `#11173` attempted to enable sub-agents and `#11230` reverted that exact enablement. Explorer model/default and request-user-input arrow hints are minor mode UX changes.
- Why: subjects/diffs establish local intent; detailed product rationale beyond those is не установлено локально.
- User info: Plan mode input becomes less race-prone, `/agent` list avoids closed entries, and release history records that sub-agent enablement was attempted but reverted before the stable release effect could stand as durable default behavior.
- Compatibility/risks: attempted enable/revert pairs must be recorded as rollout evidence, not as stable availability. App-server experimental annotations are required when exposing unstable fields.
- Evidence: `EVID-099-ux-rollout`.

## `rust-v0.100.0`

- Release commit: `8272f9a71ee7364d8de992418103628f8a17eb6f`
- Дата: `2026-02-12T09:50:36-08:00`
- Subject: release notes focus on `js_repl`, rate limits, app-server websocket transport, memory commands, Apps SDK apps and sandbox capabilities; full release subject verified locally through tag commit metadata.

### `2fac9cc8cd1afb02e3ac507b21786eaa3ab6bc52` - `chore: sub-agent never ask for approval (#11464)`

- Дата: `2026-02-11T19:19:37Z`; PR: `#11464`; поколение: old collab approval policy.
- Developer info: updates `core/src/tools/handlers/collab.rs`, review task plumbing and tests so sub-agent execution does not ask for approval in the same way as parent/root interactions.
- Why: subject/diff establish a policy change for sub-agent approval behavior.
- User info: sub-agent work becomes less blocked by redundant approval prompts; root/session policy remains the governing surface.
- Compatibility/risks: approval suppression must be scoped to child-agent context and should not weaken root approval/sandbox guarantees.
- Evidence: `EVID-100-2fac9cc8`.

## `rust-v0.101.0`

- Release commit: `cf5f1868bc3df74739fc8e06e5c2e93728c5d8f9`
- Дата: `2026-02-12T11:27:09-08:00`
- Subject: `## New Features - Memory files now include the working directory in stored context, improving memory behavior when switching between different project directories. (#11591)`
- Результат triage: прямых multi-agent/collab/subagent/Plan изменений в диапазоне `rust-v0.100.0^{}..rust-v0.101.0^{}` не найдено. Memory cwd context is excluded as memory infrastructure, not agent orchestration.
- Evidence: `EVID-101-no-change`.

## `rust-v0.102.0`

- Release commit: `f59c7c1ab912c9bc5826e5ae60e49ff1b9b255ff`
- Дата: `2026-02-17T11:21:41-08:00`
- Subject: release notes include customizable multi-agent roles and migration toward the new multi-agent naming/config surface; full release subject verified locally through tag commit metadata.

### `e41536944e7e89763c51d7780bd1c3f3b8957836` - `chore: rename collab feature flag key to multi_agent (#11918)`

- Дата: `2026-02-16T15:28:31Z`; PR: `#11918`; поколение: migration from old `collab` naming to `multi_agent`.
- Developer info: updates `core/config.schema.json`, `core/src/features.rs` and `core/src/features/legacy.rs` to introduce `multi_agent` as the feature flag key while preserving legacy handling.
- Why: release note and subject establish migration toward new multi-agent naming/config surface.
- User info: configuration naming moves away from `collab` toward `multi_agent`.
- Compatibility/risks: fork docs/config must support legacy key migration and avoid reintroducing `collab` as the primary user-facing name.
- Evidence: `EVID-102-e4153694`.

### `e47045c80686164df0d608ca652244646786c1d6` - `feat: add customizable roles for multi-agents (#11917)`

- Дата: `2026-02-16T16:29:32Z`; PR: `#11917`; поколение: configurable multi-agent roles.
- Developer info: expands role loading/config in `core/src/agent/role.rs`, adds builtin role config, updates `core/src/tools/handlers/collab.rs` and `core/src/tools/spec.rs` so roles are no longer only hardcoded presets.
- Why: release note and diff show user-configurable multi-agent roles as the explicit feature.
- User info: users can customize roles used by `spawn_agent` instead of being limited to builtin role presets.
- Compatibility/risks: role config becomes a source-of-truth; fork role extensions should use the native role config loader/schema, not hardcoded branches.
- Evidence: `EVID-102-e47045c8`.

### `beb5cb4f4808c0b1c7717e47535868f1b9e2e050` - `Rename collab modules to multi agents (#11939)`

- Дата: `2026-02-16T19:05:13Z`; PR: `#11939`; поколение: codebase naming migration.
- Developer info: renames `core/src/tools/handlers/collab.rs` to `multi_agents.rs` and `tui/src/collab.rs` to `multi_agents.rs`, updates handler/spec/TUI references.
- Why: subject/diff establish codebase migration from old collab naming to multi-agent naming.
- User info: no direct new command, but developer-facing extension points are renamed to the current concept.
- Compatibility/risks: history still contains old event/tool names; current fork changes should target `multi_agents` modules while preserving old protocol compatibility.
- Evidence: `EVID-102-beb5cb4f`.

### `76283e6b4e0fae570344b221b4bbb152d969e850` - `feat: move agents config to main config (#11982)`

- Дата: `2026-02-17T18:17:19Z`; PR: `#11982`; поколение: configurable multi-agent roles/source-of-truth.
- Developer info: moves agents config into main `Config`/schema loader path (`core/config.schema.json`, `core/src/config/mod.rs`, `config_loader`, `agent/role.rs`, `multi_agents.rs`, `tools/spec.rs`) instead of a separate builtin config file.
- Why: release note and diff show consolidation of multi-agent config into the native configuration source.
- User info: multi-agent role/config customization is loaded through the normal config surface.
- Compatibility/risks: fork additions should extend `Config`/schema and loader path so app-server/TUI/runtime see the same role definitions.
- Evidence: `EVID-102-76283e6b`.

### `851fcc377bcfa8c5bf2453bc91e9717727b412f1` - `feat: switch on dying sub-agents (#11477)`

- Дата: `2026-02-13T18:29:03Z`; PR: `#11477`; поколение: TUI sub-agent navigation/recovery.
- Developer info: updates `tui/src/app.rs` so TUI can switch focus when sub-agents are dying/closed.
- Why: subject/diff establish a navigation recovery change for dying sub-agent threads.
- User info: users are not stranded on a dying child-agent thread; TUI can move to a viable agent/thread context.
- Compatibility/risks: agent navigation must reflect lifecycle status, not only existence of a thread id.
- Evidence: `EVID-102-851fcc37`.

## `rust-v0.103.0`

- Release commit: `ff3d19f9022b154b9ab415f6ad128cd622608b1a`
- Дата: `2026-02-17T14:23:14-08:00`
- Subject: `## New Features - App listing responses now include richer app details (`app_metadata`, branding, and labels), so clients can render more complete app cards without extra requests. (#11706) - Commit co-author attribution now uses a Codex-managed `prepare-commit-msg` hook, with `command_attribution` override support (default label, custom label, or disable). (#11617)`
- Результат triage: прямых multi-agent/collab/subagent/Plan изменений в диапазоне `rust-v0.102.0^{}..rust-v0.103.0^{}` не найдено. Apps listing and commit attribution are excluded as non-agent surfaces.
- Evidence: `EVID-103-no-change`.

## `rust-v0.104.0`

- Release commit: `74d1f7b2b3af383bd3605344f3c842b194fd1d70`
- Дата: `2026-02-17T22:34:44-08:00`
- Subject: `## New Features - Added `WS_PROXY`/`WSS_PROXY` environment support (including lowercase variants) for websocket proxying in the network proxy. (#11784) - App-server v2 now emits notifications when threads are archived or unarchived, enabling clients to react without polling. (#12030) - Protocol/core now carry distinct approval IDs for command approvals to support multiple approvals within a single shell command execution flow. (#12051)`
- Результат triage: прямых multi-agent/collab/subagent/Plan изменений в диапазоне `rust-v0.103.0^{}..rust-v0.104.0^{}` не найдено. Thread archive notifications and distinct command approval IDs are useful platform evidence but do not change multi-agent behavior directly.
- Evidence: `EVID-104-no-change`.

## `rust-v0.105.0`

- Release commit: `a7eda6a29b3ee25549f385197ff109508dc49a90`
- Дата: `2026-02-25T16:36:36Z`
- Subject: release notes include easier multi-agent workflows, `spawn_agents_on_csv`, sub-agent nicknames, cleaner picker and visible child-thread approval prompts; full release subject verified locally through tag commit metadata.

### `2daa3fd44fe5787aee79016a1019fe8cec369151` - `feat: sub-agent injection (#12152)`

- Дата: `2026-02-19T11:32:10Z`; PR: `#12152`; поколение: old multi-agent runtime / sub-agent notification injection.
- Developer info: adds sub-agent injection/notification plumbing through `core/src/agent/control.rs`, `core/src/codex_thread.rs`, `core/src/session_prefix.rs`, `core/src/tools/handlers/multi_agents.rs`, `core/src/tools/spec.rs` and `core/tests/suite/subagent_notifications.rs`.
- Why: subject/diff establish native injection of sub-agent notifications into context.
- User info: parent/model context can receive structured sub-agent notification content instead of relying only on manual thread inspection.
- Compatibility/risks: notification injection is model-visible context; fork changes must preserve ordering and avoid duplicate/noisy injections.
- Evidence: `EVID-105-2daa3fd4`.

### `dcab40123f5e64ba8af962ae27abe6cbcc205344` - `Agent jobs (spawn_agents_on_csv) + progress UI (#10935)`

- Дата: `2026-02-24T21:00:19Z`; PR: `#10935`; поколение: old multi-agent fan-out workflow.
- Developer info: adds `core/src/tools/handlers/agent_jobs.rs`, tool/spec/router integration, config schema for agent jobs, tests and progress UI surfaces so `spawn_agents_on_csv` can fan out tasks from a CSV with progress/ETA.
- Why: release note and diff establish a new native multi-agent batch workflow.
- User info: users/model can launch many sub-agent jobs from CSV and track progress rather than manually spawning each agent.
- Compatibility/risks: batch fan-out must use native spawn guards/config and progress events; fork extensions should not create a separate scheduler that bypasses multi-agent limits.
- Evidence: `EVID-105-dcab4012`.

### Multi-agent config and guardrails (`#12133`, `#12157`, `#12251`, `#12667`, `#12660`, `#12770`)

- Коммиты: `7b65b05e87acc26de946eed7d47c4352fc28a93f` (`feat: validate agent config file paths (#12133)`), `9f5b17de0d2969d825385091a7e83094627ad633` (`Disable collab tools during review delegation (#12157)`), `d87cf7794c0cbff93310021ab3a159ae87b0eca7` (`Add configurable agent spawn depth (#12251)`), `8758db5d5bf9f3d98bff595f38f6124d0174a2a8` (`feat: mutli agents persist config overrides (#12667)`), `6d6570d89d13d45fe5cea4e653dbe9a4f6c25a84` (`Support external agent config detect and import (#12660)`), `01f25a7b9646bf71672cb3363132ff8f97556c27` (`chore: unify max depth parameter (#12770)`).
- Даты: `2026-02-18` through `2026-02-25`; поколение: multi-agent config/source-of-truth and guardrails.
- Developer info: validates agent config file paths, disables collab tools in review delegation, adds configurable spawn depth through `Config`/schema/guards/handler, persists config overrides in `multi_agents.rs`, and exposes external agent config detect/import through app-server protocol plus core migration logic.
- Why: subjects/diffs establish configuration safety, depth control and import/migration needs.
- User info: users can configure agents more safely, import external agent configs and keep config overrides across spawned agents; nested/review scenarios get guardrails.
- Compatibility/risks: config must flow through main schema/loader and native guard checks. External import is app-server API surface and should be updated with schema/tests if changed.
- Evidence: `EVID-105-config-guards`.

### Sub-agent TUI/model visibility (`#12072`, `#12320`, `#12327`, `#12332`, `#12570`, `#12575`, `#12663`, `#12767`)

- Коммиты: `486e60bb5515da92ae4a64ef31004a194c200b1a` (`Add message phase to agent message thread item (#12072)`), `0f9eed3a6f479395520fcd7aceb9fc5b2f3f3754` (`feat: add nick name to sub-agents (#12320)`), `4d60c803ba9c703b978e89a8f6f8a4ad1866582c` (`feat: cleaner TUI for sub-agents (#12327)`), `5a30cd3f92f28ba9a32b38c6d277b377493a253b` (`feat: better agent picker in TUI (#12332)`), `829d1080f641197c3ce8351c3e9951dccc3636c6` (`feat: keep dead agents in the agent picker (#12570)`), `cf0210bf22c5b2f0dbf5c2610add895f6ee8e96b` (`feat: agent nick names to model (#12575)`), `0679e70bfce35d589363df24c49abd75ff98b10a` (`fix: replay after `/agent` (#12663)`), `bcd6e68054ef5ac7507733fdb46cb15ec5156773` (`Display pending child-thread approvals in TUI (#12767)`).
- Даты: `2026-02-17` through `2026-02-25`; поколение: TUI/app-server/model projections for old multi-agent.
- Developer info: extends app-server thread item schema with `MessagePhase`, adds sub-agent nicknames to protocol/app-server/core agent control and model-visible tool output, improves TUI agent picker/clean rendering, preserves dead agents for navigation, fixes replay after `/agent`, and surfaces pending child-thread approvals.
- Why: release note and diffs establish UX visibility gaps: users needed trackable nicknames, cleaner picker, replay correctness and approval prompts from child threads.
- User info: sub-agents are easier to follow by nickname/status, dead/closed agents remain discoverable when useful, replay after `/agent` is less broken, and child-thread approvals are visible in TUI.
- Compatibility/risks: projections must stay aligned across core events, app-server schema and TUI. Nicknames become both user-visible and model-visible context, so fork changes should avoid independent naming systems.
- Evidence: `EVID-105-tui-model`.

### Plan mode and rollout cleanup (`#12265`, `#12028`, `#12303`, `#12579`)

- Коммиты: `c3cb38eafbbbe9ccd9fd18c10e76417941c5d59d` (`Clarify cumulative proposed_plan behavior in Plan mode (#12265)`), `6e60f724bcd18e722086e646f9d7e4f19214a9f1` (`remove feature flag collaboration modes (#12028)`), `4c1744afb22383ae61bebf7feb19a24eedc970df` (`Improve Plan mode reasoning selection flow (#12303)`), `2119532a812831578a8ffeb2b5ac014037518106` (`feat: role metrics multi-agent (#12579)`).
- Даты: `2026-02-19` through `2026-02-23`; поколение: Plan mode stability / rollout cleanup / metrics.
- Developer info: tightens Plan prompt behavior around cumulative `proposed_plan`, removes collaboration modes feature flag from runtime/TUI checks, improves mode-switch reasoning selection flow and adds role metrics in multi-agent guards/handler.
- Why: subjects/diffs establish prompt clarity, feature flag cleanup and metrics needs; detailed product rationale beyond that is не установлено локально.
- User info: Plan mode no longer depends on a feature flag, mode-switch reasoning hints improve, and multi-agent role usage becomes observable.
- Compatibility/risks: removing a feature flag means fork should not re-gate stable Plan mode accidentally. Metrics are observational and should not become behavior dependencies.
- Evidence: `EVID-105-plan-rollout`.

## `rust-v0.106.0`

- Release commit: `ffd726a656403b69b75130025587d5e0a0d6b7d1`
- Дата: `2026-02-26T11:39:41-08:00`
- Subject: release notes include enabling `request_user_input` in Default collaboration mode; full release subject verified locally through tag commit metadata.

### `2f4d6ded1dd7a6d6e3c9ed6cded8f2bf41328f05` - `Enable request_user_input in Default mode (#12735)`

- Дата: `2026-02-25T15:20:46-08:00`; PR: `#12735`; поколение: request_user_input / collaboration mode gating.
- Developer info: updates `request_user_input` mode allow-list and related user-facing/tool descriptions so the tool is no longer Plan-only.
- Why: release note and subject explicitly establish enabling `request_user_input` in Default collaboration mode.
- User info: users can receive structured model questions in Default mode, not just during Plan mode.
- Compatibility/risks: fork changes to human-in-the-loop flows must not assume `request_user_input` implies Plan mode. Mode restrictions and tool descriptions must remain synchronized.
- Evidence: `EVID-106-2f4d6ded`.

### Sub-agent polish and runtime safety (`#12884`, `#12918`, `#12911`)

- Коммиты: `51cf3977d496825a6d89d8a2326aa7aecde8af9e` (`chore: new agents name (#12884)`), `79cbca324ab0f58d9f41b1e0e90bde5adb0aee55` (`Skip history metadata scan for subagents (#12918)`), `f0a85ded1867072d5b4963b8be3be1ef4f216549` (`fix: ctrl c sub agent (#12911)`).
- Developer info: minor naming, history metadata scan and Ctrl-C behavior fixes around subagents.
- Why: subjects/diffs establish local maintenance issues; detailed product rationale is не установлено локально.
- User info: sub-agent naming/interrupt behavior becomes less surprising.
- Compatibility/risks: these are stability fixes rather than new API surfaces.
- Evidence: `EVID-106-polish`.

## `rust-v0.107.0`

- Release commit: `19f8797c0f62abecb347e817aac36d18c5fc554e`
- Дата: `2026-03-02T10:17:13-07:00`
- Subject: release notes include forking a thread into sub-agents; full release subject verified locally through tag commit metadata.

### `d3603ae5d38ab3addbf995ee8c51a22ceb068872` - `feat: fork thread multi agent (#12499)`

- Дата: `2026-02-26T18:01:53Z`; PR: `#12499`; поколение: old multi-agent thread fork.
- Developer info: adds thread forking into multi-agent/sub-agent workflow through `core/src/agent/control.rs`, `core/src/thread_manager.rs`, `core/src/tools/handlers/multi_agents.rs`, tool spec and subagent notification tests.
- Why: release note and diff establish the feature: branch/fork work into sub-agents without leaving the current conversation.
- User info: users/model can fork an existing thread context into a sub-agent, instead of spawning from scratch with only a prompt.
- Compatibility/risks: forked sub-agents must preserve thread context, config and lineage; fork features must use native thread manager/control paths.
- Evidence: `EVID-107-d3603ae5`.

### `3404ecff153241c688fd34307a4049acf92ad561` - `feat: add post-compaction sub-agent infos (#12774)`

- Дата: `2026-02-26T18:55:34Z`; PR: `#12774`; поколение: sub-agent persistence/resume after compaction.
- Developer info: persists/reconstructs sub-agent info after compaction so compacted history does not lose child-agent context.
- Why: subject/diff establish post-compaction sub-agent info preservation.
- User info: sub-agent context survives compaction more reliably.
- Compatibility/risks: compaction must preserve lineage/activity metadata, not only text transcript.
- Evidence: `EVID-107-3404ecff`.

## `rust-v0.108.0`

- Release commit: `89b79419a1e0720856d4450cf17379221e5a3b1d`
- Дата: `2026-03-04T11:55:40-08:00`
- Subject: release notes include enabling experimental multi-agent mode from `/agent`, faster subagent startup, approvals and clearer naming; full release subject verified locally through tag commit metadata.

### Experimental multi-agent mode through `/agent` (`#12935`, `#12995`, `#13246`, `#13249`, `#13218`, `#13404`, `#13460`)

- Коммиты: `2b38b4e03bcb33fb5e04bb0771714dfd9b759d6d` (`feat: approval for sub-agent in the TUI (#12995)`), `eec3b1e235ff59582acd1bb37fe3ad7df2adbe3b` (`Speed up subagent startup (#12935)`), `9a42a56d8f0a14298c0e54b1ac6d7dda48ec347b` (`chore: `/multiagent` alias for `/agent` (#13249)`), `f8838fd6f3a22f228a1ed551df67789b25c82b12` (`feat: enable ma through `/agent` (#13246)`), `2e154a35bc6df9239dff23b6552076fc2a8a49b1` (`Add role-specific subagent nickname overrides (#13218)`), `932ff2818320e7c5181a28316db75e53620aeabc` (`feat: better multi-agent prompt (#13404)`), `bda3c49dc4a4ea052ed55faf9c4310e6875ca41e` (`feat: disable request input on sub agent (#13460)`).
- Developer info: connects TUI `/agent` to enabling multi-agent mode, adds `/multiagent` alias, TUI sub-agent approval handling, faster startup, role-specific nicknames, prompt improvements and a guard disabling `request_user_input` on sub-agents.
- Why: release note and subjects establish this as the user-facing activation path for experimental multi-agent mode.
- User info: users can enable experimental multi-agent mode from `/agent`; sub-agents start faster, are easier to distinguish by nickname, and approval/user-input behavior is safer.
- Compatibility/risks: `/agent` is now more than navigation; it can enable mode. Fork UI changes must keep activation, navigation, approval routing and request-input guards aligned.
- Evidence: `EVID-108-agent-mode`.

### Runtime fixes for thread-spawn subagents (`#13052`, `#13248`, `#13235`, `#13240`)

- Коммиты: `c2e126f92ad560cfbda3a542db4e669680af9c25` (`core: reuse parent shell snapshot for thread-spawn subagents (#13052)`), `3166a5ba82...` (`fix: agent race (#13248)`), `cacefb5228...` (`fix: agent when profile (#13235)`), `e4a202ea52...` (`fix: pending messages in `/agent` (#13240)`).
- Developer info: stabilizes startup/race/profile/pending-message behavior for thread-spawn subagents.
- Why: subjects/diffs establish runtime stabilization; detailed product reason beyond observed bugs is не установлено локально.
- User info: fewer failed/racy sub-agent startups and less lost pending input while using `/agent`.
- Compatibility/risks: thread-spawn subagents should inherit parent shell snapshot and profile-derived config through native setup.
- Evidence: `EVID-108-runtime-fixes`.

## `rust-v0.109.0`

- Release commit: `37133fb8455766068008b8453a1da1dacae4274c`
- Дата: `2026-03-04T15:13:23-08:00`
- Subject: release notes repeat multi-agent TUI improvements from the previous close release window; stable interval contains only the release commit.
- Результат triage: новых functional multi-agent commits in `rust-v0.108.0^{}..rust-v0.109.0^{}` beyond release summary не найдено.
- Evidence: `EVID-109-no-change`.

## `rust-v0.110.0`

- Release commit: `77aabe4218ab7ddaf4b6d471887bda043a4c16e6`
- Дата: `2026-03-04T17:39:17-08:00`
- Subject: release notes focus on connector permissions, Windows sandbox setup and plugin install; direct multi-agent entry is a TUI notification fix.

### `f80e5d979d7a1872da11520507ae881860fa4268` - `Notify TUI about plan mode prompts and user input requests (#13495)`

- Дата: `2026-03-04T15:08:57-07:00`; PR: `#13495`; поколение: TUI notifications for Plan/user input in multi-agent context.
- Developer info: notifies TUI about plan-mode prompts and user-input requests, bridging background/sub-agent activity to visible UI.
- Why: subject/diff establish missing TUI notifications for plan/user-input prompts.
- User info: users are alerted when plan-mode prompts or user input requests need attention, including during multi-agent work.
- Compatibility/risks: prompts requiring user action must be projected through event/TUI notification surfaces, not only stored in thread history.
- Evidence: `EVID-110-f80e5d97`.

## `rust-v0.111.0`

- Release commit: `8c75cd9afcd405d134530e53c78e5e0e4e5312a3`
- Дата: `2026-03-05T10:03:09-08:00`
- Subject: release notes focus on Fast mode, `js_repl`, plugin discovery, MCP elicitation and image workflow support.
- Результат triage: прямых multi-agent/collab/subagent/Plan changes in `rust-v0.110.0^{}..rust-v0.111.0^{}` не найдено.
- Evidence: `EVID-111-no-change`.

## `rust-v0.112.0`

- Release commit: `0ec16b2d9dd7d92f5661b132afdf4df3abb3d443`
- Дата: `2026-03-08T12:47:46-07:00`
- Subject: release notes focus on `@plugin` mentions, model picker and permission profiles.
- Результат triage: прямых multi-agent/collab/subagent/Plan changes in `rust-v0.111.0^{}..rust-v0.112.0^{}` не найдено.
- Evidence: `EVID-112-no-change`.

## `rust-v0.113.0`

- Release commit: `81c4928825d1e468447a17d6bc74b9abb48743f4`
- Дата: `2026-03-09T21:17:46-07:00`
- Subject: release notes focus on request permissions, plugin workflows, app-server command execution, web search and sandbox policy.

### `e84ee33cc02e693a3cf66204c72cb37e8dda3ed6` - `Add guardian approval MVP (#13692)`

- Дата: `2026-03-07T05:40:10-08:00`; PR: `#13692`; поколение: Smart Approvals guardian subagent precursor.
- Developer info: adds an MVP guardian approval path that later evolves into Smart Approvals guardian review across core/app-server/TUI.
- Why: subject/diff establish a guardian approval MVP.
- User info: approval review starts moving toward delegated/guardian-style evaluation.
- Compatibility/risks: guardian is a subagent-like approval reviewer path; policy, prompt and app-server projection must stay aligned before treating it as stable user functionality.
- Evidence: `EVID-113-e84ee33c`.

## `rust-v0.114.0`

- Release commit: `b9904c0ae4ecb773549efd6ea3fb05229402fdb9`
- Дата: `2026-03-10T16:35:03-07:00`
- Subject: release notes focus on code mode, hooks, health checks, disabling bundled skills and realtime handoffs.
- Результат triage: direct multi-agent timeline entries не найдены; `c6343e0649` thread-level elicitation counter is useful app-server/elicitation infrastructure but not a multi-agent behavior change.
- Evidence: `EVID-114-no-change`.

## `rust-v0.115.0`

- Release commit: `f028679abb30051cec2434e624cd99975986b41b`
- Дата: `2026-03-16T11:43:12-07:00`
- Subject: release notes include Smart Approvals routing review requests through a guardian subagent; full release subject verified locally through tag commit metadata.

### Smart Approvals guardian and old multi-agent hardening (`#13860`, `#14668`, `#14160`, `#14177`, `#14273`, `#14535`, `#14536`, `#14603`, `#14631`, `#14622`, `#14650`, `#13850`)

- Коммиты: `bc24017d64829d0b97b8bc6ed529a389e1e8bc1b` (`Add Smart Approvals guardian review across core, app-server, and TUI (#13860)`), `6fdeb1d602842b80088641b941dea174435c01b7` (`Reuse guardian session across approvals (#14668)`), `91ca20c7c39e326aa995c350cb68547d57a9bf54` (`Add spawn_agent model overrides (#14160)`), `a67660da2d274282c9c8dee7101787bf023e6f94` (`Load agent metadata from role files (#14177)`), `285b3a51435d3ff1da7e4e78b613d2f451f04915` (`Show spawned agent model and effort in TUI (#14273)`), `793bf32585c31e5c3a33a538bc816c8023074da7` (`Split multi-agent handlers per tool (#14535)`), `7626f612748515d6d79e149c2ae37d7d783cf989` (`Add typed multi-agent tool outputs (#14536)`), `8e89e9ededc64253c228749521fc9d8049f8947b` (`Split multi-agent handler into dedicated files (#14603)`), `cfd97b36da76a17db407b2d9653ed993636e0a30` (`Rename multi-agent wait tool to wait_agent (#14631)`), `36dfb844277e79793766f96305c9633f90bc043e` (`Stabilize multi-agent feature flag (#14622)`), `7f571396c8819d7f4c4486ed1e967e40a2c9ffae` (`fix: sync split sandbox policies for spawned subagents (#14650)`), `3f266bcd68c78ac043969f8a7a916c7ee30df112` (`feat: make interrupt state not final for multi-agents (#13850)`).
- Developer info: introduces guardian approval review across core/app-server/TUI and reuses guardian sessions; old multi-agent gets model overrides, role metadata from role files, spawned model/effort TUI display, split handlers, typed outputs, `wait_agent` naming, stable feature flag, sandbox policy sync and non-final interrupt state.
- Why: release note establishes guardian subagent purpose; other subjects/diffs establish hardening and API cleanup for old multi-agent.
- User info: Smart Approvals can use a guardian subagent to reduce repeated setup in approvals; multi-agent tools become more typed, more modular and clearer in TUI.
- Compatibility/risks: approval reviewer state is cross-surface (`Config`, app-server schema, core, TUI). `wait_agent` rename and typed outputs are compatibility points for model-visible tools and tests.
- Evidence: `EVID-115-guardian-old-ma`.

### `a67660da2d274282c9c8dee7101787bf023e6f94` - `Load agent metadata from role files (#14177)`

- Дата: `2026-03-11T12:33:08-07:00`; PR: `#14177`; поколение: role-file metadata and autodiscovery.
- Developer info: добавляет `core/src/config/agent_roles.rs` как отдельный loader, который объединяет role declarations из `[agents.<name>]` с `.toml` файлами в `agents/` рядом с каждым config layer. Явно объявленные `config_file` добавляются в `declared_role_files`, чтобы тот же файл не был повторно подхвачен autodiscovery. Role file парсится как строгий `RawAgentRoleFileToml`: metadata-поля `name`, `description`, `nickname_candidates` плюс flatten в обычный `ConfigToml`; перед применением config-layer metadata-поля удаляются из TOML.
- User info: пользователь может держать роль целиком в `~/.codex/agents/<role>.toml` или project-layer `.codex/agents/<role>.toml`, а не дублировать описание в `config.toml`. Discovered role file должен иметь `name`, `description` и `developer_instructions`; legacy split form через `[agents.<name>].config_file` может получать `description` из config и поэтому допускает role file без `developer_instructions`.
- Why: до этого configurable roles были split contract: `[agents.<name>]` в config описывал роль, а `config_file` только применял настройки агента. Этот коммит делает role file самостоятельным authoring unit и native source для metadata, nickname candidates и config overrides.
- Compatibility/risks: loader использует `serde(deny_unknown_fields)`, поэтому любые поля вне metadata и `ConfigToml` превращают role file в invalid config. В первоначальном виде этого коммита invalid role мог остановить config load; это исправлено следующим hardening-коммитом `#14488`. При конфликте role name внутри одного layer duplicate должен быть ошибкой/предупреждением, а при переопределении между layer higher-precedence role может наследовать недостающие metadata-поля из lower-precedence role.
- Verification notes: покрытие включает сценарии `agent_role_file_metadata_overrides_config_toml_metadata`, `discovers_multiple_standalone_agent_role_files`, `agent_role_file_name_takes_precedence_over_config_key`, `loads_legacy_split_agent_roles_from_config_toml`, `agent_role_file_without_developer_instructions_is_dropped_with_warning`, `discovered_agent_role_file_without_name_is_dropped_with_warning` и inheritance между config layers.
- Evidence: `EVID-115-agent-role-files`.

### `4fa7d6f444b919afb6ccec25e49c036aa0180971` - `Handle malformed agent role definitions nonfatally (#14488)`

- Дата: `2026-03-12T11:20:31-07:00`; PR: `#14488`; поколение: nonfatal role config diagnostics.
- Developer info: `load_agent_roles` начинает принимать `startup_warnings`, ошибки разбора `[agents]`, declared roles, discovered role files, duplicate names и missing descriptions превращаются в `Ignoring malformed agent role definition: ...`, invalid role пропускается, а остальные роли продолжают загружаться. App-server получает propagation через `configWarning`, чтобы клиент видел проблему startup config без падения процесса.
- User info: один битый файл вроде `~/.codex/agents/worker.toml` больше не ломает запуск Codex целиком; Codex стартует, показывает warning и не регистрирует только malformed role. Это ровно тот behavior, который проявляется при unknown field в role file.
- Why: после `#14177` role files стали частью startup config path, поэтому authoring mistake в одном пользовательском role file мог заблокировать весь runtime. Nonfatal behavior сохраняет доступность Codex и делает проблему локальной к конкретной роли.
- Compatibility/risks: warning не означает, что роль частично применена; она полностью dropped. Это важно для `spawn_agent.agent_type`: модель не должна видеть/использовать invalid role, а app-server/TUI должны показывать warning, иначе пользователь будет думать, что role contract активен. Legacy path without config layers по-прежнему возвращает errors напрямую, поэтому compatibility зависит от того, какой loader path используется.
- Verification notes: commit message фиксирует проверки `cargo test -p codex-core agent_role_ -- --nocapture`, `just fix -p codex-core`, `just fmt`, `cargo test -p codex-app-server config_warning -- --nocapture`; current tests также проверяют warnings for missing `developer_instructions`, missing `description`, missing `name` и continued loading of valid sibling roles.
- Evidence: `EVID-115-agent-role-nonfatal`.

### Fan-out and navigation split (`#14282`, `#13923`, `#14269`, `#14410`)

- Коммиты: `a4d884c767622e694899a8ddc2de6e4c165aae1c` (`Split spawn_csv from multi_agent (#14282)`), `180a5820fc1fa3ca398f088f8906cfe74f7c22a0` (`Add keyboard based fast switching between agents in TUI (#13923)`), `ce1d9abf117651965ffb312d94929267190a3149` (`Clarify close_agent tool description (#14269)`), `bf5e997b318935732c110f3f8366394d8e1370f3` (`Include spawn agent model metadata in app-server items (#14410)`).
- Developer info: separates CSV fan-out from the core multi-agent feature, improves keyboard switching and app-server metadata, and clarifies `close_agent`.
- User info: batch/fan-out, navigation and metadata become clearer without overloading the base multi-agent surface.
- Compatibility/risks: keep fan-out-specific tools/config separate from base spawn/send/wait semantics.
- Evidence: `EVID-115-fanout-navigation`.

## `rust-v0.116.0`

- Release commit: `38771c9082535aa16b4c4d0395d3532f32f656ff`
- Дата: `2026-03-19T09:49:09-07:00`
- Subject: release notes focus on app-server TUI auth, plugin setup, `userpromptsubmit` hook and realtime sessions.

### Subagent execution context fixes (`#14935`, `#14944`, `#13702`, `#14821`)

- Коммиты: `4ed19b07664d28ef67592ab5d77aa30d13d3aba0` (`feat: rename to get more explicit close agent (#14935)`), `e8add54e5dda2fc6f49757aa939378a21b8515e9` (`feat: show effective model in spawn agent event (#14944)`), `84f4e7b39d17fea6d28c98bc748652ea4b279a14` (`fix(subagents) share execpolicy by default (#13702)`), `a265d6043edc8b41e42ae508291f4cfb9ed46805` (`feat: add memory citation to agent message (#14821)`).
- Developer info: clarifies close naming, exposes effective spawned model in events, shares exec policy for subagents by default, and adds memory citation to agent messages.
- User info: spawned agent metadata and execution policy are more transparent and consistent.
- Compatibility/risks: event metadata and execution policy must propagate through native spawn/session setup; memory citation is model/user-visible context and should not be dropped in projections.
- Evidence: `EVID-116-subagent-context`.

## `rust-v0.117.0`

- Release commit: `4c70bff480af37b1bf1a9b352b8341060fe55755`
- Дата: `2026-03-26T14:22:09-07:00`
- Subject: release notes introduce readable path-based addresses like `/root/agent_a`, structured inter-agent messaging and agent listing for multi-agent v2 workflows; full release subject verified locally through tag commit metadata.

### `79ad7b247bb6805853b00f55d2e992810ce949ea` - `feat: change multi-agent to use path-like system instead of uuids (#15313)`

- Дата: `2026-03-20T18:23:48Z`; PR: `#15313`; поколение: `multi_agent_v2`.
- Developer info: adds `AgentPath` protocol/schema support, agent resolver/guards and app-server projection updates so agents are addressed by readable paths rather than opaque UUIDs.
- Why: release note and subject establish the architectural shift to path-like addresses.
- User info: agents can be referred to as `/root/agent_a` style paths, making multi-agent workflows easier to read and direct.
- Compatibility/risks: path identity becomes source-of-truth for MAv2; fork changes must not key current behavior only by thread UUID.
- Evidence: `EVID-117-79ad7b24`.

### MAv2 structured output and messaging (`#15515`, `#15540`, `#15556`, `#15560`, `#15570`, `#15576`, `#15621`, `#15624`, `#15647`)

- Коммиты: `37ac0c093cb4be42f7812737366cab181b9d0417` (`feat: structured multi-agent output (#15515)`), `450dc289c3305bf9d94d862d6d30c4916aa2497a` (`chore: split sub-agent v2 implementation (#15540)`), `18f1a08bc9c6e39331d9cf34ee240ea0124173cb` (`feat: new op type for sub-agents communication (#15556)`), `191fd9fd16e8f4ea43adb438122fb13f1b2ed674` (`feat: use serde to differenciate inter agent communication (#15560)`), `4605c653085ac1ea3a4c48e4d1727022bd942f68` (`feat: custom watcher for multi-agent v2 (#15570)`), `527244910fb851cea6147334dbc08f8fbce4cb9d` (`feat: custom watcher for multi-agent v2 (#15576)`), `38c088ba8d03b7b211464166e04be0391e53266e` (`feat: list agents for sub-agent v2 (#15621)`), `b51d5f18c7154498524b705ab2fbb81259d2bbed` (`feat: disable notifier v2 and start turn on agent interaction (#15624)`), `773fbf56a43a38ef65e9f64da279db22264aa3d5` (`feat: communication pattern v2 (#15647)`).
- Developer info: introduces structured MAv2 outputs, splits v2 implementation, adds `InterAgentCommunication` op type and serde differentiation, custom watcher/listing, interaction-triggered turns and v2 messaging pattern including `assign_task`/`send_message` style handlers.
- Why: release note and diffs establish structured inter-agent messaging as the generation change.
- User info: MAv2 agents can be listed, watched and addressed through structured messages instead of old collab event heuristics.
- Compatibility/risks: inter-agent communication is a protocol/persistence item; fork changes must update rollout reconstruction, app-server/TUI projections and tool specs together.
- Evidence: `EVID-117-mav2-messaging`.

### V1/V2 bridge and guard fixes (`#15861`, `#15880`, `#15881`, `#15056`)

- Коммиты: `70cdb17703a4310b7173642e011f7534d2b2624f` (`feat: add graph representation of agent network (#15056)`), `4a5635b5a0336274b6ee196140bfe151b18a642d` (`feat: clean spawn v1 (#15861)`), `352f37db03315dc215fbf23cb36b442554afb8c5` (`fix: max depth agent still has v2 tools (#15880)`), `970386e8b2a776d47ef6ac6c2bd67a3c0ed86744` (`fix: root as std agent (#15881)`).
- Developer info: adds graph representation and cleans old spawn/V1 compatibility while fixing v2 availability at max depth and root-agent classification.
- User info: network visualization/mental model improves, and v2 tools are less likely to appear in invalid depth/root contexts.
- Compatibility/risks: V1 compatibility and V2 guardrails must be treated as separate layers; max-depth tool availability must be context-sensitive.
- Evidence: `EVID-117-v1-v2-guards`.

## `rust-v0.118.0`

- Release commit: `b630ce9a4e754d35a1f33e4366ba638d18626142`
- Дата: `2026-03-31T09:07:09-07:00`
- Subject: release notes focus on Windows sandbox networking, app-server sign-in and dynamic auth tokens; direct timeline entries are MAv2 spawn/mailbox and collaboration-mode/TUI fixes.

### MAv2 spawn and mailbox (`#15986`, `#15985`, `#16010`, `#16237`)

- Коммиты: `6a0c4709ca2154e9f3ebb07e58fb156386630188` (`feat: spawn v2 make task name as mandatory (#15986)`), `426f28ca99a809134bbc9d4879789582f80093c1` (`feat: spawn v2 as inter agent communication (#15985)`), `213756c9ab22b567d426fe1be9757705fb5862c9` (`feat: add mailbox concept for wait (#16010)`), `c74190a622667158e53b6d10fbfc0387ec43b964` (`fix: ma1 (#16237)`).
- Developer info: makes MAv2 spawn task naming explicit, represents spawn through inter-agent communication, adds mailbox-based wait flow and fixes the early wait implementation.
- Why: subjects/diffs establish the MAv2 runtime shift: spawn/wait should use structured inter-agent communication and mailbox state rather than ad hoc thread events.
- User info: MAv2 agents get clearer task names, more native spawn semantics and mailbox-backed result delivery.
- Compatibility/risks: high for fork work around spawn/wait/message; changing one handler without protocol/mailbox/rollout support can break result delivery.
- Evidence: `EVID-118-mav2-spawn-mailbox`.

### Collaboration mode and agent picker fixes (`#15995`, `#16014`, `#16026`, `#16110`)

- Коммиты: `37b057f0030ab34f66dbb6eb6f8e7e7a3443407c` (`Use codex-utils-template for collaboration mode presets (#15995)`), `ed977b42ac4f0b71ea218546153a751866f25b5b` (`Fix tui_app_server agent picker closed-state regression (#16014)`), `bede1d9e23202e2fce23b2ad6d154255672a675b` (`fix(tui): refresh footer on collaboration mode changes (#16026)`), `38e648ca67802d5f2fb23f9b3bd3f200cdb067fa` (`Fix tui_app_server ghost subagent entries in /agent (#16110)`).
- Developer info: reduces preset source drift, refreshes TUI footer on mode changes and fixes stale/ghost agent entries in `/agent`.
- Why: subjects/diffs show state/projection inconsistencies in collaboration-mode and agent navigation UI.
- User info: mode labels and `/agent` navigation better match actual runtime state.
- Compatibility/risks: UI must derive from runtime source-of-truth; hand-maintained lists produce stale agent entries.
- Evidence: `EVID-118-mode-agent-picker`.

## `rust-v0.119.0`

- Release commit: `4a3466efbf84cfb7469eca94bbf6307166c9f48e`
- Дата: `2026-04-10T13:45:23-07:00`
- Subject: `## New Features`

### MAv2 tool contract simplification (`#15771`, `#16325`, `#16406`, `#16409`, `#16424`, `#16571`, `#16746`, `#17005`, `#17008`, `#17009`, `#17071`)

- Коммиты: `1fc8aa0e169c74b571960f529baafc17d686beda` (`feat: fork pattern v2 (#15771)`), `285f4ea8176c74934e22fc9f78e216b9c7da429c` (`feat: restrict spawn_agent v2 to messages (#16325)`), `d0474f2bc1eb246c206c1e9c02338f52bf8d992e` (`Use message string in v2 spawn_agent (#16406)`), `23d638a573ab0775c8e8c7f54db8e19ddbb10332` (`Use message string in v2 send_message (#16409)`), `0c776c433b02ae4e07efc2db9eac0e55455630a3` (`feat: tasks can't be assigned to root agent (#16424)`), `7fc36249b5f8661fd067018282e68c12a3ae0912` (`chore: rename assign_task for followup_task (#16571)`), `8a19dbb1776d5a99df3ddb6d79ace2a7b2b93620` (`Add spawn context for MultiAgentV2 children (#16746)`), `2a8c3a2a527d1b3cb51365a72552bfa3aff32040` (`feat: drop agent ID from v2 (#17005)`), `68e16baabe79a1f48c7d93c9a3d6e234a9b2c9b7` (`chore: send_message and followup_task do not return anything (#17008)`), `4cc6818996bbe47b4f490f9ca3ce3cd8504bf6c3` (`chore: keep request_user_input tool to persist cache on multi-agents (#17009)`), `4c07dd4d25006a41d35872f520aa2e18bc3200e7` (`Configure multi_agent_v2 spawn agent hints (#17071)`).
- Developer info: adds controlled fork slices, narrows spawn/message payloads to message strings, prevents root-agent assignment, renames `assign_task` to `followup_task`, adds child spawn context, removes v2 `agent_id`, makes message/followup tools outputless and wires MAv2 usage hints/config.
- Why: subjects/diffs establish convergence toward a smaller, clearer model-visible MAv2 contract.
- User info: MAv2 tools become easier for the model to call correctly: path/context driven, with `followup_task` semantics and fewer return/output ambiguities.
- Compatibility/risks: this is a schema/behavior churn window. Fork code must preserve aliases/compatibility where current runtime still supports them and should not add functionality only to retired names such as `assign_task`.
- Evidence: `EVID-119-mav2-contract`.

### Mailbox preemption and guardian schema (`#16725`, `#17061`)

- Коммиты: `e4f1b3a65e06a74e2716b01e695a6c1d37a8fdbd` (`Preempt mailbox mail after reasoning/commentary items (#16725)`), `dcbc91fd39ba4f3d90fbfd96fa7ba6e8cf9a6159` (`Update guardian output schema (#17061)`).
- Developer info: changes where inter-agent mailbox messages can preempt ongoing work and updates guardian review output schemas across app-server/core/TUI.
- Why: diff shows concurrency and schema correctness work for agent/guardian flows.
- User info: inter-agent messages are delivered at safer item boundaries, and guardian review outputs become typed/current.
- Compatibility/risks: mailbox preemption semantics changed again in later releases; history must record this as a transitional behavior.
- Evidence: `EVID-119-mailbox-guardian`.

## `rust-v0.120.0`

- Release commit: `65319eb1400cbd2890c43d572263dabd25f18ba9`
- Дата: `2026-04-10T18:58:04-07:00`
- Subject: `## New Features - Realtime V2 can now stream background agent progress while work is still running and queue follow-up responses until the active response completes (#17264, #17306) - Hook activity in the TUI is easier to scan, with live running hooks shown separately and completed hook output kept only when useful (#17266) - Custom TUI status lines can include the renamed thread title (#17187) - Code-mode tool declarations now include MCP `outputSchema` details so structured tool results are typed more precisely (#17210) - SessionStart hooks can distinguish sessions created by `/clear` from fresh startup or resume sessions (#17073)`

### Realtime background agent (`#17278`, `#17264`, `#17306`, `#17363`)

- Коммиты: `60236e8c920f59ddec4faa70f7a7d58a2984b2f9` (`Rename Realtime V2 tool to background_agent (#17278)`), `1de0085418340b3e7f7136cfb5e56b4bebafc584` (`Stream Realtime V2 background agent progress (#17264)`), `2e81eac004e280fdb447f0d8b74dfc76f4db2913` (`Queue Realtime V2 response.create while active (#17306)`), `029fc63d13b943c1f10c8c66da6fa16488fc1ed5` (`Strengthen realtime backend delegation prompt (#17363)`).
- Developer info: renames the realtime delegation tool to `background_agent`, streams background progress, queues `response.create` while active and strengthens the backend delegation prompt.
- Why: release note and diffs establish a realtime/background-agent collaboration flow.
- User info: realtime users see progress from background work and can queue follow-up responses without racing the active response.
- Compatibility/risks: tool rename and streaming order are protocol/model-visible; clients must handle progress before final output.
- Evidence: `EVID-120-background-agent`.

### Guardian context and MAv2 descriptions (`#17249`, `#17269`, `#17194`, `#17298`, `#17338`)

- Коммиты: `4e910bf151d5a037dc522939c837ec5a81b516ec` (`adding parent_thread_id in guardian (#17249)`), `88165e179a9547e6ff56a6538462f6efdf04db46` (`feat(guardian): send only transcript deltas on guardian followups (#17269)`), `147cb8411267062c5c2b93e95171ec13c943338c` (`add parent-id to guardian context (#17194)`), `a3be74143ab7a1d9641b234ffb11f826cee471e1` (`fix(guardian, app-server): introduce guardian review ids (#17298)`), `d39a722865f5e6af210f752a6603329b2566f676` (`feat: description multi-agent v2 (#17338)`).
- Developer info: adds parent thread/context IDs and guardian review IDs, sends transcript deltas on follow-ups and improves MAv2 tool descriptions.
- Why: subjects/diffs establish guardian lifecycle identification and lower-noise follow-up context.
- User info: guardian reviews become easier to correlate in app-server/TUI, and MAv2 tool descriptions become clearer to the model.
- Compatibility/risks: app-server clients must track review IDs and schema updates; prompt/tool descriptions are model-visible behavior.
- Evidence: `EVID-120-guardian-mav2`.

## `rust-v0.121.0`

- Release commit: `d65ed92a5e440972626965d0af9a6345179783bc`
- Дата: `2026-04-15T20:43:45+01:00`
- Subject: release notes include Plan/status, memory controls, realtime APIs and plugin/MCP work; direct entries here are guardian timeout, agent identity, Plan surface and mailbox behavior.

### Guardian timeout, agent identity and Plan notifications (`#17381`, `#17385`, `#17386`, `#17419`, `#17417`)

- Коммиты: `37aac89a6db500b8647f8b0fc9251e629eecea51` (`representing guardian review timeouts in protocol types (#17381)`), `39cc85310fbb1c4d04034e596cd7420090875799` (`Add use_agent_identity feature flag (#17385)`), `8e784bba2fb795ec9248bbfd9a26fdad1b809dc6` (`Register agent identities behind use_agent_identity (#17386)`), `3b948d9dd8d2e13a36e95a05efb6bb2288b801c4` (`Support prolite plan type (#17419)`), `ce5ad7b295afbaa763b595eef5501ce4c3eb84ab` (`Emit plan-mode prompt notifications for questionnaires (#17417)`).
- Developer info: extends guardian review status with timeout representation, introduces feature-gated agent identity registration, adds `prolite` plan type handling and emits Plan questionnaire notifications in TUI.
- Why: subjects/diffs show protocol/status and UX gaps in guardian/Plan/identity surfaces.
- User info: clients can represent guardian timeouts, agent identity can be registered behind a feature flag, and Plan questionnaires surface notifications.
- Compatibility/risks: agent identity is gated; Plan type/status additions must be projected consistently across protocol and TUI.
- Evidence: `EVID-121-guardian-identity-plan`.

### Forked spawn inheritance and mailbox drain boundary (`#17247`, `#17749`)

- Коммиты: `776246c3f5f931a72ebe41f52ef719e3b413371d` (`Make forked agent spawns keep parent model config (#17247)`), `05c582992359e47afaa298c045c62af42001a463` (`[codex] drain mailbox only at request boundaries (#17749)`).
- Developer info: preserves parent model config for forked agent spawns and changes mailbox drain to request boundaries.
- Why: subjects/diffs show inherited config and mailbox preemption needed tightening.
- User info: forked agents inherit the expected model config; inter-agent mail no longer interrupts mid sampling stream in this release.
- Compatibility/risks: mailbox drain behavior is explicitly reverted in the next release; fork history should keep both sides of the change.
- Evidence: `EVID-121-fork-mailbox`.

## `rust-v0.122.0`

- Release commit: `230dcadee609fa99d6162fe1107457030e5270a7`
- Дата: `2026-04-20T18:10:09+01:00`
- Subject: release notes include `/side`, Plan clear-context implementation, marketplace/plugin workflow, sandbox permissions and tool discovery/image generation.

### Agent identity/task lifecycle and policy (`#17387`, `#17978`, `#18596`, `#18599`, `#17980`, `#18654`)

- Коммиты: `55c3de75cba1a65088ff02b91527b94a1b60a69b` (`Register agent tasks behind use_agent_identity (#17387)`), `e5b52a3caa7a050c4df81570f5698d74793ef942` (`Persist and prewarm agent tasks per thread (#17978)`), `49403e3676f8f3685f0d4ba4a1292b8953d05b42` (`chore(multiagent) skills instructions toggle (#18596)`), `0500801123ada245ba752cf0577ca08cc7fc0065` (`fix(guardian) disable skills message in guardian thread (#18599)`), `b44d2851cf0e7b728d4a848d4a4f68da3483dfd6` (`[codex] Use AgentAssertion downstream behind use_agent_identity (#17980)`), `fc758af9eb4ba82af629493d4ada6b0f0ac66973` (`fix: exec policy loading for sub-agents (#18654)`).
- Developer info: registers and persists agent tasks behind `use_agent_identity`, prewarms tasks per thread, adds a multiagent skills-instructions toggle, disables skills messages in guardian threads, uses downstream `AgentAssertion` and fixes exec-policy loading for sub-agents.
- Why: subjects/diffs establish a feature-gated agent identity/task lifecycle and sub-agent policy hardening.
- User info: agent tasks become durable/reusable per thread, guardian prompts are cleaner, and sub-agents load execution policy correctly.
- Compatibility/risks: identity/assertion and task persistence are cross-cutting; fork work must update config, auth, state and session lifecycle together.
- Evidence: `EVID-122-agent-identity-task-policy`.

### Plan clear-context, side conversations and mailbox revert (`#17499`, `#18573`, `#18190`, `#18325`)

- Коммиты: `d3692b14c900560c3775e3cdaf2f0d9a9b37902c` (`feat(tui): add clear-context plan implementation (#17499)`), `241136b0e9cd5d787d74a905af315252930c886b` (`feat(tui): show context used in plan implementation prompt (#18573)`), `95dafbc7b5b5c6905db9015b3850dc5967523b2a` (`Add /side conversations (#18190)`), `e3c2acb9cdd37277c334765a30ea2ac6020a2b9f` (`Revert "[codex] drain mailbox only at request boundaries" (#18325)`).
- Developer info: lets Plan implementation start in a fresh context, shows context usage before handoff, adds `/side` forked side conversations and reverts request-boundary-only mailbox draining.
- Why: release note and diffs establish new Plan handoff and side-conversation UX, plus correction of mailbox behavior.
- User info: users can implement a plan without carrying the whole planning context, ask side questions without disrupting the main thread, and inter-agent mail can again preempt after reasoning/commentary boundaries.
- Compatibility/risks: side/clear-context flows are fork-like; they must preserve inherited config/history guardrails. Mailbox behavior differs from `rust-v0.121.0`, so timeline consumers should not treat `#17749` as final.
- Evidence: `EVID-122-plan-side-mailbox`.

## `rust-v0.123.0`

- Release commit: `0785b66228dff87f891e291cb5686631865b6922`
- Дата: `2026-04-22T17:25:21-07:00`
- Subject: release notes include realtime handoff improvements for background agents, `/mcp verbose`, remote sandbox requirements, model metadata refresh and dependency/build chores; direct entries here are MAv2 fork/model defaults, realtime background-agent handoff, external-agent config ownership and side-conversation parent status.

### MAv2 full-history fork and inherited model defaults (`#18873`, `#18701`)

- Коммиты: `15b8cde2a4f63bb510d46c88d3ddd7e286bee203` (`chore: default multi-agent v2 fork to all (#18873)`), `54bd07d28c8c8684217fd5ab83c0e7d943d06d91` (`[codex] prefer inherited spawn agent model (#18701)`).
- Developer info: changes the MAv2 `spawn_agent` handler so omitted `fork_turns` defaults to `all`, excludes `TurnContext` from forked rollout copies so child runtime config establishes a fresh context diff baseline, and rewrites the shared `spawn_agent` tool spec/model descriptions so model override is explicitly optional and parent-model inheritance is preferred.
- Why: commit bodies and diffs state that full-history fork should default to all for fork mode and that sub-agents should inherit the parent model by default; `#18701` specifically cites reports where `5.4` picked `5.2` or lower for sub-agents.
- User info: model-visible `spawn_agent` guidance now makes full-history MAv2 fork the default and discourages unnecessary model downgrades; a user asking for delegated work is less likely to get a weaker sub-agent unless an override is explicit.
- Compatibility/risks: defaulting `fork_turns` to `all` increases inherited context by default; fork code that expected `None`/no-fork must explicitly pass `none`. The wording also changes model-visible behavior and tests, so forks must keep tool spec, handler defaults and spawn tests in sync.
- Evidence: `EVID-123-mav2-fork-model`.

### Realtime background-agent handoff context and silence (`#18597`, `#18761`, `#18635`)

- Коммиты: `126bd6e7a8839a861ee9bb40ec72c72ea1bf7b4d` (`Update realtime handoff transcript handling (#18597)`), `ca3246f77a5eca14f3424d786ce8855fe8811dbc` (`[codex] Send realtime transcript deltas on handoff (#18761)`), `1029742cf7747ad2e82fe10d931744372902e2d6` (`Add realtime silence tool (#18635)`).
- Developer info: adds active transcript accumulation in the realtime websocket layer, serializes hidden `<realtime_delegation>` envelopes with `<input>` and `<transcript_delta>` in `core/src/realtime_conversation.rs`, changes repeated handoffs to send only new transcript entries, adds Realtime V2 `remain_silent` as a function tool, parses it into `RealtimeEvent::NoopRequested`, replies with empty `function_call_output`, and keeps the event hidden from app-server/TUI surfaces.
- Why: release notes and commit bodies explicitly describe sharing realtime transcript deltas with the background agent and adding `remain_silent` when collaboration context says the realtime model should not speak aloud.
- User info: realtime users get background-agent handoffs with the same surrounding conversation context, repeated handoffs avoid resending old transcript, and silent/background updates can avoid distracting spoken acknowledgements.
- Compatibility/risks: `<transcript_delta>` is hidden context, not UI transcript; protocol clients must handle `RealtimeEvent::NoopRequested`, and realtime V2 tool names `background_agent`/`remain_silent` are model-facing contract. Fork work that changes realtime delegation must update `codex-api`, `codex-core`, `codex-protocol` and tests together.
- Evidence: `EVID-123-realtime-handoff`.

### External-agent config ownership and side-conversation parent status (`#18850`, `#18591`)

- Коммиты: `833212115e18e9eb75b0a75dcecf4034b8bab6ac` (`Move external agent config out of core (#18850)`), `0dc503ba6e0f82a27fc676bcdc8bec642b20e6fe` (`Surface parent thread status in side conversations (#18591)`).
- Developer info: moves `ExternalAgentConfigService` and migration tests from `codex-core` to `app-server/src/config`, keeps external-agent migration service crate-private to app-server, updates `external_agent_config_api` imports, and adds TUI side-conversation state for parent status, deferred interactive overlays, replay filtering and quiet session updates.
- Why: `#18850` body states the service belongs in app-server rather than core; `#18591` body states side conversations hid important parent-thread changes such as completion, failure, input requests and approvals.
- User info: external-agent config migration becomes an app-server-owned surface, and `/side` users can see lightweight parent/main-thread status while staying in the side conversation; pending approvals or user-input prompts are restored when returning to the main thread.
- Compatibility/risks: external-agent config is no longer a `codex-core` public module, so fork extensions should not add new external-agent migration behavior in core. Side conversations now suppress/restore interactive overlays across thread switches; TUI changes must preserve replay filters and parent status labels.
- Evidence: `EVID-123-external-side`.

## `rust-v0.124.0`

- Release commit: `e9fb49366c93a1478ec71cc41ecee415a197d036`
- Дата: `2026-04-23T19:23:03+02:00`
- Subject: release notes include app-server managed environments, turn-scoped environment selection, permission-mode fixes, `wait_agent` queued mailbox fix, hooks stability, remote plugin marketplaces and reasoning shortcuts.

### Multi-agent wait and legacy wrapper hardening (`#18968`, `#19059`)

- Коммиты: `639382609f8a96b4bf255fa2e735e8fb1aca4531` (`fix: wait_agent timeout for queued mailbox mail (#18968)`), `3cc3763e6c0473d6f6666524709528fdcdb7d9c1` (`core: box multi-agent wrapper futures (#19059)`).
- Developer info: adds `Session::has_pending_mailbox_items()` and makes `multi_agents_v2::wait` return immediately when mailbox mail is already queued; boxes old multi-agent wrapper futures around spawn/resume/close paths to reduce Windows stack usage without behavior changes.
- Why: `#18968` commit body describes a concrete wait race where `wait_agent` subscribed after mail was queued and then timed out; `#19059` body links stack overflow failures to legacy multi-agent wrapper future size.
- User info: `wait_agent` no longer waits until timeout when a child result is already pending; legacy collab spawn/close/resume paths become more reliable on Windows.
- Compatibility/risks: wait behavior changes from timeout-prone to immediate delivery for queued mail. Wrapper boxing is intended behavior-neutral, but it touches old collab lifecycle functions and should stay covered by cascade close/resume tests.
- Evidence: `EVID-124-wait-wrapper`.

### AgentIdentity reset and explicit auth mode (`#18757`, `#18785`)

- Коммиты: `be75785504ff152fa6333e380a2d50642f42fba0` (`fix: fully revert agent identity runtime wiring (#18757)`), `69c8913e24f2f455c8f000fa7afe039a38bdd48d` (`feat: add explicit AgentIdentity auth mode (#18785)`).
- Developer info: removes the earlier broad AgentIdentity runtime wiring, rollout/session persistence, feature flag and lazy auth mutation, then reintroduces identity as explicit `CodexAuth::AgentIdentity` plus a standalone `agent-identity` crate and auth-manager startup task.
- Why: commit bodies state the old stack was reverted to regain a clean base and reintroduced in smaller layers; `#18785` makes AgentIdentity a real auth enum variant selected by `auth.json`, not a hidden feature path.
- User info: agent identity moves from hidden experimental runtime mutation toward an explicit authentication mode; behavior is mostly infrastructure-visible in this release rather than a new end-user multi-agent command.
- Compatibility/risks: this is a churn window. Fork code must not depend on removed rollout/session persistence from the reverted stack, and should treat AgentIdentity task state as runtime-only rather than persisted thread metadata.
- Evidence: `EVID-124-agent-identity`.

### PermissionProfile as thread/fork/turn contract (`#18278`, `#18279`, `#18280`, `#18281`, `#18282`, `#19086`, `#18924`)

- Коммиты: `5eab9ff8ca5e994d68a8a9bf26f0700a937bafbc` (`app-server: expose thread permission profiles (#18278)`), `18a26d7bbc9dfe7509dc0bac134af604d4f0d145` (`app-server: accept permission profile overrides (#18279)`), `bc083e47137431a89d2aab1ff6f21c5b15062b49` (`clients: send permission profiles to app-server (#18280)`), `6ca038bbd121b1a20d8d469ac3e57b3619a3c888` (`rollout: persist turn permission profiles (#18281)`), `082fc4f632800f6aa5b0b3badcfbe3aa300f24a7` (`protocol: report session permission profiles (#18282)`), `8bc667b07be22c005769e814bc529e30cff1ea27` (`app-server: include filesystem entries in permission requests (#19086)`), `08b5e96678331cb75a124d552d6e6c2bc5813a3a` (`TUI: preserve permission state after side conversations (#18924)`).
- Developer info: adds canonical app-server v2 `PermissionProfile` response fields for thread start/resume/fork, accepts profile-shaped overrides for `thread/start`, `thread/resume`, `thread/fork` and `turn/start`, sends profiles from first-party clients where representable, persists turn `permission_profile` in rollouts, reports it in `SessionConfigured`, fixes app-server permission request filesystem `entries`, and syncs TUI cached session state after `/permissions` changes so `/side` replay does not restore stale permissions.
- Why: commit bodies describe the migration from legacy sandbox fields to a canonical permission shape that can round-trip through app-server, clients, rollout and TUI without losing deny-read/glob metadata.
- User info: permissions chosen for sessions, turns and side-conversation returns survive more reliably; app-server clients can display and echo the active permission profile instead of guessing from legacy sandbox fields.
- Compatibility/risks: legacy `sandbox`/`sandboxPolicy` fields remain compatibility surfaces and ambiguous requests are rejected. Fork multi-agent work must propagate permission changes through `TurnContext`, thread fork/start params, rollout reconstruction and TUI session snapshots rather than storing a separate permission flag.
- Evidence: `EVID-124-permission-profile`.

### Managed environments, turn selection and Plan reasoning UX (`#18401`, `#18416`, `#18866`)

- Коммиты: `ddbe2536be03af5e9b84f2890efba14543d6e7b5` (`Support multiple managed environments (#18401)`), `1d4cc494c9c0ac94e8a1eda7acf420e5df85e24f` (`Add turn-scoped environment selections (#18416)`), `e502f0b52d490a0953579b269b1c080d62b8fb0e` (`feat(tui): shortcuts to change reasoning level temporarily (#18866)`).
- Developer info: refactors `EnvironmentManager` around keyed environments, adds experimental `turn/start.environments` params for per-turn environment id and cwd selection, resolves selections before `TurnContext` creation, and lets TUI reasoning shortcuts temporarily adjust active-session reasoning including Plan mode without opening the global-vs-Plan scope prompt.
- Why: release notes and commit bodies identify multi-workspace/remote targeting and per-turn environment/cwd selection as the motivation; `#18866` states Plan mode should get temporary reasoning changes directly.
- User info: app-server sessions can target managed environments/cwds per turn, which is important for forks and subagents that inherit turn context; Plan mode users can nudge reasoning effort quickly without changing persistent defaults.
- Compatibility/risks: environment selection is experimental and part of `turn/start`; fork changes must resolve environment/cwd before constructing child `TurnContext`. Plan reasoning shortcuts are TUI/session state, not config defaults.
- Evidence: `EVID-124-env-plan`.

## `rust-v0.125.0`

- Release commit: `637f7dd6d737f3961e6bf32fbb3861c4953269c5`
- Дата: `2026-04-24T18:55:17+02:00`
- Subject: release notes include app-server resume/fork pagination, sticky environments, remote thread config/store plumbing, permission profile round-trip, rollout tracing, and config/schema handling around MAv2 and agent role config paths.

### MAv2 config contract and spawned context cleanup (`#19129`, `#19127`, `#19261`)

- Коммиты: `d3b044938d245b519c1a5baefe880ef89e3a30c1` (`Reject agents.max_threads with multi_agent_v2 (#19129)`), `a2f868c9d6a208572992da1ad7a3ea8ed00b84a1` (`feat: drop spawned-agent context instructions (#19127)`), `d87d9187162e8db2bcc287e141afea36369a0df6` (`Resolve relative agent role config paths from layers (#19261)`).
- Developer info: rejects explicit legacy `agents.max_threads` while `multi_agent_v2` is enabled, removes `SpawnAgentInstructions` and the `<spawned_agent_context>` wrapper from MAv2 child context, and resolves relative `AgentRoleToml.config_file` paths against their config layer folder.
- Why: commit bodies establish conflicting MAv2/legacy thread-limit semantics, unwanted extra model-visible spawned-agent developer fragments, and broken relative role config loading.
- User info: invalid MAv2 thread-limit config fails early with a clear error, spawned agents see inherited developer instructions without an extra wrapper, and agent roles with relative config files work from layered config.
- Compatibility/risks: `agents.max_threads` is explicitly incompatible with MAv2 in this release but later releases change this again; do not treat this as final current behavior without checking later entries. Removing spawned-agent context changes model-visible prompts.
- Evidence: `EVID-125-mav2-config-context`.

### `d87d9187162e8db2bcc287e141afea36369a0df6` - `Resolve relative agent role config paths from layers (#19261)`

- Дата: `2026-04-23T23:23:11-07:00`; PR: `#19261`; поколение: role config path resolution in config layers.
- Developer info: `load_agent_roles` captures `layer.config_folder()` and passes it into `agents_toml_from_layer`; `agents_toml_from_layer` activates `AbsolutePathBufGuard` while deserializing `[agents.*]`, so `AgentRoleToml.config_file = "./agents/researcher.toml"` resolves against the config file that declared the role. This aligns layer-local `[agents.*]` declarations with standalone role-file parsing, which already used the role file parent as base.
- User info: a user or project config layer can declare a role with a relative `config_file` and expect it to resolve next to that config layer, not against process cwd or another higher-precedence layer.
- Why: before this fix, relative role config paths in layered config failed with `AbsolutePathBuf deserialized without a base path`, even though `AgentRoleToml.config_file` documentation said relative paths were allowed.
- Compatibility/risks: this is a contract gate for any fork feature that writes persistent role config. Writing a relative `config_file` is valid only if the runtime layer loader that will parse it has a concrete config folder; validation must exercise the same startup/session path, not just schema generation.
- Verification notes: coverage includes `agent_role_relative_config_file_resolves_from_config_layer`; current code still uses this path before scanning `config_folder.join("agents")`, so declared role files are excluded from duplicate autodiscovery in the same layer.
- Evidence: `EVID-125-agent-role-layer-paths`.

### Sticky environment and thread state (`#18897`)

- Коммиты: `49fb25997f3c09c25684c2a729cb933939a7f830` (`Add sticky environment API and thread state (#18897)`).
- Developer info: adds sticky environment selections to app-server v2 `thread/start` and `turn/start`, carries thread-level selections through core session/thread state, adds `core/src/environment_selection.rs`, and updates old/v2 multi-agent spawn paths to preserve environment selection through child config.
- Why: commit body and release notes identify sticky environments as part of app-server multi-workspace/remote targeting; diff touches `ThreadManager`, `TurnContext`, `AgentControl`, old `multi_agents/spawn.rs` and MAv2 `spawn.rs`.
- User info: a thread can keep environment/cwd selection as sticky state, so future turns and spawned/forked work can target the intended environment instead of falling back to default/local.
- Compatibility/risks: sticky environment is an app-server/thread-state API; fork changes must carry thread-level selections through core state and child spawn config rather than only per-turn overrides.
- Evidence: `EVID-125-sticky-env`.

### PermissionProfile propagation continuation (`#18284`, `#18285`, `#18286`, `#18287`, `#19231`)

- Коммиты: `5c239ad7483ac58c475d5ff2c183f9e5b229094f` (`tui: sync session permission profiles (#18284)`), `f90cc0ee648b1053431a92e46ec372b2a937ba44` (`tui: carry permission profiles on user turns (#18285)`), `ff22982d752fcf70e849c21d96b896b1b88d8988` (`mcp: include permission profiles in sandbox state (#18286)`), `9c0eced39196671646862e9c918d09535ff7bff6` (`shell-escalation: carry resolved permission profiles (#18287)`), `4816b892044084de6ab5a55ea0b5854c330843fd` (`permissions: make profiles represent enforcement (#19231)`).
- Developer info: synchronizes TUI session state from `SessionConfigured.permission_profile`, carries permission profiles on user-turn operations, adds `permissionProfile` to MCP `SandboxState`, carries resolved profiles through shell escalation, and changes active `PermissionProfile` into a tagged union over `managed`, `disabled` and `external` enforcement modes.
- Why: commit bodies state the migration goal: make `PermissionProfile` the canonical representation across TUI, turns, MCP, shell escalation and app-server without collapsing `DangerFullAccess` or `ExternalSandbox` into lossy legacy sandbox projections.
- User info: permissions shown/used by TUI, MCP tools and escalated shell calls align better with actual enforcement; externally sandboxed sessions and full-access sessions round-trip without losing their meaning.
- Compatibility/risks: the experimental app-server v2 wire shape changes to tagged profiles, while legacy `SandboxPolicy` remains a compatibility projection. Fork multi-agent code must not store permissions as ad hoc legacy sandbox-only values.
- Evidence: `EVID-125-permission-profile`.

### Rollout trace boundaries and debug reducer (`#18878`, `#18880`)

- Коммиты: `6d09b6752d6a29994d97543189134c01c5c48a73` (`[rollout_trace] Trace tool and code-mode boundaries (#18878)`), `e3c8720a99114154929dbab950fac9fb1e1e0558` (`[rollout_trace] Add debug trace reduction command (#18880)`).
- Developer info: records canonical tool-call lifecycle and code-mode execution/wait attribution in rollout trace, adds reducer protocol/thread structures and a debug CLI entry point for inspecting reduced traces.
- Why: commit bodies describe deterministic trace reduction across tool/runtime boundaries; `#18880` depends on the trace stack being present. The release note also references multi-agent relationships via `#18879`, but that local commit has no diff and is tracked as evidence-only.
- User info: no primary end-user command is added for multi-agent work; developers gain a local debug path to inspect recorded runtime/tool traces, which is important for reconstructing multi-agent behavior.
- Compatibility/risks: rollout trace is developer/debug infrastructure. Do not build fork user-facing behavior that depends on debug CLI output; use runtime/protocol source-of-truth instead.
- Evidence: `EVID-125-rollout-trace`.

## `rust-v0.126.0`

- Release commit: `4695dfce4f0b63491e410628ccf1935addb635ab`
- Дата: `2026-04-29T14:45:41-07:00`
- Subject: release notes include persistent goal workflows, external-agent session/config migration including subagents, and improved MAv2 feature-scoped configuration, context hints, wait minimums and nested spawn behavior.

### Persistent goals as adjacent long-running runtime (`#18073`, `#18074`, `#18075`, `#18076`, `#18077`)

- Коммиты: `0ee737cea69f0907effceefa5da49e5ea5d0f39f` (`Add goal persistence foundation (1 / 5) (#18073)`), `6c874f9b341a8fcb01bdc35999aad1ca093afea2` (`Add goal app-server API (2 / 5) (#18074)`), `32ace07ac57ef0c7774cbc6fe55ac089fa307868` (`Add goal model tools (3 / 5) (#18075)`), `4167628622a0af70374a7a6c44a547a99b5075eb` (`Add goal core runtime (4 / 5) (#18076)`), `f1c963d77eabe02b0ebb26bea1d117ca14ffed9c` (`Add goal TUI UX (5 / 5) (#18077)`).
- Developer info: adds persisted `thread_goals` state and `goals` feature, app-server v2 `thread/goal/*` APIs and notifications, model tools `get_goal`/`create_goal`/`update_goal`, core runtime accounting/continuation/budget/interrupt/resume behavior, and TUI `/goal` UX with status/footer indicators.
- Why: commit bodies state that long-running goals need durable thread-level state, app-server observability, constrained model tools and core-owned continuation rather than per-client implementations. `#18076` explicitly gives pending user input and mailbox work priority over automatic goal continuation.
- User info: users can create, view, pause/unpause, clear and resume long-running goals; the model can inspect or complete goals through constrained tools when explicitly requested.
- Compatibility/risks: Goal is adjacent autonomy, not MAv2. Fork code should not route subagent work through goal tools, but goal runtime scheduling must not starve mailbox/inter-agent work.
- Evidence: `EVID-126-goals`.

### MAv2 feature config, interruption and tool contract churn (`#19124`, `#19351`, `#19360`, `#19733`, `#19792`, `#19805`, `#20052`, `#20139`, `#20180`)

- Коммиты: `120aa07d81ea9f3838ddec31653d1237db11f09d` (`Make MultiAgentV2 interruption markers assistant-authored (#19124)`), `28742866c78cbbef0f38a652678c9a1908dfb84a` (`Add agents.interrupt_message for interruption markers (#19351)`), `deb45093020f801b235cafc0ec9d30fffd49f3ff` (`feat: surface multi-agent thread limit in spawn description (#19360)`), `1f304dd1f2c87f907aa56cbf076a846f4d013b9a` (`Allow agents.max_threads to work with multi_agent_v2 (#19733)`), `f8c527e5298f2cd047a12624133b24de1bf3829d` (`multi_agent_v2: move thread cap into feature config (#19792)`), `fd36838cf30f837a0ae66f540c49e0f85432905c` (`Add MultiAgentV2 root and subagent context hints (#19805)`), `34d71d43eb87e16429a3945ec3de5799ea2153c0` (`Make MultiAgentV2 wait minimum configurable (#20052)`), `857146b328009c259f65b871c1c3b1f6494c2cb2` (`Delete multi_agent_v2 followup_task interrupt parameter (#20139)`), `70ac0f123c4b1869c9069d5b34e367b96c28bfad` (`Make multi-agent v2 ignore agents.max_depth (#20180)`).
- Developer info: makes v2 interruption markers assistant-authored, adds `[agents] interrupt_message`, exposes thread limits in `spawn_agent` description, briefly re-allows then moves MAv2 thread cap into `[features.multi_agent_v2].max_concurrent_threads_per_session`, adds root/subagent usage hints and filters stale inherited hints from forked history, makes MAv2 wait minimum configurable, deletes `followup_task.interrupt`, and stops applying legacy `agents.max_depth` to v2 nested spawns.
- Why: commit bodies establish the migration away from legacy v1 knobs and user-authored interruption markers toward MAv2-owned feature config, cleaner prompt context and mailbox/message-boundary semantics.
- User info: MAv2 spawn/wait descriptions expose effective limits, subagents receive role-appropriate guidance, nested v2 task trees are no longer blocked by v1 depth, and follow-up task interruption is simplified/removed.
- Compatibility/risks: `agents.max_threads` behavior changes across `0.125` and `0.126`; current fork work must check latest feature config rather than copying one intermediate rule. Removing `followup_task.interrupt` is model-visible API churn; do not add new fork behavior to the removed parameter.
- Evidence: `EVID-126-mav2-config`.

### External-agent sessions and subagent migration (`#19895`, `#19949`)

- Коммиты: `4c68bd728fe7c1c184f88ed611161e18921cf096` (`External agent session support (#19895)`), `cb8b1bbcd64bc12338893f3591ab1c150105e9d4` (`Support detect and import MCP, Subagents, hooks, commands from external (#19949)`).
- Developer info: adds `external-agent-sessions` for session discovery/source-record parsing/rollout construction/import ledger, wires session detection/import into app-server external-agent config API, adds `codex-external-agent-migration` for MCP servers, hooks, commands and subagent configs, and extends migration item details/schemas.
- Why: commit bodies state the migration path needed to cover recent external sessions and external subagent/MCP/hook/command configs in Codex-native shapes, conservatively skipping unsupported or ambiguous source behavior.
- User info: users can import recent external-agent sessions and supported external subagent configs into Codex, with large imported sessions compacted for safe resume before follow-up.
- Compatibility/risks: imported subagents do not migrate source model names; they keep Codex defaults and only migrate compatible reasoning/sandbox fields. Fork migration should remain conservative and recompute from disk instead of trusting UI preview details as source-of-truth.
- Evidence: `EVID-126-external-agent-migration`.

### Plan mode nudge (`#19901`)

- Коммиты: `c6bcd278329826c9e836099be9e24a0f84b8aa38` (`feat(tui): suggest plan mode from composer drafts (#19901)`).
- Developer info: adds a TUI composer heuristic that suggests Plan mode when the draft contains standalone `plan`, reuses `Shift+Tab` mode cycling, stores thread-scoped dismissal and keeps footer/statusline layout stable.
- Why: commit body says the desktop app already had a Plan-mode nudge and TUI had `/plan`/`Shift+Tab` but lacked a reminder at the moment planning intent was visible.
- User info: TUI users get an inline reminder to switch into Plan mode when drafting planning requests.
- Compatibility/risks: heuristic is TUI-only and thread-scoped; it must not auto-switch modes or alter runtime collaboration settings.
- Evidence: `EVID-126-plan-nudge`.

## `rust-v0.127.0`

- Release commit: `7c8a4a5b7909fd3fbd276337a95a0e5fee9f1eb0`
- Дата: `2026-04-29T22:11:17-07:00`
- Subject: release notes repeat the goal, external-agent migration and plugin/keymap work from the 0.126 window and add new polish around `/goal resume`, external session background import, protocol regression coverage and agent graph storage.

### Goal resume wording (`#20082`)

- Коммиты: `91ca551df80b77d9434587007bcc912b08915868` (`Use /goal resume for paused goals (#20082)`).
- Developer info: renames the TUI paused-goal reactivation command/copy from `/goal unpause` to `/goal resume` across app events, footer/status text, goal menu and slash dispatch tests.
- Why: commit body says bare `/goal` is a summary command, so paused-goal UI should point users to an explicit command that reads naturally.
- User info: paused goals now tell users to run `/goal resume`.
- Compatibility/risks: this is a TUI command/copy rename; docs or fork scripts that mention `/goal unpause` should update to `/goal resume`.
- Evidence: `EVID-127-goal-resume`.

### Inter-agent commentary phase regression guard (`#20046`)

- Коммиты: `05fd90457263834535d85995023102e98f9a8be8` (`test protocol: lock inter-agent commentary phase (#20046)`).
- Developer info: adds protocol coverage asserting `InterAgentCommunication::to_response_input_item` replays inter-agent messages with `phase: Some(MessagePhase::Commentary)`.
- Why: subject/body and diff show this is a regression lock for the protocol shape of inter-agent commentary messages.
- User info: no direct UX change, but replayed inter-agent messages keep the intended commentary phase instead of drifting into ordinary user/assistant transcript semantics.
- Compatibility/risks: this is test-only/protocol guardrail; fork code that changes `InterAgentCommunication` serialization must preserve phase semantics or intentionally update this contract.
- Evidence: `EVID-127-inter-agent-commentary`.

### Agent graph store interface (`#19229`)

- Коммиты: `782191547cdc2a24a12a0bb9cef39b3e1dad1f62` (`Add agent graph store interface (#19229)`).
- Developer info: adds `codex-agent-graph-store`, `AgentGraphStore` trait, local SQLite-backed implementation over existing `StateRuntime` thread-spawn edge behavior, and graph types for edge status/listing descendants.
- Why: commit body states persisted subagent topology was leaking through SQLite-specific `StateRuntime` helpers; the new boundary prepares local/remote graph operations without coupling orchestration code to state DB APIs.
- User info: no immediate visible feature; this is developer architecture for reliable parent/child subagent topology and future remote graph storage.
- Compatibility/risks: call-site migration and remote gRPC store are explicitly follow-ups, so this release adds an interface but not the complete graph-store migration.
- Evidence: `EVID-127-agent-graph-store`.

### External session import polish and backgrounding (`#20261`, `#20284`)

- Коммиты: `7bcd4626c4cc66620bf78744122ccb04fd63c446` (`Consume ai-title from external sessions and add end marker (#20261)`), `c8abcbf9259c00a2dbc5f7d08295f12d5fad2a0b` (`Import external agent sessions in background (#20284)`).
- Developer info: adds `ai-title`/`aiTitle` support and end-marker handling in external session detection/import, returns from external-agent import before session history import finishes, runs session import in the background, emits completion notifications and serializes imports to avoid duplicates.
- Why: commit bodies show import UX/performance and correctness: keep title fidelity, signal imported session endings and avoid blocking app-server import RPC on history processing.
- User info: external-agent session imports can finish asynchronously while still reporting completion, and imported sessions get better title/end metadata.
- Compatibility/risks: background import means callers must listen for completion notifications instead of assuming all imported threads exist when the RPC returns.
- Evidence: `EVID-127-external-import`.

## `rust-v0.128.0`

- Release commit: `e4310be51f617f5e60382038fa9cbf53a2429ca4`
- Дата: `2026-04-30T08:06:34-07:00`
- Subject: release notes aggregate goal workflows, plan-mode nudges, external-agent import and explicit MAv2 config work already introduced in earlier commits; direct new entries in this stable interval reduce collaboration-mode surface and polish `/side` return UX.

### Collaboration mode surface reduction (`#20149`)

- Коммиты: `fedcefe9dafcb4ee6cd74e5438a2b272f5160301` (`Reduce the surface of collaboration modes (#20149)`).
- Developer info: removes collaboration-mode dependencies from broad thread/model construction paths, narrows mode handling in app-server/TUI/model-manager/tool config surfaces, and keeps mode presets closer to the explicit collaboration-mode list/request path.
- Why: subject and diff show an intentional reduction of collaboration-mode coupling across `ThreadManager`, model provider/manager, app-server processing, TUI and `request_user_input` tool specs.
- User info: collaboration modes remain available, but their implementation becomes less invasive and less likely to mutate unrelated session/model behavior.
- Compatibility/risks: fork changes must not re-expand collaboration modes into a global model-provider/thread-manager side channel; mode behavior should stay in the native settings/preset surfaces.
- Evidence: `EVID-128-collaboration-surface`.

### Side conversation return shortcut (`#20282`)

- Коммиты: `515aa9a4fb16f89544d065ac57e8ce01a2360929` (`tui: return from side chat on Ctrl-D (#20282)`).
- Developer info: updates TUI platform action handling so empty `Ctrl-D` in a side chat returns to the parent thread instead of exiting.
- Why: subject/diff directly target `/side` navigation behavior.
- User info: side-chat users get a native keyboard path back to the parent conversation.
- Compatibility/risks: TUI-only UX; fork changes around `/side` must preserve the distinction between exiting the app and returning from a child/side thread.
- Evidence: `EVID-128-side-return`.

## `rust-v0.129.0`

- Release commit: `2808a4deb181e5ca2b1293a1a5980938cb746861`
- Дата: `2026-05-07T18:45:29+02:00`
- Subject: release notes focus on TUI resume/copy workflows, plugin management, hooks, Guardian elicitations and goal polish; direct multi-agent entries are MAv2 feature gating, tool handler ownership, realtime protocol updates, ThreadStore/agent-graph churn and Guardian routing.

### MAv2 decoupled from `collab` and usage-hint interpolation churn (`#20246`, `#20973`, `#21337`)

- Коммиты: `c37f7434badbc50a4bee80148952a98f4dca36b4` (`Gate multi-agent v2 tools independently of collab (#20246)`), `f48b777717e09eb68ef34736d328e96f7a39e9ac` (`feat: support template interpolation in multi-agent usage hints (#20973)`), `cc84e6bc6dca7d9f668028e71576457975d4d246` (`Revert "feat: support template interpolation in multi-agent usage hints" (#21337)`).
- Developer info: makes MAv2 tool availability independent from the old `collab` feature, blocks inappropriate MAv2 tools in review/guardian contexts, briefly introduces resolved-config interpolation for MAv2 usage hints, then reverts that interpolation stack before the stable release line moves on.
- Why: subjects/diffs show two important contract facts: MAv2 is no longer just a `collab` sub-mode, and usage-hint interpolation was attempted but reverted locally.
- User info: MAv2 tools can be exposed without enabling old collaboration tooling; interpolated hint templates should not be assumed stable in this release because the implementation was reverted.
- Compatibility/risks: fork config must distinguish `Feature::Collab` from `Feature::MultiAgentV2`; do not build fork behavior on the reverted template interpolation path.
- Evidence: `EVID-129-mav2-gating-hints`.

### Tool handler/spec ownership moves (`#20687`, `#21416`)

- Коммиты: `f593323ef18fcbbfd093668454ca277650d231b6` (`[codex] Split tool handlers by tool name (#20687)`), `9417cf969682330ea5fcccbdf39818bc4b213167` (`[codex] Move tool specs into core handlers (#21416)`).
- Developer info: normalizes tool registration so each handler owns a tool name and moves specs/tests into handler-adjacent modules, including old `multi_agents`, `multi_agents_v2`, `plan` and related service tools.
- Why: diffs move source-of-truth for model-visible tool spec closer to handlers, reducing drift between `spec_plan`, registry and implementation.
- User info: no direct new command, but model-visible multi-agent tools are less likely to diverge from the runtime handler that executes them.
- Compatibility/risks: fork changes to MAv2 tool arguments, names or descriptions must update handler-local specs/tests instead of an old centralized tool-spec-only file.
- Evidence: `EVID-129-tool-ownership`.

### Realtime protocol/session updates (`#20361`, `#20715`)

- Коммиты: `8a97f3cf0349a544d987e5239c2aeee6487dac3d` (`realtime: rename provider session ids (#20361)`), `e7e6267ab3e084e2c3d9647acbd83952e4e687ad` (`Make realtime sideband startup async (#20715)`).
- Developer info: renames realtime provider session fields through app-server protocol, codex-api, core and TUI, and moves realtime sideband startup off the blocking startup path.
- Why: subjects/diffs show wire-name compatibility cleanup and startup latency work for realtime collaboration.
- User info: realtime clients see the `realtimeSessionId` naming and faster startup because sideband work starts asynchronously.
- Compatibility/risks: realtime wire field rename is a client compatibility point; fork realtime delegation work must update protocol, API, core and TUI together.
- Evidence: `EVID-129-realtime`.

### ThreadStore, Guardian review forks and agent graph injection/revert (`#20577`, `#20689`, `#21481`, `#19431`)

- Коммиты: `ee02cf26d684c3e00f20e19225f8e6e6c176cb62` (`codex: use ThreadStore history for core review forks (#20577)`), `7e310bc7f3c94af86c1d06d6e387054b3685ebaf` (`Inject state DB, agent graph store (#20689)`), `a8488fec5ef27216cae96e24eec2f18ef2374ed7` (`Revert state DB injection and agent graph store (#21481)`), `346070a424f8cebcbc42d05081ec5e3ba82965b2` (`Route opted-in MCP elicitations through Guardian (#19431)`).
- Developer info: moves Guardian/review fork history reads onto `ThreadStore`, attempts explicit state DB and `AgentGraphStore` injection into app-server/core/thread-manager paths, reverts that injection, and routes opted-in MCP elicitations through Guardian review.
- Why: diffs establish a persistence and review-routing churn window: ThreadStore is becoming the history authority for forks, while direct agent graph injection was tried and rolled back.
- User info: review/Guardian forks use more faithful stored history, eligible MCP elicitations can follow Guardian approval UX, but agent-graph injection should not be treated as final behavior in this release.
- Compatibility/risks: do not base fork architecture on the reverted `#20689` injection shape; treat `ThreadStore` history use as durable evidence and the graph-store injection as an abandoned intermediate.
- Evidence: `EVID-129-threadstore-guardian-graph`.

## `rust-v0.130.0`

- Release commit: `58573da43ab697e8b79f152c53df4b42230395a8`
- Дата: `2026-05-08T11:11:22-07:00`
- Subject: release notes focus on plugin sharing, remote-control, app-server thread pagination, Bedrock auth and selected-environment image viewing; direct multi-agent entries are tool-spec ownership continuation, thread pagination and protocol-native Guardian review timing.

### Tool specs onto handlers, second pass (`#21461`)

- Коммиты: `566f2cb61220342871ea63afee2322490ecc5d0e` (`[codex] Move tool specs onto handlers (#21461)`).
- Developer info: continues moving specs into tool handlers and updates old `multi_agents`, `multi_agents_v2`, `plan`, goal, MCP, shell and unified exec handlers so spec ownership lives beside execution code.
- Why: diff directly touches `multi_agents/*`, `multi_agents_v2/*`, `multi_agents_spec`, `plan.rs` and registry/spec planning.
- User info: no new visible command, but the tool contract becomes easier to keep aligned with handler behavior.
- Compatibility/risks: current fork work should treat handler-local spec modules as the canonical edit point for model-visible tool schema and wording.
- Evidence: `EVID-130-tool-specs-handlers`.

### Thread pagination and review timing (`#21566`, `#21434`)

- Коммиты: `0d0835dd537b57913627e877f29031151b49429e` (`feat(app-server, threadstore): Thread pagination APIs and ThreadStore contract (#21566)`), `99016ec732e4c231ddb5116e159a634a1436d88c` (`[codex-analytics] plumb protocol-native review timing (#21434)`).
- Developer info: adds app-server/thread-store pagination contracts for large histories and adds protocol-native Guardian approval review timing through app-server schema, core Guardian review state and TUI display paths.
- Why: release notes and diffs establish app-server history pagination plus typed review timing as protocol surfaces rather than ad hoc client inference.
- User info: clients can request unloaded/summary/full turn item views for large histories, and Guardian review timing becomes a structured item/notification field.
- Compatibility/risks: pagination affects resume/fork/review consumers that assume full history is always loaded; fork subagent work must explicitly choose the needed history view.
- Evidence: `EVID-130-thread-pagination-review-timing`.

## `rust-v0.131.0`

- Release commit: `05eb8678451435cbc8d79c6d8254276289f2bdf1`
- Дата: `2026-05-18T10:20:25-07:00`
- Subject: release notes cover richer TUI controls, plugin and remote workflows, Python SDK and diagnostics; direct MAv2 entries here are model-only tool exposure, service-tier propagation, wait timeout config, TUI metadata hydration and removal of a resurrected `/collab` command.

### MAv2 model-only exposure and wait timeout config (`#22514`, `#22528`)

- Коммиты: `fc26af377fc64cc42ff0709a9fcebefb4f5b6b80` (`feat: expose multi-agent v2 as model-only tools (#22514)`), `7c57a59f51b605b82f49e71833aa6e675b9ec54c` (`Make multi_agent_v2 wait_agent timeouts configurable (#22528)`).
- Developer info: adds `ToolExposure::DirectModelOnly`, `features.multi_agent_v2.non_code_mode_only`, routing/spec behavior for model-only MAv2 tools, and configurable `wait_agent` timeout/minimum limits through config/schema/feature configs and handler tests.
- Why: subjects/diffs establish a new generation: MAv2 should be exposed as model-visible tools without code-mode nested tool exposure, and wait behavior should be runtime-configurable.
- User info: models can call MAv2 tools through the main model-only tool surface, and administrators can tune `wait_agent` timeouts.
- Compatibility/risks: this is a major source-of-truth shift for current MAv2. Fork changes must update `ToolExposure`, registry/router/spec planning and feature config together.
- Evidence: `EVID-131-model-only-wait`.

### Spawn metadata, service tier and handler future hardening (`#21870`, `#22266`, `#22139`)

- Коммиты: `90c0bec50c9439da7a9359b9eb0ec0e24add32ef` (`Avoid blocking TUI on agent metadata hydration (#21870)`), `9e7cdbd0d28a4400a70339042e4643b4f462da6c` (`core: box multi-agent handler futures (#22266)`), `87de4e32903610b8ab66d1bda1d491936b8b0973` (`Add service tier overrides to spawned agents (#22139)`).
- Developer info: makes TUI agent metadata hydration non-blocking, boxes old/v2 multi-agent handler futures, and adds `service_tier` handling to old and v2 `spawn_agent` specs/handlers/tests.
- Why: diffs target performance/stability of agent metadata display and runtime future sizes while extending spawn config inheritance/override.
- User info: TUI is less likely to stall on agent metadata, and spawned agents can inherit or explicitly use a service tier.
- Compatibility/risks: service tier is another spawn-time runtime contract; fork extensions must propagate it through both old and v2 spawn paths if they still support both.
- Evidence: `EVID-131-spawn-metadata-service-tier`.

### `/collab` cleanup (`#22535`)

- Коммиты: `efdcbba053f4971132d944b918d3d81c8b786201` (`Remove resurrected `/collab` slash command (#22535)`).
- Developer info: removes the resurrected TUI `/collab` slash command wiring, popup entries and tests while leaving current collaboration mode/MAv2 surfaces intact.
- Why: subject/diff show cleanup of a stale slash-command surface.
- User info: users should not rely on `/collab`; current collaboration behavior is exposed through mode/settings/tool surfaces instead.
- Compatibility/risks: fork docs/UI should not reintroduce `/collab` as a shortcut unless it is deliberately re-specified against current collaboration-mode and MAv2 semantics.
- Evidence: `EVID-131-collab-cleanup`.

## `rust-v0.132.0`

- Release commit: `13595c36e218fcbd13df118eeadf00d4eb0e6d31`
- Дата: `2026-05-19T09:56:15-07:00`
- Subject: release notes focus on Python SDK auth, text-only turn APIs, `codex exec resume`, faster TUI startup, remote executor registration and image fidelity; direct MAv2 entries are compact model-visible spawn description and configurable v2 tool namespace.

### MAv2 model-visible wording and namespace config (`#23069`, `#23147`)

- Коммиты: `061a614d857e0f2289457770e90caf8a45b5e430` (`multiagent: trim model-visible description, cap to 5 models (#23069)`), `545ede569cff13d0aabc782ebd8cdaa6d5cb86ab` (`Make multi-agent v2 tool namespace configurable (#23147)`).
- Developer info: trims `spawn_agent` model-visible description/model list and adds `features.multi_agent_v2.tool_namespace` through config/schema, feature configs and spec-plan tests.
- Why: diffs show model-visible prompt ergonomics and namespace configurability moving into explicit MAv2 feature config.
- User info: model-facing `spawn_agent` descriptions are shorter and less overloaded, and deployments can configure the MAv2 tool namespace.
- Compatibility/risks: namespace is a wire/model-visible contract. Fork changes must update config schema, runtime loader, tool registry/spec planning and tests together; hardcoded MAv2 tool names become riskier after this release.
- Evidence: `EVID-132-mav2-namespace`.

## `rust-v0.133.0`

- Release commit: `9474e5cfc4494b0ba319352aa86ce436c59e65c8`
- Дата: `2026-05-21T12:37:04-07:00`
- Subject: release notes enable goals by default, expand permission/profile/plugin workflows and add extension lifecycle events including subagent start/stop, tool execution and turn metadata.

### Fork context, v1 tool discoverability and service-tier propagation (`#23352`, `#23144`, `#23475`, `#22169`)

- Коммиты: `826b2182ed32b8534df1c44c2a82d8eeca094b58` (`Preserve context baselines for full-history agent forks (#23352)`), `b3ae3de4056f9417c968f8628cde55b428f309b8` (`Defer v1 multi-agent tools behind tool search (#23144)`), `05b8ce43541bbcd3ed14a21ea3f103c20cb9392b` (`chore: namespace v1 sub-agent tools (#23475)`), `c53da029bcb8afe07e4019fa13dfd46b37d7cd7d` (`[codex] Honor role-defined spawn service tiers (#22169)`).
- Developer info: preserves context baselines for full-history forks, hides legacy v1 multi-agent tools behind tool search, namespaces old sub-agent tools and applies role-defined service-tier defaults to old and v2 spawn paths.
- Why: diffs show fork-context correctness, tool discoverability narrowing and spawn config propagation across old/v2 handlers.
- User info: full-history forks keep cleaner context baselines, legacy tools are less exposed by default, old sub-agent tool names become namespaced, and role-defined service tier can affect spawned agents.
- Compatibility/risks: v1 tool names/discoverability are now compatibility surfaces; fork docs should distinguish legacy v1 tools from current MAv2 names and avoid unqualified old names.
- Evidence: `EVID-133-fork-v1-service-tier`.

### Subagent hooks and extension turn metadata (`#22782`, `#22873`, `#23688`)

- Коммиты: `d661ab70eda6bde758d7e48d6bf3638523cb014c` (`Add SubagentStart hook (#22782)`), `eee3e60db3d8ca25bb4e8dc9438e4f418430fc17` (`Add SubagentStop hook (#22873)`), `59507b849126f598ed7c624bbaf75d7ebc5588c2` (`feat: expose turn-start metadata to extensions (#23688)`).
- Developer info: adds `SubagentStart` and `SubagentStop` hook event types, schemas, config requirements and TUI hook browser visibility, and exposes turn-start metadata through extension lifecycle contributors.
- Why: release notes and diffs establish subagent lifecycle hooks as first-class extension surfaces.
- User info: hooks/extensions can observe subagent start/stop and richer turn-start metadata without custom transcript scraping.
- Compatibility/risks: hook input/output schemas are API contracts; fork subagent changes must update hooks schemas, runtime emission and TUI hook display together.
- Evidence: `EVID-133-subagent-hooks`.

### Thread settings through app-server (`#23502`, `#23507`)

- Коммиты: `771a4e74ac319c3d8379c62c7caa3aec1ad53382` (`Add thread/settings/update app-server API (#23502)`), `edc48e4612b457f2e6a32a007516fbde6fc5a832` (`Sync TUI thread settings through app server (#23507)`).
- Developer info: introduces app-server thread settings update flow and syncs TUI thread settings through app-server notifications/state rather than local-only mutation.
- Why: subject/diff show thread settings becoming an app-server source-of-truth; this matters for collaboration mode/settings propagation.
- User info: TUI setting changes are reflected through app-server-backed session/thread state and can align better across clients.
- Compatibility/risks: fork changes to collaboration/thread settings must update app-server APIs and TUI session state, not only local TUI config.
- Evidence: `EVID-133-thread-settings`.

## `rust-v0.134.0`

- Release commit: `a75c443fdb64db48c3cf4bdb247c7ee52c0144c9`
- Дата: `2026-05-26T10:40:13-07:00`
- Subject: release notes add local conversation search, profile selection changes, MCP environment targeting, connector schema reliability, concurrent read-only MCP tools and richer extension/hook context including subagent identity.

### Hook subagent identity and extension conversation history (`#22882`, `#23963`)

- Коммиты: `16d85e270817de21e3a6afe6162b44c13df1d1c0` (`Add subagent identity to hook inputs (#22882)`), `7e802b22f13e2714efd2fb2a6e396319958d6506` (`Expose conversation history to extension tools (#23963)`).
- Developer info: adds subagent identity fields to hook event input schemas/runtime and exposes conversation history to extension tools through the shared extension tool call path.
- Why: release notes and diffs establish richer hook/extension context, specifically including subagent identity.
- User info: hooks and extension tools can make decisions using subagent identity and conversation history instead of reconstructing context indirectly.
- Compatibility/risks: subagent identity becomes a hook contract; fork extensions must preserve schema compatibility and avoid leaking unrelated parent context.
- Evidence: `EVID-134-hook-identity-history`.

### Live agent traversal and task input cleanup (`#24057`, `#24151`, `#24157`)

- Коммиты: `5865ec45e596e67c0a1279c82d3a02e50dcaef1b` (`Avoid config snapshots in live agent subtree traversal (#24057)`), `fbd4efa9ed6b9fe13dacd56247cc714903df72b7` (`[codex] Use TurnInput for session task input (#24151)`), `6ad3a8350902f5825fe4172ef06cb5b506d10f3d` (`[codex] Remove external client session reset plumbing (#24157)`).
- Developer info: stops live agent subtree traversal from using stale config snapshots, unifies session task input around `TurnInput`, and removes external client reset plumbing.
- Why: diffs touch `AgentControl`, `ThreadManager`, session/turn task paths and client/session reset boundaries.
- User info: live subagent traversal should reflect current config more accurately, and session task input/reset behavior is less split across old paths.
- Compatibility/risks: fork code should not traverse active agent trees from stale config snapshots or introduce a second session-input/reset side channel.
- Evidence: `EVID-134-agent-traversal-turninput`.

## `rust-v0.135.0`

- Release commit: `4daceea869704f9f35e0a3949fc34711ef978a4e`
- Дата: `2026-05-28T09:28:25-07:00`
- Subject: release notes focus on doctor diagnostics, remote status, Vim mode, named permission profiles and SDK sandbox presets; direct multi-agent entries here are lineage/fork/compaction metadata and Guardian review fixes.

### Fork lineage and compaction metadata (`#24160`, `#24751`, `#24368`)

- Коммиты: `1911021c0eae972e622c8445d136ae04545a530f` (`Add forked_from_thread_id turn metadata (#24160)`), `61cbf3574eca870df6fa7f49648ec7e001901b5a` (`Drop startup context when truncating forked rollouts (#24751)`), `bee78806a9b8fad5f250f7822da08008b2c1dd01` (`[codex] add compaction metadata to turn headers (#24368)`).
- Developer info: adds fork lineage to turn metadata, drops startup context when truncating forked rollouts, and carries compaction metadata in turn headers.
- Why: diffs show lineage/compaction metadata moving into structured turn metadata rather than implicit rollout context.
- User info: forked/subagent work can be attributed and compacted with less ambiguity, improving resume and backend request context.
- Compatibility/risks: fork metadata is a durable protocol/runtime contract; fork features must update turn metadata, rollout reconstruction and app-server tests together.
- Evidence: `EVID-135-fork-lineage-compaction`.

### Guardian/auto-review stability (`#24714`, `#24746`)

- Коммиты: `e88626621bad4bb2655fa8b5fc7d2f73d68e168d` (`fix(auto-review) skip legacy notify for auto review threads (#24714)`), `7df8431bbdcdd6bdbac7d2dacd5b59aca0fb7732` (`Fix guardian review test user input (#24746)`).
- Developer info: prevents legacy notifications from leaking into auto-review threads and fixes Guardian review test input handling.
- Why: diffs are Guardian/auto-review specific and reduce cross-thread notification/test ambiguity.
- User info: auto-review/Guardian threads are less likely to expose legacy notification noise.
- Compatibility/risks: small but relevant to delegated review flows; fork auto-review changes must keep legacy-notify filtering in mind.
- Evidence: `EVID-135-guardian-stability`.

## `rust-v0.136.0`

- Release commit: `7ca611348db9446711ed16ed81c84095e3721cee`
- Дата: `2026-06-01T15:56:27+02:00`
- Subject: release notes are terse; local diffs show Guardian cache key stabilization, stale multi-agent slot reaping, subagent lineage metadata and MAv2 assignment tool rename.

### Guardian cache keys and metrics (`#24891`, `#24892`, `#24893`, `#24895`, `#24803`, `#24897`)

- Коммиты: `c95eb3d07bd7af4977d70bddfcaac925755da4c8` (`Stabilize Guardian client cache key handling (#24891)`), `bf4978a01f0241fc4bddecf4da68d45e867de72b` (`Export Guardian prompt cache key helper (#24892)`), `4ce563a87389e14072b9f6d068acb2f3e1c888a8` (`Add Guardian review prompt cache key (#24893)`), `4b9eda6ff646938344bdfc718f294c6d0b2c1302` (`Thread Guardian cache key through session (#24895)`), `3307240195ece4ef772b45e5323c683f4181f1e5` (`Use stable Guardian prompt cache keys (#24803)`), `3abf96739b543a53d2a28870eaf2a05f61fa0c15` (`Add Guardian review metrics (#24897)`).
- Developer info: stabilizes Guardian prompt/cache keys through client/session/review paths and adds Guardian review metrics.
- Why: subjects/diffs establish a coordinated Guardian cache-key and observability series.
- User info: Guardian review reuse/caching becomes more stable and measurable.
- Compatibility/risks: Guardian cache keys affect review reuse and cost/latency; fork review changes should not casually alter prompt key material.
- Evidence: `EVID-136-guardian-cache`.

### Multi-agent slot cleanup, lineage metadata and assignment rename (`#24903`, `#24161`, `#25267`)

- Коммиты: `e2551a5e362eb1538e21a47c8f7babf124f6d029` (`Reap stale multi-agent slots (#24903)`), `fc9cf62efb8098fce40b5198d3e7cfc45b4ce441` (`Add subagent lineage metadata for responsesapi (#24161)`), `8acaec73b6a227ae9375064e910ecf86f2620ec0` (`Rename multi-agent v2 assignment tool (#25267)`).
- Developer info: cleans stale multi-agent slots in close paths, adds subagent lineage metadata to Responses API/client metadata and temporarily renames the MAv2 assignment/follow-up tool surface.
- Why: diffs directly touch `AgentControl`, old/v2 close handlers, turn metadata/protocol and MAv2 tool files.
- User info: stale agent capacity is reclaimed, backend requests carry subagent lineage, and MAv2 tool names continue to churn in this release.
- Compatibility/risks: tool rename is transitional and reversed/renamed again in `rust-v0.137.0`; fork code must avoid pinning behavior to this intermediate name without alias handling.
- Evidence: `EVID-136-lineage-slots-rename`.

## `rust-v0.137.0`

- Release commit: `f221438b691b8f749d98f22077c93ebe01923fbe`
- Дата: `2026-06-03T11:17:07-07:00`
- Subject: release notes state MAv2 keeps runtime choice with each thread and exposes cleaner follow-up and metadata defaults for spawned agents.

### Thread parent identity and MAv2 dogfood defaults (`#25113`, `#25266`)

- Коммиты: `cf0911076f234e0219bd8d61dd3bc2f80a2df287` (`store and expose parent_thread_id on Threads (#25113)`), `8d49394febc57518479ece9e3dc5eb9060ac6968` (`Set multi-agent v2 dogfood defaults (#25266)`).
- Developer info: stores and projects `parent_thread_id` through app-server protocol/thread responses, analytics and core thread/session paths, and updates default MAv2 dogfood config.
- Why: release notes and diffs show lineage becoming a first-class thread field and MAv2 runtime defaults being adjusted.
- User info: clients can see parent-thread lineage directly, and MAv2 defaults are closer to the dogfood/current runtime behavior.
- Compatibility/risks: parent-thread identity is now a protocol field; fork UI and persistence should use it instead of inferring parentage from rollout paths.
- Evidence: `EVID-137-parent-thread-dogfood`.

### MAv2 follow-up naming and runtime metadata (`#25636`, `#25720`, `#25721`, `#25722`, `#25841`)

- Коммиты: `6ddb747e7687e9e6e3a2482631028c07ddc89cb6` (`[codex] Rename multi-agent v2 assign_task to followup_task (#25636)`), `3f1fb7ed8b641542add19bb841e4e4be5651693e` (`Add multi-agent runtime metadata types (#25720)`), `0c5ccd18abda96efaed9e94e26ffe22def5e28ed` (`Persist multi-agent runtime metadata (#25721)`), `bf9fd885b2546e91fe3bab271aa481b070f47518` (`Resolve per-thread multi-agent runtime (#25722)`), `3cf6f08da562ee1bf6866fbc2db45a3c3af620ff` (`session: keep startup prewarm aligned with resolved multi-agent runtime (#25841)`).
- Developer info: renames the MAv2 assignment tool back to `followup_task`, adds runtime metadata types, persists runtime metadata into rollout/thread store, resolves multi-agent runtime per thread and keeps startup prewarm aligned with that resolved runtime.
- Why: release notes and diffs establish the current direction: runtime choice is thread-owned metadata rather than a transient global default.
- User info: follow-up naming is cleaner, and resumed/spawned threads keep their chosen multi-agent runtime more reliably.
- Compatibility/risks: this is a major current-runtime source-of-truth. Fork changes to MAv2 runtime selection must update protocol metadata, rollout recorder, thread-store persistence, session startup/prewarm and tool registry selection together.
- Evidence: `EVID-137-runtime-metadata`.

### Spawn metadata hiding and close-agent guard (`#26114`, `#26144`)

- Коммиты: `668703c23f8a6cde07c317f687d9b95606293752` (`feat: default hide_spawn_agent_metadata to true (#26114)`), `51493157cd1fda08343e521ef47ee002009b8a40` (`Reject MAv2 close_agent self-targets (#26144)`).
- Developer info: changes the default for hiding spawn-agent metadata in model-visible descriptions/results and rejects MAv2 `close_agent` self-targets.
- Why: subjects/diffs target metadata noise reduction and invalid self-close guardrails.
- User info: spawned-agent metadata is hidden by default unless configured otherwise, and agents cannot close themselves through the v2 close path.
- Compatibility/risks: model-visible metadata defaults changed; fork features that depend on spawn metadata must use explicit config and tests.
- Evidence: `EVID-137-spawn-metadata-close-guard`.

## `rust-v0.138.0`

- Release commit: `c18e9f478bc940ef1ef8e1c426364c0fe3d86b73`
- Дата: `2026-06-08T15:00:08-07:00`
- Subject: release notes mention MAv2 prompt/config entries in the changelog, Plan-mode idle-turn gating and encrypted MAv2 payloads; the main release notes focus on app handoff, images, reasoning effort, auth/plugin automation and goal fixes.

### MAv2 prompt/config and namespace/code-mode exposure (`#26179`, `#26254`, `#26320`)

- Коммиты: `99c9be1d30ea78fad24d9eca8daa3526592a77b5` (`nit: small prompt update for MAv2 (#26179)`), `11bceb8f8bfdd8723da10bdcaa767a19d92a2a57` (`feat: catalog multi-agent v2 config (#26254)`), `8b1238856b0839cfdb345ee2af9f02e4ee2959f9` (`core: allow excluding tool namespaces from code mode (#26320)`).
- Developer info: updates MAv2 model-visible prompt text, moves MAv2 config into catalog/runtime loading paths and adds config/schema/spec-plan support for excluding tool namespaces from code mode.
- Why: subjects and diffs show tool/config exposure moving into structured config/catalog gates instead of scattered prompt/spec edits.
- User info: MAv2 prompt guidance and code-mode tool exposure become more controlled by native config and namespace rules.
- Compatibility/risks: namespace and code-mode exclusion are config contracts; fork changes must update config schema, loader, spec planning and tests together.
- Evidence: `EVID-138-config-namespace-prompts`.

### Plan idle-turn gate (`#26147`)

- Коммиты: `d297616d3e6a27865fb327e7b4ea3d548f7fdb45` (`Gate automatic idle turns in Plan mode (#26147)`).
- Developer info: gates automatic idle turns in Plan mode through `codex_thread`, session injection and goal runtime scheduling tests.
- Why: release notes explicitly mention that idle auto-turns stay out of Plan mode; diff ties this to goal/session scheduling.
- User info: Plan mode stops receiving autonomous idle continuations that could blur planning and execution phases.
- Compatibility/risks: fork goal/autonomy changes must preserve the mode gate so Plan mode remains a controlled planning surface.
- Evidence: `EVID-138-plan-idle`.

### Encrypted MAv2 payloads, v1 metadata and delivery reload (`#26210`, `#26599`, `#26610`, `#26623`)

- Коммиты: `5f4d06ef186b896d316620556e561d59206c3ebf` (`Encrypt multi-agent v2 message payloads (#26210)`), `66232220e21712604b0066f496e966c9a2c1bda8` (`[codex] Keep v1 spawn metadata visible (#26599)`), `0b1512c2c83935af4eaa096ac7e434992f15b066` (`refactor: split agent control modules (#26610)`), `d5e4f01af4f7d3937ee4cd54ac27507fa421d755` (`feat: reload v2 agents on delivery (#26623)`).
- Developer info: adds encrypted agent message input content and protocol/schema support, keeps legacy v1 spawn metadata visible while MAv2 hides metadata by default, splits `AgentControl` into focused modules and reloads v2 agents when delivering messages to inactive descendants.
- Why: release changelog and diffs establish privacy/serialization changes, old/new metadata compatibility and reload-on-delivery as runtime fixes.
- User info: MAv2 messages can be encrypted in rollout/protocol flow, legacy v1 metadata remains visible for compatibility, and follow-up delivery can wake/reload v2 agents.
- Compatibility/risks: encryption/plaintext message shape becomes a protocol/schema contract; refactor-only module split changes ownership but should not be treated as a user feature by itself.
- Evidence: `EVID-138-payload-delivery`.

## `rust-v0.139.0`

- Release commit: `a7dff904308535e965aee87680c1fc5ef1d19eec`
- Дата: `2026-06-09T12:02:39-07:00`
- Subject: release notes focus on web search, tool schema preservation, doctor diagnostics and plugin marketplace automation; local diffs include important MAv2 residency/concurrency/rename/resume prompt changes not emphasized in top-level notes.

### V2 residency, active concurrency and interrupt rename (`#26632`, `#26969`, `#26994`, `#26997`)

- Коммиты: `4e803a017c958dd37eb251372ba810232d3e84ba` (`feat: add v2 agent residency lru (#26632)`), `743f5aad38accd52da34bf4dcbdd1215a8c3ab9a` (`feat: count V2 concurrency by active execution (#26969)`), `8d415050fce4b4ebc6da1ba247379844235fa453` (`Rename multi-agent v2 close_agent to interrupt_agent (#26994)`), `0526cb56ac3501a02968010d03873993c319e290` (`Avoid reopening v2 descendants on resume (#26997)`).
- Developer info: introduces v2 agent residency LRU, counts concurrency by active execution rather than residency alone, renames v2 `close_agent` to `interrupt_agent`, and prevents resume from reopening inactive v2 descendants.
- Why: subjects/diffs directly target MAv2 lifecycle/resource semantics and user-facing tool naming.
- User info: v2 agents can remain resident more intelligently, concurrency limits reflect active work, the interruption tool name becomes clearer, and resume avoids unexpectedly waking descendant agents.
- Compatibility/risks: this is a breaking naming transition for model-visible tools; fork code must support compatibility classification for old `close_agent` while exposing current `interrupt_agent`.
- Evidence: `EVID-139-residency-interrupt`.

### Thread-scoped status and delegated auto-review preservation (`#26639`, `#26230`, `#27037`)

- Коммиты: `5a440c03f2f3393169c5df517d1fd8eee969e45e` (`fix(tui): scope MCP startup status by thread (#26639)`), `9e0d7f02c9416c46dde6e571068a0fb03a4facdf` (`fix: preserve auto review across config and delegation (#26230)`), `f9a680b9075562093ff78e45ff4fcb2e9a0348f9` (`[codex] Calm multi-agent v2 usage prompts (#27037)`).
- Developer info: scopes MCP startup status notifications by thread, preserves auto-review config across delegation and trims MAv2 usage prompt tone/wording.
- Why: diffs touch TUI/app-server thread routing, multi-agent common config propagation and MAv2 spec text.
- User info: child/other thread MCP startup status is less likely to pollute the wrong TUI thread, delegated auto-review keeps its configuration, and MAv2 guidance is less noisy.
- Compatibility/risks: thread-scoped notifications must stay protocol/event-driven; fork UI must not route status globally when subagents are active.
- Evidence: `EVID-139-thread-status-prompts`.

## `rust-v0.140.0`

- Release commit: `6506579001c322927a3e4bd440563267a7ac6c1f`
- Дата: `2026-06-15T13:22:00-07:00`
- Subject: release notes top-level subject is `## New Features`; local changelog/diffs contain MAv2 activity tracking, app-server interruption/input guards, child MCP transcript isolation, metrics tagging, usage hints and plaintext agent messages.

### Path-based activity tracking and app-server MAv2 guards (`#27007`, `#27166`, `#27173`)

- Коммиты: `fae270932065355b5d7f197b3f1c72912588369b` (`multi-agent: add path-based v2 activity tracking (#27007)`), `1547785657607043309fbe19d826aea8ee6e40e0` (`app-server: clear stale thread watches after v2 agent interruption (#27166)`), `1026e9de1be292ebb01579dfcce5a34ab224917a` (`app-server: reject direct input to multi-agent v2 sub-agents (#27173)`).
- Developer info: adds `SubAgentActivityKind`/path-based v2 activity projections across protocol, app-server, rollout-trace and TUI agent navigation/status feed, clears stale app-server watches after interruption and rejects direct client input to MAv2 sub-agent threads.
- Why: subjects/diffs establish app-server/TUI projections and guardrails for current path-based MAv2 behavior.
- User info: UI/API clients can see path-based subagent activity, interruption cleanup is more reliable, and users cannot accidentally drive a MAv2 child thread directly outside the parent orchestration model.
- Compatibility/risks: direct input rejection is a behavioral guardrail; fork UI must route follow-ups through native MAv2 tools rather than app-server `thread/turn/start` into child threads.
- Evidence: `EVID-140-activity-appserver`.

### Child MCP isolation, metrics versioning and thread-scoped contributions (`#27174`, `#27375`, `#27670`)

- Коммиты: `0ffcefaf3ddb3a61d8683ca0703f7d8b39ad6c1e` (`feat: keep child MCP warnings out of parent transcript (#27174)`), `ced1b8aa883f25b992f9ebe7d218f3709926f912` (`[codex] Tag multi-agent spawn metrics with version (#27375)`), `693082f3c4ca3b17d18989a8c5136f6258f54ac0` (`Make MCP server contributions thread-scoped (#27670)`).
- Developer info: prevents child MCP startup warnings from rendering into parent transcript, tags spawn metrics with multi-agent version and makes MCP server/tool contributions thread-scoped through app-server/core/session/extension contributor paths.
- Why: diffs target thread isolation and observability for multi-thread/subagent behavior.
- User info: parent transcript is cleaner when child agents have MCP startup warnings, telemetry distinguishes old/v2 spawn versions and tool contributions can remain bound to the relevant thread.
- Compatibility/risks: thread-scoped MCP contributions are a native surface; fork-added tools should use the contributor/thread scope instead of global MCP/tool lists.
- Evidence: `EVID-140-mcp-metrics-threadscope`.

### V2 usage hints, prompt cleanup and plaintext messages (`#27569`, `#27919`, `#27830`)

- Коммиты: `087035224123977defc7fda6e684088395b1d0a0` (`multi-agent: move concurrency guidance into v2 usage hints (#27569)`), `84520225b9928a9eee3ed2c9072fb5d26d6fc6e6` (`chore: prompt MAv2 (#27919)`), `8f2d6416ce41be54551185c640f83e22f061eccd` (`Support plaintext agent messages (#27830)`).
- Developer info: moves concurrency guidance from generic config/spec areas into v2 usage hints, trims prompt/spec wording and adds plaintext agent message support through protocol/schema, session reconstruction, rollout policy, history normalization, Guardian/hooks/realtime/memory/web-search consumers and tests.
- Why: subjects/diffs show MAv2 guidance becoming localized and message content supporting both encrypted and plaintext representations.
- User info: model guidance for v2 concurrency is clearer and MAv2/inter-agent messages can be represented as plaintext where the native protocol allows it.
- Compatibility/risks: plaintext support broadens message wire/storage shape; fork consumers must handle both encrypted and plaintext agent message content without assuming one representation.
- Evidence: `EVID-140-usage-plaintext`.

## `rust-v0.141.0`

- Release commit: `3fb81667d30d9d24297216ea61fbfcc4351b2aa9`
- Дата: `2026-06-17T20:59:28-07:00`
- Subject: release notes cover encrypted remote execution, executor-native cwd/shell preservation, selected executor plugin MCP activation, immediate child-thread listing, external-agent import result accounting, realtime controls, `wait_agent` steer interruption, MAv2 prompt updates, Guardian review isolation and response-item metadata plumbing.

### MAv2 prompt guidance and steer-interruptible wait (`#28283`, `#28341`)

- Коммиты: `127224cacce1bc08922f21a5ff3e3ddd33246474` (`[codex] update multi-agent v2 prompts (#28283)`), `ee40dddbf6a50a6f0641180ca299be7a3a03fd22` (`core: let steer interrupt wait_agent (#28341)`).
- Developer info: updates default MAv2 root/subagent hints with direct collaboration-tool guidance, shared-workspace behavior, `fork_turns` context tradeoff and current `interrupt_agent` naming, then changes `wait_agent` to subscribe to mailbox or steered-input activity and return `Wait interrupted by new input.` when same-turn user steer arrives.
- Why: commit bodies and diffs state that evaluated MAv2 prompt guidance was missing from defaults and that long `wait_agent` calls made user steer feel unresponsive until timeout or mailbox activity.
- User info: agents get clearer guidance about when to parallelize, how shared workspaces behave and how `fork_turns="none"`/`"all"` affect spawned context; user input can interrupt a blocking `wait_agent` without waiting for timeout.
- Compatibility/risks: prompt text remains model-visible contract, so fork prompt changes must keep current tool names and no-spawn guard placement aligned. `wait_agent` early return is a successful tool output with `timed_out: false`, not an error path.
- Evidence: `EVID-141-prompts-wait`.

### Direct spawned-child listing (`#26662`)

- Коммиты: `dfd03ea01bbec2613013b477fb82abc67534a7d7` (`feat(app-server): filter threads by parent (#26662)`).
- Developer info: adds experimental `ThreadListParams.parentThreadId` / `parent_thread_id`, validates it in app-server, routes it through thread-store listing and uses persisted `thread_spawn_edges` state to return direct children.
- Why: commit body says clients that display or coordinate spawned subagents need an authoritative immediate-child snapshot after connect or missed live events instead of scanning unrelated threads or reconstructing from rollouts.
- User info: app-server clients can ask for direct spawned children of a parent thread and recover subagent lists more reliably.
- Compatibility/risks: the filter is experimental, direct-only and state-DB-backed; it does not recursively return descendants and intentionally excludes Review/Guardian threads from this lifecycle.
- Evidence: `EVID-141-parent-filter`.

### Selected executor plugin MCPs and thread-scoped activation (`#27870`, `#27893`)

- Коммиты: `b3c423e475a9fa2bb1ad493e09408d13732d4b98` (`Discover stdio MCP servers from selected executor plugins (#27870)`), `c8c78b63a7cbc7101884613d0301db5cc58d730d` (`Activate selected executor plugin MCPs in app-server (#27893)`).
- Developer info: adds executor-plugin MCP discovery through the owning executor filesystem, freezes selected plugin MCP declarations per active thread runtime, then installs the contributor in app-server so `thread/start.selectedCapabilityRoots` can activate selected plugin stdio MCP servers only on that thread.
- Why: release notes and commit bodies describe selected executor plugin MCP activation as part of thread-scoped plugin/runtime capability selection, with no host-filesystem fallback and no global MCP registration for unselected threads.
- User info: a selected executor plugin can expose its stdio MCP tools to the chosen thread, survive normal MCP refresh and remain invisible to threads where that root was not selected.
- Compatibility/risks: only stdio declarations are activated; HTTP executor MCP placement, resume/fork persistence and dynamic catalog lifecycle remain separate decisions. Fork MCP/tool changes must use thread-scoped contributor paths instead of global tool lists.
- Evidence: `EVID-141-selected-plugin-mcp`.

### Guardian review isolation and external-agent import accounting (`#28285`, `#28008`)

- Коммиты: `a18de1f3b6482cf162a473d173db7bf24206333e` (`guardian: isolate review context from skills and memories (#28285)`), `fc1fb682a7b94b8146ee1ce6b96f278669374988` (`[codex] Add external agent import result accounting (#28008)`).
- Developer info: Guardian derived review sessions now skip skill/plugin discovery from untrusted parent transcripts and disable memory context/tools; external-agent config import now returns an `importId` and completed notifications include type-level successes/failures for plugins, sessions and other migration items.
- Why: Guardian commit body states transcript evidence must not trigger skill/plugin injection or unrelated memory context; import accounting commit body states clients need correlation and partial-failure accounting for background plugin/session imports.
- User info: approval review context is smaller and less likely to be influenced by unrelated skills/memories, while external-agent migration UIs can correlate import completion and show which item types succeeded or failed.
- Compatibility/risks: Guardian isolation is a trust-boundary change for review subagents and should stay independent from ordinary user turns. External-agent import accounting changes app-server protocol/schema payloads and clients must handle partial success.
- Evidence: `EVID-141-guardian-external-import`.

### Optional response item metadata and inter-agent compatibility (`#28355`)

- Коммиты: `040dafa32d5312f7803786d208ed6a0a11dfdabb` (`feat(core): add metadata field to ResponseItem (#28355)`).
- Developer info: adds optional `ResponseItemMetadata` with `turn_id`, optional metadata fields across response-item variants including `AgentMessage`, preserves metadata through rollout/resume/history rewrites, adds optional metadata to `InterAgentCommunication`, and strips metadata before non-OpenAI Responses requests.
- Why: commit body frames this as mechanical plumbing for future turn-id stamping, with compatibility for histories that omit metadata and provider compatibility for non-OpenAI requests.
- User info: no immediate end-user command changes, but inter-agent and rollout history items can carry optional turn metadata without breaking old metadata-free records.
- Compatibility/risks: metadata is optional and missing fields remain valid. Fork consumers of agent-message history must preserve unknown/optional metadata and not assume it is always present or provider-visible.
- Evidence: `EVID-141-response-metadata`.

## `rust-v0.142.0`

- Release commit: `3a76f3ac68c8949d1cac6ea769b6ec7b8953a415`
- Дата: `2026-06-22T23:36:01+02:00`
- Subject: release notes cover rollout token budgets across agent threads, app-server multi-agent delegation modes, terminal subagent error propagation, plugin/MCP loading fixes, time/reminder APIs and runtime/disconnect hardening. Direct multi-agent entries here are typed MAv2 envelopes, join keys, parent-visible child errors, multi-agent mode controls, rollout budget enforcement, Guardian child-session startup and external-agent import accounting.

### Typed MAv2 message envelopes, join keys and parent-visible child errors (`#28368`, `#28561`, `#28375`)

- Коммиты: `5b22a8e5b13bd4bc3b331e7a1392569107b7bccf` (`feat: render typed envelopes for multi-agent v2 messages (#28368)`), `45f603302c45269737db97443612bb4876365798` (`Add join key for MAv2 inter-agent messages (#28561)`), `1b24ba912ac4c56ef936364deb1c3e294b0ef9fa` (`core: surface terminal subagent errors to parent agents (#28375)`).
- Developer info: adds typed inter-agent completion/message envelopes through context/protocol/session/rollout-trace paths, adds join-key metadata to correlate MAv2 inter-agent messages and changes terminal child-agent errors so the parent sees the failure instead of an empty successful completion.
- Why: release notes explicitly call out terminal subagent error surfacing, while the diffs cover `core/src/context/inter_agent_completion_message.rs`, `protocol/src/models.rs`, `protocol/src/protocol.rs`, `core/src/tools/handlers/multi_agents_v2/{message_tool,spawn}.rs`, `core/src/session/*` and `rollout-trace/src/reducer/tool/agents.rs`.
- User info: parent agents get structured and correlated child-agent message/error information, making failed subagent work visible to the orchestrating agent.
- Compatibility/risks: join-key/message metadata are compatibility-sensitive history/protocol details. Fork consumers must preserve typed envelopes and optional metadata instead of flattening them to transcript text.
- Evidence: `EVID-142-mav2-messages`.

### Thread/turn multi-agent mode control (`#28685`, `#28792`, `#29324`)

- Коммиты: `fc8c6b73841e279f95f53b08771a7969e953bdf4` (`Add per-turn multi-agent mode (#28685)`), `7abfcf220bbb57029e2ff5d9914124aef7ef3d0f` (`Expose thread-level multi-agent mode (#28792)`), `c03742ca0a78a8e54cd881032a2327363678b5aa` (`Simplify multi-agent mode controls (#29324)`).
- Developer info: introduces per-turn `MultiAgentMode`, projects thread-level mode through app-server thread lifecycle/settings responses and then simplifies the control surface/config mapping around thread and turn mode selection.
- Why: release notes state app-server clients can configure multi-agent delegation as disabled, explicit-request-only or proactive at thread and turn level. Diffs touch `app-server-protocol/src/protocol/v2/{thread,turn}.rs`, generated schemas, `core/src/session/multi_agents.rs`, `core/src/context/multi_agent_mode_instructions.rs`, `core/src/tools/handlers/multi_agents_spec.rs`, `core/src/thread_manager.rs` and app-server request processors.
- User info: clients can control whether agents may proactively delegate, delegate only on explicit request or not delegate, with thread-level defaults and per-turn overrides.
- Compatibility/risks: this is a protocol/config contract. Fork changes must update schemas, app-server README/projections, runtime config locking and model-visible mode instructions together.
- Evidence: `EVID-142-multi-agent-mode`.

### Rollout token budgets for agent threads (`#28746`, `#28494`, `#28707`, `#29423`)

- Коммиты: `ecc4c30e281a9dff77ab45e0365c73a2a526a520` (`[codex] add rollout token budget configuration (varlength 1/N) (#28746)`), `32a696dbacaa1383745455ea2a77d5477891ed0b` (`[codex] rollout budget implementation (varlength 2/N) (#28494)`), `dac588f41398e8b628d71838d5745dad430477f1` (`[codex] abort turns when rollout budgets expire (token budget 3/3) (#28707)`), `bd5bd953fb2a5d610a30d112eafe20b644924085` (`[codex] configure rollout budget reminder thresholds (#29423)`).
- Developer info: adds feature/config schema for rollout budgets, runtime accounting in `core/src/rollout_budget.rs` and `core/src/session/rollout_budget.rs`, integration with `ThreadManager`/agent control, abort behavior for compact/regular/review/user-shell tasks and reminder thresholds.
- Why: release notes call out configurable rollout token budgets tracking usage across agent threads, remaining-budget reminders and exhausted-budget aborts.
- User info: long-running parent/child agent work can be bounded by a shared token budget, with reminders before exhaustion and controlled turn abort once exhausted.
- Compatibility/risks: budget state affects scheduling and task lifecycle; fork autonomy changes must respect native budget accounting instead of adding parallel counters.
- Evidence: `EVID-142-rollout-budget`.

### Guardian child session startup and external-agent import results (`#27982`, `#28396`)

- Коммиты: `15f448d8b06c25c9ad04b2abeb3e4e2f07e4c327` (`[codex] Start the guardian child session when parent session is started (#27982)`), `314fa3d25b1f8a2542ddefa72a8cdb706e9ee3c6` (`[codex] Record external agent import results (#28396)`).
- Developer info: starts the Guardian child session with the parent session through Guardian/session startup paths, and records external-agent import progress/completion/type-level failures through app-server protocol, state migrations and TUI/app-server event targets.
- Why: diffs show Guardian child-session lifecycle moving into startup prewarm/session paths and external-agent import result accounting gaining durable IDs/notifications/state.
- User info: Guardian review child sessions are ready with the parent lifecycle, and external-agent migration UIs can report import progress and partial failures more precisely.
- Compatibility/risks: Guardian child startup remains separate from ordinary MAv2 spawn tools. External-agent import accounting changes app-server protocol/state payloads and must keep partial-success compatibility.
- Evidence: `EVID-142-guardian-import`.

## `rust-v0.142.2`-`rust-v0.142.5`

- Upper release commit: `26de83050b20f7e0ee211b9739e52ae00ce8032a`
- Дата верхней границы: `2026-06-30T20:30:24-04:00`
- Subject: patch releases add model world-state environment/subagent context, durable turn-id metadata, default searched MCP/v1 tool exposure, Ultra-derived proactive MAv2 mode and restored V1 delegation guidance. `rust-v0.142.3`, `rust-v0.142.4` and `rust-v0.142.5` did not add further direct multi-agent runtime entries in this pass.

### Model world-state environment/subagent context (`#29249`, supporting `#29504`)

- Коммиты: `3b32d861c5ecc1e705812bc21177d0038e3c05ee` (`[codex] migrate environment context to model world state (#29249)`), supporting stabilization `97dc6abb11a1e5e15acb5b8a499e853418ea4902` (`fix: world state response item test (#29504)`).
- Developer info: replaces parallel environment-context baseline handling with `WorldState` / `EnvironmentsState`, keeps `<subagents>` in the model-visible environment state and stores the world-state diff baseline in `ContextManager`.
- Why: diffs add `core/src/context/world_state/*`, `core/src/session/world_state.rs` and session/context-manager integration; commit body explicitly notes initial context, turn-to-turn updates and resume baseline behavior.
- User info: model-visible environment/subagent context is rendered through a single baseline/diff system, reducing inconsistent context updates across turns.
- Compatibility/risks: world state itself is not serialized; resume seeds from latest `TurnContextItem`, whose legacy shape can reconstruct only the primary environment as `local` until live state establishes exact environment ids.
- Evidence: `EVID-142.5-world-state-subagents`.

### Durable `turn_id` metadata for inter-agent history (`#28360`)

- Коммиты: `4a82ecc3c9fcb8b9f21d5144c40a0dfdab54a02a` (`feat(core): store turn_id on ResponseItem metadata (#28360)`).
- Developer info: stamps `internal_chat_message_metadata_passthrough.turn_id` on response items entering durable history, including inter-agent communication history, and preserves existing turn ids through persistence, resume reconstruction, compaction, forked history and websocket incremental reuse.
- Why: diffs touch `protocol/src/models.rs`, session history/compaction paths, app-server protocol schemas and `core/tests/suite/subagent_notifications.rs`.
- User info: inter-agent message/history items can stay associated with the turn that introduced them across resume/fork/compaction flows.
- Compatibility/risks: metadata remains optional; consumers must preserve unknown/optional metadata and strip provider-incompatible metadata at request boundaries.
- Evidence: `EVID-142.5-turn-id-inter-agent`.

### Tool-search default and V1 multi-agent tool discovery (`#29486`)

- Коммиты: `c53b1dae09db40902c59f6a0d57d0dcc334926db` (`[codex] Use tool search for MCP tools by default (#29486)`).
- Developer info: because V1 multi-agent tools participate in the same namespaced/deferred tool planning path, supported turns expose `tool_search` first and return searchable deferred tools instead of making every V1 tool directly visible in the initial request.
- Why: diffs update `core/src/tools/spec_plan.rs`, `core/src/mcp_tool_exposure.rs`, feature flags and search-tool tests; the `add_collaboration_tools` path now uses the simplified `search_tool_enabled` condition for deferred exposure.
- User info: agents may need to search for V1 lifecycle tools before calling them when tool search is available.
- Compatibility/risks: direct V1 tools remain fallback for unsupported model/provider combinations; fork prompt/tests should not assume the initial request always contains all V1 tools.
- Evidence: `EVID-142.5-tool-search-v1-agents`.

### Ultra reasoning owns proactive MAv2 mode (`#29899`)

- Коммиты: `aedb8f345a221b22105daad9fdf080988026fbca` (`[codex] Add Ultra reasoning effort (#29899)`).
- Developer info: adds `ReasoningEffort::Ultra`, converts it to backend-compatible `max` at inference boundaries, derives effective MAv2 mode from reasoning effort (`ultra` -> proactive, other eligible efforts -> explicitRequestOnly) and removes selected multi-agent mode from core session/thread/spawn plumbing. App-server `multiAgentMode` fields remain deprecated compatibility wire: accepted but ignored, responses report `explicitRequestOnly`.
- Why: commit body states reasoning effort becomes source of truth; diffs touch app-server thread/turn protocol docs, core session/thread manager, agent control/spawn, MAv2 tests and model reasoning UI.
- User info: clients request proactive multi-agent behavior by selecting Ultra reasoning instead of setting `multiAgentMode`.
- Compatibility/risks: this supersedes the `rust-v0.142.0` thread/turn multi-agent mode controls as current runtime authority. Fork changes must not revive a parallel `multiAgentMode` runtime state.
- Evidence: `EVID-142.5-ultra-mav2-mode`.

### V1 delegation guidance restored (`#30511`)

- Коммиты: `3b8b60a58399ff79a41f8457710ae5a5fae5c39f` (`[codex] Restore v1 delegation guidance (#30511)`).
- Developer info: restores model-visible V1 `spawn_agent` guidance that depth/research/investigation requests do not authorize spawning, and that critical-path/tightly coupled work should stay local while independent sidecar tasks can be delegated.
- Why: diffs touch `core/src/tools/handlers/multi_agents_spec.rs` and focused spawn-description/search-tool tests.
- User info: V1 sub-agent spawning is more explicitly constrained even when the tool is available.
- Compatibility/risks: this is prompt/tool-description contract, not runtime enforcement. Fork changes must keep authorization guidance aligned with actual tool availability and AGENTS/skill delegation rules.
- Evidence: `EVID-142.5-v1-delegation-guidance`.
