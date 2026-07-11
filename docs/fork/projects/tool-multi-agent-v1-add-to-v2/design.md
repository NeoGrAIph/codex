# Дизайн

## Нативный baseline

Baseline: `docs/fork/research/multi-agents/architecture.md`. `multi_agent_v2` остаётся источником истины для path-based runtime, mailbox delivery, thread lineage, residency и permissions. `multi_agent_v1` является compatibility projection с собственным wire schema, но не создаёт parallel registry, alias handler или отдельную persistence model.

## Planning и schema separation

`spec_plan::add_collaboration_tools` при effective V2 регистрирует нативные V2 tools в `collaboration` либо configured namespace и отдельно пять V1 handlers в `multi_agent_v1`. V1-only path сохраняет прежнее direct/deferred exposure.

`multi_agents_spec.rs` остаётся владельцем обеих schemas. Projected V1 использует `SpawnAgentModelCatalogDisplay::OmitForProjection`: explicit model override поддерживается, но список моделей не копируется из native V2 description. `agent_type` сохраняет role discovery, потому что строковое имя роли нельзя корректно выбрать без model-visible catalog.

Role catalog строится в `agent/role/spawn_tool_spec.rs`. Project/user declarations поступают только из уже materialized effective `Config.agent_roles`, built-ins — из `built_in::configs()`; planning не перечитывает project role files и поэтому не получает TOCTOU между turn-ами. Materialization добавляет role-locked model/reasoning/service-tier guidance. User declaration выигрывает при совпадении имени. Output V2/projected V1 ограничен 32 entries, 768 JSON-encoded bytes на entry и 2048 JSON-encoded bytes суммарно, всегда сохраняет `default` и явно сообщает об усечении/пропущенных roles. V1-only сохраняет прежнюю форму description builder, но получает configured role descriptions и settings из того же immutable Config snapshot. Trusted loader ограничивает один role file 65536 bytes, читает его bounded stream-ом, ограничивает discovery 256 files и `developer_instructions` 8192 bytes.

## Canonical identity adapter

Публичный V1 spawn не принимает `task_name` и возвращает UUID. Под V2 handler генерирует только internal segment `agent_<uuid-simple>` и передаёт его существующему `thread_spawn_source`. Builder присоединяет segment к canonical parent path; `SessionSource::SubAgent(ThreadSpawn.agent_path)` владеет lineage, `AgentRegistry` — live uniqueness, `AgentMetadata`, rollout и thread store остаются projections.

Следствия:

- V1 output не раскрывает новый wire field.
- V2 `list_agents`/path resolution видят того же child.
- Terminal completion проходит через существующий V2 `InterAgentCommunication` path; legacy completion watcher под V2 не включается и duplicate delivery не возникает.
- V1 `send_input` остаётся raw `UserInput` transport даже под V2: это необходимая compatibility-граница для rich items, которые текущий `InterAgentCommunication` не представляет без потерь.
- Generated path не обязан совпадать с UUID `agent_id`; это internal V2 identity, тогда как UUID остаётся V1 locator.

## Authorization UUID targets

UUID нельзя считать capability. Для live V1 lifecycle calls под V2 target должен присутствовать в root-scoped `AgentRegistry`, разделяемом только root и его descendants. Для cold target ownership доказывается подъёмом по incoming edges `AgentGraphStore` до зарегистрированного root, с cycle detection и hard cap 256 levels; результат содержит authoritative immediate parent и depth и передаётся в resume. Effective-V2 `ThreadSpawn` проверяет наличие `AgentGraphStore` до создания runtime, поэтому конфигурация без authoritative graph fail-closed завершается без child, registry reservation или residency leak. Переданные target-ом `parent_thread_id` и `SessionSource` не являются trust anchor. Любое отсутствие authoritative lineage, foreign root, self или root target приводит к controlled rejection до emission lifecycle events и до thread operations.

Graph lifecycle является частью effective-V2 транзакции, а не best-effort telemetry. Fresh spawn сначала получает durable `PendingActivation`; internal submission loop ждёт one-shot decision и не читает queued input. Первая live metadata projection атомарно создаёт SQLite thread row и отсутствующий `PendingActivation` edge; `ON CONFLICT DO NOTHING` сохраняет уже promoted `Open`/`Closed`, а retry при missing edge восстанавливает его только как `PendingActivation`. Historical/resumed metadata reconciliation использует отдельный compatibility path и создаёт отсутствующий legacy edge как `Open`. Только после успешного enqueue initial task edge переводится в `Open`, registry/residency commit завершается, loop получает `Commit`, in-memory publication marker открывает public thread access, а client notification публикует thread. Ошибка enqueue или promotion отправляет `Abort`: buffered submissions удаляются до standard session teardown, поэтому model/tool side effects до durable `Open` невозможны. Rollback recorder сначала завершает queued `Persist`/`Flush` через `shutdown`, после чего unmaterialized Pending edge может быть удалён; если rollout уже materialized, edge остаётся `PendingActivation` как restart-durable quarantine, даже после подтверждённого shutdown и освобождения runtime/registry/residency. Ошибка shutdown или persistence также не разрешает удалять Pending marker. Resume сразу восстанавливает уже существующий edge как `Open`, но central ThreadManager gate и UUID authorization атомарно читают `{parent, status}` и отклоняют `PendingActivation` на любом resume surface. Loaded V2 thread с unpublished marker, `PendingActivation` или отсутствующим authoritative edge скрыт из `thread/list` и `thread/search`, отклоняется `thread/read` и не может обойти central resume gate; cold historical listing без live runtime сохраняет compatibility semantics. Explicit close под root-scoped lifecycle gate сначала получает cycle/depth-validated snapshot live descendants, затем записывает `Closed` и выключает именно этот snapshot. Ожидания termination под gate ограничены пятью секундами: timeout освобождает gate через RAII и не удаляет runtime/registry state, завершение которого не подтверждено.

