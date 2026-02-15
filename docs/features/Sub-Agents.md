# Sub-Agents

## Feature passport

- Code name: `SA` (`Sub-Agents`).
- Status: `implemented`.
- Goal: безопасная и предсказуемая работа цепочек `spawn_agent/send_input/wait/resume_agent/close_agent` с контрактом ownership для subtree и контролируемой metadata (`thread_note`).
- Scope in: `codex-rs/core` (`agent/control`, `tools/handlers/collab`, `tools/handlers/list_active_agents`, `tools/spec`, `tools/registry`, `thread_manager`, `rollout/session_index`), protocol/app-server события, TUI status/notifications.
- Scope out: SAW overlay transitions и hotkeys (документируются отдельно в `docs/features/saw.md`).
- API impact:
- `spawn_agent` поддерживает `agent_type`, `agent_name`, `working_directory`, `model`, `reasoning_effort`, `thread_note`;
- добавлены `list_agents`, `list_active_agents` и `set_thread_note` в collab toolset;
- добавлен runtime event `ThreadNoteUpdated` и bootstrap поле `SessionConfigured.thread_note`;
- app-server v2: `thread/note/set` (`ThreadSetNoteParams/Response`) и `thread/note/updated` notification.
- Security impact:
- `close_agent` из sub-agent треда ограничен собственным subtree;
- template-based `allow_list/deny_list` применяется к дочерним тредам и ограничивает доступный набор tools;
- template `read_only: true` принудительно задаёт `SandboxPolicy::ReadOnly` для spawned thread.
- Config impact: увеличен дефолт `agent_max_threads` до `12`.
- Baseline references: `docs/research/0.99/README.md`, `docs/research/0.99/TUI_WINDOWS_AND_OVERLAYS.md`, `docs/research/0.101/worker-prompt-flow.md`.

## User contract

- `spawn_agent` принимает:
- `message` или `items` (одновременно нельзя);
- `agent_type` (включая template-backed роли из `core/templates/agents/*.md`);
- `agent_name` (опционально, persona внутри шаблона);
- `working_directory` (опционально, override `cwd` для spawned thread);
- `model` и `reasoning_effort` (опциональные overrides);
- `thread_note` (опционально).
- Приоритет конфигурации spawned агента:
- override из `spawn_agent` (`model`/`reasoning_effort`);
- defaults из выбранного `agent_name`;
- defaults из template (`agent_type`);
- inherited turn config.
- Sandbox policy для spawned агента:
- по умолчанию наследуется из parent turn;
- если template содержит `read_only: true`, policy принудительно переключается в `SandboxPolicy::ReadOnly`.
- Валидация `spawn_agent`:
- неизвестный `model` отклоняется до spawn (с перечнем доступных моделей);
- неподдерживаемый `reasoning_effort` отклоняется до spawn (с перечнем поддерживаемых значений для выбранной модели);
- при временной недоступности model list возвращается retryable сообщение (`Models are being updated; try spawn_agent again in a moment.`).
- `working_directory` при spawn:
- если указан и после `resolve_path` отличается от `turn.cwd`, перед spawn отправляется обязательный user approval request;
- если `spawn_agent` вызван из sub-agent (`SessionSource::SubAgent::ThreadSpawn`), approval для `working_directory` маршрутизируется в текущий turn родительского thread/session;
- если parent-thread routing недоступен (например, родительский thread не найден), используется fallback на approval в текущей сессии вызова;
- при `Denied/Abort` spawn отклоняется;
- если `working_directory` не указан или совпадает с текущей `working directory` сессии, spawn выполняется без дополнительного approval.
- `thread_note` при spawn:
- если `thread_note` передан, он нормализуется и применяется к дочернему треду;
- если не передан, генерируется default note: `agent_type=...; agent_name=...; agent_description=...` (где доступны значения);
- если установка note не удалась, `spawn_agent` остаётся успешным (`agent_id` возвращается), ошибка публикуется как background warning.
- `set_thread_note`:
- принимает `id` и `note`;
- пустое/пробельное значение очищает note;
- возвращает `submission_id` и нормализованное `thread_note`.
- `send_input`:
- поддерживает `message` или структурированные `items`;
- поддерживает `interrupt=true` для принудительного прерывания текущей задачи агента.
- `resume_agent`:
- восстанавливает закрытый агент по `id` (если доступен rollout path и не превышен depth limit);
- для активного агента выполняет no-op.
- `wait`:
- принимает `ids` (непустой список) и опциональный `timeout_ms`;
- `timeout_ms` ограничивается диапазоном `[10_000, 1_800_000]`, default `300_000`;
- возвращает `{ status: {<thread_id>: <final_status>, ...}, timed_out: <bool> }`;
- в `status` всегда есть первый агент, достигший final state, и могут быть добавлены другие final статусы из той же гонки;
- если ни один агент не достиг final state до дедлайна, `status` пустой, `timed_out=true`.
- `close_agent`:
- из root-треда можно закрыть любой доступный агент;
- из sub-agent треда разрешено закрывать только себя и descendants;
- закрытие агента каскадно завершает весь его subtree.
- `list_agents`:
- по умолчанию возвращает catalog template-ролей (`agent_type`, `description`, `allow_list`, `deny_list`, optional `agent_names`);
- поддерживает `agent_type` filter и `expanded=true`;
- при `expanded=true` добавляет role-level поля `model`, `reasoning_effort`, `default_prompt`, а для `agent_names` — `model`, `reasoning_effort`, `prompt`.
- `list_active_agents`:
- возвращает spawned thread-агентов (`SessionSource::SubAgent::ThreadSpawn`) с runtime metadata:
  `thread_id`, `thread_name`, `thread_note`, `agent_type`, `agent_name`, `status`, `status_duration_sec`, `model`, `reasoning_effort`, `updated_at`;
