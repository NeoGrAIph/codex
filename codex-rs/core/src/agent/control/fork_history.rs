use super::MultiAgentRuntimeIntent;
use codex_protocol::models::ContentItem;
use codex_protocol::models::MessagePhase;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::MULTI_AGENT_MODE_CLOSE_TAG;
use codex_protocol::protocol::MULTI_AGENT_MODE_OPEN_TAG;
use codex_protocol::protocol::MultiAgentVersion;
use codex_protocol::protocol::RolloutItem;

pub(super) fn sanitize_forked_rollout_items(
    items: &mut Vec<RolloutItem>,
    preserve_reference_context_item: bool,
    multi_agent_v2_usage_hint_texts: &[String],
    multi_agent_runtime: MultiAgentRuntimeIntent,
) {
    items.retain_mut(|item| {
        if !keep_forked_rollout_item(item, preserve_reference_context_item) {
            return false;
        }

        match item {
            RolloutItem::ResponseItem(response_item) => sanitize_response_item(
                response_item,
                multi_agent_v2_usage_hint_texts,
                multi_agent_runtime,
            ),
            RolloutItem::Compacted(compacted) => {
                if let Some(replacement_history) = compacted.replacement_history.as_mut() {
                    replacement_history.retain_mut(|response_item| {
                        sanitize_response_item(
                            response_item,
                            multi_agent_v2_usage_hint_texts,
                            multi_agent_runtime,
                        )
                    });
                }
                true
            }
            RolloutItem::TurnContext(turn_context)
                if multi_agent_runtime == MultiAgentRuntimeIntent::ExactV1Spawn =>
            {
                turn_context.multi_agent_version = Some(MultiAgentVersion::V1);
                turn_context.multi_agent_mode = None;
                true
            }
            RolloutItem::SessionMeta(_)
            | RolloutItem::InterAgentCommunication(_)
            | RolloutItem::InterAgentCommunicationMetadata { .. }
            | RolloutItem::WorldState(_)
            | RolloutItem::EventMsg(_)
            | RolloutItem::TurnContext(_) => true,
        }
    });
}

fn keep_forked_rollout_item(item: &RolloutItem, preserve_reference_context_item: bool) -> bool {
    match item {
        RolloutItem::ResponseItem(ResponseItem::Message { role, phase, .. }) => match role.as_str()
        {
            "system" | "developer" | "user" => true,
            "assistant" => *phase == Some(MessagePhase::FinalAnswer),
            _ => false,
        },
        RolloutItem::ResponseItem(
            ResponseItem::AdditionalTools { .. }
            | ResponseItem::AgentMessage { .. }
            | ResponseItem::Reasoning { .. }
            | ResponseItem::LocalShellCall { .. }
            | ResponseItem::FunctionCall { .. }
            | ResponseItem::ToolSearchCall { .. }
            | ResponseItem::FunctionCallOutput { .. }
            | ResponseItem::CustomToolCall { .. }
            | ResponseItem::CustomToolCallOutput { .. }
            | ResponseItem::ToolSearchOutput { .. }
            | ResponseItem::WebSearchCall { .. }
            | ResponseItem::ImageGenerationCall { .. }
            | ResponseItem::Compaction { .. }
            | ResponseItem::CompactionTrigger { .. }
            | ResponseItem::ContextCompaction { .. }
            | ResponseItem::Other,
        ) => false,
        RolloutItem::InterAgentCommunication(_)
        | RolloutItem::InterAgentCommunicationMetadata { .. } => false,
        RolloutItem::TurnContext(_) | RolloutItem::WorldState(_) => preserve_reference_context_item,
        RolloutItem::Compacted(_) | RolloutItem::EventMsg(_) | RolloutItem::SessionMeta(_) => true,
    }
}

