use super::*;
use crate::SkillsService;
use crate::config::ConfigBuilder;
use crate::skills_load_input_from_config;
use codex_config::ConfigLayerStackOrdering;
use codex_core_plugins::PluginsManager;
use codex_protocol::config_types::Personality;
use codex_protocol::config_types::ReasoningSummary;
use codex_protocol::config_types::ServiceTier;
use codex_protocol::openai_models::ReasoningEffort;
use codex_utils_absolute_path::test_support::PathExt;
use pretty_assertions::assert_eq;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

async fn test_config_with_cli_overrides(
    cli_overrides: Vec<(String, TomlValue)>,
) -> (TempDir, Config) {
    let home = TempDir::new().expect("create temp dir");
    let home_path = home.path().to_path_buf();
    let config = ConfigBuilder::default()
        .codex_home(home_path.clone())
        .cli_overrides(cli_overrides)
        .fallback_cwd(Some(home_path))
        .build()
        .await
        .expect("load test config");
    (home, config)
}

async fn write_role_config(home: &TempDir, name: &str, contents: &str) -> PathBuf {
    let role_path = home.path().join(name);
    tokio::fs::write(&role_path, contents)
        .await
        .expect("write role config");
    role_path
}

fn session_flags_layer_count(config: &Config) -> usize {
    config
        .config_layer_stack
        .get_layers(
            ConfigLayerStackOrdering::LowestPrecedenceFirst,
            /*include_disabled*/ true,
        )
        .into_iter()
        .filter(|layer| layer.name == ConfigLayerSource::SessionFlags)
        .count()
}

#[tokio::test]
async fn apply_role_defaults_to_default_and_leaves_config_unchanged() {
    let (_home, mut config) = test_config_with_cli_overrides(Vec::new()).await;
    let before = config.clone();

    apply_role_to_config(&mut config, /*role_name*/ None)
        .await
        .expect("default role should apply");

    assert_eq!(before, config);
}

#[tokio::test]
async fn apply_role_returns_error_for_unknown_role() {
    let (_home, mut config) = test_config_with_cli_overrides(Vec::new()).await;

    let err = apply_role_to_config(&mut config, Some("missing-role"))
        .await
        .expect_err("unknown role should fail");

    assert_eq!(err, "unknown agent_type 'missing-role'");
}

#[tokio::test]
#[ignore = "No role requiring it for now"]
async fn apply_explorer_role_sets_model_and_adds_session_flags_layer() {
    let (_home, mut config) = test_config_with_cli_overrides(Vec::new()).await;
    let before_layers = session_flags_layer_count(&config);

    apply_role_to_config(&mut config, Some("explorer"))
        .await
        .expect("explorer role should apply");

    assert_eq!(config.model.as_deref(), Some("gpt-5.4-mini"));
    assert_eq!(config.model_reasoning_effort, Some(ReasoningEffort::Medium));
    assert_eq!(session_flags_layer_count(&config), before_layers + 1);
}

#[tokio::test]
async fn apply_empty_explorer_role_preserves_current_model_and_reasoning_effort() {
    let (_home, mut config) = test_config_with_cli_overrides(Vec::new()).await;
    let before_layers = session_flags_layer_count(&config);
    config.model = Some("gpt-5.4-mini".to_string());
    config.model_reasoning_effort = Some(ReasoningEffort::High);

    apply_role_to_config(&mut config, Some("explorer"))
        .await
        .expect("explorer role should apply");

    assert_eq!(config.model.as_deref(), Some("gpt-5.4-mini"));
    assert_eq!(config.model_reasoning_effort, Some(ReasoningEffort::High));
    assert_eq!(session_flags_layer_count(&config), before_layers);
}

