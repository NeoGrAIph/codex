# run_skill_script tool

## Feature passport

- Code name: `run-skill-script`
- Status: первая итерация переноса на `fork/140` реализована и локально проверена.
- Goal: дать agents инструмент запуска helper scripts из enabled skills через native unified exec path.
- Scope in: `run_skill_script` spec/handler, enabled skill resolution, local `scripts/` path validation, unified exec delegation, docs, focused tests.
- Scope out: новая система skills, plugin installation, remote/non-local skill filesystem execution, отдельные permission knobs, app-server protocol/schema changes.

## Как работает для пользователя

Agent может вызвать `run_skill_script` с enabled skill name или path to `SKILL.md`, relative script path под `scripts/` и аргументами. Handler валидирует, что skill enabled, script path остаётся внутри `scripts/`, script существует локально и является файлом, а selected environment является primary local environment текущего turn. Затем handler делегирует запуск в native `exec_command`.

Польза: skill может предоставлять подготовленный helper, но execution по-прежнему проходит через unified exec: approval policy, sandbox, Bash hook lifecycle, telemetry, output truncation и process management остаются upstream-owned. PreToolUse/PostToolUse payload для `run_skill_script` использует native Bash hook contract `{ "command": ... }`; PreToolUse rewrite меняет delegated exec command ровно для этого tool call.

## Branches and commits

| Branch | Baseline | Commit | Date | Subject | Роль |
| --- | --- | --- | --- | --- | --- |
| `fork/multi-agent` | `fork/colab-agents` lineage | `521c2a9978` | 2026-02-09 | `core: add run_skill_script tool; bump wait default` | Добавлен handler/spec и template wiring для `run_skill_script`. |
| `fork/multi-agent` | `fork/colab-agents` lineage | `ab35fe2fc1` | 2026-02-09 | `core: expose run_skill_script under unified_exec` | Tool выведен через unified exec. |

## Implementation notes

Diff затрагивал tool handler/spec, app-server schema artifacts, core templates and tests. В subject одного commit также есть wait tuning; это учтено отдельно в `agent-runtime-limits`.

## Native coverage in rust-v0.140.0

Status: `implemented-first-iteration`. Native release имеет skill metadata/resource surfaces, `skills.list`, `skills.read`, `SkillMetadata`, `SkillDependencies`, implicit skill invocation detection and ordinary `exec_command`/shell execution. Fork iteration добавляет отдельный model-visible `run_skill_script`, но deliberately delegates process execution to `ExecCommandHandler` and exposes equivalent Bash hook payloads on the outer tool so registry hooks run exactly once.

## Porting/current-state notes

Current upstream skills/plugin architecture не имеет dedicated script execution surface. Fork добавляет минимальный upstream-shaped adapter: skill/script validation в handler, outer registry hook payloads matching Bash, execution через native unified exec. Non-primary or remote environments fail fast with a controlled diagnostic instead of validating a host path and executing in another filesystem.

## Integration and compatibility

- Native source of truth: `codex-rs/core/src/tools/spec_plan.rs`, `ToolRegistry`, `ToolExposure`.
- Skill source: `TurnContext.turn_skills.outcome` and enabled `SkillMetadata`.
- Execution owner: `codex-rs/core/src/tools/handlers/unified_exec/exec_command.rs`.
- Producers: unified exec tool planning adds `run_skill_script` only when environment-backed unified exec is available.
- Consumers: model-visible `run_skill_script`; delegated native `exec_command` handles actual process launch.
- Permission/security: no new sandbox override fields; path traversal and symlink escape are rejected before exec delegation; only primary local environment is accepted in the first iteration; approval/sandbox and process execution remain native.
- Hooks: `RunSkillScriptHandler` publishes Bash PreToolUse/PostToolUse payloads from the outer registry dispatch, supports PreToolUse command rewrite through a hidden rewritten-command field, and delegates exactly one process execution to `ExecCommandHandler`.
- Persistence/resume: no persisted state or rollout format changes.
- Intentionally unaffected: skill loading, plugin installation, implicit skill invocation, app-server protocol/schema, legacy shell-only mode.

## Verification matrix

- `cargo check -p codex-core`: compile handler/spec wiring and unified exec delegation.
- `just test -p codex-core run_skill_script`: focused handler and integration coverage for hook payloads, environment rejection and real helper execution.
- `git diff --check`: whitespace guard.

## Doc changelog

- 2026-06-18: Актуализирован verification evidence для `fork/140`: `cargo check -p codex-core` и focused `codex-core` MCP/run_skill pass 15/15.
- 2026-06-17: Зафиксирована первая `fork/140` итерация: `run_skill_script` validates enabled local skill scripts, rejects non-primary/remote environments, preserves Bash hook lifecycle on the outer tool dispatch and delegates execution to unified exec.
