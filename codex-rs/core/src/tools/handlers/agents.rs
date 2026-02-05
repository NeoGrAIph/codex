use async_trait::async_trait;
use codex_protocol::openai_models::ReasoningEffort;
use serde::Deserialize;
use serde::Serialize;

use crate::agent::registry::AgentDefinition;
use crate::agent::registry::AgentRegistry;
use crate::agent::registry::AgentScope;
use crate::function_tool::FunctionCallError;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolOutput;
use crate::tools::context::ToolPayload;
use crate::tools::handlers::parse_arguments;
use crate::tools::registry::ToolHandler;
use crate::tools::registry::ToolKind;

pub struct ListAgentsHandler;
pub struct ReadAgentHandler;

#[derive(Debug, Deserialize)]
struct ReadAgentArgs {
    agent_name: String,
}

#[derive(Debug, Serialize)]
struct AgentSummary {
    name: String,
    description: String,
    model: String,
    reasoning_effort: Option<ReasoningEffort>,
    tools: Option<Vec<String>>,
    scope: String,
    agent_names: Option<Vec<AgentNameSummary>>,
}

#[derive(Debug, Serialize)]
struct AgentListResponse {
    agents: Vec<AgentSummary>,
    count: usize,
}

#[derive(Debug, Serialize)]
struct AgentNameSummary {
    name: String,
    description: String,
}

#[derive(Debug, Serialize)]
struct AgentDetailResponse {
    name: String,
    description: String,
    model: String,
    reasoning_effort: Option<ReasoningEffort>,
    tools: Option<Vec<String>>,
    scope: String,
    instructions: String,
    agent_names: Option<Vec<AgentNameDetail>>,
}

#[derive(Debug, Serialize)]
struct AgentNameDetail {
    name: String,
    description: String,
    instructions: String,
}

fn scope_label(scope: AgentScope) -> &'static str {
    match scope {
        AgentScope::Repo => "repo",
        AgentScope::User => "user",
        AgentScope::System => "system",
    }
}

fn build_agent_name_details(agent: &AgentDefinition) -> Option<Vec<AgentNameDetail>> {
    let mut agent_names = agent
        .agent_name_descriptions
        .iter()
        .map(|(name, description)| AgentNameDetail {
            name: name.clone(),
            description: description.clone(),
            instructions: agent
                .agent_name_instructions
                .get(name)
                .cloned()
                .unwrap_or_default(),
        })
        .collect::<Vec<_>>();
    agent_names.sort_by(|a, b| a.name.cmp(&b.name));
    if agent_names.is_empty() {
        None
    } else {
        Some(agent_names)
    }
}

#[async_trait]
impl ToolHandler for ListAgentsHandler {
    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<ToolOutput, FunctionCallError> {
        let ToolInvocation { payload, turn, .. } = invocation;
        match payload {
            ToolPayload::Function { .. } => {}
            _ => {
                return Err(FunctionCallError::RespondToModel(
                    "list_agents handler received unsupported payload".to_string(),
                ));
            }
        };

        let registry = AgentRegistry::load_for_config(turn.client.config().as_ref());
        if registry.agents.is_empty() {
            return Ok(ToolOutput::Function {
                content: "No agents found in the registry.".to_string(),
                content_items: None,
                success: Some(false),
            });
        }

        let agents = registry
            .agents
            .iter()
            .map(|agent| {
                let mut agent_names = agent
                    .agent_name_descriptions
                    .iter()
                    .map(|(name, description)| AgentNameSummary {
                        name: name.clone(),
                        description: description.clone(),
                    })
                    .collect::<Vec<_>>();
                agent_names.sort_by(|a, b| a.name.cmp(&b.name));
                let agent_names = if agent_names.is_empty() {
                    None
                } else {
                    Some(agent_names)
                };
                AgentSummary {
                    name: agent.name.clone(),
                    description: agent.description.clone(),
                    model: agent.model.clone(),
                    reasoning_effort: agent.reasoning_effort,
                    tools: agent.tools.clone(),
                    scope: scope_label(agent.scope).to_string(),
                    agent_names,
                }
            })
            .collect::<Vec<_>>();

        let response = AgentListResponse {
            count: agents.len(),
            agents,
        };
        let content = serde_json::to_string(&response).map_err(|err| {
            FunctionCallError::Fatal(format!("failed to serialize list_agents result: {err}"))
        })?;

        Ok(ToolOutput::Function {
            content,
            content_items: None,
            success: Some(true),
        })
    }
}

#[async_trait]
impl ToolHandler for ReadAgentHandler {
    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<ToolOutput, FunctionCallError> {
        let ToolInvocation { payload, turn, .. } = invocation;
        let arguments = match payload {
            ToolPayload::Function { arguments } => arguments,
            _ => {
                return Err(FunctionCallError::RespondToModel(
                    "read_agent handler received unsupported payload".to_string(),
                ));
            }
        };

        let args: ReadAgentArgs = parse_arguments(&arguments)?;
        let registry = AgentRegistry::load_for_config(turn.client.config().as_ref());
        let Some(agent) = registry.find(&args.agent_name) else {
            return Err(FunctionCallError::RespondToModel(format!(
                "agent \"{}\" not found",
                args.agent_name
            )));
        };

        let response = AgentDetailResponse {
            name: agent.name.clone(),
            description: agent.description.clone(),
            model: agent.model.clone(),
            reasoning_effort: agent.reasoning_effort,
            tools: agent.tools.clone(),
            scope: scope_label(agent.scope).to_string(),
            instructions: agent.instructions.clone(),
            agent_names: build_agent_name_details(agent),
        };
        let content = serde_json::to_string(&response).map_err(|err| {
            FunctionCallError::Fatal(format!("failed to serialize read_agent result: {err}"))
        })?;

        Ok(ToolOutput::Function {
            content,
            content_items: None,
            success: Some(true),
        })
    }
}
