use super::*;
use crate::config::ConfigBuilder;
use codex_app_server_protocol::ConfigLayerSource;
use codex_config::AbsolutePathBuf;
use codex_config::CONFIG_TOML_FILE;
use codex_config::ConfigLayerEntry;
use codex_config::ConfigLayerStack;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::protocol::SubAgentActionPolicyAction;
use pretty_assertions::assert_eq;
use tempfile::TempDir;

async fn test_config() -> (TempDir, Config) {
    let home = TempDir::new().expect("create temp dir");
    let config = ConfigBuilder::default()
        .codex_home(home.path().to_path_buf())
        .fallback_cwd(Some(home.path().to_path_buf()))
        .build()
        .await
        .expect("load test config");
    (home, config)
}

#[tokio::test]
async fn list_agent_role_templates_merges_user_roles_before_built_ins() {
    let (_home, mut config) = test_config().await;
    config.agent_roles.insert(
        "reviewer".to_string(),
        AgentRoleConfig {
            description: Some("Review code changes".to_string()),
            config_file: None,
            nickname_candidates: Some(vec!["Ada".to_string()]),
            metadata_sources: Default::default(),
            runtime_config_sources: Default::default(),
        },
    );

    let entries = list_agent_role_templates(&config);

    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>(),
        vec!["reviewer", "default", "explorer", "reviewer", "worker"]
    );
    assert_eq!(entries[0].source, AgentRoleTemplateSource::User);
    assert_eq!(entries[1].source, AgentRoleTemplateSource::BuiltIn);
    assert!(entries[0].shadows_built_in);
    assert!(entries[3].is_shadowed);
}

#[tokio::test]
async fn list_agent_role_templates_marks_shadowed_built_in_roles() {
    let (_home, mut config) = test_config().await;
    config.agent_roles.insert(
        "explorer".to_string(),
        AgentRoleConfig {
            description: Some("Custom explorer".to_string()),
            config_file: None,
            nickname_candidates: Some(vec!["Custom".to_string()]),
            metadata_sources: Default::default(),
            runtime_config_sources: Default::default(),
        },
    );

    let entries = list_agent_role_templates(&config);

    assert_eq!(
        entries
            .iter()
            .map(|entry| (
                entry.name.as_str(),
                entry.source,
                entry.is_shadowed,
                entry.shadows_built_in,
                entry.availability
            ))
            .collect::<Vec<_>>(),
        vec![
            (
                "explorer",
                AgentRoleTemplateSource::User,
                false,
                true,
                AgentRoleTemplateAvailability::NewSessionsOnly,
            ),
            (
                "default",
                AgentRoleTemplateSource::BuiltIn,
                false,
                false,
                AgentRoleTemplateAvailability::BuiltIn,
            ),
            (
                "explorer",
                AgentRoleTemplateSource::BuiltIn,
                true,
                false,
                AgentRoleTemplateAvailability::ShadowedBuiltIn,
            ),
            (
                "reviewer",
                AgentRoleTemplateSource::BuiltIn,
                false,
                false,
                AgentRoleTemplateAvailability::BuiltIn,
            ),
            (
                "worker",
                AgentRoleTemplateSource::BuiltIn,
                false,
                false,
                AgentRoleTemplateAvailability::BuiltIn,
            ),
        ]
    );
}

#[tokio::test]
async fn list_agent_role_templates_reports_user_role_file_validation_errors() {
    let (home, mut config) = test_config().await;
    let role_path = home.path().join("agents/reviewer.toml");
    fs::create_dir_all(role_path.parent().expect("role path parent")).expect("create dir");
    fs::write(
        &role_path,
        r#"name = "reviewer"
description = "Review code changes"
developer_instructions = "   "
"#,
    )
    .expect("write invalid role");
    config.agent_roles.insert(
        "reviewer".to_string(),
        AgentRoleConfig {
            description: Some("Review code changes".to_string()),
            config_file: Some(role_path),
            nickname_candidates: None,
            metadata_sources: Default::default(),
            runtime_config_sources: Default::default(),
        },
    );

    let entries = list_agent_role_templates(&config);

    assert!(
        entries[0]
            .validation_error
            .as_deref()
            .is_some_and(|err| err.contains("developer_instructions cannot be blank"))
    );
}

#[tokio::test]
async fn list_agent_role_templates_reports_tool_selection_validation_errors() {
    let (home, mut config) = test_config().await;
    let role_path = home.path().join("agents/reviewer.toml");
    fs::create_dir_all(role_path.parent().expect("role path parent")).expect("create dir");
    fs::write(
        &role_path,
        r#"name = "reviewer"
description = "Review code changes"
developer_instructions = "Review code changes and report concrete risks."

[tool_selection]
allowed_tools = ["update_plan", "update_plan"]
"#,
    )
    .expect("write invalid role");
    config.agent_roles.insert(
        "reviewer".to_string(),
        AgentRoleConfig {
            description: Some("Review code changes".to_string()),
            config_file: Some(role_path),
            nickname_candidates: None,
            metadata_sources: Default::default(),
            runtime_config_sources: Default::default(),
        },
    );

    let entries = list_agent_role_templates(&config);

    assert!(
        entries[0]
            .validation_error
            .as_deref()
            .is_some_and(|err| err.contains("duplicate tool_selection.allowed_tools entry"))
    );
}

