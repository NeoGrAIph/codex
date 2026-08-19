# Нативные `multi_agent_v1` agents в effective V2 threads

## Паспорт возможности

- Кодовое имя: `multi-agent-v1-native-child-under-v2`.
- Статус: базовая реализация после workflow rollback `10713a4a4040` повторно перенесена и зафиксирована в `fork/145` коммитом `39f346c47988`; текущий cross-runtime capacity/close follow-up остаётся незакоммиченным, а его focused tests, scoped lint/format, независимые audits и recorded stripped release build зафиксированы в verification ledger.
- Цель: предоставить effective-V2 model дополнительный namespace `multi_agent_v1`, создающий настоящих native V1 agents, и общий permanent-close contract между V1/V2 surfaces без смешивания остальных runtime semantics.
- Scope in: planner projection, exact V1 spawn/resume, Legacy/Paginated fork sanitation, runtime authority, cross-surface identity resolution и close, раздельный V1 capacity/V2 catalog accounting, restart persistence и существующие app-server/TUI projections.
- Scope out: новая V3 family, V2 path/mailbox/activity semantics для V1 child, новый wire identity type, public paginated fork, protocol/config/persistence migration, dependency/model/role catalog changes, binary install и app-server restart.
- Upstream baseline: `rust-v0.145.0^{}` / `25af12f7e61572b0bc18ddb1008be543b91519b0`.
- Fork baseline: `72c43db8d4a41ea1a23dc4b664cdbec52e85f683`.
- Исторический feature provenance: `2f2e7e294975881c1c83e3c46fba3a0096481890` на `fork/144.4`.

## Пользовательский контракт

Eligible effective-V2 turn получает две независимые model-visible families:

- configured V2 namespace с current V2 functions и `close_agent` для permanent close в дополнение к non-terminal `interrupt_agent`;
- `multi_agent_v1` с `spawn_agent`, `send_input`, `resume_agent`, `wait_agent` и `close_agent`.

Projection доступна, когда effective runtime равен V2, collaboration разрешена, provider поддерживает namespace tools, следующий V1 depth допустим и namespace не занят configured V2, dynamic или prebuilt runtime owner. Projected family использует V2 base exposure (`Direct` или `DirectModelOnly`), но не native V1 deferred/tool-search policy.

Schema projected V1 строится current native-V1 factories, кроме target description `close_agent`, явно разрешающего UUID и canonical task name из обеих spawn surfaces. `spawn_agent` сохраняет V1 fields/result и current `[agents]` role/model/reasoning rules. V2-only `task_name`, `fork_turns` и `list_agents` в V1 family не добавляются; V2 family получает только permanent `close_agent`, не меняя schemas существующих V2 tools.

## Runtime и persistence

Projected `spawn_agent` создаёт pathless UUID child с actual/persisted `MultiAgentVersion::V1`. Parent остаётся V2. Child использует V1 depth/capacity, completion watcher и UUID lifecycle, но не V2 `AgentPath`, residency, mailbox или `SubAgentActivity`.

`AgentRegistry` хранит общий logical catalog, но capacity reservation типизирована. `multi_agent_v1` имеет отдельный бюджет `agents.max_threads`: только V1 spawn/resume занимает его слот, тогда как V2 catalog entries учитываются отдельными upstream residency/execution limiters. V2 agent любого lifecycle status не блокирует V1 spawn, поэтому смешанная session может одновременно использовать доступный V1 slot и доступную V2 residency. Это явный fork product contract; он не трактует `agents.max_threads` как общий лимит V1+V2. Release идемпотентен и уменьшает V1 counter только для ThreadId, который действительно владел V1 reservation.

Fresh и full-history fork поддерживаются для Legacy и Paginated parent history. Fork sanitation удаляет только V2 runtime fragments, сохраняя semantic history, compaction, `WorldState`, dynamic tools, selected capability roots и paginated boundaries. Arbitrary untagged developer text не удаляется эвристически.

Existing `SessionMeta`, `SessionSource`, rollout history и graph edges остаются единственными durable contracts. После restart pathless V1 child восстанавливается как V1 даже при V2-config parent; для legacy rows spawned provenance восстанавливается из `SessionSource::ThreadSpawn`, даже если более новая колонка `parent_thread_id` отсутствует. Новый persisted selector или migration не вводится.

