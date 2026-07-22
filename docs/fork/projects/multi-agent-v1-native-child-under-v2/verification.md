# Verification ledger

Этот ledger хранит только фактически выполненные проверки current `fork/145` port. Исторические результаты `fork/144.4` не считаются current evidence.

## Baseline

| Поле | Значение |
| --- | --- |
| Upstream release | `rust-v0.145.0^{}` / `25af12f7e61572b0bc18ddb1008be543b91519b0` |
| Fork baseline | `72c43db8d4a41ea1a23dc4b664cdbec52e85f683` |
| Historical feature | `2f2e7e294975881c1c83e3c46fba3a0096481890` |
| Candidate branch | `codex/fork-145/native-v1-under-v2-port` |
| Initial status | clean worktree, exact baseline SHAs confirmed |

## Required verification matrix

| Surface | Required evidence | Current status |
| --- | --- | --- |
| Planner/schema | Dual family, exact V1 schema/exposure, depth/provider eligibility | PASS: focused planner, Responses API и Lite regressions |
| Collisions | Configured/dynamic/MCP owner, extension suppression, per-turn warning | PASS: configured/dynamic/runtime/MCP ownership и once-per-turn warning покрыты |
| Runtime selection | Exact V1 before accounting; preview/turn/resume remain V1 | PASS: exact spawn/resume, residency и stopped-V2 preservation |
| Fresh/fork | Legacy/Paginated, pathless UUID, sanitized semantic context | PASS: обе history families, semantic developer text, compaction и `WorldState` |
| Authority | Named/managed profile, environments/roots, exec policy | PASS: fresh и cold-resume authority regressions |
| Lifecycle | Live/persisted V1 success; V2/Disabled/unresolved fail before effects | PASS: bound runtime, conditional cleanup, close ordering и cycle rejection |
| Persistence | Restart, Open-only V1 descendant resume, missing-version fallback | PASS: restart tree, stale parent repair и missing-version V1 fallback |
| App-server | list/read/history and `canAcceptDirectInput` V1 true/V2 false | PASS: focused paginated V2 app-server E2E |
| TUI/events | UUID fallback and canonical V1 items without V2 activity | PASS: focused UUID fallback; event assertions входят в core regressions |
| Regression | Native V1, native V2, tool search, roles/models and permissions | PASS focused; package/workspace gates non-zero из-за baseline/environment failures ниже |
| Public artifacts | No config/protocol/persistence/schema/dependency delta | PASS: diff отсутствует в protocol, schema, Cargo/lock, Bazel lock и model catalog |
| Build | Stripped release binary, version, SHA-256 и build hash | PASS: официальный release build и installer dry-run |

## Commands and results

Проверки выполнены 2026-07-22—2026-07-23 в isolated worktree `codex/fork-145/native-v1-under-v2-port`. После `just fix` и `just fmt` тесты не перезапускались согласно repository contract; release build собирает тот же отформатированный source state.

### Focused regressions

| Команда | Результат |
| --- | --- |
| `just test -p codex-core projected_v1_` | exit 0; 15/15 PASS |
| `just test -p codex-core exact_v1_` | exit 0; 6/6 PASS |
| `just test -p codex-core native_v1_fork_history_sanitizes_only_v2_runtime_context` и `just test -p codex-core normalized_fork_history_keeps_v1_fallback_when_child_version_is_missing` | exit 0; 2/2 PASS |
| `just test -p codex-core projected_v1_cold_resume_restores_current_managed_authority` | exit 0; 1/1 PASS |
| `just test -p codex-core resume_agent_from_rollout_uses_edge_data_when_descendant_metadata_source_is_stale` | exit 0; 1/1 PASS; persisted snapshot parent отдельно проверен |
| `just test -p codex-core resume_agent_from_rollout_holds_root_lifecycle_guard_through_descendant_traversal` | exit 0; 1/1 PASS |
| `just test -p codex-core projected_v1_resume_and_close_reject_cyclic_thread_spawn_graphs` | exit 0; 1/1 PASS |
| `just test -p codex-core projected_v1_close_accepts_persisted_target_without_registry_metadata` | exit 0; 1/1 PASS; реальный `Open -> Closed` и idempotence |
| `just test -p codex-core responses_lite_exposes_projected_v1_and_native_v2` | exit 0; 1/1 PASS |
| `just test -p codex-app-server projected_v1_paginated_subagent_is_parented_persisted_and_accepts_direct_input` | exit 0; 1/1 PASS |
| `just test -p codex-tui collab_agent_title_falls_back_to_thread_uuid_without_metadata` | exit 0; 1/1 PASS |
| `just test -p codex-core generic_exact_v1_resume_requires_persisted_pathless_subagent_source`, `just test -p codex-core exact_v1_resume_preserves_stopped_v2_runtime_mapping` | exit 0; 2/2 PASS |

