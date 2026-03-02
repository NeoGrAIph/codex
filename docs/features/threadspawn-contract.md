# ThreadSpawn Contract

## Feature passport

- Code name: `ThreadSpawn Contract`.
- Status: `implemented`.
- Primary commit: `935e4739d596755505a1a3814e84ddf9bd14af2e`.
- Related baseline commit: `0f9eed3a6` (`agent_type` alias for `agent_role` introduced earlier and reused here).
- Goal: зафиксировать и стабилизировать wire-контракт `SubAgentSource::ThreadSpawn`, чтобы метаданные spawned-треда не терялись при сериализации, merge и resume-путях.
- Scope in:
- `codex-rs/protocol/src/protocol.rs`
- `codex-rs/core/src/tools/handlers/multi_agents.rs`
- `codex-rs/core/src/agent/control.rs`
- `codex-rs/app-server/src/codex_message_processor.rs`
- `codex-rs/app-server/src/filters.rs`
- `codex-rs/app-server/tests/suite/v2/thread_list.rs`
- `codex-rs/app-server-protocol/schema/{json,typescript}/*` (regen артефактов схем)
- Scope out:
- добавление новых user-facing RPC методов;
- изменение общей модели ownership/subtree;
- изменение runtime semantics `thread_note` (документируется отдельно).

## Problem statement

До этого изменения контракт `ThreadSpawn` был частично нестабилен:

- policy-метаданные (`allow_list` / `deny_list`) не были частью устойчивого wire-контракта;
- при merge/rehydration путях существовал риск потерять часть spawn-метаданных;
- backward compatibility по `agent_type` уже существовал и должен был сохраниться без регрессий, при этом canonical запись в ответах должна оставаться консистентной.

## Contract (wire format)

`SessionSource::SubAgent::ThreadSpawn` теперь включает:

- `parent_thread_id: ThreadId`
- `depth: i32`
- `agent_nickname: Option<String>`
- `agent_role: Option<String>`
- `allow_list: Option<Vec<String>>`
- `deny_list: Option<Vec<String>>`

Сериализация/десериализация:

- canonical ключи для роли и никнейма: `agent_role`, `agent_nickname`;
- для обратной совместимости `agent_role` продолжает принимать alias `agent_type` при десериализации;
- `allow_list` и `deny_list` отмечены `#[serde(default)]`, поэтому отсутствие ключа безопасно и декодируется как `None`.

## Runtime propagation rules

### 1) Spawn source construction

`multi_agents::thread_spawn_source` формирует `ThreadSpawn` с явной инициализацией policy-полей:

- `allow_list: None`
- `deny_list: None`

Это устраняет неявность и делает базовый контракт полным.

### 2) Agent control (spawn/resume)

`AgentControl` сохраняет policy-поля в обоих критичных путях:

- при подготовке source к spawn (где резервируется `agent_nickname`);
- при resume/rehydration `ThreadSpawn` из persisted summary.

Итог: `allow_list`/`deny_list` не теряются при повторном входе в runtime и при реконструкции session source.

### 3) App-server metadata merge

`with_thread_spawn_agent_metadata(...)` в app-server:

- продолжает merge `agent_nickname`/`agent_role`;
- сохраняет `allow_list`/`deny_list` без потерь.

Итог: обогащение метаданных при выдаче thread summaries не затирает policy-контекст.

### 4) Runtime enforcement for non-app MCP tools in Apps mode

Дополнительный bugfix (`3ce86025b`, адаптированный под `rust-v0.106.0`) включает фактическое применение
`allow_list` / `deny_list` к non-app MCP инструментам при сборке toolset в Apps mode:

- фильтрация применяется только к non-app MCP (`mcp__<server>__*`, кроме `codex_apps`);
- `deny_list` имеет приоритет над `allow_list`;
- если `allow_list` отсутствует, разрешаются все non-app инструменты, которые не заблокированы deny;
- wildcard-шаблоны `*` и `?` поддерживаются;
- policy для `codex_apps` инструментов по-прежнему определяется connector-политикой и не переопределяется этим шагом.

### 5) `close_agent` ownership guardrails

Для sub-agent execution контекста закреплены ограничения завершения агентов:

- запрещено завершать агента вне собственного subtree;
- запрещено self-close (`close_agent` для `session.conversation_id`);
- в обоих случаях инструмент возвращает model-facing ошибку и не отправляет `Op::Shutdown` для запрещённой цели.

### 6) Runtime spawn capacity default

Runtime default ceiling для количества одновременно живых sub-agent тредов увеличен:

- `DEFAULT_AGENT_MAX_THREADS`: `6 -> 15` (`core/src/config/mod.rs`);
- изменение применяется как default policy и не меняет wire-формат `ThreadSpawn`.

## Compatibility guarantees

- Legacy входной payload с `agent_type` (совместимость добавлена ранее, в `0f9eed3a6`) поддерживается и маппится в `agent_role`.
- На выходе сериализация остаётся canonical: `agent_role` (а не `agent_type`).
- Отсутствие `allow_list`/`deny_list` в старых данных не ломает декодирование благодаря `serde(default)`.

## Validation matrix (implemented in commit)

- Protocol tests:
- десериализация legacy alias `agent_type`;
- roundtrip сериализации/десериализации для `allow_list`/`deny_list`.
- Core tests:
- обновлены фикстуры `ThreadSpawn` с новыми полями.
- App-server tests:
- добавлен тест, гарантирующий сохранение policy-списков при metadata merge;
- обновлены thread-list/filter фикстуры.
- Schema artifacts:
- перегенерированы JSON/TS схемы `app-server-protocol` для синхронизации с контрактом.

## Integration notes for next commits

- Любой код, который создаёт `SubAgentSource::ThreadSpawn`, должен явно прокидывать `allow_list` и `deny_list`.
- Любой merge/update слой над `ThreadSpawn` обязан сохранять policy-поля, даже если обновляет только nickname/role.
- Любое изменение wire-формы `ThreadSpawn` должно сопровождаться:
- обновлением этого документа;
- regen схем (`just write-app-server-schema`);
- тестами на backward compatibility и roundtrip.

## Doc changelog

- 2026-02-27: Initial document added for commit `935e4739d` (`ThreadSpawn contract hardening`, protocol and metadata propagation).
- 2026-02-27: Corrected historical note for backward compatibility (`agent_type` alias introduced in `0f9eed3a6`) and fixed `depth` type to `i32`.
- 2026-02-27: Documented runtime non-app MCP allow/deny enforcement in Apps mode for Stage 1 bugfix (`3ce86025b`, adapted to `rust-v0.106.0`).
- 2026-03-01: Added `close_agent` guardrails note: subtree-only shutdown and explicit self-close rejection for sub-agents.
- 2026-03-02: Documented runtime default spawn capacity increase (`DEFAULT_AGENT_MAX_THREADS = 15`).
