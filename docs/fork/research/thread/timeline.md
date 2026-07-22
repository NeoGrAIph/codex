# Эволюция app-server `thread/*` RPC

Статус исследования: покрыты stable-релизы `rust-v0.86.0`-`rust-v0.145.0`. Scope намеренно начинается с `rust-v0.86.0`; более ранняя история app-server и первичный v2 `thread/*` bootstrap не восстанавливаются в этом timeline.

## Поколения

- Базовая thread history API: `thread/list`, `thread/read`, archived filtering, unarchive and thread ordering.
- Live thread lifecycle: start/resume/fork, websocket subscriptions, status notifications, `thread/unsubscribe`, non-blocking start and listener retention.
- Metadata и projections thread: `Thread.status`, `Thread.ephemeral`, names, git info, nickname/role, `sessionId`, `parentThreadId`, неизменяемый `historyMode`, permissions, reviewer/originator, runtime roots и environment settings.
- Хранение через ThreadStore: local/remote store interface, миграция ThreadStore для list/read/resume/fork/archive/history/metadata, paged turns/items, сохранение canonical `TurnItem` для paginated threads и ограниченные legacy history views.
- Advanced RPC surfaces: `thread/compact/start`, `thread/shellCommand`, `thread/rollback`, `thread/increment_elicitation`, `thread/decrement_elicitation`, `thread/approveGuardianDeniedAction`, `thread/goal/*`, `thread/settings/update`, `thread/search`, `thread/delete`, background terminals and `thread/realtime/*`.

## `rust-v0.86.0`

- Release commit: `18057fdc30b10168ab18e8c1bb86c7a0a7772191`
- Дата: `2026-01-15T16:46:06-08:00`
- Результат triage: прямых изменений app-server `thread/*` contract в диапазоне `rust-v0.85.0^{}..rust-v0.86.0^{}` не найдено.
- Evidence: `EVID-086-no-change`.

## `rust-v0.87.0`

- Release commit: `8e8c884b7534d08b8cabccd26f37a8302b4bc4fd`
- Дата: `2026-01-16T16:04:05+01:00`

### `1fa8350ae7` - `Add text element metadata to protocol, app server, and core (#9331)`

- Дата: `2026-01-15T17:26:41-08:00`; PR: `#9331`.
- Developer info: добавляет text element metadata в protocol/app-server/core так, чтобы app-server thread items могли round-trip richer user-message metadata при rebuild/resume history.
- User info: клиенты app-server получают больше metadata для UI-аннотаций в thread history.
- Compatibility/risks: это изменение формы item/history payload; клиенты должны игнорировать незнакомые metadata поля.
- Evidence: `EVID-087-text-metadata`.

## `rust-v0.88.0`

- Release commit: `149625a4f983e1b3c8530abef1c863689d98a1cc`
- Дата: `2026-01-21T10:43:17-08:00`

### `f1653dd4d3` - `feat(app-server, core): return threads by created_at or updated_at (#9247)`

- Дата: `2026-01-16T20:58:55Z`; PR: `#9247`.
- Developer info: меняет ordering semantics для app-server thread listing: list может возвращать threads по `created_at` или `updated_at`.
- User info: history UI может сортировать thread list по созданию или последней активности.
- Compatibility/risks: list ordering становится частью client expectations; future filters must preserve deterministic ordering.
- Evidence: `EVID-088-list-ordering`.

## `rust-v0.89.0`

- Release commit: `605d43719efa7f62160904d2dcecc46a64bcd32e`
- Дата: `2026-01-22T12:57:37-08:00`

### `80240b3b67` - `feat(app-server): thread/read API (#9569)`

- Дата: `2026-01-22T12:22:01-08:00`; PR: `#9569`.
- Developer info: добавляет `thread/read` для чтения persisted thread без resume.
- User info: клиенты могут открывать сохраненный thread/history preview без materializing live session.
- Compatibility/risks: read-only view должен не менять loaded/subscription state.
- Evidence: `EVID-089-read-archive`.

### `733cb68496` - `feat(app-server): support archived threads in thread/list (#9571)`

- Дата: `2026-01-22T12:22:36-08:00`; PR: `#9571`.
- Developer info: расширяет `thread/list` archived filter.
- User info: archived history становится discoverable через app-server list вместо отдельного filesystem path.
- Compatibility/risks: archived default/filter semantics должны быть явными, чтобы UI не смешивал active и archived sessions.
- Evidence: `EVID-089-read-archive`.

## `rust-v0.90.0`

- Release commit: `b4e230f8de8f71d08f48c469443ed61a9f365af3`
- Дата: `2026-01-25T16:56:24+01:00`

### `83775f4df1` - `feat: ephemeral threads (#9765)`

- Дата: `2026-01-24T14:57:40Z`; PR: `#9765`.
- Developer info: добавляет ephemeral thread support в app-server protocol/thread manager paths.
- User info: clients can create temporary in-memory threads where normal persisted rollout behavior is intentionally skipped.
- Compatibility/risks: persistence/resume differs for ephemeral threads; APIs must make durability explicit.
- Evidence: `EVID-090-ephemeral`.

## `rust-v0.91.0`

- Release commit: `3684bc646e0f1123e76a90f03cfb1111358f1aa1`
- Дата: `2026-01-25T17:52:47+01:00`
- Результат triage: прямых app-server `thread/*` изменений не найдено.
- Evidence: `EVID-091-no-change`.

## `rust-v0.92.0`

- Release commit: `a09055074e082d70e8b92795b0cec1e969a6aaf9`
- Дата: `2026-01-27T10:32:31Z`

### `62266b13f8` - `Add thread/unarchive to restore archived rollouts (#9843)`

- Дата: `2026-01-26T11:24:36-08:00`; PR: `#9843`.
- Developer info: вводит `thread/unarchive` RPC для восстановления archived rollout.
- User info: client can restore archived threads back into active session list without manual file movement.
- Compatibility/risks: unarchive must update list visibility and avoid corrupting active/archive storage.
- Evidence: `EVID-092-list-unarchive`.

### `247fb2de64` - `[app-server] feat: add filtering on thread list (#9897)`

- Дата: `2026-01-26T21:54:19Z`; PR: `#9897`.
- Developer info: расширяет `thread/list` filters.
- User info: large history UIs can browse narrower thread sets.
- Compatibility/risks: filter defaults must remain stable for existing clients.
- Evidence: `EVID-092-list-unarchive`.

## `rust-v0.93.0`

- Release commit: `d86cf538f5e7d210ebb7a3493718aaaff40146da`
- Дата: `2026-01-30T23:17:24-07:00`
- Результат triage: direct `thread/*` contract entry не найден; release contains adjacent SQLite/log/thread-id infrastructure, but no app-server thread RPC shape change selected for this timeline.
- Evidence: `EVID-093-no-change`.

## `rust-v0.94.0`

- Release commit: `dce99bc2595e58acc4d88a324f3ccab05cf5cd7d`
- Дата: `2026-02-02T09:44:08-08:00`

### `d3514bbdd2` - `Bump thread updated_at on unarchive to refresh sidebar ordering (#10280)`

- Дата: `2026-02-01T12:53:47-08:00`; PR: `#10280`.
- Developer info: updates `updated_at` on unarchive so `thread/list` ordering surfaces restored threads.
- User info: restored thread appears fresh in sidebar/history list.
- Compatibility/risks: unarchive now mutates ordering metadata; clients relying on strict historical `updated_at` need to treat this as restore activity.
- Evidence: `EVID-094-unarchive-order`.