#[tokio::test]
async fn list_agent_role_templates_reports_model_provider_locks() {
    let (home, mut config) = test_config().await;
    let role_path = home.path().join("agents/reviewer.toml");
    fs::create_dir_all(role_path.parent().expect("role path parent")).expect("create dir");
    fs::write(
        &role_path,
        r#"name = "reviewer"
description = "Review code changes"
developer_instructions = "Review code changes and report concrete risks."
model = "deepseek/deepseek-v4-flash"
model_provider = "deepseek"
model_reasoning_effort = "high"
service_tier = "priority"
approval_policy = "on-request"
default_permissions = ":read-only"

[tool_selection]
allowed_tools = ["update_plan", "codex_app/lookup"]
denied_tools = ["apply_patch", "shell_command"]

[permissions.reviewer]
description = "Reviewer read-only profile"

[mcp_servers.local]
command = "codex-mcp"

[hooks]
PreToolUse = [{ matcher = "*", hooks = [{ type = "command", command = "echo safe" }] }]

[skills]
include_instructions = false

[[skills.config]]
name = "rust"
enabled = true

[apps._default]
default_tools_enabled = false

[apps.linear]
enabled = true
"#,
    )
    .expect("write valid role");
    config.agent_roles.insert(
        "reviewer".to_string(),
        AgentRoleConfig {
            description: Some("Review code changes".to_string()),
            config_file: Some(role_path),
            nickname_candidates: None,
            metadata_sources: Default::default(),
            runtime_config_sources: Default::default(),
        },
    );

    let entries = list_agent_role_templates(&config);

    assert_eq!(
        entries[0].locks,
        AgentRoleTemplateLocks {
            model: Some("deepseek/deepseek-v4-flash".to_string()),
            model_provider: Some("deepseek".to_string()),
            reasoning_effort: Some("high".to_string()),
            service_tier: Some("priority".to_string()),
            allowed_tool_names: vec!["update_plan".to_string(), "codex_app/lookup".to_string()],
            denied_tool_names: vec!["apply_patch".to_string(), "shell_command".to_string()],
            approval_policy: Some("on-request".to_string()),
            sandbox_mode: None,
            default_permissions: Some(":read-only".to_string()),
            has_sandbox_workspace_write: false,
            permission_profile_count: 1,
            mcp_server_count: 1,
            hook_handler_count: 1,
            skill_config_count: 1,
            app_config_count: 1,
            has_apps_default_config: true,
            has_developer_instructions: true,
        }
    );
    assert_eq!(
        entries[0].field_sources,
        AgentRoleTemplateFieldSources {
            declaration_fields: vec!["description".to_string(), "config_file".to_string()],
            declaration_origins: Vec::new(),
            config_file_fields: vec![
                "name".to_string(),
                "description".to_string(),
                "developer_instructions".to_string(),
                "model".to_string(),
                "model_provider".to_string(),
                "model_reasoning_effort".to_string(),
                "service_tier".to_string(),
                "approval_policy".to_string(),
                "default_permissions".to_string(),
                "permissions".to_string(),
                "mcp_servers".to_string(),
                "hooks".to_string(),
                "tool_selection.allowed_tools".to_string(),
                "tool_selection.denied_tools".to_string(),
                "skills.config".to_string(),
                "apps".to_string(),
            ],
            bounded_provenance: vec![
                AgentRoleTemplateFieldProvenance {
                    field_name: "description".to_string(),
                    source: AgentRoleTemplateFieldProvenanceSource::RoleFileMetadata,
                    detail: None,
                },
                AgentRoleTemplateFieldProvenance {
                    field_name: "config_file".to_string(),
                    source: AgentRoleTemplateFieldProvenanceSource::DiscoveredRoleFile,
                    detail: None,
                },
                AgentRoleTemplateFieldProvenance {
                    field_name: "approval_policy".to_string(),
                    source: AgentRoleTemplateFieldProvenanceSource::EffectiveRoleFile,
                    detail: None,
                },
                AgentRoleTemplateFieldProvenance {
                    field_name: "apps".to_string(),
                    source: AgentRoleTemplateFieldProvenanceSource::EffectiveRoleFile,
                    detail: None,
                },
                AgentRoleTemplateFieldProvenance {
                    field_name: "default_permissions".to_string(),
                    source: AgentRoleTemplateFieldProvenanceSource::EffectiveRoleFile,
                    detail: None,
                },
                AgentRoleTemplateFieldProvenance {
                    field_name: "developer_instructions".to_string(),
                    source: AgentRoleTemplateFieldProvenanceSource::EffectiveRoleFile,
                    detail: None,
                },
                AgentRoleTemplateFieldProvenance {
                    field_name: "hooks".to_string(),
                    source: AgentRoleTemplateFieldProvenanceSource::EffectiveRoleFile,
                    detail: None,
                },
                AgentRoleTemplateFieldProvenance {
                    field_name: "mcp_servers".to_string(),
                    source: AgentRoleTemplateFieldProvenanceSource::EffectiveRoleFile,
                    detail: None,
                },
                AgentRoleTemplateFieldProvenance {
                    field_name: "model".to_string(),
                    source: AgentRoleTemplateFieldProvenanceSource::EffectiveRoleFile,
                    detail: None,
                },
                AgentRoleTemplateFieldProvenance {
                    field_name: "model_provider".to_string(),
                    source: AgentRoleTemplateFieldProvenanceSource::EffectiveRoleFile,
                    detail: None,
                },
                AgentRoleTemplateFieldProvenance {
                    field_name: "model_reasoning_effort".to_string(),
                    source: AgentRoleTemplateFieldProvenanceSource::EffectiveRoleFile,
                    detail: None,
                },
                AgentRoleTemplateFieldProvenance {
                    field_name: "name".to_string(),
                    source: AgentRoleTemplateFieldProvenanceSource::RoleFileMetadata,
                    detail: None,
                },
                AgentRoleTemplateFieldProvenance {
                    field_name: "permissions".to_string(),
                    source: AgentRoleTemplateFieldProvenanceSource::EffectiveRoleFile,
                    detail: None,
                },
                AgentRoleTemplateFieldProvenance {
                    field_name: "service_tier".to_string(),
                    source: AgentRoleTemplateFieldProvenanceSource::EffectiveRoleFile,
                    detail: None,
                },
                AgentRoleTemplateFieldProvenance {
                    field_name: "skills.config".to_string(),
                    source: AgentRoleTemplateFieldProvenanceSource::EffectiveRoleFile,
                    detail: None,
                },
                AgentRoleTemplateFieldProvenance {
                    field_name: "tool_selection.allowed_tools".to_string(),
                    source: AgentRoleTemplateFieldProvenanceSource::EffectiveRoleFile,
                    detail: None,
                },
                AgentRoleTemplateFieldProvenance {
                    field_name: "tool_selection.denied_tools".to_string(),
                    source: AgentRoleTemplateFieldProvenanceSource::EffectiveRoleFile,
                    detail: None,
                },
            ],
        }
    );
}

#[tokio::test]
async fn list_agent_role_templates_reports_declaration_layer_origins() {
    let (home, mut config) = test_config().await;
    let config_path = AbsolutePathBuf::from_absolute_path(home.path().join("config.toml"))
        .expect("absolute config path");
    let layer_toml: TomlValue = toml::from_str(
        r#"
[agents.reviewer]
description = "Review code changes"
nickname_candidates = ["Ada"]
"#,
    )
    .expect("layer toml");
    config.config_layer_stack = ConfigLayerStack::new(
        vec![ConfigLayerEntry::new(
            ConfigLayerSource::User {
                file: config_path.clone(),
                profile: None,
            },
            layer_toml,
        )],
        config.config_layer_stack.requirements().clone(),
        config.config_layer_stack.requirements_toml().clone(),
    )
    .expect("layer stack");
    config.agent_roles.insert(
        "reviewer".to_string(),
        AgentRoleConfig {
            description: Some("Review code changes".to_string()),
            config_file: None,
            nickname_candidates: Some(vec!["Ada".to_string()]),
            metadata_sources: Default::default(),
            runtime_config_sources: Default::default(),
        },
    );

    let entries = list_agent_role_templates(&config);

    assert_eq!(
        entries[0].field_sources.declaration_origins,
        vec![
            format!("description: user ({})", config_path.as_path().display()),
            format!(
                "nickname_candidates: user ({})",
                config_path.as_path().display()
            ),
        ]
    );
    assert_eq!(
        entries[0].field_sources.bounded_provenance,
        vec![
            AgentRoleTemplateFieldProvenance {
                field_name: "description".to_string(),
                source: AgentRoleTemplateFieldProvenanceSource::ConfigLayer,
                detail: Some(format!("user ({})", config_path.as_path().display())),
            },
            AgentRoleTemplateFieldProvenance {
                field_name: "nickname_candidates".to_string(),
                source: AgentRoleTemplateFieldProvenanceSource::ConfigLayer,
                detail: Some(format!("user ({})", config_path.as_path().display())),
            },
        ]
    );
}

