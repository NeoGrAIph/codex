# Interactive collaboration mode

## Feature passport

- Code name: `interactive-collaboration-mode`
- Status: историческая fork-возможность на `fork/101`.
- Goal: добавить отдельный collaboration mode, в котором пользователь может интерактивно вмешиваться в ход работы.
- Scope in: mode preset, slash-command support, event notifications, TUI indication.
- Scope out: поздний upstream Plan Mode и MultiAgentV2; это отдельные поколения.

## Как работает для пользователя

Пользователь работает в режиме интерактивного взаимодействия с явной индикацией в TUI. Польза: пользователю проще взаимодействовать с agent ввиду возможности интерактивного выбора из предложенных вариантов.

## Branches and commits

| Branch | Baseline | Commit | Date | Subject | Роль |
| --- | --- | --- | --- | --- | --- |
| `fork/101` | `rust-v0.101.0` | `03491cb946` | 2026-02-20 | `feat: add interactive collaboration mode and command support` | Основной функционал interactive collaboration mode. |
| `fork/101` | `rust-v0.101.0` | `305bc931c4` | 2026-02-21 | `tui: show interactive mode indicator in footer cycle` | Видимый индикатор режима в TUI footer. |

## Implementation notes

Коммиты меняли app-server protocol schemas/types, collaboration mode presets, core request handling, TUI slash commands, footer и tests.

## Native coverage in rust-v0.140.0

Status: `partial`. Native release имеет collaboration mode infrastructure, `ModeKind::{Default, Plan}`, `collaborationMode/list`, `/plan`, Shift+Tab cycle и Plan footer indicator. Отдельного `Interactive` collaboration mode нет: TUI-visible modes ограничены `Default` и `Plan`, а `PairProgramming`/`Execute` hidden/skipped или aliases to `Default`.

## Porting/current-state notes

Эта возможность может конфликтовать с более поздними upstream collaboration modes/Plan Mode. При переносе нужно сначала выяснить, является ли interactive mode отдельным fork contract или должен быть выражен через upstream collaboration mode source of truth.