## `rust-v0.95.0`

- Release commit: `12dbb76c812afaff8daf1f3a5acf1d5ef4a75cb1`
- Дата: `2026-02-03T20:07:30-08:00`
- Результат triage: no direct app-server `thread/*` RPC/schema change selected. `CODEX_THREAD_ID` is runtime environment context, not app-server thread namespace.
- Evidence: `EVID-095-no-change`.

## `rust-v0.96.0`

- Release commit: `2572f96fafb3156f79523bfbe2bc84f13e382167`
- Дата: `2026-02-04T17:00:11Z`

### `38a47700b5` - `Add thread/compact v2 (#10445)`

- Дата: `2026-02-03T18:15:55-08:00`; PR: `#10445`.
- Developer info: adds app-server v2 `thread/compact` API as an async trigger.
- User info: clients can request compaction for a thread and follow progress through ordinary turn/item events.
- Compatibility/risks: compaction is asynchronous; clients should not expect the RPC response to contain compacted history.
- Evidence: `EVID-096-compact`.

## `rust-v0.97.0`

- Release commit: `4b415c72c9cb62dded0bb8eb5d4fcf9ddc90a8e3`
- Дата: `2026-02-04T20:26:51-08:00`
- Результат triage: no direct `thread/*` RPC entry. Release has adjacent memory/dynamic-output work but no selected app-server thread namespace change.
- Evidence: `EVID-097-no-change`.

## `rust-v0.98.0`

- Release commit: `82464689ce0ba8a3b2065e73a8aa0cfdf2ad0625`
- Дата: `2026-02-05T08:12:44-08:00`

### `fe8b474acd` - `fix(core,app-server) resume with different model (#10719)`

- Дата: `2026-02-05T00:40:05-08:00`; PR: `#10719`.
- Developer info: fixes app-server resume path when caller resumes a thread with a different model/config override.
- User info: `thread/resume` becomes more reliable for clients that change model before continuing saved work.
- Compatibility/risks: resume override behavior must stay aligned with runtime config projection.
- Evidence: `EVID-098-resume-model`.

## `rust-v0.99.0`

- Release commit: `ec9f76ce4f854c7d4f3c78c9b1bacbe128df286e`
- Дата: `2026-02-11T11:54:41-08:00`

### `9ee746afd6` - `Leverage state DB metadata for thread summaries (#10621)`

- Дата: `2026-02-05T16:39:11Z`; PR: `#10621`.
- Developer info: uses state DB metadata when projecting thread summaries.
- User info: `thread/list`/summary views can show richer metadata without reopening full rollouts.
- Compatibility/risks: metadata source precedence matters when rollout and state DB disagree.
- Evidence: `EVID-099-summary-state`.

## `rust-v0.100.0`

- Release commit: `8272f9a71ee7364d8de992418103628f8a17eb6f`
- Дата: `2026-02-12T09:50:36-08:00`

### `b5339a591d` - `refactor: codex app-server ThreadState (#11419)`

- Дата: `2026-02-11T12:20:54-08:00`; PR: `#11419`.
- Developer info: extracts/reshapes `ThreadState` as app-server runtime owner for loaded thread state.
- User info: no new RPC, but live thread state handling becomes a clearer app-server-owned path.
- Compatibility/risks: refactor can affect loaded/resume/status notification ordering.
- Evidence: `EVID-100-thread-state`.

### `c0ecc2e1e1` - `app-server: thread resume subscriptions (#11474)`

- Дата: `2026-02-11T16:21:13-08:00`; PR: `#11474`.
- Developer info: adds connection-aware subscription handling for resumed threads.
- User info: clients can resume and receive live thread events without missing initial replay/current updates.
- Compatibility/risks: subscription ordering must avoid gaps between resume response and new events.
- Evidence: `EVID-100-thread-state`.

## `rust-v0.101.0`

- Release commit: `cf5f1868bc3df74739fc8e06e5c2e93728c5d8f9`
- Дата: `2026-02-12T11:27:09-08:00`
- Результат triage: no direct app-server `thread/*` change selected.
- Evidence: `EVID-101-no-change`.

## `rust-v0.102.0`

- Release commit: `f59c7c1ab912c9bc5826e5ae60e49ff1b9b255ff`
- Дата: `2026-02-17T11:21:41-08:00`

### `efc8d45750` - `feat(app-server): experimental flag to persist extended history (#11227)`

- Дата: `2026-02-12T19:34:22Z`; PR: `#11227`.
- Developer info: adds an experimental app-server flag for extended thread history persistence.
- User info: clients can opt into more complete history capture for richer later reads/resumes.
- Compatibility/risks: experimental history persistence must not become required for old rollouts.
- Evidence: `EVID-102-resume-list`.

### `ebe359b876` - `Add cwd as an optional field to thread/list (#11651)`

- Дата: `2026-02-13T02:05:04Z`; PR: `#11651`.
- Developer info: extends `thread/list` filter parameters with optional `cwd`.
- User info: history UIs can filter sessions by workspace.
- Compatibility/risks: cwd comparison/canonicalization affects cross-platform filtering.
- Evidence: `EVID-102-resume-list`.

### `fb0aaf94de` - `codex-rs: fix thread resume rejoin semantics (#11756)`

- Дата: `2026-02-13T23:09:58Z`; PR: `#11756`.
- Developer info: fixes rejoin semantics for already running thread resume.
- User info: reconnecting to active work is less likely to duplicate or miss state.
- Compatibility/risks: running-thread resume must preserve live ordering and not fork a new runtime accidentally.
- Evidence: `EVID-102-resume-list`.

## `rust-v0.103.0`

- Release commit: `ff3d19f9022b154b9ab415f6ad128cd622608b1a`
- Дата: `2026-02-17T14:23:14-08:00`
- Результат triage: no direct app-server `thread/*` change selected.
- Evidence: `EVID-103-no-change`.

## `rust-v0.104.0`

- Release commit: `74d1f7b2b3af383bd3605344f3c842b194fd1d70`
- Дата: `2026-02-17T22:34:44-08:00`

### `31cbebd3c2` - `app-server: Emit thread archive/unarchive notifications (#12030)`

- Дата: `2026-02-17T14:53:58-08:00`; PR: `#12030`.
- Developer info: adds app-server notifications for archive and unarchive state changes.
- User info: clients can update history/sidebar state without polling after archive/unarchive operations.
- Compatibility/risks: notification delivery and request response must remain consistent for each affected thread.
- Evidence: `EVID-104-archive-notifications`.

## `rust-v0.105.0`

- Release commit: `a7eda6a29b3ee25549f385197ff109508dc49a90`
- Дата: `2026-02-25T16:36:36Z`

### Thread status, resume and list/search improvements

- Included commits: `1f54496c48`, `b06f91c4fe`, `035c4c30bb`, `936e744c93`, `37610240ec`, `f46b767b7e`.
- Developer info: app-server exposes loaded `ThreadStatus` through `thread/read`, `thread/list` and notifications; improves resume rejoin; adds nickname projection to `thread/read`; carries latest thread rename metadata; retains thread listener across disconnects; and adds search term support to `thread/list`.
- User info: clients can render live/notLoaded status, reconnect to active threads more reliably, see thread naming metadata and search lists by title/content metadata.
- Compatibility/risks: status and reconnect behavior become observable protocol state; list/search defaults must not surprise existing clients.
- Evidence: `EVID-105-status-search`.

