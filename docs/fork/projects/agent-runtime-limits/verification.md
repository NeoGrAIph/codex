# Agent runtime limits verification

## Required checks

- `cargo check -p codex-core -p codex-thread-store -p codex-protocol -p codex-rollout`
- `cargo check --tests -p codex-core`
- `just fmt`
- `just test -p codex-core multi_agent_v2_runtime_defaults_use_fork_limits`
- `git diff --check`

## Scenario matrix

- Defaults: assert depth `2`, default spawned agents `12`, MAv2 concurrency-derived max spawned threads `12`, and default wait timeout `300_000`.
- Regression: existing config validation and user override paths are unchanged because only constants changed.

## Known gaps

No stress test for twelve concurrent spawned agents is included in this iteration.
