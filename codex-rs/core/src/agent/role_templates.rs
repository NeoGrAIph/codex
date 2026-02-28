use crate::config::find_codex_home;
use codex_protocol::openai_models::ReasoningEffort;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

pub(crate) const DEFAULT_AGENT_NICKNAME: &str = "default";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RoleTemplateModelSource {
    Nickname,
    Role,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentNicknameDiscovery {
    pub(crate) name: String,
    pub(crate) description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RoleDiscoveryMetadata {
    pub(crate) description: String,
    pub(crate) read_only: bool,
    pub(crate) agent_nicknames: Vec<AgentNicknameDiscovery>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RoleSpawnSettings {
    pub(crate) agent_nickname: String,
    pub(crate) agent_persona: String,
    pub(crate) instructions: String,
    pub(crate) model: Option<String>,
    pub(crate) model_source: Option<RoleTemplateModelSource>,
    pub(crate) reasoning_effort: Option<ReasoningEffort>,
    pub(crate) read_only: bool,
    pub(crate) allow_list: Option<Vec<String>>,
    pub(crate) deny_list: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NicknameTemplate {
    name: String,
    display_name: String,
    description: String,
    model: Option<String>,
    reasoning_effort: Option<ReasoningEffort>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedRoleTemplate {
    role_name: String,
    description: String,
    read_only: bool,
    model: Option<String>,
    reasoning_effort: Option<ReasoningEffort>,
    allow_list: Option<Vec<String>>,
    deny_list: Option<Vec<String>>,
    nicknames: Vec<NicknameTemplate>,
    prompts_by_nickname: BTreeMap<String, String>,
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
    agent_names: Vec<NicknameFrontmatter>,
    model: Option<String>,
    reasoning_effort: Option<ReasoningEffort>,
    allow_list: Option<Vec<String>>,
    deny_list: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NicknameFrontmatter {
    name: String,
    description: String,
    model: Option<String>,
    reasoning_effort: Option<ReasoningEffort>,
}

impl LoadedRoleTemplates {
    pub(crate) fn load() -> Result<Self, String> {
        Self::load_for_context(None, None)
    }

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
            agent_nicknames: template
                .nicknames
                .iter()
                .map(|nickname| AgentNicknameDiscovery {
                    name: nickname.name.clone(),
                    description: nickname.description.clone(),
                })
                .collect(),
        }))
    }

    pub(crate) fn resolve_role_name(&self, role_name: &str) -> Result<Option<String>, String> {
        self.resolve_stem(role_name)
    }

    pub(crate) fn role_names(&self) -> Vec<String> {
        self.templates_by_stem.keys().cloned().collect()
    }

    pub(crate) fn resolve_spawn_settings(
        &self,
        role_name: &str,
        agent_nickname: Option<&str>,
    ) -> Result<Option<RoleSpawnSettings>, String> {
        let Some(stem) = self.resolve_stem(role_name)? else {
            return Ok(None);
        };
        let Some(template) = self.templates_by_stem.get(&stem) else {
            return Ok(None);
        };
        let requested_nickname = agent_nickname
            .map(normalize_role_or_nickname)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| DEFAULT_AGENT_NICKNAME.to_string());

        let Some(nickname) = template
            .nicknames
            .iter()
            .find(|nickname| nickname.name == requested_nickname)
        else {
            return Err(format!(
                "unknown agent_nickname '{requested_nickname}' for agent_type '{role_name}'"
            ));
        };

        let instructions = template
            .prompts_by_nickname
            .get(&requested_nickname)
            .cloned()
            .ok_or_else(|| {
                format!(
                    "missing prompt block for agent_nickname '{requested_nickname}' in agent_type '{role_name}'"
                )
            })?;

        let (model, model_source) = if let Some(model) = nickname.model.clone() {
            (Some(model), Some(RoleTemplateModelSource::Nickname))
        } else if let Some(model) = template.model.clone() {
            (Some(model), Some(RoleTemplateModelSource::Role))
        } else {
            (None, None)
        };

        Ok(Some(RoleSpawnSettings {
            agent_nickname: requested_nickname,
            agent_persona: nickname.display_name.clone(),
            instructions,
            model,
            model_source,
            reasoning_effort: nickname.reasoning_effort.or(template.reasoning_effort),
            read_only: template.read_only,
            allow_list: template.allow_list.clone(),
            deny_list: template.deny_list.clone(),
        }))
    }

    fn resolve_stem(&self, role_name: &str) -> Result<Option<String>, String> {
        let normalized = normalize_role_or_nickname(role_name);
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
        TemplateSource {
            stem: "awaiter".to_string(),
            source_label: "embedded:awaiter.md".to_string(),
            contents: include_str!("../../templates/agent_roles/awaiter.md").to_string(),
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
        let template = parse_role_template(&stem, &source.contents).map_err(|err| {
            format!(
                "invalid agent template '{}' from {}: {}",
                stem, source.source_label, err
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

        let stem = normalize_role_or_nickname(stem);
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

fn parse_role_template(stem: &str, contents: &str) -> Result<ParsedRoleTemplate, String> {
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

    let mut seen_nicknames = BTreeSet::new();
    let mut nicknames = Vec::new();
    for entry in parsed.agent_names {
        let name = normalize_role_or_nickname(&entry.name);
        if name.is_empty() {
            return Err("agent_names[].name must be non-empty".to_string());
        }
        if !seen_nicknames.insert(name.clone()) {
            return Err(format!("duplicate agent_nickname '{name}' in agent_names"));
        }

        let nickname_description = entry.description.trim().to_string();
        if nickname_description.is_empty() {
            return Err(format!(
                "agent_names[].description must be non-empty for agent_nickname '{name}'"
            ));
        }

        nicknames.push(NicknameTemplate {
            name,
            display_name: entry.name.trim().to_string(),
            description: nickname_description,
            model: entry
                .model
                .map(|model| model.trim().to_string())
                .filter(|model| !model.is_empty()),
            reasoning_effort: entry.reasoning_effort,
        });
    }

    if !seen_nicknames.contains(DEFAULT_AGENT_NICKNAME) {
        return Err(format!(
            "agent_names must include required agent_nickname '{DEFAULT_AGENT_NICKNAME}'"
        ));
    }

    let prompts_by_nickname = parse_prompt_blocks(&body)?;
    if prompts_by_nickname.is_empty() {
        return Err(
            "template body must define at least one agent_nickname prompt block".to_string(),
        );
    }

    for nickname in &nicknames {
        let Some(prompt) = prompts_by_nickname.get(&nickname.name) else {
            return Err(format!(
                "missing prompt block for agent_nickname '{}'",
                nickname.name
            ));
        };
        if prompt.trim().is_empty() {
            return Err(format!(
                "prompt block for agent_nickname '{}' must be non-empty",
                nickname.name
            ));
        }
    }

    for prompt_nickname in prompts_by_nickname.keys() {
        if !seen_nicknames.contains(prompt_nickname) {
            return Err(format!(
                "prompt block references undeclared agent_nickname '{}'",
                prompt_nickname
            ));
        }
    }

    Ok(ParsedRoleTemplate {
        role_name: stem.to_string(),
        description,
        read_only: parsed.read_only,
        model: parsed
            .model
            .map(|model| model.trim().to_string())
            .filter(|model| !model.is_empty()),
        reasoning_effort: parsed.reasoning_effort,
        allow_list: normalize_tool_list(parsed.allow_list),
        deny_list: normalize_tool_list(parsed.deny_list),
        nicknames,
        prompts_by_nickname,
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
    let mut current_nickname: Option<String> = None;
    let mut prompts: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for line in body.lines() {
        if let Some(nickname) = agent_nickname_marker(line) {
            if prompts.contains_key(&nickname) {
                return Err(format!(
                    "duplicate prompt block marker for agent_nickname '{nickname}'"
                ));
            }
            prompts.insert(nickname.clone(), Vec::new());
            current_nickname = Some(nickname);
            continue;
        }

        match current_nickname.as_ref() {
            Some(nickname) => {
                let Some(lines) = prompts.get_mut(nickname) else {
                    return Err(format!(
                        "missing prompt block container for agent_nickname '{nickname}'"
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
        .map(|(nickname, lines)| (nickname, lines.join("\n").trim().to_string()))
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

    Some(normalize_role_or_nickname(inner))
}

fn normalize_role_or_nickname(value: &str) -> String {
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
enabled: false
agent_names:
  - name: default
    description: default persona
  - name: runner
    description: runner persona
read_only: true
---
<!-- agent_nickname: default -->
default prompt
<!-- agent_nickname: runner -->
runner prompt
"#;

    #[test]
    fn parse_role_template_requires_default_nickname() {
        let template = r#"---
description: Test role
agent_names:
  - name: runner
    description: runner persona
---
<!-- agent_nickname: runner -->
runner prompt
"#;

        let err = parse_role_template("worker", template).expect_err("must fail");
        assert!(err.contains("required agent_nickname 'default'"));
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

        let err = parse_role_template("worker", template).expect_err("must fail");
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

        let err = parse_role_template("worker", template).expect_err("must fail");
        assert!(err.contains("missing prompt block for agent_nickname 'runner'"));
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

        let err = parse_role_template("worker", template).expect_err("must fail");
        assert!(err.contains("template body must use <!-- agent_nickname"));
    }

    #[test]
    fn resolve_spawn_settings_defaults_to_default_persona() {
        let parsed = parse_role_template(
            "worker",
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
            .resolve_spawn_settings("worker", None)
            .expect("resolve should succeed")
            .expect("template should exist");
        assert_eq!(settings.agent_nickname, "default".to_string());
        assert_eq!(settings.agent_persona, "default".to_string());
        assert_eq!(settings.instructions, "default prompt".to_string());
    }

    #[test]
    fn load_templates_uses_precedence_order() {
        let root = TempDir::new().expect("create tempdir");
        let project_dir = root.path().join("project");
        let nested_dir = project_dir.join("nested");
        fs::create_dir_all(nested_dir.join(".codex").join(".agents")).expect("create nested");
        fs::create_dir_all(project_dir.join(".codex").join(".agents")).expect("create project");

        fs::write(
            nested_dir.join(".codex").join(".agents").join("worker.md"),
            r#"---
description: Nested role
agent_names:
  - name: default
    description: nested default
---
<!-- agent_nickname: default -->
nested prompt
"#,
        )
        .expect("write nested template");

        fs::write(
            project_dir.join(".codex").join(".agents").join("worker.md"),
            r#"---
description: Project role
agent_names:
  - name: default
    description: project default
---
<!-- agent_nickname: default -->
project prompt
"#,
        )
        .expect("write project template");

        let templates = load_templates_from_sources(
            &[
                nested_dir.join(".codex").join(".agents"),
                project_dir.join(".codex").join(".agents"),
            ],
            None,
            Vec::new(),
        )
        .expect("templates should load");
        let loaded = LoadedRoleTemplates {
            templates_by_stem: templates,
        };

        let metadata = loaded
            .role_metadata("worker")
            .expect("metadata query should succeed")
            .expect("role metadata should exist");
        assert_eq!(metadata.description, "Nested role".to_string());
    }

    #[test]
    fn parse_role_template_normalizes_tool_lists() {
        let template = r#"---
description: Test role
agent_names:
  - name: default
    description: default persona
allow_list: [" wait ", "spawn_agent", "wait"]
deny_list: ["", "send_input"]
---
<!-- agent_nickname: default -->
default prompt
"#;

        let parsed = parse_role_template("worker", template).expect("template should parse");
        assert_eq!(
            parsed.allow_list,
            Some(vec!["spawn_agent".to_string(), "wait".to_string()])
        );
        assert_eq!(parsed.deny_list, Some(vec!["send_input".to_string()]));
    }

    #[test]
    fn parse_role_template_validates_frontmatter_schema() {
        let err = parse_role_template("worker", VALID_TEMPLATE).expect_err("must fail");
        assert!(err.contains("invalid YAML"));
    }
}
