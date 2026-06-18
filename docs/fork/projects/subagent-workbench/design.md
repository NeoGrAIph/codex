# Subagent Workbench design

## Native source of truth

The workbench must remain a projection over native Codex state. It must not introduce a parallel task registry, team file, mailbox, role registry or persistent UI cache.

| Data | Owner | Workbench use |
| --- | --- | --- |
| Agent lineage | `SessionSource::SubAgent(ThreadSpawn)` and `AgentPath` | Stable identity, parent/child relation, replay/resume compatibility. |
| Role/profile label | `agent_role`, `agent_nickname`, `Config.agent_roles` | Row label and detail context; role catalog is not copied into runtime state. |
| Thread note | `thread_note` in session/source metadata and MAv2 list output | Short purpose/status clue for orchestrator recovery. |
| Runtime lifecycle | `AgentControl`, `AgentStatus`, app-server `ThreadStatus` | Derived status: running, idle, waiting, error, closed/not loaded. |
| Activity preview | `ThreadItem`, `ThreadEventStore`, bounded summary helpers | Bounded current activity and recent tool summary; legacy `agent_status_feed` is retained only as test/reference coverage after picker unification. |
| Workdir/cwd | app-server `Thread.cwd`, stored thread/session metadata | Detail recovery anchor. |
| Prompt/context preview | app-server `Thread.preview` from existing thread metadata and receiver-matched `CollabAgentToolCall.prompt` from buffered app-server item events | Optional bounded first-prompt and latest-input recovery anchors; never read raw turns for workbench detail. |
| Model/provider/reasoning | app-server `Thread.model_provider` and TUI `ThreadSessionState` when the thread is loaded in the current client | Detail context; missing values must render as absent rather than guessed. |
| Tokens/context | app-server `ThreadTokenUsageUpdated` notifications buffered in `ThreadEventStore` | Optional selected-detail anchor; never required for list correctness. |

## UI contract

The workbench list is scan-first. A row should be built from:

- status: derived from MAv2 `AgentStatus` and app-server `ThreadStatus` active flags;
- kind: MAv2 sub-agent, legacy sub-agent, replay-only/closed, or other native classification;
- label: prefer MAv2 agent name, then nickname/role, then canonical path, then thread id;
- age: only if a clear source is chosen, such as thread creation time or active turn start time;
- current activity: bounded live summary, then `last_task_message`, then `thread_note`, then empty state.

The selected detail panel should show recovery anchors: thread id/path, nickname/role, bounded prompt preview from `Thread.preview` when available, receiver-matched initial/latest collab prompt context when already buffered, note, cwd, model/provider/reasoning when known, recent activity, current plan when safely available, last task message when not encrypted and already projected through a native source, and status/error. It must not render full transcripts, raw command output, raw reasoning, secrets or encrypted inter-agent content.

## Team/status layer

OpenClaude `TeamStatus` and `TeamsDialog` are useful as UX references for a compact team indicator and status list, but their team file, mailbox, task files and pane backend are not portable to Codex. In this fork, team/status must remain a derived projection over the same workbench sources: `SessionSource::SubAgent(ThreadSpawn)`, `AgentPath`, `ThreadStatus`, `ThreadEventStore`, role metadata and existing TUI navigation state.

Planner/implementer/verifier labels are ordinary role template names from `agent_role`; they are not hardcoded team-member enums. There is no separate Codex task-owner registry, unread mailbox state or busy/idle task file in v1. If a later feature needs those semantics, it must first define a native source of truth instead of copying OpenClaude `.claude/teams` or file-mailbox state.

### Derived status contract

The first status implementation should use one UI enum derived in this order:

| Derived status | Rule | Notes |
| --- | --- | --- |
| `error` | app-server `ThreadStatus::SystemError` or equivalent loaded-thread system error | Highest priority because it blocks normal interaction. |
| `closed/not_loaded` | `ThreadStatus::NotLoaded`, replay-only row, or persisted closed spawn edge | Watch/detail may still be available; destructive actions stay disabled unless ownership is proven. |
| `waiting_approval` | `ThreadStatus::Active` with `ThreadActiveFlag::WaitingOnApproval` | Do not collapse into generic running/idle. |
| `waiting_user` | `ThreadStatus::Active` with `ThreadActiveFlag::WaitingOnUserInput` | Do not infer child state from the parent waiting on `wait_agent`. |
| `running` | `ThreadStatus::Active` without waiting flags, or a live event channel with an active turn | Implemented through cached `AgentPickerThreadStatus`; live turn state remains a fallback for path-backed local agents. |
| `idle` | `ThreadStatus::Idle` | Stable loaded thread with no active turn. |
| `historical_final` | latest bounded collab/sub-agent activity says completed/interrupted/shutdown/not found | Annotation only; it must not override live `ThreadStatus` when the thread is loaded. |

