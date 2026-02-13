// SA: fork-specific template loader/parser for `spawn_agent` roles and personalities.
use include_dir::Dir;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

use codex_protocol::openai_models::ReasoningEffort;
// [SA] COMMIT OPEN: compile-time agent templates
// Role: allow adding new `spawn_agent.agent_type` values by adding a single
// `templates/agents/<name>.md` file, without touching Rust code.
static AGENTS_TEMPLATES_DIR: Dir =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/templates/agents");
// [SA] COMMIT CLOSE: compile-time agent templates

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TemplateMeta {
    pub(crate) description: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) reasoning_effort: Option<ReasoningEffort>,
    pub(crate) agent_names: Vec<TemplateAgentName>,
    // SA: template-level tool policy for spawned agents of this agent_type.
    pub(crate) allow_list: Option<Vec<String>>,
    // SA: template-level tool policy for spawned agents of this agent_type.
    pub(crate) deny_list: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TemplateAgentName {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ParsedTemplate {
    pub(crate) meta: TemplateMeta,
    pub(crate) default_instructions: String,
    pub(crate) named_instructions: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub(crate) struct TemplateSummary {
    pub(crate) stem: String,
    pub(crate) description: Option<String>,
    pub(crate) agent_names: Vec<TemplateAgentName>,
}

#[derive(Debug, Deserialize)]
struct TemplateFrontmatter {
    description: Option<String>,
    model: Option<String>,
    reasoning_effort: Option<ReasoningEffort>,
    agent_names: Option<Vec<TemplateAgentNameEntry>>,
    // SA: optional per-agent_type allow_list/deny_list from template YAML frontmatter.
    allow_list: Option<Vec<String>>,
    // SA: optional per-agent_type allow_list/deny_list from template YAML frontmatter.
    deny_list: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct TemplateAgentNameEntry {
    name: String,
    description: Option<String>,
}

fn normalize_stem(input: &str) -> String {
    input.trim().to_ascii_lowercase()
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
    // Strict allowlist to prevent path traversal and keep the contract simple.
    stem.chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
}

pub(crate) fn get_md(stem: &str) -> Result<&'static str, String> {
    let stem = normalize_stem(stem);
    if !is_valid_stem(&stem) {
        return Err(format!(
            "invalid agent_type {stem:?}; expected snake_case like \"worker\" or \"my_custom_role\""
        ));
    }

    let file_name = format!("{stem}.md");
    let file = AGENTS_TEMPLATES_DIR
        .get_file(&file_name)
        .ok_or_else(|| format!("missing agent template: templates/agents/{file_name}"))?;
    file.contents_utf8()
        .ok_or_else(|| format!("agent template is not valid UTF-8: templates/agents/{file_name}"))
}

pub(crate) fn list_stems() -> Vec<String> {
    let mut stems: Vec<String> = AGENTS_TEMPLATES_DIR
        .files()
        .filter_map(|file| file.path().file_name()?.to_str())
        .filter_map(|name| name.strip_suffix(".md"))
        .map(ToString::to_string)
        .collect();
    stems.sort();
    stems
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
        // Unterminated frontmatter: treat input as-is.
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
            })
            .collect::<Vec<_>>();
        TemplateMeta {
            description: parsed.description,
            model: parsed
                .model
                .map(|m| m.trim().to_string())
                .filter(|m| !m.is_empty()),
            reasoning_effort: parsed.reasoning_effort,
            agent_names,
            allow_list: normalize_tool_list(parsed.allow_list),
            deny_list: normalize_tool_list(parsed.deny_list),
        }
    } else {
        TemplateMeta::default()
    };

    if !meta.agent_names.is_empty() {
        let declared: std::collections::HashMap<_, _> = meta
            .agent_names
            .iter()
            .map(|n| (n.name.as_str(), ()))
            .collect();
        for name in named_instructions.keys() {
            if !declared.contains_key(name.as_str()) {
                return Err(format!(
                    "invalid agent_names: missing agent_names entry for {name:?}"
                ));
            }
        }
        for name in meta.agent_names.iter().map(|n| n.name.as_str()) {
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

fn templates_cache() -> &'static HashMap<String, ParsedTemplate> {
    static CACHE: OnceLock<HashMap<String, ParsedTemplate>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut out = HashMap::new();
        for stem in list_stems() {
            let Ok(md) = get_md(&stem) else {
                continue;
            };
            let parsed = match parse_template(md) {
                Ok(parsed) => parsed,
                Err(_) => {
                    // Keep backward compatibility: if parsing fails, treat as legacy template.
                    ParsedTemplate {
                        meta: TemplateMeta::default(),
                        default_instructions: md.to_string(),
                        named_instructions: HashMap::new(),
                    }
                }
            };
            out.insert(stem, parsed);
        }
        out
    })
}

pub(crate) fn get_parsed(stem: &str) -> Result<&'static ParsedTemplate, String> {
    let stem = normalize_stem(stem);
    if !is_valid_stem(&stem) {
        return Err(format!(
            "invalid agent_type {stem:?}; expected snake_case like \"worker\" or \"my_custom_role\""
        ));
    }
    templates_cache()
        .get(&stem)
        .ok_or_else(|| format!("missing agent template: templates/agents/{stem}.md"))
}

pub(crate) fn list_summaries() -> Vec<TemplateSummary> {
    let mut summaries: Vec<TemplateSummary> = templates_cache()
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

pub(crate) fn spawn_agent_templates_hint() -> String {
    let templates = list_summaries();
    if templates.is_empty() {
        return String::new();
    }

    let max_templates = 12usize;
    let mut items = Vec::new();
    for tpl in templates.into_iter().take(max_templates) {
        let mut chunk = tpl.stem;
        if let Some(desc) = tpl
            .description
            .as_deref()
            .map(str::trim)
            .filter(|d| !d.is_empty())
        {
            let mut desc = desc.to_string();
            let max = 120usize;
            if desc.chars().count() > max {
                desc = desc.chars().take(max).collect::<String>();
                desc.push_str("...");
            }
            chunk.push_str(&format!(" ({desc})"));
        }
        if !tpl.agent_names.is_empty() {
            let names = tpl
                .agent_names
                .into_iter()
                .take(6)
                .map(|n| {
                    if let Some(d) = n
                        .description
                        .as_deref()
                        .map(str::trim)
                        .filter(|d| !d.is_empty())
                    {
                        format!("{}: {d}", n.name)
                    } else {
                        n.name
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            chunk.push_str(&format!("; agent_names: [{names}]"));
        }
        items.push(chunk);
    }

    format!(" Available templates: {}.", items.join(" | "))
}
