// FORK COMMIT NEW FILE [SA]: template loader/parser for `spawn_agent` roles and personalities.
// Role: allow role customization from `core/templates/agents/*.md` with compile-time embedding.
use codex_protocol::openai_models::ReasoningEffort;
use include_dir::Dir;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::OnceLock;

// FORK COMMIT OPEN [SA]: compile-time agent template bundle.
// Role: keep runtime independent from local filesystem and make template updates reproducible.
static AGENT_TEMPLATES_DIR: Dir = include_dir::include_dir!("$CARGO_MANIFEST_DIR/templates/agents");
// FORK COMMIT CLOSE: compile-time agent template bundle.

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TemplateMeta {
    pub(crate) description: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) reasoning_effort: Option<ReasoningEffort>,
    pub(crate) agent_names: Vec<TemplateAgentName>,
    pub(crate) read_only: bool,
    pub(crate) allow_list: Option<Vec<String>>,
    pub(crate) deny_list: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TemplateAgentName {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) reasoning_effort: Option<ReasoningEffort>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ParsedTemplate {
    pub(crate) meta: TemplateMeta,
    pub(crate) default_instructions: String,
    pub(crate) named_instructions: HashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
struct TemplatesCache {
    raw_templates: HashMap<String, String>,
    parsed_templates: HashMap<String, ParsedTemplate>,
    stems: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct TemplateSummary {
    pub(crate) stem: String,
    pub(crate) description: Option<String>,
    pub(crate) agent_names: Vec<TemplateAgentName>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
// FORK COMMIT OPEN [SA]: list_agents summary contract with optional expanded fields.
// Role: expose compact defaults and opt-in detailed role/persona metadata.
pub(crate) struct ListAgentsSummary {
    pub(crate) agent_type: String,
    pub(crate) description: String,
    pub(crate) allow_list: Option<Vec<String>>,
    pub(crate) deny_list: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reasoning_effort: Option<ReasoningEffort>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) default_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) agent_names: Option<Vec<ListAgentsAgentNameSummary>>,
}
// FORK COMMIT CLOSE: list_agents summary contract.

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct ListAgentsAgentNameSummary {
    pub(crate) name: String,
    pub(crate) description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reasoning_effort: Option<ReasoningEffort>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) prompt: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TemplateFrontmatter {
    description: Option<String>,
    model: Option<String>,
    reasoning_effort: Option<ReasoningEffort>,
    agent_names: Option<Vec<TemplateAgentNameEntry>>,
    #[serde(default)]
    read_only: bool,
    allow_list: Option<Vec<String>>,
    deny_list: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct TemplateAgentNameEntry {
    name: String,
    description: Option<String>,
    model: Option<String>,
    reasoning_effort: Option<ReasoningEffort>,
}

fn normalize_stem(input: &str) -> String {
    input.trim().to_ascii_lowercase()
}

fn canonical_stem(stem: &str) -> String {
    normalize_stem(stem).replace('-', "_")
}

fn normalize_tool_list(input: Option<Vec<String>>) -> Option<Vec<String>> {
    let mut tools: Vec<String> = input
        .unwrap_or_default()
        .into_iter()
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .collect();
    tools.sort();
    tools.dedup();
    (!tools.is_empty()).then_some(tools)
}

fn is_valid_stem(stem: &str) -> bool {
    if stem.is_empty() {
        return false;
    }
    stem.chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
}

fn invalid_agent_type_error(stem: &str) -> String {
    format!(
        "invalid agent_type {stem:?}; expected snake_case or kebab-case like \"worker\", \"my_custom_role\", or \"my-custom-role\""
    )
}

fn missing_agent_template_error(stem: &str) -> String {
    format!("missing agent template: {stem}")
}

fn discover_project_agent_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let mut cursor = match std::env::current_dir() {
        Ok(path) => path,
        Err(_) => return dirs,
    };

    loop {
        let candidate = cursor.join(".codex").join(".agents");
        if candidate.is_dir() {
            dirs.push(candidate);
        }
        if cursor.join(".git").exists() {
            break;
        }
        let Some(parent) = cursor.parent() else {
            break;
        };
        if parent == cursor {
            break;
        }
        cursor = parent.to_path_buf();
    }

    dirs
}

fn user_agent_dir() -> Option<PathBuf> {
    crate::config::find_codex_home()
        .ok()
        .map(|codex_home| codex_home.join(".agents"))
}

fn embedded_templates() -> Vec<(String, String)> {
    let mut templates: Vec<(String, String)> = AGENT_TEMPLATES_DIR
        .files()
        .filter_map(|file| {
            let file_name = file.path().file_name()?.to_str()?;
            let stem = file_name.strip_suffix(".md")?;
            let contents = file.contents_utf8()?.to_string();
            Some((stem.to_string(), contents))
        })
        .collect();
    templates.sort_by(|a, b| a.0.cmp(&b.0));
    templates
}

fn load_templates_from_dir(dir: &Path) -> Vec<(String, String)> {
    let mut templates = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return templates;
    };

    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect();
    paths.sort();

    for path in paths {
        if !path.is_file() {
            continue;
        }
        let is_markdown = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md"));
        if !is_markdown {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let stem = normalize_stem(stem);
        if !is_valid_stem(&stem) {
            continue;
        }
        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };
        templates.push((stem, contents));
    }

    templates
}

