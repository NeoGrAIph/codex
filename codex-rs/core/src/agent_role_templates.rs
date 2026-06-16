//! Public projection helpers for sub-agent role templates.
//!
//! This module exposes the existing role source of truth to UI clients without
//! duplicating built-in role names or adding a parallel profile registry.

use std::collections::BTreeSet;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

use serde::Serialize;
use thiserror::Error;
use toml::Value as TomlValue;

use crate::agent::role::DEFAULT_ROLE_NAME;
use crate::agent::role::built_in_role_config_file_contents;
use crate::agent::role::built_in_role_configs;
use crate::agent::role::resolve_role_config;
use crate::config::AgentRoleConfig;
use crate::config::Config;
use crate::config::agent_roles::parse_agent_role_file_contents;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRoleTemplateSource {
    User,
    BuiltIn,
}

impl AgentRoleTemplateSource {
    pub fn label(self) -> &'static str {
        match self {
            AgentRoleTemplateSource::User => "user",
            AgentRoleTemplateSource::BuiltIn => "built-in",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgentRoleTemplateLocks {
    pub model: Option<String>,
    pub reasoning_effort: Option<String>,
    pub service_tier: Option<String>,
    pub has_developer_instructions: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRoleTemplateEntry {
    pub name: String,
    pub description: Option<String>,
    pub source: AgentRoleTemplateSource,
    pub config_file: Option<PathBuf>,
    pub nickname_candidates: Vec<String>,
    pub locks: AgentRoleTemplateLocks,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedAgentRoleTemplate {
    pub name: String,
    pub path: PathBuf,
    pub config: AgentRoleConfig,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AgentRoleTemplateCreateError {
    #[error("agent role name cannot be empty")]
    EmptyName,
    #[error("agent role name must contain ASCII letters or numbers")]
    InvalidName,
    #[error("agent role `{0}` already exists")]
    AlreadyExists(String),
    #[error("agent role template file already exists: {0}")]
    FileExists(PathBuf),
    #[error("failed to write agent role template at {path}: {source_message}")]
    Write {
        path: PathBuf,
        source_message: String,
    },
    #[error("generated agent role template is invalid: {0}")]
    InvalidTemplate(String),
}

pub fn list_agent_role_templates(config: &Config) -> Vec<AgentRoleTemplateEntry> {
    let mut seen = BTreeSet::new();
    let mut entries = Vec::new();

    for (name, role_config) in &config.agent_roles {
        if seen.insert(name.clone()) {
            entries.push(entry_from_config(
                name,
                role_config,
                AgentRoleTemplateSource::User,
            ));
        }
    }

    for (name, role_config) in built_in_role_configs() {
        if seen.insert(name.clone()) {
            entries.push(entry_from_config(
                name,
                role_config,
                AgentRoleTemplateSource::BuiltIn,
            ));
        }
    }

    entries
}

pub fn create_user_agent_role_template(
    config: &Config,
    raw_name: &str,
) -> Result<CreatedAgentRoleTemplate, AgentRoleTemplateCreateError> {
    let name = normalize_agent_role_template_name(raw_name)?;
    if resolve_role_config(config, &name).is_some() {
        return Err(AgentRoleTemplateCreateError::AlreadyExists(name));
    }

    let agents_dir = config.codex_home.join("agents");
    let path = agents_dir.join(format!("{name}.toml"));
    let agents_dir_path = agents_dir.to_path_buf();
    let path_buf = path.to_path_buf();
    if path.exists() {
        return Err(AgentRoleTemplateCreateError::FileExists(path_buf));
    }

    let contents = starter_template_contents(&name);
    parse_agent_role_file_contents(&contents, &path, &agents_dir, Some(&name))
        .map_err(|err| AgentRoleTemplateCreateError::InvalidTemplate(err.to_string()))?;

    fs::create_dir_all(&agents_dir).map_err(|err| AgentRoleTemplateCreateError::Write {
        path: agents_dir_path,
        source_message: err.to_string(),
    })?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|err| AgentRoleTemplateCreateError::Write {
            path: path_buf.clone(),
            source_message: err.to_string(),
        })?;
    file.write_all(contents.as_bytes())
        .map_err(|err| AgentRoleTemplateCreateError::Write {
            path: path_buf.clone(),
            source_message: err.to_string(),
        })?;

    let config = AgentRoleConfig {
        description: Some(starter_description(&name)),
        config_file: Some(path_buf.clone()),
        nickname_candidates: Some(vec![starter_nickname(&name)]),
    };

    Ok(CreatedAgentRoleTemplate {
        name,
        path: path_buf,
        config,
    })
}

pub fn normalize_agent_role_template_name(
    raw_name: &str,
) -> Result<String, AgentRoleTemplateCreateError> {
    let trimmed = raw_name.trim();
    if trimmed.is_empty() {
        return Err(AgentRoleTemplateCreateError::EmptyName);
    }

    let mut normalized = String::new();
    let mut previous_separator = false;
    for ch in trimmed.chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch.to_ascii_lowercase());
            previous_separator = false;
        } else if matches!(ch, '-' | '_' | ' ') && !normalized.is_empty() && !previous_separator {
            normalized.push('-');
            previous_separator = true;
        }
    }

    while normalized.ends_with('-') {
        normalized.pop();
    }

    if normalized.is_empty() {
        return Err(AgentRoleTemplateCreateError::InvalidName);
    }

    Ok(normalized)
}

fn entry_from_config(
    name: &str,
    role_config: &AgentRoleConfig,
    source: AgentRoleTemplateSource,
) -> AgentRoleTemplateEntry {
    AgentRoleTemplateEntry {
        name: name.to_string(),
        description: role_config.description.clone(),
        source,
        config_file: role_config.config_file.clone(),
        nickname_candidates: role_config.nickname_candidates.clone().unwrap_or_default(),
        locks: role_locks(role_config, source),
    }
}

fn role_locks(
    role_config: &AgentRoleConfig,
    source: AgentRoleTemplateSource,
) -> AgentRoleTemplateLocks {
    let Some(config_file) = role_config.config_file.as_ref() else {
        return AgentRoleTemplateLocks::default();
    };

    let contents = match source {
        AgentRoleTemplateSource::BuiltIn => {
            built_in_role_config_file_contents(config_file).map(str::to_owned)
        }
        AgentRoleTemplateSource::User => fs::read_to_string(config_file).ok(),
    };

    contents
        .as_deref()
        .and_then(|contents| toml::from_str::<TomlValue>(contents).ok())
        .map(locks_from_toml)
        .unwrap_or_default()
}

fn locks_from_toml(role_toml: TomlValue) -> AgentRoleTemplateLocks {
    AgentRoleTemplateLocks {
        model: role_toml
            .get("model")
            .and_then(TomlValue::as_str)
            .map(str::to_string),
        reasoning_effort: role_toml
            .get("model_reasoning_effort")
            .and_then(TomlValue::as_str)
            .map(str::to_string),
        service_tier: role_toml
            .get("service_tier")
            .and_then(TomlValue::as_str)
            .map(str::to_string),
        has_developer_instructions: role_toml
            .get("developer_instructions")
            .and_then(TomlValue::as_str)
            .is_some_and(|value| !value.trim().is_empty()),
    }
}

#[derive(Serialize)]
struct StarterTemplate<'a> {
    name: &'a str,
    description: String,
    nickname_candidates: Vec<String>,
    developer_instructions: String,
}

fn starter_template_contents(name: &str) -> String {
    let template = StarterTemplate {
        name,
        description: starter_description(name),
        nickname_candidates: vec![starter_nickname(name)],
        developer_instructions: format!(
            "You are a sub-agent running with the `{name}` role.\n\nReplace this starter template with specific responsibilities, constraints, and completion criteria before using it for production delegation."
        ),
    };
    let mut contents = toml::to_string_pretty(&template).expect("starter template serializes");
    contents.push('\n');
    contents
}

fn starter_description(name: &str) -> String {
    if name == DEFAULT_ROLE_NAME {
        "Custom default sub-agent role template.".to_string()
    } else {
        format!("Custom sub-agent role template for `{name}`.")
    }
}

fn starter_nickname(name: &str) -> String {
    name.split('-')
        .find(|part| !part.is_empty())
        .unwrap_or("agent")
        .to_string()
}

#[cfg(test)]
#[path = "agent_role_templates_tests.rs"]
mod tests;
