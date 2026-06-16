use crate::function_tool::FunctionCallError;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::handlers::parse_arguments;
use crate::tools::handlers::rewrite_function_arguments;
use crate::tools::handlers::run_skill_script_spec::create_run_skill_script_tool;
use crate::tools::handlers::unified_exec::ExecCommandHandler;
use crate::tools::handlers::unified_exec::ExecCommandHandlerOptions;
use crate::tools::handlers::updated_hook_command;
use crate::tools::hook_names::HookToolName;
use crate::tools::registry::CoreToolRuntime;
use crate::tools::registry::PostToolUsePayload;
use crate::tools::registry::PreToolUsePayload;
use crate::tools::registry::ToolExecutor;
use codex_core_skills::SkillMetadata;
use codex_shell_command::parse_command::shlex_join;
use codex_tools::ToolEnvironmentMode;
use codex_tools::ToolName;
use codex_tools::ToolSpec;
use serde::Deserialize;
use serde_json::Map;
use serde_json::Value;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

pub struct RunSkillScriptHandler {
    exec_options: ExecCommandHandlerOptions,
    environment_mode: ToolEnvironmentMode,
}

#[derive(Debug, Deserialize)]
struct RunSkillScriptArgs {
    skill: String,
    script: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    environment_id: Option<String>,
    #[serde(default)]
    yield_time_ms: Option<u64>,
    #[serde(default)]
    max_output_tokens: Option<usize>,
    #[serde(default, rename = "_hook_updated_cmd")]
    hook_updated_cmd: Option<String>,
}

#[derive(Debug)]
struct ResolvedSkillScriptInvocation {
    script: PathBuf,
    cmd: String,
}

impl RunSkillScriptHandler {
    pub(crate) fn new(
        exec_options: ExecCommandHandlerOptions,
        environment_mode: ToolEnvironmentMode,
    ) -> Self {
        Self {
            exec_options,
            environment_mode,
        }
    }

    async fn handle_call(
        &self,
        invocation: ToolInvocation,
    ) -> Result<Box<dyn crate::tools::context::ToolOutput>, FunctionCallError> {
        let args = match &invocation.payload {
            ToolPayload::Function { arguments } => parse_arguments::<RunSkillScriptArgs>(arguments),
            _ => Err(FunctionCallError::RespondToModel(
                "run_skill_script handler received unsupported payload".to_string(),
            )),
        }?;
        let resolved = resolve_skill_script_invocation(invocation.turn.as_ref(), &args)?;
        let workdir = resolved
            .script
            .parent()
            .ok_or_else(|| {
                FunctionCallError::RespondToModel(format!(
                    "script `{}` has no parent directory",
                    resolved.script.display()
                ))
            })?
            .to_string_lossy()
            .into_owned();

        let mut exec_arguments = Map::from_iter([
            ("cmd".to_string(), Value::String(resolved.cmd)),
            ("workdir".to_string(), Value::String(workdir)),
        ]);
        if matches!(self.environment_mode, ToolEnvironmentMode::Multiple)
            && let Some(environment_id) = args.environment_id
        {
            exec_arguments.insert("environment_id".to_string(), Value::String(environment_id));
        }
        if let Some(yield_time_ms) = args.yield_time_ms {
            exec_arguments.insert(
                "yield_time_ms".to_string(),
                Value::Number(yield_time_ms.into()),
            );
        }
        if let Some(max_output_tokens) = args.max_output_tokens {
            exec_arguments.insert(
                "max_output_tokens".to_string(),
                Value::Number(max_output_tokens.into()),
            );
        }

        ExecCommandHandler::new(self.exec_options)
            .handle(ToolInvocation {
                tool_name: ToolName::plain("exec_command"),
                payload: ToolPayload::Function {
                    arguments: Value::Object(exec_arguments).to_string(),
                },
                ..invocation
            })
            .await
    }
}

