//! Public projection helpers for sub-agent role templates.
//!
//! This module exposes the existing role source of truth to UI clients without
//! duplicating built-in role names or adding a parallel profile registry.

use std::collections::BTreeSet;
use std::fs;
use std::fs::OpenOptions;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use serde::Serialize;
use thiserror::Error;
use toml::Value as TomlValue;

use crate::agent::role::DEFAULT_ROLE_NAME;
use crate::agent::role::built_in_role_config_file_contents;
use crate::agent::role::built_in_role_configs;
use crate::agent::role::resolve_role_config;
use crate::config::AgentRoleConfig;
use crate::config::AgentRoleConfigFieldSource;
use crate::config::AgentRoleConfigFieldSourceKind;
use crate::config::AgentRoleConfigRuntimeFieldSource;
use crate::config::Config;
use crate::config::agent_roles::agent_role_file_field_names;
use crate::config::agent_roles::agent_role_runtime_config_sources;
use crate::config::agent_roles::parse_agent_role_file_contents;
use codex_config::CONFIG_TOML_FILE;
use codex_config::format_config_layer_source;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRoleTemplateConfigFileOrigin {
    UserAgentsDir,
    ExternalConfigFile,
    BuiltInBundle,
}

impl AgentRoleTemplateConfigFileOrigin {
    pub fn label(self) -> &'static str {
        match self {
            AgentRoleTemplateConfigFileOrigin::UserAgentsDir => "$CODEX_HOME/agents",
            AgentRoleTemplateConfigFileOrigin::ExternalConfigFile => "external config_file",
            AgentRoleTemplateConfigFileOrigin::BuiltInBundle => "built-in bundle",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRoleTemplateAvailability {
    NewSessionsOnly,
    BuiltIn,
    ShadowedBuiltIn,
}

impl AgentRoleTemplateAvailability {
    pub fn label(self) -> &'static str {
        match self {
            AgentRoleTemplateAvailability::NewSessionsOnly => {
                "loaded threads use this TOML file after native config reload or restart; the TUI wizard triggers reload after successful create"
            }
            AgentRoleTemplateAvailability::BuiltIn => "built in, no user file required",
            AgentRoleTemplateAvailability::ShadowedBuiltIn => {
                "built-in role is currently not selected by agent_type"
            }
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgentRoleTemplateLocks {
    pub model: Option<String>,
    pub model_provider: Option<String>,
    pub reasoning_effort: Option<String>,
    pub service_tier: Option<String>,
    pub allowed_tool_names: Vec<String>,
    pub approval_policy: Option<String>,
    pub sandbox_mode: Option<String>,
    pub default_permissions: Option<String>,
    pub has_sandbox_workspace_write: bool,
    pub permission_profile_count: usize,
    pub mcp_server_count: usize,
    pub hook_handler_count: usize,
    pub skill_config_count: usize,
    pub app_config_count: usize,
    pub has_apps_default_config: bool,
    pub has_developer_instructions: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgentRoleTemplateFieldSources {
    pub declaration_fields: Vec<String>,
    pub declaration_origins: Vec<String>,
    pub config_file_fields: Vec<String>,
    pub bounded_provenance: Vec<AgentRoleTemplateFieldProvenance>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRoleTemplateFieldProvenance {
    pub field_name: String,
    pub source: AgentRoleTemplateFieldProvenanceSource,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRoleTemplateFieldProvenanceSource {
    ConfigLayer,
    RoleFileMetadata,
    EffectiveRoleFile,
    DiscoveredRoleFile,
    BuiltInBundle,
    Unknown,
}

impl AgentRoleTemplateFieldProvenanceSource {
    pub fn label(self) -> &'static str {
        match self {
            AgentRoleTemplateFieldProvenanceSource::ConfigLayer => "config layer",
            AgentRoleTemplateFieldProvenanceSource::RoleFileMetadata => "role-file metadata",
            AgentRoleTemplateFieldProvenanceSource::EffectiveRoleFile => "effective role file",
            AgentRoleTemplateFieldProvenanceSource::DiscoveredRoleFile => {
                "$CODEX_HOME/agents discovery"
            }
            AgentRoleTemplateFieldProvenanceSource::BuiltInBundle => "built-in bundle",
            AgentRoleTemplateFieldProvenanceSource::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRoleTemplateEntry {
    pub name: String,
    pub description: Option<String>,
    pub source: AgentRoleTemplateSource,
    pub is_shadowed: bool,
    pub shadows_built_in: bool,
    pub availability: AgentRoleTemplateAvailability,
    pub config_file: Option<PathBuf>,
    pub config_file_origin: Option<AgentRoleTemplateConfigFileOrigin>,
    pub nickname_candidates: Vec<String>,
    pub locks: AgentRoleTemplateLocks,
    pub field_sources: AgentRoleTemplateFieldSources,
    pub validation_error: Option<String>,
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
    #[error("agent role template file does not exist: {0}")]
    FileMissing(PathBuf),
    #[error("failed to write agent role template at {path}: {source_message}")]
    Write {
        path: PathBuf,
        source_message: String,
    },
    #[error("generated agent role template is invalid: {0}")]
    InvalidTemplate(String),
}

pub fn list_agent_role_templates(config: &Config) -> Vec<AgentRoleTemplateEntry> {
    let built_in_names = built_in_role_configs()
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut user_names = BTreeSet::new();
    let mut entries = Vec::new();

    for (name, role_config) in &config.agent_roles {
        if user_names.insert(name.clone()) {
            entries.push(entry_from_config(
                config,
                name,
                role_config,
                AgentRoleTemplateSource::User,
                /*is_shadowed*/ false,
                built_in_names.contains(name),
            ));
        }
    }

    for (name, role_config) in built_in_role_configs() {
        entries.push(entry_from_config(
            config,
            name,
            role_config,
            AgentRoleTemplateSource::BuiltIn,
            user_names.contains(name),
            /*shadows_built_in*/ false,
        ));
    }

    entries
}

pub fn create_user_agent_role_template(
    config: &Config,
    raw_name: &str,
) -> Result<CreatedAgentRoleTemplate, AgentRoleTemplateCreateError> {
    let name = normalize_agent_role_template_name(raw_name)?;
    let contents = starter_template_contents(&name, None, StarterModelDefaults::default());
    create_user_agent_role_template_from_contents(config, &name, &contents)
}

pub fn starter_agent_role_template_draft() -> String {
    starter_template_contents("new-role", None, StarterModelDefaults::default())
}

pub fn starter_agent_role_template_draft_with_allowed_tools(allowed_tools: &[String]) -> String {
    starter_template_contents(
        "new-role",
        Some(allowed_tools),
        StarterModelDefaults::default(),
    )
}

pub fn starter_agent_role_template_draft_from_current_model(config: &Config) -> String {
    starter_template_contents(
        "new-role",
        None,
        StarterModelDefaults {
            model: config
                .model
                .as_deref()
                .filter(|model| !model.trim().is_empty())
                .map(str::to_string),
            model_provider: (!config.model_provider_id.trim().is_empty())
                .then(|| config.model_provider_id.clone()),
            model_reasoning_effort: config
                .model_reasoning_effort
                .as_ref()
                .map(ToString::to_string),
            service_tier: config
                .service_tier
                .as_deref()
                .filter(|service_tier| !service_tier.trim().is_empty())
                .map(str::to_string),
        },
    )
}

pub fn user_agent_role_template_draft_with_allowed_tools(
    config: &Config,
    role_name: &str,
    path: &Path,
    allowed_tools: &[String],
) -> Result<String, AgentRoleTemplateCreateError> {
    let mut contents = String::new();
    let path_buf = path.to_path_buf();
    fs::File::open(path)
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                AgentRoleTemplateCreateError::FileMissing(path_buf.clone())
            } else {
                AgentRoleTemplateCreateError::Write {
                    path: path_buf.clone(),
                    source_message: err.to_string(),
                }
            }
        })?
        .read_to_string(&mut contents)
        .map_err(|err| AgentRoleTemplateCreateError::Write {
            path: path_buf.clone(),
            source_message: err.to_string(),
        })?;

    let mut role_toml: TomlValue = toml::from_str(&contents).map_err(|err| {
        AgentRoleTemplateCreateError::InvalidTemplate(format!(
            "failed to parse agent role file at {}: {err}",
            path.display()
        ))
    })?;
    let Some(table) = role_toml.as_table_mut() else {
        return Err(AgentRoleTemplateCreateError::InvalidTemplate(format!(
            "agent role file at {} must contain a TOML table",
            path.display()
        )));
    };
    let mut tool_selection = toml::Table::new();
    tool_selection.insert(
        "allowed_tools".to_string(),
        TomlValue::Array(
            allowed_tools
                .iter()
                .map(|tool| TomlValue::String(tool.clone()))
                .collect(),
        ),
    );
    table.insert(
        "tool_selection".to_string(),
        TomlValue::Table(tool_selection),
    );
    let draft = toml::to_string_pretty(&role_toml)
        .map_err(|err| AgentRoleTemplateCreateError::InvalidTemplate(err.to_string()))?;
    validate_user_agent_role_template_update(config, role_name, path, &draft)?;
    Ok(draft)
}

pub fn create_user_agent_role_template_from_draft(
    config: &Config,
    draft: &str,
) -> Result<CreatedAgentRoleTemplate, AgentRoleTemplateCreateError> {
    let agents_dir = config.codex_home.join("agents");
    let draft_label = agents_dir.join("<draft>.toml");
    let parsed = parse_agent_role_file_contents(draft, &draft_label, &agents_dir, None)
        .map_err(|err| AgentRoleTemplateCreateError::InvalidTemplate(err.to_string()))?;
    let name = normalize_agent_role_template_name(&parsed.role_name)?;
    if parsed.role_name != name {
        return Err(AgentRoleTemplateCreateError::InvalidTemplate(format!(
            "agent role name `{}` must be written as normalized slug `{name}`",
            parsed.role_name
        )));
    }
    if parsed.description.is_none() {
        return Err(AgentRoleTemplateCreateError::InvalidTemplate(format!(
            "agent role `{name}` must define a description"
        )));
    }

    create_user_agent_role_template_from_contents(config, &name, draft)
}

pub fn update_user_agent_role_template_from_draft(
    config: &Config,
    role_name: &str,
    path: &Path,
    draft: &str,
) -> Result<CreatedAgentRoleTemplate, AgentRoleTemplateCreateError> {
    let name = validate_user_agent_role_template_update(config, role_name, path, draft)?;
    let path_buf = path.to_path_buf();
    let parsed =
        parse_agent_role_file_contents(draft, path, &config.codex_home.join("agents"), Some(&name))
            .map_err(|err| AgentRoleTemplateCreateError::InvalidTemplate(err.to_string()))?;
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(path)
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                AgentRoleTemplateCreateError::FileMissing(path_buf.clone())
            } else {
                AgentRoleTemplateCreateError::Write {
                    path: path_buf.clone(),
                    source_message: err.to_string(),
                }
            }
        })?;
    file.write_all(terminated_toml(draft).as_bytes())
        .map_err(|err| AgentRoleTemplateCreateError::Write {
            path: path_buf.clone(),
            source_message: err.to_string(),
        })?;

    Ok(CreatedAgentRoleTemplate {
        name,
        path: path_buf.clone(),
        config: role_config_from_parsed_file(parsed, path_buf),
    })
}

fn validate_user_agent_role_template_update(
    config: &Config,
    role_name: &str,
    path: &Path,
    draft: &str,
) -> Result<String, AgentRoleTemplateCreateError> {
    let name = normalize_agent_role_template_name(role_name)?;
    let path_buf = path.to_path_buf();
    if !path.exists() {
        return Err(AgentRoleTemplateCreateError::FileMissing(path_buf));
    }
    let parsed =
        parse_agent_role_file_contents(draft, path, &config.codex_home.join("agents"), Some(&name))
            .map_err(|err| AgentRoleTemplateCreateError::InvalidTemplate(err.to_string()))?;
    if parsed.role_name != name {
        return Err(AgentRoleTemplateCreateError::InvalidTemplate(format!(
            "agent role name `{}` must remain `{name}` for update",
            parsed.role_name
        )));
    }
    if parsed.description.is_none() {
        return Err(AgentRoleTemplateCreateError::InvalidTemplate(format!(
            "agent role `{name}` must define a description"
        )));
    }
    Ok(name)
}

fn create_user_agent_role_template_from_contents(
    config: &Config,
    name: &str,
    contents: &str,
) -> Result<CreatedAgentRoleTemplate, AgentRoleTemplateCreateError> {
    if resolve_role_config(config, name).is_some() {
        return Err(AgentRoleTemplateCreateError::AlreadyExists(
            name.to_string(),
        ));
    }

    let agents_dir = config.codex_home.join("agents");
    let path = agents_dir.join(format!("{name}.toml"));
    let agents_dir_path = agents_dir.to_path_buf();
    let path_buf = path.to_path_buf();
    if path.exists() {
        return Err(AgentRoleTemplateCreateError::FileExists(path_buf));
    }

    let parsed = parse_agent_role_file_contents(contents, &path, &agents_dir, Some(name))
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
    file.write_all(terminated_toml(contents).as_bytes())
        .map_err(|err| AgentRoleTemplateCreateError::Write {
            path: path_buf.clone(),
            source_message: err.to_string(),
        })?;

    Ok(CreatedAgentRoleTemplate {
        name: name.to_string(),
        path: path_buf.clone(),
        config: role_config_from_parsed_file(parsed, path_buf),
    })
}

fn role_config_from_parsed_file(
    parsed: crate::config::agent_roles::ResolvedAgentRoleFile,
    path: PathBuf,
) -> AgentRoleConfig {
    let runtime_config_sources = agent_role_runtime_config_sources(&parsed.config);
    AgentRoleConfig {
        description: parsed
            .description
            .or_else(|| Some(starter_description(&parsed.role_name))),
        config_file: Some(path),
        nickname_candidates: parsed
            .nickname_candidates
            .or_else(|| Some(vec![starter_nickname(&parsed.role_name)])),
        metadata_sources: crate::config::AgentRoleConfigMetadataSources {
            description: AgentRoleConfigFieldSource::role_file_metadata(),
            config_file: AgentRoleConfigFieldSource::discovered_role_file(),
            nickname_candidates: AgentRoleConfigFieldSource::role_file_metadata(),
        },
        runtime_config_sources,
    }
}

fn terminated_toml(contents: &str) -> String {
    let mut contents = contents.trim().to_string();
    contents.push('\n');
    contents
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
    config: &Config,
    name: &str,
    role_config: &AgentRoleConfig,
    source: AgentRoleTemplateSource,
    is_shadowed: bool,
    shadows_built_in: bool,
) -> AgentRoleTemplateEntry {
    let role_file_projection = role_file_projection(role_config, source);
    let config_file_origin = role_config
        .config_file
        .as_ref()
        .map(|config_file| config_file_origin(config, source, config_file));
    let availability = match (source, is_shadowed) {
        (AgentRoleTemplateSource::User, _) => AgentRoleTemplateAvailability::NewSessionsOnly,
        (AgentRoleTemplateSource::BuiltIn, true) => AgentRoleTemplateAvailability::ShadowedBuiltIn,
        (AgentRoleTemplateSource::BuiltIn, false) => AgentRoleTemplateAvailability::BuiltIn,
    };
    AgentRoleTemplateEntry {
        name: name.to_string(),
        description: role_config.description.clone(),
        source,
        is_shadowed,
        shadows_built_in,
        availability,
        config_file: role_config.config_file.clone(),
        config_file_origin,
        nickname_candidates: role_config.nickname_candidates.clone().unwrap_or_default(),
        locks: role_file_projection.locks,
        field_sources: AgentRoleTemplateFieldSources {
            declaration_fields: declaration_fields(role_config),
            declaration_origins: declaration_origins(config, name, role_config),
            config_file_fields: role_file_projection.field_names.clone(),
            bounded_provenance: bounded_field_provenance(
                config,
                name,
                role_config,
                source,
                config_file_origin,
                &role_file_projection.field_names,
            ),
        },
        validation_error: role_template_validation_error(name, role_config, source),
    }
}

fn config_file_origin(
    config: &Config,
    source: AgentRoleTemplateSource,
    config_file: &Path,
) -> AgentRoleTemplateConfigFileOrigin {
    if source == AgentRoleTemplateSource::BuiltIn {
        return AgentRoleTemplateConfigFileOrigin::BuiltInBundle;
    }

    let user_agents_dir = config.codex_home.join("agents");
    if config_file.starts_with(&user_agents_dir) {
        AgentRoleTemplateConfigFileOrigin::UserAgentsDir
    } else {
        AgentRoleTemplateConfigFileOrigin::ExternalConfigFile
    }
}

fn declaration_fields(role_config: &AgentRoleConfig) -> Vec<String> {
    let mut fields = Vec::new();
    if role_config.description.is_some() {
        fields.push("description".to_string());
    }
    if role_config.config_file.is_some() {
        fields.push("config_file".to_string());
    }
    if role_config
        .nickname_candidates
        .as_ref()
        .is_some_and(|candidates| !candidates.is_empty())
    {
        fields.push("nickname_candidates".to_string());
    }
    fields
}

fn declaration_origins(
    config: &Config,
    role_name: &str,
    role_config: &AgentRoleConfig,
) -> Vec<String> {
    let origins = config.config_layer_stack.origins();
    declaration_fields(role_config)
        .into_iter()
        .filter_map(|field_name| {
            let key = format!("agents.{role_name}.{field_name}");
            let metadata = origins
                .get(&key)
                .or_else(|| origin_for_descendant(&origins, &key))?;
            Some(format!(
                "{field_name}: {}",
                format_config_layer_source(&metadata.name, CONFIG_TOML_FILE)
            ))
        })
        .collect()
}

fn bounded_field_provenance(
    config: &Config,
    role_name: &str,
    role_config: &AgentRoleConfig,
    source: AgentRoleTemplateSource,
    config_file_origin: Option<AgentRoleTemplateConfigFileOrigin>,
    role_file_fields: &[String],
) -> Vec<AgentRoleTemplateFieldProvenance> {
    let role_file_fields = role_file_fields
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut provenance = Vec::new();

    if role_config.description.is_some() {
        provenance.push(metadata_field_provenance(
            config,
            role_name,
            "description",
            role_config.metadata_sources.description,
            role_file_fields.contains("description"),
        ));
    }
    if role_config.config_file.is_some() {
        provenance.push(config_file_field_provenance(
            config,
            role_name,
            role_config.metadata_sources.config_file,
            source,
            config_file_origin,
        ));
    }
    if role_config
        .nickname_candidates
        .as_ref()
        .is_some_and(|candidates| !candidates.is_empty())
    {
        provenance.push(metadata_field_provenance(
            config,
            role_name,
            "nickname_candidates",
            role_config.metadata_sources.nickname_candidates,
            role_file_fields.contains("nickname_candidates"),
        ));
    }

    for field_name in role_file_fields {
        if matches!(
            field_name,
            "description" | "nickname_candidates" | "config_file"
        ) {
            continue;
        }
        let is_known_runtime_field = matches!(
            field_name,
            "developer_instructions"
                | "model"
                | "model_provider"
                | "model_reasoning_effort"
                | "service_tier"
                | "approval_policy"
                | "sandbox_mode"
                | "default_permissions"
                | "sandbox_workspace_write"
                | "permissions"
                | "mcp_servers"
                | "hooks"
                | "tool_selection.allowed_tools"
                | "skills.config"
                | "apps"
        );
        provenance.push(AgentRoleTemplateFieldProvenance {
            field_name: field_name.to_string(),
            source: match source {
                AgentRoleTemplateSource::User => {
                    if field_name == "name" {
                        AgentRoleTemplateFieldProvenanceSource::RoleFileMetadata
                    } else if is_known_runtime_field {
                        AgentRoleTemplateFieldProvenanceSource::EffectiveRoleFile
                    } else {
                        AgentRoleTemplateFieldProvenanceSource::Unknown
                    }
                }
                AgentRoleTemplateSource::BuiltIn => {
                    AgentRoleTemplateFieldProvenanceSource::BuiltInBundle
                }
            },
            detail: if source == AgentRoleTemplateSource::User && is_known_runtime_field {
                role_config
                    .runtime_config_sources
                    .get(field_name)
                    .copied()
                    .and_then(runtime_config_field_detail)
                    .or_else(|| effective_role_file_detail(config, role_name, role_config))
            } else {
                None
            },
        });
    }

    provenance
}

fn effective_role_file_detail(
    config: &Config,
    role_name: &str,
    role_config: &AgentRoleConfig,
) -> Option<String> {
    provenance_detail(
        config,
        role_name,
        "config_file",
        role_config.metadata_sources.config_file,
    )
    .map(|detail| format!("selected config_file {detail}"))
}

fn runtime_config_field_detail(source: AgentRoleConfigRuntimeFieldSource) -> Option<String> {
    let mut details = Vec::new();
    if source.inherited_from_lower_precedence {
        details.push("inherited from lower-precedence role".to_string());
    }
    if source.overrides_lower_precedence {
        details.push("overrides lower-precedence role".to_string());
    }
    (!details.is_empty()).then(|| details.join("; "))
}

fn metadata_field_provenance(
    config: &Config,
    role_name: &str,
    field_name: &str,
    metadata_source: AgentRoleConfigFieldSource,
    role_file_contains_field: bool,
) -> AgentRoleTemplateFieldProvenance {
    if let Some(source) = metadata_source_to_template_source(metadata_source.kind) {
        return AgentRoleTemplateFieldProvenance {
            field_name: field_name.to_string(),
            source,
            detail: provenance_detail(config, role_name, field_name, metadata_source),
        };
    }

    if role_file_contains_field {
        return AgentRoleTemplateFieldProvenance {
            field_name: field_name.to_string(),
            source: AgentRoleTemplateFieldProvenanceSource::RoleFileMetadata,
            detail: None,
        };
    }

    if let Some(origin) = declaration_origin(config, role_name, field_name) {
        return AgentRoleTemplateFieldProvenance {
            field_name: field_name.to_string(),
            source: AgentRoleTemplateFieldProvenanceSource::ConfigLayer,
            detail: Some(origin),
        };
    }

    AgentRoleTemplateFieldProvenance {
        field_name: field_name.to_string(),
        source: AgentRoleTemplateFieldProvenanceSource::Unknown,
        detail: None,
    }
}

fn config_file_field_provenance(
    config: &Config,
    role_name: &str,
    metadata_source: AgentRoleConfigFieldSource,
    source: AgentRoleTemplateSource,
    config_file_origin: Option<AgentRoleTemplateConfigFileOrigin>,
) -> AgentRoleTemplateFieldProvenance {
    if let Some(source) = metadata_source_to_template_source(metadata_source.kind) {
        return AgentRoleTemplateFieldProvenance {
            field_name: "config_file".to_string(),
            source,
            detail: provenance_detail(config, role_name, "config_file", metadata_source),
        };
    }

    if let Some(origin) = declaration_origin(config, role_name, "config_file") {
        return AgentRoleTemplateFieldProvenance {
            field_name: "config_file".to_string(),
            source: AgentRoleTemplateFieldProvenanceSource::ConfigLayer,
            detail: Some(origin),
        };
    }

    let provenance_source = match (source, config_file_origin) {
        (AgentRoleTemplateSource::BuiltIn, _) => {
            AgentRoleTemplateFieldProvenanceSource::BuiltInBundle
        }
        (AgentRoleTemplateSource::User, Some(AgentRoleTemplateConfigFileOrigin::UserAgentsDir)) => {
            AgentRoleTemplateFieldProvenanceSource::DiscoveredRoleFile
        }
        _ => AgentRoleTemplateFieldProvenanceSource::Unknown,
    };

    AgentRoleTemplateFieldProvenance {
        field_name: "config_file".to_string(),
        source: provenance_source,
        detail: None,
    }
}

fn metadata_source_to_template_source(
    source: AgentRoleConfigFieldSourceKind,
) -> Option<AgentRoleTemplateFieldProvenanceSource> {
    match source {
        AgentRoleConfigFieldSourceKind::Unknown => None,
        AgentRoleConfigFieldSourceKind::ConfigLayer => {
            Some(AgentRoleTemplateFieldProvenanceSource::ConfigLayer)
        }
        AgentRoleConfigFieldSourceKind::RoleFileMetadata => {
            Some(AgentRoleTemplateFieldProvenanceSource::RoleFileMetadata)
        }
        AgentRoleConfigFieldSourceKind::DiscoveredRoleFile => {
            Some(AgentRoleTemplateFieldProvenanceSource::DiscoveredRoleFile)
        }
    }
}

fn provenance_detail(
    config: &Config,
    role_name: &str,
    field_name: &str,
    metadata_source: AgentRoleConfigFieldSource,
) -> Option<String> {
    let mut details = Vec::new();
    if metadata_source.kind == AgentRoleConfigFieldSourceKind::ConfigLayer
        && let Some(origin) = declaration_origin(config, role_name, field_name)
    {
        details.push(origin);
    }
    if metadata_source.inherited_from_lower_precedence {
        details.push("inherited from lower-precedence role".to_string());
    }
    if metadata_source.overrides_lower_precedence {
        details.push("overrides lower-precedence role".to_string());
    }
    (!details.is_empty()).then(|| details.join("; "))
}

fn declaration_origin(config: &Config, role_name: &str, field_name: &str) -> Option<String> {
    let origins = config.config_layer_stack.origins();
    let key = format!("agents.{role_name}.{field_name}");
    let metadata = origins
        .get(&key)
        .or_else(|| origin_for_descendant(&origins, &key))?;
    Some(format_config_layer_source(&metadata.name, CONFIG_TOML_FILE))
}

fn origin_for_descendant<'a>(
    origins: &'a std::collections::HashMap<String, codex_app_server_protocol::ConfigLayerMetadata>,
    key: &str,
) -> Option<&'a codex_app_server_protocol::ConfigLayerMetadata> {
    let prefix = format!("{key}.");
    origins
        .iter()
        .find_map(|(origin_key, metadata)| origin_key.starts_with(&prefix).then_some(metadata))
}

fn role_template_validation_error(
    name: &str,
    role_config: &AgentRoleConfig,
    source: AgentRoleTemplateSource,
) -> Option<String> {
    if source != AgentRoleTemplateSource::User {
        return None;
    }

    let config_file = role_config.config_file.as_ref()?;
    let contents = fs::read_to_string(config_file).map_err(|err| err.to_string());
    let contents = match contents {
        Ok(contents) => contents,
        Err(err) => {
            return Some(format!("failed to read {}: {err}", config_file.display()));
        }
    };
    let config_base_dir = config_file.parent().unwrap_or_else(|| Path::new("."));

    parse_agent_role_file_contents(&contents, config_file, config_base_dir, Some(name))
        .err()
        .map(|err| err.to_string())
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct RoleFileProjection {
    locks: AgentRoleTemplateLocks,
    field_names: Vec<String>,
}

fn role_file_projection(
    role_config: &AgentRoleConfig,
    source: AgentRoleTemplateSource,
) -> RoleFileProjection {
    let Some(config_file) = role_config.config_file.as_ref() else {
        return RoleFileProjection::default();
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
        .map(|role_toml| RoleFileProjection {
            locks: locks_from_toml(&role_toml),
            field_names: agent_role_file_field_names(&role_toml),
        })
        .unwrap_or_default()
}

fn locks_from_toml(role_toml: &TomlValue) -> AgentRoleTemplateLocks {
    AgentRoleTemplateLocks {
        model: role_toml
            .get("model")
            .and_then(TomlValue::as_str)
            .map(str::to_string),
        model_provider: role_toml
            .get("model_provider")
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
        allowed_tool_names: allowed_tool_names(role_toml),
        approval_policy: role_toml
            .get("approval_policy")
            .and_then(TomlValue::as_str)
            .map(str::to_string),
        sandbox_mode: role_toml
            .get("sandbox_mode")
            .and_then(TomlValue::as_str)
            .map(str::to_string),
        default_permissions: role_toml
            .get("default_permissions")
            .and_then(TomlValue::as_str)
            .map(str::to_string),
        has_sandbox_workspace_write: role_toml
            .get("sandbox_workspace_write")
            .and_then(TomlValue::as_table)
            .is_some_and(|table| !table.is_empty()),
        permission_profile_count: role_toml
            .get("permissions")
            .and_then(TomlValue::as_table)
            .map_or(0, toml::map::Map::len),
        mcp_server_count: role_toml
            .get("mcp_servers")
            .and_then(TomlValue::as_table)
            .map_or(0, toml::map::Map::len),
        hook_handler_count: hook_handler_count(role_toml),
        skill_config_count: role_toml
            .get("skills")
            .and_then(TomlValue::as_table)
            .and_then(|skills| skills.get("config"))
            .and_then(TomlValue::as_array)
            .map_or(0, Vec::len),
        app_config_count: role_toml
            .get("apps")
            .and_then(TomlValue::as_table)
            .map_or(0, |apps| {
                apps.keys().filter(|key| key.as_str() != "_default").count()
            }),
        has_apps_default_config: role_toml
            .get("apps")
            .and_then(TomlValue::as_table)
            .is_some_and(|apps| apps.contains_key("_default")),
        has_developer_instructions: role_toml
            .get("developer_instructions")
            .and_then(TomlValue::as_str)
            .is_some_and(|value| !value.trim().is_empty()),
    }
}

fn allowed_tool_names(role_toml: &TomlValue) -> Vec<String> {
    role_toml
        .get("tool_selection")
        .and_then(TomlValue::as_table)
        .and_then(|tool_selection| tool_selection.get("allowed_tools"))
        .and_then(TomlValue::as_array)
        .into_iter()
        .flatten()
        .filter_map(TomlValue::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect()
}

fn hook_handler_count(role_toml: &TomlValue) -> usize {
    let Some(hooks) = role_toml.get("hooks").and_then(TomlValue::as_table) else {
        return 0;
    };
    [
        "PreToolUse",
        "PermissionRequest",
        "PostToolUse",
        "PreCompact",
        "PostCompact",
        "SessionStart",
        "UserPromptSubmit",
        "SubagentStart",
        "SubagentStop",
        "Stop",
    ]
    .into_iter()
    .filter_map(|event_name| hooks.get(event_name))
    .filter_map(TomlValue::as_array)
    .flat_map(|groups| groups.iter())
    .filter_map(TomlValue::as_table)
    .filter_map(|group| group.get("hooks"))
    .filter_map(TomlValue::as_array)
    .map(Vec::len)
    .sum()
}

#[derive(Serialize)]
struct StarterTemplate<'a> {
    name: &'a str,
    description: String,
    nickname_candidates: Vec<String>,
    developer_instructions: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model_reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    service_tier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_selection: Option<StarterToolSelection<'a>>,
}

#[derive(Default)]
struct StarterModelDefaults {
    model: Option<String>,
    model_provider: Option<String>,
    model_reasoning_effort: Option<String>,
    service_tier: Option<String>,
}

#[derive(Serialize)]
struct StarterToolSelection<'a> {
    allowed_tools: &'a [String],
}

fn starter_template_contents(
    name: &str,
    allowed_tools: Option<&[String]>,
    model_defaults: StarterModelDefaults,
) -> String {
    let template = StarterTemplate {
        name,
        description: starter_description(name),
        nickname_candidates: vec![starter_nickname(name)],
        developer_instructions: format!(
            "Define `{name}` responsibilities and completion criteria."
        ),
        model: model_defaults.model,
        model_provider: model_defaults.model_provider,
        model_reasoning_effort: model_defaults.model_reasoning_effort,
        service_tier: model_defaults.service_tier,
        tool_selection: allowed_tools
            .filter(|allowed_tools| !allowed_tools.is_empty())
            .map(|allowed_tools| StarterToolSelection { allowed_tools }),
    };
    let mut contents = toml::to_string_pretty(&template)
        .unwrap_or_else(|err| unreachable!("starter template serializes: {err}"));
    if allowed_tools.is_none_or(<[String]>::is_empty) {
        contents.push_str(
            "# Optional tool boundary: add [tool_selection] with allowed_tools = [\"update_plan\"].\n",
        );
    }
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
