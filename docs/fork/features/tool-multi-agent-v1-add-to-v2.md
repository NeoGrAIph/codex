# Пространство имён `multi_agent_v1` рядом с V2

## Паспорт возможности

- Кодовое имя: `tool-multi-agent-v1-add-to-v2`.
- Статус: draft/uncommitted доработки в рабочем дереве `fork/144.1`; activation baseline перенесён из `fork/142`, commit `dcf47dfce3b682bc2efe64025ab2de8265fab145`.
- Цель: при effective `MultiAgentVersion::V2` показывать модели отдельный legacy namespace `multi_agent_v1` и сохранять его V1 wire contract, одновременно проводя созданных через него agents через нативный path-based V2 runtime.
- В scope: tool planning/spec projection, bounded role discovery, V1-under-V2 runtime adapter, lifecycle authorization, V2 timeout/depth policy, resume reconstruction, reserved namespace contract, generated config schema, dynamic-tool collision checks, app-server error projection, regression tests и fork-документация.
- Вне scope: новые protocol fields, V1/V2 aliases, новый registry, изменение TUI/API wire shapes, dependency updates, rollout/SQLite schema migrations и автоматическая замена или перезапуск установленных CLI/app-server binaries.

## Пользовательский смысл

В V2-сессии модель видит нативный namespace `collaboration` по умолчанию либо configured V2 namespace и отдельный `multi_agent_v1` с инструментами `spawn_agent`, `send_input`, `resume_agent`, `wait_agent`, `close_agent`. V1 surface сохраняет ответы на основе `agent_id`; V2 surface сохраняет path-based contract. Оба набора управляют одним child runtime, а не двумя параллельными деревьями состояния.

## Контракт