## `rust-v0.106.0`

- Release commit: `ffd726a656403b69b75130025587d5e0a0d6b7d1`
- Дата: `2026-02-26T11:39:41-08:00`

### Thread realtime, dynamic items and unsubscribe

- Included commits: `947092283a`, `a0fd94bde6`, `21f7032dbb`.
- Developer info: adds experimental thread-scoped realtime API, `ThreadItem::DynamicToolCall`, and `thread/unsubscribe`.
- User info: clients can run realtime sessions attached to a thread, render dynamic tool calls in thread history, and detach from live thread event streams without archiving.
- Compatibility/risks: realtime and unsubscribe are lifecycle-sensitive; clients must handle live thread remaining loaded after unsubscribe.
- Evidence: `EVID-106-realtime-unsubscribe`.

## `rust-v0.107.0`

- Release commit: `19f8797c0f62abecb347e817aac36d18c5fc554e`
- Дата: `2026-03-02T10:17:13-07:00`

### `thread/start`, `thread/resume` and `Thread.ephemeral`

- Included commits: `69d7a456bb`, `8fa792868c`, `8c1e3f3e64`.
- Developer info: `thread/resume` replays pending item requests, `thread/start` becomes non-blocking, and `Thread` gets an `ephemeral` field.
- User info: clients receive faster start responses, better replay after reconnect, and explicit durable-vs-ephemeral thread metadata.
- Compatibility/risks: non-blocking start changes timing; clients must rely on notifications for full lifecycle state.
- Evidence: `EVID-107-start-resume`.

## `rust-v0.108.0`

- Release commit: `89b79419a1e0720856d4450cf17379221e5a3b1d`
- Дата: `2026-03-04T11:55:40-08:00`

### Thread names, status noise and metadata update

- Included commits: `14fcb6645c`, `9022cdc563`, `935754baa3`.
- Developer info: `thread/name/set` supports not-loaded threads, thread-created status changes are silenced, and `thread/metadata/update` is added for stored metadata patches.
- User info: clients can rename persisted threads, avoid redundant initial status notifications, and patch metadata without resume.
- Compatibility/risks: metadata update has to respect stored thread source of truth and not require a loaded runtime.
- Evidence: `EVID-108-name-metadata`.

## `rust-v0.109.0`

- Release commit: `37133fb8455766068008b8453a1da1dacae4274c`
- Дата: `2026-03-04T15:13:23-08:00`
- Результат triage: no direct app-server `thread/*` change selected.
- Evidence: `EVID-109-no-change`.

## `rust-v0.110.0`

- Release commit: `77aabe4218ab7ddaf4b6d471887bda043a4c16e6`
- Дата: `2026-03-04T17:39:17-08:00`
- Результат triage: no direct app-server `thread/*` change selected.
- Evidence: `EVID-110-no-change`.

## `rust-v0.111.0`

- Release commit: `8c75cd9afcd405d134530e53c78e5e0e4e5312a3`
- Дата: `2026-03-05T10:03:09-08:00`

### `22f4113ac1` - `Preserve persisted thread git info in resume (#13504)`

- Дата: `2026-03-04T17:16:43-08:00`; PR: `#13504`.
- Developer info: keeps persisted `gitInfo` stable through `thread/resume`.
- User info: clients continue seeing stored repository metadata after reopening a thread.
- Compatibility/risks: runtime recomputation must not clobber user-visible persisted metadata unexpectedly.
- Evidence: `EVID-111-gitinfo-resume`.

## `rust-v0.112.0`

- Release commit: `0ec16b2d9dd7d92f5661b132afdf4df3abb3d443`
- Дата: `2026-03-08T12:47:46-07:00`
- Результат triage: no direct app-server `thread/*` change selected.
- Evidence: `EVID-112-no-change`.

## `rust-v0.113.0`

- Release commit: `81c4928825d1e468447a17d6bc74b9abb48743f4`
- Дата: `2026-03-09T21:17:46-07:00`

### `51fcdc760d` - `app-server: Emit thread/name/updated event globally (#13674)`

- Дата: `2026-03-06T10:25:18-08:00`; PR: `#13674`.
- Developer info: makes `thread/name/updated` visible globally to initialized clients.
- User info: multiple clients can update thread title UI after another client renames a thread.
- Compatibility/risks: clients should deduplicate local optimistic title updates against global notifications.
- Evidence: `EVID-113-name-global`.

## `rust-v0.114.0`

- Release commit: `b9904c0ae4ecb773549efd6ea3fb05229402fdb9`
- Дата: `2026-03-10T16:35:03-07:00`

### `c6343e0649` - `Implemented thread-level atomic elicitation counter for stopwatch pausing (#12296)`

- Дата: `2026-03-09T22:29:26-07:00`; PR: `#12296`.
- Developer info: adds experimental `thread/increment_elicitation` and `thread/decrement_elicitation` methods so app-server can track pending elicitation count at thread level.
- User info: clients can pause/resume elapsed-time UI around interactive elicitation without inferring state from unrelated transcript items.
- Compatibility/risks: counter methods are experimental and must stay balanced per thread; stale increments can leave UI timers paused.
- Evidence: `EVID-114-elicitation-counter`.

## `rust-v0.115.0`

- Release commit: `f028679abb30051cec2434e624cd99975986b41b`
- Дата: `2026-03-16T11:43:12-07:00`

### `8ac27b2a16` - `Add ephemeral flag support to thread fork (#14248)`

- Дата: `2026-03-11T12:33:08-07:00`; PR: `#14248`.
- Developer info: extends `thread/fork` with ephemeral support.
- User info: clients can fork a stored or live thread into an in-memory branch.
- Compatibility/risks: fork durability must be explicit in the response and not silently persist temporary branches.
- Evidence: `EVID-115-fork-ephemeral`.

## `rust-v0.116.0`

- Release commit: `38771c9082535aa16b4c4d0395d3532f32f656ff`
- Дата: `2026-03-19T09:49:09-07:00`
- Результат triage: no direct app-server `thread/*` change selected.
- Evidence: `EVID-116-no-change`.

## `rust-v0.117.0`

- Release commit: `4c70bff480af37b1bf1a9b352b8341060fe55755`
- Дата: `2026-03-26T14:22:09-07:00`

### Resume settings, shell command and realtime transcript

- Included commits: `bb30432421`, `01df50cf42`, `3431f01776`.
- Developer info: `thread/resume` reuses persisted model and reasoning effort; app-server adds `thread/shellCommand`; realtime v2 gets transcript notification.
- User info: resumed threads keep model/reasoning continuity, clients can run user shell commands against a thread, and realtime clients can render transcript deltas/completions.
- Compatibility/risks: `thread/shellCommand` is intentionally user-initiated and unsandboxed per current docs; realtime notification volume needs opt-out handling.
- Evidence: `EVID-117-shell-realtime`.

## `rust-v0.118.0`

- Release commit: `b630ce9a4e754d35a1f33e4366ba638d18626142`
- Дата: `2026-03-31T09:07:09-07:00`
- Результат triage: no direct app-server `thread/*` change selected.
- Evidence: `EVID-118-no-change`.

## `rust-v0.119.0`