#[tokio::test]
async fn apply_role_returns_unavailable_for_missing_user_role_file() {
    let (_home, mut config) = test_config_with_cli_overrides(Vec::new()).await;
    config.agent_roles.insert(
        "custom".to_string(),
        AgentRoleConfig {
            description: None,
            config_file: Some(PathBuf::from("/path/does/not/exist.toml")),
            nickname_candidates: None,
        },
    );

    let err = apply_role_to_config(&mut config, Some("custom"))
        .await
        .expect_err("missing role file should fail");

    assert_eq!(err, AGENT_TYPE_UNAVAILABLE_ERROR);
}

#[tokio::test]
async fn apply_role_returns_unavailable_for_invalid_user_role_toml() {
    let (home, mut config) = test_config_with_cli_overrides(Vec::new()).await;
    let role_path = write_role_config(&home, "invalid-role.toml", "model = [").await;
    config.agent_roles.insert(
        "custom".to_string(),
        AgentRoleConfig {
            description: None,
            config_file: Some(role_path),
            nickname_candidates: None,
        },
    );

    let err = apply_role_to_config(&mut config, Some("custom"))
        .await
        .expect_err("invalid role file should fail");

    assert_eq!(err, AGENT_TYPE_UNAVAILABLE_ERROR);
}

#[tokio::test]
async fn apply_role_ignores_agent_metadata_fields_in_user_role_file() {
    let (home, mut config) = test_config_with_cli_overrides(Vec::new()).await;
    let role_path = write_role_config(
        &home,
        "metadata-role.toml",
        r#"
name = "archivist"
description = "Role metadata"
nickname_candidates = ["Hypatia"]
developer_instructions = "Stay focused"
model = "role-model"
"#,
    )
    .await;
    config.agent_roles.insert(
        "custom".to_string(),
        AgentRoleConfig {
            description: None,
            config_file: Some(role_path),
            nickname_candidates: None,
        },
    );

    crate::config::agent_roles::materialize_agent_role_for_test(&mut config, "custom")
        .await
        .expect("custom role should materialize");
    apply_role_to_config(&mut config, Some("custom"))
        .await
        .expect("custom role should apply");

    assert_eq!(config.model.as_deref(), Some("role-model"));
}

#[tokio::test]
async fn apply_role_preserves_unspecified_keys() {
    let (home, mut config) = test_config_with_cli_overrides(vec![(
        "model".to_string(),
        TomlValue::String("base-model".to_string()),
    )])
    .await;
    config.codex_linux_sandbox_exe = Some(PathBuf::from("/tmp/codex-linux-sandbox"));
    config.main_execve_wrapper_exe = Some(PathBuf::from("/tmp/codex-execve-wrapper"));
    let role_path = write_role_config(
        &home,
        "effort-only.toml",
        "developer_instructions = \"Stay focused\"\nmodel_reasoning_effort = \"high\"",
    )
    .await;
    config.agent_roles.insert(
        "custom".to_string(),
        AgentRoleConfig {
            description: None,
            config_file: Some(role_path),
            nickname_candidates: None,
        },
    );

    crate::config::agent_roles::materialize_agent_role_for_test(&mut config, "custom")
        .await
        .expect("custom role should materialize");
    apply_role_to_config(&mut config, Some("custom"))
        .await
        .expect("custom role should apply");

    assert_eq!(config.model.as_deref(), Some("base-model"));
    assert_eq!(config.model_reasoning_effort, Some(ReasoningEffort::High));
    assert_eq!(
        config.codex_linux_sandbox_exe,
        Some(PathBuf::from("/tmp/codex-linux-sandbox"))
    );
    assert_eq!(
        config.main_execve_wrapper_exe,
        Some(PathBuf::from("/tmp/codex-execve-wrapper"))
    );
}

