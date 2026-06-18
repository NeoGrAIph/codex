# Agent runtime limits verification

## Required checks

- `cargo check -p codex-core -p codex-thread-store -p codex-protocol -p codex-rollout`
- `cargo check --tests -p codex-core`
- `just fmt`
- `just test -p codex-core multi_agent_v2_default_session_thread_cap_counts_root multi_agent_v2_runtime_defaults_use_fork_limits multi_agent_v2_wait_agent_uses_configured_default_timeout spawn_agent_v2_default_limit_allows_twelve_spawned_agents multi_agent_v2_wait_agent_schema_uses_configured_fork_timeout_defaults`
- `git diff --check`

## Scenario matrix

- Defaults: assert depth `2`, default spawned agents `12`, MAv2 concurrency-derived max spawned threads `12`, and default wait timeout `300_000`.
- Runtime capacity: native `AgentControl` accepts twelve MAv2 spawned agents under the default cap and rejects the thirteenth with `AgentLimitReached { max_threads: 12 }`.
- Schema projection: native `spec_plan` passes `MultiAgentV2Config` wait options into `WaitAgentHandlerV2`, so model-visible `wait_agent.timeout_ms` advertises default `300000`, min `10000`, max `3600000`.
- Regression: existing config validation and user override paths are unchanged because only constants changed.

## Known gaps

No full end-to-end stress test with twelve actively working agents is included in this iteration. The native concurrency limiter path is covered by a focused `AgentControl` test that reserves twelve spawned MAv2 slots and rejects the thirteenth.

## Verification log

- 2026-06-18: `just test -p codex-core multi_agent_v2_default_session_thread_cap_counts_root multi_agent_v2_runtime_defaults_use_fork_limits` прошёл 2/2 после исправления stale expectation в config test: default MAv2 session cap теперь подтверждает root-inclusive `13` slots as `12` spawned agents.
- 2026-06-18: Added `spawn_agent_v2_default_limit_allows_twelve_spawned_agents` to cover the runtime limiter path for the fork default: twelve spawned MAv2 agents are accepted and the thirteenth is rejected with `AgentLimitReached { max_threads: 12 }`.
- 2026-06-18: Spark test-runner rerun confirmed `just test -p codex-core multi_agent_v2_default_session_thread_cap_counts_root multi_agent_v2_runtime_defaults_use_fork_limits spawn_agent_v2_default_limit_allows_twelve_spawned_agents`; scoped `git diff --check` also passed.
- 2026-06-18: Added `multi_agent_v2_wait_agent_schema_uses_configured_fork_timeout_defaults` to cover the model-visible schema projection path for fork wait defaults.
- 2026-06-18: Spark test-runner current-state rerun confirmed runtime defaults, limiter behavior and schema projection together: `just test -p codex-core multi_agent_v2_default_session_thread_cap_counts_root multi_agent_v2_runtime_defaults_use_fork_limits multi_agent_v2_wait_agent_uses_configured_default_timeout spawn_agent_v2_default_limit_allows_twelve_spawned_agents multi_agent_v2_wait_agent_schema_uses_configured_fork_timeout_defaults` passed 5/5, and scoped `git diff --check` passed.
