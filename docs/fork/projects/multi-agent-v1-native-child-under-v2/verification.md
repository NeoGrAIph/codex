# Verification ledger

Этот ledger разделяет фактические проверки зафиксированной в `fork/145` базовой реализации, текущего незакоммиченного cross-runtime follow-up и исторические результаты первого 0.145 candidate. Evidence из `fork/144.4` не считается проверкой текущего релиза.

## Baseline и provenance

| Поле | Значение |
| --- | --- |
| Upstream release | `rust-v0.145.0^{}` / `25af12f7e61572b0bc18ddb1008be543b91519b0` |
| Fork baseline | `72c43db8d4a41ea1a23dc4b664cdbec52e85f683` |
| Historical feature contract | `2f2e7e294975881c1c83e3c46fba3a0096481890` на `fork/144.4` |
| First 0.145 implementation | `a5e21107643db7c94d9409b4c11cbee551a7dfe7` |
| Workflow rollback | `10713a4a40404db487a2e42aa245cbfe8767cf9a`; выполнен по явной пользовательской команде при уточнении смысла feature, а не как локально зафиксированный defect verdict |
| Committed 0.145 implementation | `39f346c47988cc3a4385fa12f931547a5646ec62`; повторное применение базовой feature с release-specific adaptations |
| Current candidate | незакоммиченный cross-runtime capacity/close follow-up поверх `39f346c47988`; прошёл focused verification, scoped lint/format, architecture/propagation/runtime audits и recorded release build |

## Required verification matrix

| Surface | Required evidence | Current status |
| --- | --- | --- |
| Planner/schema | Dual family, exact V1 schema/exposure, depth/provider/Guardian eligibility | PASS: current `projected_v1_` aggregate 22/22 |
| Collisions | Configured/dynamic/MCP owner, extension suppression, per-turn warning | PASS targeted |
| Runtime selection | Exact V1 before accounting; preview/turn/resume remain V1 | PASS: `exact_v1_` и focused selectors |
| Fresh/fork | Legacy/Paginated, pathless UUID, sanitized semantic context | PASS targeted, включая semantic suffix |
| Authority | Named/managed profile, environments/roots, shared ignored exec-policy rules | PASS targeted; shared role reload сохраняет ignore bit, fresh role не расширяет authority, removed persisted role не блокирует cold resume |
| Lifecycle | Projected V1 и V2 permanent close принимают spawned V1/V2; V1-only send/interrupt/wait/resume отклоняют V2; root/Disabled/unresolved fail before effects | PASS targeted, включая bound runtime, cross-subtree bearer UUID, root rejection, `Closed` before shutdown и документированный sequential first-error close |
| Persistence | Target и доступные Open V1 descendants, missing-version fallback, native best-effort failure policy | PASS targeted; transactional subtree semantics намеренно не заявляются |
| App-server | list/read/history and `canAcceptDirectInput` V1 true/V2 false | PASS: focused paginated V2 app-server E2E |
| TUI/events | UUID fallback and canonical V1 items without V2 activity | PASS: focused UUID fallback; event assertions входят в core regressions |
| Public artifacts | No config/protocol/persistence/schema/dependency delta | PASS by diff inspection; generated artifacts не требуются |
| Lint/format | Scoped `just fix`, `just fmt`, `just fmt-check`, `git diff --check` | PASS для current cross-runtime follow-up |
| Build | Current stripped release binary, version, SHA-256 и BuildID | PASS для current cross-runtime follow-up |
| Release | Cross-platform CI | NOT RUN; отдельный release gate |

## Cross-runtime lifecycle follow-up (2026-07-24)

Follow-up разделяет runtime budgets и совместит lifecycle surfaces: `multi_agent_v1` получает собственный `agents.max_threads` budget, V2 catalog/residency его не расходует, projected V1 `close_agent` принимает canonical V2 path/UUID, а V2 получает permanent `close_agent` для spawned V1/V2 targets рядом с non-terminal `interrupt_agent`. Отдельный V1 slot — явное пользовательское решение для fork, поэтому mixed session может одновременно использовать V1 и V2 budgets.