Parent `wait_agent` activity means the parent is waiting for a result; it is not proof that the child is waiting. Child status must come from the child thread's native status when available. Missing status data renders as `unknown`/empty detail, not as guessed running or idle.

## fork/140 first slice

The first implemented slice enriches the existing `/agent` picker instead of introducing a new overlay or app-server method. The native source is `AgentNavigationState` plus `AgentPickerThreadEntry`, refreshed through the existing `open_agent_picker` / `refresh_agent_picker_thread_liveness` path. The picker header now carries a bounded team/status summary derived from the same cache (`total`, `running`, `waiting`, `error`, `closed`, excluding the main thread), `SelectionItem.description` carries a scan-first summary (`status`, kind/role, path or short thread id), and `SelectionItem.selected_description` carries compact detail (`status`, current-view flag, kind, `Inspect` anchors, bounded recent activity, full thread id and `Actions`).

The first observational action remains existing `SelectAgentThread`: selecting a normal row watches that thread and does not inject input into the child. Search still includes the full thread id through the existing `search_value`, so replacing the row description with scan-first text does not remove thread-id findability. Mutating lifecycle actions are separate from normal rows; the current implemented mutations are the confirmed `interrupt one`, root/subtree-owner queue-only `send-message`, root/subtree-owner trigger-turn `follow-up` and confirmed `close one` action rows described below.

The second local slice removes the separate running path-backed MAv2 history-cell bypass in `open_agent_picker`. Those entries still get liveness from live `ThreadEventStore` state or the existing refresh path, but they now flow into the same `SelectionView` picker/window as other tracked threads. The generic backend liveness refresh is skipped for already recognized path-backed entries so a local live sub-agent is not incorrectly marked closed just because it does not have a loaded app-server thread record.

The current detail slice derives recent activity at picker-open time from the existing per-thread `ThreadEventStore`. It only reads `ItemStarted` and `ItemCompleted`, deduplicates item ids, caps the selected detail to three activity summaries, and uses the same `ThreadItem` summary helper as the legacy status-feed tests. The summary helper renders command names without command output, reasoning summaries without raw reasoning, tool names without raw tool JSON, and ignores user messages and hook prompts.

The current status slice uses one cached `AgentPickerThreadStatus` enum instead of separate running/closed booleans. It is derived from native `ThreadStatus` on `thread/read`, updated from `ThreadStatusChanged` notifications for tracked agent rows, and still accepts local live-turn hints for path-backed MAv2 agents. This lets the picker render `waiting approval`, `waiting user` and `error` without a parallel workbench status store.

The current team/status summary slice is intentionally presentation-only. It counts non-main cached picker entries by derived `AgentPickerThreadStatus`, groups `waiting approval` and `waiting user` as `waiting`, and renders a single bounded header line. It does not add a team file, mailbox, separate registry, new app-server method or `Ctrl+T` binding; `Ctrl+T` remains the native transcript overlay.

The current metadata hydration slice reuses existing `Thread` and TUI session metadata. `thread/read` and `ThreadStarted` update picker detail with `thread_note` and `agent_path` from `SessionSource::SubAgent(ThreadSpawn)`, plus thread `cwd` and `model_provider`. When a thread already has a loaded `ThreadSessionState` in this TUI process, `/agent` also hydrates `model`, `reasoning_effort` and `service_tier` from that native client session cache. Replay-only rows and app-server `thread/read` responses without session state leave those fields absent rather than guessing.

The current token anchor slice reads the latest `ThreadTokenUsageUpdated` notification already buffered for that thread and renders a bounded selected-detail summary: total tokens, last context size and context window when available. It does not query account-wide usage, does not synthesize values from status-line state, and does not create a persistent workbench token cache.

