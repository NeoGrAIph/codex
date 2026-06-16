# run_skill_script design

## Canonical state

Skill inventory remains owned by `TurnContext.turn_skills.outcome`. Process execution remains owned by `ExecCommandHandler` and `UnifiedExecProcessManager`. The fork adds an adapter tool that maps a validated enabled skill script to a native `exec_command` invocation and exposes Bash hook payloads from the outer registry dispatch.

## Native propagation path

- Source of truth: `ToolRegistry` construction in `codex-rs/core/src/tools/spec_plan.rs`.
- Tool producer: `add_shell_tools` registers `RunSkillScriptHandler` only in `ConfigShellToolType::UnifiedExec`.
- Skill producer: loaded skills in `TurnSkillsContext`.
- Execution projection: `RunSkillScriptHandler` rewrites to `exec_command` with `cmd`, `workdir`, optional primary-local `environment_id`, `yield_time_ms` and `max_output_tokens`.
- Hook projection: `RunSkillScriptHandler` publishes Bash PreToolUse/PostToolUse payloads as `{ "command": ... }`; PreToolUse rewrite is stored in a hidden handler argument and becomes the delegated exec `cmd`.
- Permission/sandbox projection: delegated `ExecCommandHandler` applies approval policy, turn permissions, sandbox permissions, telemetry, process tracking and output truncation.

## Data flow

1. Turn tool planning confirms an environment-backed unified exec surface exists.
2. `spec_plan` registers `run_skill_script` with environment parameters matching the turn environment mode; the first iteration accepts only omitted `environment_id` or the primary local environment id.
3. The model calls `run_skill_script`.
4. The handler resolves the requested enabled skill by exact skill name or `SKILL.md` path.
5. The handler rejects empty, disabled, missing or ambiguous skills.
6. The handler resolves the script under the skill's local `scripts/` directory, rejecting absolute paths, `..`, symlink escape, missing files and non-files.
7. The registry runs Bash PreToolUse hooks for the outer `run_skill_script` call. If a hook rewrites `command`, the handler uses that command for the delegated exec call.
8. The handler builds a shell-quoted command when no hook rewrite is present and delegates to `ExecCommandHandler`.
9. Unified exec performs the actual process launch or approval/sandbox rejection.
10. The registry runs Bash PostToolUse hooks using the actual unified exec output and `hook_command`.

## Invariants

- No separate process runner.
- No separate permission model.
- No shell execution when unified exec is unavailable.
- No path traversal outside a skill's `scripts/` directory.
- No validation/execution split across different filesystems: non-primary and remote environments are rejected in this iteration.
- No silent fallback for non-local or unreadable skill scripts.
- Existing `exec_command` behavior remains the only execution contract.

## Intentional tradeoffs

This iteration supports local primary-environment skill scripts only. Skills loaded through an executor filesystem or selected for a non-primary/remote environment return a controlled diagnostic. Supporting remote/non-local skill scripts requires a separate native filesystem-aware materialization/execution contract rather than guessing a host path.

## Intentionally unaffected surfaces

- Skill loading, parsing and implicit skill invocation.
- Plugin installation and marketplace flows.
- App-server protocol/schema.
- Legacy shell-only mode.
- Unified exec approval, sandbox, hook and telemetry behavior.
