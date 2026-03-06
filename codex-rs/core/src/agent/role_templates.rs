use crate::config::find_codex_home;
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
    model: Option<String>,
    reasoning_effort: Option<ReasoningEffort>,
    allow_list: Option<Vec<String>>,
    deny_list: Option<Vec<String>>,
    personas: Vec<PersonaTemplate>,
    prompts_by_persona: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct LoadedRoleTemplates {
    templates_by_stem: BTreeMap<String, ParsedRoleTemplate>,
}

#[derive(Debug)]
struct TemplateSource {
    stem: String,
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
    pub(crate) fn load_for_context(
        start_cwd: Option<&Path>,
        codex_home: Option<&Path>,
    ) -> Result<Self, String> {
        let start = match start_cwd {
            Some(path) => path.to_path_buf(),
            None => std::env::current_dir()
                .map_err(|err| format!("failed to resolve current working directory: {err}"))?,
        };
        let project_dirs = discover_project_agent_dirs(&start);
        let user_dir = codex_home
            .map(|path| path.join(".agents"))
            .or_else(|| find_codex_home().ok().map(|path| path.join(".agents")));
        let templates =
            load_templates_from_sources(&project_dirs, user_dir.as_deref(), embedded_templates())?;
        Ok(Self {
            templates_by_stem: templates,
        })
    }

