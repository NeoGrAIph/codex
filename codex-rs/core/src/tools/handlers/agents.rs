use async_trait::async_trait;
use codex_protocol::openai_models::ReasoningEffort;
use serde::Deserialize;
use serde::Serialize;

use crate::agent::AgentStatus;
use crate::agent::registry::AgentDefinition;
use crate::agent::registry::AgentRegistry;
use crate::agent::registry::AgentScope;
use crate::agent::status::is_final;
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
struct ListAgentsArgs {
    #[serde(default)]
    only_active: bool,
}

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
    #[serde(skip_serializing_if = "Option::is_none")]
    active_agents: Option<Vec<ActiveAgentSummary>>,
}

#[derive(Debug, Serialize)]
struct AgentNameSummary {
    name: String,
    description: String,
}

#[derive(Debug, Serialize)]
struct ActiveAgentSummary {
    id: String,
    status: AgentStatus,
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
        let ToolInvocation {
            payload,
            turn,
            session,
            ..
        } = invocation;
        let arguments = match payload {
            ToolPayload::Function { arguments } => arguments,
            _ => {
                return Err(FunctionCallError::RespondToModel(
                    "list_agents handler received unsupported payload".to_string(),
                ));
            }
        };

        let args = if arguments.trim().is_empty() {
            ListAgentsArgs { only_active: false }
        } else {
            parse_arguments::<ListAgentsArgs>(&arguments)?
        };
        let only_active = args.only_active;

        let active_agents = if only_active {
            let agent_ids = session
                .services
                .agent_control
                .list_agent_ids()
                .await
                .map_err(|err| {
                    FunctionCallError::RespondToModel(format!(
                        "failed to list active agents: {err}"
                    ))
                })?;
            let mut active = Vec::new();
            for agent_id in agent_ids {
                if agent_id == session.conversation_id {
                    continue;
                }
                let status = session.services.agent_control.get_status(agent_id).await;
                if is_final(&status) {
                    continue;
                }
                active.push(ActiveAgentSummary {
                    id: agent_id.to_string(),
                    status,
                });
            }
            Some(active)
        } else {
            None
        };

        if only_active {
            let count = active_agents.as_ref().map(|list| list.len()).unwrap_or(0);
            let response = AgentListResponse {
                agents: Vec::new(),
                count,
                active_agents,
            };
            let content = serde_json::to_string(&response).map_err(|err| {
                FunctionCallError::Fatal(format!("failed to serialize list_agents result: {err}"))
            })?;
            return Ok(ToolOutput::Function {
                content,
                content_items: None,
                success: Some(true),
            });
        }

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
            active_agents: None,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CodexAuth;
    use crate::ThreadManager;
    use crate::built_in_model_providers;
    use crate::codex::make_session_and_context;
    use crate::protocol::Op;
    use crate::tools::context::ToolPayload;
    use crate::turn_diff_tracker::TurnDiffTracker;
    use pretty_assertions::assert_eq;
    use serde::Deserialize;
    use serde_json::json;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[derive(Debug, Deserialize)]
    struct ActiveAgentEntry {
        id: String,
        status: AgentStatus,
    }

    #[derive(Debug, Deserialize)]
    struct ListAgentsOutput {
        agents: Vec<serde_json::Value>,
        count: usize,
        active_agents: Option<Vec<ActiveAgentEntry>>,
    }

    fn invocation(
        session: Arc<crate::codex::Session>,
        turn: Arc<crate::codex::TurnContext>,
        args: serde_json::Value,
    ) -> ToolInvocation {
        ToolInvocation {
            session,
            turn,
            tracker: Arc::new(Mutex::new(TurnDiffTracker::default())),
            call_id: "call-1".to_string(),
            tool_name: "list_agents".to_string(),
            payload: ToolPayload::Function {
                arguments: args.to_string(),
            },
        }
    }

    fn thread_manager() -> ThreadManager {
        ThreadManager::with_models_provider(
            CodexAuth::from_api_key("dummy"),
            built_in_model_providers()["openai"].clone(),
        )
    }

    #[tokio::test]
    async fn list_agents_only_active_returns_active_agents() {
        let (mut session, turn) = make_session_and_context().await;
        let manager = thread_manager();
        session.services.agent_control = manager.agent_control();
        let config = turn.client.config().as_ref().clone();
        let thread = manager.start_thread(config).await.expect("start thread");

        let invocation = invocation(
            Arc::new(session),
            Arc::new(turn),
            json!({"only_active": true}),
        );
        let output = ListAgentsHandler
            .handle(invocation)
            .await
            .expect("list_agents should succeed");

        let ToolOutput::Function {
            content, success, ..
        } = output
        else {
            panic!("expected function output");
        };
        assert_eq!(success, Some(true));
        let result: ListAgentsOutput =
            serde_json::from_str(&content).expect("list_agents result should be json");
        let active = result.active_agents.expect("active agents expected");
        assert_eq!(result.agents.len(), 0);
        assert_eq!(result.count, active.len());
        assert!(
            active
                .iter()
                .any(|entry| entry.id == thread.thread_id.to_string())
        );
        assert!(active.iter().all(|entry| !is_final(&entry.status)));

        let _ = thread
            .thread
            .submit(Op::Shutdown {})
            .await
            .expect("shutdown should submit");
    }

    #[tokio::test]
    async fn list_agents_keeps_no_agents_message_when_registry_empty() {
        let (session, turn) = make_session_and_context().await;
        let invocation = invocation(Arc::new(session), Arc::new(turn), json!({}));
        let output = ListAgentsHandler
            .handle(invocation)
            .await
            .expect("list_agents should succeed");
        let ToolOutput::Function {
            content, success, ..
        } = output
        else {
            panic!("expected function output");
        };
        assert_eq!(content, "No agents found in the registry.");
        assert_eq!(success, Some(false));
    }
}
