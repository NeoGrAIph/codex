use super::*;
use std::collections::BTreeSet;

pub(crate) const MAX_AGENT_ROLE_CATALOG_JSON_BYTES: usize = 2_048;
const MAX_AGENT_ROLE_CATALOG_ENTRIES: usize = 32;
const MAX_AGENT_ROLE_ENTRY_JSON_BYTES: usize = 768;
const TRUNCATED_ROLE_ENTRY_MARKER: &str = "\n[role metadata truncated]";

/// Builds the spawn-agent tool description text from built-in and configured roles.
pub(crate) fn build(user_defined_agent_roles: &BTreeMap<String, AgentRoleConfig>) -> String {
    let built_in_roles = built_in::configs();
    build_from_configs(built_in_roles, user_defined_agent_roles)
}

/// Builds the historical unbounded V1-only description, including role-locked settings.
pub(crate) fn build_legacy_v1(
    user_defined_agent_roles: &BTreeMap<String, AgentRoleConfig>,
) -> String {
    let built_in_roles = built_in::configs();
    let mut seen = BTreeSet::new();
    let mut formatted_roles = Vec::new();
    for (name, declaration) in user_defined_agent_roles {
        if seen.insert(name.as_str()) {
            formatted_roles.push(format_role(name, declaration));
        }
    }
    for (name, declaration) in built_in_roles {
        if seen.insert(name.as_str()) {
            formatted_roles.push(format_built_in_legacy_v1_role(name, declaration));
        }
    }

    format!(
        "Optional type name for the new agent. If omitted, `{DEFAULT_ROLE_NAME}` is used.\nAvailable roles:\n{}",
        formatted_roles.join("\n"),
    )
}

fn build_from_configs(
    built_in_roles: &BTreeMap<String, AgentRoleConfig>,
    user_defined_roles: &BTreeMap<String, AgentRoleConfig>,
) -> String {
    let mut seen = BTreeSet::new();
    let mut selected_roles = Vec::new();
    let mut default_role = None;
    let mut role_count = 0;
    for roles in [user_defined_roles, built_in_roles] {
        for (name, declaration) in roles {
            if !seen.insert(name.as_str()) {
                continue;
            }
            let order = role_count;
            role_count += 1;
            let role = (order, name.as_str(), declaration);
            if name == DEFAULT_ROLE_NAME {
                default_role = Some(role);
            }
            if selected_roles.len() < MAX_AGENT_ROLE_CATALOG_ENTRIES {
                selected_roles.push(role);
            }
        }
    }
    if let Some(default_role) = default_role
        && !selected_roles
            .iter()
            .any(|(_, name, _)| *name == DEFAULT_ROLE_NAME)
    {
        selected_roles.pop();
        selected_roles.push(default_role);
        selected_roles.sort_by_key(|(order, _, _)| *order);
    }
    let formatted_roles = selected_roles
        .into_iter()
        .map(|(_, name, declaration)| (name, format_role(name, declaration)))
        .collect();

    bounded_catalog(formatted_roles, role_count)
}