- Release commit: `4a3466efbf84cfb7469eca94bbf6307166c9f48e`
- Дата: `2026-04-10T13:45:23-07:00`

### `95e809c135` - `Auto-trust cwd on thread start (#16492)`

- Дата: `2026-04-03T00:02:56Z`; PR: `#16492`.
- Developer info: `thread/start` can mark the cwd as trusted when the resolved permission/sandbox conditions allow it.
- User info: app-server-created threads can avoid repeated trust prompts for known workspaces.
- Compatibility/risks: trust persistence failure must not hide security boundaries; later releases harden this behavior.
- Evidence: `EVID-119-thread-start-trust`.

## `rust-v0.120.0`

- Release commit: `65319eb1400cbd2890c43d572263dabd25f18ba9`
- Дата: `2026-04-10T18:58:04-07:00`
- Результат triage: no direct app-server `thread/*` change selected.
- Evidence: `EVID-120-no-change`.

## `rust-v0.121.0`

- Release commit: `d65ed92a5e440972626965d0af9a6345179783bc`
- Дата: `2026-04-15T20:43:45+01:00`

### Thread start/list/realtime and ThreadStore foundation

- Included commits: `e9e7ef3d36`, `d25a9822a7`, `2f6fc7c137`, `dae56994da`, `cdfcd2ca92`.
- Developer info: fixes Windows cwd filtering in `thread/list`, avoids failing `thread/start` when trust persistence fails, expands realtime output modality/transcript events, introduces the ThreadStore interface and adds local ThreadStore listing.
- User info: thread list filters work more reliably across platforms, thread start is more tolerant of non-critical trust write failures, realtime UI has richer outputs, and thread listing starts moving behind a storage abstraction.
- Compatibility/risks: ThreadStore becomes an important backend source of truth; future app-server changes should avoid direct rollout-file assumptions.
- Evidence: `EVID-121-threadstore-foundation`.

## `rust-v0.122.0`

- Release commit: `230dcadee609fa99d6162fe1107457030e5270a7`
- Дата: `2026-04-20T18:10:09+01:00`

### ThreadStore migration and paged thread history

- Included commits: `50d3128269`, `6e72f0dbfd`, `ec8d4bfc77`, `9d6f4f2e2e`, `fad3d0f1d0`, `eaf78e43f2`.
- Developer info: archive/unarchive move to local ThreadStore; remote ThreadStore implementation appears; app-server replays token usage after resume/fork; `thread/read` view loading is split and routed through ThreadStore; `thread/list` gains sorting/backwards cursor; `thread/turns/list` is introduced.
- User info: clients get more scalable list/read/history behavior, better resume/fork token usage restoration and paged turn history access.
- Compatibility/risks: pagination cursors and ThreadStore-backed reads must remain stable across local/remote stores.
- Evidence: `EVID-122-threadstore-history`.

## `rust-v0.123.0`

- Release commit: `0785b66228dff87f891e291cb5686631865b6922`
- Дата: `2026-04-22T17:25:21-07:00`

### `ac7c9a685f` - `codex: move unloaded thread writes into store (#18361)`

- Дата: `2026-04-20T09:50:01-07:00`; PR: `#18361`.
- Developer info: routes unloaded thread writes through ThreadStore.
- User info: metadata/name/history updates against unloaded threads use the same storage path as list/read.
- Compatibility/risks: unloaded mutations should not require live runtime state.
- Evidence: `EVID-123-unloaded-store`.

### `660153b6de` - `feat: cascade thread archive (#18112)`

- Дата: `2026-04-20T23:38:18+01:00`; PR: `#18112`.
- Developer info: archive operation cascades to descendant threads.
- User info: archiving a parent can also hide spawned descendants from active history.
- Compatibility/risks: cascade semantics must avoid archiving unrelated threads; lineage source matters.
- Evidence: `EVID-123-unloaded-store`.

## `rust-v0.124.0`

- Release commit: `e9fb49366c93a1478ec71cc41ecee415a197d036`
- Дата: `2026-04-23T19:23:03+02:00`

### `4f8c58f737` - `Support multiple cwd filters for thread list (#18502)`

- Дата: `2026-04-22T06:10:09-04:00`; PR: `#18502`.
- Developer info: `thread/list` accepts multiple cwd filters.
- User info: history UIs can aggregate several workspace roots in one list request.
- Compatibility/risks: cwd list filtering should preserve existing single-cwd behavior.
- Evidence: `EVID-124-cwd-guardian`.

### `11e5af53c4` - `Add plumbing to approve stored Auto-Review denials (#18955)`

- Дата: `2026-04-22T10:38:19-07:00`; PR: `#18955`.
- Developer info: adds `thread/approveGuardianDeniedAction` protocol/schema/app-server plumbing for approving a stored Guardian denial event on a thread.
- User info: clients can unblock a previously denied action through a thread-scoped app-server method instead of replaying unrelated runtime state.
- Compatibility/risks: approval payload must preserve the stored denial event semantics and remain aligned with generated JSON/TypeScript schemas.
- Evidence: `EVID-124-cwd-guardian`.

## `rust-v0.125.0`

- Release commit: `637f7dd6d737f3961e6bf32fbb3861c4953269c5`
- Дата: `2026-04-24T18:55:17+02:00`

### Resume/fork payload trimming and environment state

- Included commits: `3d3028a5a9`, `49fb25997f`.
- Developer info: adds `excludeTurns` to `thread/resume` and `thread/fork`; adds sticky environment API and thread state.
- User info: clients can resume/fork without receiving full turn arrays and can keep thread-scoped environment selection state.
- Compatibility/risks: clients that request excluded turns must use paging APIs; environment state is thread-scoped and must survive later turns.
- Evidence: `EVID-125-exclude-turns-env`.

## `rust-v0.126.0`

- Release commit: `4695dfce4f0b63491e410628ccf1935addb635ab`
- Дата: `2026-04-29T14:45:41-07:00`

### ThreadStore resume/fork and goal APIs

- Included commits: `0a9b559c0b`, `6c874f9b34`.
- Developer info: migrates fork/resume reads to ThreadStore and adds app-server `thread/goal/*` API surface.
- User info: resume/fork become ThreadStore-backed, and clients can create/read/clear persisted thread goals.
- Compatibility/risks: goal state is persisted thread state and should not be tied to transient TUI-only status.
- Evidence: `EVID-126-goal-store`.

## `rust-v0.127.0`

- Release commit: `7c8a4a5b7909fd3fbd276337a95a0e5fee9f1eb0`
- Дата: `2026-04-29T22:11:17-07:00`
- Результат triage: no new direct app-server `thread/*` entry beyond goal work already contained in `rust-v0.126.0`.
- Evidence: `EVID-127-no-change`.

## `rust-v0.128.0`

- Release commit: `e4310be51f617f5e60382038fa9cbf53a2429ca4`
- Дата: `2026-04-30T08:06:34-07:00`
- Результат triage: no new direct app-server `thread/*` entry selected.
- Evidence: `EVID-128-no-change`.

## `rust-v0.129.0`

- Release commit: `2808a4deb181e5ca2b1293a1a5980938cb746861`
- Дата: `2026-05-07T08:13:37-07:00`

### ThreadStore history consolidation, item views and session IDs

