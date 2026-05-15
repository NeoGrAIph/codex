# Fork Documentation

This directory is the source of truth for fork-specific behavior that is not plain upstream
`openai/codex` behavior.

## Baseline

- Target release: `rust-v0.130.0`
- Baseline commit: `58573da43ab697e8b79f152c53df4b42230395a8`
- Working branch: `fork/130`
- Baseline branch: `fork/130-upstream`

## Current Feature Batch

The `0.130.0` feature batch is documented in three layers:

- `features/<code-name>.md`: user/developer contract for a fork feature.
- `projects/<code-name>/`: implementation dossier for the feature.
- `research/0.130.0/<code-name>.md`: release-specific gap analysis and risky integration points.

Implementation order:

1. `subagent-cwd`
2. `agent-role-templates`
3. `thread-note`
4. `agent-switch-viewport`
5. `agents-overlay`

The order keeps session-root and role/template source-of-truth work ahead of metadata display and
TUI projections.

## Compatibility Rule

Stable `codex app-server` API is a client contract. Prefer TUI-local or core-local
implementation when possible. If app-server payloads change, use additive optional fields,
regenerate schemas, and document old/new client behavior.