impl ToolExecutor<ToolInvocation> for RunSkillScriptHandler {
    fn tool_name(&self) -> ToolName {
        ToolName::plain("run_skill_script")
    }

    fn spec(&self) -> ToolSpec {
        create_run_skill_script_tool(self.environment_mode)
    }

    fn supports_parallel_tool_calls(&self) -> bool {
        true
    }

    fn handle(&self, invocation: ToolInvocation) -> codex_tools::ToolExecutorFuture<'_> {
        Box::pin(self.handle_call(invocation))
    }
}

impl CoreToolRuntime for RunSkillScriptHandler {
    fn matches_kind(&self, payload: &ToolPayload) -> bool {
        matches!(payload, ToolPayload::Function { .. })
    }

    fn pre_tool_use_payload(&self, invocation: &ToolInvocation) -> Option<PreToolUsePayload> {
        let ToolPayload::Function { arguments } = &invocation.payload else {
            return None;
        };

        parse_arguments::<RunSkillScriptArgs>(arguments)
            .ok()
            .and_then(|args| resolve_skill_script_invocation(invocation.turn.as_ref(), &args).ok())
            .map(|resolved| PreToolUsePayload {
                tool_name: HookToolName::bash(),
                tool_input: serde_json::json!({ "command": resolved.cmd }),
            })
    }

    fn with_updated_hook_input(
        &self,
        mut invocation: ToolInvocation,
        updated_input: Value,
    ) -> Result<ToolInvocation, FunctionCallError> {
        let ToolPayload::Function { arguments } = invocation.payload else {
            return Err(FunctionCallError::RespondToModel(
                "hook input rewrite received unsupported run_skill_script payload".to_string(),
            ));
        };
        let command = updated_hook_command(&updated_input)?.to_string();
        invocation.payload = ToolPayload::Function {
            arguments: rewrite_function_arguments(&arguments, "run_skill_script", |arguments| {
                arguments.insert("_hook_updated_cmd".to_string(), Value::String(command));
            })?,
        };
        Ok(invocation)
    }

    fn post_tool_use_payload(
        &self,
        invocation: &ToolInvocation,
        result: &dyn crate::tools::context::ToolOutput,
    ) -> Option<PostToolUsePayload> {
        let ToolPayload::Function { .. } = &invocation.payload else {
            return None;
        };

        let tool_input = result.post_tool_use_input(&invocation.payload)?;
        let tool_use_id = result.post_tool_use_id(&invocation.call_id);
        let tool_response = result.post_tool_use_response(&tool_use_id, &invocation.payload)?;
        Some(PostToolUsePayload {
            tool_name: HookToolName::bash(),
            tool_use_id,
            tool_input,
            tool_response,
        })
    }
}

fn resolve_skill_script_invocation(
    turn: &crate::session::turn_context::TurnContext,
    args: &RunSkillScriptArgs,
) -> Result<ResolvedSkillScriptInvocation, FunctionCallError> {
    ensure_primary_local_environment(turn, args.environment_id.as_deref())?;
    let script = resolve_skill_script(turn, args)?;
    let cmd = args
        .hook_updated_cmd
        .clone()
        .unwrap_or_else(|| command_for_script(&script, &args.args));
    Ok(ResolvedSkillScriptInvocation { script, cmd })
}

fn ensure_primary_local_environment(
    turn: &crate::session::turn_context::TurnContext,
    requested_environment_id: Option<&str>,
) -> Result<(), FunctionCallError> {
    let primary = turn.environments.primary().ok_or_else(|| {
        FunctionCallError::RespondToModel(
            "run_skill_script requires a local primary turn environment".to_string(),
        )
    })?;
    if primary.environment.is_remote() {
        return Err(FunctionCallError::RespondToModel(
            "run_skill_script supports only the local primary environment in this iteration"
                .to_string(),
        ));
    }
    if let Some(requested_environment_id) = requested_environment_id
        && requested_environment_id != primary.environment_id
    {
        return Err(FunctionCallError::RespondToModel(format!(
            "run_skill_script supports only the primary local environment `{}` in this iteration; requested `{requested_environment_id}`",
            primary.environment_id
        )));
    }
    Ok(())
}

