use std::collections::BTreeMap;
use std::sync::Arc;

use crate::client_common::tools::ResponsesApiTool;
use crate::client_common::tools::ToolSpec;
use crate::tools::handlers::ListAgentsHandler;
use crate::tools::handlers::ReadAgentHandler;
use crate::tools::registry::ToolRegistryBuilder;

use super::JsonSchema;
use super::ToolsConfig;

pub(super) fn register_agent_tools(builder: &mut ToolRegistryBuilder, config: &ToolsConfig) {
    if !config.agent_registry_present {
        return;
    }

    let list_agents_handler = Arc::new(ListAgentsHandler);
    let read_agent_handler = Arc::new(ReadAgentHandler);

    builder.push_spec_with_parallel_support(create_list_agents_tool(), true);
    builder.register_handler("list_agents", list_agents_handler);
    builder.push_spec_with_parallel_support(create_read_agent_tool(), true);
    builder.register_handler("read_agent", read_agent_handler);
}

fn create_list_agents_tool() -> ToolSpec {
    let properties = BTreeMap::from([(
        "only_active".to_string(),
        JsonSchema::Boolean {
            description: Some(
                "When true, returns only active agent sessions with status.".to_string(),
            ),
        },
    )]);
    ToolSpec::Function(ResponsesApiTool {
        name: "list_agents".to_string(),
        description: "List available agent profiles from the local registry. Set only_active=true to return only active agent sessions with status.".to_string(),
        strict: false,
        parameters: JsonSchema::Object {
            properties,
            required: None,
            additional_properties: Some(false.into()),
        },
    })
}

fn create_read_agent_tool() -> ToolSpec {
    let properties = BTreeMap::from([(
        "agent_name".to_string(),
        JsonSchema::String {
            description: Some(
                "Agent name from list_agents. Returns default instructions plus agent_name instructions."
                    .to_string(),
            ),
        },
    )]);

    ToolSpec::Function(ResponsesApiTool {
        name: "read_agent".to_string(),
        description: "Read the selected agent profile body (instructions and agent_name variants)."
            .to_string(),
        strict: false,
        parameters: JsonSchema::Object {
            properties,
            required: Some(vec!["agent_name".to_string()]),
            additional_properties: Some(false.into()),
        },
    })
}
