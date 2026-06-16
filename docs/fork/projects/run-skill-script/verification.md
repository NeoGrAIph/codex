# run_skill_script verification

## Required checks

- `cargo check -p codex-core`: verifies spec/handler wiring and unified exec delegation compile.
- `just test -p codex-core run_skill_script`: verifies tool planning, handler unit coverage, hook payload behavior and real helper execution through unified exec.
- `git diff --check`: verifies no whitespace errors.

## Scenarios

| Scenario | Expected result | Evidence |
| --- | --- | --- |
| No turn environment | `run_skill_script` is absent with other environment-backed tools. | `environment_count_controls_environment_backed_tools` |
| Multiple environments | `run_skill_script` is visible, but first iteration accepts only omitted `environment_id` or the primary local environment id. | `environment_count_controls_environment_backed_tools`, `environment_id_is_only_present_for_multiple_environments`, `run_skill_script_rejects_non_primary_environment_id` |
| Remote primary environment | Handler rejects execution before host-path validation can be reused against another filesystem. | `run_skill_script_rejects_remote_primary_environment` |
| Missing or disabled skill | Handler rejects before exec delegation. | `run_skill_script_rejects_missing_skill`, `run_skill_script_rejects_disabled_skill` |
| Path traversal | `../` script paths are rejected before exec delegation. | `script_rejects_path_traversal` |
| Shell quoting | Script path and args are quoted before being passed to `exec_command`. | `command_quotes_script_arguments` |
| PreToolUse hook payload | Outer `run_skill_script` registry dispatch exposes Bash `{ "command": ... }`. | `pre_tool_use_payload_uses_resolved_script_command` |
| PreToolUse hook rewrite | Hook-updated Bash command becomes the delegated exec command. | `hook_rewrite_updates_delegated_command` |
| PostToolUse hook payload | Post hook sees the actual unified exec command/output. | `post_tool_use_payload_uses_unified_exec_output` |
| Enabled skill helper execution | Core suite launches an executable skill helper through unified exec and returns its output to the model. | `run_skill_script_executes_enabled_skill_helper_through_unified_exec` |

## Coverage gaps

The first iteration deliberately has no remote/non-local e2e because that path is rejected until a native filesystem-aware materialization contract exists. Future remote support must add an integration test that validates the exact remote filesystem path used for both validation and execution.
