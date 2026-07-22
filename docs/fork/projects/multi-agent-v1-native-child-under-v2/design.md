# Проектирование

## Архитектурное решение

Feature является additive projection существующего native V1 runtime в eligible V2 turn. Она не вводит третий runtime и не эмулирует V1 поверх V2. Current upstream owners сохраняются: planner владеет publication, AgentControl — spawn/lifecycle, ThreadManager/Session — runtime selection, ThreadStore/LiveThread — history, а app-server/TUI — существующими projections.

## Tool planning

Planner сначала строит unchanged V2 family, затем вычисляет `ProjectedV1Availability` для всего namespace. Available projection использует current V1 handler/spec factories, explicit projected invocation mode и V2 base exposure. Native V1 по-прежнему может быть deferred через tool search; projection всегда direct или direct-model-only и проходит current code-mode transforms независимо от V2 namespace.

Namespace рассматривается как единица ownership. Configured V2, dynamic или prebuilt runtime/MCP owner подавляет projection. Если projection доступна, lower-priority extension tools того же namespace пропускаются. Registry dedupe не используется как policy и не должен создавать смешанную family.

## Exact runtime intent

Private `MultiAgentRuntimeIntent` содержит `Inherit`, `ExactV1Spawn` и `ExactV1Resume`.

- `Inherit` выполняет current config/history/lineage resolution без изменений.
- `ExactV1Spawn` допустим только для pathless `ThreadSpawn`, `UserInput`, fresh или full-history fork.
- `ExactV1Resume` допустим только для Resumed history, разрешённой как V1; running target повторно проверяется по actual version.

AgentControl выбирает exact version до capacity/residency. ThreadManager повторно проверяет форму creation/resume и передаёт intent в `SessionSpawnArgs`. Session применяет exact V1 раньше config override только для проверенного intent, после чего existing Session state и `SessionMeta` фиксируют V1 для preview/turn/resume. Intent нигде не сериализуется.

Generic resume автоматически выбирает `ExactV1Resume` только для persisted pathless `ThreadSpawn`, разрешённого как V1. Root и остальные sessions сохраняют config-authoritative 0.145 behavior.

## Spawn, fork и authority

Projected V1 spawn строит pathless source и V1 UUID result через existing handler. Child использует common V1 reservation, depth limit и completion watcher. V2 residency, mailbox, path/activity и execution accounting не запускаются.

Config сначала получает requested model/role/service tier по current `[agents]` rules. После role overlay fresh spawn и projected cold resume восстанавливают authority активного turn: approval policy, reviewer, полный permission snapshot с active identity и roots, primary compatibility cwd и exact environment selections. Exec policy наследуется только через current layer-stack compatibility rule. Ordinary native V1 и V2 paths сохраняют upstream overlay и не получают projected-only authority rewrite.

Fork path загружает current model context по `ThreadHistoryMode`. После current truncation/event filtering V1 sanitizer удаляет exact V2 usage hints и structural mode fragments, нормализует retained turn context и сохраняет semantic/model-context invariants. Paginated destination mode, inherited prefix и ordinal boundary остаются под контролем LiveThread.

## Lifecycle и resume

V1 handler instance знает, является ли вызов Native или ProjectedFromV2. Projected target resolution выполняется до любого observable effect и возвращает `ProjectedV1LifecycleTarget`, а не только результат проверки.

1. Live target проверяется по actual Session version и связывается с конкретным `Arc<CodexThread>` в `ValidatedV1Thread`.
2. При отсутствии live target metadata читается без history.
3. Legacy загружает rollout history, Paginated — latest model context.
4. Version разрешается только из target history без inheritance caller runtime.
5. Missing target возвращается native handler-у; unreadable/unresolved/V2/Disabled завершается controlled error.

Projected send/interrupt/status subscription используют один bound target на весь вызов. При закрытии watch channel wait читает финальный status из того же instance. Close связывает root и только V1 descendants, останавливается на V2/Disabled branch, отправляет shutdown bound instances и удаляет manager entry только через `remove_thread_if_same`. Registry/residency cleanup выполняется лишь после успешного `Arc::ptr_eq`; late replacement остаётся нетронутым, а caller получает controlled race diagnostic. Persisted close не выполняет UUID-based shutdown: он сначала подтверждает отсутствие позднего live occupant и затем идемпотентно закрывает persisted edge. Native ID-based lifecycle сохраняется без изменений.

Projected cold resume повторяет проверку на AgentControl, ThreadManager и running-session boundary. Keyed lifecycle guard сериализует операции по `ThreadId`: root guard удерживается от финальной проверки через регистрацию root, весь обход `Open` edges, загрузку V1 descendants и graph upserts. Каждый descendant effect выполняется под собственным guard при сохранённом root guard; краткий `threads` lock берётся только после lifecycle guard и не удерживается при ожидании другого lifecycle lock. Close использует тот же порядок, сначала фиксирует durable `Closed` для подтверждённого root и только затем обрабатывает bound V1 descendants. Это исключает состояние `close success` одновременно с возвращённым уже удалённым root или вновь загруженным ребёнком.

Graph traversal следует только `Open` edges и загружает только V1 descendants. `visited` set отклоняет повреждённый cyclic graph controlled error до повторного захвата lifecycle guard. Edge-derived parent/depth имеет приоритет над stale stored `parent_thread_id` и source, а stored nickname/role/path merge сохраняет current upstream order. Completion watcher, handler result и queue несут конкретный `Arc<CodexThread>`, поэтому повторно использованный UUID не становится continuation target.

UUID не является credential или ownership proof. Cross-root/cross-subtree V1 lifecycle остаётся разрешённым при знании UUID; feature отделяет V1 от V2, но не вводит authorization model.

## Public и client boundaries

Existing `SessionMeta`, `SessionSource`, graph edge, canonical collaboration items и `Thread.canAcceptDirectInput` достаточны для persistence и clients. Actual V1 child допускает direct input; V2 child остаётся parent-owned. Stored `thread/list` row в current 0.145 возвращает `canAcceptDirectInput: null`, тогда как live/resumed `thread/read` возвращает `true`; feature не дублирует capability в store. После cold resume model request использует existing persisted `session_id` как prompt cache key. TUI уже отображает UUID без path/nickname metadata.

Не меняются config schema, app-server wire, protocol enum, persistence format, generated artifacts, model catalog, role format, Guardian/review/realtime/external-agent paths и public paginated fork policy.
