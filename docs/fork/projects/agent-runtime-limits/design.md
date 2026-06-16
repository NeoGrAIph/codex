# Agent runtime limits design

## Canonical state

Runtime limits stay in existing `Config` and `MultiAgentV2Config` fields. The fork changes defaults only, so existing user config remains authoritative when explicitly set.

## Data flow

`DEFAULT_AGENT_MAX_THREADS` feeds V1/global agent limits. `DEFAULT_MULTI_AGENT_V2_MAX_CONCURRENT_THREADS_PER_SESSION` feeds MAv2 session concurrency and is set to 13 to represent root plus 12 spawned agents. `DEFAULT_AGENT_MAX_DEPTH` feeds spawn depth checks. `DEFAULT_MULTI_AGENT_V2_DEFAULT_WAIT_TIMEOUT_MS` feeds `WaitAgentTimeoutOptions`.

## Invariants

- Existing config validation still enforces min/default/max timeout consistency.
- User overrides continue to win over defaults.
- MAv2's root-inclusive concurrency model is documented as an intentional fork divergence.

## Tradeoffs

The hard max wait timeout remains upstream `3_600_000` ms. This avoids increasing long-running wait blast radius while restoring the fork's practical default wait window.