## Authority и lifecycle

Fresh и cold-resumed child получают authority активного вызывающего turn: approval/reviewer, named или managed permission profile с provenance и roots, точные environment selections/cwd/workspace roots и допустимый inherited exec policy. Fresh spawn применяет выбранную role до восстановления runtime authority, поэтому role не может расширить её. Cold resume доверяет child-owned persisted history и current resume config: persisted role identity остаётся metadata, а удалённый или изменённый mutable role file не блокирует и не перенастраивает старую session. Полный active-turn authority overlay ограничен projected exact-V1 path. Общий role reload для projected и ordinary native consumers дополнительно сохраняет `ignore_user_and_project_exec_policy_rules`; остальные config-authoritative role semantics native V1/V2 не меняются.

Projected V1 send/interrupt/wait/resume продолжают требовать actual/persisted V1 до events, subscriptions, reload или mutation. Permanent close намеренно является cross-runtime exception: projected V1 и V2 `close_agent` принимают UUID либо canonical task name, разрешают actual/persisted V1/V2 и dispatch-ят в runtime-native shutdown path. Live validation возвращает capability на конкретный `Arc<CodexThread>`, поэтому повторно использованный UUID не перенаправляет effect и conditional cleanup не удаляет replacement. Disabled, unresolved runtime и повреждённая history отклоняются controlled error; missing target сохраняет controlled not-found.

Точный текст внутренних lifecycle errors не является стабильным client contract: контрактом являются отсутствие side effects до проверки target, сохранение native not-found для отсутствующего UUID, controlled failure для Disabled/unresolved/cyclic/root state и V2 rejection всеми V1-only handlers кроме permanent close. Пустая projected family не публикуется: при ineligible turn модель видит только исходную upstream family.

V1 UUID spawned agent остаётся bearer capability внутри доступного ThreadManager/store, а не доказательством subtree ownership. Cross-subtree addressing spawned V1 target сохраняется, но live или persisted root не является closeable agent и отклоняется обеими cross-runtime close surfaces; version guard не вводит V2 path authorization для остальных V1 tools.

Cold resume открывает проверенный V1 target и следует доступным `Open` V1 graph edges с native best-effort semantics. Без graph store возобновляется только target; graph/read и descendant-resume failures логируются и пропускаются, а уже загруженные instances не откатываются. Keyed guards сериализуют effects одного `ThreadId`, но не образуют atomic subtree transaction. Edge-derived parent вытесняет stale stored parent, а watcher/result/queue остаются связанными с конкретными runtime instances. Повторный ID в повреждённом cyclic graph отклоняется controlled error; загруженные до обнаружения resume-cycle instances могут остаться live. Cross-runtime close до первого side effect выполняет read-only preflight persisted `Open` graph, затем объединяет descendants из logical path catalog, live ThreadManager topology и persisted edges, записывает `Closed` до shutdown конкретного bound instance и удаляет runtime/catalog/residency state без reload для unloaded V2. Close обрабатывает descendants последовательно и прекращается на первой распространяемой lookup/replacement/shutdown/cleanup error без rollback уже закрытых instances; это non-transactional fail-fast traversal. Preflight намеренно не превращает динамический union catalog/live/graph в atomic snapshot: corrupt cross-source duplicate может быть обнаружен только при обработке соответствующего parent после более ранних effects. Parent lifecycle guard не позволяет конкурентному spawn опубликоваться после закрытия parent и не допускает утечки его V1 reservation. Explicit UUID resume остаётся caller-mediated и может обновить immediate parent edge; parent lineage не является authorization boundary. App-server direct input разрешён actual V1 child и по-прежнему запрещён actual V2 child.

## Namespace ownership

- Configured V2, dynamic или prebuilt runtime/MCP owner сохраняет весь namespace; projection подавляется и один раз за affected turn эмитится warning.
- Когда projection владеет namespace, extension executors из `multi_agent_v1` пропускаются с internal diagnostic.
- Частичное смешивание разных owners в одной projected family запрещено.
- Global config и dynamic-tool validators не ужесточаются.

## Публичные границы

