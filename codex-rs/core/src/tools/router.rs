use crate::client_common::tools::ToolSpec;
use crate::codex::Session;
use crate::codex::TurnContext;
use crate::function_tool::FunctionCallError;
use crate::mcp_connection_manager::ToolInfo;
use crate::sandboxing::SandboxPermissions;
use crate::tools::context::SharedTurnDiffTracker;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::registry::ConfiguredToolSpec;
use crate::tools::registry::ToolRegistry;
use crate::tools::spec::ToolsConfig;
use crate::tools::spec::build_specs;
use codex_protocol::dynamic_tools::DynamicToolSpec;
use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::LocalShellAction;
use codex_protocol::models::ResponseInputItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::models::ShellToolCallParams;
use rmcp::model::Tool;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::instrument;

pub use crate::tools::context::ToolCallSource;

#[derive(Clone, Debug)]
pub struct ToolCall {
    pub tool_name: String,
    pub call_id: String,
    pub payload: ToolPayload,
}

pub struct ToolRouter {
    registry: ToolRegistry,
    specs: Vec<ConfiguredToolSpec>,
    policy: Option<crate::tools::policy::ToolAccessPolicy>,
}

impl ToolRouter {
    pub fn from_config(
        config: &ToolsConfig,
        mcp_tools: Option<HashMap<String, Tool>>,
        app_tools: Option<HashMap<String, ToolInfo>>,
        dynamic_tools: &[DynamicToolSpec],
    ) -> Self {
        let builder = build_specs(config, mcp_tools, app_tools, dynamic_tools);
        let (specs, registry) = builder.build();
        let policy = config.tool_access_policy.clone();
        let specs = match &policy {
            Some(policy) => specs
                .into_iter()
                .filter(|configured| policy.allows_spec(&configured.spec))
                .collect(),
            None => specs,
        };

        Self {
            registry,
            specs,
            policy,
        }
    }

    pub fn specs(&self) -> Vec<ToolSpec> {
        self.specs
            .iter()
            .map(|config| config.spec.clone())
            .collect()
    }

    pub fn tool_supports_parallel(&self, tool_name: &str) -> bool {
        self.specs
            .iter()
            .filter(|config| config.supports_parallel_tool_calls)
            .any(|config| config.spec.name() == tool_name)
    }

    #[instrument(level = "trace", skip_all, err)]
    pub async fn build_tool_call(
        session: &Session,
        item: ResponseItem,
    ) -> Result<Option<ToolCall>, FunctionCallError> {
        match item {
            ResponseItem::FunctionCall {
                name,
                arguments,
                call_id,
                ..
            } => {
                if let Some((server, tool)) = session.parse_mcp_tool_name(&name).await {
                    Ok(Some(ToolCall {
                        tool_name: name,
                        call_id,
                        payload: ToolPayload::Mcp {
                            server,
                            tool,
                            raw_arguments: arguments,
                        },
                    }))
                } else {
                    Ok(Some(ToolCall {
                        tool_name: name,
                        call_id,
                        payload: ToolPayload::Function { arguments },
                    }))
                }
            }
            ResponseItem::CustomToolCall {
                name,
                input,
                call_id,
                ..
            } => Ok(Some(ToolCall {
                tool_name: name,
                call_id,
                payload: ToolPayload::Custom { input },
            })),
            ResponseItem::LocalShellCall {
                id,
                call_id,
                action,
                ..
            } => {
                let call_id = call_id
                    .or(id)
                    .ok_or(FunctionCallError::MissingLocalShellCallId)?;

                match action {
                    LocalShellAction::Exec(exec) => {
                        let params = ShellToolCallParams {
                            command: exec.command,
                            workdir: exec.working_directory,
                            timeout_ms: exec.timeout_ms,
                            sandbox_permissions: Some(SandboxPermissions::UseDefault),
                            additional_permissions: None,
                            prefix_rule: None,
                            justification: None,
                        };
                        Ok(Some(ToolCall {
                            tool_name: "local_shell".to_string(),
                            call_id,
                            payload: ToolPayload::LocalShell { params },
                        }))
                    }
                }
            }
            _ => Ok(None),
        }
    }

