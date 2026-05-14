use crate::config::Config;
use codex_config::ConfigLayerStackOrdering;
use codex_protocol::openai_models::ReasoningEffort;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

pub(crate) const DEFAULT_AGENT_PERSONA: &str = "default";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RoleTemplateSettings {
    pub(crate) agent_persona: String,
    pub(crate) instructions: String,
    pub(crate) base_instructions: Option<String>,
    pub(crate) read_only: bool,
    pub(crate) model: Option<String>,
    pub(crate) reasoning_effort: Option<ReasoningEffort>,
    pub(crate) allow_list: Option<Vec<String>>,
    pub(crate) deny_list: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RolePersonaDiscovery {
    pub(crate) name: String,
    pub(crate) description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RoleDiscoveryMetadata {
    pub(crate) description: String,
    pub(crate) read_only: bool,
    pub(crate) agent_personas: Vec<RolePersonaDiscovery>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PersonaTemplate {
    name: String,
    description: String,
    model: Option<String>,
    reasoning_effort: Option<ReasoningEffort>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedRoleTemplate {
    description: String,
    read_only: bool,
    base_instructions: Option<String>,
    model: Option<String>,
    reasoning_effort: Option<ReasoningEffort>,
    allow_list: Option<Vec<String>>,
    deny_list: Option<Vec<String>>,
    personas: Vec<PersonaTemplate>,
    prompts_by_persona: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct LoadedRoleTemplates {
    templates_by_stem: BTreeMap<String, ParsedRoleTemplate>,
    errors_by_stem: BTreeMap<String, String>,
}

#[derive(Debug)]
struct TemplateSource {
    stem: String,
    path: PathBuf,
    source_label: String,
    contents: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RoleFrontmatter {
    description: String,
    #[serde(default)]
    read_only: bool,
    agent_names: Vec<PersonaFrontmatter>,
    model_instructions_file: Option<PathBuf>,
    model: Option<String>,
    reasoning_effort: Option<ReasoningEffort>,
    allow_list: Option<Vec<String>>,
    deny_list: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PersonaFrontmatter {
    name: String,
    description: String,
    model: Option<String>,
    reasoning_effort: Option<ReasoningEffort>,
}

impl LoadedRoleTemplates {
    pub(crate) fn load_for_config(config: &Config) -> Self {
        let mut ordered_dirs = Vec::new();
        let mut seen_dirs = BTreeSet::new();
        let mut config_folders = Vec::new();
        for layer in config.config_layer_stack.get_layers(
            ConfigLayerStackOrdering::HighestPrecedenceFirst,
            /*include_disabled*/ false,
        ) {
            let Some(config_folder) = layer.config_folder() else {
                continue;
            };
            config_folders.push(config_folder);
        }
        for config_folder in &config_folders {
            let dir = config_folder.as_path().join(".agents");
            if seen_dirs.insert(dir.clone()) {
                ordered_dirs.push(dir);
            }
        }

        let mut unique_sources: BTreeMap<String, TemplateSource> = BTreeMap::new();
        for dir in ordered_dirs {
            let Ok(sources) = read_templates_from_dir(&dir) else {
                continue;
            };
            for source in sources {
                unique_sources.entry(source.stem.clone()).or_insert(source);
            }
        }

        let mut templates_by_stem = BTreeMap::new();
        let mut errors_by_stem = BTreeMap::new();
        for (stem, source) in unique_sources {
            match parse_role_template(&source.contents, &source.path) {
                Ok(template) => {
                    templates_by_stem.insert(stem, template);
                }
                Err(err) => {
                    errors_by_stem.insert(stem.clone(), err.clone());
                    tracing::warn!(
                        "Ignoring malformed agent template: invalid agent template '{}' from {}: {err}",
                        stem,
                        source.source_label
                    );
                }
            }
        }

        Self {
            templates_by_stem,
            errors_by_stem,
        }
    }

    pub(crate) fn role_names(&self) -> impl Iterator<Item = &String> {
        self.templates_by_stem.keys()
    }

    pub(crate) fn resolve_role_name(&self, role_name: &str) -> Result<Option<String>, String> {
        let normalized = normalize_name(role_name);
        if normalized.is_empty() {
            return Ok(None);
        }
        if self.templates_by_stem.contains_key(&normalized) {
            return Ok(Some(normalized));
        }

        let canonical = canonical_stem(&normalized);
        let mut candidates: Vec<String> = self
            .templates_by_stem
            .keys()
            .filter(|stem| canonical_stem(stem) == canonical)
            .cloned()
            .collect();
        candidates.sort();
        candidates.dedup();

        match candidates.len() {
            0 => Ok(None),
            1 => Ok(Some(candidates.remove(0))),
            _ => Err(format!(
                "ambiguous agent_type '{role_name}'; matched multiple templates: {}",
                candidates.join(", ")
            )),
        }
    }

    fn template_error_for_role(&self, role_name: &str) -> Option<String> {
        let normalized = normalize_name(role_name);
        let canonical = canonical_stem(&normalized);
        self.errors_by_stem
            .iter()
            .find(|(stem, _)| *stem == &normalized || canonical_stem(stem) == canonical)
            .map(|(stem, err)| format!("invalid agent template '{stem}': {err}"))
    }

    pub(crate) fn resolve_settings(
        &self,
        role_name: &str,
        agent_persona: Option<&str>,
    ) -> Result<Option<RoleTemplateSettings>, String> {
        let Some(stem) = self.resolve_role_name(role_name)? else {
            if let Some(err) = self.template_error_for_role(role_name) {
                return Err(err);
            }
            return if let Some(agent_persona) = agent_persona {
                Err(format!(
                    "agent_persona '{agent_persona}' requires a role template for agent_type '{role_name}'"
                ))
            } else {
                Ok(None)
            };
        };
        let Some(template) = self.templates_by_stem.get(&stem) else {
            return Ok(None);
        };

        let requested_persona = agent_persona
            .map(normalize_name)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| DEFAULT_AGENT_PERSONA.to_string());
        let requested_persona_canonical = canonical_stem(&requested_persona);

        let Some(persona) = template
            .personas
            .iter()
            .find(|persona| canonical_stem(&persona.name) == requested_persona_canonical)
        else {
            return Err(format!(
                "unknown agent_persona '{requested_persona}' for agent_type '{role_name}'"
            ));
        };

        let instructions = template
            .prompts_by_persona
            .get(&persona.name)
            .cloned()
            .ok_or_else(|| {
                format!(
                    "missing prompt block for agent_persona '{}' in agent_type '{role_name}'",
                    persona.name
                )
            })?;

        Ok(Some(RoleTemplateSettings {
            agent_persona: persona.name.clone(),
            instructions,
            base_instructions: template.base_instructions.clone(),
            read_only: template.read_only,
            model: persona.model.clone().or_else(|| template.model.clone()),
            reasoning_effort: persona.reasoning_effort.or(template.reasoning_effort),
            allow_list: template.allow_list.clone(),
            deny_list: template.deny_list.clone(),
        }))
    }

    pub(crate) fn role_metadata(
        &self,
        role_name: &str,
    ) -> Result<Option<RoleDiscoveryMetadata>, String> {
        let Some(stem) = self.resolve_role_name(role_name)? else {
            return Ok(None);
        };
        let Some(template) = self.templates_by_stem.get(&stem) else {
            return Ok(None);
        };

        Ok(Some(RoleDiscoveryMetadata {
            description: template.description.clone(),
            read_only: template.read_only,
            agent_personas: template
                .personas
                .iter()
                .map(|persona| RolePersonaDiscovery {
                    name: persona.name.clone(),
                    description: persona.description.clone(),
                })
                .collect(),
        }))
    }
}

fn read_templates_from_dir(dir: &Path) -> Result<Vec<TemplateSource>, String> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(format!("failed to read {}: {err}", dir.display())),
    };

    let mut sources = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|err| format!("failed to read {}: {err}", dir.display()))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let contents = fs::read_to_string(&path)
            .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
        if !looks_like_role_template(&contents) {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let stem = normalize_name(stem);
        if stem.is_empty() {
            return Err(format!(
                "invalid agent template filename {}",
                path.display()
            ));
        }
        sources.push(TemplateSource {
            stem,
            source_label: path.display().to_string(),
            path,
            contents,
        });
    }
    sources.sort_by(|left, right| left.stem.cmp(&right.stem));
    Ok(sources)
}

fn looks_like_role_template(contents: &str) -> bool {
    contents
        .lines()
        .find(|line| !line.trim().is_empty())
        .is_some_and(|line| line.trim() == "---")
}

fn parse_role_template(contents: &str, path: &Path) -> Result<ParsedRoleTemplate, String> {
    let (frontmatter, body) = split_frontmatter(contents)?;
    let frontmatter: RoleFrontmatter =
        serde_yaml::from_str(frontmatter).map_err(|err| format!("invalid frontmatter: {err}"))?;

    if frontmatter.agent_names.is_empty() {
        return Err("agent_names must contain at least one persona".to_string());
    }
    let description = frontmatter.description.trim().to_string();
    if description.is_empty() {
        return Err("description must not be empty".to_string());
    }

    let mut seen_personas = BTreeSet::new();
    let mut personas = Vec::new();
    for persona in frontmatter.agent_names {
        let name = parse_declared_persona_name(&persona.name)?;
        if !seen_personas.insert(name.clone()) {
            return Err(format!("duplicate agent_persona '{name}'"));
        }
        let description = persona.description.trim().to_string();
        if description.is_empty() {
            return Err(format!("agent_persona '{name}' must have a description"));
        }
        personas.push(PersonaTemplate {
            name,
            description,
            model: persona.model,
            reasoning_effort: persona.reasoning_effort,
        });
    }
    if !seen_personas.contains(DEFAULT_AGENT_PERSONA) {
        return Err("agent_names must include a default persona".to_string());
    }

    let prompts_by_persona = parse_prompt_blocks(body)?;
    for persona in &personas {
        match prompts_by_persona.get(&persona.name) {
            Some(prompt) if !prompt.trim().is_empty() => {}
            _ => {
                return Err(format!(
                    "missing prompt block for agent_persona '{}'",
                    persona.name
                ));
            }
        }
    }
    for prompt_name in prompts_by_persona.keys() {
        if !seen_personas.contains(prompt_name) {
            return Err(format!(
                "prompt block declares unknown agent_persona '{prompt_name}'"
            ));
        }
    }

    let base_instructions = match frontmatter.model_instructions_file {
        Some(relative_path) => {
            let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
            let path = base_dir.join(relative_path);
            let contents = fs::read_to_string(&path)
                .map_err(|err| format!("failed to read model_instructions_file: {err}"))?;
            let trimmed = contents.trim().to_string();
            if trimmed.is_empty() {
                return Err("model_instructions_file must not be empty".to_string());
            }
            Some(trimmed)
        }
        None => None,
    };

    Ok(ParsedRoleTemplate {
        description,
        read_only: frontmatter.read_only,
        base_instructions,
        model: frontmatter.model,
        reasoning_effort: frontmatter.reasoning_effort,
        allow_list: normalize_patterns(frontmatter.allow_list),
        deny_list: normalize_patterns(frontmatter.deny_list),
        personas,
        prompts_by_persona,
    })
}

fn split_frontmatter(contents: &str) -> Result<(&str, &str), String> {
    let mut lines = contents.lines();
    let Some(first_line) = lines.next() else {
        return Err("template is empty".to_string());
    };
    if first_line.trim() != "---" {
        return Err("template must start with YAML frontmatter".to_string());
    }

    let frontmatter_start = first_line.len() + 1;
    let mut byte_offset = frontmatter_start;
    for line in lines {
        if line.trim() == "---" {
            let frontmatter = &contents[frontmatter_start..byte_offset];
            let body_start = byte_offset + line.len() + 1;
            let body = contents.get(body_start..).unwrap_or_default();
            return Ok((frontmatter, body));
        }
        byte_offset += line.len() + 1;
    }
    Err("frontmatter closing marker is missing".to_string())
}

fn parse_prompt_blocks(body: &str) -> Result<BTreeMap<String, String>, String> {
    let mut prompts = BTreeMap::new();
    let mut current_persona: Option<String> = None;
    let mut current_lines = Vec::new();
    let mut stray = Vec::new();

    for line in body.lines() {
        if let Some(persona) = parse_agent_nickname_marker(line) {
            flush_prompt(&mut prompts, current_persona.take(), &mut current_lines)?;
            current_persona = Some(persona);
            continue;
        }
        if current_persona.is_some() {
            current_lines.push(line.to_string());
        } else if !line.trim().is_empty() {
            stray.push(line.trim().to_string());
        }
    }
    flush_prompt(&mut prompts, current_persona, &mut current_lines)?;

    if !stray.is_empty() {
        return Err("markdown outside agent_nickname blocks is not allowed".to_string());
    }
    Ok(prompts)
}

fn parse_agent_nickname_marker(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let name = trimmed
        .strip_prefix("<!--")
        .and_then(|value| value.strip_suffix("-->"))?
        .trim()
        .strip_prefix("agent_nickname:")?
        .trim();
    let normalized = normalize_name(name);
    (!normalized.is_empty()).then_some(normalized)
}

fn parse_declared_persona_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    let normalized = normalize_name(trimmed);
    if normalized.is_empty() {
        return Err("agent_names contains an empty name".to_string());
    }
    if normalized != trimmed {
        return Err(format!(
            "agent_persona '{trimmed}' must contain only lowercase letters, digits, '-' or '_'"
        ));
    }
    Ok(normalized)
}

fn flush_prompt(
    prompts: &mut BTreeMap<String, String>,
    persona: Option<String>,
    lines: &mut Vec<String>,
) -> Result<(), String> {
    let Some(persona) = persona else {
        return Ok(());
    };
    let prompt = lines.join("\n").trim().to_string();
    lines.clear();
    if prompt.is_empty() {
        return Err(format!(
            "prompt block for agent_persona '{persona}' is empty"
        ));
    }
    if prompts.insert(persona.clone(), prompt).is_some() {
        return Err(format!(
            "duplicate prompt block for agent_persona '{persona}'"
        ));
    }
    Ok(())
}

fn normalize_patterns(patterns: Option<Vec<String>>) -> Option<Vec<String>> {
    patterns
        .map(|patterns| {
            patterns
                .into_iter()
                .map(|pattern| pattern.trim().to_string())
                .filter(|pattern| !pattern.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|patterns| !patterns.is_empty())
}

fn normalize_name(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '-')
        .collect()
}

fn canonical_stem(value: &str) -> String {
    value.replace('-', "_")
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn parses_default_and_named_personas() {
        let template = parse_role_template(
            r#"---
description: Test role
agent_names:
  - name: default
    description: Default persona
  - name: auditor
    description: Audit persona
model: gpt-test
allow_list:
  - tool_*
---
<!-- agent_nickname: default -->
Default prompt

<!-- agent_nickname: auditor -->
Audit prompt
"#,
            Path::new("role.md"),
        )
        .expect("parse template");

        assert_eq!(template.description, "Test role");
        assert_eq!(template.personas[1].name, "auditor");
        assert_eq!(
            template.prompts_by_persona.get("auditor"),
            Some(&"Audit prompt".to_string())
        );
        assert_eq!(template.allow_list, Some(vec!["tool_*".to_string()]));
    }

    #[test]
    fn rejects_stray_markdown() {
        let err = parse_role_template(
            r#"---
description: Test role
agent_names:
  - name: default
    description: Default persona
---
Stray text
<!-- agent_nickname: default -->
Default prompt
"#,
            Path::new("role.md"),
        )
        .expect_err("reject stray markdown");

        assert_eq!(err, "markdown outside agent_nickname blocks is not allowed");
    }

    #[test]
    fn rejects_empty_description_and_invalid_persona_name() {
        let err = parse_role_template(
            r#"---
description: " "
agent_names:
  - name: default
    description: Default persona
---
<!-- agent_nickname: default -->
Default prompt
"#,
            Path::new("role.md"),
        )
        .expect_err("reject empty description");
        assert_eq!(err, "description must not be empty");

        let err = parse_role_template(
            r#"---
description: Test role
agent_names:
  - name: "bad name!"
    description: Bad persona
  - name: default
    description: Default persona
---
<!-- agent_nickname: badname -->
Bad prompt

<!-- agent_nickname: default -->
Default prompt
"#,
            Path::new("role.md"),
        )
        .expect_err("reject invalid persona name");
        assert_eq!(
            err,
            "agent_persona 'bad name!' must contain only lowercase letters, digits, '-' or '_'"
        );
    }

    #[test]
    fn resolves_canonical_underscore_dash_fallback() {
        let mut templates = LoadedRoleTemplates::default();
        templates.templates_by_stem.insert(
            "code-review".to_string(),
            ParsedRoleTemplate {
                description: "Review".to_string(),
                read_only: false,
                base_instructions: None,
                model: None,
                reasoning_effort: None,
                allow_list: None,
                deny_list: None,
                personas: vec![PersonaTemplate {
                    name: DEFAULT_AGENT_PERSONA.to_string(),
                    description: "Default".to_string(),
                    model: None,
                    reasoning_effort: None,
                }],
                prompts_by_persona: BTreeMap::from([(
                    DEFAULT_AGENT_PERSONA.to_string(),
                    "prompt".to_string(),
                )]),
            },
        );

        assert_eq!(
            templates.resolve_role_name("code_review"),
            Ok(Some("code-review".to_string()))
        );
    }
}
