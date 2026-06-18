# Interactive collaboration mode

## Feature passport

- Code name: `interactive-collaboration-mode`
- Status: реализовано локально на `fork/140` как upstream-shaped `ModeKind::Interactive`; исторические коммиты `fork/101` остаются evidence для исходного fork contract.
- Goal: добавить отдельный collaboration mode, в котором пользователь может интерактивно вмешиваться в ход работы.
- Scope in: native mode preset, `/interactive` slash command, TUI-visible mode list/cycle, footer indication, app-server protocol/schema value, `request_user_input` availability.
- Scope out: замена Plan Mode, изменение MultiAgentV2 runtime semantics, отдельный OpenClaude-style task/workflow storage.

## Как работает для пользователя

Пользователь может переключиться в `Interactive` через `/interactive` или обычный native collaboration-mode flow. В footer отображается `Interactive mode`, а `request_user_input` доступен в этом режиме без включения Default-mode feature flag. Польза: пользователю проще взаимодействовать с agent, когда следующий шаг зависит от выбора из предложенных вариантов, при этом режим остаётся частью native collaboration-mode source of truth.

## Branches and commits

| Branch | Baseline | Commit | Date | Subject | Роль |
| --- | --- | --- | --- | --- | --- |
| `fork/101` | `rust-v0.101.0` | `03491cb946` | 2026-02-20 | `feat: add interactive collaboration mode and command support` | Основной функционал interactive collaboration mode. |
| `fork/101` | `rust-v0.101.0` | `305bc931c4` | 2026-02-21 | `tui: show interactive mode indicator in footer cycle` | Видимый индикатор режима в TUI footer. |

## Implementation notes

Исторические коммиты меняли app-server protocol schemas/types, collaboration mode presets, core request handling, TUI slash commands, footer и tests. Текущий `fork/140` перенос выполнен через аналогичные native surfaces: `ModeKind`, generated app-server schema, model-manager presets, TUI slash/footer/cycle projection and request-user-input mode gating.

## Native coverage in rust-v0.140.0

Status: `implemented locally`. Upstream `rust-v0.140.0` имел collaboration mode infrastructure, `ModeKind::{Default, Plan}`, `collaborationMode/list`, `/plan`, Shift+Tab cycle и Plan footer indicator. Текущий fork добавляет отдельный `ModeKind::Interactive` в этот же source of truth, включает его в `TUI_VISIBLE_COLLABORATION_MODES`, генерирует protocol/schema значение `interactive`, добавляет builtin preset, `/interactive`, footer indicator и `request_user_input` availability для `Interactive`.

## Porting/current-state notes

Porting decision for `fork/140`: interactive mode является отдельным fork contract, но реализован не как TUI-only switch, а как upstream-shaped extension of native collaboration-mode source of truth. Это сохраняет Plan Mode как отдельный режим и не меняет MAv2 execution semantics.

## Project dossier

Текущий проектный dossier находится в `docs/fork/projects/interactive-collaboration-mode/`:

- `README.md`: current status, references and implementation surfaces;
- `design.md`: native source map, candidate implementation paths and invariants;
- `verification.md`: current-state guard, required checks and scenario matrix.

## Doc changelog

- 2026-06-18: создан project dossier для текущего `fork/140` состояния; отдельный `Interactive` mode не считался реализованным, пока не выбран native implementation path.
- 2026-06-18: выбран и реализован native path `ModeKind::Interactive`: app-server schema, presets, TUI slash/footer/cycle projection and request-user-input mode gating обновлены и покрыты focused checks.