fn resolve_skill_script(
    turn: &crate::session::turn_context::TurnContext,
    args: &RunSkillScriptArgs,
) -> Result<PathBuf, FunctionCallError> {
    let skill = find_enabled_skill(turn, args.skill.trim())?;
    let skill_path = skill.path_to_skills_md.to_path_buf();
    let skill_dir = skill_path.parent().ok_or_else(|| {
        FunctionCallError::RespondToModel(format!(
            "skill `{}` has SKILL.md without a parent directory",
            skill.name
        ))
    })?;
    let scripts_dir = skill_dir.join("scripts");
    let script_rel = validate_relative_script_path(&args.script)?;
    let script = scripts_dir.join(script_rel);
    let canonical_scripts_dir = std::fs::canonicalize(&scripts_dir).map_err(|err| {
        FunctionCallError::RespondToModel(format!(
            "skill `{}` does not have a readable local scripts directory at `{}`: {err}",
            skill.name,
            scripts_dir.display()
        ))
    })?;
    let canonical_script = std::fs::canonicalize(&script).map_err(|err| {
        FunctionCallError::RespondToModel(format!(
            "skill script `{}` was not found or is not readable: {err}",
            script.display()
        ))
    })?;
    if !canonical_script.starts_with(&canonical_scripts_dir) {
        return Err(FunctionCallError::RespondToModel(format!(
            "skill script `{}` escapes the scripts directory",
            args.script
        )));
    }
    if !canonical_script.is_file() {
        return Err(FunctionCallError::RespondToModel(format!(
            "skill script `{}` is not a file",
            canonical_script.display()
        )));
    }
    Ok(canonical_script)
}

fn find_enabled_skill<'a>(
    turn: &'a crate::session::turn_context::TurnContext,
    requested: &str,
) -> Result<&'a SkillMetadata, FunctionCallError> {
    if requested.is_empty() {
        return Err(FunctionCallError::RespondToModel(
            "`skill` must not be empty".to_string(),
        ));
    }

    let mut matches = turn
        .turn_skills
        .outcome
        .skills_with_enabled()
        .filter(|(_, enabled)| *enabled)
        .filter_map(|(skill, _)| skill_matches_request(skill, requested).then_some(skill))
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| left.name.cmp(&right.name));
    match matches.as_slice() {
        [skill] => Ok(skill),
        [] => Err(FunctionCallError::RespondToModel(format!(
            "enabled skill `{requested}` was not found"
        ))),
        _ => Err(FunctionCallError::RespondToModel(format!(
            "enabled skill `{requested}` is ambiguous; use the SKILL.md path"
        ))),
    }
}

fn skill_matches_request(skill: &SkillMetadata, requested: &str) -> bool {
    skill.name == requested || skill.path_to_skills_md.to_string_lossy() == requested
}

fn validate_relative_script_path(script: &str) -> Result<PathBuf, FunctionCallError> {
    let path = Path::new(script);
    if script.trim().is_empty() || path.is_absolute() {
        return Err(FunctionCallError::RespondToModel(
            "`script` must be a non-empty relative path under scripts/".to_string(),
        ));
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(FunctionCallError::RespondToModel(
            "`script` must not contain parent directory components".to_string(),
        ));
    }
    Ok(path.to_path_buf())
}

