# Subagent Workbench project

## Current status

`subagent-workbench` is the central project for turning the existing `fork/140` sub-agent navigation into a scan-first agent workbench. Current native coverage is partial but expanded locally: `/agent` and `/subagents` can switch/watch active sub-agent threads, `AgentNavigationState` tracks ordering, labels and derived `AgentPickerThreadStatus`, the `/agent` picker now has a bounded agents summary header, scan-first row summaries and selected detail, selected detail includes explicit `Connection:` and Enter action copy over native `SelectAgentThread`, repeated `Ctrl+T` from the native transcript overlay switches into Agent Window, `thread_note`, cwd, model provider, stable `Created`/`Updated` UTC timestamps, session-backed model/reasoning/service tier, persisted role-applied tool/action policy summaries from native `ThreadSpawn` snapshots, latest per-thread token usage, bounded prompt/context/plan/activity anchors from native `Thread`/`ThreadSessionState`/`ThreadEventStore`/`ThreadItem` sources, running path-backed MAv2 agents open in the same picker/workbench window, and protocol/runtime carry `agent_path`, `agent_role`, `agent_nickname`, `thread_note`, `initial_task` for new MAv2 spawns and `agentHidden`. Implemented lifecycle actions are confirmed `interrupt one` with cached pre-interrupt status copy, root/subtree-owner queue-only `send-message`, root/subtree-owner trigger-turn `follow-up`, confirmed `close one` over native `AgentControl::close_agent`, non-destructive `dismiss` over persisted rollout `SessionMeta.agent_hidden`, confirmed native `retry` over durable `ThreadSpawn.initial_task`, and scoped destructive `stop all` for live descendants. Remaining workbench polish should focus on final cross-feature acceptance and any deliberately scoped role-definition detail that can be derived from native role/project docs without creating a role registry in the workbench.

## Canonical references

| Artifact | Purpose |
| --- | --- |
| `docs/fork/features/subagent-workbench.md` | Feature contract and historical release/commit references. |
| `docs/fork/research/agent-window/README.md` | Historical fork visual snapshots and accepted Agent window reference. |
| `docs/fork/research/0.140.0/openclaude-agent-ux-runtime.md` | OpenClaude-inspired current research and native source map. |
| `docs/fork/research/0.140.0/openclaude-agent-ux-runtime-backlog.md` | Implementation lanes, verification gates and agent coordination. |
| `docs/fork/research/multi-agents/architecture.md` | Current MAv2 architecture context. |

## Implementation surfaces

- Runtime: `codex-rs/core/src/agent/control.rs`, `codex-rs/core/src/tools/handlers/multi_agents_v2/*`.
- Protocol/persistence: `codex-rs/protocol/src/protocol.rs`, `SessionSource::SubAgent(ThreadSpawn.initial_task)`, thread-store `ThreadMetadataPatch.agent_hidden`, app-server `Thread`/`ThreadStatus`/`ThreadItem`/`agentHidden`.
- TUI state: `codex-rs/tui/src/app/agent_navigation.rs`, `loaded_threads.rs`, `session_lifecycle.rs`, `multi_agents.rs`; legacy `agent_status_feed.rs` remains test-only reference for bounded activity rendering.
- UI command routing: `/agent`, `/subagents`, `AppEvent::OpenAgentPicker`, `AppEvent::SelectAgentThread`.
- Adjacent feature inputs: `agent-role-templates`, `thread-notes`, `spawn-agent-cwd`, `subagent-policy-and-ownership`, `agent-runtime-limits`.

## First target

The first implementation target is implemented as an observational workbench slice: `/agent` picker rows expose scan-first status/kind/path/thread summary and selected detail through existing `SelectionView`, including running path-backed MAv2 agents that previously used the `AgentStatusHistoryCell` path. The current detail slices add stable created/updated timestamps plus bounded prompt/context/plan/token/activity anchors from native thread metadata, session state and buffered notifications while keeping command output, raw reasoning, tool JSON, raw user prompts and encrypted inter-agent content out of the panel. The current mutating actions are confirmed `interrupt one` over existing `turn/interrupt`, root/subtree-owner queue-only `send-message` over experimental `agent/message/send` -> `ThreadManager::queue_inter_agent_message` -> native `AgentControl::send_inter_agent_communication`, root/subtree-owner trigger-turn `follow-up` over experimental `agent/followup/send` -> `ThreadManager::send_inter_agent_followup` -> native `AgentControl::send_inter_agent_communication`, confirmed `close one` over experimental `agent/close` -> `ThreadManager::close_agent_from_workbench` -> native `AgentControl::close_agent`, non-destructive `dismiss` over experimental `agent/dismiss` -> `ThreadManager::dismiss_agent_from_workbench` -> rollout `SessionMeta.agent_hidden`, confirmed `retry` over experimental `agent/retry` -> `ThreadManager::retry_agent_from_workbench` -> native `AgentControl::spawn_agent_with_metadata` with durable `SessionSource::SubAgent(ThreadSpawn.initial_task)`, and scoped destructive `stop all` over experimental `agent/stopAll` -> `ThreadManager::stop_all_agents_from_workbench` -> native `AgentControl::close_agent` for each affected live descendant. `stop all` creates an observable partial-failure result instead of pretending a bulk action is atomic.
