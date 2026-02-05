// === FORK: AGENT REGISTRY MODULE ===
//! User-defined agent loading system.
//!
//! This module provides functionality to load custom agent definitions
//! from `.codex/agents/*.md` files with YAML frontmatter.
//!
//! Part of Feature::FnMultiAgents enhancements.
// === FORK: AGENT REGISTRY MODULE ===

use crate::config::Config;
use crate::config_loader::ConfigLayerStack;
use crate::config_loader::ConfigLayerStackOrdering;
use crate::protocol::SandboxPolicy;
use codex_app_server_protocol::ConfigLayerSource;
use codex_protocol::openai_models::ReasoningEffort;
use serde::Deserialize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

const AGENTS_DIR_NAME: &str = "agents";
const MAX_SCAN_DEPTH: usize = 4;
const MAX_AGENT_DIRS_PER_ROOT: usize = 2000;
const MAX_NAME_LEN: usize = 64;
const MIN_NAME_LEN: usize = 3;
const MAX_DESCRIPTION_LEN: usize = 4096;

/// Prefix for built-in agent files that get seeded to ~/.codex/agents/
const BUILTIN_AGENT_PREFIX: &str = "codex_";

/// Built-in agents included at compile time for seeding
static BUILTIN_AGENTS: &[(&str, &str)] = &[
    (
        "codex_worker",
        include_str!("../../templates/agents/codex_worker.md"),
    ),
    (
        "codex_explorer",
        include_str!("../../templates/agents/codex_explorer.md"),
    ),
    (
        "codex_reviewer",
        include_str!("../../templates/agents/codex_reviewer.md"),
    ),
    (
        "codex_architect",
        include_str!("../../templates/agents/codex_architect.md"),
    ),
    (
        "codex_bug-hunter",
        include_str!("../../templates/agents/codex_bug-hunter.md"),
    ),
    (
        "codex_orchestrator",
        include_str!("../../templates/agents/codex_orchestrator.md"),
    ),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentScope {
    Repo,
    User,
    System,
}

#[derive(Debug, Clone)]
pub(crate) struct AgentRoot {
    pub(crate) path: PathBuf,
    pub(crate) scope: AgentScope,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AgentColor {
    Blue,
    Cyan,
    Green,
    Yellow,
    Magenta,
    Red,
}

impl AgentColor {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "blue" => Some(AgentColor::Blue),
            "cyan" => Some(AgentColor::Cyan),
            "green" => Some(AgentColor::Green),
            "yellow" => Some(AgentColor::Yellow),
            "magenta" => Some(AgentColor::Magenta),
            "red" => Some(AgentColor::Red),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentDefinition {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) model: String,
    pub(crate) reasoning_effort: Option<ReasoningEffort>,
    pub(crate) agent_name_models: HashMap<String, String>,
    pub(crate) agent_name_reasoning_efforts: HashMap<String, ReasoningEffort>,
    pub(crate) color: AgentColor,
    pub(crate) tools: Option<Vec<String>>,
    pub(crate) read_only: bool,
    pub(crate) tool_denylist: Option<Vec<String>>,
    pub(crate) agent_name_descriptions: HashMap<String, String>,
    pub(crate) agent_name_instructions: HashMap<String, String>,
    pub(crate) instructions: String,
    pub(crate) path: PathBuf,
    pub(crate) scope: AgentScope,
}

impl AgentDefinition {
    pub(crate) fn apply_to_config(
        &self,
        config: &mut Config,
        agent_name: Option<&str>,
    ) -> Result<(), String> {
        config.base_instructions = Some(self.instructions_for(agent_name)?.to_string());
        config.model = Some(self.model.clone());
        if let Some(agent_name) = agent_name {
            if let Some(model) = self.agent_name_models.get(agent_name) {
                config.model = Some(model.clone());
            }
            if let Some(effort) = self.agent_name_reasoning_efforts.get(agent_name) {
                config.model_reasoning_effort = Some(*effort);
            } else if let Some(effort) = self.reasoning_effort {
                config.model_reasoning_effort = Some(effort);
            }
        } else if let Some(effort) = self.reasoning_effort {
            config.model_reasoning_effort = Some(effort);
        }

        if let Some(tools) = self.tools.as_ref() {
            apply_agent_tool_allowlist(config, tools);
        }

        // Apply read_only → sandbox_policy
        if self.read_only {
            config
                .sandbox_policy
                .set(SandboxPolicy::new_read_only_policy())
                .map_err(|err| format!("sandbox_policy invalid: {err}"))?;
        }

        // Apply tool_denylist (merge with existing)
        if let Some(denylist) = self.tool_denylist.as_ref() {
            apply_agent_tool_denylist(config, denylist);
        }

        Ok(())
    }

    pub(crate) fn instructions_for(&self, agent_name: Option<&str>) -> Result<&str, String> {
        let Some(agent_name) = agent_name else {
            return Ok(&self.instructions);
        };

        if let Some(instructions) = self.agent_name_instructions.get(agent_name) {
            return Ok(instructions);
        }

        let mut available = self
            .agent_name_descriptions
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        available.sort();
        let available = if available.is_empty() {
            "none".to_string()
        } else {
            available.join(", ")
        };
        Err(format!(
            "agent_name \"{agent_name}\" not found for agent type \"{}\". Available: {available}",
            self.name
        ))
    }
}

#[derive(Debug, Default, Clone)]
pub(crate) struct AgentRegistry {
    pub(crate) agents: Vec<AgentDefinition>,
    pub(crate) errors: Vec<AgentRegistryError>,
}

impl AgentRegistry {
    pub(crate) fn load_for_config(config: &Config) -> Self {
        let roots = agent_roots_from_layer_stack(&config.config_layer_stack);
        load_agents_from_roots(roots)
    }

    pub(crate) fn find(&self, name: &str) -> Option<&AgentDefinition> {
        self.agents.iter().find(|agent| agent.name == name)
    }

    pub(crate) fn format_agent_descriptions(&self) -> String {
        if self.agents.is_empty() {
            return String::new();
        }

        self.agents
            .iter()
            .map(|agent| {
                let description = agent.description.trim();
                let name_json = serde_json::to_string(&agent.name)
                    .expect("agent names should always serialize to JSON");
                let description_json = serde_json::to_string(description)
                    .expect("agent descriptions should always serialize to JSON");
                format!("{{ \"name\": {name_json}, \"description\": {description_json} }}")
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub(crate) fn format_agent_name_descriptions(&self) -> String {
        let mut entries = Vec::new();
        for agent in &self.agents {
            if agent.agent_name_descriptions.is_empty() {
                continue;
            }
            let mut agent_names = agent.agent_name_descriptions.iter().collect::<Vec<_>>();
            agent_names.sort_by(|a, b| a.0.cmp(b.0));
            let agent_names = agent_names
                .into_iter()
                .map(|(name, description)| {
                    let name_json = serde_json::to_string(name)
                        .expect("agent_name should always serialize to JSON");
                    let description_json = serde_json::to_string(description)
                        .expect("agent_name descriptions should always serialize to JSON");
                    format!("{{ \"name\": {name_json}, \"description\": {description_json} }}")
                })
                .collect::<Vec<_>>()
                .join(", ");
            let agent_type_json = serde_json::to_string(&agent.name)
                .expect("agent names should always serialize to JSON");
            entries.push(format!(
                "{{ \"agent_type\": {agent_type_json}, \"agent_names\": [{agent_names}] }}"
            ));
        }
        entries.join(", ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentRegistryError {
    pub(crate) path: PathBuf,
    pub(crate) message: String,
}

#[derive(Debug)]
enum AgentParseError {
    Read(std::io::Error),
    MissingFrontmatter,
    InvalidYaml(serde_yaml::Error),
    MissingField(&'static str),
    InvalidField { field: &'static str, reason: String },
}

impl fmt::Display for AgentParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentParseError::Read(e) => write!(f, "failed to read file: {e}"),
            AgentParseError::MissingFrontmatter => {
                write!(f, "missing YAML frontmatter delimited by ---")
            }
            AgentParseError::InvalidYaml(e) => write!(f, "invalid YAML: {e}"),
            AgentParseError::MissingField(field) => write!(f, "missing field `{field}`"),
            AgentParseError::InvalidField { field, reason } => {
                write!(f, "invalid {field}: {reason}")
            }
        }
    }
}

impl Error for AgentParseError {}

#[derive(Debug, Deserialize)]
struct AgentFrontmatter {
    name: String,
    description: String,
    model: String,
    #[serde(default)]
    reasoning_effort: Option<ReasoningEffort>,
    color: String,
    #[serde(default)]
    tools: Option<ToolsField>,
    #[serde(default)]
    read_only: bool,
    #[serde(default)]
    tool_denylist: Option<ToolsField>,
    #[serde(default)]
    agent_names: Option<Vec<AgentNameEntry>>,
}

#[derive(Debug, Deserialize)]
struct AgentNameEntry {
    name: String,
    description: String,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    reasoning_effort: Option<ReasoningEffort>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ToolsField {
    Single(String),
    List(Vec<String>),
}

pub(crate) fn agent_roots_from_layer_stack(
    config_layer_stack: &ConfigLayerStack,
) -> Vec<AgentRoot> {
    let mut roots = Vec::new();
    for layer in
        config_layer_stack.get_layers(ConfigLayerStackOrdering::HighestPrecedenceFirst, false)
    {
        let Some(config_folder) = layer.config_folder() else {
            continue;
        };

        match &layer.name {
            ConfigLayerSource::Project { .. } => {
                roots.push(AgentRoot {
                    path: config_folder.as_path().join(AGENTS_DIR_NAME),
                    scope: AgentScope::Repo,
                });
            }
            ConfigLayerSource::User { .. } => {
                roots.push(AgentRoot {
                    path: config_folder.as_path().join(AGENTS_DIR_NAME),
                    scope: AgentScope::User,
                });
            }
            ConfigLayerSource::System { .. } => {
                roots.push(AgentRoot {
                    path: config_folder.as_path().join(AGENTS_DIR_NAME),
                    scope: AgentScope::System,
                });
            }
            ConfigLayerSource::Mdm { .. }
            | ConfigLayerSource::SessionFlags
            | ConfigLayerSource::LegacyManagedConfigTomlFromFile { .. }
            | ConfigLayerSource::LegacyManagedConfigTomlFromMdm => {}
        }
    }
    roots
}

pub(crate) fn load_agents_from_roots<I>(roots: I) -> AgentRegistry
where
    I: IntoIterator<Item = AgentRoot>,
{
    let mut registry = AgentRegistry::default();
    let mut seen = HashSet::new();
    for root in roots {
        discover_agents_under_root(&root.path, root.scope, &mut registry, &mut seen);
    }

    registry.agents.sort_by(|a, b| {
        scope_rank(a.scope)
            .cmp(&scope_rank(b.scope))
            .then_with(|| a.name.cmp(&b.name))
    });
    registry
}

fn scope_rank(scope: AgentScope) -> u8 {
    match scope {
        AgentScope::Repo => 0,
        AgentScope::User => 1,
        AgentScope::System => 2,
    }
}

fn discover_agents_under_root(
    root: &Path,
    scope: AgentScope,
    registry: &mut AgentRegistry,
    seen: &mut HashSet<String>,
) {
    let root = match fs::canonicalize(root) {
        Ok(root) => root,
        Err(_) => return,
    };
    if !root.is_dir() {
        return;
    }

    let mut queue: VecDeque<(PathBuf, usize)> = VecDeque::new();
    queue.push_back((root.clone(), 0));
    let mut dir_count = 0usize;
    let mut truncated_by_dir_limit = false;

    while let Some((dir, depth)) = queue.pop_front() {
        if depth > MAX_SCAN_DEPTH {
            continue;
        }
        dir_count += 1;
        if dir_count > MAX_AGENT_DIRS_PER_ROOT {
            truncated_by_dir_limit = true;
            break;
        }
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                queue.push_back((path, depth + 1));
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case("AGENTS.md"))
            {
                continue;
            }
            if path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| !ext.eq_ignore_ascii_case("md"))
            {
                continue;
            }
            match parse_agent_file(&path, scope) {
                Ok(agent) => {
                    let key = agent.name.clone();
                    if !seen.insert(key.clone()) {
                        registry.errors.push(AgentRegistryError {
                            path: path.clone(),
                            message: format!("duplicate agent name \"{key}\""),
                        });
                        continue;
                    }
                    registry.agents.push(agent);
                }
                Err(err) => {
                    registry.errors.push(AgentRegistryError {
                        path: path.clone(),
                        message: err.to_string(),
                    });
                }
            }
        }
    }

    if truncated_by_dir_limit {
        tracing::warn!(
            "agents scan truncated after {} directories (root: {})",
            MAX_AGENT_DIRS_PER_ROOT,
            root.display()
        );
    }
}

fn parse_agent_file(path: &Path, scope: AgentScope) -> Result<AgentDefinition, AgentParseError> {
    let contents = fs::read_to_string(path).map_err(AgentParseError::Read)?;
    let (frontmatter, body) =
        split_frontmatter(&contents).ok_or(AgentParseError::MissingFrontmatter)?;
    let parsed: AgentFrontmatter =
        serde_yaml::from_str(&frontmatter).map_err(AgentParseError::InvalidYaml)?;

    let name = parsed.name.trim().to_string();
    if name.is_empty() {
        return Err(AgentParseError::MissingField("name"));
    }
    validate_agent_name(&name)?;

    let description = parsed.description.trim().to_string();
    if description.is_empty() {
        return Err(AgentParseError::MissingField("description"));
    }
    validate_len(&description, MAX_DESCRIPTION_LEN, "description")?;

    let model = parsed.model.trim().to_string();
    if model.is_empty() {
        return Err(AgentParseError::MissingField("model"));
    }

    let color = AgentColor::parse(&parsed.color).ok_or_else(|| AgentParseError::InvalidField {
        field: "color",
        reason: format!("unsupported color \"{}\"", parsed.color.trim()),
    })?;
    let reasoning_effort = parsed.reasoning_effort;

    let tools = normalize_tools(parsed.tools)?;
    let read_only = parsed.read_only;
    let tool_denylist = normalize_tools(parsed.tool_denylist)?;

    let (instructions, agent_name_instructions) = split_agent_instructions(&body)?;
    let agent_name_overrides = normalize_agent_names(parsed.agent_names)?;
    validate_agent_name_blocks(&agent_name_instructions, &agent_name_overrides.descriptions)?;

    let resolved_path = dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

    Ok(AgentDefinition {
        name,
        description,
        model,
        reasoning_effort,
        agent_name_models: agent_name_overrides.models,
        agent_name_reasoning_efforts: agent_name_overrides.reasoning_efforts,
        color,
        tools,
        read_only,
        tool_denylist,
        agent_name_descriptions: agent_name_overrides.descriptions,
        agent_name_instructions,
        instructions,
        path: resolved_path,
        scope,
    })
}

fn parse_agent_name_marker(line: &str) -> Result<Option<String>, AgentParseError> {
    let trimmed = line.trim();
    let Some(inner) = trimmed
        .strip_prefix("<!--")
        .and_then(|rest| rest.strip_suffix("-->"))
    else {
        return Ok(None);
    };
    let inner = inner.trim();
    let Some(name) = inner.strip_prefix("agent_name:") else {
        return Ok(None);
    };
    let name = name.trim();
    if name.is_empty() {
        return Err(AgentParseError::InvalidField {
            field: "agent_name",
            reason: "marker is missing a name".to_string(),
        });
    }
    validate_agent_name(name)?;
    Ok(Some(name.to_string()))
}

fn split_agent_instructions(
    body: &str,
) -> Result<(String, HashMap<String, String>), AgentParseError> {
    let mut agent_name_instructions = HashMap::new();
    let mut default_instructions = None;
    let mut current_name: Option<String> = None;
    let mut buffer = String::new();

    for segment in body.split_inclusive('\n') {
        let line = segment.trim_end_matches(['\r', '\n']);
        if let Some(name) = parse_agent_name_marker(line)? {
            let trimmed = buffer.trim().to_string();
            if let Some(current) = current_name.take() {
                if trimmed.is_empty() {
                    return Err(AgentParseError::InvalidField {
                        field: "agent_name",
                        reason: format!("instructions for \"{current}\" are empty"),
                    });
                }
                if agent_name_instructions
                    .insert(current.clone(), trimmed)
                    .is_some()
                {
                    return Err(AgentParseError::InvalidField {
                        field: "agent_name",
                        reason: format!("duplicate agent_name \"{current}\""),
                    });
                }
            } else {
                default_instructions = Some(trimmed);
            }
            buffer.clear();
            current_name = Some(name);
            continue;
        }
        buffer.push_str(segment);
    }

    let trimmed = buffer.trim().to_string();
    if let Some(current) = current_name {
        if trimmed.is_empty() {
            return Err(AgentParseError::InvalidField {
                field: "agent_name",
                reason: format!("instructions for \"{current}\" are empty"),
            });
        }
        if agent_name_instructions
            .insert(current.clone(), trimmed)
            .is_some()
        {
            return Err(AgentParseError::InvalidField {
                field: "agent_name",
                reason: format!("duplicate agent_name \"{current}\""),
            });
        }
    } else {
        default_instructions = Some(trimmed);
    }

    let instructions = default_instructions.unwrap_or_default();
    if instructions.is_empty() {
        return Err(AgentParseError::InvalidField {
            field: "body",
            reason: "agent instructions are empty".to_string(),
        });
    }

    Ok((instructions, agent_name_instructions))
}

struct AgentNameOverrides {
    descriptions: HashMap<String, String>,
    models: HashMap<String, String>,
    reasoning_efforts: HashMap<String, ReasoningEffort>,
}

fn normalize_agent_names(
    agent_names: Option<Vec<AgentNameEntry>>,
) -> Result<AgentNameOverrides, AgentParseError> {
    let Some(agent_names) = agent_names else {
        return Ok(AgentNameOverrides {
            descriptions: HashMap::new(),
            models: HashMap::new(),
            reasoning_efforts: HashMap::new(),
        });
    };
    let mut entries = HashMap::new();
    let mut models = HashMap::new();
    let mut reasoning_efforts = HashMap::new();
    for entry in agent_names {
        let name = entry.name.trim();
        if name.is_empty() {
            return Err(AgentParseError::InvalidField {
                field: "agent_names",
                reason: "agent_name is empty".to_string(),
            });
        }
        validate_agent_name(name)?;
        let description = entry.description.trim();
        if description.is_empty() {
            return Err(AgentParseError::InvalidField {
                field: "agent_names",
                reason: format!("description is empty for agent_name \"{name}\""),
            });
        }
        if let Some(model) = entry.model.as_deref() {
            let model = model.trim();
            if model.is_empty() {
                return Err(AgentParseError::InvalidField {
                    field: "agent_names",
                    reason: format!("model is empty for agent_name \"{name}\""),
                });
            }
            models.insert(name.to_string(), model.to_string());
        }
        if let Some(effort) = entry.reasoning_effort {
            reasoning_efforts.insert(name.to_string(), effort);
        }
        if entries
            .insert(name.to_string(), description.to_string())
            .is_some()
        {
            return Err(AgentParseError::InvalidField {
                field: "agent_names",
                reason: format!("duplicate agent_name \"{name}\""),
            });
        }
    }
    Ok(AgentNameOverrides {
        descriptions: entries,
        models,
        reasoning_efforts,
    })
}

fn validate_agent_name_blocks(
    agent_name_instructions: &HashMap<String, String>,
    agent_name_descriptions: &HashMap<String, String>,
) -> Result<(), AgentParseError> {
    if agent_name_instructions.is_empty() && agent_name_descriptions.is_empty() {
        return Ok(());
    }
    if agent_name_instructions.is_empty() {
        return Err(AgentParseError::InvalidField {
            field: "agent_names",
            reason: "agent_names metadata requires agent_name blocks".to_string(),
        });
    }
    if agent_name_descriptions.is_empty() {
        return Err(AgentParseError::InvalidField {
            field: "agent_names",
            reason: "agent_name blocks require agent_names metadata".to_string(),
        });
    }
    for name in agent_name_instructions.keys() {
        if !agent_name_descriptions.contains_key(name) {
            return Err(AgentParseError::InvalidField {
                field: "agent_names",
                reason: format!("missing agent_names entry for \"{name}\""),
            });
        }
    }
    for name in agent_name_descriptions.keys() {
        if !agent_name_instructions.contains_key(name) {
            return Err(AgentParseError::InvalidField {
                field: "agent_names",
                reason: format!("missing agent_name block for \"{name}\""),
            });
        }
    }
    Ok(())
}

fn normalize_tools(tools: Option<ToolsField>) -> Result<Option<Vec<String>>, AgentParseError> {
    let Some(tools) = tools else {
        return Ok(None);
    };
    let mut entries = Vec::new();
    match tools {
        ToolsField::Single(value) => {
            entries.extend(
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(String::from),
            );
        }
        ToolsField::List(values) => {
            entries.extend(
                values
                    .into_iter()
                    .map(|value| value.trim().to_string())
                    .filter(|s| !s.is_empty()),
            );
        }
    }

    if entries.is_empty() {
        return Ok(Some(Vec::new()));
    }

    // Deduplicate without expanding aliases
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for tool in entries {
        let key = tool.to_ascii_lowercase();
        if seen.insert(key) {
            deduped.push(tool);
        }
    }
    if deduped.iter().any(|entry| entry == "*") {
        return Ok(Some(vec!["*".to_string()]));
    }
    Ok(Some(deduped))
}

fn validate_len(value: &str, max: usize, field: &'static str) -> Result<(), AgentParseError> {
    if value.len() <= max {
        return Ok(());
    }
    Err(AgentParseError::InvalidField {
        field,
        reason: format!("length exceeds {max} characters"),
    })
}

fn validate_agent_name(name: &str) -> Result<(), AgentParseError> {
    if name.len() < MIN_NAME_LEN || name.len() > MAX_NAME_LEN {
        return Err(AgentParseError::InvalidField {
            field: "name",
            reason: format!("length must be {MIN_NAME_LEN}-{MAX_NAME_LEN} characters"),
        });
    }
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Err(AgentParseError::InvalidField {
            field: "name",
            reason: "name is empty".to_string(),
        });
    };
    if !first.is_ascii_alphanumeric() {
        return Err(AgentParseError::InvalidField {
            field: "name",
            reason: "must start with alphanumeric character".to_string(),
        });
    }
    let mut last = first;
    for ch in chars {
        last = ch;
        if !(ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-') {
            return Err(AgentParseError::InvalidField {
                field: "name",
                reason: "only lowercase letters, digits, and hyphens are allowed".to_string(),
            });
        }
    }
    if !last.is_ascii_alphanumeric() {
        return Err(AgentParseError::InvalidField {
            field: "name",
            reason: "must end with alphanumeric character".to_string(),
        });
    }
    Ok(())
}

fn split_frontmatter(contents: &str) -> Option<(String, String)> {
    let mut segments = contents.split_inclusive('\n');
    let first_segment = segments.next()?;
    let first_line = first_segment.trim_end_matches(['\r', '\n']);
    if first_line.trim() != "---" {
        return None;
    }

    let mut frontmatter_lines = Vec::new();
    let mut consumed = first_segment.len();
    let mut found_closing = false;

    for segment in segments {
        let line = segment.trim_end_matches(['\r', '\n']);
        if line.trim() == "---" {
            consumed += segment.len();
            found_closing = true;
            break;
        }
        frontmatter_lines.push(line);
        consumed += segment.len();
    }

    if frontmatter_lines.is_empty() || !found_closing {
        return None;
    }

    let frontmatter = frontmatter_lines.join("\n");
    let body = if consumed >= contents.len() {
        String::new()
    } else {
        contents[consumed..].to_string()
    };
    Some((frontmatter, body))
}

fn apply_agent_tool_allowlist(config: &mut Config, tools: &[String]) {
    if tools.len() == 1 && tools[0] == "*" {
        return;
    }
    let next = match config.tool_allowlist.as_ref() {
        Some(existing) => {
            let allowlist = crate::tool_allowlist::ToolAllowlist::from_patterns(existing);
            tools
                .iter()
                .filter(|tool| allowlist.allows(tool))
                .cloned()
                .collect()
        }
        None => tools.to_vec(),
    };
    config.tool_allowlist = Some(next);
}

fn apply_agent_tool_denylist(config: &mut Config, denylist: &[String]) {
    let mut next = config.tool_denylist.take().unwrap_or_default();
    for tool in denylist {
        if !next.contains(tool) {
            next.push(tool.clone());
        }
    }
    config.tool_denylist = if next.is_empty() { None } else { Some(next) };
}

/// Seeds built-in agents to ~/.codex/agents/ if no codex_*.md files exist.
///
/// This function checks if the agents directory contains any files with the
/// `codex_` prefix. If none exist, it writes all built-in agents to the directory.
/// This ensures users have default agents available while allowing them to
/// customize or override them.
pub fn seed_builtin_agents(codex_home: &Path) -> std::io::Result<()> {
    let agents_dir = codex_home.join(AGENTS_DIR_NAME);

    // Check for any existing codex_*.md file
    let has_builtin = agents_dir.is_dir()
        && fs::read_dir(&agents_dir)?.filter_map(Result::ok).any(|e| {
            e.file_name()
                .to_str()
                .map(|n| n.starts_with(BUILTIN_AGENT_PREFIX) && n.ends_with(".md"))
                .unwrap_or(false)
        });

    if has_builtin {
        return Ok(());
    }

    fs::create_dir_all(&agents_dir)?;
    for (filename, content) in BUILTIN_AGENTS {
        let path = agents_dir.join(format!("{filename}.md"));
        fs::write(&path, content)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigBuilder;
    use crate::config::ConfigOverrides;
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;

    fn write_agent(dir: &Path, name: &str, model: &str, tools: &str) -> PathBuf {
        let path = dir.join(format!("{name}.md"));
        let contents = format!(
            "---\nname: {name}\ndescription: test agent\nmodel: {model}\nreasoning_effort: high\ncolor: blue\ntools: {tools}\n---\n\nBody\n"
        );
        fs::write(&path, contents).expect("write agent");
        path
    }

    #[tokio::test]
    async fn loads_agents_from_project_layer() {
        let codex_home = TempDir::new().expect("tempdir");
        let repo = TempDir::new().expect("tempdir");
        let agent_dir = repo.path().join(".codex").join("agents");
        fs::create_dir_all(&agent_dir).expect("create agents dir");
        write_agent(&agent_dir, "explorer", "opus", "Read, Grep");

        let config = ConfigBuilder::default()
            .codex_home(codex_home.path().to_path_buf())
            .harness_overrides(ConfigOverrides {
                cwd: Some(repo.path().to_path_buf()),
                ..Default::default()
            })
            .build()
            .await
            .expect("load config");
        let registry = AgentRegistry::load_for_config(&config);
        assert_eq!(registry.errors, Vec::new());
        assert_eq!(registry.agents.len(), 1);
        let agent = &registry.agents[0];
        assert_eq!(agent.name, "explorer");
        assert_eq!(agent.model, "opus");
        assert_eq!(
            agent.tools.as_ref().unwrap(),
            &vec!["Read".to_string(), "Grep".to_string()]
        );
        assert_eq!(agent.reasoning_effort, Some(ReasoningEffort::High));
    }

    #[test]
    fn format_agent_descriptions_uses_full_description() {
        let registry = AgentRegistry {
            agents: vec![
                AgentDefinition {
                    name: "worker".to_string(),
                    description: "Use for execution.\nExtra line.".to_string(),
                    model: "gpt-5".to_string(),
                    reasoning_effort: None,
                    agent_name_models: HashMap::new(),
                    agent_name_reasoning_efforts: HashMap::new(),
                    color: AgentColor::Blue,
                    tools: None,
                    read_only: false,
                    tool_denylist: None,
                    agent_name_descriptions: HashMap::new(),
                    agent_name_instructions: HashMap::new(),
                    instructions: "Do the work.".to_string(),
                    path: PathBuf::from("worker.md"),
                    scope: AgentScope::User,
                },
                AgentDefinition {
                    name: "explorer".to_string(),
                    description: "Use for exploration.".to_string(),
                    model: "gpt-5".to_string(),
                    reasoning_effort: None,
                    agent_name_models: HashMap::new(),
                    agent_name_reasoning_efforts: HashMap::new(),
                    color: AgentColor::Cyan,
                    tools: None,
                    read_only: false,
                    tool_denylist: None,
                    agent_name_descriptions: HashMap::new(),
                    agent_name_instructions: HashMap::new(),
                    instructions: "Explore the codebase.".to_string(),
                    path: PathBuf::from("explorer.md"),
                    scope: AgentScope::User,
                },
            ],
            errors: Vec::new(),
        };

        let descriptions = registry.format_agent_descriptions();

        assert_eq!(
            descriptions,
            "{ \"name\": \"worker\", \"description\": \"Use for execution.\\nExtra line.\" }, { \"name\": \"explorer\", \"description\": \"Use for exploration.\" }"
        );
    }

    #[test]
    fn validates_agent_name_rules() {
        let err = validate_agent_name("Bad_Name").expect_err("invalid name should error");
        assert_eq!(
            err.to_string(),
            "invalid name: only lowercase letters, digits, and hyphens are allowed"
        );
    }

    #[test]
    fn parses_read_only_and_tool_denylist() {
        let content = r#"---
name: test-agent
description: Test agent description
model: gpt-5
color: red
read_only: true
tool_denylist:
  - apply_patch
  - shell
---
Instructions for the agent
"#;
        let temp_dir = TempDir::new().expect("tempdir");
        let path = temp_dir.path().join("test-agent.md");
        fs::write(&path, content).expect("write agent file");

        let agent = parse_agent_file(&path, AgentScope::User).expect("parse agent");
        assert_eq!(agent.name, "test-agent");
        assert!(agent.read_only);
        assert_eq!(
            agent.tool_denylist,
            Some(vec!["apply_patch".to_string(), "shell".to_string()])
        );
    }

    #[tokio::test]
    async fn apply_to_config_merges_tool_denylist() {
        let codex_home = TempDir::new().expect("tempdir");
        let mut config = ConfigBuilder::default()
            .codex_home(codex_home.path().to_path_buf())
            .build()
            .await
            .expect("load config");
        config.tool_denylist = Some(vec!["shell".to_string()]);

        let agent = AgentDefinition {
            name: "worker".to_string(),
            description: "Use for execution.".to_string(),
            model: "gpt-5".to_string(),
            reasoning_effort: None,
            agent_name_models: HashMap::new(),
            agent_name_reasoning_efforts: HashMap::new(),
            color: AgentColor::Blue,
            tools: None,
            read_only: false,
            tool_denylist: Some(vec!["apply_patch".to_string(), "shell".to_string()]),
            agent_name_descriptions: HashMap::new(),
            agent_name_instructions: HashMap::new(),
            instructions: "Do the work.".to_string(),
            path: PathBuf::from("worker.md"),
            scope: AgentScope::User,
        };

        agent
            .apply_to_config(&mut config, None)
            .expect("apply to config");

        assert_eq!(
            config.tool_denylist,
            Some(vec!["shell".to_string(), "apply_patch".to_string()])
        );
    }

    #[tokio::test]
    async fn agent_name_overrides_model_and_effort() {
        let codex_home = TempDir::new().expect("tempdir");
        let mut config = ConfigBuilder::default()
            .codex_home(codex_home.path().to_path_buf())
            .build()
            .await
            .expect("load config");
        config.model_reasoning_effort = Some(ReasoningEffort::Medium);

        let agent = AgentDefinition {
            name: "reviewer".to_string(),
            description: "Default reviewer description".to_string(),
            model: "gpt-5".to_string(),
            reasoning_effort: None,
            agent_name_models: HashMap::from([("strict".to_string(), "gpt-4.1".to_string())]),
            agent_name_reasoning_efforts: HashMap::from([(
                "strict".to_string(),
                ReasoningEffort::High,
            )]),
            color: AgentColor::Red,
            tools: None,
            read_only: false,
            tool_denylist: None,
            agent_name_descriptions: HashMap::from([(
                "strict".to_string(),
                "Strict instructions".to_string(),
            )]),
            agent_name_instructions: HashMap::from([(
                "strict".to_string(),
                "Strict instructions".to_string(),
            )]),
            instructions: "Default instructions.".to_string(),
            path: PathBuf::from("reviewer.md"),
            scope: AgentScope::User,
        };

        agent
            .apply_to_config(&mut config, None)
            .expect("apply to config");
        assert_eq!(config.model.as_deref(), Some("gpt-5"));
        assert_eq!(config.model_reasoning_effort, Some(ReasoningEffort::Medium));

        agent
            .apply_to_config(&mut config, Some("strict"))
            .expect("apply to config");
        assert_eq!(config.model.as_deref(), Some("gpt-4.1"));
        assert_eq!(config.model_reasoning_effort, Some(ReasoningEffort::High));
    }

    #[test]
    fn parses_agent_name_blocks() {
        let content = r#"---
name: reviewer
description: Default reviewer description
model: gpt-5
color: red
agent_names:
  - name: strict
    description: Strict instructions
  - name: lenient
    description: Lenient instructions
---
Instructions for the reviewer

<!-- agent_name: strict -->
Strict instructions

<!-- agent_name: lenient -->
Lenient instructions
"#;
        let temp_dir = TempDir::new().expect("tempdir");
        let path = temp_dir.path().join("reviewer.md");
        fs::write(&path, content).expect("write agent file");

        let agent = parse_agent_file(&path, AgentScope::User).expect("parse agent");
        assert_eq!(agent.name, "reviewer");
        assert_eq!(agent.instructions, "Instructions for the reviewer");
        assert_eq!(agent.agent_name_instructions.len(), 2);
        assert_eq!(agent.agent_name_descriptions.len(), 2);
        assert_eq!(
            agent.agent_name_instructions.get("strict").unwrap(),
            "Strict instructions"
        );
        assert_eq!(
            agent.agent_name_instructions.get("lenient").unwrap(),
            "Lenient instructions"
        );
    }

    #[test]
    fn instructions_for_uses_agent_name_blocks() {
        let content = r#"---
name: reviewer
description: Default description
model: gpt-5
color: red
agent_names:
  - name: strict
    description: Strict variant instructions
---
Default instructions

<!-- agent_name: strict -->
Strict variant instructions
"#;
        let temp_dir = TempDir::new().expect("tempdir");
        let path = temp_dir.path().join("reviewer.md");
        fs::write(&path, content).expect("write agent file");

        let agent = parse_agent_file(&path, AgentScope::User).expect("parse agent");

        assert_eq!(
            agent.instructions_for(None).unwrap(),
            "Default instructions"
        );
        assert_eq!(
            agent.instructions_for(Some("strict")).unwrap(),
            "Strict variant instructions"
        );
        assert!(
            agent.instructions_for(Some("nonexistent")).is_err(),
            "unknown agent_name should error"
        );
    }

    #[test]
    fn agent_name_blocks_require_metadata() {
        let content = r#"---
name: reviewer
description: Default description
model: gpt-5
color: red
---
Default instructions

<!-- agent_name: strict -->
Strict variant instructions
"#;
        let temp_dir = TempDir::new().expect("tempdir");
        let path = temp_dir.path().join("reviewer.md");
        fs::write(&path, content).expect("write agent file");

        let err = parse_agent_file(&path, AgentScope::User).expect_err("parse should fail");
        assert_eq!(
            err.to_string(),
            "invalid agent_names: agent_name blocks require agent_names metadata"
        );
    }

    #[test]
    fn agent_name_metadata_requires_blocks() {
        let content = r#"---
name: reviewer
description: Default description
model: gpt-5
color: red
agent_names:
  - name: strict
    description: Strict variant instructions
---
Default instructions
"#;
        let temp_dir = TempDir::new().expect("tempdir");
        let path = temp_dir.path().join("reviewer.md");
        fs::write(&path, content).expect("write agent file");

        let err = parse_agent_file(&path, AgentScope::User).expect_err("parse should fail");
        assert_eq!(
            err.to_string(),
            "invalid agent_names: agent_names metadata requires agent_name blocks"
        );
    }

    #[test]
    fn seeding_creates_files_only_when_no_codex_files() {
        let temp_dir = TempDir::new().expect("tempdir");
        let codex_home = temp_dir.path();

        // First call should create files
        seed_builtin_agents(codex_home).expect("seed agents");

        let agents_dir = codex_home.join(AGENTS_DIR_NAME);
        assert!(agents_dir.exists());

        // Check that all builtin agents were created
        for (filename, _) in BUILTIN_AGENTS {
            let path = agents_dir.join(format!("{filename}.md"));
            assert!(path.exists(), "Expected {path:?} to exist");
        }

        // Second call should not recreate (files already exist)
        let worker_path = agents_dir.join("codex_worker.md");
        let _original_content = fs::read_to_string(&worker_path).expect("read worker");
        fs::write(&worker_path, "modified content").expect("modify worker");

        seed_builtin_agents(codex_home).expect("seed agents again");

        // Content should still be modified (not overwritten)
        let content_after = fs::read_to_string(&worker_path).expect("read worker again");
        assert_eq!(content_after, "modified content");
    }

    #[test]
    fn seeding_skips_when_any_codex_file_exists() {
        let temp_dir = TempDir::new().expect("tempdir");
        let codex_home = temp_dir.path();
        let agents_dir = codex_home.join(AGENTS_DIR_NAME);
        fs::create_dir_all(&agents_dir).expect("create agents dir");

        // Create a single codex_ file
        fs::write(agents_dir.join("codex_custom.md"), "custom agent").expect("write custom");

        // Seeding should skip
        seed_builtin_agents(codex_home).expect("seed agents");

        // Worker should not exist
        assert!(!agents_dir.join("codex_worker.md").exists());
    }
}