#[tokio::test]
async fn apply_role_preserves_unspecified_runtime_model_and_instructions() {
    let (home, mut config) = test_config_with_cli_overrides(Vec::new()).await;
    config.model = Some("runtime-model".to_string());
    config.model_reasoning_effort = Some(ReasoningEffort::High);
    config.model_reasoning_summary = Some(ReasoningSummary::Detailed);
    config.base_instructions = Some("Runtime base instructions".to_string());
    config.developer_instructions = Some("Runtime developer instructions".to_string());
    let role_path = write_role_config(
        &home,
        "unrelated-setting-role.toml",
        "show_raw_agent_reasoning = true",
    )
    .await;
    config.agent_roles.insert(
        "custom".to_string(),
        AgentRoleConfig {
            description: None,
            config_file: Some(role_path),
            nickname_candidates: None,
        },
    );

    crate::config::agent_roles::materialize_agent_role_for_test(&mut config, "custom")
        .await
        .expect("custom role should materialize");
    apply_role_to_config(&mut config, Some("custom"))
        .await
        .expect("custom role should apply");

    assert_eq!(
        (
            config.model.as_deref(),
            config.model_reasoning_effort,
            config.model_reasoning_summary,
            config.base_instructions.as_deref(),
            config.developer_instructions.as_deref(),
        ),
        (
            Some("runtime-model"),
            Some(ReasoningEffort::High),
            Some(ReasoningSummary::Detailed),
            Some("Runtime base instructions"),
            Some("Runtime developer instructions"),
        )
    );
    assert!(config.show_raw_agent_reasoning);
}

#[tokio::test]
async fn reapply_role_on_resume_preserves_runtime_owned_security_context() {
    let (home, mut config) = test_config_with_cli_overrides(Vec::new()).await;
    let role_path = write_role_config(
        &home,
        "resumed-role.toml",
        r#"
model = "current-role-model"
developer_instructions = "Current trusted role instructions"
approval_policy = "never"
approvals_reviewer = "user"
sandbox_mode = "danger-full-access"
allow_login_shell = false

[shell_environment_policy]
inherit = "none"
ignore_default_excludes = false
"#,
    )
    .await;
    config.agent_roles.insert(
        "custom".to_string(),
        AgentRoleConfig {
            description: None,
            config_file: Some(role_path),
            nickname_candidates: None,
        },
    );

    crate::config::agent_roles::materialize_agent_role_for_test(&mut config, "custom")
        .await
        .expect("custom role should materialize");

    let runtime_cwd = home.path().join("runtime-cwd");
    let runtime_workspace_root = home.path().join("runtime-root").abs();
    config.cwd = runtime_cwd.clone().abs();
    config.workspace_roots = vec![runtime_workspace_root.clone()];
    config.workspace_roots_explicit = true;
    config
        .permissions
        .set_workspace_roots(vec![runtime_workspace_root.clone()]);
    config.base_instructions = Some("Runtime base instructions".to_string());
    config
        .permissions
        .approval_policy
        .set(codex_protocol::protocol::AskForApproval::OnRequest)
        .expect("runtime approval policy should be accepted");
    config.approvals_reviewer = codex_protocol::config_types::ApprovalsReviewer::AutoReview;
    let runtime_permissions = config.permissions.clone();
    let source = SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
        parent_thread_id: codex_protocol::ThreadId::new(),
        depth: 1,
        agent_path: Some(
            codex_protocol::AgentPath::try_from("/root/custom").expect("valid agent path"),
        ),
        agent_nickname: None,
        agent_role: Some("custom".to_string()),
    });

    reapply_role_to_resumed_agent_config(&mut config, &source, ResumeRoleOverridePolicy::RoleWins)
        .await
        .expect("current trusted role should be reapplied");

    assert_eq!(config.model.as_deref(), Some("current-role-model"));
    assert_eq!(
        config.developer_instructions.as_deref(),
        Some("Current trusted role instructions")
    );
    assert_eq!(
        config.base_instructions.as_deref(),
        Some("Runtime base instructions")
    );
    assert_eq!(config.cwd, runtime_cwd.abs());
    assert_eq!(config.permissions, runtime_permissions);
    assert_eq!(
        config.approvals_reviewer,
        codex_protocol::config_types::ApprovalsReviewer::AutoReview
    );
    assert_eq!(config.workspace_roots, vec![runtime_workspace_root]);
    assert!(config.workspace_roots_explicit);
}