- поддерживает опции:
  `scope` (`children` default, `descendants`, `all`), `include_tree` (добавляет `parent_thread_id` и `depth`), `include_closed`;
- при `include_closed=false` исключаются только `shutdown` и `not_found`; `completed`/`errored` считаются активными (ожидающими новый input);
- `status_duration_sec` вычисляется как snapshot на момент вызова инструмента по последнему transition event текущего статуса (reverse-scan rollout jsonl).
- После `spawn_agent` запускается background observer статуса дочернего агента:
- lifecycle: `pending_init -> running -> completed/errored/shutdown`;
- observer не завершается на первом финальном статусе и продолжает мониторинг до закрытия канала статусов;
- каждое изменение публикуется `BackgroundEvent`;
- для финальных состояний устанавливается `BackgroundEvent.is_final = true`.
- Desktop notification при завершении суб-агента:
- при `BackgroundEvent.is_final == true` TUI отправляет `Notification::SubAgentComplete`;
- backend доставки: BEL (`\x07`) или OSC 9 (`\x1b]9;<msg>\x07`) в зависимости от терминала;
- уведомление подавляется при активном фокусе терминала;
- тип уведомления: `sub-agent-complete` (фильтруется через `[tui] notifications`).
- Финальный статус `completed` формируется как:
- `agent <role> (<agent-id>) [<call-id>] completed in <time>: <agent message>`.

## Implementation map

- Spawn/interaction contract:
- `codex-rs/core/src/tools/handlers/collab.rs`
- `codex-rs/core/src/tools/handlers/collab/spawn.rs`
- Tool schemas and registration:
- `codex-rs/core/src/tools/spec.rs`
- `codex-rs/core/src/tools/handlers/mod.rs`
- Runtime active-agent listing:
- `codex-rs/core/src/agent/control.rs`
- `codex-rs/core/src/tools/handlers/list_active_agents.rs`
- Template catalog and policy:
- `codex-rs/core/src/agent/role_templates.rs`
- `codex-rs/core/src/tools/handlers/list_agents.rs`
- `codex-rs/core/src/tools/policy.rs`
- `codex-rs/core/src/tools/registry.rs`
- `codex-rs/core/templates/agents/orchestrator.md`
- `codex-rs/core/templates/agents/worker.md`
- `codex-rs/core/templates/agents/explorer.md`
- Ownership/cascade shutdown:
- `codex-rs/core/src/agent/control.rs`
- `codex-rs/core/src/thread_manager.rs`
- Runtime thread note persistence/events:
- `codex-rs/core/src/util.rs`
- `codex-rs/core/src/rollout/session_index.rs`
- `codex-rs/core/src/codex.rs`
- `codex-rs/core/src/codex_thread.rs`
- `codex-rs/core/src/agent/control.rs`
- `codex-rs/protocol/src/protocol.rs`
- App-server wiring:
- `codex-rs/app-server-protocol/src/protocol/common.rs`
- `codex-rs/app-server-protocol/src/protocol/v2.rs`
- `codex-rs/app-server/src/codex_message_processor.rs`
- `codex-rs/app-server/src/bespoke_event_handling.rs`
- TUI projection and notifications:
- `codex-rs/tui/src/chatwidget.rs`
- `codex-rs/tui/src/status/card.rs`
- `codex-rs/tui/src/resume_picker.rs`
- Build/config plumbing:
- `codex-rs/core/build.rs`
- `codex-rs/core/src/config/mod.rs`