#[tokio::test]
async fn list_agent_role_templates_reports_role_file_metadata_source() {
    let (home, mut config) = test_config().await;
    let role_path = home.path().join("agents/reviewer.toml");
    fs::create_dir_all(role_path.parent().expect("role path parent")).expect("create dir");
    fs::write(
        &role_path,
        r#"name = "reviewer"
description = "Review from role file"
nickname_candidates = ["FileAda"]
developer_instructions = "Review code changes and report concrete risks."
"#,
    )
    .expect("write role");
    config.agent_roles.insert(
        "reviewer".to_string(),
        AgentRoleConfig {
            description: Some("Review from role file".to_string()),
            config_file: Some(role_path),
            nickname_candidates: Some(vec!["FileAda".to_string()]),
            metadata_sources: Default::default(),
            runtime_config_sources: Default::default(),
        },
    );

    let entries = list_agent_role_templates(&config);

    assert_eq!(
        entries[0].field_sources.bounded_provenance,
        vec![
            AgentRoleTemplateFieldProvenance {
                field_name: "description".to_string(),
                source: AgentRoleTemplateFieldProvenanceSource::RoleFileMetadata,
                detail: None,
            },
            AgentRoleTemplateFieldProvenance {
                field_name: "config_file".to_string(),
                source: AgentRoleTemplateFieldProvenanceSource::DiscoveredRoleFile,
                detail: None,
            },
            AgentRoleTemplateFieldProvenance {
                field_name: "nickname_candidates".to_string(),
                source: AgentRoleTemplateFieldProvenanceSource::RoleFileMetadata,
                detail: None,
            },
            AgentRoleTemplateFieldProvenance {
                field_name: "developer_instructions".to_string(),
                source: AgentRoleTemplateFieldProvenanceSource::EffectiveRoleFile,
                detail: None,
            },
            AgentRoleTemplateFieldProvenance {
                field_name: "name".to_string(),
                source: AgentRoleTemplateFieldProvenanceSource::RoleFileMetadata,
                detail: None,
            },
        ]
    );
}

#[tokio::test]
async fn list_agent_role_templates_reports_inherited_and_overridden_metadata_sources()
-> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;
    let nested_cwd = repo_root.path().join("packages").join("app");
    std::fs::create_dir_all(repo_root.path().join(".git"))?;
    std::fs::create_dir_all(&nested_cwd)?;

    let workspace_key = repo_root.path().to_string_lossy().replace('\\', "\\\\");
    std::fs::write(
        codex_home.path().join(CONFIG_TOML_FILE),
        format!(
            r#"[projects."{workspace_key}"]
trust_level = "trusted"

[agents.researcher]
description = "Research role from config"
config_file = "./agents/researcher.toml"
nickname_candidates = ["Noether"]
"#
        ),
    )?;

    let home_agents_dir = codex_home.path().join("agents");
    std::fs::create_dir_all(&home_agents_dir)?;
    std::fs::write(
        home_agents_dir.join("researcher.toml"),
        r#"
developer_instructions = "Research carefully"
model = "gpt-5.2"
"#,
    )?;

    let standalone_agents_dir = repo_root.path().join(".codex").join("agents");
    std::fs::create_dir_all(&standalone_agents_dir)?;
    std::fs::write(
        standalone_agents_dir.join("researcher.toml"),
        r#"
name = "researcher"
nickname_candidates = ["Hypatia"]
developer_instructions = "Research from file"
model = "gpt-5-mini"
"#,
    )?;

    let config = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .harness_overrides(crate::config::ConfigOverrides {
            cwd: Some(nested_cwd),
            ..Default::default()
        })
        .build()
        .await?;

    let entries = list_agent_role_templates(&config);
    let researcher = entries
        .iter()
        .find(|entry| entry.name == "researcher" && entry.source == AgentRoleTemplateSource::User)
        .expect("researcher role");

    assert_eq!(
        researcher.field_sources.bounded_provenance,
        vec![
            AgentRoleTemplateFieldProvenance {
                field_name: "description".to_string(),
                source: AgentRoleTemplateFieldProvenanceSource::ConfigLayer,
                detail: Some(format!(
                    "user ({}); inherited from lower-precedence role",
                    codex_home.path().join(CONFIG_TOML_FILE).display()
                )),
            },
            AgentRoleTemplateFieldProvenance {
                field_name: "config_file".to_string(),
                source: AgentRoleTemplateFieldProvenanceSource::DiscoveredRoleFile,
                detail: Some("overrides lower-precedence role".to_string()),
            },
            AgentRoleTemplateFieldProvenance {
                field_name: "nickname_candidates".to_string(),
                source: AgentRoleTemplateFieldProvenanceSource::RoleFileMetadata,
                detail: Some("overrides lower-precedence role".to_string()),
            },
            AgentRoleTemplateFieldProvenance {
                field_name: "developer_instructions".to_string(),
                source: AgentRoleTemplateFieldProvenanceSource::EffectiveRoleFile,
                detail: Some("overrides lower-precedence role".to_string()),
            },
            AgentRoleTemplateFieldProvenance {
                field_name: "model".to_string(),
                source: AgentRoleTemplateFieldProvenanceSource::EffectiveRoleFile,
                detail: Some("overrides lower-precedence role".to_string()),
            },
            AgentRoleTemplateFieldProvenance {
                field_name: "name".to_string(),
                source: AgentRoleTemplateFieldProvenanceSource::RoleFileMetadata,
                detail: None,
            },
        ]
    );

    Ok(())
}

