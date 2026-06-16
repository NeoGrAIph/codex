use crate::common::ResponseEvent;
use crate::common::ResponseStream;
use crate::error::ApiError;
use crate::telemetry::SseTelemetry;
use codex_client::ByteStream;
use codex_client::StreamResponse;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ReasoningItemContent;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::TokenUsage;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::Instant;
use tokio::time::timeout;
use tracing::debug;

const REQUEST_ID_HEADER: &str = "x-request-id";

pub fn spawn_chat_completions_stream(
    stream_response: StreamResponse,
    idle_timeout: Duration,
    telemetry: Option<Arc<dyn SseTelemetry>>,
) -> ResponseStream {
    let upstream_request_id = stream_response
        .headers
        .get(REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let (tx_event, rx_event) = mpsc::channel::<Result<ResponseEvent, ApiError>>(1600);
    tokio::spawn(process_chat_sse(
        stream_response.bytes,
        tx_event,
        idle_timeout,
        telemetry,
    ));

    ResponseStream {
        rx_event,
        upstream_request_id,
    }
}

#[derive(Debug, Default)]
struct ChatAccumulator {
    id: Option<String>,
    content: String,
    reasoning_content: String,
    tool_calls: Vec<AccumulatedToolCall>,
    usage: Option<TokenUsage>,
    created_sent: bool,
}

#[derive(Debug, Default)]
struct AccumulatedToolCall {
    index: usize,
    id: Option<String>,
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct ChatChunk {
    id: Option<String>,
    choices: Option<Vec<ChatChoice>>,
    usage: Option<ChatUsage>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    delta: Option<ChatDelta>,
}

#[derive(Debug, Deserialize)]
struct ChatDelta {
    content: Option<String>,
    reasoning_content: Option<String>,
    tool_calls: Option<Vec<ChatDeltaToolCall>>,
}

#[derive(Debug, Deserialize)]
struct ChatDeltaToolCall {
    index: usize,
    id: Option<String>,
    function: Option<ChatDeltaToolCallFunction>,
}

#[derive(Debug, Deserialize)]
struct ChatDeltaToolCallFunction {
    name: Option<String>,
    arguments: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatUsage {
    prompt_tokens: i64,
    completion_tokens: i64,
    total_tokens: i64,
    prompt_tokens_details: Option<ChatPromptTokensDetails>,
    completion_tokens_details: Option<ChatCompletionTokensDetails>,
}

#[derive(Debug, Deserialize)]
struct ChatPromptTokensDetails {
    cached_tokens: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionTokensDetails {
    reasoning_tokens: Option<i64>,
}

impl From<ChatUsage> for TokenUsage {
    fn from(value: ChatUsage) -> Self {
        TokenUsage {
            input_tokens: value.prompt_tokens,
            cached_input_tokens: value
                .prompt_tokens_details
                .and_then(|details| details.cached_tokens)
                .unwrap_or(0),
            output_tokens: value.completion_tokens,
            reasoning_output_tokens: value
                .completion_tokens_details
                .and_then(|details| details.reasoning_tokens)
                .unwrap_or(0),
            total_tokens: value.total_tokens,
        }
    }
}

pub async fn process_chat_sse(
    stream: ByteStream,
    tx_event: mpsc::Sender<Result<ResponseEvent, ApiError>>,
    idle_timeout: Duration,
    telemetry: Option<Arc<dyn SseTelemetry>>,
) {
    let mut stream = stream.eventsource();
    let mut acc = ChatAccumulator::default();

    loop {
        let start = Instant::now();
        let response = timeout(idle_timeout, stream.next()).await;
        if let Some(t) = telemetry.as_ref() {
            t.on_sse_poll(&response, start.elapsed());
        }
        let sse = match response {
            Ok(Some(Ok(sse))) => sse,
            Ok(Some(Err(e))) => {
                debug!("chat completions SSE error: {e:#}");
                let _ = tx_event.send(Err(ApiError::Stream(e.to_string()))).await;
                return;
            }
            Ok(None) => {
                let _ = tx_event
                    .send(Err(ApiError::Stream(
                        "stream closed before chat completion finished".into(),
                    )))
                    .await;
                return;
            }
            Err(_) => {
                let _ = tx_event
                    .send(Err(ApiError::Stream("idle timeout waiting for SSE".into())))
                    .await;
                return;
            }
        };

        if sse.data.trim() == "[DONE]" {
            emit_accumulated_response(acc, tx_event).await;
            return;
        }

        let chunk: ChatChunk = match serde_json::from_str(&sse.data) {
            Ok(chunk) => chunk,
            Err(e) => {
                debug!("failed to parse chat completions SSE chunk: {e}");
                continue;
            }
        };
        if !acc.created_sent {
            acc.created_sent = true;
            if tx_event.send(Ok(ResponseEvent::Created)).await.is_err() {
                return;
            }
        }
        if acc.id.is_none() {
            acc.id = chunk.id;
        }
        if let Some(usage) = chunk.usage {
            acc.usage = Some(usage.into());
        }
        for choice in chunk.choices.unwrap_or_default() {
            if let Some(delta) = choice.delta {
                apply_delta(&mut acc, delta);
            }
        }
    }
}

fn apply_delta(acc: &mut ChatAccumulator, delta: ChatDelta) {
    if let Some(reasoning_content) = delta.reasoning_content {
        acc.reasoning_content.push_str(&reasoning_content);
    }
    if let Some(content) = delta.content {
        acc.content.push_str(&content);
    }
    for tool_call in delta.tool_calls.unwrap_or_default() {
        let index = tool_call.index;
        let position = acc
            .tool_calls
            .iter()
            .position(|call| call.index == index)
            .unwrap_or_else(|| {
                acc.tool_calls.push(AccumulatedToolCall {
                    index,
                    ..Default::default()
                });
                acc.tool_calls.len() - 1
            });
        let call = &mut acc.tool_calls[position];
        if let Some(id) = tool_call.id {
            call.id = Some(id);
        }
        if let Some(function) = tool_call.function {
            if let Some(name) = function.name {
                call.name.push_str(&name);
            }
            if let Some(arguments) = function.arguments {
                call.arguments.push_str(&arguments);
            }
        }
    }
}

async fn emit_accumulated_response(
    acc: ChatAccumulator,
    tx_event: mpsc::Sender<Result<ResponseEvent, ApiError>>,
) {
    let response_id = acc.id.unwrap_or_else(|| "chatcmpl".to_string());
    let has_tool_calls = !acc.tool_calls.is_empty();
    if !acc.reasoning_content.is_empty()
        && tx_event
            .send(Ok(ResponseEvent::OutputItemDone(ResponseItem::Reasoning {
                id: format!("{response_id}_reasoning"),
                summary: Vec::new(),
                content: Some(vec![ReasoningItemContent::ReasoningText {
                    text: acc.reasoning_content,
                }]),
                encrypted_content: None,
            })))
            .await
            .is_err()
    {
        return;
    }
    if !acc.content.is_empty()
        && tx_event
            .send(Ok(ResponseEvent::OutputItemDone(ResponseItem::Message {
                id: None,
                role: "assistant".to_string(),
                content: vec![ContentItem::OutputText { text: acc.content }],
                phase: None,
            })))
            .await
            .is_err()
    {
        return;
    }
    for call in acc.tool_calls {
        let call_id = call
            .id
            .unwrap_or_else(|| format!("{response_id}_tool_{}", call.index));
        if tx_event
            .send(Ok(ResponseEvent::OutputItemDone(
                ResponseItem::FunctionCall {
                    id: None,
                    name: call.name,
                    namespace: None,
                    arguments: call.arguments,
                    call_id,
                },
            )))
            .await
            .is_err()
        {
            return;
        }
    }
    let _ = tx_event
        .send(Ok(ResponseEvent::Completed {
            response_id,
            token_usage: acc.usage,
            end_turn: Some(!has_tool_calls),
        }))
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::TryStreamExt;
    use pretty_assertions::assert_eq;
    use tokio_test::io::Builder as IoBuilder;
    use tokio_util::io::ReaderStream;

    async fn collect_events(chunks: &[&[u8]]) -> Vec<ResponseEvent> {
        let mut builder = IoBuilder::new();
        for chunk in chunks {
            builder.read(chunk);
        }

        let reader = builder.build();
        let stream = ReaderStream::new(reader)
            .map_err(|err| codex_client::TransportError::Network(err.to_string()));
        let (tx, mut rx) = mpsc::channel::<Result<ResponseEvent, ApiError>>(16);
        tokio::spawn(process_chat_sse(
            Box::pin(stream),
            tx,
            Duration::from_secs(1),
            /*telemetry*/ None,
        ));

        let mut events = Vec::new();
        while let Some(ev) = rx.recv().await {
            events.push(ev.expect("chat SSE should parse"));
        }
        events
    }

    #[tokio::test]
    async fn parses_reasoning_tool_call_and_usage() {
        let events = collect_events(&[br#"data: {"id":"chatcmpl-1","choices":[{"delta":{"reasoning_content":"think ","tool_calls":[{"index":0,"id":"call_1","function":{"name":"lookup","arguments":"{\"id\""}}]}}]}

data: {"choices":[{"delta":{"reasoning_content":"more","tool_calls":[{"index":0,"function":{"arguments":":1}"}}]}}]}

data: {"usage":{"prompt_tokens":5,"completion_tokens":7,"total_tokens":12,"prompt_tokens_details":{"cached_tokens":2},"completion_tokens_details":{"reasoning_tokens":3}}}

data: [DONE]

"#]).await;

        assert_eq!(events.len(), 4);
        assert!(matches!(events[0], ResponseEvent::Created));
        match &events[1] {
            ResponseEvent::OutputItemDone(item) => assert_eq!(
                item,
                &ResponseItem::Reasoning {
                    id: "chatcmpl-1_reasoning".to_string(),
                    summary: Vec::new(),
                    content: Some(vec![ReasoningItemContent::ReasoningText {
                        text: "think more".to_string()
                    }]),
                    encrypted_content: None,
                }
            ),
            other => panic!("expected reasoning item, got {other:?}"),
        }
        match &events[2] {
            ResponseEvent::OutputItemDone(item) => assert_eq!(
                item,
                &ResponseItem::FunctionCall {
                    id: None,
                    name: "lookup".to_string(),
                    namespace: None,
                    arguments: "{\"id\":1}".to_string(),
                    call_id: "call_1".to_string(),
                }
            ),
            other => panic!("expected function call item, got {other:?}"),
        }
        match &events[3] {
            ResponseEvent::Completed {
                response_id,
                token_usage,
                end_turn,
            } => {
                assert_eq!(response_id, "chatcmpl-1");
                assert_eq!(*end_turn, Some(false));
                assert_eq!(
                    *token_usage,
                    Some(TokenUsage {
                        input_tokens: 5,
                        cached_input_tokens: 2,
                        output_tokens: 7,
                        reasoning_output_tokens: 3,
                        total_tokens: 12,
                    })
                );
            }
            other => panic!("expected completed event, got {other:?}"),
        }
    }
}
