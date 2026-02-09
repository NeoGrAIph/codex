use async_trait::async_trait;
use codex_protocol::models::FunctionCallOutputBody;
use serde::Deserialize;
use serde::Serialize;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use crate::codex::Session;
use crate::codex::TurnContext;
use crate::exec_env::create_env;
use crate::exec_policy::ExecApprovalRequest;
use crate::function_tool::FunctionCallError;
use crate::protocol::ExecCommandSource;
use crate::sandboxing::SandboxPermissions;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolOutput;
use crate::tools::context::ToolPayload;
use crate::tools::events::ToolEmitter;
use crate::tools::events::ToolEventCtx;
use crate::tools::handlers::parse_arguments;
use crate::tools::orchestrator::ToolOrchestrator;
use crate::tools::registry::ToolHandler;
use crate::tools::registry::ToolKind;
use crate::tools::runtimes::shell::ShellRequest;
use crate::tools::runtimes::shell::ShellRuntime;

pub struct RunSkillScriptHandler;

#[derive(Debug, Deserialize)]
struct RunSkillScriptArgs {
    /// Skill to run from: either a skill name (resolved via loaded skills) or a filesystem path
    /// to a skill directory / SKILL.md.
    skill: String,
    /// Relative path to the script within the skill's `scripts/` directory.
    script: String,
    /// Arguments passed to the script.
    #[serde(default)]
    args: Vec<String>,
    /// Optional timeout for the command in milliseconds.
    timeout_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
struct RunSkillScriptResult {
    skill_dir: String,
    script: String,
    command: Vec<String>,
}

#[async_trait]
impl ToolHandler for RunSkillScriptHandler {
    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    fn matches_kind(&self, payload: &ToolPayload) -> bool {
        matches!(payload, ToolPayload::Function { .. })
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<ToolOutput, FunctionCallError> {
        let ToolInvocation {
            session,
            turn,
            tracker,
            call_id,
            tool_name,
            payload,
        } = invocation;

        let ToolPayload::Function { arguments } = payload else {
            return Err(FunctionCallError::RespondToModel(format!(
                "unsupported payload for {tool_name}"
            )));
        };

        let args: RunSkillScriptArgs = parse_arguments(&arguments)?;

        let skill_dir = resolve_skill_dir(&session, &turn, args.skill.as_str())?;
        let (script_path, script_rel) =
            resolve_skill_script_path(skill_dir.as_path(), args.script.as_str())?;

        let command = build_command_for_script(script_path.as_path(), &args.args)?;

        let features = session.features();
        let mut env = create_env(&turn.shell_environment_policy);
        let dependency_env = session.dependency_env().await;
        if !dependency_env.is_empty() {
            env.extend(dependency_env);
        }

        let exec_approval_requirement = session
            .services
            .exec_policy
            .create_exec_approval_requirement_for_command(ExecApprovalRequest {
                features: &features,
                command: &command,
                approval_policy: turn.approval_policy,
                sandbox_policy: &turn.sandbox_policy,
                sandbox_permissions: SandboxPermissions::default(),
                prefix_rule: None,
            })
            .await;

        let req = ShellRequest {
            command: command.clone(),
            cwd: skill_dir.clone(),
            timeout_ms: args.timeout_ms,
            env,
            sandbox_permissions: SandboxPermissions::default(),
            justification: None,
            exec_approval_requirement,
        };

        let emitter = ToolEmitter::shell(
            req.command.clone(),
            req.cwd.clone(),
            ExecCommandSource::Agent,
            /* freeform */ false,
        );
        let event_ctx =
            ToolEventCtx::new(session.as_ref(), turn.as_ref(), &call_id, Some(&tracker));
        emitter.begin(event_ctx).await;

        let mut orchestrator = ToolOrchestrator::new();
        let mut runtime = ShellRuntime::new();
        let tool_ctx = crate::tools::sandboxing::ToolCtx {
            session: session.as_ref(),
            turn: turn.as_ref(),
            call_id: call_id.clone(),
            tool_name: tool_name.clone(),
        };
        let out = orchestrator
            .run(&mut runtime, &req, &tool_ctx, &turn, turn.approval_policy)
            .await;

        // Emit ExecCommandEnd and format output as the model-facing response, matching `shell`.
        let event_ctx =
            ToolEventCtx::new(session.as_ref(), turn.as_ref(), &call_id, Some(&tracker));
        let content = emitter.finish(event_ctx, out).await?;

        let response = RunSkillScriptResult {
            skill_dir: skill_dir.to_string_lossy().into_owned(),
            script: script_rel,
            command,
        };
        let header = serde_json::to_string(&response).map_err(|err| {
            FunctionCallError::Fatal(format!(
                "failed to serialize run_skill_script header: {err}"
            ))
        })?;

        Ok(ToolOutput::Function {
            body: FunctionCallOutputBody::Text(format!("{header}\n{content}")),
            success: Some(true),
        })
    }
}

fn resolve_skill_dir(
    session: &Arc<Session>,
    turn: &Arc<TurnContext>,
    skill_ref: &str,
) -> Result<PathBuf, FunctionCallError> {
    if looks_like_path(skill_ref) {
        let path = resolve_user_path(turn.as_ref(), skill_ref);
        let path = dunce::canonicalize(&path).unwrap_or(path);
        return resolve_skill_dir_from_path(path.as_path(), session, turn);
    }

    let outcome = session
        .services
        .skills_manager
        .skills_for_config(turn.config.as_ref());
    let enabled_skills = outcome.enabled_skills();
    let mut matches = enabled_skills
        .into_iter()
        .filter(|skill| skill.name == skill_ref)
        .collect::<Vec<_>>();
    if matches.is_empty() {
        return Err(FunctionCallError::RespondToModel(format!(
            "skill \"{skill_ref}\" not found"
        )));
    }
    if matches.len() > 1 {
        // Prefer explicit path to avoid guessing.
        matches.sort_by(|a, b| a.path.cmp(&b.path));
        let candidates = matches
            .iter()
            .take(10)
            .map(|skill| skill.path.to_string_lossy())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(FunctionCallError::RespondToModel(format!(
            "skill \"{skill_ref}\" is ambiguous; pass an explicit skill path instead. Candidates: {candidates}"
        )));
    }

    let skill_md = matches.pop().expect("matches is non-empty").path;

    if outcome.disabled_paths.contains(&skill_md) {
        return Err(FunctionCallError::RespondToModel(format!(
            "skill \"{skill_ref}\" is disabled"
        )));
    }

    let Some(dir) = skill_md.parent() else {
        return Err(FunctionCallError::RespondToModel(format!(
            "skill \"{skill_ref}\" has invalid path {}",
            skill_md.to_string_lossy()
        )));
    };
    Ok(dir.to_path_buf())
}

fn resolve_skill_dir_from_path(
    path: &Path,
    session: &Arc<Session>,
    turn: &Arc<TurnContext>,
) -> Result<PathBuf, FunctionCallError> {
    let (skill_dir, skill_md) = if path.is_dir() {
        (path.to_path_buf(), path.join("SKILL.md"))
    } else {
        let Some(dir) = path.parent() else {
            return Err(FunctionCallError::RespondToModel(format!(
                "invalid skill path {}",
                path.to_string_lossy()
            )));
        };
        (dir.to_path_buf(), path.to_path_buf())
    };

    if !skill_md.is_file() {
        return Err(FunctionCallError::RespondToModel(format!(
            "skill path {} does not contain SKILL.md",
            skill_dir.to_string_lossy()
        )));
    }

    // Respect disabled skills when the skill is discoverable via config.
    let outcome = session
        .services
        .skills_manager
        .skills_for_config(turn.config.as_ref());
    let skill_md = dunce::canonicalize(&skill_md).unwrap_or(skill_md);
    if outcome.disabled_paths.contains(&skill_md) {
        return Err(FunctionCallError::RespondToModel(format!(
            "skill at {} is disabled",
            skill_dir.to_string_lossy()
        )));
    }

    Ok(skill_dir)
}

fn resolve_skill_script_path(
    skill_dir: &Path,
    script: &str,
) -> Result<(PathBuf, String), FunctionCallError> {
    if script.trim().is_empty() {
        return Err(FunctionCallError::RespondToModel(
            "script must be non-empty".to_string(),
        ));
    }
    let rel = Path::new(script);
    for comp in rel.components() {
        match comp {
            Component::Normal(_) => {}
            Component::CurDir
            | Component::ParentDir
            | Component::RootDir
            | Component::Prefix(_) => {
                return Err(FunctionCallError::RespondToModel(format!(
                    "script must be a relative path within scripts/ (got {script:?})"
                )));
            }
        }
    }

    let scripts_dir = skill_dir.join("scripts");
    if !scripts_dir.is_dir() {
        return Err(FunctionCallError::RespondToModel(format!(
            "skill at {} has no scripts/ directory",
            skill_dir.to_string_lossy()
        )));
    }

    let scripts_dir = dunce::canonicalize(&scripts_dir).unwrap_or(scripts_dir);
    let candidate = scripts_dir.join(rel);
    if !candidate.exists() {
        return Err(FunctionCallError::RespondToModel(format!(
            "script not found: {}",
            candidate.to_string_lossy()
        )));
    }

    let candidate = dunce::canonicalize(&candidate).unwrap_or(candidate);
    if !candidate.starts_with(&scripts_dir) {
        return Err(FunctionCallError::RespondToModel(
            "script must resolve inside scripts/".to_string(),
        ));
    }

    Ok((candidate, script.to_string()))
}

fn build_command_for_script(
    script_path: &Path,
    args: &[String],
) -> Result<Vec<String>, FunctionCallError> {
    let script_str = script_path.to_string_lossy().into_owned();
    let mut command = if script_str.ends_with(".py") {
        vec!["python3".to_string(), script_str]
    } else {
        vec![script_str]
    };
    command.extend(args.iter().cloned());
    Ok(command)
}

fn looks_like_path(skill_ref: &str) -> bool {
    // Heuristic: if it contains path separators or begins with a path-y prefix, treat it as a path.
    skill_ref.contains('/')
        || skill_ref.contains('\\')
        || skill_ref.starts_with('.')
        || skill_ref.starts_with('~')
}

fn resolve_user_path(turn: &TurnContext, raw: &str) -> PathBuf {
    if let Some(rest) = raw.strip_prefix("~/").or_else(|| raw.strip_prefix("~\\")) {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        path
    } else {
        turn.cwd.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codex::make_session_and_context;
    use crate::tools::context::SharedTurnDiffTracker;
    use crate::turn_diff_tracker::TurnDiffTracker;
    use pretty_assertions::assert_eq;
    use std::fs;
    use tokio::sync::Mutex;

    fn invocation(
        session: Session,
        mut turn: TurnContext,
        arguments: serde_json::Value,
    ) -> ToolInvocation {
        // Avoid approval prompts in tests.
        turn.approval_policy = codex_protocol::protocol::AskForApproval::Never;
        // Avoid sandbox pipelines that require external helpers in this test harness.
        turn.sandbox_policy = codex_protocol::protocol::SandboxPolicy::DangerFullAccess;
        ToolInvocation {
            session: Arc::new(session),
            turn: Arc::new(turn),
            tracker: Arc::new(Mutex::new(TurnDiffTracker::new())) as SharedTurnDiffTracker,
            call_id: "call-1".to_string(),
            tool_name: "run_skill_script".to_string(),
            payload: ToolPayload::Function {
                arguments: arguments.to_string(),
            },
        }
    }

    #[test]
    fn script_rejects_path_traversal() {
        let err = resolve_skill_script_path(Path::new("/tmp/skill"), "../evil.sh").unwrap_err();
        assert!(matches!(err, FunctionCallError::RespondToModel(_)));
    }

    #[tokio::test]
    #[cfg(not(windows))]
    async fn runs_script_with_cwd_set_to_skill_dir() {
        let (session, turn) = make_session_and_context().await;
        let skill_dir = tempfile::tempdir().expect("tempdir");
        let scripts_dir = skill_dir.path().join("scripts");
        fs::create_dir_all(&scripts_dir).expect("create scripts dir");
        fs::write(
            skill_dir.path().join("SKILL.md"),
            "---\nname: x\ndescription: y\n---\n",
        )
        .expect("write skill");

        let script_path = scripts_dir.join("echo.sh");
        fs::write(&script_path, "#!/bin/sh\necho ok\n").expect("write script");
        let mut perms = fs::metadata(&script_path).expect("stat").permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            perms.set_mode(0o755);
            fs::set_permissions(&script_path, perms).expect("chmod");
        }

        let args = serde_json::json!({
            "skill": skill_dir.path().to_string_lossy(),
            "script": "echo.sh",
            "args": []
        });
        let inv = invocation(session, turn, args);
        let out = RunSkillScriptHandler
            .handle(inv)
            .await
            .expect("run should succeed");

        let ToolOutput::Function { body, success } = out else {
            panic!("expected function output");
        };
        assert_eq!(success, Some(true));
        let text = body.to_text().expect("text body");
        assert!(
            text.contains("ok"),
            "expected script output to be present; got {text:?}"
        );
    }

    #[tokio::test]
    async fn rejects_missing_skill_md() {
        let (session, turn) = make_session_and_context().await;
        let skill_dir = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(skill_dir.path().join("scripts")).expect("scripts dir");

        let args = serde_json::json!({
            "skill": skill_dir.path().to_string_lossy(),
            "script": "echo.sh",
            "args": []
        });
        let inv = invocation(session, turn, args);
        let err = match RunSkillScriptHandler.handle(inv).await {
            Ok(_) => panic!("expected failure"),
            Err(err) => err,
        };
        let FunctionCallError::RespondToModel(msg) = err else {
            panic!("expected RespondToModel");
        };
        assert!(msg.contains("does not contain SKILL.md"));
    }
}