#[tokio::test]
async fn list_agent_role_templates_does_not_inherit_runtime_fields_from_shadowed_role_file()
-> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let repo_root = TempDir::new()?;
    let nested_cwd = repo_root.path().join("packages").join("app");
    std::fs::create_dir_all(repo_root.path().join(".git"))?;
    std::fs::create_dir_all(&nested_cwd)?;

    let workspace_key = repo_root.path().to_string_lossy().replace('\\', "\\\\");
    std::fs::write(
        codex_home.path().join(CONFIG_TOML_FILE),
        format!(
            r#"[projects."{workspace_key}"]
trust_level = "trusted"

[agents.researcher]
description = "Research role from config"
config_file = "./agents/researcher.toml"
"#
        ),
    )?;

    let home_agents_dir = codex_home.path().join("agents");
    std::fs::create_dir_all(&home_agents_dir)?;
    std::fs::write(
        home_agents_dir.join("researcher.toml"),
        r#"
developer_instructions = "Research carefully"
model = "gpt-5.2"
"#,
    )?;

    let standalone_agents_dir = repo_root.path().join(".codex").join("agents");
    std::fs::create_dir_all(&standalone_agents_dir)?;
    std::fs::write(
        standalone_agents_dir.join("researcher.toml"),
        r#"
name = "researcher"
developer_instructions = "Research from file"
"#,
    )?;

    let config = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .harness_overrides(crate::config::ConfigOverrides {
            cwd: Some(nested_cwd),
            ..Default::default()
        })
        .build()
        .await?;

    let entries = list_agent_role_templates(&config);
    let researcher = entries
        .iter()
        .find(|entry| entry.name == "researcher" && entry.source == AgentRoleTemplateSource::User)
        .expect("researcher role");

    assert!(
        researcher
            .field_sources
            .bounded_provenance
            .iter()
            .all(|provenance| provenance.field_name != "model")
    );

    Ok(())
}

#[tokio::test]
async fn list_agent_role_templates_reports_config_file_origin() {
    let (home, mut config) = test_config().await;
    let user_role_path = home.path().join("agents/reviewer.toml");
    let external_role_path = home.path().join("external/worker.toml");
    fs::create_dir_all(user_role_path.parent().expect("role path parent")).expect("create dir");
    fs::create_dir_all(external_role_path.parent().expect("role path parent")).expect("create dir");
    fs::write(
        &user_role_path,
        r#"name = "reviewer"
description = "Review code changes"
developer_instructions = "Review code changes and report concrete risks."
"#,
    )
    .expect("write user role");
    fs::write(
        &external_role_path,
        r#"name = "external-worker"
description = "Work from an explicit config file"
developer_instructions = "Work from an explicit config file."
"#,
    )
    .expect("write external role");
    config.agent_roles.insert(
        "reviewer".to_string(),
        AgentRoleConfig {
            description: Some("Review code changes".to_string()),
            config_file: Some(user_role_path),
            nickname_candidates: None,
            metadata_sources: Default::default(),
            runtime_config_sources: Default::default(),
        },
    );
    config.agent_roles.insert(
        "external-worker".to_string(),
        AgentRoleConfig {
            description: Some("Work from an explicit config file".to_string()),
            config_file: Some(external_role_path),
            nickname_candidates: None,
            metadata_sources: Default::default(),
            runtime_config_sources: Default::default(),
        },
    );

    let entries = list_agent_role_templates(&config);

    let reviewer = entries
        .iter()
        .find(|entry| entry.name == "reviewer")
        .expect("reviewer role");
    assert_eq!(
        reviewer.config_file_origin,
        Some(AgentRoleTemplateConfigFileOrigin::UserAgentsDir)
    );
    let external_worker = entries
        .iter()
        .find(|entry| entry.name == "external-worker")
        .expect("external worker role");
    assert_eq!(
        external_worker.config_file_origin,
        Some(AgentRoleTemplateConfigFileOrigin::ExternalConfigFile)
    );
    let built_in = entries
        .iter()
        .find(|entry| {
            entry.source == AgentRoleTemplateSource::BuiltIn && entry.config_file.is_some()
        })
        .expect("built-in role with config file");
    assert_eq!(
        built_in.config_file_origin,
        Some(AgentRoleTemplateConfigFileOrigin::BuiltInBundle)
    );
}

#[tokio::test]
async fn create_user_agent_role_template_writes_valid_discovered_role_file() {
    let (home, mut config) = test_config().await;

    let created = create_user_agent_role_template(&config, "Code Reviewer").expect("created");
    config
        .agent_roles
        .insert(created.name.clone(), created.config.clone());

    assert_eq!(created.name, "code-reviewer");
    assert_eq!(created.path, home.path().join("agents/code-reviewer.toml"));
    let contents = fs::read_to_string(&created.path).expect("read template");
    assert!(contents.contains("name = \"code-reviewer\""));
    assert!(contents.contains("developer_instructions"));
    assert!(contents.contains("# Optional tool boundary: add [tool_selection]"));
    assert!(!contents.contains("\n[tool_selection]\n"));
    assert!(resolve_role_config(&config, "code-reviewer").is_some());
}

#[tokio::test]
async fn create_user_agent_role_template_from_draft_writes_valid_native_toml() {
    let (home, mut config) = test_config().await;
    let draft = r#"name = "audit-reviewer"
description = "Review code changes before handoff."
nickname_candidates = ["Ada"]
developer_instructions = "Review the diff and report concrete risks."
model_provider = "deepseek"

[tool_selection]
allowed_tools = ["update_plan"]
"#;

    let created = create_user_agent_role_template_from_draft(&config, draft).expect("created");
    config
        .agent_roles
        .insert(created.name.clone(), created.config.clone());

    assert_eq!(created.name, "audit-reviewer");
    assert_eq!(created.path, home.path().join("agents/audit-reviewer.toml"));
    let contents = fs::read_to_string(&created.path).expect("read template");
    assert_eq!(contents, format!("{}\n", draft.trim()));
    assert!(contents.contains("[tool_selection]\nallowed_tools = [\"update_plan\"]"));
    assert_eq!(
        resolve_role_config(&config, "audit-reviewer")
            .expect("role")
            .description
            .as_deref(),
        Some("Review code changes before handoff.")
    );
}

#[tokio::test]
async fn starter_agent_role_template_draft_with_allowed_tools_writes_native_tool_selection() {
    let (home, config) = test_config().await;
    let draft = starter_agent_role_template_draft_with_allowed_tools(&[
        "update_plan".to_string(),
        "codex_app/lookup".to_string(),
    ]);

    assert!(draft.contains("[tool_selection]\nallowed_tools = ["));
    assert!(draft.contains("\"update_plan\""));
    assert!(draft.contains("\"codex_app/lookup\""));
    assert!(!draft.contains("# Optional tool boundary"));

    let created = create_user_agent_role_template_from_draft(&config, &draft).expect("created");

    assert_eq!(created.name, "new-role");
    assert_eq!(created.path, home.path().join("agents/new-role.toml"));
    let contents = fs::read_to_string(&created.path).expect("read template");
    assert!(contents.contains("[tool_selection]\nallowed_tools = ["));
    assert!(contents.contains("\"update_plan\""));
    assert!(contents.contains("\"codex_app/lookup\""));
}