fn command_for_script(script: &Path, args: &[String]) -> String {
    let mut command = if script.extension().and_then(|extension| extension.to_str()) == Some("py") {
        vec!["python3".to_string(), script.to_string_lossy().into_owned()]
    } else {
        vec![script.to_string_lossy().into_owned()]
    };
    command.extend(args.iter().cloned());
    shlex_join(&command)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::tests::make_session_and_context;
    use crate::session::turn_context::TurnEnvironment;
    use crate::session::turn_context::TurnSkillsContext;
    use crate::tools::context::ExecCommandToolOutput;
    use crate::tools::context::ToolCallSource;
    use crate::turn_diff_tracker::TurnDiffTracker;
    use codex_core_skills::SkillLoadOutcome;
    use codex_exec_server::Environment;
    use codex_exec_server::LOCAL_ENVIRONMENT_ID;
    use codex_exec_server::REMOTE_ENVIRONMENT_ID;
    use codex_protocol::protocol::SkillScope;
    use codex_utils_absolute_path::AbsolutePathBuf;
    use codex_utils_output_truncation::TruncationPolicy;
    use pretty_assertions::assert_eq;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    fn make_script_skill(root: &Path) -> (tempfile::TempDir, PathBuf) {
        let temp_dir = tempfile::tempdir_in(root).expect("create skill temp dir");
        let skill_dir = temp_dir.path().join("demo");
        let scripts_dir = skill_dir.join("scripts");
        std::fs::create_dir_all(&scripts_dir).expect("create scripts dir");
        let skill_path = skill_dir.join("SKILL.md");
        std::fs::write(&skill_path, "---\nname: demo\ndescription: demo\n---\n")
            .expect("write skill");
        let script_path = scripts_dir.join("run.sh");
        std::fs::write(&script_path, "#!/bin/sh\nprintf skill-output\n").expect("write script");
        (temp_dir, script_path)
    }

    fn skill_metadata(skill_path: PathBuf) -> SkillMetadata {
        SkillMetadata {
            name: "demo".to_string(),
            description: "demo skill".to_string(),
            short_description: None,
            interface: None,
            dependencies: None,
            policy: None,
            path_to_skills_md: AbsolutePathBuf::from_absolute_path(skill_path)
                .expect("absolute skill path"),
            scope: SkillScope::User,
            plugin_id: None,
        }
    }

    async fn invocation_with_skill(script_path: &Path) -> (tempfile::TempDir, ToolInvocation) {
        let (session, mut turn) = make_session_and_context().await;
        let root = tempfile::tempdir().expect("create root");
        let skill_path = script_path
            .parent()
            .expect("script parent")
            .parent()
            .expect("scripts parent")
            .join("SKILL.md");
        let mut outcome = SkillLoadOutcome::default();
        outcome.skills = vec![skill_metadata(skill_path)];
        turn.turn_skills = TurnSkillsContext::new(Arc::new(outcome));
        let payload = ToolPayload::Function {
            arguments: serde_json::json!({
                "skill": "demo",
                "script": "run.sh",
                "args": ["hello world"],
            })
            .to_string(),
        };
        (
            root,
            ToolInvocation {
                session: session.into(),
                turn: turn.into(),
                cancellation_token: tokio_util::sync::CancellationToken::new(),
                tracker: Arc::new(Mutex::new(TurnDiffTracker::new())),
                call_id: "call-run-skill".to_string(),
                tool_name: ToolName::plain("run_skill_script"),
                source: ToolCallSource::Direct,
                payload,
            },
        )
    }

    #[test]
    fn script_rejects_path_traversal() {
        let err = validate_relative_script_path("../x").expect_err("path traversal should fail");
        assert!(
            err.to_string()
                .contains("must not contain parent directory components")
        );
    }

    #[test]
    fn command_quotes_script_arguments() {
        let command = command_for_script(
            Path::new("/tmp/skill scripts/run.py"),
            &["hello world".to_string()],
        );
        assert_eq!(command, "python3 '/tmp/skill scripts/run.py' 'hello world'");
    }

    #[tokio::test]
    async fn pre_tool_use_payload_uses_resolved_script_command() {
        let root = tempfile::tempdir().expect("create root");
        let (_skill_dir_guard, script_path) = make_script_skill(root.path());
        let (_turn_guard, invocation) = invocation_with_skill(&script_path).await;
        let handler = RunSkillScriptHandler::new(
            ExecCommandHandlerOptions {
                allow_login_shell: false,
                exec_permission_approvals_enabled: false,
                include_environment_id: false,
                include_shell_parameter: true,
            },
            ToolEnvironmentMode::Single,
        );

        assert_eq!(
            handler.pre_tool_use_payload(&invocation),
            Some(PreToolUsePayload {
                tool_name: HookToolName::bash(),
                tool_input: serde_json::json!({
                    "command": format!("{} 'hello world'", script_path.canonicalize().expect("canonical script").display()),
                }),
            })
        );
    }

    #[tokio::test]
    async fn hook_rewrite_updates_delegated_command() {
        let root = tempfile::tempdir().expect("create root");
        let (_skill_dir_guard, script_path) = make_script_skill(root.path());
        let (_turn_guard, invocation) = invocation_with_skill(&script_path).await;
        let handler = RunSkillScriptHandler::new(
            ExecCommandHandlerOptions {
                allow_login_shell: false,
                exec_permission_approvals_enabled: false,
                include_environment_id: false,
                include_shell_parameter: true,
            },
            ToolEnvironmentMode::Single,
        );

        let rewritten = handler
            .with_updated_hook_input(
                invocation,
                serde_json::json!({ "command": "printf rewritten" }),
            )
            .expect("rewrite should succeed");
        let ToolPayload::Function { arguments } = &rewritten.payload else {
            panic!("expected function payload");
        };
        let args: RunSkillScriptArgs = parse_arguments(arguments).expect("parse rewritten args");
        let resolved =
            resolve_skill_script_invocation(rewritten.turn.as_ref(), &args).expect("resolve");

        assert_eq!(resolved.cmd, "printf rewritten");
    }

    #[tokio::test]
    async fn post_tool_use_payload_uses_unified_exec_output() {
        let root = tempfile::tempdir().expect("create root");
        let (_skill_dir_guard, script_path) = make_script_skill(root.path());
        let (_turn_guard, invocation) = invocation_with_skill(&script_path).await;
        let handler = RunSkillScriptHandler::new(
            ExecCommandHandlerOptions {
                allow_login_shell: false,
                exec_permission_approvals_enabled: false,
                include_environment_id: false,
                include_shell_parameter: true,
            },
            ToolEnvironmentMode::Single,
        );
        let output = ExecCommandToolOutput {
            event_call_id: "exec-call-run-skill".to_string(),
            chunk_id: "chunk-1".to_string(),
            wall_time: std::time::Duration::from_millis(15),
            raw_output: b"skill-output".to_vec(),
            truncation_policy: TruncationPolicy::Tokens(10_000),
            max_output_tokens: None,
            process_id: None,
            exit_code: Some(0),
            original_token_count: None,
            hook_command: Some("printf rewritten".to_string()),
        };

        assert_eq!(
            handler.post_tool_use_payload(&invocation, &output),
            Some(PostToolUsePayload {
                tool_name: HookToolName::bash(),
                tool_use_id: "exec-call-run-skill".to_string(),
                tool_input: serde_json::json!({ "command": "printf rewritten" }),
                tool_response: serde_json::json!("skill-output"),
            })
        );
    }

    #[tokio::test]
    async fn run_skill_script_rejects_non_primary_environment_id() {
        let root = tempfile::tempdir().expect("create root");
        let (_skill_dir_guard, script_path) = make_script_skill(root.path());
        let (_turn_guard, mut invocation) = invocation_with_skill(&script_path).await;
        let mut turn = Arc::try_unwrap(invocation.turn).expect("unique turn");
        turn.environments
            .turn_environments
            .push(TurnEnvironment::new(
                REMOTE_ENVIRONMENT_ID.to_string(),
                Arc::new(
                    Environment::create_for_tests(Some("ws://127.0.0.1:1/remote".to_string()))
                        .expect("remote environment"),
                ),
                AbsolutePathBuf::current_dir().expect("cwd"),
                /*shell*/ None,
            ));
        invocation.turn = Arc::new(turn);
        invocation.payload = ToolPayload::Function {
            arguments: serde_json::json!({
                "skill": "demo",
                "script": "run.sh",
                "environment_id": REMOTE_ENVIRONMENT_ID,
            })
            .to_string(),
        };
        let args: RunSkillScriptArgs = parse_arguments(match &invocation.payload {
            ToolPayload::Function { arguments } => arguments,
            _ => unreachable!("function payload"),
        })
        .expect("parse args");

        let err = resolve_skill_script_invocation(invocation.turn.as_ref(), &args)
            .expect_err("remote environment should be rejected");

        assert!(
            err.to_string()
                .contains("supports only the primary local environment")
        );
    }

    #[tokio::test]
    async fn run_skill_script_rejects_remote_primary_environment() {
        let root = tempfile::tempdir().expect("create root");
        let (_skill_dir_guard, script_path) = make_script_skill(root.path());
        let (_turn_guard, mut invocation) = invocation_with_skill(&script_path).await;
        let mut turn = Arc::try_unwrap(invocation.turn).expect("unique turn");
        turn.environments.turn_environments[0] = TurnEnvironment::new(
            LOCAL_ENVIRONMENT_ID.to_string(),
            Arc::new(
                Environment::create_for_tests(Some("ws://127.0.0.1:1/remote".to_string()))
                    .expect("remote environment"),
            ),
            AbsolutePathBuf::current_dir().expect("cwd"),
            /*shell*/ None,
        );
        invocation.turn = Arc::new(turn);
        let args: RunSkillScriptArgs = parse_arguments(match &invocation.payload {
            ToolPayload::Function { arguments } => arguments,
            _ => unreachable!("function payload"),
        })
        .expect("parse args");

        let err = resolve_skill_script_invocation(invocation.turn.as_ref(), &args)
            .expect_err("remote primary should be rejected");

        assert!(
            err.to_string()
                .contains("supports only the local primary environment")
        );
    }

    #[tokio::test]
    async fn run_skill_script_rejects_missing_skill() {
        let root = tempfile::tempdir().expect("create root");
        let (_skill_dir_guard, script_path) = make_script_skill(root.path());
        let (_turn_guard, mut invocation) = invocation_with_skill(&script_path).await;
        invocation.payload = ToolPayload::Function {
            arguments: serde_json::json!({
                "skill": "missing",
                "script": "run.sh",
            })
            .to_string(),
        };
        let args: RunSkillScriptArgs = parse_arguments(match &invocation.payload {
            ToolPayload::Function { arguments } => arguments,
            _ => unreachable!("function payload"),
        })
        .expect("parse args");

        let err =
            resolve_skill_script_invocation(invocation.turn.as_ref(), &args).expect_err("missing");

        assert!(
            err.to_string()
                .contains("enabled skill `missing` was not found")
        );
    }

    #[tokio::test]
    async fn run_skill_script_rejects_disabled_skill() {
        let root = tempfile::tempdir().expect("create root");
        let (_skill_dir_guard, script_path) = make_script_skill(root.path());
        let (_turn_guard, mut invocation) = invocation_with_skill(&script_path).await;
        let mut turn = Arc::try_unwrap(invocation.turn).expect("unique turn");
        let skill_path = turn.turn_skills.outcome.skills[0].path_to_skills_md.clone();
        let mut outcome = (*turn.turn_skills.outcome).clone();
        outcome.disabled_paths.insert(skill_path);
        turn.turn_skills = TurnSkillsContext::new(Arc::new(outcome));
        invocation.turn = Arc::new(turn);
        let args: RunSkillScriptArgs = parse_arguments(match &invocation.payload {
            ToolPayload::Function { arguments } => arguments,
            _ => unreachable!("function payload"),
        })
        .expect("parse args");

        let err =
            resolve_skill_script_invocation(invocation.turn.as_ref(), &args).expect_err("disabled");

        assert!(
            err.to_string()
                .contains("enabled skill `demo` was not found")
        );
    }
}