    #[instrument(level = "trace", skip_all, err)]
    pub async fn dispatch_tool_call(
        &self,
        session: Arc<Session>,
        turn: Arc<TurnContext>,
        tracker: SharedTurnDiffTracker,
        call: ToolCall,
        source: ToolCallSource,
    ) -> Result<ResponseInputItem, FunctionCallError> {
        let ToolCall {
            tool_name,
            call_id,
            payload,
        } = call;
        let payload_outputs_custom = matches!(payload, ToolPayload::Custom { .. });
        let failure_call_id = call_id.clone();

        if source == ToolCallSource::Direct
            && turn.tools_config.js_repl_tools_only
            && !matches!(tool_name.as_str(), "js_repl" | "js_repl_reset")
        {
            let err = FunctionCallError::RespondToModel(
                "direct tool calls are disabled; use js_repl and codex.tool(...) instead"
                    .to_string(),
            );
            return Ok(Self::failure_response(
                failure_call_id,
                payload_outputs_custom,
                err,
            ));
        }

        if self
            .policy
            .as_ref()
            .is_some_and(|policy| !policy.allows_tool_name(tool_name.as_str()))
        {
            let err = FunctionCallError::RespondToModel(format!(
                "tool {tool_name} is not permitted for this spawned agent"
            ));
            return Ok(Self::failure_response(
                failure_call_id,
                payload_outputs_custom,
                err,
            ));
        }

        let invocation = ToolInvocation {
            session,
            turn,
            tracker,
            call_id,
            tool_name,
            payload,
        };

        match self.registry.dispatch(invocation).await {
            Ok(response) => Ok(response),
            Err(FunctionCallError::Fatal(message)) => Err(FunctionCallError::Fatal(message)),
            Err(err) => Ok(Self::failure_response(
                failure_call_id,
                payload_outputs_custom,
                err,
            )),
        }
    }

