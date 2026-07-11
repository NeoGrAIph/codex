# Проверка

## Правило evidence

Evidence относится к полному current workspace diff `fork/144.1`, а не только к первоначальному staged donor subset. Результаты старого snapshot не переносятся как PASS после изменения production code. Focused checks, affected-crate suites, workspace suite, post-test schema/fix/fmt gates и source/audit review ниже выполнены после последних production и regression-test corrections. Release binary содержит workspace build identity, а этот tracked документ участвует в её вычислении, поэтому final artifact SHA намеренно не встраивается сюда: build/version/SHA/install dry-run выполняются после последнего tracked edit и приводятся в handoff вместе с ignored `scripts/.codex-build-hash`.

## Focused checks текущего snapshot

| Контракт | Команда/тест | Текущий результат |
| --- | --- | --- |
| V2/V1 namespace coexistence, schema separation, role/model projection | `just test -p codex-core spec_plan`; `just test -p codex-core multi_agents_spec` | 28 + 10 passed |
| V1 UUID output плюс canonical V2 path/message/list/wait interop | `multi_agent_v1_spawn_under_v2_returns_agent_id_and_assigns_internal_path` | passed |
| Реальный V2 terminal completion, generated author path, single mailbox delivery | `projected_multi_agent_v1_completion_uses_canonical_v2_path` | passed |
| Projected V1 следует V2 depth policy | `multi_agent_v1_spawn_under_v2_ignores_configured_max_depth` | passed |
| Projected V1 wait исполняет configured timeout и zero policy | `projected_wait_agent_uses_configured_timeout_options`; `projected_wait_agent_allows_zero_when_configured_timeout_allows_zero` | passed |
| Projected wait target dedup/cap; V1-only duplicate compatibility | `projected_wait_agent_deduplicates_targets_in_first_seen_order`; `projected_wait_agent_rejects_more_than_the_supported_target_count`; `legacy_wait_agent_parser_preserves_duplicate_targets` | passed в core suite |
| Same-manager foreign root не авторизуется; owned pathless UUID resume получает path | `projected_v1_resume_authorizes_owned_pathless_child_and_rejects_foreign_root` | passed |
| Root-control authorization с graph; graphless cold authorization fail-closed | `agent_target_authorization_is_scoped_to_the_root_control_tree`; `persisted_agent_authorization_rejects_graph_cycles`; enriched `resume_thread_subagent_restores_stored_metadata` | passed |
| Pathless backfill стабилен после второго resume и требует durable store | `pathless_v2_agent_resume_backfills_and_reuses_a_stable_path`; `agent_identity_backfill_requires_durable_state_metadata` | passed; тест первоначально обнаружил и после fix подтвердил metadata persistence |
| Nested resume сохраняет stored lineage/model/reasoning | `nested_agent_resume_preserves_persisted_lineage_model_and_reasoning` | passed с одним успешно повторённым flaky setup run |
| Current trusted role reapply и live security precedence | enriched `resume_thread_subagent_restores_stored_metadata`; `reapply_role_on_resume_preserves_runtime_owned_security_context` | passed |
| Removed persisted role fail-fast, thread не остаётся loaded | `cold_resume_rejects_removed_persisted_agent_role_without_loading_thread` | passed |
| Historical role-less fork не получает guessed default | `resume_without_persisted_role_does_not_guess_default_role` | passed |
| Bounded role catalog сохраняет default и unicode/escape-safe JSON-byte truncation | `spawn_tool_spec_bounds_role_catalog_and_preserves_default_role` | passed |
| Static reserved config namespaces и generated schema; `multi_agent_v1` остаётся loadable до mode-aware runtime gate | `multi_agent_v2_rejects_invalid_tool_namespace`; `config_schema_rejects_reserved_multi_agent_v2_tool_namespaces` | passed; schema regenerated on current snapshot |
| Shared dynamic validator и прежний MCP error contract | `reserved_dynamic_tool_namespaces_are_rejected`; `mcp_namespace_keeps_its_existing_validation_error_contract` | passed |
| Fresh V2 start и cold V2 resume отклоняют collision с configured namespace | `thread_start_rejects_dynamic_namespace_matching_configured_v2_namespace`; `resume_rejects_restored_dynamic_namespace_matching_configured_v2_namespace` | passed |
| Configured V2 namespace `multi_agent_v1` отклоняется mode-aware runtime gate | `thread_start_rejects_configured_v2_namespace_matching_projected_v1` | passed |
| Persisted V1 resume остаётся совместим с одноимённым неактивным V2 namespace | enriched `resume_restores_dynamic_tools_from_rollout_with_sqlite_enabled` | passed |
| Responses Lite request сохраняет оба model-visible namespace | `responses_lite_exposes_native_v2_and_projected_v1_namespaces` | passed |
| App-server input/config paths возвращают invalid request | `thread_start_rejects_invalid_dynamic_tool_inputs`; `thread_start_rejects_dynamic_namespace_matching_configured_multi_agent_namespace`; `thread_start_rejects_reserved_multi_agent_v1_namespace_from_project_config`; `validate_dynamic_tools_rejects_reserved_responses_namespace` | passed |
| Full-history projected child сохраняет exact canonical path, V2 marker, inherited role и reasoning через JSONL/cold resume | `full_history_fork_persists_inherited_role_for_resume` | focused run `25521d56-6299-4574-ae4d-60a7e95d6156` и current core suite passed |
| JSONL-only metadata reader проецирует canonical path из persisted source | `stored_thread_from_rollout_item_projects_agent_path_from_source` | current thread-store suite passed |
| Live metadata не понижает Open, missing edge восстанавливается Pending, row+edge commit атомарен; historical backfill остаётся Open | state `live_insert_uses_pending_activation_while_backfill_uses_open`; `live_insert_rolls_back_thread_row_when_pending_edge_insert_fails`; thread-store live/historical tests | focused runs `b9b0e615-8b61-4ba1-b98e-05901df9c3a0`, `0fadfa2e-c718-4b89-9c97-b746a5f2a44b`, `ff017229-39e5-4487-a402-85ee86b422a7`, `9e18d931-c00c-4f48-afa5-14f4af916b0a`, passed |
| Effective-V2 spawn без authoritative graph отказывает до создания child и не расходует spawn slot | `v2_thread_spawn_without_authoritative_graph_fails_closed` | run `77547f6e-794f-4b26-826c-9d6b29351723`, passed |
| Open, registry и residency готовы до latch Commit; public marker остаётся закрыт до завершения activation | `spawn_publication_waits_for_open_activation` | run `c2dfb17c-7659-44a3-81e9-dbe34bd9b2c9`, passed |
| Enqueue/promotion failure выполняет rollback/quarantine без graph/registry leak | `spawn_lifecycle_rollbacks_preserve_runtime_graph_and_registry_invariants` | run `b464376b-3f13-4a88-a15c-954b01589114`, passed |
| Latch commit failure сохраняет materialized Pending quarantine; queued recorder persistence дренируется до решения об удалении edge | `latch_commit_failure_rolls_back_and_retains_materialized_pending_quarantine`; `discard_waits_for_queued_persist_before_pending_cleanup` | runs `30213bd0-a641-4760-92e6-ea30e9f57978`, `7e2a5257-744c-4465-b05d-d0c7bcc05810`, passed |
| Late termination освобождает runtime/registry/residency, но сохраняет materialized Pending quarantine | `late_termination_cleanup_releases_runtime_and_retains_materialized_quarantine` | run `1a84569d-a431-4f57-b032-ae412fa0a1f8`, passed |
| Native V1 сохраняет best-effort spawn/close, transactional V2 close fail-closed и lifecycle operations сериализованы | `native_v1_spawn_keeps_graph_persistence_best_effort_and_skips_v2_latch`; `native_v1_close_keeps_graph_persistence_best_effort`; `transactional_close_keeps_live_thread_running_when_persisted_edge_is_missing`; lifecycle serialization test | runs `ad53a750-caaf-4738-a9c4-7fae18b9fac7`, `9aac0b28-ea0e-4476-9a81-9c17f5d3ffe8`, `ccb14846-47c7-4f76-98c7-bddcac929ca6`, `3a424d44-b36c-4017-85b8-e8d0be645b6b`, passed |
| Legacy handlers выбирают native V1 либо transactional V2 по `turn.multi_agent_version`; весь multi-agent regression filter | `legacy_send_and_close_route_by_turn_multi_agent_version`; `just test -p codex-core multi_agent` | deterministic waiter-counter focused run `6605b6ca-22b1-4c7f-b7df-2d62ee14d861`, passed; aggregate run `576336f9-ede5-41f0-86f5-f047616ae899`, 140 passed, exit 0 |
| App-server скрывает loaded V2 child при Pending и missing edge из list/search/read/resume, но показывает Open | `thread_resume_rejects_loaded_v2_agent_with_pending_activation` | run `aba25191-1956-42f6-8246-edbaa42ec32e`, passed |
| Config-map `model_provider` и explicit model/reasoning остаются выше persisted metadata/role | unit `merge_persisted_resume_metadata_preserves_config_map_provider_override`; integration `thread_resume_explicit_overrides_win_over_persisted_v2_role` | runs `f003440c-7c61-4a44-babc-282ec41e2e47`, `af1501ec-43cd-4693-a919-cf8ac856231c`, passed |