fn maybe_seed_user_templates(
    user_dir: &Path,
    embedded: &[(String, String)],
) -> std::io::Result<bool> {
    if embedded.is_empty() {
        return Ok(false);
    }
    fs::create_dir_all(user_dir)?;
    let has_any_embedded_match = embedded
        .iter()
        .any(|(stem, _)| user_dir.join(format!("{stem}.md")).is_file());
    if has_any_embedded_match {
        return Ok(false);
    }
    for (stem, contents) in embedded {
        fs::write(user_dir.join(format!("{stem}.md")), contents)?;
    }
    Ok(true)
}

fn seed_user_templates_if_needed(embedded: &[(String, String)]) {
    if cfg!(test) {
        return;
    }
    let Some(user_dir) = user_agent_dir() else {
        return;
    };
    let _ = maybe_seed_user_templates(&user_dir, embedded);
}

fn build_templates_cache() -> TemplatesCache {
    let embedded = embedded_templates();
    // FORK COMMIT OPEN [SA]: seed global ~/.codex/.agents with embedded templates when empty.
    // Role: guarantee baseline runtime templates for external customization without forcing overwrite.
    seed_user_templates_if_needed(&embedded);
    // FORK COMMIT CLOSE: seed global ~/.codex/.agents with embedded templates when empty.

    let mut raw_templates: HashMap<String, String> = HashMap::new();
    for project_dir in discover_project_agent_dirs() {
        for (stem, contents) in load_templates_from_dir(&project_dir) {
            raw_templates.entry(stem).or_insert(contents);
        }
    }
    if let Some(user_dir) = user_agent_dir() {
        for (stem, contents) in load_templates_from_dir(&user_dir) {
            raw_templates.entry(stem).or_insert(contents);
        }
    }
    for (stem, contents) in embedded {
        raw_templates.entry(stem).or_insert(contents);
    }

    let mut stems: Vec<String> = raw_templates.keys().cloned().collect();
    stems.sort();

    let mut parsed_templates = HashMap::new();
    for stem in &stems {
        let Some(md) = raw_templates.get(stem) else {
            continue;
        };
        let parsed = match parse_template(md) {
            Ok(parsed) => parsed,
            // FORK COMMIT OPEN [SA]: preserve legacy behavior on template parse failures.
            // Role: avoid hard crashes for malformed templates by treating file as plain instructions.
            Err(_) => ParsedTemplate {
                meta: TemplateMeta::default(),
                default_instructions: md.to_string(),
                named_instructions: HashMap::new(),
            },
            // FORK COMMIT CLOSE: preserve legacy behavior on template parse failures.
        };
        parsed_templates.insert(stem.clone(), parsed);
    }

    TemplatesCache {
        raw_templates,
        parsed_templates,
        stems,
    }
}

