# Thread notes

## Feature passport

- Code name: `thread-notes`
- Status: переносимая и неоднократно усиленная fork-возможность.
- Goal: дать thread/sub-agent короткую устойчивую заметку, видимую в runtime, TUI и app-server surfaces.
- Scope in: metadata-only notes, persistence, resume/restart reconstruction, TUI workbench rendering, app-server source metadata projection, post-spawn app-server updates through `thread/metadata/update.threadNote`, and model-visible MAv2 `set_thread_note` set/clear.
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

Status: `partial, expanded locally`. Native release уже хранит thread metadata (`parent_thread_id`, `cwd`, `thread_source`, `agent_nickname`, `agent_role`, `agent_path`, permissions), но не имеет fork-level note contract: `thread_note`/`threadNote`, `set_thread_note`, `CollabAgentState.threadNote`, `thread/note/updated`, TUI/list/wait projection and metadata-only 500-char normalized note contract. Текущий `fork/140` добавляет metadata-only `thread_note` substrate, exposes it through native MAv2 `list_agents`, MAv2 `wait_agent` visible-agent snapshot, app-server `Thread.threadNote`, app-server `CollabAgentState.threadNote`, generated schemas and `/agent` workbench selected detail, lets app-server clients update or clear the note through the existing native `thread/metadata/update.threadNote` contract, and lets the orchestrating model update/clear visible spawned sub-agent notes through MAv2 `set_thread_note`. SQLite migration or old `thread_note_index.jsonl` port are not required.

## Porting/current-state notes

В новых ветках нужно проверять, есть ли upstream-native thread metadata/note surface. Если есть, fork-note должен встраиваться туда. Старые sessions должны оставаться читаемыми при отсутствии note.

## Fork/140 implementation status

Первая итерация добавляет metadata-only `thread_note` в `SubAgentSource::ThreadSpawn`, `SessionMeta`, thread-store read model/patch и live `AgentMetadata`, нормализует `spawn_agent.thread_note` до непустой строки максимум 500 символов и показывает note через MAv2 `list_agents`. MAv2 `wait_agent` returns the same visible-agent metadata snapshot when it completes or times out; it does not expose mailbox content or final answers. Старые sessions остаются читаемыми через `serde(default)`. Generated app-server protocol schemas включают `SubAgentSource.thread_note`, top-level `Thread.threadNote`, `CollabAgentState.threadNote` and `thread/note/updated`: первое поле сохраняет source metadata, второе даёт клиентам удобную read projection без отдельного storage, третье переносит note в существующие collab tool-call notifications/history для spawn/send/wait/close/resume, четвёртое сообщает app-server clients о post-spawn set/clear. Текущий `/agent` workbench гидрирует `thread_note` из `Thread.threadNote` with fallback to existing `Thread.source` / `SessionSource::SubAgent(ThreadSpawn)` and renders it as bounded selected-detail `Note:` recovery anchor; live `thread/note/updated` обновляет тот же picker cache without thread/read. Post-spawn app-server update реализован как расширение native `thread/metadata/update`: поле `threadNote` uses omit/`null`/string semantics, updates rollout `SessionMeta.thread_note` and the nested `ThreadSpawn.thread_note`, then reuses existing metadata read/projection paths without sqlite column migration. MAv2 `set_thread_note` uses the same normalization and `ThreadMetadataPatch` path, updates only visible spawned sub-agents, rejects root/non-sub-agent targets, and refreshes live `AgentMetadata` so `list_agents`/`wait_agent` see the new note immediately.

## Doc changelog

- 2026-06-17: Зафиксирован fork/140 metadata-only substrate: spawn-time `thread_note`, rollout/session metadata, live list projection, generated app-server protocol schema projection, старые rollout records без note читаются как `None`.
- 2026-06-18: Focused verification passed for spawn-time note propagation, over-500-character fail-fast validation, thread-store resume reconstruction and generated app-server protocol schema sync.
- 2026-06-18: Синхронизирован contract с текущим `/agent` workbench: note теперь отображается в selected-detail `Inspect` block через native `Thread.source` hydration; post-spawn update, `set_thread_note` and wait-agent projection оставались follow-up work на этом checkpoint. SQLite column/migration не является частью metadata-only contract.
- 2026-06-18: Добавлен native post-spawn app-server update path: `thread/metadata/update.threadNote` updates/clears metadata-only notes through rollout `SessionMeta` and `Thread.source` projection without sqlite migration; at that checkpoint `set_thread_note` model tool and notification surfaces remained future work.
- 2026-06-18: Добавлена top-level app-server read projection `Thread.threadNote` generated in JSON/TypeScript schemas. It is derived from the same metadata source and does not add sqlite storage or a parallel note index.
- 2026-06-18: MAv2 `wait_agent` output now includes the same visible-agent snapshot shape as `list_agents`, including `thread_note`. This is metadata-only status projection; mailbox/final-answer content remains excluded.
- 2026-06-18: Added MAv2 model-visible `set_thread_note` set/clear through native `ThreadMetadataPatch` and live `AgentMetadata` refresh. Root/non-sub-agent targets are rejected; no sqlite migration or `thread_note_index.jsonl` port was added.
- 2026-06-18: Added `CollabAgentState.threadNote` through existing app-server collab tool-call event/history projections. Spawn/send/wait/close/resume states can carry the current metadata note without adding a dedicated notification or sqlite storage.
- 2026-06-18: Added dedicated app-server `thread/note/updated` notification emitted after successful `thread/metadata/update.threadNote` set/clear and consumed by TUI `/agent` cache without an extra `thread/read`.