Handler tests используют test-only `ThreadManager::with_persistence_for_tests` с in-memory `ThreadStore` и `AgentGraphStore`, чтобы effective-V2 path проверял тот же обязательный persistence contract, что production runtime; helper закрыт `#[cfg(test)]` и не меняет public/runtime API.

## Broad gates текущего snapshot

| Gate | Run ID | Результат | Disposition |
| --- | --- | --- | --- |
| `just test -p codex-agent-graph-store` | `128c0cbe-3841-4a36-b788-f4d39e32edb0` | 4 passed, exit 0 | PASS |
| `just test -p codex-state` | `5f1e5a1b-7269-4623-8679-8c176c384fa9` | 159 passed, exit 0 | PASS |
| `just test -p codex-thread-store` | `fc3fa808-f4df-4dae-b893-2d82f5ae2fae` | 100 passed, exit 0 | PASS |
| `just test -p codex-tools` | `4cba0e1e-6d49-466d-bbe7-d500d9558a5b` | 92 passed, exit 0 | PASS |
| `just test -p codex-features` | `46ad5274-e708-46b7-a736-a7d580945d00` | 55 passed, exit 0 | PASS |
| `just test -p codex-core` | `73cbc1b1-46e9-493f-bc93-cc4ec931e8c3` | 2944 passed, 32 failed, 15 skipped, exit 100 | Failures относятся к ambient `/tmp` project discovery, read-only proxy CA path, incompatible bundled zsh/GLIBC, sandbox/helper/request-permissions host restrictions либо unrelated timing. Multi-agent/lifecycle feature failures отсутствуют; единственный candidate-adjacent `spawned_subagent_execpolicy_amendment_propagates_to_parent_session` прошёл отдельно в run `3a17e274-23c0-49d4-9e55-c6f669c027fa`. |
| `just test -p codex-app-server` | `b9653326-674b-4f64-addf-d70d8dbcf960` | 950 passed, 1 failed, 1 skipped, exit 100 | Единственный failure вызван bundled Ubuntu 24.04 zsh, которому на host не хватает `GLIBC_2.38`; feature regression нет. |
| полный `just test` | `f18a2089-bf47-4cbb-b675-fb8ace2b52d9` | 11981 passed, 76 failed, 28 skipped, exit 100 | Все failures относятся к доказанным ambient/upstream категориям: read-only managed CA path, bundled zsh/GLIBC, `/tmp` project discovery, unrelated TUI snapshots, sandbox/helper/request-permission timing, MCP/network baseline и другие unrelated tests. Multi-agent/lifecycle feature failures отсутствуют; candidate-adjacent test прошёл отдельно в run `3a17e274-23c0-49d4-9e55-c6f669c027fa`. |

