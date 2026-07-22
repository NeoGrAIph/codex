# multi-agent-v1-native-child-under-v2

## Статус

Upstream-first порт на `fork/145` реализован в isolated candidate, прошёл focused regressions, domain audits и официальный release build. Локальные package/workspace gates остаются non-zero только по зафиксированным baseline/environment классам; cross-platform CI и явная пользовательская приёмка ещё требуются. Исторический commit `2f2e7e2949` не является current implementation source of truth.

## Цель

Сделать exact native `multi_agent_v1` доступным effective-V2 models, сохранив current upstream 0.145 runtime selection, roles/models, Legacy/Paginated persistence, permission/environment authority и V2 path/mailbox lifecycle.

## Канонические документы

- [Пользовательский контракт](../../features/multi-agent-v1-native-child-under-v2.md)
- [Current fork architecture и data flow](design.md)
- [Фактическая verification](verification.md)
- [Release-specific baseline и porting decision](../../research/rust-v0.145.0/multi-agent-v1-native-child-under-v2.md)
- [Чистая upstream architecture map](../../research/multi-agents/architecture.md)

`design.md` является source of truth fork delta. Общая multi-agent architecture map намеренно остаётся описанием чистого upstream `rust-v0.145.0`.

## Карта реализации

| Поверхность | Основные файлы | Роль |
| --- | --- | --- |
| Planner/registry | `codex-rs/core/src/tools/spec_plan.rs`, `tools/multi_agent_v1_projection.rs`, `tools/handlers/multi_agents.rs` | Eligibility, namespace ownership, native V1 factories и projected handler mode |
| Runtime admission | `codex-rs/core/src/agent/control.rs`, `agent/control/spawn.rs`, `thread_manager.rs`, `thread_manager/multi_agent_runtime.rs`, `session/*` | Private exact-V1 intent до accounting/Session и unchanged ordinary `Inherit` |
| History/authority | `codex-rs/core/src/agent/control/fork_history.rs`, `tools/handlers/multi_agents/spawn.rs`, `resume_agent.rs`, `config/mod.rs` | Legacy/Paginated sanitation и active turn permission/environment authority |
| Lifecycle/persistence | `codex-rs/core/src/agent/control/{target_version,resume,legacy}.rs`, `thread_manager/lifecycle.rs` | Bound V1 capability, conditional cleanup, keyed resume/close и graph restart |
| App/TUI | `codex-rs/app-server/src/request_processors/thread_processor_tests.rs`, `app-server/tests/suite/v2/projected_v1.rs`, `codex-rs/tui/src/multi_agents.rs` | Existing thread projections, direct-input E2E и pathless UUID fallback |
| Evidence | `codex-rs/core/src/**/*tests*.rs`, `codex-rs/core/tests/suite/*`, `verification.md` | Focused regressions, package/workspace classification, audits и build evidence |

## Границы проекта

- Переносится capability, а не старый diff.
- Один write-owner меняет cross-module runtime contract; parallel agents выполняют только read-only research/audit.
- Public config/protocol/persistence и dependency surfaces не меняются.
- Worker checkpoints интегрируются в основной `fork/145` только как незакоммиченный candidate patch.
- Final commits, push, install и runtime replacement выполняются только после соответствующей пользовательской приёмки/команды.