fn resolve_stem(cache: &TemplatesCache, stem: &str) -> Result<String, String> {
    let stem = normalize_stem(stem);
    if !is_valid_stem(&stem) {
        return Err(invalid_agent_type_error(&stem));
    }
    if cache.raw_templates.contains_key(&stem) {
        return Ok(stem);
    }

    let canonical = canonical_stem(&stem);
    let mut candidates: Vec<String> = cache
        .stems
        .iter()
        .filter(|existing| canonical_stem(existing) == canonical)
        .cloned()
        .collect();
    candidates.sort();
    candidates.dedup();

    match candidates.len() {
        1 => Ok(candidates.remove(0)),
        0 => Err(missing_agent_template_error(&stem)),
        _ => Err(format!(
            "ambiguous agent_type {stem:?}; matched multiple templates: {}. Use exact agent_type.",
            candidates.join(", ")
        )),
    }
}

pub(crate) fn list_stems() -> Vec<String> {
    templates_cache().stems.clone()
}

pub(crate) fn get_md(stem: &str) -> Result<String, String> {
    let cache = templates_cache();
    let resolved = resolve_stem(cache, stem)?;
    cache
        .raw_templates
        .get(&resolved)
        .cloned()
        .ok_or_else(|| missing_agent_template_error(&resolved))
}

fn split_frontmatter(contents: &str) -> (Option<String>, String) {
    let mut lines = contents.lines();
    let Some(first) = lines.next() else {
        return (None, String::new());
    };
    if first.trim() != "---" {
        return (None, contents.to_string());
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
        return (None, contents.to_string());
    }

    let frontmatter = frontmatter_lines.join("\n");
    let body = lines.collect::<Vec<_>>().join("\n");
    (Some(frontmatter), body)
}

fn agent_name_marker(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let prefix = "<!-- agent_name:";
    if !trimmed.starts_with(prefix) || !trimmed.ends_with("-->") {
        return None;
    }
    let inner = trimmed.strip_prefix(prefix)?.strip_suffix("-->")?.trim();
    if inner.is_empty() {
        return None;
    }
    Some(inner.to_ascii_lowercase())
}

fn split_agent_name_blocks(body: &str) -> (String, HashMap<String, String>) {
    let mut default_lines: Vec<&str> = Vec::new();
    let mut blocks: HashMap<String, Vec<&str>> = HashMap::new();
    let mut current: Option<String> = None;

    for line in body.lines() {
        if let Some(name) = agent_name_marker(line) {
            current = Some(name.clone());
            blocks.entry(name).or_default();
            continue;
        }
        if let Some(name) = current.as_ref() {
            blocks.entry(name.clone()).or_default().push(line);
        } else {
            default_lines.push(line);
        }
    }

    let default_body = default_lines.join("\n");
    let named = blocks.into_iter().map(|(k, v)| (k, v.join("\n"))).collect();
    (default_body, named)
}