Предыдущие workspace results, включая `11 921 passed, 60 failed, 28 skipped`, `11 934 passed, 76 failed, 28 skipped` и run `866bc33e-77ad-4238-9e6c-0d78645a0055`, относятся к более ранним snapshots и не являются сертификатом текущей реализации. Current workspace suite создал 26 unrelated TUI `*.snap.new`; удалены только эти generated artifacts, после cleanup `snap_new_count=0`.

Post-test gates выполнены в требуемом порядке: после удаления 26 generated snapshots `just write-config-schema` завершился с exit 0; `just fix -p` для `codex-agent-graph-store`, `codex-app-server`, `codex-core`, `codex-features`, `codex-state`, `codex-thread-store`, `codex-tools` и `codex-thread-manager-sample` завершились с exit 0; feature-related Clippy diagnostics исправлены. После добавления handler-routing regression повторный `just fix -p codex-core` и финальный `just fmt` завершились с exit 0. После последнего `fix`/`fmt` tests по repo policy не перезапускались. `git diff --check`, `git diff --cached --check`, проверка unmerged entries и sequencer markers прошли, `snap_new_count=0`.

Последовательность финального release gate: после последнего tracked docs update выполнить `scripts/codex-fork-build.sh`, проверить `codex-rs/target/release/codex --version` и SHA-256, затем запустить `scripts/2_codex-fork-install-binary.sh --dry-run`; после build tracked files больше не менять. Пользовательская acceptance описанного change-size disposition и current-binary installed-runtime acceptance остаются отдельными delivery gates.

