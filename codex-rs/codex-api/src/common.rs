use crate::error::ApiError;
use codex_protocol::config_types::ReasoningSummary as ReasoningSummaryConfig;
use codex_protocol::config_types::Verbosity as VerbosityConfig;
use codex_protocol::models::ContentItem;
use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::ReasoningItemContent;
use codex_protocol::models::ResponseItem;
use codex_protocol::models::function_call_output_content_items_to_text;
use codex_protocol::openai_models::ReasoningEffort as ReasoningEffortConfig;
use codex_protocol::protocol::ModelVerification;
use codex_protocol::protocol::RateLimitSnapshot;
use codex_protocol::protocol::TokenUsage;
use codex_protocol::protocol::W3cTraceContext;
use futures::Stream;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use tokio::sync::mpsc;

pub const WS_REQUEST_HEADER_TRACEPARENT_CLIENT_METADATA_KEY: &str = "ws_request_header_traceparent";
pub const WS_REQUEST_HEADER_TRACESTATE_CLIENT_METADATA_KEY: &str = "ws_request_header_tracestate";

/// Canonical input payload for the compaction endpoint.
#[derive(Debug, Clone, Serialize)]
pub struct CompactionInput<'a> {
    pub model: &'a str,
    pub input: &'a [ResponseItem],
    #[serde(skip_serializing_if = "str::is_empty")]
    pub instructions: &'a str,
    pub tools: Vec<Value>,
    pub parallel_tool_calls: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<Reasoning>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_key: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<TextControls>,
}