#[tokio::test]
async fn reapply_role_on_resume_preserves_explicit_session_overrides() {
    let (home, mut config) = test_config_with_cli_overrides(vec![
        (
            "model".to_string(),
            TomlValue::String("request-model".to_string()),
        ),
        (
            "model_reasoning_effort".to_string(),
            TomlValue::String("high".to_string()),
        ),
        (
            "developer_instructions".to_string(),
            TomlValue::String("Request instructions".to_string()),
        ),
        (
            "service_tier".to_string(),
            TomlValue::String("priority".to_string()),
        ),
        (
            "personality".to_string(),
            TomlValue::String("friendly".to_string()),
        ),
    ])
    .await;
    let role_path = write_role_config(
        &home,
        "override-role.toml",
        r#"
model = "role-model"
model_reasoning_effort = "low"
developer_instructions = "Role instructions"
service_tier = "flex"
personality = "pragmatic"
"#,
    )
    .await;
    config.agent_roles.insert(
        "custom".to_string(),
        AgentRoleConfig {
            description: None,
            config_file: Some(role_path),
            nickname_candidates: None,
        },
    );
    crate::config::agent_roles::materialize_agent_role_for_test(&mut config, "custom")
        .await
        .expect("custom role should materialize");
    let source = SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
        parent_thread_id: codex_protocol::ThreadId::new(),
        depth: 1,
        agent_path: Some(
            codex_protocol::AgentPath::try_from("/root/custom").expect("valid agent path"),
        ),
        agent_nickname: None,
        agent_role: Some("custom".to_string()),
    });

    reapply_role_to_resumed_agent_config(
        &mut config,
        &source,
        ResumeRoleOverridePolicy::ExplicitSessionFlagsWin,
    )
    .await
    .expect("current trusted role should apply beneath explicit session flags");

    assert_eq!(
        (
            config.model.as_deref(),
            config.model_reasoning_effort,
            config.developer_instructions.as_deref(),
            config.service_tier.as_deref(),
            config.personality,
        ),
        (
            Some("request-model"),
            Some(ReasoningEffort::High),
            Some("Request instructions"),
            Some("priority"),
            Some(Personality::Friendly),
        )
    );
}

#[tokio::test]
async fn reapply_role_on_resume_does_not_treat_persisted_reasoning_as_explicit() {
    let (home, mut config) = test_config_with_cli_overrides(Vec::new()).await;
    config.model_reasoning_effort = Some(ReasoningEffort::High);
    let role_path = write_role_config(
        &home,
        "persisted-reasoning-role.toml",
        "model_reasoning_effort = \"low\"",
    )
    .await;
    config.agent_roles.insert(
        "custom".to_string(),
        AgentRoleConfig {
            description: None,
            config_file: Some(role_path),
            nickname_candidates: None,
        },
    );
    crate::config::agent_roles::materialize_agent_role_for_test(&mut config, "custom")
        .await
        .expect("custom role should materialize");
    let source = SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
        parent_thread_id: codex_protocol::ThreadId::new(),
        depth: 1,
        agent_path: Some(
            codex_protocol::AgentPath::try_from("/root/custom").expect("valid agent path"),
        ),
        agent_nickname: None,
        agent_role: Some("custom".to_string()),
    });

    reapply_role_to_resumed_agent_config(
        &mut config,
        &source,
        ResumeRoleOverridePolicy::ExplicitSessionFlagsWin,
    )
    .await
    .expect("trusted role should override unmarked persisted reasoning");

    assert_eq!(config.model_reasoning_effort, Some(ReasoningEffort::Low));
}