| Команда | Результат |
| --- | --- |
| `just test -p codex-core catalog_only_agent_does_not_consume_dedicated_v1_capacity` | exit 0; 1/1 PASS; registry ownership и идемпотентное освобождение V1 slot |
| `just test -p codex-core live_v2_agent_does_not_consume_dedicated_v1_capacity` | exit 0; 1/1 PASS; live V2 agent остаётся в catalog/residency, пока projected native V1 успешно занимает собственный slot при `agents.max_threads = 1` |
| `just test -p codex-core projected_v1_close_accepts_live_v2_while_other_lifecycle_tools_reject_it` | exit 0; 1/1 PASS |
| `just test -p codex-core projected_v1_close_accepts_persisted_v2_without_reload` | exit 0; 1/1 PASS |
| `just test -p codex-core v2_spawn_projected_v1_close_by_path_then_v1_spawn_succeeds` | exit 0; 1/1 PASS; exact reproduction path V2 spawn -> projected V1 close by `/root/worker` -> native V1 spawn при `agents.max_threads = 1` |
| `just test -p codex-core multi_agent_v2_close_accepts_persisted_v1_uuid_without_registry_metadata` | exit 0; 1/1 PASS; обратный V2 close -> persisted V1 UUID без reload |
| `just test -p codex-core multi_agent_v2_close` | exit 0; 3/3 PASS; live V1 capacity release, persisted V1 UUID и canonical V2 subtree |
| `just test -p codex-core cross_runtime_close` | exit 0; 5/5 PASS; root rejection, production root/descendant replacement binding, persisted cycle preflight и schema |
| `just test -p codex-core concurrent_close_prevents_late_v1_spawn_and_preserves_capacity` | exit 0; 1/1 PASS; barrier подтверждает parent guard, отсутствие late orphan и отсутствие утечки V1 slot |
| `just test -p codex-core projected_v1_close_accepts_legacy_spawn_source_without_parent_column` | exit 0; 1/1 PASS; legacy `ThreadSpawn` source без `parent_thread_id` и без version marker остаётся spawned V1 target |
| Три affected planner/negative regressions после schema/diagnostic adaptation | exit 0; 3/3 PASS |
| `just test -p codex-core` | earlier pre-final-follow-up snapshot: exit 100; 3033 tests, 2955 passed, 78 failed, 12 skipped; current focused inventory содержит 3053 tests, поэтому этот non-PASS run не является итоговым current aggregate; feature-specific failures были локализованы и исправлены, остальные failures относились к ambient config/git/MCP/network/sandbox classes historical baseline |

## Current focused commands

Проверки ниже выполнены 2026-07-23 в основном checkout `fork/145` после `git cherry-pick --no-commit a5e21107643d` и соответствующих adaptations.

| Команда | Результат |
| --- | --- |
| `just test -p codex-core projected_v1_` | current exit 0; 22/22 PASS; первый pre-adaptation aggregate был 16/16 |
| `just test -p codex-core exact_v1_` | exit 0; 6/6 PASS |
| `just test -p codex-core apply_role_preserves_ignored_user_and_project_exec_policy_rules` | exit 0; 1/1 PASS |
| `just test -p codex-core projected_v1_spawn_reapplies_runtime_authority_after_role_config` | exit 0; 1/1 PASS |
| `just test -p codex-core projected_v1_cold_resume_restores_current_managed_authority` | exit 0; 1/1 PASS; missing mutable role остаётся metadata и resume сохраняет active authority/exec-policy compatibility |
| `just test -p codex-core projected_v1_is_hidden_without_namespace_tool_support` и `just test -p codex-core projected_v1_is_hidden_for_guardian_reviewer` | exit 0; 2/2 PASS |
| `just test -p codex-core projected_v1_persisted_disabled_and_unresolved_targets_fail_before_effects` | exit 0; 1/1 PASS |
| `just test -p codex-core projected_v1_cross_root_uuid_addressing_remains_allowed` | exit 0; 1/1 PASS |
| `just test -p codex-core projected_v1_live_close_persists_closed_before_shutdown` | exit 0; 1/1 PASS; barrier доказывает write-before-shutdown, injected live write failure — warning-only continuation |
| `just test -p codex-core native_v1_fork_history_preserves_semantic_suffix_after_structural_mode_fragment` | exit 0; 1/1 PASS |
| `just test -p codex-core responses_lite_exposes_projected_v1_and_native_v2` | exit 0; 1/1 PASS |
| `just test -p codex-app-server projected_v1_paginated_subagent_is_parented_persisted_and_accepts_direct_input` | exit 0; 1/1 PASS |
| `just test -p codex-tui collab_agent_title_falls_back_to_thread_uuid_without_metadata` | exit 0; 1/1 PASS |
| Девять focused runtime/persistence regressions: fork sanitation/fallback, cold resume, stale-parent, lifecycle guard, cycle, persisted close, explicit resume и stopped-V2 mapping | exit 0; 9/9 PASS |

Первый compile attempt новых tests выявил только несовместимые test helpers (`ModelProvider` required methods и отсутствие `PartialEq` у `RolloutItem`); helpers адаптированы к upstream 0.145, после чего tests прошли. Первый Disabled fixture ошибочно выключал только feature flag при `agents_enabled=true`; fixture исправлен на реальный `Disabled` config. Эти промежуточные failures не являлись production regressions.

## Historical 0.145 package/workspace evidence

Эти команды выполнялись для первого candidate `a5e21107643d` до rollback и follow-up adaptations. Они помогают классифицировать ambient failures, но не выдаются за повторный current full-suite run.

