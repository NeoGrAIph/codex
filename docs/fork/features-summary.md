# Fork Feature Summary For rust-v0.130.0

| Feature | Status | App-server impact | Primary risk |
| --- | --- | --- | --- |
| `subagent-cwd` | Implemented | None in v1 | Incorrect cwd-scoped config/environment rebuild |
| `thread-note` | Implemented | Additive required-nullable metadata fields | Accidental model-facing exposure |
| `agent-role-templates` | Implemented | Additive optional `agentPersona` and source metadata | Breaking native TOML role precedence or policy enforcement |
| `agent-switch-viewport` | Deferred / Obsoleted by upstream 0.130 native switch path | None | Re-implementing behavior already covered by upstream |
| `agents-overlay` | Implemented | None in v1 | Creating a second thread-switching path |

Recommended delivery sequence:

1. Implement `subagent-cwd` so spawned agents can have correct session roots.
2. Implement `agent-role-templates` on top of cwd-correct role discovery.
3. Implement `thread-note` metadata and app-server visibility.
4. Do not implement `agent-switch-viewport` for `fork/130`; upstream 0.130 already provides the
   native switch/replay path this fork feature was meant to supply.
5. Maintain `agents-overlay` as a projection over the native runtime and TUI switch paths.
