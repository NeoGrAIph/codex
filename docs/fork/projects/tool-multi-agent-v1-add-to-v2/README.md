# `multi_agent_v1` рядом с V2

## Цель проекта

Добавить model-visible legacy namespace `multi_agent_v1` в effective V2 sessions без aliasing и без второй модели runtime state: V1 wire contract остаётся UUID-based, а child identity, completion, persistence и security проходят через нативные V2 `AgentPath`, `SessionSource`, `AgentControl` и thread-store paths.

## Критерии готовности

- V2 planning показывает default `collaboration` либо configured V2 namespace и отдельный namespace `multi_agent_v1`; V1-only direct/deferred behavior не меняется.
- V1/V2 request и response schemas остаются раздельными.
- Projected V1 role catalog доступен модели, ограничен 32 entries, 768 JSON-bytes на entry и 2048 JSON-bytes суммарно и корректно объединяет materialized configured project/user roles с отдельным built-in catalog без повторного чтения файлов; native V1 и V2 spawn также применяют тот же immutable per-Config role snapshot.
- Каждый новый V1-under-V2 child получает canonical generated path, доступен обеим control surfaces и доставляет terminal completion через V2 mailbox ровно один раз.
- Effective-V2 spawn требует authoritative graph до создания child. Fresh child получает атомарно согласованные live metadata и `PendingActivation`; queued task не исполняется до durable `Open`, registry/residency commit и latch `Commit`, а public marker/client notification публикуются только после этого. Enqueue/promotion/latch failure выполняет fail-closed rollback: recorder shutdown дренирует queued persistence до materialization decision, unmaterialized edge удаляется, а materialized rejected rollout сохраняет restart-durable Pending quarantine после runtime cleanup. Loaded V2 thread без published marker, с `PendingActivation` либо без authoritative edge отсутствует в `thread/list`/`thread/search` и отклоняется `thread/read`/`thread/resume`.
- V1 lifecycle UUID targets под V2 авторизуются только в пределах root control tree; live membership берётся из root-scoped registry, cold membership — из authoritative persisted graph, а graphless/self/root/foreign targets fail-fast.
- Projected V1 depth, wait и transactional lifecycle runtime соответствуют V2 planning/config contract; native V1 сохраняет прежние best-effort graph/lifecycle semantics без V2 coordinator и пятисекундного termination timeout.
- Historical persisted-V2 pathless resume получает один stable persisted path только при durable SQLite metadata; существующий persisted path никогда не заменяется fallback, недоступная persistence приводит к controlled failure, а implicit send/reload требует сначала явный resume.
- Cold resume восстанавливает stored lineage/model/reasoning, повторно применяет current trusted persisted role, сохраняет live security/workspace/cwd/base-instruction context и отклоняет удалённую роль без fallback; historical role-less fork не получает guessed default.
- Config loader/generated schema для статических namespace и mode-aware dynamic tool start/resume/app-server error projection используют согласованный reserved namespace contract.
- Focused tests, полные затронутые crate suites, workspace `just test`, scoped `just fix`, `just fmt`, diff checks и независимые финальные аудиты имеют snapshot-aware evidence.

## Scope

В scope входят planning/spec, V1 compatibility handlers, shared `AgentControl`/thread-manager resume paths, bounded role description, config/schema/dynamic validation, app-server error mapping, tests и fork docs.

Не входят новые protocol/API types, TUI changes, dependency/lockfile changes, data migrations, launcher/daemon replacement и install/restart automation.

## Source of truth и provenance

- Нативный architecture baseline: `docs/fork/research/multi-agents/architecture.md`.
- Feature contract: `docs/fork/features/tool-multi-agent-v1-add-to-v2.md`.
- Design decisions: `design.md`.
- Verification evidence: `verification.md`.
- Donor activation baseline: `dcf47dfce3b682bc2efe64025ab2de8265fab145` из `fork/142`.
- Текущий implementation source of truth до отдельного commit-запроса: полный tracked/untracked diff рабочего дерева `fork/144.1`, а не только ранее staged donor subset.