fn parse_template(contents: &str) -> Result<ParsedTemplate, String> {
    let (frontmatter, body) = split_frontmatter(contents);
    let (default_instructions, named_instructions) = split_agent_name_blocks(&body);

    let meta = if let Some(frontmatter) = frontmatter {
        let parsed: TemplateFrontmatter =
            serde_yaml::from_str(&frontmatter).map_err(|err| format!("invalid YAML: {err}"))?;
        let agent_names = parsed
            .agent_names
            .unwrap_or_default()
            .into_iter()
            .map(|entry| TemplateAgentName {
                name: entry.name.trim().to_ascii_lowercase(),
                description: entry.description,
                model: entry
                    .model
                    .map(|model| model.trim().to_string())
                    .filter(|model| !model.is_empty()),
                reasoning_effort: entry.reasoning_effort,
            })
            .collect::<Vec<_>>();
        TemplateMeta {
            description: parsed.description,
            model: parsed
                .model
                .map(|model| model.trim().to_string())
                .filter(|model| !model.is_empty()),
            reasoning_effort: parsed.reasoning_effort,
            agent_names,
            // FORK COMMIT [SA]: template-level read_only maps to spawned thread sandbox policy.
            read_only: parsed.read_only,
            allow_list: normalize_tool_list(parsed.allow_list),
            deny_list: normalize_tool_list(parsed.deny_list),
        }
    } else {
        TemplateMeta::default()
    };

    if !meta.agent_names.is_empty() {
        let declared: HashMap<&str, ()> = meta
            .agent_names
            .iter()
            .map(|entry| (entry.name.as_str(), ()))
            .collect();
        for name in named_instructions.keys() {
            if !declared.contains_key(name.as_str()) {
                return Err(format!(
                    "invalid agent_names: missing agent_names entry for {name:?}"
                ));
            }
        }
        for name in meta.agent_names.iter().map(|entry| entry.name.as_str()) {
            if !named_instructions.contains_key(name) {
                return Err(format!(
                    "invalid agent_names: missing agent_name block for {name:?}"
                ));
            }
        }
    }

    Ok(ParsedTemplate {
        meta,
        default_instructions,
        named_instructions,
    })
}

fn validate_list_agents_contract(stem: &str, parsed: &ParsedTemplate) -> Result<(), String> {
    if parsed
        .meta
        .description
        .as_deref()
        .map(str::trim)
        .is_none_or(str::is_empty)
    {
        return Err(format!(
            "invalid agent template {stem:?}: missing required frontmatter description"
        ));
    }
    if parsed.default_instructions.trim().is_empty() {
        return Err(format!(
            "invalid agent template {stem:?}: missing required default prompt block"
        ));
    }
    if !parsed.named_instructions.is_empty() && parsed.meta.agent_names.is_empty() {
        return Err(format!(
            "invalid agent template {stem:?}: found agent_name blocks but frontmatter agent_names is missing"
        ));
    }
    for entry in &parsed.meta.agent_names {
        if entry
            .description
            .as_deref()
            .map(str::trim)
            .is_none_or(str::is_empty)
        {
            return Err(format!(
                "invalid agent template {stem:?}: missing description for agent_name {:?}",
                entry.name
            ));
        }
    }
    Ok(())
}

fn templates_cache() -> &'static TemplatesCache {
    static CACHE: OnceLock<TemplatesCache> = OnceLock::new();
    CACHE.get_or_init(build_templates_cache)
}

pub(crate) fn get_parsed(stem: &str) -> Result<&'static ParsedTemplate, String> {
    let cache = templates_cache();
    let stem = resolve_stem(cache, stem)?;
    templates_cache()
        .parsed_templates
        .get(&stem)
        .ok_or_else(|| missing_agent_template_error(&stem))
}

pub(crate) fn list_summaries() -> Vec<TemplateSummary> {
    let mut summaries: Vec<TemplateSummary> = templates_cache()
        .parsed_templates
        .iter()
        .map(|(stem, parsed)| TemplateSummary {
            stem: stem.clone(),
            description: parsed.meta.description.clone(),
            agent_names: parsed.meta.agent_names.clone(),
        })
        .collect();
    summaries.sort_by(|a, b| a.stem.cmp(&b.stem));
    summaries
}