- `multi_agent_v1.spawn_agent` сохраняет V1 input/output schemas: model-visible `task_name` не добавляется, ответ остаётся `{agent_id, nickname}`.
- При effective V2 новый V1-spawned child получает внутренний canonical `AgentPath` вида `/root/.../agent_<uuid-simple>`. Этот путь хранится в `SessionSource::SubAgent(ThreadSpawn)` и используется нативными V2 inventory, path resolution и terminal completion delivery. `multi_agent_v1.send_input` намеренно сохраняет raw V1 `UserInput` transport, потому что V2 `InterAgentCommunication` не может без потерь представить image/local-image/skill/mention payloads.
- UUID является locator, а не capability. V1 lifecycle tools под V2 принимают live target только из того же root-scoped `AgentControl`; cold target авторизуется только через authoritative persisted spawn graph, который атомарно возвращает immediate parent и status. Эти graph-derived значения управляют resume и не могут быть заменены caller/stored metadata. Effective-V2 `ThreadSpawn` проверяет наличие authoritative graph до создания child и fail-closed завершает spawn, если graph store недоступен. Fresh child до initial delivery хранится как `PendingActivation`: submission loop paused, queued task не исполняется, а все resume/target surfaces fail-closed. Live metadata path атомарно создаёт thread row и отсутствующий `PendingActivation` edge, не понижает уже существующий `Open`/`Closed` и при retry восстанавливает missing edge только как `PendingActivation`; historical/resumed backfill отдельно сохраняет compatibility `Open`. После успешного enqueue edge переводится в `Open`, registry/residency commit завершается до latch `Commit`, затем in-memory publication marker и client notification открывают thread. Loaded V2 thread с unpublished marker, `PendingActivation` или без authoritative edge исключается из `thread/list`/`thread/search`, отклоняется `thread/read` и не может обойти central `thread/resume` gate. Promotion/latch failure выполняет `Abort` и удаляет buffered submissions. Rollback сначала дренирует queued recorder `Persist`/`Flush` через `shutdown`, затем удаляет Pending edge только для действительно unmaterialized rollout; materialized rejected child сохраняет restart-durable `PendingActivation` quarantine, даже когда runtime, registry и residency уже освобождены, а shutdown/persistence error также оставляет Pending fail-closed. Self/root/foreign targets отклоняются до lifecycle events и операций над thread. Все mutation paths одного effective-V2 root tree сериализованы lifecycle gate; graph traversal отклоняет cycles и depth выше 256, а termination под gate имеет пятисекундный timeout без удаления неподтверждённого runtime/registry state. Native V1 сохраняет прежние best-effort graph persistence и lifecycle behavior без V2 coordinator/timeout.
- Projected V1 spawn/resume использует V2 depth semantics. Projected V1 `wait_agent` исполняет те же configured default/min/max timeout values, которые показывает его schema, принимает не более 64 raw target entries и устраняет дубликаты в first-seen порядке. V1-only режим сохраняет прежние defaults, clamp, ordering и duplicate semantics.
- Исторический persisted-V2 pathless child при явном V1 resume под V2 получает generated fallback path. Persisted path всегда имеет приоритет; первый backfill обновляет in-memory resumed history и durable SQLite thread metadata, поэтому дальнейшие resume сохраняют identity. Для уже canonical JSONL-only rollout metadata reader проецирует `StoredThread.agent_path` из persisted `SessionSource`, поэтому отсутствие SQLite не скрывает существующий path; новый pathless backfill при этом по-прежнему требует SQLite state metadata. Если обязательная persistence недоступна или запись не удалась, resumed thread останавливается, а операция fail-closed. Implicit residency reload через `send_input` не угадывает task segment и требует сначала выполнить явный resume.
- Cold resume effective-V2 ThreadSpawn-agent сначала восстанавливает сохранённые model/reasoning и graph-derived lineage, затем повторно разрешает сохранённое имя роли только через текущий trusted role catalog. Role-owned model/provider/reasoning/instructions/service tier применяются заново, после чего live approval policy, reviewer, полный permission state, workspace roots, `cwd` и base instructions восстанавливаются; явные app-server/session resume flags остаются выше role layer. Native V1 resume не получает новый role-reapply contract. Новые spawn без `agent_type` сохраняют роль `default`; historical full-history fork без persisted role не получает guessed default. Удалённая, невалидная или превышающая лимит роль приводит к контролируемому `InvalidRequest`, без silent fallback и без частично загруженного thread.
- Projected V1 не дублирует V2 model catalog: explicit model override остаётся доступен, а schema содержит фиксированное compatibility-сообщение. V2/projected role catalog строится только из уже materialized `Config.agent_roles` и built-ins, включая materialized role-locked model/reasoning/service-tier guidance, сохраняет user override/order и `default`, но ограничен 32 entries, 768 JSON-encoded bytes на entry и 2048 JSON-encoded bytes суммарно. V1-only сохраняет прежнюю форму role-description contract, но native V1, projected V1 и V2 применяют один immutable role snapshot, построенный при загрузке `Config`; существующий thread не перечитывает role TOML между turn-ами. Role file читается bounded stream-ом и ограничен 65536 bytes, discovery — 256 files, а `developer_instructions` — 8192 bytes до spawn/resume.
- Built-in Responses namespaces, `mcp` и `mcp__*` запрещены статически единым contract source. Core runtime дополнительно резервирует фактически активные namespace: `multi_agent_v1` при любом включённом multi-agent режиме и configured V2 namespace только при effective V2. Dynamic namespaces проверяются на `thread/start` и при восстановлении из rollout.
- Persisted V1 thread остаётся pinned к V1: включение V2 в новом config само по себе не активирует configured V2 namespace для этого thread. Активный V1 namespace `multi_agent_v1` при этом по-прежнему защищён от dynamic collision.

## Миграция и совместимость

Значение:

```toml
[features.multi_agent_v2]
tool_namespace = "multi_agent_v1"
```

остаётся синтаксически допустимым, чтобы старый config можно было загрузить и диагностировать без schema-level migration, но effective V2 `thread/start`/resume отклоняет его как collision с projected namespace. Для default `collaboration` значение необходимо удалить либо заменить незарезервированным именем, например `agents`. Runtime не выполняет silent remap. Новый config key и data migration не добавляются; `config.schema.json` отражает только статические запреты, а mode-dependent collision остаётся runtime contract.