Feature не добавляет app-server method, protocol/config field, enum variant, persistence format, migration, generated schema, dependency или model catalog entry. CLI и app-server получают обе families через shared core planner; TUI использует существующий UUID fallback и thread projections. В current 0.145 projection `thread/list` для stored row не обещает `canAcceptDirectInput`, поэтому возвращает `null`; authoritative live/resumed `thread/read` возвращает `true` для actual V1 child. Prompt cache key после cold resume следует persisted `session_id`, а не выводится из child UUID.

`dynamicTools` и `Thread.canAcceptDirectInput` остаются experimental app-server surfaces; stable app-server methods и wire/schema shape не меняются.

### App-server compatibility

| Client | App-server/core binary | Wire/schema | Результат |
| --- | --- | --- | --- |
| Current fork client | Current fork `0.145.0` | Unchanged 0.145 schema; experimental capability opt-in сохраняется | Получает projected V1 tools на eligible V2 turn; live/resumed V1 `thread/read` возвращает `canAcceptDirectInput: true` |
| Upstream или более ранний 0.145 client | Current fork `0.145.0` | Совместим: новых methods/fields/variants нет | Продолжает работать; feature реализуется server/core side, неизвестный client-side state не требуется |
| Current fork client | Upstream `0.145.0` или более ранний 0.145 candidate без feature | Совместим в пределах проверенной 0.145 schema | Работает с upstream behavior; projected `multi_agent_v1` отсутствует, silent server-side emulation не выполняется |
| Client, читающий сохранённые pre-port sessions | Current fork `0.145.0` | Existing `SessionMeta`, `SessionSource` и graph rows | Старые sessions читаются; missing version применяет только документированный Legacy V1 compatibility fallback, migration не требуется |

Schema regeneration не требуется, потому что `app-server-protocol`, `protocol`, generated schemas и persistence shape не изменены. Experimental `dynamicTools` namespace collision видим через existing per-turn warning; stable client compatibility от этой projection не зависит. Сочетания с app-server/client из других release lines не проверялись этой feature и подчиняются обычной upstream compatibility policy.

## Матрица распространения

| Capability | Native source of truth | Producers | Consumers | Projections | Намеренно не затронуто | Generated artifacts | Verification |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Additive tools | Turn eligibility и current V1 factories | Planner eligibility, native V1 spec/handler factories | Registry, Responses/Lite request builders, code mode | Model-visible `multi_agent_v1` рядом с unchanged V2 family | Native V1/Disabled plans и V2 schema | Не требуются: wire/schema не менялись | `projected_v1_`, Responses/Lite и code-mode regressions |
| Exact V1 | Private `MultiAgentRuntimeIntent` | Projected handler и generic persisted V1 resume | AgentControl, ThreadManager, Session, capacity/residency | Existing `SessionMeta.multi_agent_version = V1` | Ordinary `Inherit`, config precedence и V2 residency | Не требуются: intent private и non-persisted | `exact_v1_`, stopped-V2 и residency regressions |
| Fork history | Current history loader + V1 sanitizer | Legacy rollout loader, Paginated model-context loader | AgentControl child creation и LiveThread | Sanitized child context с preserved boundaries | Public paginated fork policy | Не требуются | Sanitizer, compaction, `WorldState` и Paginated fork regressions |
| Authority | Active turn/config snapshots, shared role reload и child-owned persisted history | Projected fresh/cold-resume handlers и ordinary role consumers | Fresh role overlay, permissions, environments, exec policy | Child runtime config, persisted role metadata и shared ignored-rules state | Mutable role replay при cold resume, другие ordinary V1/V2 config-authoritative semantics | Не требуются | Shared role ignore-bit, fresh-role, removed-role compatibility и cold managed-profile authority regressions |
| Cross-runtime lifecycle | Actual/persisted version + bound `Arc<CodexThread>` и existing UUID/`AgentPath` resolver | Projected V1/V2 close resolution, V1-only send/interrupt/wait/resume guards | AgentControl, watcher, queue, graph, residency и conditional cleanup | Existing `CollabAgentTool::CloseAgent` events/results; UUID и canonical task name | Новый wire identity type и V2 path authorization для остальных V1 tools | Не требуются | V2 spawn -> V1 close -> V1 spawn, V2 close -> V1 target, reused UUID, unloaded close и subtree regressions |
| Cold resume | Existing metadata и доступные graph `Open` edges | ThreadStore/graph lookup и exact-V1 admission | Session, registry, watcher, best-effort descendants | Restored pathless V1 target и доступная subtree | Closed/V2 branches, transactional rollback | Не требуются: existing rows reused | Restart, per-ID lifecycle guard, cycle и stale-parent regressions |
| App/TUI | Actual runtime и existing thread projections | Core runtime, app-server thread processor | Existing clients и TUI renderer | list/read/history/direct-input и UUID fallback | New client wire/state | Не требуются: schema/snapshots unchanged | Paginated app-server E2E и TUI UUID test |
| Collision | Resolved namespace ownership | Configured, dynamic, prebuilt runtime/MCP и extension tool sources | Planner/registry | Suppression + existing warning/diagnostic | Global validators и other namespaces | Не требуются | Configured/dynamic/runtime/MCP collision и once-per-turn warning regressions |