- Included commits: `5affb7f9d5`, `127be0612c`, `e4d6675632`, `541e99cf09`, `707e51bd8b`, `33d24b0df5`, `9e0c191c13`, `2c1a361a2e`, `06e5dfa4dd`, `5ecff05196`.
- Developer info: marks `thread/turns/list` and `excludeTurns` experimental; migrates turns/read/history/metadata through ThreadStore; always returns limited thread history; adds turn items view; moves thread naming to app server; returns `sessionId` from `thread/fork` and then moves `sessionId` onto `Thread`.
- User info: clients get bounded history by default, page or choose item views when needed, and can reason about live session identity directly from the `Thread` object.
- Compatibility/risks: history payload size is intentionally bounded; clients needing full history must use paged APIs.
- Evidence: `EVID-129-history-sessionid`.

## `rust-v0.130.0`

- Release commit: `58573da43ab697e8b79f152c53df4b42230395a8`
- Дата: `2026-05-08T14:57:54-07:00`

### Thread pagination contract and store cleanup

- Included commits: `0d0835dd53`, `56823ec46b`, `4242bba2eb`.
- Developer info: formalizes Thread pagination APIs and ThreadStore contract; routes thread name edits and ThreadManager rollout path reads through ThreadStore.
- User info: large threads are pageable, and name/history reads become more consistent across storage backends.
- Compatibility/risks: pagination options and cursor semantics become a cross-client contract.
- Evidence: `EVID-130-pagination-contract`.

## `rust-v0.131.0`

- Release commit: `05eb8678451435cbc8d79c6d8254276289f2bdf1`
- Дата: `2026-05-18T08:40:25-07:00`

### Remote redaction and permission/runtime-root projection

- Included commits: `7bddb3083d`, `8a5306ff88`, `83bbb4f326`.
- Developer info: adds thread history redaction for remote clients; app-server moves to permission ids/runtime workspace roots; stops returning thread permission profiles.
- User info: remote clients get redacted history where needed and read stable permission/runtime-root identity rather than full permission profile payloads.
- Compatibility/risks: permission profile payload removal is an API shape change; clients should rely on ids and active projections.
- Evidence: `EVID-131-redaction-permissions`.

## `rust-v0.132.0`

- Release commit: `13595c36e218fcbd13df118eeadf00d4eb0e6d31`
- Дата: `2026-05-19T16:22:37-07:00`
- Результат triage: no direct app-server `thread/*` change selected.
- Evidence: `EVID-132-no-change`.

## `rust-v0.133.0`

- Release commit: `9474e5cfc4494b0ba319352aa86ce436c59e65c8`
- Дата: `2026-05-21T08:27:49-07:00`

### `771a4e74ac` - `Add thread/settings/update app-server API (#23502)`

- Дата: `2026-05-20T11:03:20-07:00`; PR: `#23502`.
- Developer info: adds `thread/settings/update` for queued next-turn settings changes on a loaded thread.
- User info: clients can update model/provider/sandbox-like next-turn settings without adding transcript items or starting a turn.
- Compatibility/risks: settings update must emit `thread/settings/updated` only when effective state changes.
- Evidence: `EVID-133-settings-update`.

## `rust-v0.134.0`

- Release commit: `a75c443fdb64db48c3cf4bdb247c7ee52c0144c9`
- Дата: `2026-05-26T09:46:10-07:00`

### Thread content search

- Included commits: `ac0bff27e7`, `05cf2fc4ce`.
- Developer info: adds rollout-backed thread content search and makes thread search case-insensitive.
- User info: clients can search conversation history content, not only names or metadata.
- Compatibility/risks: search snippets and matching must remain bounded and avoid loading excessive rollout content.
- Evidence: `EVID-134-thread-search`.

## `rust-v0.135.0`

- Release commit: `4daceea869704f9f35e0a3949fc34711ef978a4e`
- Дата: `2026-05-28T09:14:50-07:00`
- Результат triage: no direct app-server `thread/*` change selected.
- Evidence: `EVID-135-no-change`.

## `rust-v0.136.0`

- Release commit: `7ca611348db9446711ed16ed81c84095e3721cee`
- Дата: `2026-06-01T09:00:23-07:00`

### `2a1158b8e2` - `feat(app-server): include turns page on thread resume (#23534)`

- Дата: `2026-05-28T09:18:13-07:00`; PR: `#23534`.
- Developer info: adds `initialTurnsPage` support on `thread/resume`.
- User info: clients can resume and receive a first paged history slice in the same round trip.
- Compatibility/risks: response shape must stay aligned with `thread/turns/list` pagination controls.
- Evidence: `EVID-136-initial-turns-page`.

## `rust-v0.137.0`

- Release commit: `f221438b691b8f749d98f22077c93ebe01923fbe`
- Дата: `2026-06-03T16:42:27-07:00`

### Parent metadata and search/persistence cleanup

- Included commits: `cf0911076f`, `11e0f3d3ae`, `45912a6dc6`.
- Developer info: stores and exposes `parent_thread_id` on `Thread`; removes the old experimental `persist_extended_history` bool; reuses compressed rollout search snippets.
- User info: clients can see direct parent lineage for subagent/spawned threads and get more efficient search snippets.
- Compatibility/risks: parent lineage is optional metadata; older threads without it remain readable.
- Evidence: `EVID-137-parent-search`.

## `rust-v0.138.0`

- Release commit: `c18e9f478bc940ef1ef8e1c426364c0fe3d86b73`
- Дата: `2026-06-08T15:00:08-07:00`

### Thread fork naming and cwd/runtime-root validation

- Included commits: `d8121f93c8`, `40c8f1a007`, `76c0a5379c`.
- Developer info: fixes forked thread name inheritance, requires absolute cwd in thread settings, and makes runtime workspace roots absolute in app-server API.
- User info: forked threads inherit names predictably, and clients receive stricter path validation for thread settings/runtime roots.
- Compatibility/risks: relative path inputs that previously passed may now fail fast.
- Evidence: `EVID-138-paths-names`.

## `rust-v0.139.0`

- Release commit: `a7dff904308535e965aee87680c1fc5ef1d19eec`
- Дата: `2026-06-09T12:02:39-07:00`

### `f3c1283411` - `Pair thread environment settings (#26687)`

- Дата: `2026-06-08T13:55:15-07:00`; PR: `#26687`.
- Developer info: pairs thread environment settings so environment selection stays coherent across thread state.
- User info: clients can rely on environment settings being projected as a thread-level selection rather than loose per-field state.
- Compatibility/risks: environment settings updates must preserve pair invariants.
- Evidence: `EVID-139-environments`.

## `rust-v0.140.0`

- Release commit: `6506579001c322927a3e4bd440563267a7ac6c1f`
- Дата: `2026-06-15T13:22:00-07:00`

### Cold resume, realtime overrides, background terminals and delete

- Included commits: `6a9a49b334`, `4a3eac2144`, `a1a8807e9d`, `a19d43a40a`, `216dee1189`.
- Developer info: avoids rereading rollout history during cold resume; adds per-session realtime model/version overrides; adds thread background terminal process APIs; adds `thread/delete`; and adds role support to realtime append text.
- User info: cold resume is more efficient, realtime sessions are more configurable, clients can manage background terminals, hard-delete threads, and send developer/user role text into realtime.
- Compatibility/risks: delete is destructive and must cascade only as documented; background terminal APIs are experimental and process-id based.
- Evidence: `EVID-140-delete-realtime`.

## `rust-v0.141.0`

