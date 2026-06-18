# Subagent Workbench project

## Current status

`subagent-workbench` is the central project for turning the existing `fork/140` sub-agent navigation into a scan-first agent workbench. Current native coverage is partial but expanded locally: `/agent` and `/subagents` can switch/watch active sub-agent threads, `AgentNavigationState` tracks ordering, labels and derived `AgentPickerThreadStatus`, the `/agent` picker now has a bounded agents summary header, scan-first row summaries and selected detail, selected detail includes `thread_note`, cwd, model provider, stable `Created`/`Updated` UTC timestamps, session-backed model/reasoning/service tier, latest per-thread token usage, bounded prompt/context/plan/activity anchors from native `Thread`/`ThreadSessionState`/`ThreadEventStore`/`ThreadItem` sources, running path-backed MAv2 agents open in the same picker/workbench window, and protocol/runtime carry `agent_path`, `agent_role`, `agent_nickname` and `thread_note`. Implemented lifecycle actions are confirmed `interrupt one` with cached pre-interrupt status copy, root/subtree-owner queue-only `send-message`, root/subtree-owner trigger-turn `follow-up` and confirmed `close one` over native `AgentControl::close_agent`. Missing pieces are richer role/profile hydration beyond cached labels, historical hotkey parity for SAW because `Ctrl+T` remains the native transcript overlay, and deferred lifecycle actions such as dismiss-only hiding, retry and stop-all.

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
- Protocol/persistence: `codex-rs/protocol/src/protocol.rs`, app-server `Thread`/`ThreadStatus`/`ThreadItem`.
- TUI state: `codex-rs/tui/src/app/agent_navigation.rs`, `loaded_threads.rs`, `session_lifecycle.rs`, `multi_agents.rs`; legacy `agent_status_feed.rs` remains test-only reference for bounded activity rendering.
- UI command routing: `/agent`, `/subagents`, `AppEvent::OpenAgentPicker`, `AppEvent::SelectAgentThread`.
- Adjacent feature inputs: `agent-role-templates`, `thread-notes`, `spawn-agent-cwd`, `subagent-policy-and-ownership`, `agent-runtime-limits`.

## First target

The first implementation target is implemented as an observational workbench slice: `/agent` picker rows expose scan-first status/kind/path/thread summary and selected detail through existing `SelectionView`, including running path-backed MAv2 agents that previously used the `AgentStatusHistoryCell` path. The current detail slices add stable created/updated timestamps plus bounded prompt/context/plan/token/activity anchors from native thread metadata, session state and buffered notifications while keeping command output, raw reasoning, tool JSON, raw user prompts and encrypted inter-agent content out of the panel. The first mutating actions are confirmed `interrupt one` over existing `turn/interrupt`, root/subtree-owner queue-only `send-message` over experimental `agent/message/send` -> `ThreadManager::queue_inter_agent_message` -> native `AgentControl::send_inter_agent_communication`, root/subtree-owner trigger-turn `follow-up` over experimental `agent/followup/send` -> `ThreadManager::send_inter_agent_followup` -> native `AgentControl::send_inter_agent_communication`, and confirmed `close one` over experimental `agent/close` -> `ThreadManager::close_agent_from_workbench` -> native `AgentControl::close_agent`; broader lifecycle actions remain disabled or documented until their native contracts, ownership rules and destructive confirmations are implemented and tested.