## Матрица верификации

| Surface | Required command/evidence | Что подтверждает |
| --- | --- | --- |
| Planner, collision и native compatibility | `just test -p codex-core projected_v1_` | Dual families, exact V1 schema, depth/exposure, owners, tool search, native V1/V2 preservation |
| Runtime, authority, lifecycle и persistence | `just test -p codex-core exact_v1_` плюс focused tests из [verification ledger](../projects/multi-agent-v1-native-child-under-v2/verification.md) | Exact admission, residency, permissions, bound UUID lifecycle, restart/cycle/stale-parent behavior |
| Responses API/Lite | `just test -p codex-core responses_lite_exposes_projected_v1_and_native_v2` | Shared planner reaches actual model request |
| App-server | `just test -p codex-app-server projected_v1_paginated_subagent_is_parented_persisted_and_accepts_direct_input` | Paginated persistence, list/read/history, prompt cache key и direct input |
| TUI | `just test -p codex-tui collab_agent_title_falls_back_to_thread_uuid_without_metadata` | Pathless V1 UUID projection без нового UI state |
| Package/workspace regression | `just test -p codex-core`, `just test -p codex-app-server`, `just test -p codex-tui`, затем `just test` | Affected crates и cross-workspace upstream paths; non-zero local results классифицируются только через baseline evidence |
| Quality/build | `just fix -p codex-core`, `just fix -p codex-app-server`, `just fix -p codex-tui`, `just fmt`, `just fmt-check`, `git diff --check`, `scripts/codex-fork-build.sh` | Lints/format, patch hygiene и stripped release binary |
| Release | Cross-platform CI и явная user acceptance | Platform confidence и разрешение на commit/install/release |

Фактические counts, non-zero classifications, audit verdicts и build evidence хранятся только в [verification ledger](../projects/multi-agent-v1-native-child-under-v2/verification.md), чтобы эта матрица оставалась contract-level checklist.

## Связанные документы

- [Project dossier](../projects/multi-agent-v1-native-child-under-v2/README.md)
- [Canonical design](../projects/multi-agent-v1-native-child-under-v2/design.md)
- [Verification ledger](../projects/multi-agent-v1-native-child-under-v2/verification.md)
- [Release research](../research/rust-v0.145.0/multi-agent-v1-native-child-under-v2.md)
- [Upstream multi-agent architecture](../research/multi-agents/architecture.md)

## Changelog документа

- 2026-07-22: зафиксированы upstream 0.145 baseline, upstream-shaped port contract и первоначальные propagation/lifecycle invariants.
- 2026-07-23: контракт синхронизирован с реализованными exact-runtime, authority, lifecycle и persistence paths; добавлены полный scope, propagation/verification и app-server compatibility matrices.
- 2026-07-23: после пользовательского workflow rollback начато повторное применение `a5e21107643d`; исправлены exec-policy inheritance, semantic fork suffix, eligibility/target coverage и порядок `Closed` before shutdown, а lifecycle/role docs возвращены к исходному upstream-first best-effort contract.
- 2026-07-23: current `fork/145` adaptation прошла focused verification, scoped lint/format, domain/integration audits и stripped release build; cross-platform CI оставлен отдельным release gate.
- 2026-07-24: cross-runtime follow-up отделил V1 capacity budget от V2 catalog/residency, добавил общий permanent close в projected V1 и V2 families, root rejection, mixed-subtree cleanup и двусторонние acceptance regressions.