The first lifecycle slice adds `interrupt one` without adding a new runtime control path. `/agent` appends an action row only for interruptable path-backed sub-agent threads; selecting that row opens a confirmation popup with `Cancel` selected first and shows the cached `AgentPickerThreadStatus` as current/pre-interrupt status. Confirmation sends `AppEvent::InterruptAgentThreadConfirmed`, repeats the same preflight, submits `AppCommand::interrupt()` through existing `SubmitThreadOp` / app-server `turn/interrupt`, reports the cached previous status in success copy, then refreshes that row through the existing `thread/read` liveness path so the picker cache returns to the app-server `ThreadStatus` source of truth. The slice is intentionally narrower than the MAv2 model tool result: it stops the target turn but does not close/delete the thread and does not require app-server `turn/interrupt` to return an `InterruptAgentResult.previous_status` payload.

The close lifecycle slice adds confirmed `close one` as a native control projection, not a hidden-row UI cache. `/agent` appends `Close <agent>` only for owned path-backed spawned descendants whose picker status is not already closed. Selecting it opens a destructive confirmation popup with `Cancel` selected first and copy that names the current status, persisted spawn-edge close and live subtree shutdown. Confirmation sends experimental app-server `agent/close`; `ThreadManager::close_agent_from_workbench` parses author and target thread ids, denies self/root/pathless/outside-subtree targets, reads the previous native `AgentStatus`, delegates to `AgentControl::close_agent`, marks the picker entry closed and refreshes liveness through the existing app-server path. This is not `dismiss`: the thread remains available for replay and native resume paths.

The follow-up lifecycle slice adds root/subtree-owner trigger-turn follow-up as a native control projection over the existing MAv2 `followup_task` semantics. `/agent` appends `Follow up <agent>` only for idle owned path-backed spawned descendants of the current workbench owner. Selecting it opens `Follow up with agent`, then a `Start a follow-up turn?` confirmation that states the message is encrypted and starts the target when native capacity is available. Confirmation sends experimental app-server `agent/followup/send` with the active owner thread id, not always the primary thread id. `ThreadManager::send_inter_agent_followup` parses author and target thread ids, denies empty/self/root/pathless target/outside-subtree targets, rejects pathless non-root authors instead of treating them as root, requires the target to be idle/interrupted/completed rather than running, errored or closed, then delegates to `AgentControl::send_inter_agent_communication` with encrypted `InterAgentCommunication { trigger_turn: true }`. This is not queue-only send-message and it is not `turn/start`, `turn/steer` or raw app-server op injection.

The current UI-polish slice keeps the same `SelectionItem` and event flow but makes the selected detail easier to scan. Stable recovery fields live under `Inspect:` and executable UI affordances live under `Actions:`. This does not add new lifecycle authority: normal rows still use `SelectAgentThread`, and mutating workbench actions still require their own separate action row plus confirmation contract.

The current prompt/context slices add stable `Created:`/`Updated:` timestamps from native `Thread.created_at`/`Thread.updated_at`, one bounded `Prompt:` line under `Inspect:` when app-server `Thread.preview` is available, plus an optional `Context:` section derived from already-buffered `CollabAgentToolCall.prompt` events whose `receiver_thread_ids` include the selected thread. `SpawnAgent` renders as `Initial request`, `SendInput` renders as `Latest input`, and both are whitespace-normalized and capped. `Thread.preview`, `created_at` and `updated_at` are already part of native `thread/read` and `ThreadStarted` metadata, while collab prompts are app-server item events already present in `ThreadEventStore`. The workbench does not request `includeTurns`, does not scan child `UserMessage` items, does not read rollout raw operations, does not show prompts for other receivers and does not expose encrypted `InterAgentCommunication` content. The timestamp slice deliberately formats absolute UTC times instead of a live “age” counter so the picker does not need ticking state or a parallel activity cache.

## Lifecycle actions

Lifecycle actions are a projection over native MAv2 runtime state. They must use `AgentControl`, `SessionSource::SubAgent(ThreadSpawn)`, `AgentPath`, `SubAgentActivityEvent`, existing TUI thread selection and bounded thread-event summaries. They must not introduce OpenClaude task files, team storage, mailbox files, tmux/iTerm pane controls or workflow retry/skip storage.

### Action and state matrix