- Release commit: `3fb81667d30d9d24297216ea61fbfcc4351b2aa9`
- Дата: `2026-06-17T20:59:28-07:00`

### Parent filtering, dynamic tool namespaces and realtime speech

- Included commits: `dfd03ea01b`, `11faf9af94`, `1d8ff89aa3`.
- Developer info: adds experimental `ThreadListParams.parentThreadId`, exposes explicit dynamic tool namespaces in `thread/start`, and adds realtime speech append control.
- User info: clients can page direct spawned child threads, start threads with namespaced dynamic tools, and instruct realtime output speech explicitly.
- Compatibility/risks: `parentThreadId` is experimental and direct-child only; dynamic tool namespace wire shape must stay aligned with tool registry semantics.
- Evidence: `EVID-141-parent-dynamic-realtime`.

## `rust-v0.142.0`

- Release commit: `3a76f3ac68c8949d1cac6ea769b6ec7b8953a415`
- Дата: `2026-06-22T23:36:01+02:00`
- Subject: release notes include app-server multi-agent delegation controls, plugin recommendations, rollout budgets, reminders/time APIs, goal-first thread visibility and runtime disconnect fixes. Direct `thread/*` entries here are recency ordering migration, incremental history changes, resume/session continuity, goal-first list/search visibility, thread/turn multi-agent mode projections and realtime thread API deltas.

### Recency ordering migration and compatibility

- Included commits: `fac3158c2a`, `cb15c64760`, `7dc7096ae1`.
- Developer info: introduces `recencyAt`/sort metadata for sidebar ordering, reverts the initial shape, then restores thread recency with compatible migration history across app-server protocol schemas, ThreadStore local/in-memory backends, rollout/state migrations and thread list/read/search tests.
- User info: clients can order sidebar/thread lists by recent activity while older records remain readable through migration-compatible metadata.
- Compatibility/risks: this is a migration story, not a single forward-only field addition. Fork clients should follow the final compatible `rust-v0.142.0` shape and keep old/missing recency metadata readable.
- Evidence: `EVID-142-thread-recency`.

### Incremental history and optional ThreadStore turn filters

- Included commits: `1e6970542e`, `01a2df2947`.
- Developer info: adds incremental thread history change structures for changed items/turns/removed turn ids and makes ThreadStore turn filtering optional in store/type backends.
- User info: clients can consume narrower thread-history deltas and storage-backed thread reads can avoid forcing a turn filter when the caller needs broader thread content.
- Compatibility/risks: incremental changes must remain bounded and reconstructable; callers should not infer that optional turn filters mean unbounded full-history loading is always cheap.
- Evidence: `EVID-142-thread-history`.

### Resume, goal-first visibility and session metadata continuity

- Included commits: `e8dd1b45cb`, `6d15bb3d17`; supporting metadata evidence: `8f8de7844f`.
- Developer info: fixes goal-first live threads missing from `thread/list`, persists session ids across thread resume/read/list flows and restores `thread_source` in turn metadata used by client-metadata tests.
- User info: goal-first threads are discoverable again through list/search, resumed threads keep stable session identity and clients receive more faithful turn metadata.
- Compatibility/risks: session-id persistence is durability-sensitive; resume should not mint unrelated session identity for existing stored threads. `thread_source` metadata remains supporting evidence rather than a new standalone `thread/*` method.
- Evidence: `EVID-142-thread-resume-goal`.

### Thread realtime controls and text append shape

- Included commits: `683bd170dc`, `e922f46a0f`.
- Developer info: adds app-server protocol/docs/tests for controlling automatic realtime handoff delivery and supports assistant realtime append text through app-server protocol/schema, realtime websocket methods and protocol event paths.
- User info: clients have more explicit realtime handoff behavior and can append assistant-role text into realtime flows.
- Compatibility/risks: realtime APIs remain under `thread/realtime/*`; ordinary thread history should not consume realtime control events unless the protocol explicitly projects them.
- Evidence: `EVID-142-thread-realtime`.

### Multi-agent mode projections into thread and turn APIs

- Included commits: `fc8c6b7384`, `7abfcf220b`, `c03742ca0a` (shared with `docs/fork/research/multi-agents`, `EVID-142-multi-agent-mode`).
- Developer info: adds per-turn and thread-level `MultiAgentMode`, projects it through thread start/fork/resume/settings notifications and then simplifies the control model.
- User info: app-server clients can configure multi-agent delegation mode for a thread and override it for a specific turn.
- Compatibility/risks: the detailed runtime semantics belong in the multi-agent research package; this thread timeline records only the `thread/*`/`turn/start` API projection.
- Evidence: `EVID-142-thread-multi-agent-mode`.

## `rust-v0.142.1`

- Release commit: `95da8fd25193fd58d1c5984eee20d1ef7bd50e77`
- Дата: `2026-06-24T16:46:26-07:00`
- Результат triage: прямых изменений app-server `thread/*` contract в диапазоне `rust-v0.142.0^{}..rust-v0.142.1^{}` не найдено.
- Evidence: `EVID-142.1-no-change`.

## `rust-v0.142.2`

- Release commit: `390b0d254d658148751d0cca50ca41832c7894a1`
- Дата: `2026-06-24T23:36:23-07:00`
- Subject: patch release with durable response-item turn metadata, world-state resume baseline behavior and app-server ingress validation relevant to this thread research package.

### ResponseItem `turn_id` metadata for durable history/resume/fork/websocket reuse

- Included commits: `4a82ecc3c9`.
- Developer info: durable `ResponseItem` variants get `internal_chat_message_metadata_passthrough.turn_id` when missing as they enter history, replacement history, inter-agent communication history and compaction summaries, while existing item turn ids are preserved through persistence, resume reconstruction, compaction, forked history and websocket incremental reuse.
- User info: clients and history reconstruction can correlate response items with the turn that introduced them across resume/fork/compaction flows.
- Compatibility/risks: metadata remains optional for older records; `compaction_trigger` is request control and intentionally does not carry durable metadata.
- Evidence: `EVID-142.2-turn-metadata-inject-items`.

### World-state resume baseline and `thread/inject_items` image validation

- Included commits: `3b32d861c5`, `b294638bb5`.
- Developer info: session resume can seed the in-memory world-state baseline from the latest `TurnContextItem`, and app-server validates injected raw history items so remote HTTP(S) image URLs are rejected at ingress while legacy `thread/resume.history` remains readable.
- User info: resumed threads get more consistent environment context updates, and clients cannot inject remote image URLs into thread history through the app-server thread item injection path.
- Compatibility/risks: world state itself is not serialized and legacy turn context can only reconstruct the primary environment as `local`; image rejection belongs at app-server ingress and should not make existing stored legacy history unreadable.
- Evidence: `EVID-142.2-turn-metadata-inject-items`.

## `rust-v0.142.3`

- Release commit: `e2b60462a7321517895dd94920661599303a7539`
- Дата: `2026-06-26T13:46:44-07:00`
- Результат triage: прямых изменений app-server `thread/*` contract в диапазоне `rust-v0.142.2^{}..rust-v0.142.3^{}` не найдено.
- Evidence: `EVID-142.3-no-change`.

## `rust-v0.142.4`

- Release commit: `d0fd96663e19a6cd5d6f315e3420c4d154562013`
- Дата: `2026-06-29T04:20:21Z`
- Результат triage: прямых изменений app-server `thread/*` contract в диапазоне `rust-v0.142.3^{}..rust-v0.142.4^{}` не найдено.
- Evidence: `EVID-142.4-no-change`.