Исторический experimental rollout с persisted `dynamicTools` в namespace, который теперь активен для thread (`multi_agent_v1` либо effective configured V2 namespace), также fail-fast отклоняется при cold resume. Безопасного автоматического merge/remap нет: создайте новый thread с незарезервированным namespace и явно перенесите нужный context/tool declarations. Immutable rollout автоматически не переписывается.

Graph status `pending_activation` является внутренним persisted-format extension без SQLite migration. Перед rollback/downgrade на binary, который не знает этот status и не выполняет status-aware resume authorization, дождитесь завершения pending cleanup либо удалите точно идентифицированный pending edge штатным current-runtime cleanup; старый runtime нельзя использовать для resume такого thread. Exactly-once replay initial task после crash уже зафиксированного `Open` не входит в этот contract: latch гарантирует отсутствие исполнения до activation, но не является durable submission outbox.

Если persisted sub-agent ссылается на удалённую custom role, восстановите declaration/role file перед resume либо выполните отдельную явную migration. Автоматически применять `default` вместо сохранённой роли небезопасно и не поддерживается.

Изменение custom role TOML не обновляет уже построенный `Config` и live thread. Чтобы новая версия роли вступила в силу, запустите новый config/thread startup path; повторное чтение role file внутри существующей session намеренно не поддерживается как защита от planning/spawn TOCTOU.

Role file с `developer_instructions` больше 8192 bytes больше не входит в trusted catalog. Сократите instructions до лимита до spawn/resume; loader выдаёт явную warning, а resume сохранённой роли fail-fast, если после валидации роль недоступна. Persisted identity с несовпадающими source/metadata parent, path или depth также требует явного исправления данных и не восстанавливается через fallback.

## Матрица распространения

| Поверхность | Source of truth и изменение | Основная проверка |
| --- | --- | --- |
| Tool planning/spec | `spec_plan.rs` регистрирует V2 и ровно пять V1 tools; `multi_agents_spec.rs` сохраняет отдельные schemas и projection policy | `spec_plan`, `multi_agents_spec` tests |
| Role discovery/context | Config loader bounded stream-ом материализует configured role snapshots; `agent/role/spawn_tool_spec.rs` строит catalog из snapshot и built-ins | role materialization/removal и boundary catalog tests |
| Spawn/completion | V1 handler генерирует internal task segment; `SessionSource`/`AgentControl` создают canonical path; session доставляет один V2 `FINAL_ANSWER` | handler interop test и `projected_multi_agent_v1_completion_uses_canonical_v2_path` |
| Graph lifecycle/persistence | Effective-V2 spawn требует authoritative graph до создания child; `AgentGraphStore`/`StateRuntime` владеют `PendingActivation`/`Open`/`Closed`; live metadata атомарно создаёт row + missing Pending edge, сохраняет promoted status, historical reconciliation создаёт compatibility Open | `v2_thread_spawn_without_authoritative_graph_fails_closed`; `live_insert_uses_pending_activation_while_backfill_uses_open`; `live_insert_rolls_back_thread_row_when_pending_edge_insert_fails`; thread-store live/historical metadata tests |
| Submission latch/publication | `SubmissionLoopActivation` блокирует queued input; `AgentControl` выполняет Open, registry/residency commit, `Commit`, publication marker и notification в фиксированном порядке | `spawn_publication_waits_for_open_activation`; `spawn_lifecycle_rollbacks_preserve_runtime_graph_and_registry_invariants` |
| Public thread projections | `ThreadManager` проверяет live marker и authoritative V2 edge; app-server применяет тот же loaded-thread guard к list/search/read, central resume gate повторно проверяет persisted status | `thread_resume_rejects_loaded_v2_agent_with_pending_activation` для Open, Pending и missing-edge состояний |
| Lifecycle authorization | Root-scoped registry авторизует live UUID, authoritative persisted graph — cold UUID; graphless spawn/target fail-closed; recorder shutdown дренирует queued persistence до materialization decision, materialized rollback сохраняет Pending quarantine, а runtime/registry/residency освобождаются только после подтверждённого shutdown | same-manager foreign-root resume, `discard_waits_for_queued_persist_before_pending_cleanup`, materialized late-cleanup и AgentControl tests |
| Close/reopen subtree | Только effective-V2 lifecycle coordinator и graph store сериализуют snapshot descendants, Closed/Open transitions и stale-metadata precedence; native V1 сохраняет compatibility best-effort behavior | native-V1 compatibility, transactional close/resume subtree, parent failure и graph-authoritative stale metadata tests |
| V2 inventory/residency | `AgentRegistry`/`V2Residency` владеют live identity/status/LRU; `list_agents`, explicit resume и unload используют одну metadata projection | list_agents status tests и explicit-resume/residency-slot tests |
| Wait/depth | Mode-gated depth checks и переданные `WaitAgentTimeoutOptions`; target bound/dedup | focused handler tests |
| Resume identity | `control/resume.rs` восстанавливает lineage; `thread-store/identity_projection.rs` согласует source/metadata; JSONL summary проецирует existing path из source; pathless V2 backfill требует durable SQLite metadata | stable two-resume, full-history fork, JSONL projection и no-state-store fail-closed tests |
| Resume role/security | `thread_manager::spawn_thread_with_source` повторно применяет current trusted role и восстанавливает runtime-owned security context | changed-role cold resume, removed-role fail-fast и helper tests |
| Config/schema | Общий predicate задаёт статические Responses/MCP запреты; runtime добавляет active multi-agent namespaces | config/schema/runtime tests и `just write-config-schema` |
| Dynamic tools | Shared validator применяется app-server input path и core session start/resume с effective version resolution | core/app-server start и cold-resume tests |
| App-server errors/overrides | Namespace/role/lifecycle resume diagnostics проецируются как JSON-RPC invalid request; explicit top-level и config-map model/provider/reasoning overrides остаются выше persisted role/metadata | app-server focused/full tests, включая config-map provider override |