| Action | Applies to states | First implementation | Native path | State transition / UI result | Notes |
| --- | --- | --- | --- | --- | --- |
| `view` | any listed row, including closed/replay-only rows | Allowed | `/agent` picker row detail from `AgentNavigationState` and thread metadata | No runtime transition; selected row shows bounded detail. | Missing metadata renders as empty/unknown, not inferred. |
| `watch` / `select` | live, waiting, completed, interrupted, errored, replay-only | Allowed | `AppEvent::SelectAgentThread` -> existing thread switching | TUI foreground changes to the selected thread transcript. | `foreground` means watch/select only; it never sends direct input to the child. |
| `send_message` | known non-root MAv2 target with `agent_path` and current workbench owner ancestry | Implemented for root/subtree-owner workbench | TUI `OpenAgentMessagePrompt` / `SendAgentMessage` -> app-server `agent/message/send` -> `ThreadManager::queue_inter_agent_message` -> `AgentControl::send_inter_agent_communication`; model tool path remains MAv2 `send_message` -> `MessageDeliveryMode::QueueOnly` | Queues encrypted `InterAgentCommunication { trigger_turn: false }` without waking/starting the target. | TUI preflight allows descendants of the current path-backed workbench owner only and server preflight repeats ownership/path-backed-author checks. Must use queue-only semantics, not `turn/start`, `turn/steer`, trigger-turn `followup_task` or raw app-server `Op::InterAgentCommunication`. UI copy says `queued`. |
| `followup_task` | known idle non-root MAv2 target with `agent_path` and current workbench owner ancestry | Implemented for root/subtree-owner workbench | TUI `OpenAgentFollowupPrompt` / `FollowupAgentThreadConfirmed` -> experimental app-server `agent/followup/send` -> `ThreadManager::send_inter_agent_followup` -> `AgentControl::send_inter_agent_communication`; model tool path remains MAv2 `followup_task` -> `MessageDeliveryMode::TriggerTurn` | Queues encrypted `InterAgentCommunication { trigger_turn: true }` and starts the target when native capacity permits. | TUI preflight allows idle descendants of the current path-backed workbench owner only and server preflight repeats ownership/status/path-backed-author checks. It must not use queue-only semantics, `turn/start`, `turn/steer` or raw app-server `Op::InterAgentCommunication`. |
| `interrupt one` | non-root spawned target that is running or waiting | Implemented after confirmation | TUI `SubmitThreadOp` -> app-server `turn/interrupt` -> `Op::Interrupt`; MAv2 tool path remains `interrupt_agent` -> `AgentControl::interrupt_agent` | Sends `Op::Interrupt`, refreshes row status via `thread/read`, and keeps the thread available for watch/replay. | Not close/kill/delete. Main/root/self/missing-path/idle/closed/error/outside-subtree targets are denied before UI enablement and again before submit. |
| `wait` / `result` | waiting/running targets and completed mailbox notifications | Allowed as observational detail only | MAv2 `wait_agent`, mailbox notifications, `CollabWaiting*` events, bounded thread-event summaries | Workbench may indicate waiting, timed out or completed signal. | Native MAv2 does not have OpenClaude task result files. |
| `close one` | owned spawned target and live descendants | Implemented after destructive confirmation | TUI `OpenAgentCloseConfirmation` / `CloseAgentThreadConfirmed` -> experimental app-server `agent/close` -> `ThreadManager::close_agent_from_workbench` -> `AgentControl::close_agent`; persisted spawn-edge `Closed` and `shutdown_agent_tree` remain native runtime behavior | Marks one target spawn edge closed, shuts down target subtree and keeps the thread replayable/resumable. | Not dismiss/delete. Main/root/self/pathless/already-closed/outside-subtree targets are denied before UI enablement and again before submit. |
| `dismiss` | completed/interrupted/errored rows | Deferred | No separate native UI-hide source selected | No first-slice transition. | Hiding rows can undermine replay/recovery; do not add a parallel hidden-row cache. |
| `retry` | completed/error rows | Deferred | No native MAv2 retry primitive selected | No first-slice transition. | Future retry should be follow-up or spawn-from-existing native action with explicit idempotency. |
| `stop all` | multiple live/waiting agents | Deferred | No broad native workbench primitive selected | No first-slice transition. | Bulk action needs root/subtree scope, count summary and destructive confirmation tests. |

`CollabAgentToolCall::CloseAgent` rendering in transcript/history summaries is historical tool output only. It is not a `/agent` workbench close action and must not be treated as UI close support.

### Ownership rules