#[tokio::test]
async fn resume_without_persisted_role_does_not_guess_default_role() {
    let (home, mut config) = test_config_with_cli_overrides(Vec::new()).await;
    let default_role_path = write_role_config(
        &home,
        "default-role.toml",
        "model = \"unexpected-default-model\"",
    )
    .await;
    config.agent_roles.insert(
        DEFAULT_ROLE_NAME.to_string(),
        AgentRoleConfig {
            description: Some("Configured default role".to_string()),
            config_file: Some(default_role_path),
            nickname_candidates: None,
        },
    );
    let before = config.clone();
    let source = SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
        parent_thread_id: codex_protocol::ThreadId::new(),
        depth: 1,
        agent_path: Some(
            codex_protocol::AgentPath::try_from("/root/full_history").expect("valid agent path"),
        ),
        agent_nickname: None,
        agent_role: None,
    });

    reapply_role_to_resumed_agent_config(&mut config, &source, ResumeRoleOverridePolicy::RoleWins)
        .await
        .expect("historical role-less resume should remain compatible");

    assert_eq!(config, before);
}

#[tokio::test]
async fn apply_role_reports_explicit_service_tier() {
    let (home, mut config) = test_config_with_cli_overrides(Vec::new()).await;
    let role_path = write_role_config(
        &home,
        "tiered-role.toml",
        r#"developer_instructions = "Stay focused"
service_tier = "priority"
"#,
    )
    .await;
    config.agent_roles.insert(
        "custom".to_string(),
        AgentRoleConfig {
            description: None,
            config_file: Some(role_path),
            nickname_candidates: None,
        },
    );

    crate::config::agent_roles::materialize_agent_role_for_test(&mut config, "custom")
        .await
        .expect("custom role should materialize");
    apply_role_to_config(&mut config, Some("custom"))
        .await
        .expect("custom role should apply");

    assert_eq!(
        config.service_tier,
        Some(ServiceTier::Fast.request_value().to_string())
    );
}

#[tokio::test]
async fn apply_role_preserves_existing_service_tier_without_override() {
    let (home, mut config) = test_config_with_cli_overrides(Vec::new()).await;
    config.service_tier = Some(ServiceTier::Fast.request_value().to_string());
    let role_path = write_role_config(
        &home,
        "default-tier-role.toml",
        r#"developer_instructions = "Stay focused"
"#,
    )
    .await;
    config.agent_roles.insert(
        "custom".to_string(),
        AgentRoleConfig {
            description: None,
            config_file: Some(role_path),
            nickname_candidates: None,
        },
    );

    crate::config::agent_roles::materialize_agent_role_for_test(&mut config, "custom")
        .await
        .expect("custom role should materialize");
    apply_role_to_config(&mut config, Some("custom"))
        .await
        .expect("custom role should apply");

    assert_eq!(
        config.service_tier,
        Some(ServiceTier::Fast.request_value().to_string())
    );
}

#[tokio::test]
#[cfg(not(windows))]
async fn apply_role_does_not_materialize_default_sandbox_workspace_write_fields() {
    use codex_protocol::protocol::SandboxPolicy;
    let (home, mut config) = test_config_with_cli_overrides(vec![
        (
            "sandbox_mode".to_string(),
            TomlValue::String("workspace-write".to_string()),
        ),
        (
            "sandbox_workspace_write.network_access".to_string(),
            TomlValue::Boolean(true),
        ),
    ])
    .await;
    let role_path = write_role_config(
        &home,
        "sandbox-role.toml",
        r#"developer_instructions = "Stay focused"

[sandbox_workspace_write]
writable_roots = ["./sandbox-root"]
"#,
    )
    .await;
    config.agent_roles.insert(
        "custom".to_string(),
        AgentRoleConfig {
            description: None,
            config_file: Some(role_path),
            nickname_candidates: None,
        },
    );

    crate::config::agent_roles::materialize_agent_role_for_test(&mut config, "custom")
        .await
        .expect("custom role should materialize");
    apply_role_to_config(&mut config, Some("custom"))
        .await
        .expect("custom role should apply");

    let role_layer = config
        .config_layer_stack
        .get_layers(
            ConfigLayerStackOrdering::LowestPrecedenceFirst,
            /*include_disabled*/ true,
        )
        .into_iter()
        .rfind(|layer| layer.name == ConfigLayerSource::SessionFlags)
        .expect("expected a session flags layer");
    let sandbox_workspace_write = role_layer
        .config
        .get("sandbox_workspace_write")
        .and_then(TomlValue::as_table)
        .expect("role layer should include sandbox_workspace_write");
    assert_eq!(
        sandbox_workspace_write.contains_key("network_access"),
        false
    );
    assert_eq!(
        sandbox_workspace_write.contains_key("exclude_tmpdir_env_var"),
        false
    );
    assert_eq!(
        sandbox_workspace_write.contains_key("exclude_slash_tmp"),
        false
    );

    match &config.legacy_sandbox_policy() {
        SandboxPolicy::WorkspaceWrite { network_access, .. } => {
            assert_eq!(*network_access, true);
        }
        other => panic!("expected workspace-write sandbox policy, got {other:?}"),
    }
}

