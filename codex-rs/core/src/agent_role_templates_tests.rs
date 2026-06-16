use super::*;
use crate::config::ConfigBuilder;
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
        },
    );

    let entries = list_agent_role_templates(&config);

    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>(),
        vec!["reviewer", "default", "explorer", "worker"]
    );
    assert_eq!(entries[0].source, AgentRoleTemplateSource::User);
    assert_eq!(entries[1].source, AgentRoleTemplateSource::BuiltIn);
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
    assert!(resolve_role_config(&config, "code-reviewer").is_some());
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
