# Spawn agent cwd

## Feature passport

- Code name: `spawn-agent-cwd`
- Status: переносимая fork-возможность.
- Goal: позволить явно задавать рабочий каталог sub-agent и безопасно проверять этот каталог.
- Scope in: `spawn_agent` cwd argument/contract, permission checks, TUI/thread routing interactions, docs.
- Scope out: общие sandbox policies; они описаны в `subagent-policy-and-ownership`.

## Как работает для пользователя

При создании sub-agent можно указать, в каком каталоге он должен работать. Польза: удобно делегировать задачи по разным проектам/worktrees из одного основного thread, а созданный в другой рабочей папке agent использует нативный функционал в директории, где был создан, что снижает риск внесения изменений в некорректное место.

## Branches and commits

| Branch | Baseline | Commit | Date | Subject | Роль |
| --- | --- | --- | --- | --- | --- |
| `fork/114` | `rust-v0.114.0` | `eedb7deda0` | 2026-03-13 | `feat(spawn_agent): add cwd override contract` | Первый explicit cwd override contract. |
| `fork/118` | `rust-v0.118.0` | `79df90762c` | 2026-04-02 | `Add spawn_agent cwd support` | Перенос cwd support в более позднюю architecture. |
| `fork/130` | `rust-v0.130.0` | `03c5491730` | 2026-05-12 | `feat: support explicit subagent cwd` | Новый explicit cwd path для sub-agent runtime. |
| `fork/130` | `rust-v0.130.0` | `5123475402` | 2026-05-13 | `Implement subagent cwd permissions` | Permission checks и TUI/thread routing updates. |
| `fork/130` | `rust-v0.130.0` | `022e7b0889` | 2026-05-14 | `docs(fork): update subagent cwd contract` | Документирование contract. |
| `chrome_plugin` | `rust-v0.130.0` lineage | `03c5491730` | 2026-05-12 | `feat: support explicit subagent cwd` | Та же возможность в chrome-plugin lineage. |
| `chrome_plugin` | `rust-v0.130.0` lineage | `5123475402` | 2026-05-13 | `Implement subagent cwd permissions` | Та же permission часть в chrome-plugin lineage. |

## Implementation notes

Затрагивались multi-agent handlers/spec, common cwd resolution, config/session/thread manager, protocol projections, TUI routing/replay filtering, tests and feature/project/research docs. В позднем 0.130 contract explicit cwd incompatible with forked history; invalid cwd fails fast; workspace-write permissions rebased to child cwd, read-only stays read-only, non-projectable permission profiles fail fast, child cwd is not auto-trusted, resume fails if stored child cwd cannot be restored.

## Native coverage in rust-v0.140.0

Status: `partial`. Native release имеет `TurnEnvironmentSelection`, `TurnStartParams.environments`, legacy `cwd`, runtime workspace roots, session cwd/environment updates, persisted thread `cwd`, and spawned agents inherit runtime-owned turn context (`config.cwd = turn.cwd`). Не хватает `spawn_agent.cwd` argument/schema, explicit child cwd validation before spawn, fail-fast invalid/non-projectable cwd behavior, spawn-specific permission rebase contract, no auto-trust guarantee and resume invariant for stored child cwd.

## Porting/current-state notes

При переносе на `fork/140` нужно учитывать upstream PathUri/environment path changes и проверять cwd через native environment/sandbox path contract, а не через ad hoc string path.

## Fork/140 implementation status

Первая итерация добавляет `spawn_agent.cwd` в MAv2 tool schema/handler, валидирует только existing absolute directory внутри текущих workspace roots, дополнительно проверяет canonical containment после symlink resolution, применяет cwd через native child `Config`/`TurnEnvironmentSelections`, сохраняет исходный permission profile/workspace roots без auto-trust и fail-fast возвращает ошибку модели для cwd вне workspace roots, symlink escape, nonexistent path или file path. Не реализовано в этой итерации: отдельный app-server RPC surface для cwd, replay-time restore failure policy для устаревшего cwd и расширенная TUI-индикация child cwd.

## Doc changelog

- 2026-06-17: Зафиксирован fork/140 first iteration: MAv2 `spawn_agent.cwd`, workspace-root and existing-directory validation, no permission widening, known resume/TUI gaps.
- 2026-06-18: Focused core verification passed for MAv2 schema visibility, positive cwd application and invalid/nonexistent cwd fail-fast behavior.
- 2026-06-18: Symlink escape guard added: cwd inside workspace lexically but resolving outside workspace roots is rejected before spawn; focused cwd tests passed 4/4.
- 2026-06-18: Added explicit file-path cwd regression test so an existing non-directory inside workspace roots is covered by the same fail-fast contract.