Первичный поиск по несуществующему prefix `normalize_multi_agent_runtime_intent_` завершился exit 4 с `0 tests run`; он не засчитывается как verification failure. Вместо него выполнены два прямых runtime-policy regression-теста из последней строки таблицы.

### Package и workspace gates

| Команда | Результат и классификация |
| --- | --- |
| Чистый baseline: `just test -p codex-core` | exit 100; 2999 total, 2918 passed, 81 failed. Повторно воспроизведены missing test binaries, timeouts и ambient config/git/realtime assertions. |
| Candidate: `just test -p codex-core` | exit 100; 3024 total, 3004 passed, 20 failed. Feature-specific regressions прошли; оставшиеся failures относятся к baseline/environment classes. |
| `just test -p codex-app-server` | exit 100; 967 run, 966 passed, 1 failed, 1 skipped. Единственный failure: bundled Ubuntu 24.04 zsh требует `GLIBC_2.38` на Ubuntu 22.04; flaky OAuth повтор прошёл. |
| `just test -p codex-tui` | exit 100; 3221 executed, 3189 passed, 32 failed, 4 skipped. Все failures — существующие version/status snapshots (`v0.0.0` против `v0.145.0`); feature UUID regression прошёл. 27 `.snap.new` не приняты и удалены. |
| `just test` | exit 100; 12672 total, 12612 passed, 60 failed. Feature-specific app-server E2E, residency и spawn/resume config regressions прошли; failures сгруппированы как app-server-transport 1, app-server 1, core 20, core-skills 2, mcp-server 3, secrets 1 и TUI 32. |

Non-zero package/workspace gates не обозначаются как PASS. Их baseline/environment classification подтверждена отдельным clean-baseline прогоном и точечным сопоставлением; cross-platform CI остаётся release gate.

### Lint, format и build

| Команда | Результат |
| --- | --- |
| `just fix -p codex-core` | exit 0; один non-blocking `too_many_arguments` warning в private runtime normalizer |
| `just fix -p codex-app-server` | exit 0 |
| `just fix -p codex-tui` | exit 0 |
| `just fmt` | exit 0; formatter также нормализовал root `justfile` |
| `just fmt-check` | exit 0 после final docs update |
| `git diff --check` | exit 0 |
| `scripts/codex-fork-build.sh` | exit 0; release profile завершён за 9m21s; `codex-rs/target/release/codex` собран и stripped |
| `codex-rs/target/release/codex --version` | `codex-cli 0.145.0` |
| `sha256sum codex-rs/target/release/codex` | `d3b818cc86f16155a3221cdc87fa199bde87f70dbf5a507be04b8b38b49ed3cc` |
| `file codex-rs/target/release/codex` и `readelf -S ...` | ELF x86-64 PIE, BuildID `6b07cfb793af177e5a6681b8e159092fd6677e30`, stripped; `.debug_info` и `.symtab` отсутствуют |
| `scripts/2_codex-fork-install-binary.sh --dry-run` | exit 0; source и три managed target path подтверждены, файлов не изменено |

Build hash записывается скриптом в ignored `scripts/.codex-build-hash` и намеренно не встраивается в tracked ledger: изменение самого ledger изменяет dirty workspace hash. Финальное значение сообщается в handoff; после final commit build hash должен быть записан заново.

## Audit findings

Architecture, propagation, contract, security и runtime-persistence audits завершены PASS после исправления замечаний по cold authority, bound-runtime races, close ordering, lifecycle serialization, cyclic graph handling и module size. Tests-regression audit завершён `PASS_WITH_GAPS` с 0 High/Medium: остаются только локальные non-zero package/workspace gates, deferred cross-platform CI и необязательные focused cases для `namespace_tools=false`/Guardian eligibility и persisted Disabled/unresolved targets. Docs-evidence audit завершён PASS с 0 High/Medium после добавления полного feature delivery contract и 0.145 app-server compatibility matrix. Final integration audit завершён PASS с 0 High/Medium: main pre-state чист, 49 candidate paths изолированы, полный patch stream применим без конфликтов, build artifacts намеренно исключены.

## Known gaps

- Фактическая установка fork binary и restart app-server не входят в candidate verification без отдельной пользовательской команды.
- Ignored `codex-rs/target/` и `scripts/.codex-build-hash` являются локальными build artifacts и не коммитятся.
- Cross-platform CI не запускался; package/workspace gates локально остаются non-zero по перечисленным baseline/environment причинам.
