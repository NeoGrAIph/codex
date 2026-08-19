# Проектирование

## Архитектурное решение

Feature является additive projection существующего native V1 runtime в eligible V2 turn и добавляет общий permanent-close entrypoint в обе model-visible families. Она не вводит третий runtime и не эмулирует V1 поверх V2. Current upstream owners сохраняются: planner владеет publication, AgentControl — spawn/lifecycle, ThreadManager/Session — runtime selection, ThreadStore/LiveThread — history, а app-server/TUI — существующими projections.

## Архитектурный baseline

Fork delta спроектирован относительно [нормализованной upstream multi-agent architecture map](../../research/multi-agents/architecture.md) для `rust-v0.145.0^{}` / `25af12f7e61572b0bc18ddb1008be543b91519b0`. Native owners из этой карты остаются source of truth: `TurnContext` и planner определяют публикацию tools, `AgentControl` и `ThreadManager` владеют lifecycle/runtime selection, `Session` и `SessionMeta` — actual/persisted runtime, ThreadStore/graph — history и lineage. Feature добавляет private exact-V1 intent, V1 projection и bound-instance guards в эти же owners, не создавая fork-only registry, persisted selector или client wire.

## Tool planning

Planner сначала строит unchanged V2 family, затем вычисляет `ProjectedV1Availability` для всего namespace. Available projection использует current V1 handler/spec factories, explicit projected invocation mode и V2 base exposure. Native V1 по-прежнему может быть deferred через tool search; projection всегда direct или direct-model-only и проходит current code-mode transforms независимо от V2 namespace.

Namespace рассматривается как единица ownership. Configured V2, dynamic или prebuilt runtime/MCP owner подавляет projection. Если projection доступна, lower-priority extension tools того же namespace пропускаются. Registry dedupe не используется как policy и не должен создавать смешанную family.

## Exact runtime intent

Private `MultiAgentRuntimeIntent` содержит `Inherit`, `ExactV1Spawn` и `ExactV1Resume`.

- `Inherit` выполняет current config/history/lineage resolution без изменений.
- `ExactV1Spawn` допустим только для pathless `ThreadSpawn`, `UserInput`, fresh или full-history fork.
- `ExactV1Resume` допустим только для Resumed history, разрешённой как V1; running target повторно проверяется по actual version.

AgentControl выбирает exact version до capacity/residency. `SpawnCapacity::V1Limited` и `SpawnCapacity::CatalogOnly` разделяют V1 execution ownership и V2 logical catalog: общий registry остаётся identity/listing source, но только V1 ThreadId владеет отдельным `agents.max_threads` slot. V2 catalog/residency не расходует этот бюджет, поэтому смешанная session может одновременно занимать доступные V1 и V2 slots. Это явная fork-policy, а не восстановление единого upstream session cap. ThreadManager повторно проверяет форму creation/resume и передаёт intent в `SessionSpawnArgs`. Session применяет exact V1 раньше config override только для проверенного intent, после чего existing Session state и `SessionMeta` фиксируют V1 для preview/turn/resume. Intent нигде не сериализуется.

Generic resume автоматически выбирает `ExactV1Resume` только для persisted pathless `ThreadSpawn`, разрешённого как V1. Root и остальные sessions сохраняют config-authoritative 0.145 behavior.

## Spawn, fork и authority

Projected V1 spawn строит pathless source и V1 UUID result через existing handler. Child использует common V1 reservation, depth limit и completion watcher. V2 residency, mailbox, path/activity и execution accounting не запускаются.

Fresh spawn сначала получает requested model/role/service tier по current `[agents]` rules, затем восстанавливает authority активного turn: approval policy, reviewer, полный permission snapshot с active identity и roots, primary compatibility cwd и exact environment selections. Cold resume доверяет child-owned persisted history и current resume config; сохранённое имя role остаётся metadata и отсутствие либо изменение mutable role file не блокирует и не перенастраивает старую session. Exec policy наследуется только через current layer-stack compatibility rule. Общий `apply_role_to_config` сохраняет `ignore_user_and_project_exec_policy_rules` для projected и ordinary role consumers; остальные native V1/V2 overlay semantics не получают projected-only authority rewrite.

Fork path загружает current model context по `ThreadHistoryMode`. После current truncation/event filtering V1 sanitizer удаляет exact V2 usage hints и structural mode fragments, нормализует retained turn context и сохраняет semantic/model-context invariants. Paginated destination mode, inherited prefix и ordinal boundary остаются под контролем LiveThread.

## Lifecycle и resume

V1 handler instance знает, является ли вызов Native или ProjectedFromV2. Projected target resolution выполняется до любого observable effect и возвращает `ProjectedV1LifecycleTarget`, а не только результат проверки.

1. Live target проверяется по actual Session version и связывается с конкретным `Arc<CodexThread>` в `ValidatedV1Thread`.
2. При отсутствии live target metadata читается без history.
3. Legacy загружает rollout history, Paginated — latest model context.
4. Version разрешается только из target history без inheritance caller runtime.
5. Missing target возвращается native handler-у; V1-only projected send/interrupt/wait/resume отклоняют unreadable/unresolved/V2/Disabled до effects.