fn sanitize_response_item(
    item: &mut ResponseItem,
    multi_agent_v2_usage_hint_texts: &[String],
    multi_agent_runtime: MultiAgentRuntimeIntent,
) -> bool {
    if is_multi_agent_v2_usage_hint_message(item, multi_agent_v2_usage_hint_texts) {
        return false;
    }
    if multi_agent_runtime != MultiAgentRuntimeIntent::ExactV1Spawn {
        return true;
    }

    let ResponseItem::Message { role, content, .. } = item else {
        return true;
    };
    if role != "developer" {
        return true;
    }

    content.retain_mut(sanitize_multi_agent_mode_fragment);
    !content.is_empty()
}

fn is_multi_agent_v2_usage_hint_message(item: &ResponseItem, usage_hint_texts: &[String]) -> bool {
    let ResponseItem::Message { role, content, .. } = item else {
        return false;
    };
    if role != "developer" {
        return false;
    }
    let [ContentItem::InputText { text }] = content.as_slice() else {
        return false;
    };

    usage_hint_texts
        .iter()
        .any(|usage_hint_text| usage_hint_text == text)
}

fn sanitize_multi_agent_mode_fragment(content_item: &mut ContentItem) -> bool {
    let ContentItem::InputText { text } = content_item else {
        return true;
    };
    let trimmed = text.trim_start();
    let Some(prefix) = trimmed.get(..MULTI_AGENT_MODE_OPEN_TAG.len()) else {
        return true;
    };
    if !prefix.eq_ignore_ascii_case(MULTI_AGENT_MODE_OPEN_TAG) {
        return true;
    }
    let after_open = &trimmed[MULTI_AGENT_MODE_OPEN_TAG.len()..];
    let Some(close_offset) = after_open
        .to_ascii_lowercase()
        .find(&MULTI_AGENT_MODE_CLOSE_TAG.to_ascii_lowercase())
    else {
        return true;
    };
    let suffix = &after_open[close_offset + MULTI_AGENT_MODE_CLOSE_TAG.len()..];
    let suffix = suffix
        .strip_prefix("\r\n")
        .or_else(|| suffix.strip_prefix('\n'))
        .unwrap_or(suffix);
    if suffix.trim().is_empty() {
        return false;
    }
    *text = suffix.to_string();
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_protocol::ThreadId;
    use codex_protocol::config_types::MultiAgentMode;
    use codex_protocol::config_types::ReasoningSummary;
    use codex_protocol::protocol::AskForApproval;
    use codex_protocol::protocol::CompactedItem;
    use codex_protocol::protocol::InitialHistory;
    use codex_protocol::protocol::ResumedHistory;
    use codex_protocol::protocol::SandboxPolicy;
    use codex_protocol::protocol::SessionMeta;
    use codex_protocol::protocol::SessionMetaLine;
    use codex_protocol::protocol::TurnContextItem;
    use codex_protocol::protocol::WorldStateItem;
    use codex_utils_absolute_path::AbsolutePathBuf;
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use std::sync::Arc;

    const ROOT_USAGE_HINT: &str = "parent V2 root usage hint";
    const MULTI_AGENT_MODE: &str = "<multi_agent_mode>parent V2 mode</multi_agent_mode>";

    fn message(role: &str, content: Vec<ContentItem>) -> ResponseItem {
        ResponseItem::Message {
            id: None,
            role: role.to_string(),
            content,
            phase: None,
            internal_chat_message_metadata_passthrough: None,
        }
    }

    fn assistant_message(text: &str, phase: MessagePhase) -> ResponseItem {
        ResponseItem::Message {
            id: None,
            role: "assistant".to_string(),
            content: vec![ContentItem::OutputText {
                text: text.to_string(),
            }],
            phase: Some(phase),
            internal_chat_message_metadata_passthrough: None,
        }
    }

    fn turn_context_item() -> TurnContextItem {
        TurnContextItem {
            turn_id: Some("parent-turn".to_string()),
            cwd: AbsolutePathBuf::try_from(
                std::env::current_dir().expect("current directory should be available"),
            )
            .expect("current directory should be absolute"),
            workspace_roots: None,
            current_date: None,
            timezone: None,
            approval_policy: AskForApproval::OnRequest,
            approvals_reviewer: None,
            sandbox_policy: SandboxPolicy::new_read_only_policy(),
            permission_profile: None,
            network: None,
            file_system_sandbox_policy: None,
            model: "gpt-5.6-sol".to_string(),
            comp_hash: None,
            personality: None,
            collaboration_mode: None,
            multi_agent_version: Some(MultiAgentVersion::V2),
            multi_agent_mode: Some(MultiAgentMode::Custom("parent V2 mode".to_string())),
            realtime_active: None,
            effort: None,
            summary: ReasoningSummary::Auto,
        }
    }

    fn session_meta(
        thread_id: ThreadId,
        multi_agent_version: Option<MultiAgentVersion>,
    ) -> RolloutItem {
        RolloutItem::SessionMeta(SessionMetaLine {
            meta: SessionMeta {
                id: thread_id,
                session_id: thread_id.into(),
                multi_agent_version,
                ..Default::default()
            },
            git: None,
        })
    }

    #[test]
    fn native_v1_fork_history_sanitizes_only_v2_runtime_context() {
        let parent_thread_id = ThreadId::new();
        let parent_meta = session_meta(parent_thread_id, Some(MultiAgentVersion::V2));
        let parent_turn_context = turn_context_item();
        let world_state = RolloutItem::WorldState(WorldStateItem::full(json!({
            "environments": {"subagents": "v2-worker"}
        })));
        let compacted = RolloutItem::Compacted(CompactedItem {
            message: "semantic compacted summary".to_string(),
            replacement_history: Some(vec![
                message(
                    "user",
                    vec![ContentItem::InputText {
                        text: "semantic compacted user context".to_string(),
                    }],
                ),
                message(
                    "developer",
                    vec![ContentItem::InputText {
                        text: ROOT_USAGE_HINT.to_string(),
                    }],
                ),
                message(
                    "developer",
                    vec![
                        ContentItem::InputText {
                            text: MULTI_AGENT_MODE.to_string(),
                        },
                        ContentItem::InputText {
                            text: "keep compacted developer context".to_string(),
                        },
                    ],
                ),
            ]),
            window_number: Some(2),
            first_window_id: Some("first-window".to_string()),
            previous_window_id: Some("previous-window".to_string()),
            window_id: Some("current-window".to_string()),
        });
        let mut inherited_baseline = vec![
            RolloutItem::ResponseItem(message(
                "developer",
                vec![ContentItem::InputText {
                    text: MULTI_AGENT_MODE.to_string(),
                }],
            )),
            RolloutItem::TurnContext(parent_turn_context.clone()),
            world_state.clone(),
        ];
        let expected_inherited_baseline = inherited_baseline.clone();
        sanitize_forked_rollout_items(
            &mut inherited_baseline,
            /*preserve_reference_context_item*/ true,
            &[],
            MultiAgentRuntimeIntent::Inherit,
        );
        assert_eq!(
            serde_json::to_value(inherited_baseline).expect("inherited baseline should serialize"),
            serde_json::to_value(expected_inherited_baseline)
                .expect("expected inherited baseline should serialize")
        );

        let mut items = vec![
            parent_meta.clone(),
            RolloutItem::ResponseItem(message(
                "developer",
                vec![
                    ContentItem::InputText {
                        text: MULTI_AGENT_MODE.to_string(),
                    },
                    ContentItem::InputText {
                        text: "keep top-level developer context".to_string(),
                    },
                ],
            )),
            compacted,
            RolloutItem::TurnContext(parent_turn_context.clone()),
            world_state.clone(),
            RolloutItem::ResponseItem(assistant_message(
                "keep final answer",
                MessagePhase::FinalAnswer,
            )),
            RolloutItem::ResponseItem(assistant_message(
                "drop commentary",
                MessagePhase::Commentary,
            )),
        ];

        sanitize_forked_rollout_items(
            &mut items,
            /*preserve_reference_context_item*/ true,
            &[ROOT_USAGE_HINT.to_string()],
            MultiAgentRuntimeIntent::ExactV1Spawn,
        );

        let mut native_v1_turn_context = parent_turn_context;
        native_v1_turn_context.multi_agent_version = Some(MultiAgentVersion::V1);
        native_v1_turn_context.multi_agent_mode = None;
        let expected = vec![
            parent_meta,
            RolloutItem::ResponseItem(message(
                "developer",
                vec![ContentItem::InputText {
                    text: "keep top-level developer context".to_string(),
                }],
            )),
            RolloutItem::Compacted(CompactedItem {
                message: "semantic compacted summary".to_string(),
                replacement_history: Some(vec![
                    message(
                        "user",
                        vec![ContentItem::InputText {
                            text: "semantic compacted user context".to_string(),
                        }],
                    ),
                    message(
                        "developer",
                        vec![ContentItem::InputText {
                            text: "keep compacted developer context".to_string(),
                        }],
                    ),
                ]),
                window_number: Some(2),
                first_window_id: Some("first-window".to_string()),
                previous_window_id: Some("previous-window".to_string()),
                window_id: Some("current-window".to_string()),
            }),
            RolloutItem::TurnContext(native_v1_turn_context),
            world_state,
            RolloutItem::ResponseItem(assistant_message(
                "keep final answer",
                MessagePhase::FinalAnswer,
            )),
        ];

        assert_eq!(
            serde_json::to_value(items).expect("sanitized items should serialize"),
            serde_json::to_value(expected).expect("expected items should serialize")
        );
    }

    #[test]
    fn native_v1_fork_history_preserves_semantic_suffix_after_structural_mode_fragment() {
        let combined = format!("{MULTI_AGENT_MODE}\nkeep semantic developer context");
        let mut items = vec![
            RolloutItem::ResponseItem(message(
                "developer",
                vec![ContentItem::InputText {
                    text: combined.clone(),
                }],
            )),
            RolloutItem::Compacted(CompactedItem {
                message: "semantic compacted summary".to_string(),
                replacement_history: Some(vec![message(
                    "developer",
                    vec![ContentItem::InputText { text: combined }],
                )]),
                window_number: None,
                first_window_id: None,
                previous_window_id: None,
                window_id: None,
            }),
        ];

        sanitize_forked_rollout_items(
            &mut items,
            /*preserve_reference_context_item*/ true,
            &[],
            MultiAgentRuntimeIntent::ExactV1Spawn,
        );

        let semantic_message = message(
            "developer",
            vec![ContentItem::InputText {
                text: "keep semantic developer context".to_string(),
            }],
        );
        let expected = vec![
            RolloutItem::ResponseItem(semantic_message.clone()),
            RolloutItem::Compacted(CompactedItem {
                message: "semantic compacted summary".to_string(),
                replacement_history: Some(vec![semantic_message]),
                window_number: None,
                first_window_id: None,
                previous_window_id: None,
                window_id: None,
            }),
        ];
        assert_eq!(
            serde_json::to_value(items).expect("sanitized items should serialize"),
            serde_json::to_value(expected).expect("expected items should serialize")
        );
    }

    #[test]
    fn normalized_fork_history_keeps_v1_fallback_when_child_version_is_missing() {
        let parent_thread_id = ThreadId::new();
        let child_thread_id = ThreadId::new();
        let mut copied_parent_items = vec![
            session_meta(parent_thread_id, Some(MultiAgentVersion::V2)),
            RolloutItem::TurnContext(turn_context_item()),
        ];
        sanitize_forked_rollout_items(
            &mut copied_parent_items,
            /*preserve_reference_context_item*/ true,
            &[],
            MultiAgentRuntimeIntent::ExactV1Spawn,
        );

        let mut child_history = vec![session_meta(
            child_thread_id,
            /*multi_agent_version*/ None,
        )];
        child_history.extend(copied_parent_items);
        let initial_history = InitialHistory::Resumed(ResumedHistory {
            conversation_id: child_thread_id,
            history: Arc::new(child_history),
            rollout_path: None,
        });

        assert_eq!(
            initial_history.get_multi_agent_version(),
            Some(MultiAgentVersion::V1)
        );
    }
}
