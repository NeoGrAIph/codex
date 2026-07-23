# Verification ledger

Этот ledger разделяет фактические проверки текущего повторного применения в `fork/145` и исторические результаты первого 0.145 candidate. Evidence из `fork/144.4` не считается проверкой текущего релиза.

## Baseline и provenance

| Поле | Значение |
| --- | --- |
| Upstream release | `rust-v0.145.0^{}` / `25af12f7e61572b0bc18ddb1008be543b91519b0` |
| Fork baseline | `72c43db8d4a41ea1a23dc4b664cdbec52e85f683` |
| Historical feature contract | `2f2e7e294975881c1c83e3c46fba3a0096481890` на `fork/144.4` |
| First 0.145 implementation | `a5e21107643db7c94d9409b4c11cbee551a7dfe7` |
| Workflow rollback | `10713a4a40404db487a2e42aa245cbfe8767cf9a`; выполнен по явной пользовательской команде при уточнении смысла feature, а не как локально зафиксированный defect verdict |
| Current candidate | повторное применение `a5e21107643d` с release-specific adaptations прямо в `fork/145`; focused verification, domain audits, final integration audit и current release build завершены, delivery commit/push identity фиксируется в итоговом handoff |

## Required verification matrix

| Surface | Required evidence | Current status |
| --- | --- | --- |
| Planner/schema | Dual family, exact V1 schema/exposure, depth/provider/Guardian eligibility | PASS: final `projected_v1_` aggregate 21/21 |
| Collisions | Configured/dynamic/MCP owner, extension suppression, per-turn warning | PASS targeted |
| Runtime selection | Exact V1 before accounting; preview/turn/resume remain V1 | PASS: `exact_v1_` и focused selectors |
| Fresh/fork | Legacy/Paginated, pathless UUID, sanitized semantic context | PASS targeted, включая semantic suffix |
| Authority | Named/managed profile, environments/roots, shared ignored exec-policy rules | PASS targeted; shared role reload сохраняет ignore bit, fresh role не расширяет authority, removed persisted role не блокирует cold resume |
| Lifecycle | Live/persisted V1 success; V2/Disabled/unresolved fail before effects | PASS targeted, включая bound runtime, cross-root bearer UUID, `Closed` before shutdown и документированный sequential first-error close |
| Persistence | Target и доступные Open V1 descendants, missing-version fallback, native best-effort failure policy | PASS targeted; transactional subtree semantics намеренно не заявляются |
| App-server | list/read/history and `canAcceptDirectInput` V1 true/V2 false | PASS: focused paginated V2 app-server E2E |
| TUI/events | UUID fallback and canonical V1 items without V2 activity | PASS: focused UUID fallback; event assertions входят в core regressions |
| Public artifacts | No config/protocol/persistence/schema/dependency delta | PASS by diff inspection; generated artifacts не требуются |
| Lint/format | Scoped `just fix`, `just fmt`, `just fmt-check`, `git diff --check` | PASS для окончательного Rust diff |
| Build | Current stripped release binary, version, SHA-256, BuildID и installer dry-run | PASS для current candidate; build hash повторно актуализируется после ledger/commit |
| Release | Cross-platform CI | NOT RUN; отдельный release gate |

## Current focused commands

Проверки ниже выполнены 2026-07-23 в основном checkout `fork/145` после `git cherry-pick --no-commit a5e21107643d` и соответствующих adaptations.

| Команда | Результат |
| --- | --- |
| `just test -p codex-core projected_v1_` | final exit 0; 21/21 PASS; первый pre-adaptation aggregate был 16/16 |
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

Non-zero package/workspace results не обозначаются как PASS. Полный current workspace suite не перезапускается без отдельного разрешения; focused gates являются текущим test evidence, а release build фиксируется отдельно после выполнения pending build gate.

## Current lint, format и build

| Команда | Результат |
| --- | --- |
| `just fix -p codex-core` | exit 0; после точечного архитектурно обоснованного `#[allow(clippy::too_many_arguments)]` на cross-owner validation boundary повторный запуск завершился без warnings |
| `just fix -p codex-app-server` | exit 0 |
| `just fix -p codex-tui` | exit 0 |
| `just fmt` / `just fmt-check` | exit 0 |
| `git diff --check HEAD` | exit 0 после final formatting |
| `scripts/codex-fork-build.sh` | exit 0; первоначальная release-сборка завершена за 7m19s, после обновления ledger выполнен успешный инкрементальный rebuild и workspace hash обновлён; окончательный clean hash после commit должен совпасть с commit SHA. Единственный `unused_mut` warning относится к неизменённому `app-server/src/lib.rs:1290` вне feature diff |
| Version/hash/stripping/installer dry-run | `codex-cli 0.145.0`; SHA-256 `99be8badcb550292dc4666a39f5d188f66b1ad85fffcb6eec7ca1cee7fd0f957`; BuildID `0cbebeb94f750c6afbc92eb468661af813f6c959`; `file` сообщает `stripped`, `.debug_info`/`.symtab` отсутствуют; `scripts/2_codex-fork-install-binary.sh --dry-run` exit 0 и не изменил файлы |

Обязательный `just fmt` также нормализовал корневой `justfile` через `just --unstable --fmt`; это formatter-only churn, а не функциональная часть feature. Исторический binary первого candidate имел SHA-256 `d3b818cc86f16155a3221cdc87fa199bde87f70dbf5a507be04b8b38b49ed3cc` и BuildID `6b07cfb793af177e5a6681b8e159092fd6677e30`, но он не является artifact текущего повторного применения.

## Audit findings

Первичные повторные аудиты нашли и потребовали исправить потерю `ignore_user_and_project_exec_policy_rules` при role reload, semantic suffix после structural mode fragment, отсутствующие provider/Guardian и Disabled/unresolved cases, отсутствие explicit bearer-UUID regression, порядок `Closed` после shutdown, stale provenance и завышенную atomic subtree/role-replay документацию. Current candidate:

- сохраняет exec-policy ignore flag и полный active-turn `Permissions` на projected fresh/cold paths;
- сохраняет semantic suffix top-level и compacted history;
- проверяет provider/Guardian, Disabled/unresolved и cross-root bearer behavior;
- записывает live root `Closed` до shutdown с native warning-only graph policy;
- сохраняет cold-resume compatibility для удалённой mutable role;
- документирует per-ID hardening и native best-effort traversal без atomic subtree claims.

Final architecture, propagation, contract, security, runtime-persistence, tests-regression и docs-evidence re-audits завершились с `PASS` и 0 High/Medium findings. Final integration re-audit после formatting, полного staging и current build завершился с `PASS`, 0 High/Medium/Low: 51 staged path, no unstaged/untracked/unmerged state, cached diff clean, stored workspace hash совпадает с пересчитанным, binary identity и stripping подтверждены.

## Known gaps

- Cross-platform CI не запускался.
- Фактическая установка fork binary и restart app-server не входят в эту задачу; разрешены build, commit и push, но не runtime replacement.
- Ignored `codex-rs/target/` и `scripts/.codex-build-hash` являются локальными build artifacts и не коммитятся; после commit build script запускается повторно, чтобы recorded hash стал SHA чистого committed tree.