Projected send/interrupt/status subscription используют один bound V1 target на весь вызов. При закрытии watch channel wait читает финальный status из того же instance. Для permanent close отдельный `ResolvedCloseTarget` допускает spawned V1/V2 и не ослабляет V1-only contract остальных lifecycle handlers. Persisted spawned provenance вычисляется тем же compatibility rule, что resume: parent из `SessionSource::ThreadSpawn` имеет fallback на stored `parent_thread_id`, поэтому legacy rows без новой колонки не становятся roots. Projected V1 и V2 `close_agent` используют общий UUID/canonical-path resolver и единый cross-runtime close traversal без reload unloaded target; native V1 handler сохраняет исходный UUID-only contract. Mixed-runtime subtree образуется union logical catalog descendants, live descendants и persisted `Open` graph descendants. Shutdown удаляет manager entry только через `remove_thread_if_same`; registry/residency cleanup выполняется лишь для проверенного instance/ThreadId, late replacement остаётся нетронутым, а caller получает controlled race diagnostic.

Spawn удерживает lifecycle guard непосредственного parent до публикации child и повторно проверяет, что parent остаётся live. Поэтому close snapshot не может завершиться, после чего уже ожидавший spawn опубликует новый orphan child. Guards остаются keyed per ThreadId и не превращают последовательный subtree close в атомарную транзакцию.

Projected cold resume повторяет проверку на AgentControl, ThreadManager и running-session boundary. Keyed lifecycle guard сериализует effects одного `ThreadId`, а bound `Arc<CodexThread>` не позволяет операции перейти на replacement runtime с тем же UUID. Root guard удерживается во время доступного обхода, но graph/read и descendant-resume остаются native best-effort: без graph store возобновляется только target, ошибки отдельных edges/descendants логируются и пропускаются, а уже загруженные instances не откатываются. Per-ID guards не являются subtree transaction.

`ThreadLifecycleLocks` — intentional identity-scoped hardening внутри существующего `ThreadManager`: weak keyed locks сериализуют resumed-history entrypoints в общем manager path, включая projected resume/close и current V2 ensure-load, но не становятся persisted lifecycle cache, новым source of truth или изменением runtime selection. Они не дают cross-ID ordering и не усиливают best-effort graph contract до атомарности.

Resume graph traversal следует только доступным `Open` edges и загружает только V1 descendants. `visited` set отклоняет повреждённый cyclic graph controlled error; instances, загруженные до traversal failure, могут остаться live в соответствии с best-effort policy. Edge-derived parent/depth имеет приоритет над stale stored `parent_thread_id` и source, а stored nickname/role/path merge сохраняет current upstream order. Completion watcher, handler result и queue несут конкретный `Arc<CodexThread>`, поэтому повторно использованный UUID не становится continuation target. Cross-runtime close сначала валидирует persisted `Open` graph read-only, чтобы обратное edge не исчезло из проверки после первой записи `Closed`, затем обходит mixed V1/V2 descendants из catalog/live/graph union и для каждого bound target пытается записать `Closed` до shutdown. Ошибка live V1 graph write остаётся warning-only, а live V2 и persisted-only write failure распространяются. Close последовательно обрабатывает descendants, прекращается на первой распространяемой lookup/replacement/shutdown/cleanup error и не откатывает уже завершённые effects; в отличие от resume, он не пропускает ошибочный descendant для продолжения обхода. Persisted preflight не является atomic snapshot динамического union: повреждённое cross-source duplicate ownership может обнаружиться во время дальнейшего traversal после более ранних effects, что входит в явно non-transactional fail-fast contract.

UUID spawned V1 agent не является credential или subtree ownership proof. Cross-subtree V1 lifecycle остаётся разрешённым при знании UUID, но root sessions не считаются agents и permanent close их отклоняет. Feature отделяет V1 от V2, но не вводит новую authorization model.

Explicit UUID resume остаётся caller-mediated native V1 operation: pathless source строится от вызывающей session и может обновить immediate parent edge. Persisted target version остаётся target-owned, но parent lineage не является authorization boundary.

## Public и client boundaries

Existing `SessionMeta`, `SessionSource`, graph edge, canonical collaboration items и `Thread.canAcceptDirectInput` достаточны для persistence и clients. Actual V1 child допускает direct input; V2 child остаётся parent-owned. Stored `thread/list` row в current 0.145 возвращает `canAcceptDirectInput: null`, тогда как live/resumed `thread/read` возвращает `true`; feature не дублирует capability в store. После cold resume model request использует existing persisted `session_id` как prompt cache key. TUI уже отображает UUID без path/nickname metadata.

Не меняются config schema, app-server wire, protocol enum, persistence format, generated artifacts, model catalog, role format, Guardian/review/realtime/external-agent paths и public paginated fork policy.