    fn failure_response(
        call_id: String,
        payload_outputs_custom: bool,
        err: FunctionCallError,
    ) -> ResponseInputItem {
        let message = err.to_string();
        if payload_outputs_custom {
            ResponseInputItem::CustomToolCallOutput {
                call_id,
                output: codex_protocol::models::FunctionCallOutputPayload {
                    body: FunctionCallOutputBody::Text(message),
                    success: Some(false),
                },
            }
        } else {
            ResponseInputItem::FunctionCallOutput {
                call_id,
                output: codex_protocol::models::FunctionCallOutputPayload {
                    body: FunctionCallOutputBody::Text(message),
                    success: Some(false),
                },
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use crate::client_common::tools::ToolSpec;
    use std::collections::HashMap;
    use std::sync::Arc;

    use crate::codex::make_session_and_context;
    use crate::tools::context::ToolPayload;
    use crate::tools::spec::ToolsConfig;
    use crate::tools::spec::ToolsConfigParams;
    use crate::turn_diff_tracker::TurnDiffTracker;
    use codex_protocol::ThreadId;
    use codex_protocol::models::ResponseInputItem;
    use codex_protocol::protocol::SessionSource;
    use codex_protocol::protocol::SubAgentSource;
    use pretty_assertions::assert_eq;

    use super::ToolCall;
    use super::ToolCallSource;
    use super::ToolRouter;

    fn thread_spawn_source(
        allow_list: Option<&[&str]>,
        deny_list: Option<&[&str]>,
    ) -> SessionSource {
        SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id: ThreadId::new(),
            depth: 1,
            agent_nickname: Some("Euler".to_string()),
            agent_role: Some("worker".to_string()),
            agent_persona: None,
            allow_list: allow_list
                .map(|items| items.iter().map(std::string::ToString::to_string).collect()),
            deny_list: deny_list
                .map(|items| items.iter().map(std::string::ToString::to_string).collect()),
            thread_note: None,
        })
    }

    fn rebuild_tools_config(turn: &mut crate::codex::TurnContext) {
        turn.tools_config = ToolsConfig::new(&ToolsConfigParams {
            model_info: &turn.model_info,
            features: &turn.features,
            web_search_mode: turn.tools_config.web_search_mode,
            session_source: turn.session_source.clone(),
        })
        .with_allow_login_shell(turn.tools_config.allow_login_shell)
        .with_agent_roles(turn.config.agent_roles.clone());
    }

    fn mcp_tool(
        name: &str,
        description: &str,
        input_schema: serde_json::Value,
    ) -> rmcp::model::Tool {
        rmcp::model::Tool {
            name: name.to_string().into(),
            title: None,
            description: Some(description.to_string().into()),
            input_schema: std::sync::Arc::new(rmcp::model::object(input_schema)),
            output_schema: None,
            annotations: None,
            execution: None,
            icons: None,
            meta: None,
        }
    }

    #[tokio::test]
    async fn js_repl_tools_only_blocks_direct_tool_calls() -> anyhow::Result<()> {
        let (session, mut turn) = make_session_and_context().await;
        turn.tools_config.js_repl_tools_only = true;

        let session = Arc::new(session);
        let turn = Arc::new(turn);
        let mcp_tools = session
            .services
            .mcp_connection_manager
            .read()
            .await
            .list_all_tools()
            .await;
        let app_tools = Some(mcp_tools.clone());
        let router = ToolRouter::from_config(
            &turn.tools_config,
            Some(
                mcp_tools
                    .into_iter()
                    .map(|(name, tool)| (name, tool.tool))
                    .collect(),
            ),
            app_tools,
            turn.dynamic_tools.as_slice(),
        );

        let call = ToolCall {
            tool_name: "shell".to_string(),
            call_id: "call-1".to_string(),
            payload: ToolPayload::Function {
                arguments: "{}".to_string(),
            },
        };
        let tracker = Arc::new(tokio::sync::Mutex::new(TurnDiffTracker::new()));
        let response = router
            .dispatch_tool_call(session, turn, tracker, call, ToolCallSource::Direct)
            .await?;

        match response {
            ResponseInputItem::FunctionCallOutput { output, .. } => {
                let content = output.text_content().unwrap_or_default();
                assert!(
                    content.contains("direct tool calls are disabled"),
                    "unexpected tool call message: {content}",
                );
            }
            other => panic!("expected function call output, got {other:?}"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn js_repl_tools_only_allows_js_repl_source_calls() -> anyhow::Result<()> {
        let (session, mut turn) = make_session_and_context().await;
        turn.tools_config.js_repl_tools_only = true;

        let session = Arc::new(session);
        let turn = Arc::new(turn);
        let mcp_tools = session
            .services
            .mcp_connection_manager
            .read()
            .await
            .list_all_tools()
            .await;
        let app_tools = Some(mcp_tools.clone());
        let router = ToolRouter::from_config(
            &turn.tools_config,
            Some(
                mcp_tools
                    .into_iter()
                    .map(|(name, tool)| (name, tool.tool))
                    .collect(),
            ),
            app_tools,
            turn.dynamic_tools.as_slice(),
        );

        let call = ToolCall {
            tool_name: "shell".to_string(),
            call_id: "call-2".to_string(),
            payload: ToolPayload::Function {
                arguments: "{}".to_string(),
            },
        };
        let tracker = Arc::new(tokio::sync::Mutex::new(TurnDiffTracker::new()));
        let response = router
            .dispatch_tool_call(session, turn, tracker, call, ToolCallSource::JsRepl)
            .await?;

        match response {
            ResponseInputItem::FunctionCallOutput { output, .. } => {
                let content = output.text_content().unwrap_or_default();
                assert!(
                    !content.contains("direct tool calls are disabled"),
                    "js_repl source should bypass direct-call policy gate"
                );
            }
            other => panic!("expected function call output, got {other:?}"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn thread_spawn_policy_hides_denied_tools_from_specs() -> anyhow::Result<()> {
        let (session, mut turn) = make_session_and_context().await;
        turn.session_source = thread_spawn_source(Some(&["view_image"]), None);
        rebuild_tools_config(&mut turn);

        let session = Arc::new(session);
        let turn = Arc::new(turn);
        let mcp_tools = session
            .services
            .mcp_connection_manager
            .read()
            .await
            .list_all_tools()
            .await;
        let app_tools = Some(mcp_tools.clone());
        let router = ToolRouter::from_config(
            &turn.tools_config,
            Some(
                mcp_tools
                    .into_iter()
                    .map(|(name, tool)| (name, tool.tool))
                    .collect(),
            ),
            app_tools,
            turn.dynamic_tools.as_slice(),
        );

        let tool_names = router
            .specs()
            .into_iter()
            .map(|tool| match tool {
                ToolSpec::Function(spec) => spec.name,
                ToolSpec::LocalShell {} => "local_shell".to_string(),
                ToolSpec::ImageGeneration {} => "image_generation".to_string(),
                ToolSpec::WebSearch { .. } => "web_search".to_string(),
                ToolSpec::Freeform(spec) => spec.name,
            })
            .collect::<Vec<_>>();

        assert_eq!(tool_names, vec!["view_image".to_string()]);

        Ok(())
    }

    #[tokio::test]
    async fn thread_spawn_policy_blocks_direct_tool_calls() -> anyhow::Result<()> {
        let (session, mut turn) = make_session_and_context().await;
        turn.session_source = thread_spawn_source(None, Some(&["shell"]));
        rebuild_tools_config(&mut turn);

        let session = Arc::new(session);
        let turn = Arc::new(turn);
        let mcp_tools = session
            .services
            .mcp_connection_manager
            .read()
            .await
            .list_all_tools()
            .await;
        let app_tools = Some(mcp_tools.clone());
        let router = ToolRouter::from_config(
            &turn.tools_config,
            Some(
                mcp_tools
                    .into_iter()
                    .map(|(name, tool)| (name, tool.tool))
                    .collect(),
            ),
            app_tools,
            turn.dynamic_tools.as_slice(),
        );

        let call = ToolCall {
            tool_name: "shell".to_string(),
            call_id: "call-policy-direct".to_string(),
            payload: ToolPayload::Function {
                arguments: "{}".to_string(),
            },
        };
        let tracker = Arc::new(tokio::sync::Mutex::new(TurnDiffTracker::new()));
        let response = router
            .dispatch_tool_call(session, turn, tracker, call, ToolCallSource::Direct)
            .await?;

        match response {
            ResponseInputItem::FunctionCallOutput { output, .. } => {
                assert_eq!(
                    output.text_content(),
                    Some("tool shell is not permitted for this spawned agent")
                );
            }
            other => panic!("expected function call output, got {other:?}"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn thread_spawn_policy_blocks_js_repl_tool_calls() -> anyhow::Result<()> {
        let (session, mut turn) = make_session_and_context().await;
        turn.session_source = thread_spawn_source(None, Some(&["shell"]));
        rebuild_tools_config(&mut turn);

        let session = Arc::new(session);
        let turn = Arc::new(turn);
        let mcp_tools = session
            .services
            .mcp_connection_manager
            .read()
            .await
            .list_all_tools()
            .await;
        let app_tools = Some(mcp_tools.clone());
        let router = ToolRouter::from_config(
            &turn.tools_config,
            Some(
                mcp_tools
                    .into_iter()
                    .map(|(name, tool)| (name, tool.tool))
                    .collect(),
            ),
            app_tools,
            turn.dynamic_tools.as_slice(),
        );

        let call = ToolCall {
            tool_name: "shell".to_string(),
            call_id: "call-policy-jsrepl".to_string(),
            payload: ToolPayload::Function {
                arguments: "{}".to_string(),
            },
        };
        let tracker = Arc::new(tokio::sync::Mutex::new(TurnDiffTracker::new()));
        let response = router
            .dispatch_tool_call(session, turn, tracker, call, ToolCallSource::JsRepl)
            .await?;

        match response {
            ResponseInputItem::FunctionCallOutput { output, .. } => {
                assert_eq!(
                    output.text_content(),
                    Some("tool shell is not permitted for this spawned agent")
                );
            }
            other => panic!("expected function call output, got {other:?}"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn thread_spawn_policy_blocks_qualified_mcp_tool_calls() -> anyhow::Result<()> {
        let (session, mut turn) = make_session_and_context().await;
        turn.session_source = thread_spawn_source(None, Some(&["mcp__atlas__read_file"]));
        rebuild_tools_config(&mut turn);

        let session = Arc::new(session);
        let turn = Arc::new(turn);
        let router = ToolRouter::from_config(
            &turn.tools_config,
            Some(HashMap::from([(
                "mcp__atlas__read_file".to_string(),
                mcp_tool(
                    "read_file",
                    "Read a file",
                    serde_json::json!({
                        "type": "object",
                        "properties": {},
                    }),
                ),
            )])),
            None,
            turn.dynamic_tools.as_slice(),
        );

        assert!(
            !router
                .specs()
                .iter()
                .any(|tool| tool.name() == "mcp__atlas__read_file")
        );

        let call = ToolCall {
            tool_name: "mcp__atlas__read_file".to_string(),
            call_id: "call-policy-mcp".to_string(),
            payload: ToolPayload::Mcp {
                server: "atlas".to_string(),
                tool: "read_file".to_string(),
                raw_arguments: "{}".to_string(),
            },
        };
        let tracker = Arc::new(tokio::sync::Mutex::new(TurnDiffTracker::new()));
        let response = router
            .dispatch_tool_call(session, turn, tracker, call, ToolCallSource::Direct)
            .await?;

        match response {
            ResponseInputItem::FunctionCallOutput { output, .. } => {
                assert_eq!(
                    output.text_content(),
                    Some("tool mcp__atlas__read_file is not permitted for this spawned agent")
                );
            }
            other => panic!("expected function call output, got {other:?}"),
        }

        Ok(())
    }
}