## `rust-v0.142.5`

- Release commit: `26de83050b20f7e0ee211b9739e52ae00ce8032a`
- Дата: `2026-06-30T20:30:24-04:00`
- Результат triage: прямых изменений app-server `thread/*` contract в диапазоне `rust-v0.142.4^{}..rust-v0.142.5^{}` не найдено. Backport `e019402a9e` removes full Responses WebSocket request payload trace logging in `codex-api`, but it does not change app-server `thread/*`, ThreadStore/history/resume/realtime/session lifecycle contract.
- Evidence: `EVID-142.5-no-change`.

## `rust-v0.143.0`

- Release commit: `c4d748f586a84a3ed5b6aceb82e9a1db4abb1cda`
- Дата: `2026-07-07T17:43:00-07:00`
- Topology boundary от `rust-v0.142.5`: left/right `6/258`, merge-base `27f22b54aef4d7e5eb6c564e969c961c74605461`. Здесь и далее `A..B` означает только commits, достижимые из `B`, но не из `A`, а не последовательную ancestry-цепочку stable tags. Шесть left-only commits остаются в patch-линии `0.142.x`; их полный disposition зафиксирован в `evidence.md`.
- Тема release notes: environment inspection, descendant thread listing, fork-through-turn, realtime tail preservation и terminal rollout durability. Прямые thread entries также включают переименование item-pagination API, compatibility floor для history mode, сохранение originator при resume и world-state replay.

### Item pagination, listing по ancestor и fork-through-turn

- Включённые commits: `1882719b30`, `8057603d0c`, `f72976a5f1`; документальное подтверждение: `8d80b0176a`.
- Для разработчика: experimental `thread/turns/items/list` переименован в `thread/items/list`, а `turnId` стал optional; добавлен взаимоисключающий `thread/list.ancestorThreadId` для strict transitive descendants с сохранением direct-child семантики `parentThreadId`; stable optional `thread/fork.lastTurnId` копирует сохранённую историю до указанного terminal turn включительно; thread/turn ids документированы как UUID7.
- Для пользователя: клиенты могут постранично читать persisted items всего thread или одного turn, получать всё дерево spawned threads одним paged query и делать fork от выбранного исторического turn без rollback.
- Совместимость/риски: старое имя experimental item method заменено без alias; сам ancestor исключён; отсутствие `lastTurnId` сохраняет full-history fork behavior, а invalid/non-terminal id должен завершаться ошибкой, а не неоднозначным truncation.
- Evidence: `EVID-143-thread-wire`.

### Неизменяемый thread history mode и deprecation rollback

- Включённые commits: `5267e805fb`, `812cd2bb57`, `268328001f`.
- Для разработчика: добавлены experimental `Thread.historyMode` и `thread/start.historyMode` со значениями `legacy|paginated`; значение сохраняется в первом canonical `SessionMeta` и SQLite/thread-store metadata, защищено от последующих compatibility metadata writes, а `thread/rollback` помечен deprecated.
- Для пользователя: metadata-only clients могут обнаруживать paginated thread, даже если текущий runtime не умеет resume его storage format; fork-at-turn служит целевым replacement path для части rollback UX.
- Совместимость/риски: `legacy` остаётся compatibility default; binaries без поддержки paginated history должны fail closed для full-history/read/resume mutation вместо silent downgrade, а последующие строки `SessionMeta` не могут переписать storage contract.
- Evidence: `EVID-143-thread-wire`.

### Provider fallback при thread start и сохраняемый originator

- Включённые commits: `6d9dbacf1a`, `1acb722e8a`.
- Для разработчика: добавлен opt-in experimental `thread/start.allowProviderModelFallback`, ограниченный authoritative static model catalogs; thread-level originator вычисляется из поддерживаемых Work service names, сохраняется с приоритетом explicit internal override и наследуется через resume/fork/sub-agent.
- Для пользователя: helper threads могут намеренно перейти на active provider default, а attribution остаётся стабильным после restart, fork и создания child thread.
- Совместимость/риски: fallback по умолчанию false и не должен переписывать произвольные ids для dynamic catalogs; originator является durable metadata, а не per-request analytics-only label.
- Evidence: `EVID-143-thread-wire`, `EVID-143-resume-persistence`.

### Resume/history и сохранение world state

- Включённые commits: `330ae6a516`, `806a4b66c9`, `3e51b46eba`, `fa036d39aa`, `a74771340d`, `723b23efd0`.
- Для разработчика: cold-resume rollout history совместно используется через `Arc` без изменения serialization; недостающие response-item ids назначаются fork-only history до reconstruction и persistence; world-state snapshots и merge patches сериализуются в rollouts, replay при resume/fork/rollback/compaction, а section reinject, если его persisted snapshot существует, но обязательный model-visible fragment отсутствует в retained history.
- Для пользователя: большие cold resumes обходятся без повторных full-rollout clones, forked items сохраняют stable identity, а resumed model context точнее сохраняет environment/extension state и не подавляет отсутствующие instructions.
- Совместимость/риски: старые binaries пропускают неизвестные world-state rollout records; legacy/malformed rollouts безопасно выдают новый full snapshot. Shared storage через `Arc` является implementation-only и не меняет wire или JSONL representation.
- Evidence: `EVID-143-resume-persistence`.

### Надёжность terminal events, teardown и закрытие realtime

- Включённые commits: `f5f812389e`, `c55ce3b51b`, `c976741124`; основа canonical history: `a107b84967`, `b9b934e99b`.
- Для разработчика: ThreadStore выполняет flush после terminal completion/interruption events; live thread persistence закрывается при завершении submission channel; определены canonical app-server projections и централизованные canonical-to-legacy mappings для семейств turn items, позднее используемых paginated storage; добавлен opt-in `thread/realtime/start.flushTranscriptTailOnSessionEnd`.
- Для пользователя: buffered remote stores сохраняют terminal events, same-process resume больше не конфликтует с leaked live writer, а opt-in realtime sessions доставляют остаток transcript перед закрытием.
- Совместимость/риски: tail flush по умолчанию false и должен быть idempotent/non-duplicating; teardown order обязан сохранять durable writes; одни canonical item definitions ещё не переключают persistence format для legacy threads.
- Evidence: `EVID-143-resume-persistence`, `EVID-143-thread-wire`.

## `rust-v0.144.0`

- Release commit: `767822446c7a594caa19609ca435281a9ec67e0d`
- Дата: `2026-07-09T08:59:13-07:00`
- Topology boundary от `rust-v0.143.0`: left/right `1/80`, merge-base `6afcf26d5d76c2f88b9096caa758931ffa673745`; left-only содержит только release wrapper `c4d748f586a84a3ed5b6aceb82e9a1db4abb1cda`, поэтому right-only `A..B` используется как inventory, а не как доказательство ancestry stable tag commit.

### Canonical lifecycle live items и сохранение paginated rollout

