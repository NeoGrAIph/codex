use std::sync::Arc;
use std::time::Instant;

use tokio::sync::RwLock;
use tokio_util::either::Either;
use tokio_util::sync::CancellationToken;
use tokio_util::task::AbortOnDropHandle;
use tracing::Instrument;
use tracing::instrument;
use tracing::trace_span;

use crate::codex::Session;
use crate::codex::TurnContext;
use crate::error::CodexErr;
use crate::function_tool::FunctionCallError;
use crate::tools::context::SharedTurnDiffTracker;
use crate::tools::context::ToolPayload;
use crate::tools::router::ToolCall;
use crate::tools::router::ToolRouter;
use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ResponseInputItem;

#[derive(Clone)]
pub(crate) struct ToolCallRuntime {
    router: Arc<ToolRouter>,
    session: Arc<Session>,
    turn_context: Arc<TurnContext>,
    tracker: SharedTurnDiffTracker,
    parallel_execution: Arc<RwLock<()>>,
}

impl ToolCallRuntime {
    pub(crate) fn new(
        router: Arc<ToolRouter>,
        session: Arc<Session>,
        turn_context: Arc<TurnContext>,
        tracker: SharedTurnDiffTracker,
    ) -> Self {
        Self {
            router,
            session,
            turn_context,
            tracker,
            parallel_execution: Arc::new(RwLock::new(())),
        }
    }

    #[instrument(level = "trace", skip_all, fields(call = ?call))]
    pub(crate) fn handle_tool_call(
        self,
        call: ToolCall,
        cancellation_token: CancellationToken,
    ) -> impl std::future::Future<Output = Result<ResponseInputItem, CodexErr>> {
        let supports_parallel = self.router.tool_supports_parallel(&call.tool_name);

        let router = Arc::clone(&self.router);
        let session = Arc::clone(&self.session);
        let turn = Arc::clone(&self.turn_context);
        let tracker = Arc::clone(&self.tracker);
        let lock = Arc::clone(&self.parallel_execution);
        let started = Instant::now();

        let dispatch_span = trace_span!(
            "dispatch_tool_call",
            otel.name = call.tool_name.as_str(),
            tool_name = call.tool_name.as_str(),
            call_id = call.call_id.as_str(),
            aborted = false,
        );

        let handle: AbortOnDropHandle<Result<ResponseInputItem, FunctionCallError>> =
            AbortOnDropHandle::new(tokio::spawn(async move {
                tokio::select! {
                    _ = cancellation_token.cancelled() => {
                        let secs = started.elapsed().as_secs_f32().max(0.1);
                        dispatch_span.record("aborted", true);
                        Ok(Self::aborted_response(&call, secs))
                    },
                    res = async {
                        let _guard = if supports_parallel {
                            Either::Left(lock.read().await)
                        } else {
                            Either::Right(lock.write().await)
                        };

                        router
                            .dispatch_tool_call(
                                session,
                                turn,
                                tracker,
                                call.clone(),
                                crate::tools::router::ToolCallSource::Direct,
                            )
                            .instrument(dispatch_span.clone())
                            .await
                    } => res,
                }
            }));

        async move {
            match handle.await {
                Ok(Ok(response)) => Ok(response),
                Ok(Err(FunctionCallError::Fatal(message))) => Err(CodexErr::Fatal(message)),
                Ok(Err(other)) => Err(CodexErr::Fatal(other.to_string())),
                Err(err) => Err(CodexErr::Fatal(format!(
                    "tool task failed to receive: {err:?}"
                ))),
            }
        }
        .in_current_span()
    }
}

impl ToolCallRuntime {
    fn aborted_response(call: &ToolCall, secs: f32) -> ResponseInputItem {
        match &call.payload {
            ToolPayload::Custom { .. } => ResponseInputItem::CustomToolCallOutput {
                call_id: call.call_id.clone(),
                output: FunctionCallOutputPayload {
                    body: FunctionCallOutputBody::Text(Self::abort_message(call, secs)),
                    ..Default::default()
                },
            },
            ToolPayload::Mcp { .. } => ResponseInputItem::McpToolCallOutput {
                call_id: call.call_id.clone(),
                result: Err(Self::abort_message(call, secs)),
            },
            _ => ResponseInputItem::FunctionCallOutput {
                call_id: call.call_id.clone(),
                output: FunctionCallOutputPayload {
                    body: FunctionCallOutputBody::Text(Self::abort_message(call, secs)),
                    ..Default::default()
                },
            },
        }
    }

    fn abort_message(call: &ToolCall, secs: f32) -> String {
        match call.tool_name.as_str() {
            "shell" | "container.exec" | "local_shell" | "shell_command" | "unified_exec" => {
                format!("Wall time: {secs:.1} seconds\naborted by user")
            }
            _ => format!("aborted by user after {secs:.1}s"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ToolCallRuntime;
    use crate::codex::make_session_and_context;
    use crate::tools::context::ToolPayload;
    use crate::tools::router::ToolCall;
    use crate::tools::router::ToolRouter;
    use crate::tools::spec::ToolsConfig;
    use crate::tools::spec::ToolsConfigParams;
    use crate::turn_diff_tracker::TurnDiffTracker;
    use codex_protocol::ThreadId;
    use codex_protocol::models::ResponseInputItem;
    use codex_protocol::protocol::SessionSource;
    use codex_protocol::protocol::SubAgentSource;
    use pretty_assertions::assert_eq;
    use std::sync::Arc;
    use tokio_util::sync::CancellationToken;

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

    #[tokio::test]
    async fn handle_tool_call_blocks_forbidden_tools() -> anyhow::Result<()> {
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
        let router = Arc::new(ToolRouter::from_config(
            &turn.tools_config,
            Some(
                mcp_tools
                    .into_iter()
                    .map(|(name, tool)| (name, tool.tool))
                    .collect(),
            ),
            app_tools,
            turn.dynamic_tools.as_slice(),
        ));
        let runtime = ToolCallRuntime::new(
            router,
            Arc::clone(&session),
            Arc::clone(&turn),
            Arc::new(tokio::sync::Mutex::new(TurnDiffTracker::new())),
        );

        let response = runtime
            .handle_tool_call(
                ToolCall {
                    tool_name: "shell".to_string(),
                    call_id: "parallel-policy".to_string(),
                    payload: ToolPayload::Function {
                        arguments: "{}".to_string(),
                    },
                },
                CancellationToken::new(),
            )
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
}