| Команда | Исторический результат и классификация |
| --- | --- |
| Чистый baseline: `just test -p codex-core` | exit 100; 2999 total, 2918 passed, 81 failed: missing test binaries, timeouts и ambient config/git/realtime assertions |
| Candidate: `just test -p codex-core` | exit 100; 3024 total, 3004 passed, 20 failed; feature-specific regressions прошли, остаток соответствовал baseline/environment classes |
| `just test -p codex-app-server` | exit 100; 967 run, 966 passed, 1 failed, 1 skipped; bundled Ubuntu 24.04 zsh требовал `GLIBC_2.38` на Ubuntu 22.04 |
| `just test -p codex-tui` | exit 100; 3221 executed, 3189 passed, 32 failed, 4 skipped; failures относились к существующим version/status snapshots |
| `just test` | exit 100; 12672 total, 12612 passed, 60 failed; группы: app-server-transport 1, app-server 1, core 20, core-skills 2, mcp-server 3, secrets 1, TUI 32 |

Non-zero package/workspace results не обозначаются как PASS. Полный current workspace suite не перезапускается без отдельного разрешения; focused gates являются текущим test evidence, а release build фиксируется отдельно.

## Current lint, format и build

| Команда | Результат |
| --- | --- |
| `just fix -p codex-core` | current exit 0; первый запуск обнаружил `expect_used` warning в новом V1 counter decrement, после замены на explicit invariant assertion повторный запуск завершился без warnings |
| `just fix -p codex-app-server` | exit 0 для предшествующего candidate; cross-runtime follow-up не меняет crate |
| `just fix -p codex-tui` | exit 0 для предшествующего candidate; cross-runtime follow-up не меняет crate |
| `just fmt` / `just fmt-check` | current exit 0 |
| `git diff --check HEAD` | current exit 0 |
| `scripts/codex-fork-build.sh` | recorded exit 0; release build завершён за 7m27s, binary built and stripped; единственный `unused_mut` warning относится к неизменённому `app-server/src/lib.rs:1290` вне feature diff; локальный release binary больше не доступен для повторной проверки |
| Version/hash/stripping | recorded `codex-cli 0.145.0`; SHA-256 `6823de437245e6caa9a265d7b51a0c3b40cca495ee47691bad4f528ffc845304`; BuildID `3e4692db48dfb21ba38d5a32e6a8b128b8de70eb`; при исходной проверке `file` сообщил `stripped`, `.debug_info`/`.symtab` отсутствовали |

Текущий `just fmt` не оставил formatter-only изменений вне feature scope. Исторический binary первого candidate имел SHA-256 `d3b818cc86f16155a3221cdc87fa199bde87f70dbf5a507be04b8b38b49ed3cc` и BuildID `6b07cfb793af177e5a6681b8e159092fd6677e30`, но он не является artifact текущего повторного применения.

## Audit findings

Первичные повторные аудиты нашли и потребовали исправить потерю `ignore_user_and_project_exec_policy_rules` при role reload, semantic suffix после structural mode fragment, отсутствующие provider/Guardian и Disabled/unresolved cases, отсутствие explicit bearer-UUID regression, порядок `Closed` после shutdown, stale provenance и завышенную atomic subtree/role-replay документацию. Current candidate:

- сохраняет exec-policy ignore flag и полный active-turn `Permissions` на projected fresh/cold paths;
- сохраняет semantic suffix top-level и compacted history;
- проверяет provider/Guardian, Disabled/unresolved, cross-subtree bearer behavior и отдельный root rejection;
- записывает live root `Closed` до shutdown с native warning-only graph policy;
- сохраняет cold-resume compatibility для удалённой mutable role;
- документирует per-ID hardening и native best-effort traversal без atomic subtree claims.

Для текущего cross-runtime follow-up architecture, propagation, contract/runtime и final reviewer re-audits завершились `PASS` с 0 High/Medium. Tests/docs audit подтвердил отсутствие поведенческих High/Medium и оставил только два procedural findings — stale ledger и pending build; оба закрыты current lint/format/build evidence выше. Reviewer отдельно проверил cross-source duplicate ownership и снял первоначальный Medium: частичное закрытие до controlled fail-fast не нарушает явно non-transactional contract, а persisted preflight намеренно предотвращает только self-masking reverse edge. Историческое утверждение предыдущего candidate про 51 staged path неприменимо к текущему незакоммиченному дереву.

## Known gaps

- Cross-platform CI не запускался.
- Фактическая установка fork binary и restart app-server не входят в эту задачу; build разрешён как verification, commit/push не выполняются без отдельной текущей команды.
- Ignored `codex-rs/target/` и `scripts/.codex-build-hash` являются локальными build artifacts и не коммитятся; для текущего dirty candidate build script записывает составной workspace hash, а не commit SHA.
