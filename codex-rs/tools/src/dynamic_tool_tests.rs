use super::parse_dynamic_tool;
use super::validate_dynamic_tool_namespace;
use super::validate_reserved_dynamic_tool_namespaces;
use crate::JsonSchema;
use crate::ToolDefinition;
use codex_protocol::dynamic_tools::DynamicToolFunctionSpec;
use codex_protocol::dynamic_tools::DynamicToolNamespaceSpec;
use codex_protocol::dynamic_tools::DynamicToolSpec;
use pretty_assertions::assert_eq;
use std::collections::BTreeMap;

#[test]
fn parse_dynamic_tool_sanitizes_input_schema() {
    let tool = DynamicToolFunctionSpec {
        name: "lookup_ticket".to_string(),
        description: "Fetch a ticket".to_string(),
        input_schema: serde_json::json!({
            "properties": {
                "id": {
                    "description": "Ticket identifier"
                }
            }
        }),
        defer_loading: false,
    };

    assert_eq!(
        parse_dynamic_tool(&tool).expect("parse dynamic tool"),
        ToolDefinition {
            name: "lookup_ticket".to_string(),
            description: "Fetch a ticket".to_string(),
            input_schema: JsonSchema::object(
                BTreeMap::from([("id".to_string(), JsonSchema::default(),)]),
                /*required*/ None,
                /*additional_properties*/ None
            ),
            output_schema: None,
            defer_loading: false,
        }
    );
}

#[test]
fn parse_dynamic_tool_preserves_defer_loading() {
    let tool = DynamicToolFunctionSpec {
        name: "lookup_ticket".to_string(),
        description: "Fetch a ticket".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {}
        }),
        defer_loading: true,
    };

    assert_eq!(
        parse_dynamic_tool(&tool).expect("parse dynamic tool"),
        ToolDefinition {
            name: "lookup_ticket".to_string(),
            description: "Fetch a ticket".to_string(),
            input_schema: JsonSchema::object(
                BTreeMap::new(),
                /*required*/ None,
                /*additional_properties*/ None
            ),
            output_schema: None,
            defer_loading: true,
        }
    );
}

#[test]
fn reserved_dynamic_tool_namespaces_are_rejected() {
    for namespace in ["functions", "mcp", "mcp__server"] {
        let tools = vec![DynamicToolSpec::Namespace(DynamicToolNamespaceSpec {
            name: namespace.to_string(),
            description: String::new(),
            tools: Vec::new(),
        })];
        let error = validate_reserved_dynamic_tool_namespaces(&tools, &[])
            .expect_err("namespace should be reserved");
        assert!(error.contains(namespace), "unexpected error: {error}");
    }
    for namespace in ["multi_agent_v1", "agents"] {
        let tools = vec![DynamicToolSpec::Namespace(DynamicToolNamespaceSpec {
            name: namespace.to_string(),
            description: String::new(),
            tools: Vec::new(),
        })];
        let error =
            validate_reserved_dynamic_tool_namespaces(&tools, &["multi_agent_v1", "agents"])
                .expect_err("active runtime namespace should be reserved");
        assert_eq!(
            error,
            format!(
                "dynamic tool namespace collides with an active runtime namespace: {namespace}"
            )
        );
    }

    let tools = vec![DynamicToolSpec::Namespace(DynamicToolNamespaceSpec {
        name: "tickets".to_string(),
        description: String::new(),
        tools: Vec::new(),
    })];
    validate_reserved_dynamic_tool_namespaces(&tools, &["agents"])
        .expect("unrelated namespace should remain available");
}

#[test]
fn mcp_namespace_keeps_its_existing_validation_error_contract() {
    let tools = vec![DynamicToolSpec::Namespace(DynamicToolNamespaceSpec {
        name: "mcp__server".to_string(),
        description: String::new(),
        tools: Vec::new(),
    })];

    let error = validate_reserved_dynamic_tool_namespaces(&tools, &[])
        .expect_err("MCP namespace should be reserved");

    assert_eq!(
        error,
        "dynamic tool namespace is reserved: mcp__server".to_string()
    );
}

#[test]
fn dynamic_namespace_rejects_name_over_shared_limit_without_reflecting_value() {
    let name = "a".repeat(4_096);
    let namespace = DynamicToolNamespaceSpec {
        name: name.clone(),
        description: String::new(),
        tools: Vec::new(),
    };

    let error = validate_dynamic_tool_namespace(&namespace)
        .expect_err("oversized namespace should be rejected");

    assert!(error.contains("at most 64"));
    assert!(!error.contains(&name));
    assert!(error.len() < 256);
}

#[test]
fn dynamic_namespace_checks_size_before_formatting_whitespace_diagnostic() {
    let name = " \n".repeat(50_000);
    let namespace = DynamicToolNamespaceSpec {
        name: name.clone(),
        description: String::new(),
        tools: Vec::new(),
    };

    let error = validate_dynamic_tool_namespace(&namespace)
        .expect_err("oversized whitespace namespace should be rejected");

    assert_eq!(
        error,
        "dynamic tool namespace must be at most 64 characters to match Responses API"
    );
    assert!(!error.contains(&name));
}

#[test]
fn dynamic_namespace_rejects_invalid_identifier_with_bounded_escaped_diagnostic() {
    let namespace = DynamicToolNamespaceSpec {
        name: format!("a{}b", "\n".repeat(58)),
        description: String::new(),
        tools: Vec::new(),
    };

    let error = validate_dynamic_tool_namespace(&namespace)
        .expect_err("invalid namespace should be rejected");

    assert!(error.contains("^[a-zA-Z0-9_-]+$"));
    assert!(!error.contains('\n'));
    assert!(error.len() < 256);
}

#[test]
fn dynamic_namespace_rejects_description_over_shared_limit() {
    let namespace = DynamicToolNamespaceSpec {
        name: "tickets".to_string(),
        description: "a".repeat(1_025),
        tools: Vec::new(),
    };

    let error = validate_dynamic_tool_namespace(&namespace)
        .expect_err("oversized description should be rejected");

    assert_eq!(
        error,
        "dynamic tool namespace description must be at most 1024 characters"
    );
}