Ownership is based on canonical `AgentPath` and the root-scoped `AgentControl` tree. The root thread may watch/select every known row in its tree, may request `interrupt one` or `close one` for one spawned non-root target, may queue `send_message` for one eligible spawned non-root target and may start a follow-up turn for one idle eligible spawned non-root target. A non-root agent may interrupt, close, queue `send_message` or follow up a descendant whose `AgentPath` is inside its own subtree. `ThreadManager::queue_inter_agent_message` and `ThreadManager::send_inter_agent_followup` enforce descendant-only targeting. Sibling, ancestor and unrelated subtree targets are denied. Self-target is denied. The root path is not a spawned agent and is denied for interrupt/close/message-style/follow-up actions. Pathless non-root owners are denied for message/follow-up instead of being treated as root.

The first TUI lifecycle slice applies the same ownership rules before displaying an enabled interrupt action and repeats the preflight before submitting. Disabled interrupt targets are omitted from the first action-row implementation and covered by focused preflight tests for root target, self target, outside subtree, missing agent path and unavailable status. Future disabled-action rows can add compact reason copy once broader action families exist.

### Confirmation and destructive policy

`view`, `watch` and `select` need no confirmation. `interrupt one` uses a lightweight confirmation because it mutates a running child turn: title `Interrupt this agent?`, target subtitle `{label} · {agent_path} · current status: {status}`, default row `Cancel`, and action row `Interrupt agent` with copy `Current status: {status}. Stop the current turn only. It does not close or delete the thread.` The confirmed action names the target label/path, reports the cached previous status after submit, and uses existing app-server thread interrupt, not a new workbench runtime path.

`send-message` is non-destructive and uses a prompt rather than a destructive confirmation: action label `Send message`, prompt title `Message agent`, placeholder `Write a message for this agent`, target context `{label} · {agent_path}`, success copy `Message queued for <agent>.`, empty-input error `Cannot send an empty agent message.`, and unsupported-runtime diagnostic `Agent messaging is not supported by the connected app-server. Restart Codex from this fork build.` `follow-up` starts work and therefore uses prompt plus explicit confirmation: action label `Follow up`, prompt title `Follow up with agent`, placeholder `Write the next task for this agent`, confirmation title `Start a follow-up turn?`, default row `Cancel`, action row `Start follow-up` with copy `Queue an encrypted follow-up and start this agent when capacity is available.`, success copy `Follow-up started for <agent>.`, hint `The agent may request approvals under its existing sandbox.`, empty-input error `Cannot send an empty agent follow-up.`, and unsupported-runtime diagnostic `Agent follow-up is not supported by the connected app-server. Restart Codex from this fork build.` `close one` uses destructive confirmation: title `Close this agent?`, target subtitle `{label} · {agent_path} · current status: {status}`, default row `Cancel`, action row `Close agent` with copy `Current status: {status}. Mark this spawn edge closed and shut down the live target and descendants.`, success copy `Closed <agent> (previous status: <status>).`, resume hint `The thread remains available for replay and native resume paths.`, and unsupported-runtime diagnostic `Agent close is not supported by the connected app-server. Restart Codex from this fork build.` Future `stop all` confirmation must show the number of affected running/waiting agents and the scope (`root tree` or `{agent_path} subtree`). Future `retry` must not reuse OpenClaude workflow retry wording unless it maps to a native Codex action.

### Permissions, security and resume

Lifecycle actions must not widen cwd, workspace roots, sandbox, approval policy, MCP tools, model/provider settings or role-template permissions. Sending/followup must preserve encrypted `InterAgentCommunication` behavior and must not write plaintext prompts into session logs beyond existing bounded previews. Detail/result surfaces must bound recent activity and redact raw reasoning, secrets, encrypted content and long tool output.

Resume compatibility is part of the contract. Old sessions may lack `agent_path`, `thread_note`, cwd, role or recent activity; those rows remain watchable when a native thread id exists, but destructive actions stay disabled until ownership can be proven. Close/dismiss/retry must not be implemented by a parallel workbench cache; if a persisted state is needed, it must be native thread-spawn edge state or another existing backward-compatible source.

## OpenClaude comparison

OpenClaude's background task UI is a quality reference for density and recovery anchors, but its storage/control plane is not portable. Codex must not copy `.claude/teams`, file mailbox, task files, tmux/iTerm pane controls or OpenClaude markdown agent profiles. Planner/implementer/verifier-style labels should be ordinary role template names, not hardcoded workbench enums.

## Compatibility

Old sessions can be missing `agent_path`, `agent_role`, `thread_note`, cwd, model/reasoning session state or token usage notifications. The UI must render controlled empty states. Existing `/agent` and `/subagents` thread switching must continue to work. `/agents` is already the alias for role templates and must not be hijacked by the workbench.