/// Canonical input payload for the memory summarize endpoint.
#[derive(Debug, Clone, Serialize)]
pub struct MemorySummarizeInput {
    pub model: String,
    #[serde(rename = "traces")]
    pub raw_memories: Vec<RawMemory>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<Reasoning>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RawMemory {
    pub id: String,
    pub metadata: RawMemoryMetadata,
    pub items: Vec<Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RawMemoryMetadata {
    pub source_path: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct MemorySummarizeOutput {
    #[serde(rename = "trace_summary", alias = "raw_memory")]
    pub raw_memory: String,
    pub memory_summary: String,
}

#[derive(Debug)]
pub enum ResponseEvent {
    Created,
    OutputItemDone(ResponseItem),
    OutputItemAdded(ResponseItem),
    /// Emitted when the server includes `OpenAI-Model` on the stream response.
    /// This can differ from the requested model when backend safety routing applies.
    ServerModel(String),
    /// Emitted when the server recommends additional account verification.
    ModelVerifications(Vec<ModelVerification>),
    /// Emitted when `X-Reasoning-Included: true` is present on the response,
    /// meaning the server already accounted for past reasoning tokens and the
    /// client should not re-estimate them.
    ServerReasoningIncluded(bool),
    Completed {
        response_id: String,
        token_usage: Option<TokenUsage>,
        /// Did the model affirmatively end its turn? Some providers do not set this,
        /// so we rely on fallback logic when this is `None`.
        end_turn: Option<bool>,
    },
    OutputTextDelta(String),
    ToolCallInputDelta {
        item_id: String,
        call_id: Option<String>,
        delta: String,
    },
    ReasoningSummaryDelta {
        delta: String,
        summary_index: i64,
    },
    ReasoningContentDelta {
        delta: String,
        content_index: i64,
    },
    ReasoningSummaryPartAdded {
        summary_index: i64,
    },
    RateLimits(RateLimitSnapshot),
    ModelsEtag(String),
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct Reasoning {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<ReasoningEffortConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<ReasoningSummaryConfig>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct ChatCompletionsApiRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<Value>,
    pub tool_choice: String,
    pub parallel_tool_calls: bool,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<DeepSeekReasoningEffort>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<DeepSeekThinking>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<ChatStreamOptions>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct ChatStreamOptions {
    pub include_usage: bool,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct DeepSeekThinking {
    pub r#type: DeepSeekThinkingType,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DeepSeekThinkingType {
    Enabled,
}

#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DeepSeekReasoningEffort {
    High,
    Max,
}

impl From<ReasoningEffortConfig> for DeepSeekReasoningEffort {
    fn from(value: ReasoningEffortConfig) -> Self {
        match value {
            ReasoningEffortConfig::None
            | ReasoningEffortConfig::Minimal
            | ReasoningEffortConfig::Low
            | ReasoningEffortConfig::Medium
            | ReasoningEffortConfig::High => Self::High,
            ReasoningEffortConfig::XHigh => Self::Max,
        }
    }
}

impl ChatCompletionsApiRequest {
    pub fn new(
        model: String,
        instructions: String,
        input: Vec<ResponseItem>,
        tools: Vec<Value>,
        parallel_tool_calls: bool,
        reasoning: Option<Reasoning>,
    ) -> Result<Self, ApiError> {
        let messages = chat_messages_from_response_items(instructions, input)?;
        let reasoning_effort = reasoning
            .and_then(|reasoning| reasoning.effort)
            .map(DeepSeekReasoningEffort::from);
        Ok(Self {
            model,
            messages,
            tools,
            tool_choice: "auto".to_string(),
            parallel_tool_calls,
            stream: true,
            reasoning_effort,
            thinking: Some(DeepSeekThinking {
                r#type: DeepSeekThinkingType::Enabled,
            }),
            stream_options: Some(ChatStreamOptions {
                include_usage: true,
            }),
        })
    }
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct ChatMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ChatToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct ChatToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub function: ChatToolCallFunction,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct ChatToolCallFunction {
    pub name: String,
    pub arguments: String,
}

fn chat_messages_from_response_items(
    instructions: String,
    input: Vec<ResponseItem>,
) -> Result<Vec<ChatMessage>, ApiError> {
    let mut messages = Vec::new();
    if !instructions.trim().is_empty() {
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: Some(instructions),
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: None,
        });
    }

    let mut pending_reasoning: Option<String> = None;
    for item in input {
        match item {
            ResponseItem::Message { role, content, .. } => messages.push(ChatMessage {
                role,
                content: Some(content_items_to_chat_text(&content)?),
                reasoning_content: pending_reasoning.take(),
                tool_calls: None,
                tool_call_id: None,
            }),
            ResponseItem::Reasoning { content, .. } => {
                pending_reasoning = reasoning_content_to_chat_text(content);
            }
            ResponseItem::FunctionCall {
                name,
                arguments,
                call_id,
                ..
            } => messages.push(ChatMessage {
                role: "assistant".to_string(),
                content: Some(String::new()),
                reasoning_content: pending_reasoning.take(),
                tool_calls: Some(vec![ChatToolCall {
                    id: call_id,
                    r#type: "function".to_string(),
                    function: ChatToolCallFunction { name, arguments },
                }]),
                tool_call_id: None,
            }),
            ResponseItem::FunctionCallOutput { call_id, output } => messages.push(ChatMessage {
                role: "tool".to_string(),
                content: Some(function_call_output_to_chat_text(&output.body)),
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: Some(call_id),
            }),
            ResponseItem::CustomToolCall { .. }
            | ResponseItem::CustomToolCallOutput { .. }
            | ResponseItem::LocalShellCall { .. }
            | ResponseItem::ToolSearchCall { .. }
            | ResponseItem::ToolSearchOutput { .. }
            | ResponseItem::WebSearchCall { .. }
            | ResponseItem::ImageGenerationCall { .. }
            | ResponseItem::Compaction { .. }
            | ResponseItem::ContextCompaction { .. }
            | ResponseItem::Other => {}
        }
    }

    Ok(messages)
}

fn content_items_to_chat_text(content: &[ContentItem]) -> Result<String, ApiError> {
    let mut text = String::new();
    for item in content {
        match item {
            ContentItem::InputText { text: value } | ContentItem::OutputText { text: value } => {
                text.push_str(value);
            }
            ContentItem::InputImage { .. } => {
                return Err(ApiError::Stream(
                    "chat_completions providers do not support image input".to_string(),
                ));
            }
        }
    }
    Ok(text)
}

fn reasoning_content_to_chat_text(content: Option<Vec<ReasoningItemContent>>) -> Option<String> {
    let mut text = String::new();
    for item in content.unwrap_or_default() {
        match item {
            ReasoningItemContent::ReasoningText { text: value }
            | ReasoningItemContent::Text { text: value } => text.push_str(&value),
        }
    }
    (!text.is_empty()).then_some(text)
}

fn function_call_output_to_chat_text(output: &FunctionCallOutputBody) -> String {
    match output {
        FunctionCallOutputBody::Text(text) => text.clone(),
        FunctionCallOutputBody::ContentItems(items) => {
            function_call_output_content_items_to_text(items).unwrap_or_default()
        }
    }
}

#[derive(Debug, Serialize, Default, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TextFormatType {
    #[default]
    JsonSchema,
}

#[derive(Debug, Serialize, Default, Clone, PartialEq)]
pub struct TextFormat {
    /// Format type used by the OpenAI text controls.
    pub r#type: TextFormatType,
    /// When true, the server is expected to strictly validate responses.
    pub strict: bool,
    /// JSON schema for the desired output.
    pub schema: Value,
    /// Friendly name for the format, used in telemetry/debugging.
    pub name: String,
}

/// Controls the `text` field for the Responses API, combining verbosity and
/// optional JSON schema output formatting.
#[derive(Debug, Serialize, Default, Clone, PartialEq)]
pub struct TextControls {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbosity: Option<OpenAiVerbosity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<TextFormat>,
}

#[derive(Debug, Serialize, Default, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OpenAiVerbosity {
    Low,
    #[default]
    Medium,
    High,
}

impl From<VerbosityConfig> for OpenAiVerbosity {
    fn from(v: VerbosityConfig) -> Self {
        match v {
            VerbosityConfig::Low => OpenAiVerbosity::Low,
            VerbosityConfig::Medium => OpenAiVerbosity::Medium,
            VerbosityConfig::High => OpenAiVerbosity::High,
        }
    }
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct ResponsesApiRequest {
    pub model: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub instructions: String,
    pub input: Vec<ResponseItem>,
    pub tools: Vec<serde_json::Value>,
    pub tool_choice: String,
    pub parallel_tool_calls: bool,
    pub reasoning: Option<Reasoning>,
    pub store: bool,
    pub stream: bool,
    pub include: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<TextControls>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_metadata: Option<HashMap<String, String>>,
}

impl From<&ResponsesApiRequest> for ResponseCreateWsRequest {
    fn from(request: &ResponsesApiRequest) -> Self {
        Self {
            model: request.model.clone(),
            instructions: request.instructions.clone(),
            previous_response_id: None,
            input: request.input.clone(),
            tools: request.tools.clone(),
            tool_choice: request.tool_choice.clone(),
            parallel_tool_calls: request.parallel_tool_calls,
            reasoning: request.reasoning.clone(),
            store: request.store,
            stream: request.stream,
            include: request.include.clone(),
            service_tier: request.service_tier.clone(),
            prompt_cache_key: request.prompt_cache_key.clone(),
            text: request.text.clone(),
            generate: None,
            client_metadata: request.client_metadata.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ResponseCreateWsRequest {
    pub model: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub instructions: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_response_id: Option<String>,
    pub input: Vec<ResponseItem>,
    pub tools: Vec<Value>,
    pub tool_choice: String,
    pub parallel_tool_calls: bool,
    pub reasoning: Option<Reasoning>,
    pub store: bool,
    pub stream: bool,
    pub include: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<TextControls>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize)]
pub struct ResponseProcessedWsRequest {
    pub response_id: String,
}

pub fn response_create_client_metadata(
    client_metadata: Option<HashMap<String, String>>,
    trace: Option<&W3cTraceContext>,
) -> Option<HashMap<String, String>> {
    let mut client_metadata = client_metadata.unwrap_or_default();

    if let Some(traceparent) = trace.and_then(|trace| trace.traceparent.as_deref()) {
        client_metadata.insert(
            WS_REQUEST_HEADER_TRACEPARENT_CLIENT_METADATA_KEY.to_string(),
            traceparent.to_string(),
        );
    }
    if let Some(tracestate) = trace.and_then(|trace| trace.tracestate.as_deref()) {
        client_metadata.insert(
            WS_REQUEST_HEADER_TRACESTATE_CLIENT_METADATA_KEY.to_string(),
            tracestate.to_string(),
        );
    }

    (!client_metadata.is_empty()).then_some(client_metadata)
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
#[allow(clippy::large_enum_variant)]
pub enum ResponsesWsRequest {
    #[serde(rename = "response.create")]
    ResponseCreate(ResponseCreateWsRequest),
    #[serde(rename = "response.processed")]
    ResponseProcessed(ResponseProcessedWsRequest),
}

pub fn create_text_param_for_request(
    verbosity: Option<VerbosityConfig>,
    output_schema: &Option<Value>,
    output_schema_strict: bool,
) -> Option<TextControls> {
    if verbosity.is_none() && output_schema.is_none() {
        return None;
    }

    Some(TextControls {
        verbosity: verbosity.map(std::convert::Into::into),
        format: output_schema.as_ref().map(|schema| TextFormat {
            r#type: TextFormatType::JsonSchema,
            strict: output_schema_strict,
            schema: schema.clone(),
            name: "codex_output_schema".to_string(),
        }),
    })
}

pub struct ResponseStream {
    pub rx_event: mpsc::Receiver<Result<ResponseEvent, ApiError>>,
    /// Server-assigned `x-request-id` response header, when present.
    pub upstream_request_id: Option<String>,
}

impl Stream for ResponseStream {
    type Item = Result<ResponseEvent, ApiError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx_event.poll_recv(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_protocol::models::FunctionCallOutputPayload;
    use codex_protocol::models::ReasoningItemContent;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    #[test]
    fn chat_request_preserves_reasoning_content_before_tool_call() {
        let request = ChatCompletionsApiRequest::new(
            "deepseek-v4-pro".to_string(),
            "You are Codex.".to_string(),
            vec![
                ResponseItem::Message {
                    id: None,
                    role: "user".to_string(),
                    content: vec![ContentItem::InputText {
                        text: "call a tool".to_string(),
                    }],
                    phase: None,
                },
                ResponseItem::Reasoning {
                    id: "rs_1".to_string(),
                    summary: Vec::new(),
                    content: Some(vec![ReasoningItemContent::ReasoningText {
                        text: "I should inspect state.".to_string(),
                    }]),
                    encrypted_content: None,
                },
                ResponseItem::FunctionCall {
                    id: None,
                    name: "lookup".to_string(),
                    namespace: None,
                    arguments: "{\"id\":1}".to_string(),
                    call_id: "call_1".to_string(),
                },
                ResponseItem::FunctionCallOutput {
                    call_id: "call_1".to_string(),
                    output: FunctionCallOutputPayload::from_text("ok".to_string()),
                },
            ],
            Vec::new(),
            true,
            Some(Reasoning {
                effort: Some(ReasoningEffortConfig::XHigh),
                summary: None,
            }),
        )
        .expect("build chat request");

        assert_eq!(
            serde_json::to_value(&request).expect("serialize request"),
            json!({
                "model": "deepseek-v4-pro",
                "messages": [
                    {"role": "system", "content": "You are Codex."},
                    {"role": "user", "content": "call a tool"},
                    {
                        "role": "assistant",
                        "content": "",
                        "reasoning_content": "I should inspect state.",
                        "tool_calls": [{
                            "id": "call_1",
                            "type": "function",
                            "function": {
                                "name": "lookup",
                                "arguments": "{\"id\":1}"
                            }
                        }]
                    },
                    {"role": "tool", "content": "ok", "tool_call_id": "call_1"}
                ],
                "tool_choice": "auto",
                "parallel_tool_calls": true,
                "stream": true,
                "reasoning_effort": "max",
                "thinking": {"type": "enabled"},
                "stream_options": {"include_usage": true}
            })
        );
    }
}
