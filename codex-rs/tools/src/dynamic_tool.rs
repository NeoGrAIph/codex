use crate::ToolDefinition;
use crate::parse_tool_input_schema;
use codex_features::is_reserved_responses_tool_namespace;
use codex_protocol::dynamic_tools::DynamicToolFunctionSpec;
use codex_protocol::dynamic_tools::DynamicToolNamespaceSpec;
use codex_protocol::dynamic_tools::DynamicToolSpec;

pub const DYNAMIC_TOOL_NAME_MAX_LEN: usize = 128;
pub const DYNAMIC_TOOL_NAMESPACE_MAX_LEN: usize = 64;
pub const DYNAMIC_TOOL_NAMESPACE_DESCRIPTION_MAX_LEN: usize = 1024;
pub const DYNAMIC_TOOL_IDENTIFIER_PATTERN: &str = "^[a-zA-Z0-9_-]+$";
const DYNAMIC_TOOL_DIAGNOSTIC_VALUE_MAX_CHARS: usize = 96;

pub fn parse_dynamic_tool(
    tool: &DynamicToolFunctionSpec,
) -> Result<ToolDefinition, serde_json::Error> {
    Ok(ToolDefinition {
        name: tool.name.clone(),
        description: tool.description.clone(),
        input_schema: parse_tool_input_schema(&tool.input_schema)?,
        output_schema: None,
        defer_loading: tool.defer_loading,
    })
}

pub fn validate_dynamic_tool_identifier(
    value: &str,
    label: &str,
    max_len: usize,
) -> Result<(), String> {
    if value.chars().take(max_len + 1).count() > max_len {
        return Err(format!(
            "{label} must be at most {max_len} characters to match Responses API"
        ));
    }
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    if trimmed != value {
        return Err(format!(
            "{label} has leading/trailing whitespace: {}",
            bounded_dynamic_tool_value(value)
        ));
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(format!(
            "{label} must match {DYNAMIC_TOOL_IDENTIFIER_PATTERN} to match Responses API: {}",
            bounded_dynamic_tool_value(value)
        ));
    }
    Ok(())
}

pub fn validate_dynamic_tool_namespace(namespace: &DynamicToolNamespaceSpec) -> Result<(), String> {
    validate_dynamic_tool_identifier(
        &namespace.name,
        "dynamic tool namespace",
        DYNAMIC_TOOL_NAMESPACE_MAX_LEN,
    )?;
    if namespace.description.chars().count() > DYNAMIC_TOOL_NAMESPACE_DESCRIPTION_MAX_LEN {
        return Err(format!(
            "dynamic tool namespace description must be at most {DYNAMIC_TOOL_NAMESPACE_DESCRIPTION_MAX_LEN} characters"
        ));
    }
    Ok(())
}

fn bounded_dynamic_tool_value(value: &str) -> String {
    let mut escaped = value
        .chars()
        .flat_map(char::escape_default)
        .take(DYNAMIC_TOOL_DIAGNOSTIC_VALUE_MAX_CHARS + 1)
        .collect::<String>();
    if escaped.chars().count() > DYNAMIC_TOOL_DIAGNOSTIC_VALUE_MAX_CHARS {
        escaped.pop();
        escaped.push_str("...");
    }
    escaped
}

/// Reject dynamic namespaces that would collide with built-in or effective runtime namespaces.
pub fn validate_reserved_dynamic_tool_namespaces(
    tools: &[DynamicToolSpec],
    additional_reserved_namespaces: &[&str],
) -> Result<(), String> {
    for tool in tools {
        let DynamicToolSpec::Namespace(namespace) = tool else {
            continue;
        };
        validate_dynamic_tool_namespace(namespace)?;
        if namespace.name == "mcp" || namespace.name.starts_with("mcp__") {
            return Err(format!(
                "dynamic tool namespace is reserved: {}",
                namespace.name
            ));
        }
        if is_reserved_responses_tool_namespace(&namespace.name) {
            return Err(format!(
                "dynamic tool namespace collides with a reserved Responses API namespace: {}",
                namespace.name
            ));
        }
        if additional_reserved_namespaces.contains(&namespace.name.as_str()) {
            return Err(format!(
                "dynamic tool namespace collides with an active runtime namespace: {}",
                namespace.name
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "dynamic_tool_tests.rs"]
mod tests;