#[tokio::test]
async fn agent_role_template_current_model_draft_writes_native_model_defaults() {
    let (home, mut config) = test_config().await;
    config.model = Some("deepseek/deepseek-v4-flash".to_string());
    config.model_provider_id = "deepseek".to_string();
    config.model_reasoning_effort = Some(ReasoningEffort::High);
    config.service_tier = Some("priority".to_string());

    let draft = starter_agent_role_template_draft_from_current_model(&config);

    assert!(draft.contains("model = \"deepseek/deepseek-v4-flash\""));
    assert!(draft.contains("model_provider = \"deepseek\""));
    assert!(draft.contains("model_reasoning_effort = \"high\""));
    assert!(draft.contains("service_tier = \"priority\""));
    assert!(draft.contains("# Optional tool boundary"));
    let created = create_user_agent_role_template_from_draft(&config, &draft).expect("created");

    assert_eq!(created.name, "new-role");
    assert_eq!(created.path, home.path().join("agents/new-role.toml"));
    let contents = fs::read_to_string(&created.path).expect("read template");
    assert_eq!(contents, format!("{}\n", draft.trim()));
    assert_eq!(
        created
            .config
            .runtime_config_sources
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec![
            "developer_instructions",
            "model",
            "model_provider",
            "model_reasoning_effort",
            "service_tier",
        ]
    );
}

#[tokio::test]
async fn agent_role_template_explicit_model_defaults_draft_writes_native_model_defaults() {
    let (home, config) = test_config().await;
    let draft =
        starter_agent_role_template_draft_with_model_defaults(&AgentRoleTemplateModelDefaults {
            model: Some("deepseek/deepseek-v4-flash".to_string()),
            model_provider: Some("deepseek".to_string()),
            model_reasoning_effort: Some("high".to_string()),
            service_tier: Some("priority".to_string()),
        });

    assert!(draft.contains("model = \"deepseek/deepseek-v4-flash\""));
    assert!(draft.contains("model_provider = \"deepseek\""));
    assert!(draft.contains("model_reasoning_effort = \"high\""));
    assert!(draft.contains("service_tier = \"priority\""));
    let created = create_user_agent_role_template_from_draft(&config, &draft).expect("created");

    assert_eq!(created.name, "new-role");
    assert_eq!(created.path, home.path().join("agents/new-role.toml"));
    assert_eq!(
        created
            .config
            .runtime_config_sources
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec![
            "developer_instructions",
            "model",
            "model_provider",
            "model_reasoning_effort",
            "service_tier",
        ]
    );
}

#[tokio::test]
async fn user_agent_role_template_update_draft_rewrites_native_model_defaults() {
    let (_home, mut config) = test_config().await;
    config.model = Some("deepseek/deepseek-v4-flash".to_string());
    config.model_provider_id = "deepseek".to_string();
    config.model_reasoning_effort = Some(ReasoningEffort::High);
    config.service_tier = Some("priority".to_string());
    let initial = r#"name = "audit-reviewer"
description = "Review code changes before handoff."
nickname_candidates = ["Ada"]
developer_instructions = "Review the diff and report concrete risks."
model = "gpt-5.3-codex"
model_provider = "openai"
model_reasoning_effort = "medium"

[tool_selection]
allowed_tools = ["update_plan"]
denied_tools = ["apply_patch"]

[subagent_action_policy]
allowed_actions = ["agent_message_send"]
"#;
    let created = create_user_agent_role_template_from_draft(&config, initial).expect("created");

    let draft =
        user_agent_role_template_draft_from_current_model(&config, "audit-reviewer", &created.path)
            .expect("draft");
    assert!(draft.contains("model = \"deepseek/deepseek-v4-flash\""));
    assert!(draft.contains("model_provider = \"deepseek\""));
    assert!(draft.contains("model_reasoning_effort = \"high\""));
    assert!(draft.contains("service_tier = \"priority\""));
    assert!(draft.contains("[tool_selection]\nallowed_tools = ["));
    assert!(draft.contains("\"update_plan\""));
    assert!(draft.contains("denied_tools = ["));
    assert!(draft.contains("\"apply_patch\""));
    assert!(draft.contains("[subagent_action_policy]\nallowed_actions = [\"agent_message_send\"]"));

    let updated = update_user_agent_role_template_from_draft(
        &config,
        "audit-reviewer",
        &created.path,
        &draft,
    )
    .expect("updated");

    assert_eq!(updated.name, "audit-reviewer");
    let contents = fs::read_to_string(&created.path).expect("read template");
    assert!(contents.contains("model = \"deepseek/deepseek-v4-flash\""));
    assert!(contents.contains("model_provider = \"deepseek\""));
    assert!(contents.contains("model_reasoning_effort = \"high\""));
    assert!(contents.contains("service_tier = \"priority\""));
    assert!(contents.contains("[tool_selection]\nallowed_tools = ["));
    assert!(
        contents.contains("[subagent_action_policy]\nallowed_actions = [\"agent_message_send\"]")
    );
}

#[tokio::test]
async fn user_agent_role_template_update_draft_rewrites_explicit_model_defaults() {
    let (_home, config) = test_config().await;
    let initial = r#"name = "audit-reviewer"
description = "Review code changes before handoff."
nickname_candidates = ["Ada"]
developer_instructions = "Review the diff and report concrete risks."
model = "gpt-5.3-codex"
model_provider = "openai"
model_reasoning_effort = "medium"

[tool_selection]
allowed_tools = ["update_plan"]
denied_tools = ["apply_patch"]

[subagent_action_policy]
allowed_actions = ["agent_message_send"]
"#;
    let created = create_user_agent_role_template_from_draft(&config, initial).expect("created");

    let draft = user_agent_role_template_draft_with_model_defaults(
        &config,
        "audit-reviewer",
        &created.path,
        &AgentRoleTemplateModelDefaults {
            model: Some("deepseek/deepseek-v4-flash".to_string()),
            model_provider: Some("deepseek".to_string()),
            model_reasoning_effort: Some("high".to_string()),
            service_tier: Some("priority".to_string()),
        },
    )
    .expect("draft");

    assert!(draft.contains("model = \"deepseek/deepseek-v4-flash\""));
    assert!(draft.contains("model_provider = \"deepseek\""));
    assert!(draft.contains("model_reasoning_effort = \"high\""));
    assert!(draft.contains("service_tier = \"priority\""));
    assert!(draft.contains("[tool_selection]\nallowed_tools = ["));
    assert!(draft.contains("\"update_plan\""));
    assert!(draft.contains("denied_tools = ["));
    assert!(draft.contains("\"apply_patch\""));
    assert!(draft.contains("[subagent_action_policy]\nallowed_actions = [\"agent_message_send\"]"));

    let updated = update_user_agent_role_template_from_draft(
        &config,
        "audit-reviewer",
        &created.path,
        &draft,
    )
    .expect("updated");

    assert_eq!(updated.name, "audit-reviewer");
    let contents = fs::read_to_string(&created.path).expect("read template");
    assert!(contents.contains("model = \"deepseek/deepseek-v4-flash\""));
    assert!(contents.contains("model_provider = \"deepseek\""));
    assert!(contents.contains("model_reasoning_effort = \"high\""));
    assert!(contents.contains("service_tier = \"priority\""));
    assert!(contents.contains("[tool_selection]\nallowed_tools = ["));
    assert!(
        contents.contains("[subagent_action_policy]\nallowed_actions = [\"agent_message_send\"]")
    );
}