Lifecycle coordinator различает три operational состояния target-а:

| Состояние | Инвариант | Разрешённые операции |
| --- | --- | --- |
| Pending activation | Persisted edge уже существует, submission loop paused, initial task может быть только queued | Никакие target operations и resume не разрешены; только durable promotion в `Open` с `Commit` либо `Abort` и cleanup |
| Active | Для target нет pending cleanup; runtime/registry и persisted graph согласованы либо target явно авторизован как cold persisted descendant | Обычные spawn, delivery, interrupt, resume и close после соответствующей authorization |
| Pending cleanup | Coordinator хранит cleanup policy и единственного background owner; rejected spawn остаётся зарегистрированным и quarantined, а его durable edge — `PendingActivation`, пока cleanup не завершён | Delivery, interrupt, resume, nested spawn и explicit close fail-closed; owner делает три быстрые попытки, затем продолжает редкие retries без удержания lifecycle gate и снимает marker только после graph/runtime/registry cleanup |

Ownerless `RepairRequired` состояния нет: исчерпание быстрых retries не снимает process-local quarantine. После восстановления persistence тот же owner завершает cleanup; если `ThreadManagerState` уже уничтожен, persisted `PendingActivation` остаётся restart-durable fail-closed marker. Materialized rejected rollout после завершённого runtime cleanup намеренно сохраняет такой marker и не считается безопасным для resume; unmaterialized edge считается удалённым только после фактического `remove`. SQLite migration не нужна, потому что существующая status-column хранит text без `CHECK`, но это internal persisted-format extension: текущий runtime понимает `pending_activation`, `open`, `closed`, а downgrade до binary, который не проверяет activation status, запрещён до удаления pending edges. Latch не является durable submission outbox и не обещает exactly-once replay при process crash после `Open`, но до durable activation queued task исполниться не может.

Mode gate принципиален для UUID authorization, transactional activation, lifecycle coordinator с пятисекундным termination timeout, V2 depth/residency и projected wait: эти изменения применяются только к newly exposed V1-under-V2 surface и native V2. Native V1 сохраняет HEAD-like immediate publication, best-effort graph persistence и прежние send/interrupt/close/shutdown semantics без V2 coordinator. Общие loader и dynamic-namespace hardening явно описаны отдельно и применяются также к native V1.

## Depth и wait contract

Native V2 не использует legacy depth cap, поэтому projected V1 spawn/resume пропускает только legacy `agent_max_depth` gate при `turn.multi_agent_version == V2`; execution/concurrency/residency limits остаются нативными.

Planning передаёт V2 `WaitAgentTimeoutOptions` в projected V1 handler, поэтому runtime использует именно эти default/min/max values. Положительные значения сохраняют V1 clamp semantics; zero допустим только при configured minimum zero; отрицательное значение отклоняется. Только projected raw target list ограничен 64 entries до parsing, дубликаты устраняются с сохранением порядка; V1-only parser намеренно сохраняет прежние duplicate/ordering semantics.

## Resume identity и metadata backfill

`AgentControl::resume_single_agent_from_rollout` в отдельном `control/resume.rs` выбирает persisted identity и использует caller path только как источник task segment для historical persisted-V2 pathless child. Fallback создаётся V1 resume handler только под V2; persisted V1 thread остаётся pinned к V1.

`thread-store/identity_projection.rs` перед каждым resume объединяет отдельные store-owned parent/path/role/nickname с вложенным `SessionSource::ThreadSpawn`, проверяет matching `SessionMeta`, self-parent, depth, path size и расхождение source/metadata, затем синхронизирует `StoredThread.source` и history одним canonical source. JSONL-only summary reader также проецирует `StoredThread.agent_path` из того же persisted `SessionSource`, чтобы metadata-only и history readers не расходились при отключённой SQLite state DB. Persisted depth ограничен 256 levels, path — 4096 bytes. Для V2 `control/resume.rs` дополнительно сверяет immediate parent path и path depth с persisted parent thread. Corrupt lineage fail-closed до открытия runtime.