## Generated artifacts

Изменение `MultiAgentV2ConfigToml.tool_namespace` schema проверено current-snapshot `just write-config-schema`, exit 0. `codex-rs/core/config.schema.json` содержит optional string с identifier/length constraints и `not` pattern для статических built-in Responses namespaces и `mcp(?:__.*)?`; mode-aware collision с `multi_agent_v1` проверяет exact runtime, поэтому поле остаётся loadable и не стало required. Dependency и Bazel lockfiles не меняются.

## Delivery size disposition

Полный candidate превышает 800 review-bearing lines и не должен приземляться одним atomic commit. Код остаётся единым незакоммиченным integration diff до пользовательской приёмки, но landing обязан сохранять dependency order: behavior-preserving module extraction; bounded/materialized role catalog; static/dynamic namespace contract; thread-store identity projection/durable metadata; graph status/storage и атомарный live metadata edge; submission latch; delayed registry/residency/publication; public list/search/read/resume projection; rollback/quarantine cleanup; close/reopen subtree и stale-metadata precedence; trusted effective-V2 role reconstruction; dormant projected V1 runtime semantics; UUID ownership authorization; model-visible planning activation последней. Соответствующие doc hunks следуют вместе с каждой стадией. Случайное hunk-staging из mixed index и `git add -A` запрещены; patches должны быть подготовлены из проверенного candidate отдельно, а generated TUI `*.snap.new` не относятся к feature.

## Live runtime evidence и build identity

Исторический A/B до текущего hardening patch:

- foreground `target/release/codex` при default route использовал stale reused daemon другой сборки, где legacy namespace отсутствовал;
- override `features.multi_agent_v2.enabled=true` переключил тот запуск на embedded runtime;
- реальный `multi_agent_v1.spawn_agent` создал child, `multi_agent_v1.wait_agent` дождался результата, `pwd` вернул `/home/neograiph/repo/AGENTS/codex-fork`, exit `0`;
- отсутствие DirectModelOnly tools в `ALL_TOOLS` оказалось непригодным negative probe.

Pre-final current-code release build `scripts/codex-fork-build.sh` завершился с exit 0 за 11:29 и создал `codex-cli 0.144.1`; install dry-run также завершился с exit 0 и подтвердил `dry run: no files changed`. Этот build предшествует последнему tracked evidence update и потому используется только как compilation proof. Финальный artifact строится повторно по последовательности выше; exact binary SHA и workspace identity не записываются в tracked input, а приводятся в handoff и ignored `scripts/.codex-build-hash`. Единственный наблюдавшийся compiler warning `unused_mut` относится к нетронутому `codex-rs/app-server/src/lib.rs` и не является feature regression. Реальная замена binaries, daemon restart и current-binary spawn/wait/completion probe остаются отдельным явно разрешаемым delivery step и не подменяются historical A/B.

## Audit disposition

Финальный tests-regression audit обнаружил единственный Medium: control primitives были покрыты, но handler routing для V1/V2 не проверялся напрямую. Finding закрыт test-only `legacy_send_and_close_route_by_turn_multi_agent_version`: тест дожидается фактического waiter на том же root lifecycle mutex и доказывает, что projected V1 под V2 ждёт gate для send/close, а native V1 выполняет обе операции без V2 coordinator. Focused и aggregate runs прошли; test-only hooks существуют только под `#[cfg(test)]`. Повторный legacy auditor call завершился без payload (`completed: null`), поэтому closure независимо перепроверен root-agent по source diff и exact runs. Latest architecture/propagation/contract/security/runtime-persistence/docs-evidence/integration reruns также потеряли payload в legacy transport; их предыдущие подробные PASS/единственный stale-doc finding были учтены, stale docs исправлены, а последние изменения ограничены graph preflight, discard ordering и test-only coverage и проверены root-agent по тем же scopes. Unresolved feature-causal High/Medium нет.

## Provenance delta

Donor `dcf47dfce3b682bc2efe64025ab2de8265fab145` добавлял activation/projection и передавал role/model metadata в projected V1. Current port сохраняет role discovery, но заменяет дублируемый model catalog на `OmitForProjection`, ограничивает role catalog и добавляет обязательный V1-under-V2 runtime/security/persistence adapter, которого activation-only patch не содержал.