#[tokio::test]
async fn user_agent_role_template_update_draft_rewrites_native_model_provider_defaults() {
    let (_home, mut config) = test_config().await;
    config.model = Some("deepseek/deepseek-v4-flash".to_string());
    config.model_provider_id = "deepseek".to_string();
    config.model_reasoning_effort = Some(ReasoningEffort::High);
    config.service_tier = Some("priority".to_string());
    let initial = r#"name = "audit-reviewer"
description = "Review code changes before handoff."
nickname_candidates = ["Ada"]
developer_instructions = "Review the diff and report concrete risks."
model = "gpt-5.3-codex"
model_provider = "openai"
model_reasoning_effort = "medium"
service_tier = "standard"

[tool_selection]
allowed_tools = ["update_plan"]
denied_tools = ["apply_patch"]

[subagent_action_policy]
allowed_actions = ["agent_message_send"]
"#;
    let created = create_user_agent_role_template_from_draft(&config, initial).expect("created");

    let draft = user_agent_role_template_draft_from_current_model_provider(
        &config,
        "audit-reviewer",
        &created.path,
    )
    .expect("draft");
    assert!(draft.contains("model = \"deepseek/deepseek-v4-flash\""));
    assert!(draft.contains("model_provider = \"deepseek\""));
    assert!(draft.contains("model_reasoning_effort = \"medium\""));
    assert!(draft.contains("service_tier = \"standard\""));
    assert!(draft.contains("[tool_selection]\nallowed_tools = ["));
    assert!(draft.contains("\"update_plan\""));
    assert!(draft.contains("denied_tools = ["));
    assert!(draft.contains("\"apply_patch\""));
    assert!(draft.contains("[subagent_action_policy]\nallowed_actions = [\"agent_message_send\"]"));

    let updated = update_user_agent_role_template_from_draft(
        &config,
        "audit-reviewer",
        &created.path,
        &draft,
    )
    .expect("updated");

    assert_eq!(updated.name, "audit-reviewer");
    let contents = fs::read_to_string(&created.path).expect("read template");
    assert!(contents.contains("model = \"deepseek/deepseek-v4-flash\""));
    assert!(contents.contains("model_provider = \"deepseek\""));
    assert!(contents.contains("model_reasoning_effort = \"medium\""));
    assert!(contents.contains("service_tier = \"standard\""));
    assert!(contents.contains("[tool_selection]\nallowed_tools = ["));
    assert!(
        contents.contains("[subagent_action_policy]\nallowed_actions = [\"agent_message_send\"]")
    );
}

#[tokio::test]
async fn user_agent_role_template_update_draft_rewrites_native_reasoning_defaults() {
    let (_home, mut config) = test_config().await;
    config.model = Some("deepseek/deepseek-v4-flash".to_string());
    config.model_provider_id = "deepseek".to_string();
    config.model_reasoning_effort = Some(ReasoningEffort::High);
    config.service_tier = Some("priority".to_string());
    let initial = r#"name = "audit-reviewer"
description = "Review code changes before handoff."
nickname_candidates = ["Ada"]
developer_instructions = "Review the diff and report concrete risks."
model = "gpt-5.3-codex"
model_provider = "openai"
model_reasoning_effort = "medium"

[tool_selection]
allowed_tools = ["update_plan"]
denied_tools = ["apply_patch"]

[subagent_action_policy]
allowed_actions = ["agent_message_send"]
"#;
    let created = create_user_agent_role_template_from_draft(&config, initial).expect("created");

    let draft = user_agent_role_template_draft_from_current_reasoning(
        &config,
        "audit-reviewer",
        &created.path,
    )
    .expect("draft");
    assert!(draft.contains("model = \"gpt-5.3-codex\""));
    assert!(draft.contains("model_provider = \"openai\""));
    assert!(draft.contains("model_reasoning_effort = \"high\""));
    assert!(draft.contains("service_tier = \"priority\""));
    assert!(draft.contains("[tool_selection]\nallowed_tools = ["));
    assert!(draft.contains("\"update_plan\""));
    assert!(draft.contains("denied_tools = ["));
    assert!(draft.contains("\"apply_patch\""));
    assert!(draft.contains("[subagent_action_policy]\nallowed_actions = [\"agent_message_send\"]"));

    let updated = update_user_agent_role_template_from_draft(
        &config,
        "audit-reviewer",
        &created.path,
        &draft,
    )
    .expect("updated");

    assert_eq!(updated.name, "audit-reviewer");
    let contents = fs::read_to_string(&created.path).expect("read template");
    assert!(contents.contains("model = \"gpt-5.3-codex\""));
    assert!(contents.contains("model_provider = \"openai\""));
    assert!(contents.contains("model_reasoning_effort = \"high\""));
    assert!(contents.contains("service_tier = \"priority\""));
    assert!(contents.contains("[tool_selection]\nallowed_tools = ["));
    assert!(
        contents.contains("[subagent_action_policy]\nallowed_actions = [\"agent_message_send\"]")
    );
}

