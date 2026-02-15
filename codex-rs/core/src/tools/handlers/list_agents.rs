// FORK COMMIT NEW FILE [SA]: list_agents tool handler backed by agent templates.
// Role: expose strict role/persona metadata to orchestrator and sub-agents.
use async_trait::async_trait;
use codex_protocol::models::FunctionCallOutputBody;
use serde::Deserialize;
use serde::Serialize;

use crate::agent::role_templates::ListAgentsSummary;
use crate::function_tool::FunctionCallError;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolOutput;
use crate::tools::context::ToolPayload;
use crate::tools::handlers::parse_arguments;
use crate::tools::registry::ToolHandler;
use crate::tools::registry::ToolKind;

pub struct ListAgentsHandler;

#[derive(Debug, Serialize)]
struct ListAgentsResponse {
    agents: Vec<ListAgentsSummary>,
}

#[derive(Debug, Deserialize, Default)]
// FORK COMMIT OPEN [SA]: list_agents query args.
// Role: support filtered role lookup and opt-in expanded metadata.
struct ListAgentsArgs {
    agent_type: Option<String>,
    #[serde(default)]
    expanded: bool,
}
// FORK COMMIT CLOSE: list_agents query args.

#[async_trait]
impl ToolHandler for ListAgentsHandler {
    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<ToolOutput, FunctionCallError> {
        let ToolInvocation { payload, .. } = invocation;
        let arguments = match payload {
            ToolPayload::Function { arguments } => arguments,
            _ => {
                return Err(FunctionCallError::RespondToModel(
                    "list_agents handler received unsupported payload".to_string(),
                ));
            }
        };

        let args: ListAgentsArgs = parse_arguments(&arguments)?;
        // FORK COMMIT [SA]: forward list_agents query options to strict template reader.
        let agents = crate::agent::role_templates::list_agents_summaries(
            args.agent_type.as_deref(),
            args.expanded,
        )
        .map_err(FunctionCallError::RespondToModel)?;
        let content = serde_json::to_string(&ListAgentsResponse { agents }).map_err(|err| {
            FunctionCallError::Fatal(format!("failed to serialize list_agents result: {err}"))
        })?;

        Ok(ToolOutput::Function {
            body: FunctionCallOutputBody::Text(content),
            success: Some(true),
        })
    }
}