- Включённые commits: `cca16a1087`, `f659eb12bc`, `1bd9d841ca`, `058d97c5dc`, `6b4882528e`, `23aac925e7`, `ba5dd1fd3a`, `2342b2c2a`; compatibility foundation из `rust-v0.143.0`: `a107b84967`, `b9b934e99b`.
- Для разработчика: canonical `ItemStarted`/`ItemCompleted` становится live source для command execution, dynamic tools, MAv1/MAv2 collaboration/sub-agent activity, review markers и hook prompts; app-server v2 однократно потребляет canonical items, а централизованные mappings сохраняют legacy event consumers. Paginated threads сохраняют completed canonical `TurnItem` snapshots со stable turn/item ids и исключают redundant legacy projections; legacy threads сохраняют прежний event format, а forks наследуют source history mode.
- Для пользователя: live app-server item notifications сохраняют публичные shapes, а paginated history получает independently materializable stable item snapshots для `thread/turns/list` и `thread/items/list`.
- Совместимость/риски: один thread не должен смешивать legacy и paginated formats; streamed paginated items без stable server ids отклоняются; compatibility fanout не должен дублировать live app-server items.
- Evidence: `EVID-144-canonical-history-resume`.

### Сохранение approval reviewer при resume

- Включённый commit: `0746e8a345`.
- Для разработчика: effective approval reviewer сохраняется в turn context и settings events, затем `thread/resume` восстанавливает последнее persisted value, если typed или config-map resume override не задан явно.
- Для пользователя: перезапуск app-server больше не переключает незаметно auto-reviewed thread обратно на default reviewer текущего процесса.
- Совместимость/риски: старые rollout items могут не содержать reviewer metadata; explicit request overrides сохраняют приоритет, а settings events не должны materialize иначе нетронутый rollout только ради сохранения default.
- Evidence: `EVID-144-canonical-history-resume`.

## `rust-v0.144.1`

- Release commit: `44918ea10c0f99151c6710411b4322c2f5c96bea`
- Дата: `2026-07-09T15:10:27-07:00`
- Topology boundary от `rust-v0.144.0`: left/right `1/4`, merge-base `3380969a29134630d56feb6218e8e8dcc5e8196d`; left-only содержит release wrapper `767822446c7a594caa19609ca435281a9ec67e0d`, а right-only — три backports и новый release wrapper.
- Результат triage: прямых изменений app-server `thread/*`, ThreadStore/history/resume, rollout/session persistence или realtime lifecycle contract в диапазоне `rust-v0.144.0^{}..rust-v0.144.1^{}` не найдено; patch содержит только installer/Darwin code-mode/embedded-V8 reliability backports.
- Evidence: `EVID-144.1-no-change`.

## `rust-v0.144.2`

- Release commit: `a6645b6b8a656360fa16fb7e1c6721d0697d3d6a`
- Дата: `2026-07-12T20:24:45-07:00`
- Topology boundary от `rust-v0.144.1`: left/right `1/2`, merge-base `77c42a202aa7e89189c1126021dcc054b1795e1d`; left-only содержит предыдущий release wrapper.
- Результат triage: `32649bc5e6591ad8ea0b7b8ce073df447565ec7c` откатывает Guardian auto-review prompting и tool selection, но не меняет app-server `thread/*`, ThreadStore/history/resume или session persistence.
- Evidence: `EVID-144.2-no-change`.

## `rust-v0.144.3`

- Release commit: `78ad6e6bfd1d3b6a209acd3ef82172a96b25179c`
- Дата: `2026-07-12T22:21:53-07:00`
- Topology boundary от `rust-v0.144.2`: left/right `1/2`, merge-base `32649bc5e6591ad8ea0b7b8ce073df447565ec7c`; release wrapper называет выпуск version-only, но tag содержит отдельный commit `8a4d35a1e100efc2c64b72515668a84da663f067`.

### Сохранение model/reasoning settings при resume

- Включённый commit: `8a4d35a1e100efc2c64b72515668a84da663f067`.
- Для разработчика: `EventMsg::ThreadSettingsApplied` становится metadata producer для state и ThreadStore; `reasoning_effort` получает clearable patch semantics; cold `thread/resume` восстанавливает сохранённые model/provider/effort при отсутствии явного override.
- Для пользователя: изменения model и reasoning effort через thread settings сохраняются после restart/resume, а явное удаление effort не заменяется глобальным default.
- Совместимость/риски: wire/schema не меняются; старые записи без metadata остаются читаемыми; explicit request overrides имеют приоритет; клиент не должен превращать неявные defaults в resume overrides или передавать implicit default service tier.
- Evidence: `EVID-144.3-resume-model-settings`.

## `rust-v0.144.4`

- Release commit: `8c68d4c87dc54d38861f5114e920c3de2efa5876`
- Дата: `2026-07-13T21:20:37-07:00`
- Topology boundary от `rust-v0.144.3`: left/right `1/2`, merge-base `8a4d35a1e100efc2c64b72515668a84da663f067`; left-only содержит предыдущий release wrapper.
- Результат triage: `d82b7e5d4c1c274bee0eb55f92ec12d017e78634` добавляет Guardian auto-review policy в model catalog, но не меняет app-server `thread/*`, ThreadStore/history/resume или session persistence.
- Evidence: `EVID-144.4-no-change`.

## `rust-v0.145.0`

- Release commit: `25af12f7e61572b0bc18ddb1008be543b91519b0`.
- Topology boundary от `rust-v0.144.4`: left/right `7/342`, merge-base `3380969a29134630d56feb6218e8e8dcc5e8196d`; right-only множество используется как inventory, поскольку stable tag commits разошлись.

### Paginated storage и projection

- Включённые commits: `414217dc8a`, `bfe31598c7`, `5c19155cbd`, `0ef9fa4d65`, `b24aa20107`.
- Durable JSONL с monotonic ordinals остаётся canonical history, а dedicated SQLite storage и materialization/checkpoint дают rebuildable projection для paginated reads.
- Evidence: `EVID-145-paginated-storage`.

### Paginated app-server API

- Включённые commits: `e0a3641e6c`, `da61f7d8e1`, `0fb559f0f6`, `c1862b8db4`.
- `thread/start`, `thread/resume`, `thread/read`, `thread/turns/list` и `thread/items/list` получают paged projections/cursors; `thread/searchOccurrences` добавляет literal case-insensitive occurrence search с controlled unsupported response.
- Evidence: `EVID-145-paginated-app-server`.

### Lineage и mutable metadata

- Включённые commits: `69acc71cb1`, `2be648ba4a`, `5a208c1fc3`, `86102db5a1`, `19b2273d8a`, `2793c826e8`, `b7e39aa316`.
- Inherited prefixes, parent/ancestor lineage, thread names, Git/memory metadata получают явное ownership в ThreadStore/SQLite и не требуют загрузки active runtime; неизвестный history mode отклоняется fail closed.
- Evidence: `EVID-145-paginated-lineage-metadata`.

### Resume и повторное использование context

- Включённые commits: `d88db19144`, `592467fb96`, `769a5de257`, `088239294a`.
- Retry/edit сохраняет thread context, model context загружается из bounded rollout suffix, advanced-reasoning selection становится явным, а root-thread resume восстанавливает V2 agent identities.
- Evidence: `EVID-145-fork-resume-context`.

### Realtime и history lifecycle

- Включённые commits: `2e1607ee2f`, `b03545b5b8`, `025db22058`, `312caf176a`, `6f785632b0`.
- Frameless Bidi, streamed/channel handoffs, realtime V3 `initialItems` и audio preservation остаются отдельным protocol surface, не неявной частью paginated history.
- Evidence: `EVID-145-thread-realtime-history`.