pub(crate) fn list_agents_summaries(
    agent_type: Option<&str>,
    expanded: bool,
) -> Result<Vec<ListAgentsSummary>, String> {
    // FORK COMMIT OPEN [SA]: list_agents query options for focused and expanded output.
    // Role: allow filtering by role and opt-in expanded metadata without changing default payload shape.
    let cache = templates_cache();
    let normalized_filter = agent_type
        .map(|agent_type| resolve_stem(cache, agent_type))
        .transpose()?;

    let mut summaries = Vec::new();
    let stems = if let Some(filter) = normalized_filter.clone() {
        vec![filter]
    } else {
        list_stems()
    };
    for stem in stems {
        if normalized_filter
            .as_deref()
            .is_some_and(|filter| filter != stem)
        {
            continue;
        }
        let md = get_md(&stem)?;
        let parsed =
            parse_template(&md).map_err(|err| format!("invalid agent template {stem:?}: {err}"))?;
        validate_list_agents_contract(&stem, &parsed)?;

        let description = parsed
            .meta
            .description
            .as_deref()
            .map(str::trim)
            .ok_or_else(|| {
                format!("invalid agent template {stem:?}: missing required frontmatter description")
            })?
            .to_string();

        let mut agent_names = parsed
            .meta
            .agent_names
            .iter()
            .map(|entry| {
                let description = entry
                    .description
                    .as_deref()
                    .map(str::trim)
                    .ok_or_else(|| {
                        format!(
                            "invalid agent template {stem:?}: missing description for agent_name {:?}",
                            entry.name
                        )
                    })?
                    .to_string();
                let prompt = if expanded {
                    parsed
                        .named_instructions
                        .get(&entry.name)
                        .map(|prompt| prompt.trim().to_string())
                } else {
                    None
                };
                Ok(ListAgentsAgentNameSummary {
                    name: entry.name.clone(),
                    description,
                    model: expanded.then(|| entry.model.clone()).flatten(),
                    reasoning_effort: expanded.then_some(entry.reasoning_effort).flatten(),
                    prompt,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        agent_names.sort_by(|a, b| a.name.cmp(&b.name));

        summaries.push(ListAgentsSummary {
            agent_type: stem,
            description,
            allow_list: parsed.meta.allow_list.clone(),
            deny_list: parsed.meta.deny_list.clone(),
            model: expanded.then(|| parsed.meta.model.clone()).flatten(),
            reasoning_effort: expanded.then_some(parsed.meta.reasoning_effort).flatten(),
            default_prompt: expanded.then(|| parsed.default_instructions.trim().to_string()),
            agent_names: (!agent_names.is_empty()).then_some(agent_names),
        });
    }
    summaries.sort_by(|a, b| a.agent_type.cmp(&b.agent_type));
    // FORK COMMIT CLOSE: list_agents query options for focused and expanded output.
    Ok(summaries)
}

pub(crate) fn spawn_agent_templates_hint() -> String {
    let templates = list_summaries();
    if templates.is_empty() {
        return String::new();
    }

    let max_templates = 12usize;
    let mut entries = Vec::new();
    for summary in templates.into_iter().take(max_templates) {
        let mut chunk = summary.stem;
        if let Some(description) = summary
            .description
            .as_deref()
            .map(str::trim)
            .filter(|description| !description.is_empty())
        {
            let mut description = description.to_string();
            let max_len = 120usize;
            if description.chars().count() > max_len {
                description = description.chars().take(max_len - 1).collect::<String>() + "…";
            }
            chunk.push_str(": ");
            chunk.push_str(&description);
        }
        if !summary.agent_names.is_empty() {
            let names = summary
                .agent_names
                .iter()
                .map(|name| name.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            chunk.push_str(" [agent_names: ");
            chunk.push_str(&names);
            chunk.push(']');
        }
        entries.push(chunk);
    }

    if entries.is_empty() {
        return String::new();
    }

    format!("\nAvailable template roles: {}.", entries.join("; "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use tempfile::tempdir;

    #[test]
    fn get_md_rejects_invalid_stem() {
        let err = get_md("../worker").expect_err("stem should fail");
        assert!(err.contains("invalid agent_type"));
    }

    #[test]
    fn get_md_resolves_embedded_template() {
        let md = get_md("orchestrator").expect("template should exist");
        assert!(!md.trim().is_empty());
    }

    #[test]
    fn parse_template_supports_frontmatter_and_named_blocks() {
        let template = r#"---
description: Worker profile
model: gpt-5-codex
reasoning_effort: medium
read_only: true
agent_names:
  - name: fast
    model: gpt-5.1-codex-mini
    reasoning_effort: low
allow_list: [exec_command, wait]
deny_list: [rm]
---
default instructions
<!-- agent_name: fast -->
fast instructions
"#;
        let parsed = parse_template(template).expect("template parses");
        assert_eq!(parsed.meta.description, Some("Worker profile".to_string()));
        assert_eq!(parsed.meta.model, Some("gpt-5-codex".to_string()));
        assert_eq!(parsed.meta.reasoning_effort, Some(ReasoningEffort::Medium));
        assert!(parsed.meta.read_only);
        assert_eq!(parsed.meta.agent_names.len(), 1);
        assert_eq!(
            parsed.meta.agent_names[0].model,
            Some("gpt-5.1-codex-mini".to_string())
        );
        assert_eq!(
            parsed.meta.agent_names[0].reasoning_effort,
            Some(ReasoningEffort::Low)
        );
        assert_eq!(
            parsed.meta.allow_list,
            Some(vec!["exec_command".to_string(), "wait".to_string()])
        );
        assert_eq!(parsed.meta.deny_list, Some(vec!["rm".to_string()]));
        assert_eq!(parsed.default_instructions.trim(), "default instructions");
        assert_eq!(
            parsed.named_instructions.get("fast").map(|s| s.trim()),
            Some("fast instructions")
        );
    }

    #[test]
    fn parse_template_rejects_declared_agent_without_block() {
        let template = r#"---
agent_names:
  - name: deep
---
default
"#;
        let err = parse_template(template).expect_err("must fail");
        assert!(err.contains("missing agent_name block"));
    }

    #[test]
    fn parse_template_rejects_block_without_declared_agent() {
        let template = r#"---
agent_names:
  - name: deep
---
<!-- agent_name: fast -->
fast
"#;
        let err = parse_template(template).expect_err("must fail");
        assert!(err.contains("missing agent_names entry"));
    }

    #[test]
    fn validate_list_agents_contract_requires_description() {
        let template = r#"---
allow_list: [exec_command]
---
default
"#;
        let parsed = parse_template(template).expect("template parses");
        let err = validate_list_agents_contract("worker", &parsed).expect_err("must fail");
        assert!(err.contains("missing required frontmatter description"));
    }

    #[test]
    fn validate_list_agents_contract_requires_default_prompt() {
        let template = r#"---
description: Worker role
---
   "#;
        let parsed = parse_template(template).expect("template parses");
        let err = validate_list_agents_contract("worker", &parsed).expect_err("must fail");
        assert!(err.contains("missing required default prompt block"));
    }

    #[test]
    fn list_agents_summaries_omits_empty_agent_names() {
        let parsed = ParsedTemplate {
            meta: TemplateMeta {
                description: Some("Worker role".to_string()),
                allow_list: Some(vec!["exec_command".to_string()]),
                ..TemplateMeta::default()
            },
            default_instructions: "do work".to_string(),
            named_instructions: HashMap::new(),
        };
        validate_list_agents_contract("worker", &parsed).expect("contract should be valid");
        let summary = ListAgentsSummary {
            agent_type: "worker".to_string(),
            description: "Worker role".to_string(),
            allow_list: Some(vec!["exec_command".to_string()]),
            deny_list: None,
            model: None,
            reasoning_effort: None,
            default_prompt: None,
            agent_names: None,
        };
        assert_eq!(summary.agent_names, None);
    }

    #[test]
    fn list_agents_summaries_filters_by_agent_type() {
        let summaries = list_agents_summaries(Some("worker"), false).expect("must list worker");
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].agent_type, "worker".to_string());
        assert_eq!(summaries[0].model, None);
        assert_eq!(summaries[0].default_prompt, None);
    }

    #[test]
    fn list_agents_summaries_expanded_includes_extended_fields() {
        let summaries =
            list_agents_summaries(Some("explorer"), true).expect("must list explorer with details");
        assert_eq!(summaries.len(), 1);
        let explorer = &summaries[0];
        let parsed = get_parsed("explorer").expect("must parse explorer template");
        assert_eq!(explorer.agent_type, "explorer".to_string());
        assert_eq!(explorer.model, parsed.meta.model.clone());
        assert_eq!(explorer.reasoning_effort, parsed.meta.reasoning_effort);
        assert!(
            explorer
                .default_prompt
                .as_ref()
                .is_some_and(|text| !text.is_empty())
        );
        if parsed.meta.agent_names.is_empty() {
            assert_eq!(explorer.agent_names, None);
        } else {
            let agent_names = explorer
                .agent_names
                .clone()
                .expect("agent_names must exist");
            assert_eq!(agent_names.len(), parsed.meta.agent_names.len());
            assert!(
                agent_names
                    .iter()
                    .all(|agent| agent.prompt.as_ref().is_some_and(|text| !text.is_empty()))
            );
        }
    }

    #[test]
    fn resolve_stem_accepts_snake_and_kebab_alias() {
        let cache = TemplatesCache {
            stems: vec!["worker_role".to_string()],
            raw_templates: HashMap::from([(
                "worker_role".to_string(),
                "description: test".to_string(),
            )]),
            parsed_templates: HashMap::new(),
        };
        assert_eq!(
            resolve_stem(&cache, "worker-role").expect("must resolve alias"),
            "worker_role".to_string()
        );
    }

    #[test]
    fn resolve_stem_prefers_exact_match_when_canonical_is_ambiguous() {
        let cache = TemplatesCache {
            stems: vec!["worker_role".to_string(), "worker-role".to_string()],
            raw_templates: HashMap::from([
                ("worker_role".to_string(), "a".to_string()),
                ("worker-role".to_string(), "b".to_string()),
            ]),
            parsed_templates: HashMap::new(),
        };
        assert_eq!(
            resolve_stem(&cache, "worker-role").expect("must resolve exact stem"),
            "worker-role".to_string()
        );
        assert_eq!(
            resolve_stem(&cache, "worker_role").expect("must resolve exact stem"),
            "worker_role".to_string()
        );
    }

    #[test]
    fn maybe_seed_user_templates_seeds_only_when_no_embedded_match_exists() {
        let temp = tempdir().expect("tempdir");
        let user_dir = temp.path().join(".agents");
        let embedded = vec![
            ("worker".to_string(), "worker template".to_string()),
            ("explorer".to_string(), "explorer template".to_string()),
        ];
        let seeded = maybe_seed_user_templates(&user_dir, &embedded).expect("seed should work");
        assert!(seeded);
        assert!(user_dir.join("worker.md").is_file());
        assert!(user_dir.join("explorer.md").is_file());
    }

    #[test]
    fn maybe_seed_user_templates_skips_when_any_embedded_match_exists() {
        let temp = tempdir().expect("tempdir");
        let user_dir = temp.path().join(".agents");
        std::fs::create_dir_all(&user_dir).expect("create user dir");
        std::fs::write(user_dir.join("worker.md"), "custom worker").expect("write custom file");

        let embedded = vec![
            ("worker".to_string(), "worker template".to_string()),
            ("explorer".to_string(), "explorer template".to_string()),
        ];
        let seeded = maybe_seed_user_templates(&user_dir, &embedded).expect("seed should work");
        assert!(!seeded);
        assert_eq!(
            std::fs::read_to_string(user_dir.join("worker.md")).expect("read custom file"),
            "custom worker".to_string()
        );
        assert!(!user_dir.join("explorer.md").exists());
    }
}
