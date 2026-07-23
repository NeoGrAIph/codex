# rust-v0.145.0: нативный V1 child при effective V2

## Паспорт baseline

- Release tag: `rust-v0.145.0`.
- Peeled release commit: `25af12f7e61572b0bc18ddb1008be543b91519b0`.
- Fork baseline commit: `72c43db8d4a41ea1a23dc4b664cdbec52e85f683`.
- Исторический feature commit: `2f2e7e294975881c1c83e3c46fba3a0096481890` на baseline `rust-v0.144.4`.
- Нормализованная карта чистой upstream-архитектуры: [архитектура multi-agent](../multi-agents/architecture.md).

Rust source fork baseline совпадает с release snapshot; существующее отличие `codex-rs/Cargo.lock` принадлежит workflow baseline и не относится к этой feature. Исторический commit используется только как источник пользовательского контракта и regression cases: его runtime, persistence и planner diff не переносится механически.

## Нативное поведение rust-v0.145.0

| Поверхность | Нативный контракт |
| --- | --- |
| Runtime selection | `Config::multi_agent_version_override()` имеет приоритет над history/inheritance для обычных sessions |
| Tool planner | Effective V1 и V2 публикуют взаимоисключающие families через `CoreToolRuntime`; V1 уже является namespace tool |
| Roles и модели | Единый `[agents]`, current role catalog, active-backend model filtering и post-role reasoning validation |
| Spawn/fork | AgentControl выбирает accounting/residency до Session; spawned subagents поддерживают Legacy и Paginated history |
| Paginated persistence | Bounded model context, inherited prefix и `subagent_history_start_ordinal` принадлежат ThreadStore/LiveThread |
| Resume | V2 identity, nickname и role восстанавливаются из current store/graph path; ordinary V1 resume использует UUID lifecycle |
| Permissions | Active permission profile identity, per-environment roots и exec-policy inheritance являются runtime authority |
| App/TUI | `Thread.canAcceptDirectInput` зависит от actual child runtime; parent-owned V2 child остаётся read-only; stored `thread/list` не вычисляет live capability и может возвращать `null` |
| Tool sources | Prebuilt runtimes, extensions и dynamic tools участвуют в едином planner/registry pipeline |

## Разрыв относительно feature contract

Upstream по-прежнему не публикует V1 tools при effective V2 и не имеет creation-scoped exact-V1 intent. Простого добавления specs недостаточно:

- config-authoritative V2 снова превратит fresh, forked или resumed child в V2;
- старый full-rollout loader не поддерживает Paginated history;
- старый sanitizer потеряет bounded context, response item IDs и inherited-prefix boundary;
- старый target guard не сможет проверить persisted Paginated target;
- старый permission override потеряет named/managed profile identity и profile roots;
- collision check только для configured/dynamic namespace не учитывает prebuilt runtime/MCP tools.

## Conflict-prone upstream owners

- `codex-rs/core/src/tools/spec_plan.rs` и `codex-rs/core/src/tools/handlers/multi_agents*.rs`: model-visible family planning, registry ownership и lifecycle handler dispatch.
- `codex-rs/core/src/agent/control.rs` и `codex-rs/core/src/agent/control/{spawn,resume,legacy}.rs`: accounting, history fork, completion watcher и UUID lifecycle.
- `codex-rs/core/src/thread_manager.rs`, `codex-rs/core/src/thread_manager/*` и `codex-rs/core/src/session/*`: runtime selection, Session creation, registry/residency и permission/environment authority.
- `codex-rs/app-server/src/request_processors/thread_processor.rs` и `codex-rs/tui/src/multi_agents.rs`: existing client/UI projections, которые должны наследовать core state без нового wire/UI source of truth.

При следующем release port эти owners нужно повторно сверять с [чистой upstream architecture map](../multi-agents/architecture.md); исторические 0.144.4 line numbers и diff hunk boundaries не являются переносимым baseline.

## Porting decision

Выбрана повторная реализация capability поверх native 0.145 owners:

- private exact-runtime intent выбирает V1 до accounting и Session creation, но не меняет ordinary `Inherit` precedence;
- existing `SessionMeta.multi_agent_version` и pathless `ThreadSpawn` остаются единственным durable contract;
- V2-to-V1 fork sanitation встраивается в current Legacy/Paginated path после bounded loading и filtering;
- projected lifecycle сначала подтверждает actual/persisted V1 и удерживает live runtime capability на конкретный `Arc<CodexThread>`, не добавляя UUID ownership policy;
- keyed lifecycle guards и bound instances сериализуют effects одного `ThreadId`; graph traversal и descendant recovery сохраняют native best-effort semantics, а graph-derived parent имеет приоритет над stale rollout metadata;
- current permission/environment snapshot переносится без деградации к legacy profile;
- fresh role overlay не расширяет runtime authority, а cold V1 resume не требует повторной загрузки mutable role file и доверяет child-owned persisted history;
- shared role reload сохраняет security-sensitive `ignore_user_and_project_exec_policy_rules` для projected и ordinary role consumers, не меняя остальные role/model semantics;
- projected live close пытается записать root `Closed` до shutdown, сохраняя upstream warning-only policy для live graph-write failures;
- tool projection строится current native-V1 factories и подчиняется namespace-level ownership policy;
- app-server protocol и TUI production contract не расширяются.

## Обязательные invariants

- Exact V1 разрешён только для pathless subagent new/full-history-fork или проверенного resume.
- Exact intent не является config, protocol, persisted field или новым `MultiAgentVersion`.
- Native V2 family, path/mailbox/activity/residency и current model/role behavior не меняются.
- Native V1 UUID остаётся bearer capability внутри доступного manager/store, а не security boundary.
- Missing UUID сохраняет native per-tool not-found semantics; V2/Disabled/unresolved target отклоняется до side effects.
- Live projected lifecycle не следует за повторно использованным UUID: send/wait/close действуют на bound V1 instance, а conditional cleanup не удаляет late V2 replacement.
- Same-UUID effects не переходят на replacement runtime; per-ID guards не являются atomic subtree transaction.
- Cold resume следует доступным `Open` V1 edges best-effort: без graph store возобновляется только target, descendant failures пропускаются с warning, уже загруженные instances не откатываются.
- Projected close последователен и non-transactional: первая распространяемая descendant error прекращает обход без rollback уже закрытых instances.
- Legacy missing-version history сохраняет V1 compatibility fallback; нечитаемая history завершается controlled error.
- Public fork paginated thread остаётся запрещён; internal subagent fork сохраняет current paginated semantics.
- Configured/dynamic/prebuilt runtime owner сохраняет `multi_agent_v1`; активная projection не смешивается с extension tools того же namespace.
- Не добавляются dependencies, lockfile updates, config schema, app-server wire или persistence migration.

## Release-specific verification

Фактические команды, exit codes, audit findings, build hash и coverage gaps фиксируются только в [verification ledger](../../projects/multi-agent-v1-native-child-under-v2/verification.md). Этот документ описывает baseline и porting decision, но не является доказательством release readiness.
