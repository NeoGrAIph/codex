# Thread notes

## Feature passport

- Code name: `thread-notes`
- Status: переносимая и неоднократно усиленная fork-возможность.
- Goal: дать thread/sub-agent короткую устойчивую заметку, видимую в runtime, TUI и app-server surfaces.
- Scope in: metadata-only notes, persistence, resume/restart reconstruction, TUI/app-server rendering, `set_thread_note`.
- Scope out: role templates и cwd.

## Как работает для пользователя

Agent может закрепить короткую заметку за thread/sub-agent: назначение, состояние или следующий шаг. Польза: агент-оркестратор может быстро понять назначение и состояние каждого agent без перечитывания transcript, в том числе после resume/restart.

## Branches and commits

| Branch | Baseline | Commit | Date | Subject | Роль |
| --- | --- | --- | --- | --- | --- |
| `fork/105` | `rust-v0.105.0` | `935e4739d5` | 2026-02-26 | `feat(threadspawn): harden protocol contract and metadata propagation` | Подготовка metadata propagation для threadspawn. |
| `fork/105` | `rust-v0.105.0` | `b60b94ab2f` | 2026-02-27 | `feat(thread-note): persist and expose thread notes across runtime` | Основная реализация persistence/protocol/runtime. |
| `fork/106` | `rust-v0.106.0` | `b8945422af` | 2026-02-27 | `feat(thread-note): port thread note contract to fork/106` | Перенос thread-note на 0.106. |
| `fork/107` | `rust-v0.107.0` | `ddc72fd801` | 2026-03-03 | `feat(thread-note): port thread note contract to fork/106` | Присутствует в 0.107 lineage как перенос contract. |
| `fork/107` | `rust-v0.107.0` | `2f37b8a840` | 2026-03-03 | `fix(tui): include thread_note in session helper` | TUI helper начинает учитывать note. |
| `fork/111` | `rust-v0.111.0` | `e542d0586e` | 2026-03-09 | `feat(thread-note): add thread note contract and persistence` | Обновленная реализация contract/persistence. |
| `fork/114` | `rust-v0.114.0` | `356c6df2e5` | 2026-03-13 | `Реализован restart-safe thread_note, обновлен TUI note rendering и перенесены fork docs` | Restart-safe note и TUI rendering. |
| `fork/116` | `rust-v0.116.0` | `6aa62ee56e` | 2026-03-20 | `feat: add restart-safe thread note support` | Расширенный restart-safe support. |
| `fork/116` | `rust-v0.116.0` | `d27a94dc0e` | 2026-03-21 | `fix: surface thread notes on resume` | Заметки видны после resume. |
| `fork/118` | `rust-v0.118.0` | `35187c0529` | 2026-04-02 | `feat(thread-note): add metadata-only thread note support` | Metadata-only перенос. |
| `fork/118` | `rust-v0.118.0` | `722b52cdf7` | 2026-04-04 | `Add agents overlay and restore resume thread notes` | Восстановление notes в связке с overlay/resume. |
| `fork/130` | `rust-v0.130.0` | `163e551cdb` | 2026-05-15 | `feat(thread-note): add metadata-only thread notes` | Перенос на 0.130 architecture. |
| `chrome_plugin` | `rust-v0.130.0` lineage | `163e551cdb` | 2026-05-15 | `feat(thread-note): add metadata-only thread notes` | Та же возможность в chrome-plugin lineage. |

## Implementation notes

Затрагивались protocol types/schemas, app-server request/notification handling, rollout/session metadata, thread-store/state, multi-agent tool handlers, TUI and app-server TUI rendering, tests and docs. В ранних ветках note мог попадать в runtime environment (`CODEX_THREAD_NOTE`), но поздний contract стал metadata-only: note не должен попадать в model/developer/environment context. В `fork/130` note нормализуется как `Назначение: ... | Компетенции: ...`, имеет лимит 500 символов и виден через `list_agents`/legacy `wait_agent`, app-server `Thread.threadNote`, `CollabAgentState.threadNote` и `thread/note/updated`.

## Native coverage in rust-v0.140.0

Status: `partial`, но собственно note surface отсутствует. Native release уже хранит thread metadata (`parent_thread_id`, `cwd`, `thread_source`, `agent_nickname`, `agent_role`, `agent_path`, permissions), но не имеет `thread_note`/`threadNote`, `set_thread_note`, `Thread.threadNote`, `CollabAgentState.threadNote`, `thread/note/updated`, TUI/list/wait projection and metadata-only 500-char normalized note contract.

## Porting/current-state notes

В новых ветках нужно проверять, есть ли upstream-native thread metadata/note surface. Если есть, fork-note должен встраиваться туда. Старые sessions должны оставаться читаемыми при отсутствии note.