#[tokio::test]
async fn apply_role_takes_precedence_over_existing_session_flags_for_same_key() {
    let (home, mut config) = test_config_with_cli_overrides(vec![(
        "model".to_string(),
        TomlValue::String("cli-model".to_string()),
    )])
    .await;
    let before_layers = session_flags_layer_count(&config);
    let role_path = write_role_config(
        &home,
        "model-role.toml",
        "developer_instructions = \"Stay focused\"\nmodel = \"role-model\"",
    )
    .await;
    config.agent_roles.insert(
        "custom".to_string(),
        AgentRoleConfig {
            description: None,
            config_file: Some(role_path),
            nickname_candidates: None,
        },
    );

    crate::config::agent_roles::materialize_agent_role_for_test(&mut config, "custom")
        .await
        .expect("custom role should materialize");
    apply_role_to_config(&mut config, Some("custom"))
        .await
        .expect("custom role should apply");

    assert_eq!(config.model.as_deref(), Some("role-model"));
    assert_eq!(session_flags_layer_count(&config), before_layers + 1);
}

#[cfg_attr(windows, ignore)]
#[tokio::test]
async fn apply_role_skills_config_disables_skill_for_spawned_agent() {
    let (home, mut config) = test_config_with_cli_overrides(Vec::new()).await;
    let skill_dir = home.path().join("skills").join("demo");
    fs::create_dir_all(&skill_dir).expect("create skill dir");
    let skill_path = skill_dir.join("SKILL.md");
    fs::write(
        &skill_path,
        "---\nname: demo-skill\ndescription: demo description\n---\n\n# Body\n",
    )
    .expect("write skill");
    let role_path = write_role_config(
        &home,
        "skills-role.toml",
        &format!(
            r#"developer_instructions = "Stay focused"

[[skills.config]]
path = "{}"
enabled = false
"#,
            skill_path.display()
        ),
    )
    .await;
    config.agent_roles.insert(
        "custom".to_string(),
        AgentRoleConfig {
            description: None,
            config_file: Some(role_path),
            nickname_candidates: None,
        },
    );

    crate::config::agent_roles::materialize_agent_role_for_test(&mut config, "custom")
        .await
        .expect("custom role should materialize");
    apply_role_to_config(&mut config, Some("custom"))
        .await
        .expect("custom role should apply");

    let plugins_manager = Arc::new(PluginsManager::new(home.path().to_path_buf()));
    let skills_service =
        SkillsService::new(home.path().abs(), /*bundled_skills_enabled*/ true);
    let plugins_input = config.plugins_config_input();
    let plugin_outcome = plugins_manager.plugins_for_config(&plugins_input).await;
    let effective_skill_roots = plugin_outcome.effective_plugin_skill_roots();
    let plugin_skill_snapshots = plugins_manager.plugin_skill_snapshots_for_config(&plugins_input);
    let skills_input = skills_load_input_from_config(&config, effective_skill_roots)
        .with_plugin_skill_snapshots(plugin_skill_snapshots);
    let snapshot = skills_service
        .snapshot_for_config(
            &skills_input,
            Some(Arc::clone(&codex_exec_server::LOCAL_FS)),
        )
        .await;
    let outcome = snapshot.outcome();
    let skill = outcome
        .skills
        .iter()
        .find(|skill| skill.name == "demo-skill")
        .expect("demo skill should be discovered");

    assert_eq!(outcome.is_skill_enabled(skill), false);
}

