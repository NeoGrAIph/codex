# run_skill_script project

## Status

Первая итерация для `fork/140` реализует `run_skill_script` как adapter над native unified exec. Canonical feature contract: `docs/fork/features/run-skill-script.md`.

## Implementation map

- Tool planning: `codex-rs/core/src/tools/spec_plan.rs`.
- Tool spec: `codex-rs/core/src/tools/handlers/run_skill_script_spec.rs`.
- Tool handler: `codex-rs/core/src/tools/handlers/run_skill_script.rs`.
- Execution owner: `codex-rs/core/src/tools/handlers/unified_exec/exec_command.rs`.
- Skill metadata source: `TurnContext.turn_skills.outcome`.

## Current user contract

`run_skill_script` appears when unified exec is model-visible and an environment is available. The caller provides an enabled skill name or exact `SKILL.md` path, a relative script path under that skill's `scripts/` directory, optional string args, and optional output wait/budget parameters. In multiple-environment turns, `environment_id` is model-visible for consistency with native environment-backed tools, but the first iteration accepts only the primary local environment; remote/non-primary selections fail fast.

The handler validates the skill, script path and primary-local environment, exposes Bash hook payloads through the outer registry lifecycle, then delegates actual process execution to `exec_command`. It does not bypass approval policy, sandbox policy, telemetry, output truncation or unified exec process management.

## Canonical links

- Feature passport: `docs/fork/features/run-skill-script.md`.
- Design: `docs/fork/projects/run-skill-script/design.md`.
- Verification: `docs/fork/projects/run-skill-script/verification.md`.
- Release research: `docs/fork/research/0.140.0/tools-mcp-skills.md`.