    pub(crate) fn resolve_settings(
        &self,
        role_name: &str,
        agent_persona: Option<&str>,
    ) -> Result<Option<RoleTemplateSettings>, String> {
        let Some(stem) = self.resolve_stem(role_name)? else {
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
        let Some(stem) = self.resolve_stem(role_name)? else {
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

    fn resolve_stem(&self, role_name: &str) -> Result<Option<String>, String> {
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
}

fn discover_project_agent_dirs(start_cwd: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let mut cursor = if start_cwd.is_dir() {
        start_cwd.to_path_buf()
    } else if let Some(parent) = start_cwd.parent() {
        parent.to_path_buf()
    } else {
        return dirs;
    };

    loop {
        let candidate = cursor.join(".codex").join(".agents");
        if candidate.is_dir() {
            dirs.push(candidate);
        }

        let reached_git_root = cursor.join(".git").exists();
        let Some(parent) = cursor.parent() else {
            break;
        };
        if parent == cursor {
            break;
        }
        if reached_git_root {
            break;
        }
        cursor = parent.to_path_buf();
    }

    dirs
}

fn embedded_templates() -> Vec<TemplateSource> {
    vec![
        TemplateSource {
            stem: "default".to_string(),
            source_label: "embedded:default.md".to_string(),
            contents: include_str!("../../templates/agent_roles/default.md").to_string(),
        },
        TemplateSource {
            stem: "explorer".to_string(),
            source_label: "embedded:explorer.md".to_string(),
            contents: include_str!("../../templates/agent_roles/explorer.md").to_string(),
        },
        TemplateSource {
            stem: "worker".to_string(),
            source_label: "embedded:worker.md".to_string(),
            contents: include_str!("../../templates/agent_roles/worker.md").to_string(),
        },
    ]
}

fn load_templates_from_sources(
    project_dirs: &[PathBuf],
    user_dir: Option<&Path>,
    embedded: Vec<TemplateSource>,
) -> Result<BTreeMap<String, ParsedRoleTemplate>, String> {
    let mut ordered_sources = Vec::new();
    for dir in project_dirs {
        ordered_sources.extend(read_templates_from_dir(dir)?);
    }
    if let Some(dir) = user_dir {
        ordered_sources.extend(read_templates_from_dir(dir)?);
    }
    ordered_sources.extend(embedded);

    let mut unique_sources: BTreeMap<String, TemplateSource> = BTreeMap::new();
    for source in ordered_sources {
        unique_sources.entry(source.stem.clone()).or_insert(source);
    }

    let mut parsed = BTreeMap::new();
    for (stem, source) in unique_sources {
        let template = parse_role_template(&source.contents).map_err(|err| {
            format!(
                "invalid agent template '{stem}' from {}: {err}",
                source.source_label
            )
        })?;
        parsed.insert(stem, template);
    }

    Ok(parsed)
}

fn read_templates_from_dir(dir: &Path) -> Result<Vec<TemplateSource>, String> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => {
            return Err(format!(
                "failed to read templates directory {}: {err}",
                dir.display()
            ));
        }
    };

    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect();
    paths.sort();

    let mut templates = Vec::new();
    for path in paths {
        if !path.is_file() {
            continue;
        }
        let Some(extension) = path.extension().and_then(|ext| ext.to_str()) else {
            continue;
        };
        if !extension.eq_ignore_ascii_case("md") {
            continue;
        }

        let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
            return Err(format!(
                "invalid role template file name: {}",
                path.display()
            ));
        };

        let stem = normalize_name(stem);
        if !is_valid_stem(&stem) {
            return Err(format!(
                "invalid agent_type '{stem}' in {} (expected snake_case or kebab-case)",
                path.display()
            ));
        }

        let contents = fs::read_to_string(&path)
            .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
        templates.push(TemplateSource {
            stem,
            source_label: path.display().to_string(),
            contents,
        });
    }

    Ok(templates)
}

fn parse_role_template(contents: &str) -> Result<ParsedRoleTemplate, String> {
    let (frontmatter, body) = split_frontmatter(contents)?;
    let parsed: RoleFrontmatter =
        serde_yaml::from_str(&frontmatter).map_err(|err| format!("invalid YAML: {err}"))?;

    let description = parsed.description.trim().to_string();
    if description.is_empty() {
        return Err("missing required frontmatter field 'description'".to_string());
    }

    if parsed.agent_names.is_empty() {
        return Err("agent_names must contain at least one persona".to_string());
    }

    let mut seen_personas = BTreeSet::new();
    let mut personas = Vec::new();
    for entry in parsed.agent_names {
        let name = normalize_name(&entry.name);
        if name.is_empty() {
            return Err("agent_names[].name must be non-empty".to_string());
        }
        if !seen_personas.insert(name.clone()) {
            return Err(format!("duplicate agent_persona '{name}' in agent_names"));
        }

        let persona_description = entry.description.trim().to_string();
        if persona_description.is_empty() {
            return Err(format!(
                "agent_names[].description must be non-empty for agent_persona '{name}'"
            ));
        }

        personas.push(PersonaTemplate {
            name,
            description: persona_description,
            model: entry
                .model
                .map(|model| model.trim().to_string())
                .filter(|model| !model.is_empty()),
            reasoning_effort: entry.reasoning_effort,
        });
    }

    if !seen_personas.contains(DEFAULT_AGENT_PERSONA) {
        return Err(format!(
            "agent_names must include required agent_persona '{DEFAULT_AGENT_PERSONA}'"
        ));
    }

    let prompts_by_persona = parse_prompt_blocks(&body)?;
    if prompts_by_persona.is_empty() {
        return Err(
            "template body must define at least one agent_nickname prompt block".to_string(),
        );
    }

    for persona in &personas {
        let Some(prompt) = prompts_by_persona.get(&persona.name) else {
            return Err(format!(
                "missing prompt block for agent_persona '{}'",
                persona.name
            ));
        };
        if prompt.trim().is_empty() {
            return Err(format!(
                "prompt block for agent_persona '{}' must be non-empty",
                persona.name
            ));
        }
    }

    for prompt_persona in prompts_by_persona.keys() {
        if !seen_personas.contains(prompt_persona) {
            return Err(format!(
                "prompt block references undeclared agent_persona '{prompt_persona}'"
            ));
        }
    }

    Ok(ParsedRoleTemplate {
        description,
        read_only: parsed.read_only,
        model: parsed
            .model
            .map(|model| model.trim().to_string())
            .filter(|model| !model.is_empty()),
        reasoning_effort: parsed.reasoning_effort,
        allow_list: normalize_tool_list(parsed.allow_list),
        deny_list: normalize_tool_list(parsed.deny_list),
        personas,
        prompts_by_persona,
    })
}

fn split_frontmatter(contents: &str) -> Result<(String, String), String> {
    let mut lines = contents.lines();
    let Some(first) = lines.next() else {
        return Err("template is empty".to_string());
    };
    if first.trim() != "---" {
        return Err("missing YAML frontmatter opening delimiter '---'".to_string());
    }

    let mut frontmatter_lines = Vec::new();
    let mut found_closing = false;
    for line in lines.by_ref() {
        if line.trim() == "---" {
            found_closing = true;
            break;
        }
        frontmatter_lines.push(line);
    }

    if !found_closing {
        return Err("missing YAML frontmatter closing delimiter '---'".to_string());
    }

    let frontmatter = frontmatter_lines.join("\n");
    let body = lines.collect::<Vec<_>>().join("\n");
    Ok((frontmatter, body))
}

fn parse_prompt_blocks(body: &str) -> Result<BTreeMap<String, String>, String> {
    let mut current_persona: Option<String> = None;
    let mut prompts: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for line in body.lines() {
        if let Some(persona) = agent_nickname_marker(line) {
            if prompts.contains_key(&persona) {
                return Err(format!(
                    "duplicate prompt block marker for agent_persona '{persona}'"
                ));
            }
            prompts.insert(persona.clone(), Vec::new());
            current_persona = Some(persona);
            continue;
        }

        match current_persona.as_ref() {
            Some(persona) => {
                let Some(lines) = prompts.get_mut(persona) else {
                    return Err(format!(
                        "missing prompt block container for agent_persona '{persona}'"
                    ));
                };
                lines.push(line.to_string());
            }
            None => {
                if !line.trim().is_empty() {
                    return Err(
                        "template body must use <!-- agent_nickname: <name> --> blocks for prompts"
                            .to_string(),
                    );
                }
            }
        }
    }

    Ok(prompts
        .into_iter()
        .map(|(persona, lines)| (persona, lines.join("\n").trim().to_string()))
        .collect())
}

fn agent_nickname_marker(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let prefix = "<!-- agent_nickname:";
    if !trimmed.starts_with(prefix) || !trimmed.ends_with("-->") {
        return None;
    }

    let inner = trimmed.strip_prefix(prefix)?.strip_suffix("-->")?.trim();
    if inner.is_empty() {
        return None;
    }

    Some(normalize_name(inner))
}

fn normalize_name(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn canonical_stem(stem: &str) -> String {
    stem.replace('-', "_")
}

fn normalize_tool_list(input: Option<Vec<String>>) -> Option<Vec<String>> {
    let mut items: Vec<String> = input
        .unwrap_or_default()
        .into_iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect();
    items.sort();
    items.dedup();
    (!items.is_empty()).then_some(items)
}

fn is_valid_stem(stem: &str) -> bool {
    if stem.is_empty() {
        return false;
    }

    stem.chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;

    const VALID_TEMPLATE: &str = r#"---
description: Explorer role
agent_names:
  - name: default
    description: default persona
  - name: runner
    description: runner persona
model: gpt-5
reasoning_effort: high
allow_list:
  - shell
  - web.search
deny_list:
  - exec_command
---
<!-- agent_nickname: default -->
default prompt
<!-- agent_nickname: runner -->
runner prompt
"#;

    #[test]
    fn parse_role_template_requires_default_persona() {
        let template = r#"---
description: Test role
agent_names:
  - name: runner
    description: runner persona
---
<!-- agent_nickname: runner -->
runner prompt
"#;

        let err = parse_role_template(template).expect_err("must fail");
        assert!(err.contains("required agent_persona 'default'"));
    }

    #[test]
    fn parse_role_template_rejects_unknown_frontmatter_field() {
        let template = r#"---
description: Test role
unsupported: true
agent_names:
  - name: default
    description: default persona
---
<!-- agent_nickname: default -->
default prompt
"#;

        let err = parse_role_template(template).expect_err("must fail");
        assert!(err.contains("unknown field"));
    }

    #[test]
    fn parse_role_template_accepts_read_only_metadata() {
        let template = r#"---
description: Test role
read_only: true
agent_names:
  - name: default
    description: default persona
---
<!-- agent_nickname: default -->
default prompt
"#;

        let parsed = parse_role_template(template).expect("must parse");
        assert_eq!(parsed.read_only, true);
    }

    #[test]
    fn parse_role_template_requires_persona_descriptions() {
        let template = r#"---
description: Test role
agent_names:
  - name: default
    description: ""
---
<!-- agent_nickname: default -->
default prompt
"#;

        let err = parse_role_template(template).expect_err("must fail");
        assert!(err.contains("agent_names[].description must be non-empty"));
    }

    #[test]
    fn parse_role_template_requires_declared_prompt_blocks() {
        let template = r#"---
description: Test role
agent_names:
  - name: default
    description: default persona
  - name: runner
    description: runner persona
---
<!-- agent_nickname: default -->
default prompt
"#;

        let err = parse_role_template(template).expect_err("must fail");
        assert!(err.contains("missing prompt block for agent_persona 'runner'"));
    }

    #[test]
    fn parse_role_template_rejects_prompt_text_outside_blocks() {
        let template = r#"---
description: Test role
agent_names:
  - name: default
    description: default persona
---
stray prompt
<!-- agent_nickname: default -->
default prompt
"#;

        let err = parse_role_template(template).expect_err("must fail");
        assert!(err.contains("template body must use <!-- agent_nickname"));
    }

    #[test]
    fn resolve_settings_defaults_to_default_persona() {
        let parsed = parse_role_template(
            r#"---
description: Worker role
agent_names:
  - name: default
    description: default persona
  - name: runner
    description: runner persona
---
<!-- agent_nickname: default -->
default prompt
<!-- agent_nickname: runner -->
runner prompt
"#,
        )
        .expect("template should parse");

        let mut templates = BTreeMap::new();
        templates.insert("worker".to_string(), parsed);
        let loaded = LoadedRoleTemplates {
            templates_by_stem: templates,
        };

        let settings = loaded
            .resolve_settings("worker", None)
            .expect("resolve should succeed")
            .expect("template should exist");
        assert_eq!(settings.agent_persona, "default".to_string());
        assert_eq!(settings.read_only, false);
        assert_eq!(settings.instructions, "default prompt".to_string());
    }

    #[test]
    fn resolve_settings_rejects_unknown_persona() {
        let parsed = parse_role_template(
            r#"---
description: Worker role
agent_names:
  - name: default
    description: default persona
---
<!-- agent_nickname: default -->
default prompt
"#,
        )
        .expect("template should parse");
        let mut templates = BTreeMap::new();
        templates.insert("worker".to_string(), parsed);
        let loaded = LoadedRoleTemplates {
            templates_by_stem: templates,
        };

        let err = loaded
            .resolve_settings("worker", Some("runner"))
            .expect_err("must fail");
        assert!(err.contains("unknown agent_persona 'runner'"));
    }

    #[test]
    fn resolve_settings_requires_template_when_persona_requested() {
        let loaded = LoadedRoleTemplates::default();

        let err = loaded
            .resolve_settings("worker", Some("runner"))
            .expect_err("must fail");
        assert!(err.contains("requires a role template"));
    }

    #[test]
    fn resolve_settings_prefers_persona_model_and_normalizes_policy_lists() {
        let parsed = parse_role_template(VALID_TEMPLATE).expect("template should parse");
        let mut templates = BTreeMap::new();
        templates.insert("worker".to_string(), parsed);
        let loaded = LoadedRoleTemplates {
            templates_by_stem: templates,
        };

        let settings = loaded
            .resolve_settings("worker", Some("runner"))
            .expect("resolve should succeed")
            .expect("template should exist");
        assert_eq!(settings.agent_persona, "runner".to_string());
        assert_eq!(settings.read_only, false);
        assert_eq!(settings.instructions, "runner prompt".to_string());
        assert_eq!(settings.model, Some("gpt-5".to_string()));
        assert_eq!(settings.reasoning_effort, Some(ReasoningEffort::High));
        assert_eq!(
            settings.allow_list,
            Some(vec!["shell".to_string(), "web.search".to_string()])
        );
        assert_eq!(settings.deny_list, Some(vec!["exec_command".to_string()]));
    }

    #[test]
    fn load_for_context_prefers_project_then_user_then_embedded() {
        let temp = TempDir::new().expect("temp dir");
        let repo_root = temp.path().join("repo");
        let project_agents = repo_root.join(".codex").join(".agents");
        let user_home = temp.path().join("codex-home");
        let user_agents = user_home.join(".agents");
        fs::create_dir_all(&project_agents).expect("create project agents");
        fs::create_dir_all(&user_agents).expect("create user agents");
        fs::create_dir_all(repo_root.join("nested")).expect("create nested dir");
        fs::write(repo_root.join(".git"), "").expect("create git sentinel");
        fs::write(
            project_agents.join("worker.md"),
            r#"---
description: Project worker
agent_names:
  - name: default
    description: project default
---
<!-- agent_nickname: default -->
project prompt
"#,
        )
        .expect("write project template");
        fs::write(
            user_agents.join("worker.md"),
            r#"---
description: User worker
agent_names:
  - name: default
    description: user default
---
<!-- agent_nickname: default -->
user prompt
"#,
        )
        .expect("write user template");

        let loaded = LoadedRoleTemplates::load_for_context(
            Some(repo_root.join("nested").as_path()),
            Some(user_home.as_path()),
        )
        .expect("load templates");
        let settings = loaded
            .resolve_settings("worker", None)
            .expect("resolve should succeed")
            .expect("template should exist");

        assert_eq!(settings.instructions, "project prompt".to_string());
    }

    #[test]
    fn resolve_settings_supports_case_insensitive_role_lookup() {
        let parsed = parse_role_template(
            r#"---
description: Worker role
agent_names:
  - name: default
    description: default persona
---
<!-- agent_nickname: default -->
default prompt
"#,
        )
        .expect("template should parse");
        let mut templates = BTreeMap::new();
        templates.insert("worker-runner".to_string(), parsed);
        let loaded = LoadedRoleTemplates {
            templates_by_stem: templates,
        };

        let settings = loaded
            .resolve_settings("Worker_Runner", None)
            .expect("resolve should succeed")
            .expect("template should exist");

        assert_eq!(settings.agent_persona, "default".to_string());
    }

    #[test]
    fn role_metadata_reports_personas_and_read_only() {
        let template = r#"---
description: Worker role
read_only: true
agent_names:
  - name: default
    description: default persona
  - name: reviewer
    description: reviewer persona
---
<!-- agent_nickname: default -->
default prompt
<!-- agent_nickname: reviewer -->
reviewer prompt
"#;
        let temp_dir = TempDir::new().expect("create tempdir");
        let agents_dir = temp_dir.path().join(".codex").join(".agents");
        fs::create_dir_all(&agents_dir).expect("create agents dir");
        fs::write(agents_dir.join("worker.md"), template).expect("write template");

        let templates = LoadedRoleTemplates::load_for_context(Some(temp_dir.path()), None)
            .expect("templates should load");
        let metadata = templates
            .role_metadata("worker")
            .expect("lookup should succeed")
            .expect("metadata should exist");
        assert_eq!(
            metadata,
            RoleDiscoveryMetadata {
                description: "Worker role".to_string(),
                read_only: true,
                agent_personas: vec![
                    RolePersonaDiscovery {
                        name: "default".to_string(),
                        description: "default persona".to_string(),
                    },
                    RolePersonaDiscovery {
                        name: "reviewer".to_string(),
                        description: "reviewer persona".to_string(),
                    },
                ],
            }
        );
    }
}