#[tokio::test]
async fn user_agent_role_template_update_draft_clears_native_model_defaults() {
    let (_home, config) = test_config().await;
    let initial = r#"name = "audit-reviewer"
description = "Review code changes before handoff."
nickname_candidates = ["Ada"]
developer_instructions = "Review the diff and report concrete risks."
model = "deepseek/deepseek-v4-flash"
model_provider = "deepseek"
model_reasoning_effort = "high"
service_tier = "priority"

[tool_selection]
allowed_tools = ["update_plan"]
denied_tools = ["apply_patch"]

[subagent_action_policy]
allowed_actions = ["agent_message_send"]
"#;
    let created = create_user_agent_role_template_from_draft(&config, initial).expect("created");

    let draft = user_agent_role_template_draft_without_model_defaults(
        &config,
        "audit-reviewer",
        &created.path,
    )
    .expect("draft");
    assert!(!draft.contains("model = "));
    assert!(!draft.contains("model_provider = "));
    assert!(!draft.contains("model_reasoning_effort = "));
    assert!(!draft.contains("service_tier = "));
    assert!(draft.contains("[tool_selection]\nallowed_tools = ["));
    assert!(draft.contains("\"update_plan\""));
    assert!(draft.contains("denied_tools = ["));
    assert!(draft.contains("\"apply_patch\""));
    assert!(draft.contains("[subagent_action_policy]\nallowed_actions = [\"agent_message_send\"]"));

    let updated = update_user_agent_role_template_from_draft(
        &config,
        "audit-reviewer",
        &created.path,
        &draft,
    )
    .expect("updated");

    assert_eq!(updated.name, "audit-reviewer");
    let contents = fs::read_to_string(&created.path).expect("read template");
    assert!(!contents.contains("model = "));
    assert!(!contents.contains("model_provider = "));
    assert!(!contents.contains("model_reasoning_effort = "));
    assert!(!contents.contains("service_tier = "));
    assert!(contents.contains("[tool_selection]\nallowed_tools = ["));
    assert!(
        contents.contains("[subagent_action_policy]\nallowed_actions = [\"agent_message_send\"]")
    );
}

#[tokio::test]
async fn user_agent_role_template_draft_from_file_preserves_raw_native_toml() {
    let (_home, config) = test_config().await;
    let initial = r#"# reviewer role
name = "audit-reviewer"
description = "Review code changes before handoff."
nickname_candidates = ["Ada"]
developer_instructions = "Review the diff and report concrete risks."

# Keep this comment in the inline editor.
[tool_selection]
allowed_tools = ["update_plan"]
"#;
    let created = create_user_agent_role_template_from_draft(&config, initial).expect("created");

    let draft = user_agent_role_template_draft_from_file(&config, "audit-reviewer", &created.path)
        .expect("draft");

    assert_eq!(draft, format!("{}\n", initial.trim()));
}

#[tokio::test]
async fn user_agent_role_template_update_draft_rewrites_native_tool_selection() {
    let (_home, config) = test_config().await;
    let initial = r#"name = "audit-reviewer"
description = "Review code changes before handoff."
nickname_candidates = ["Ada"]
developer_instructions = "Review the diff and report concrete risks."

[tool_selection]
allowed_tools = ["update_plan"]
denied_tools = ["apply_patch"]
"#;
    let created = create_user_agent_role_template_from_draft(&config, initial).expect("created");

    let draft = user_agent_role_template_draft_with_allowed_tools(
        &config,
        "audit-reviewer",
        &created.path,
        &["tool_search".to_string(), "update_plan".to_string()],
    )
    .expect("draft");
    assert!(draft.contains("[tool_selection]\nallowed_tools = ["));
    assert!(draft.contains("\"tool_search\""));
    assert!(draft.contains("\"update_plan\""));

    let updated = update_user_agent_role_template_from_draft(
        &config,
        "audit-reviewer",
        &created.path,
        &draft,
    )
    .expect("updated");

    assert_eq!(updated.name, "audit-reviewer");
    let contents = fs::read_to_string(&created.path).expect("read template");
    assert!(contents.contains("[tool_selection]\nallowed_tools = ["));
    assert!(contents.contains("\"tool_search\""));
    assert!(contents.contains("\"update_plan\""));
    assert!(contents.contains("denied_tools = ["));
    assert!(contents.contains("\"apply_patch\""));
}

#[tokio::test]
async fn user_agent_role_template_update_draft_rewrites_native_denied_tools() {
    let (_home, config) = test_config().await;
    let initial = r#"name = "audit-reviewer"
description = "Review code changes before handoff."
nickname_candidates = ["Ada"]
developer_instructions = "Review the diff and report concrete risks."

[tool_selection]
allowed_tools = ["update_plan"]
denied_tools = ["apply_patch"]
"#;
    let created = create_user_agent_role_template_from_draft(&config, initial).expect("created");

    let draft = user_agent_role_template_draft_with_denied_tools(
        &config,
        "audit-reviewer",
        &created.path,
        &["tool_search".to_string(), "apply_patch".to_string()],
    )
    .expect("draft");
    assert!(draft.contains("[tool_selection]\nallowed_tools = ["));
    assert!(draft.contains("\"update_plan\""));
    assert!(draft.contains("denied_tools = ["));
    assert!(draft.contains("\"tool_search\""));
    assert!(draft.contains("\"apply_patch\""));

    let updated = update_user_agent_role_template_from_draft(
        &config,
        "audit-reviewer",
        &created.path,
        &draft,
    )
    .expect("updated");

    assert_eq!(updated.name, "audit-reviewer");
    let contents = fs::read_to_string(&created.path).expect("read template");
    assert!(contents.contains("[tool_selection]\nallowed_tools = ["));
    assert!(contents.contains("\"update_plan\""));
    assert!(contents.contains("denied_tools = ["));
    assert!(contents.contains("\"tool_search\""));
    assert!(contents.contains("\"apply_patch\""));
}

#[tokio::test]
async fn user_agent_role_template_denied_tools_draft_preserves_empty_allowlist() {
    let (_home, config) = test_config().await;
    let initial = r#"name = "audit-reviewer"
description = "Review code changes before handoff."
nickname_candidates = ["Ada"]
developer_instructions = "Review the diff and report concrete risks."

[tool_selection]
allowed_tools = []
denied_tools = ["apply_patch"]
"#;
    let created = create_user_agent_role_template_from_draft(&config, initial).expect("created");

    let draft = user_agent_role_template_draft_with_denied_tools(
        &config,
        "audit-reviewer",
        &created.path,
        &["tool_search".to_string(), "apply_patch".to_string()],
    )
    .expect("draft");
    assert!(draft.contains("[tool_selection]\nallowed_tools = []"));
    assert!(draft.contains("denied_tools = ["));
    assert!(draft.contains("\"tool_search\""));
    assert!(draft.contains("\"apply_patch\""));

    let updated = update_user_agent_role_template_from_draft(
        &config,
        "audit-reviewer",
        &created.path,
        &draft,
    )
    .expect("updated");

    assert_eq!(updated.name, "audit-reviewer");
    let contents = fs::read_to_string(&created.path).expect("read template");
    assert!(contents.contains("[tool_selection]\nallowed_tools = []"));
    assert!(contents.contains("denied_tools = ["));
    assert!(contents.contains("\"tool_search\""));
    assert!(contents.contains("\"apply_patch\""));
}