## Намеренно не затронутые поверхности

- V2 handler schemas, V2 target resolution и TUI projection не меняются.
- V1-only public schemas и lifecycle behavior не меняются. Общие hardening-изменения — fail-fast запрет dynamic tool namespace, который конфликтует с фактически активным multi-agent namespace, и immutable bounded role snapshot вместо hot-read configured role TOML внутри существующей session.
- Protocol structs, app-server API structs, rollout format, SQLite schema, dependencies и lockfiles не меняются.
- Backfilled path использует существующие `SessionSource`, `ThreadMetadataPatch` и agent graph surfaces; новый persistence side channel не создаётся.

## Runtime delivery boundary

Локальный A/B до текущего hardening patch подтвердил реальный legacy spawn/wait через embedded app-server: child выполнил `pwd` с exit `0`. Тот же A/B обнаружил независимое operational ограничение: foreground fork CLI может подключиться к уже запущенному app-server daemon другой сборки, и тогда capabilities определяет daemon, а не новый TUI binary. CLI override в том запуске отключил reuse и переключил путь на embedded runtime. Отсутствие DirectModelOnly tool в `ALL_TOOLS` само по себе не доказывает отсутствие model-visible tool.

Это не исправляется данной feature: после сборки необходимо обновить managed runtime targets штатным install script и перезапустить/recreate существующий daemon перед live acceptance. Исторический A/B не считается доказательством финального current snapshot; актуальные результаты фиксируются в project verification.

## Журнал

- 2026-06-23: donor activation patch добавил V1 namespace рядом с V2.
- 2026-07-10: порт адаптирован к default `collaboration`, bounded model-visible metadata и current config contract.
- 2026-07-10: production audit расширил реализацию canonical path, authorization, wait/depth consistency, stable path backfill, trusted role reconstruction, shared namespace/schema/dynamic validation и focused lifecycle tests.

Подробный дизайн и evidence: `docs/fork/projects/tool-multi-agent-v1-add-to-v2/`.
