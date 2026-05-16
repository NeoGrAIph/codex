use codex_models_manager::model_info::BASE_INSTRUCTIONS;
use codex_protocol::config_types::ReasoningSummary;
use codex_protocol::openai_models::ApplyPatchToolType;
use codex_protocol::openai_models::ConfigShellToolType;
use codex_protocol::openai_models::InputModality;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::openai_models::ModelVisibility;
use codex_protocol::openai_models::ModelsResponse;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::openai_models::ReasoningEffortPreset;
use codex_protocol::openai_models::TruncationPolicyConfig;
use codex_protocol::openai_models::WebSearchToolType;

const DEEPSEEK_CONTEXT_WINDOW: i64 = 128_000;

pub(crate) fn static_model_catalog() -> ModelsResponse {
    ModelsResponse {
        models: vec![
            deepseek_model(
                "deepseek-v4-pro",
                "DeepSeek V4 Pro",
                "DeepSeek V4 Pro with thinking mode and tool calls.",
                /*priority*/ 0,
            ),
            deepseek_model(
                "deepseek-v4-flash",
                "DeepSeek V4 Flash",
                "DeepSeek V4 Flash with thinking mode and tool calls.",
                /*priority*/ 1,
            ),
        ],
    }
}

fn deepseek_model(slug: &str, display_name: &str, description: &str, priority: i32) -> ModelInfo {
    ModelInfo {
        slug: slug.to_string(),
        display_name: display_name.to_string(),
        description: Some(description.to_string()),
        default_reasoning_level: Some(ReasoningEffort::High),
        supported_reasoning_levels: vec![
            reasoning_effort_preset(ReasoningEffort::Low),
            reasoning_effort_preset(ReasoningEffort::Medium),
            reasoning_effort_preset(ReasoningEffort::High),
            reasoning_effort_preset(ReasoningEffort::XHigh),
        ],
        shell_type: ConfigShellToolType::ShellCommand,
        visibility: ModelVisibility::List,
        supported_in_api: true,
        priority,
        additional_speed_tiers: Vec::new(),
        service_tiers: Vec::new(),
        availability_nux: None,
        upgrade: None,
        base_instructions: BASE_INSTRUCTIONS.to_string(),
        model_messages: None,
        supports_reasoning_summaries: true,
        default_reasoning_summary: ReasoningSummary::None,
        support_verbosity: false,
        default_verbosity: None,
        apply_patch_tool_type: Some(ApplyPatchToolType::Function),
        web_search_tool_type: WebSearchToolType::Text,
        truncation_policy: TruncationPolicyConfig::tokens(/*limit*/ 10_000),
        supports_parallel_tool_calls: true,
        supports_image_detail_original: false,
        context_window: Some(DEEPSEEK_CONTEXT_WINDOW),
        max_context_window: Some(DEEPSEEK_CONTEXT_WINDOW),
        auto_compact_token_limit: None,
        effective_context_window_percent: 95,
        experimental_supported_tools: Vec::new(),
        input_modalities: vec![InputModality::Text],
        used_fallback_model_metadata: false,
        supports_search_tool: false,
    }
}

fn reasoning_effort_preset(effort: ReasoningEffort) -> ReasoningEffortPreset {
    ReasoningEffortPreset {
        effort,
        description: match effort {
            ReasoningEffort::None => "No reasoning",
            ReasoningEffort::Minimal => "Minimal reasoning",
            ReasoningEffort::Low => "Mapped to DeepSeek high reasoning",
            ReasoningEffort::Medium => "Mapped to DeepSeek high reasoning",
            ReasoningEffort::High => "DeepSeek high reasoning",
            ReasoningEffort::XHigh => "DeepSeek max reasoning",
        }
        .to_string(),
    }
}