При первом backfill обновляются reconstructed `SessionMeta` в resumed in-memory history и существующие `ThreadMetadataPatch.source/agent_path`. Identity-only backfill требует доступную SQLite state metadata: без неё операция не может гарантировать durable identity и завершается контролируемой ошибкой. Если metadata persistence не удалась, resumed thread сначала пытается штатно остановиться и удаляется из live registry только после подтверждённого shutdown; если shutdown не подтверждён, thread остаётся зарегистрированным и учитывается в concurrency. Implicit residency load не располагает task segment и отклоняет pathless source с инструкцией сначала выполнить явный resume. Новое protocol field или schema migration не требуется.

Residency-unloaded V2 target сохраняет существующие `AgentRegistry` metadata, canonical path и terminal status cache. Explicit resume такого target-а резервирует только residency slot, переиспользует registry identity без нового `SpawnReservation` и очищает cached status лишь после успешного graph persistence; это исключает duplicate path/count и ложный live status при неуспешной реактивации.

## Trusted role reconstruction

Configured custom role TOML полностью читается, валидируется и материализуется один раз при построении `Config`; этот snapshot используют native V1, projected V1 и native V2 spawn/resume paths. Существующий live thread и уже построенный `Config` не перечитывают role file между turn-ами. Изменение role TOML вступает в силу только после нового config/thread startup path, который заново построит snapshot. Это намеренное общее security hardening против metadata/read и planning/spawn TOCTOU; wire schema и precedence role layer не меняются, но недокументированная возможность hot-read файла в существующей native V1 session больше не поддерживается.

Rollout хранит имя роли, а не effective serialized `Config`. Центральная точка `ThreadManagerState::spawn_thread_with_source` после reconstruction `SessionSource` вызывает `reapply_role_to_resumed_agent_config` только для persisted effective V2 ThreadSpawn. Поэтому один path покрывает projected-V1 explicit resume, V2 residency reload, direct rollout resume и app-server cold resume, но не изменяет native V1 resume.

Порядок reconstruction:

1. Взять current caller-derived resume config и stored thread identity.
2. Восстановить stored model/reasoning и persisted lineage, не переписывая их caller fallback-ом.
3. Сохранить live approval policy, approvals reviewer, полный permission state/profile/custom profiles, workspace roots, `cwd` и base instructions.
4. Разрешить persisted role name через current trusted configured/built-in catalog. Новые spawn без `agent_type` сохраняют `default`; historical full-history fork без persisted role остаётся без role reapply.
5. Применить role-owned model/provider/reasoning/developer instructions/service tier и разрешённые config settings; для app-server/direct resume восстановить явно переданные session flags поверх role layer.
6. Восстановить live runtime-owned values.
7. Продолжить нативное inheritance environments/exec policy и session spawn.

Неизвестная, невалидная или превышающая лимит persisted role отклоняет resume как `InvalidRequest`. Persisted config path или permissions не считаются trusted. Explicit per-spawn model/reasoning без role восстанавливаются из `StoredThread`, а current trusted named role затем может намеренно переопределить role-owned значения.

## Reserved namespace contract

`codex-features` экспортирует единый predicate для статически зарезервированных built-in Responses namespaces, `mcp` и `mcp__*`. Его используют core config loader и shared dynamic-tool validator; generated schema строит эквивалентный `not` pattern для existing optional `features.multi_agent_v2.tool_namespace`.

App-server сохраняет прежний validation order и отдельное native error wording для `mcp` namespaces. Core session повторяет validation после dynamic tools resolution из request или rollout и дополняет статический набор фактически активными namespace: `multi_agent_v1` для V1/V2 и configured V2 namespace только для effective V2. Для fresh thread effective version вычисляется тем же порядком, что на первом turn: inherited/persisted version, model metadata, feature config. Поэтому V2 collision блокируется уже на thread start, а persisted V1 thread не получает ложный запрет от неактивного configured V2 namespace. App-server переводит только новые namespace/role resume diagnostics в JSON-RPC invalid request; остальные resume errors сохраняют прежний internal-error mapping.

## Compatibility и failure policy

- Public V1/V2 tool schemas не смешиваются.
- Existing V1-only и native V2 public schemas сохраняются. Native V1 lifecycle остаётся best-effort и не получает transactional V2 latch/coordinator/termination timeout; native V2 и projected V1-under-V2 используют один transactional path. Общие intentional hardening-изменения: fail-fast защита dynamic namespace от фактически активного built-in surface и immutable bounded snapshot configured role files вместо повторного чтения файла существующей session.
- Silent fallback отсутствует для namespace collision, foreign UUID, unstable path persistence и unavailable role.
- Protocol/TUI/API shapes, rollout format, SQLite schema, dependencies и lockfiles не меняются.
- Foreground CLI и reused daemon могут быть разными builds; deployment acceptance обязана проверять executable/build identity каждого parsing/runtime process отдельно.