## Verification matrix

- `cd codex-rs && just fmt`
- `cd codex-rs && cargo test -p codex-core --lib`
- `cd codex-rs && cargo test -p codex-tui`
- `cd codex-rs && cargo test -p codex-app-server-protocol`
- `cd codex-rs && cargo test -p codex-exec`
- Focused checks:
- `cd codex-rs && cargo test -p codex-core tools::handlers::collab::tests::set_thread_note_submits_and_can_clear_note`
- `cd codex-rs && cargo test -p codex-core tools::handlers::collab::tests::close_agent_cascades_to_descendants`
- `cd codex-rs && cargo test -p codex-core tools::handlers::list_active_agents::tests::list_active_agents_returns_descendants_with_tree_metadata`
- `cd codex-rs && cargo test -p codex-core tools::spec::tests::list_agents_schema_supports_filter_and_expanded_options`
- `cd codex-rs && cargo test -p codex-core tools::spec::tests::list_active_agents_schema_supports_scope_and_visibility_flags`
- `cd codex-rs && cargo test -p codex-core --lib spawn_agent_rejects_different_working_directory_without_approval`
- `cd codex-rs && cargo test -p codex-core --lib spawn_agent_accepts_matching_working_directory_without_approval`
- `cd codex-rs && cargo test -p codex-core --lib spawn_agent_routes_working_directory_approval_to_parent_thread`

Known unrelated failures must be tracked separately and explicitly marked as non-feature.

## Doc changelog

- 2026-02-14: Updated `wait` timeout contract to default `300_000` ms (5 min) and max `1_800_000` ms (30 min).
- 2026-02-14: Expanded Sub-Agents contract to cover `list_agents`, `set_thread_note`, spawn-time `thread_note`, subtree close guards, and policy propagation via template `allow_list/deny_list`.
- 2026-02-14: Added protocol/app-server/TUI mapping for `thread_note` (`SessionConfigured.thread_note`, `ThreadNoteUpdated`, `thread/note/set`, `thread/note/updated`).
- 2026-02-14: Added observer finality contract (`BackgroundEvent.is_final`) and desktop notification behavior (`sub-agent-complete`).
- 2026-02-14: Updated implementation map and verification matrix to match current fork state.
- 2026-02-14: Clarified strict spawn-time validation (`model`/`reasoning_effort`) and exact `wait`/`list_agents(expanded)` payload semantics.
- 2026-02-14: Added template `read_only` spawn-time sandbox override contract (`SandboxPolicy::ReadOnly`) for sub-agents.
- 2026-02-14: Added `spawn_agent.working_directory` override contract with mandatory approval when target directory differs from parent session `cwd`.
- 2026-02-14: Clarified `working_directory` approval routing for sub-agent calls: prompt is emitted via parent thread current turn/session.
- 2026-02-14: Documented parent-routing fallback behavior and added explicit verification test for routed approval flow.
- 2026-02-14: Added `list_active_agents` collab contract with scope/tree/closed filters and snapshot `status_duration_sec` from rollout reverse-scan.
- 2026-02-14: Extracted `list_active_agents` implementation into dedicated handler file (`core/src/tools/handlers/list_active_agents.rs`) to reduce churn in `collab.rs`.
- 2026-02-14: Updated `list_active_agents` semantics: `completed`/`errored` remain in active list (waiting state); closed-only filter excludes `shutdown/not_found`.
