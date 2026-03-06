# ThreadSpawn Agent Persona

## Feature passport

- Code name: `ThreadSpawn Agent Persona`.
- Status: `implemented`.
- Goal: ввести `agent_persona` как canonical persisted/wire metadata для spawned thread-spawn sub-agent, не смешивая её с `agent_role` и `agent_nickname`.
- Scope in:
- `codex-rs/protocol/src/protocol.rs`
- `codex-rs/core/src/tools/handlers/multi_agents.rs`
- `codex-rs/core/src/agent/control.rs`
- `codex-rs/core/src/rollout/*`
- `codex-rs/state/*`
- `codex-rs/app-server-protocol/src/protocol/v2.rs`
- `codex-rs/app-server/src/codex_message_processor.rs`
- `codex-rs/app-server/README.md`
- Scope out:
- новые RPC методы;
- thread note;
- SAW.

## Problem statement

До этого изменения у thread-spawn sub-agent были только:

- ingress selector роли (`agent_type`);
- canonical role metadata (`agent_role`);
- runtime/display nickname (`agent_nickname`).

Этого недостаточно для fork persona layer:

- persona нужна как отдельный selector system prompt;
- persona defaults должны переживать spawn/resume/fork/list/read;
- persona нельзя смешивать с display nickname.

## Contract

`spawn_agent` получает новый optional аргумент:

- `agent_persona`

`SubAgentSource::ThreadSpawn` получает новые optional поля:

- `agent_persona`
- `allow_list`
- `deny_list`

`SessionMeta` получает top-level mirror:

- `agent_persona`

App-server `Thread` получает top-level convenience field:

- `agentPersona`

Canonical source of truth:

- `thread.source.subAgent.thread_spawn.agent_persona`

`Thread.agentPersona` и `SessionMeta.agent_persona` считаются только convenience mirrors.

## Semantics

- `agent_type` — selector роли на входе;
- `agent_role` — canonical persisted identity роли;
- `agent_persona` — canonical persisted identity персоны;
- `agent_nickname` — только display/runtime nickname.

Если nested `thread_spawn.agent_persona` присутствует, он всегда имеет приоритет над top-level mirror.

## Compatibility guarantees

- старые payload без `agent_persona`, `allow_list`, `deny_list` должны безопасно читаться как `None`;
- `agent_role` продолжает принимать legacy alias `agent_type`;
- старые rollout/state данные не требуют backfill для корректного чтения.

## Persistence and propagation

`agent_persona` должен сохраняться через все runtime boundaries:

- spawn source construction;
- rollout recorder;
- sqlite state persistence;
- rollout/state reconstruction;
- app-server `thread/list`;
- app-server `thread/read`;
- app-server `thread/resume`;
- app-server `thread/fork`;
- app-server `thread/unarchive`.

Дополнительно `agent_persona` проходит через collab/event surfaces, которые уже несут identity spawned agent'ов:

- `CollabAgentRef`
- `CollabAgentStatusEntry`
- `CollabAgentSpawnEndEvent`
- `CollabAgentInteractionEndEvent`
- `CollabCloseEndEvent`
- `CollabResumeBeginEvent`
- `CollabResumeEndEvent`

`allow_list` и `deny_list` в этом commit:

- входят в canonical `ThreadSpawn` wire format;
- проходят через source merge и app-server conversion;
- остаются nested metadata, а не top-level `Thread` fields.

## API notes

На `thread/*` v2 surface:

- `agentPersona` доступен как convenience field на `Thread`;
- canonical nested metadata остаётся в `thread.source.subAgent.thread_spawn`;
- nested source также несёт `allowList` и `denyList`.

Collab-specific `threadHistory` items пока не получают отдельные top-level persona mirrors в v2 API. На этой стадии canonical persona metadata остаётся в raw protocol events и в canonical nested thread source.

## Validation matrix

- protocol tests на backward-compatible decode и roundtrip новых полей;
- core tests на spawn/resume preservation;
- protocol/core tests на backward-compatible collab payload enrichment;
- state tests на sqlite persistence для `agent_persona`;
- app-server tests на `thread/list`, `thread/read`, `thread/fork`, `thread/unarchive`;
- schema regeneration для `app-server-protocol`.