#[tokio::test]
async fn user_agent_role_template_update_tool_selection_rewrites_allowlist_and_denylist() {
    let (_home, config) = test_config().await;
    let initial = r#"name = "audit-reviewer"
description = "Review code changes before handoff."
nickname_candidates = ["Ada"]
developer_instructions = "Review the diff and report concrete risks."

[tool_selection]
allowed_tools = ["update_plan"]
"#;
    let created = create_user_agent_role_template_from_draft(&config, initial).expect("created");

    let updated = update_user_agent_role_template_tool_selection(
        &config,
        "audit-reviewer",
        &created.path,
        Some(&["tool_search".to_string(), "update_plan".to_string()]),
        Some(&["apply_patch".to_string()]),
    )
    .expect("updated");

    assert_eq!(updated.name, "audit-reviewer");
    assert_eq!(
        updated
            .config
            .runtime_config_sources
            .keys()
            .collect::<Vec<_>>(),
        vec![
            &"developer_instructions".to_string(),
            &"tool_selection.allowed_tools".to_string(),
            &"tool_selection.denied_tools".to_string()
        ]
    );
    let contents = fs::read_to_string(&created.path).expect("read template");
    assert!(contents.contains("[tool_selection]\nallowed_tools = ["));
    assert!(contents.contains("\"tool_search\""));
    assert!(contents.contains("\"update_plan\""));
    assert!(contents.contains("denied_tools = ["));
    assert!(contents.contains("\"apply_patch\""));
}

#[tokio::test]
async fn user_agent_role_template_update_tool_selection_removes_section_for_null_policy() {
    let (_home, config) = test_config().await;
    let initial = r#"name = "audit-reviewer"
description = "Review code changes before handoff."
nickname_candidates = ["Ada"]
developer_instructions = "Review the diff and report concrete risks."

[tool_selection]
allowed_tools = ["update_plan"]
denied_tools = ["apply_patch"]
"#;
    let created = create_user_agent_role_template_from_draft(&config, initial).expect("created");

    let updated = update_user_agent_role_template_tool_selection(
        &config,
        "audit-reviewer",
        &created.path,
        None,
        None,
    )
    .expect("updated");

    assert_eq!(updated.name, "audit-reviewer");
    let contents = fs::read_to_string(&created.path).expect("read template");
    assert!(!contents.contains("[tool_selection]"));
    assert!(!contents.contains("allowed_tools"));
    assert!(!contents.contains("denied_tools"));
}

#[tokio::test]
async fn user_agent_role_template_update_action_policy_rewrites_allowlist_and_denylist() {
    let (_home, config) = test_config().await;
    let initial = r#"name = "audit-reviewer"
description = "Review code changes before handoff."
nickname_candidates = ["Ada"]
developer_instructions = "Review the diff and report concrete risks."

[subagent_action_policy]
allowed_actions = ["agent_message_send"]
"#;
    let created = create_user_agent_role_template_from_draft(&config, initial).expect("created");

    let updated = update_user_agent_role_template_action_policy(
        &config,
        "audit-reviewer",
        &created.path,
        Some(&[
            SubAgentActionPolicyAction::AgentMessageSend,
            SubAgentActionPolicyAction::AgentRetry,
        ]),
        Some(&[SubAgentActionPolicyAction::AgentClose]),
    )
    .expect("updated");

    assert_eq!(updated.name, "audit-reviewer");
    assert_eq!(
        updated
            .config
            .runtime_config_sources
            .keys()
            .collect::<Vec<_>>(),
        vec![
            &"developer_instructions".to_string(),
            &"subagent_action_policy.allowed_actions".to_string(),
            &"subagent_action_policy.denied_actions".to_string()
        ]
    );
    let contents = fs::read_to_string(&created.path).expect("read template");
    assert!(contents.contains("[subagent_action_policy]\nallowed_actions = ["));
    assert!(contents.contains("\"agent_message_send\""));
    assert!(contents.contains("\"agent_retry\""));
    assert!(contents.contains("denied_actions = ["));
    assert!(contents.contains("\"agent_close\""));
}

#[tokio::test]
async fn user_agent_role_template_update_action_policy_removes_section_for_null_policy() {
    let (_home, config) = test_config().await;
    let initial = r#"name = "audit-reviewer"
description = "Review code changes before handoff."
nickname_candidates = ["Ada"]
developer_instructions = "Review the diff and report concrete risks."

[subagent_action_policy]
allowed_actions = ["agent_message_send"]
denied_actions = ["agent_close"]
"#;
    let created = create_user_agent_role_template_from_draft(&config, initial).expect("created");

    let updated = update_user_agent_role_template_action_policy(
        &config,
        "audit-reviewer",
        &created.path,
        None,
        None,
    )
    .expect("updated");

    assert_eq!(updated.name, "audit-reviewer");
    let contents = fs::read_to_string(&created.path).expect("read template");
    assert!(!contents.contains("[subagent_action_policy]"));
    assert!(!contents.contains("allowed_actions"));
    assert!(!contents.contains("denied_actions"));
}

#[tokio::test]
async fn create_user_agent_role_template_from_draft_rejects_non_slug_name() {
    let (_home, config) = test_config().await;
    let draft = r#"name = "Code Reviewer"
description = "Review code changes before handoff."
developer_instructions = "Review the diff and report concrete risks."
"#;

    let err = create_user_agent_role_template_from_draft(&config, draft)
        .expect_err("non-normalized role names are rejected");

    assert_eq!(
        err,
        AgentRoleTemplateCreateError::InvalidTemplate(
            "agent role name `Code Reviewer` must be written as normalized slug `code-reviewer`"
                .to_string()
        )
    );
}

#[tokio::test]
async fn create_user_agent_role_template_from_draft_rejects_invalid_tool_selection() {
    let (_home, config) = test_config().await;
    let draft = r#"name = "reviewer"
description = "Review code changes before handoff."
developer_instructions = "Review the diff and report concrete risks."

[tool_selection]
allowed_tools = [""]
"#;

    let err = create_user_agent_role_template_from_draft(&config, draft)
        .expect_err("invalid tool selection should be rejected");

    assert!(
        matches!(err, AgentRoleTemplateCreateError::InvalidTemplate(message) if message.contains("tool_selection.allowed_tools[0] cannot be blank"))
    );
}

#[tokio::test]
async fn create_user_agent_role_template_rejects_existing_role() {
    let (_home, config) = test_config().await;

    let err = create_user_agent_role_template(&config, "explorer")
        .expect_err("built-in role name should be rejected");

    assert_eq!(
        err,
        AgentRoleTemplateCreateError::AlreadyExists("explorer".to_string())
    );
}

#[test]
fn normalize_agent_role_template_name_slugifies_user_input() {
    assert_eq!(
        normalize_agent_role_template_name("  Code Reviewer_2  "),
        Ok("code-reviewer-2".to_string())
    );
    assert_eq!(
        normalize_agent_role_template_name("!!!"),
        Err(AgentRoleTemplateCreateError::InvalidName)
    );
}
