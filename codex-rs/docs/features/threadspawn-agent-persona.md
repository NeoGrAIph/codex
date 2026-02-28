# ThreadSpawn Agent Persona Metadata

## Summary

This feature makes selected agent personas first-class metadata in the multi-agent runtime.

When an agent is spawned from role templates (`.agents/*.md`), the selected persona label
(`agent_nickname` entry from YAML, for example `runner`) is now propagated as
`agent_persona` through protocol/state/runtime/app-server/TUI layers.

The goal is to keep agent identity stable and inspectable across:

- spawn events,
- wait/list/read APIs,
- resume from rollout/state DB,
- UI labels in agent picker and collaboration transcript.

## Contract

## ThreadSpawn Source

`SubAgentSource::ThreadSpawn` now includes optional:

- `agent_persona: Option<String>`

`agent_persona` is independent from:

- `agent_nickname` (runtime-generated display nickname, for example `Ptolemy`),
- `agent_role` (role key, for example `worker`).

Compatibility note:

- `agent_role` keeps legacy deserialize alias `agent_type`.

## Session Metadata

`SessionMeta` includes optional:

- `agent_persona: Option<String>`

This ensures rollout persistence and replay/resume compatibility.

## Collaboration Events

Persona metadata is carried in collab event payloads:

- `CollabAgentRef.agent_persona`
- `CollabAgentStatusEntry.agent_persona`
- `CollabAgentSpawnEndEvent.new_agent_persona`
- `CollabAgentInteractionEndEvent.receiver_agent_persona`
- `CollabCloseEndEvent.receiver_agent_persona`
- `CollabResumeBeginEvent.receiver_agent_persona`
- `CollabResumeEndEvent.receiver_agent_persona`

## State DB Persistence

State DB thread metadata includes:

- `threads.agent_persona` column (`0019_threads_agent_persona.sql`).

Extraction and upsert/read paths propagate this field, so resumed agents can restore persona metadata.

## Runtime Behavior

## Spawn

- Persona is resolved from role templates (`role_templates`) and attached to `ThreadSpawn`.
- Persona metadata is exposed through agent control metadata methods used by collab tools.

### Model Resolution Order

`spawn_agent` resolves model in this order:

1. `model` from tool arguments (explicit override),
2. persona-level `model` from template (`agent_names[].model`),
3. role-level `model` from template frontmatter,
4. parent thread model (inherited).
5. catalog default model (if none of the above resolved one).

`reasoning_effort` never selects model on its own. If only `reasoning_effort` is provided, model
still follows the chain above.

For transparency, `spawn_agent` result includes:

- `requested_model` (raw explicit model override from arguments, if any),
- `model_source` (`explicit_argument` | `template_persona` | `template_role` | `inherited_parent` | `catalog_default`),
- `model` (effective model used by spawned thread).

### Spawn Validation

`spawn_agent` validates model/effort compatibility before spawn:

- unknown model fails with an `Available models: ...` error,
- unsupported `reasoning_effort` for selected model fails with
  `Supported efforts: ...` for that exact model.

This validation applies equally to template-provided and explicit overrides.

## Resume

- When resuming a `ThreadSpawn` agent, runtime restores persona from state DB if available.
- For old rows/rollouts without persona, behavior remains valid (`None`).

## TUI Behavior

Persona-aware labels are used in:

- collaboration transcript entries,
- agent picker entries,
- waiting/finished agent status lines.

Display rule:

- non-default persona is shown as a prefix before runtime nickname (for example `Runner Ptolemy [worker]`),
- `default` persona is suppressed in labels to avoid noise.

TUI also keeps model/reasoning continuity on fork/resume and active-thread updates so the
session-configured state reflects current runtime settings.

## App-server Behavior

`with_thread_spawn_agent_metadata(...)` merges persona with other thread-spawn metadata.
This preserves persona when reconstructing thread source from rollout/state-derived summaries.

`thread/list` and related thread summaries continue exposing top-level `agentNickname`/`agentRole`.
Persona is carried in `source` when source is `subagent -> thread_spawn`.

## Runtime Normalization Outside Spawn

Core session configuration normalizes unsupported `reasoning_effort` to a model-supported fallback
when reconciling session settings with a model. This protects runtime state consistency.
`spawn_agent` itself remains strict and rejects incompatible explicit/template model+effort
combinations during spawn validation.

## Backward Compatibility

- `agent_persona` is optional everywhere.
- Existing sessions without persona remain readable.
- `agent_role` keeps read compatibility alias for `agent_type` in protocol payloads.

## Related Feature

Role/persona template authoring and validation rules are documented in:

- `docs/features/agent-role-templates.md`
- `docs/features/saw.md`