fn bounded_catalog(formatted_roles: Vec<(&str, String)>, role_count: usize) -> String {
    let header = format!(
        "Optional type name for the new agent. If omitted, `{DEFAULT_ROLE_NAME}` is used.\nAvailable roles:"
    );
    let formatted_roles = formatted_roles
        .into_iter()
        .map(|(name, role)| (name, truncate_role_entry(role)))
        .collect::<Vec<_>>();
    let mut selected = Vec::new();
    for index in 0..formatted_roles.len() {
        if selected.len() == MAX_AGENT_ROLE_CATALOG_ENTRIES {
            break;
        }
        let mut candidate = selected.clone();
        candidate.push(index);
        if json_string_size(&render_catalog(&header, &formatted_roles, &candidate, None))
            <= MAX_AGENT_ROLE_CATALOG_JSON_BYTES
        {
            selected = candidate;
        }
    }

    if let Some(default_index) = formatted_roles
        .iter()
        .position(|(name, _)| *name == DEFAULT_ROLE_NAME)
        && !selected.contains(&default_index)
    {
        while selected.len() == MAX_AGENT_ROLE_CATALOG_ENTRIES {
            selected.pop();
        }
        selected.push(default_index);
        selected.sort_unstable();
        while json_string_size(&render_catalog(&header, &formatted_roles, &selected, None))
            > MAX_AGENT_ROLE_CATALOG_JSON_BYTES
        {
            let Some(position) = selected.iter().rposition(|index| *index != default_index) else {
                break;
            };
            selected.remove(position);
        }
    }

    loop {
        let omitted = role_count.saturating_sub(selected.len());
        let omission_marker = (omitted > 0).then(|| {
            format!(
                "[{omitted} additional role(s) omitted because the model-visible role catalog reached its safety limit]"
            )
        });
        let rendered = render_catalog(
            &header,
            &formatted_roles,
            &selected,
            omission_marker.as_deref(),
        );
        if json_string_size(&rendered) <= MAX_AGENT_ROLE_CATALOG_JSON_BYTES {
            return rendered;
        }
        let Some(position) = selected
            .iter()
            .rposition(|index| formatted_roles[*index].0 != DEFAULT_ROLE_NAME)
        else {
            return rendered;
        };
        selected.remove(position);
    }
}

fn render_catalog(
    header: &str,
    formatted_roles: &[(&str, String)],
    selected: &[usize],
    omission_marker: Option<&str>,
) -> String {
    let mut sections = selected
        .iter()
        .map(|index| formatted_roles[*index].1.as_str())
        .collect::<Vec<_>>();
    if let Some(omission_marker) = omission_marker {
        sections.push(omission_marker);
    }
    if sections.is_empty() {
        header.to_string()
    } else {
        format!("{header}\n{}", sections.join("\n"))
    }
}

fn truncate_role_entry(role: String) -> String {
    if json_string_size(&role) <= MAX_AGENT_ROLE_ENTRY_JSON_BYTES {
        return role;
    }
    let marker_content_bytes = json_string_size(TRUNCATED_ROLE_ENTRY_MARKER).saturating_sub(2);
    let mut used_bytes = 2 + marker_content_bytes;
    let mut truncated = String::new();
    for character in role.chars() {
        let character_bytes = json_string_size(&character.to_string()).saturating_sub(2);
        if used_bytes + character_bytes > MAX_AGENT_ROLE_ENTRY_JSON_BYTES {
            break;
        }
        truncated.push(character);
        used_bytes += character_bytes;
    }
    truncated.push_str(TRUNCATED_ROLE_ENTRY_MARKER);
    truncated
}

fn format_role(name: &str, declaration: &AgentRoleConfig) -> String {
    if let Some(description) = &declaration.description {
        format!("{name}: {{\n{description}\n}}")
    } else {
        format!("{name}: no description")
    }
}

fn format_built_in_legacy_v1_role(name: &str, declaration: &AgentRoleConfig) -> String {
    let Some(description) = &declaration.description else {
        return format!("{name}: no description");
    };
    let locked_settings_note = declaration
        .config_file
        .as_ref()
        .and_then(|config_file| built_in::config_file_contents(config_file).map(str::to_owned))
        .and_then(|contents| toml::from_str::<TomlValue>(&contents).ok())
        .map(|role_toml| agent_role_locked_settings_note(&role_toml))
        .unwrap_or_default();
    let locked_settings_note = if description.ends_with(&locked_settings_note) {
        String::new()
    } else {
        locked_settings_note
    };
    format!("{name}: {{\n{description}{locked_settings_note}\n}}")
}

fn json_string_size(value: &str) -> usize {
    serde_json::to_string(value).map_or(usize::MAX, |encoded| encoded.len())
}