#[test]
fn spawn_tool_spec_build_deduplicates_user_defined_built_in_roles() {
    let user_defined_roles = BTreeMap::from([
        (
            "explorer".to_string(),
            AgentRoleConfig {
                description: Some("user override".to_string()),
                config_file: None,
                nickname_candidates: None,
            },
        ),
        ("researcher".to_string(), AgentRoleConfig::default()),
    ]);

    let spec = spawn_tool_spec::build(&user_defined_roles);

    assert!(spec.contains("researcher: no description"));
    assert!(spec.contains("explorer: {\nuser override\n}"));
    assert!(spec.contains("default: {\nDefault agent.\n}"));
    assert!(!spec.contains("Explorers are fast and authoritative."));
}

#[test]
fn spawn_tool_spec_lists_user_defined_roles_before_built_ins() {
    let user_defined_roles = BTreeMap::from([(
        "aaa".to_string(),
        AgentRoleConfig {
            description: Some("first".to_string()),
            config_file: None,
            nickname_candidates: None,
        },
    )]);

    let spec = spawn_tool_spec::build(&user_defined_roles);
    let user_index = spec.find("aaa: {\nfirst\n}").expect("find user role");
    let built_in_index = spec
        .find("default: {\nDefault agent.\n}")
        .expect("find built-in role");

    assert!(user_index < built_in_index);
}

#[test]
fn legacy_v1_spawn_tool_spec_preserves_role_locked_setting_guidance() {
    let role_config: TomlValue = toml::from_str(
        "model = \"gpt-5\"\nmodel_reasoning_effort = \"high\"\nservice_tier = \"priority\"\n",
    )
    .expect("parse materialized role config");
    let user_defined_roles = BTreeMap::from([(
        "researcher".to_string(),
        AgentRoleConfig {
            description: Some(format!(
                "Research carefully.{}",
                crate::config::agent_roles::agent_role_locked_settings_note(&role_config)
            )),
            config_file: None,
            nickname_candidates: None,
        },
    )]);

    let spec = spawn_tool_spec::build_legacy_v1(&user_defined_roles);

    assert!(spec.contains(
        "Research carefully.\n- This role's model is set to `gpt-5` and its reasoning effort is set to `high`. These settings cannot be changed."
    ));
    assert!(spec.contains(
        "This role's service tier is set to `priority`. If it is supported by the resolved model, it takes precedence over a valid spawn request service tier."
    ));
}

#[test]
fn spawn_tool_spec_bounds_role_catalog_and_preserves_default_role() {
    let user_defined_roles = (0..40)
        .map(|index| {
            (
                format!("role_{index:02}"),
                AgentRoleConfig {
                    description: Some("界".repeat(2_000)),
                    config_file: None,
                    nickname_candidates: None,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();

    let spec = spawn_tool_spec::build(&user_defined_roles);

    assert!(
        serde_json::to_string(&spec)
            .expect("serialize role catalog")
            .len()
            <= spawn_tool_spec::MAX_AGENT_ROLE_CATALOG_JSON_BYTES
    );
    assert!(spec.contains("[role metadata truncated]"));
    assert!(spec.contains("additional role(s) omitted"));
    assert!(spec.contains("default: {\nDefault agent.\n}"));
}

#[test]
fn built_in_config_file_contents_resolves_explorer_only() {
    assert_eq!(
        built_in::config_file_contents(Path::new("missing.toml")),
        None
    );
}
