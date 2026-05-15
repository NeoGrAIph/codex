# Fork Projects

This directory contains implementation dossiers for the `fork/130` feature batch.

| Project | Depends on | Notes |
| --- | --- | --- |
| `subagent-cwd` | none | Foundation for cwd-scoped config and role discovery |
| `agent-role-templates` | `subagent-cwd` | Uses child-cwd-scoped role/template discovery |
| `thread-note` | none | Only feature in this batch with v1 app-server schema additions |
| `agent-switch-viewport` | none | Deferred; upstream 0.130 native switch path remains authoritative |
| `agents-overlay` | optional `thread-note` / `agent-role-templates` metadata | TUI projection over existing thread state |

Each project directory contains:

- `README.md`: status, links, implementation map, locked decisions.
- `design.md`: canonical state, data flow, invariants, tradeoffs.
- `verification.md`: scenarios, commands, and known coverage gaps.
