//! Ephemeral read model for the `/agent` workbench rows.
//!
//! The model is a projection over [`AgentNavigationState`] and already-bounded per-thread
//! summaries. It deliberately does not own lifecycle actions, app-server requests, or persistence.

use super::agent_navigation::AgentNavigationState;
use crate::multi_agents::AgentPickerSelectedDescriptionContext;
use crate::multi_agents::agent_picker_item_description;
use crate::multi_agents::agent_picker_selected_description;
use crate::multi_agents::agent_picker_status_dot_spans;
use crate::multi_agents::agent_picker_summary_line;
use crate::multi_agents::format_agent_picker_item_name;
use codex_protocol::ThreadId;
use ratatui::text::Span;
use std::collections::HashMap;

#[derive(Debug)]
pub(super) struct AgentWorkbenchDetailSummaries<'a> {
    pub(super) prompt_context_by_thread_id: &'a HashMap<ThreadId, Vec<String>>,
    pub(super) recent_activity_by_thread_id: &'a HashMap<ThreadId, Vec<String>>,
    pub(super) token_usage_by_thread_id: &'a HashMap<ThreadId, String>,
    pub(super) plan_progress_by_thread_id: &'a HashMap<ThreadId, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AgentWorkbenchReadModel {
    pub(super) summary_line: String,
    pub(super) rows: Vec<AgentWorkbenchRow>,
    pub(super) initial_selected_idx: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AgentWorkbenchRow {
    pub(super) thread_id: ThreadId,
    pub(super) name: String,
    pub(super) name_prefix_spans: Vec<Span<'static>>,
    pub(super) description: String,
    pub(super) selected_description: String,
    pub(super) is_current: bool,
    pub(super) search_value: String,
    pub(super) prompt_context: Vec<String>,
    pub(super) recent_activity: Vec<String>,
    pub(super) token_usage_summary: Option<String>,
    pub(super) plan_progress_summary: Option<String>,
}

impl AgentWorkbenchReadModel {
    pub(super) fn build(
        agent_navigation: &AgentNavigationState,
        primary_thread_id: Option<ThreadId>,
        active_thread_id: Option<ThreadId>,
        detail_summaries: AgentWorkbenchDetailSummaries<'_>,
    ) -> Self {
        let ordered_threads = agent_navigation.ordered_workbench_threads();
        let summary_line = agent_picker_summary_line(&ordered_threads, primary_thread_id);
        let mut initial_selected_idx = None;
        let empty_strings: &[String] = &[];
        let rows = ordered_threads
            .into_iter()
            .enumerate()
            .map(|(idx, (thread_id, entry))| {
                if active_thread_id == Some(thread_id) {
                    initial_selected_idx = Some(idx);
                }
                let is_primary = primary_thread_id == Some(thread_id);
                let is_current = active_thread_id == Some(thread_id);
                let name = format_agent_picker_item_name(
                    entry.agent_nickname.as_deref(),
                    entry.agent_role.as_deref(),
                    is_primary,
                );
                let uuid = thread_id.to_string();
                let prompt_context = detail_summaries
                    .prompt_context_by_thread_id
                    .get(&thread_id)
                    .map(Vec::as_slice)
                    .unwrap_or(empty_strings);
                let recent_activity = detail_summaries
                    .recent_activity_by_thread_id
                    .get(&thread_id)
                    .map(Vec::as_slice)
                    .unwrap_or(empty_strings);
                let token_usage_summary = detail_summaries
                    .token_usage_by_thread_id
                    .get(&thread_id)
                    .map(String::as_str);
                let plan_progress_summary = detail_summaries
                    .plan_progress_by_thread_id
                    .get(&thread_id)
                    .map(String::as_str);
                let prompt_context_vec = prompt_context.to_vec();
                let recent_activity_vec = recent_activity.to_vec();
                AgentWorkbenchRow {
                    thread_id,
                    name: name.clone(),
                    name_prefix_spans: agent_picker_status_dot_spans(entry.status),
                    description: agent_picker_item_description(thread_id, entry, is_primary),
                    selected_description: agent_picker_selected_description(
                        thread_id,
                        entry,
                        is_primary,
                        is_current,
                        AgentPickerSelectedDescriptionContext {
                            prompt_context,
                            recent_activity,
                            token_usage_summary,
                            plan_progress_summary,
                        },
                    ),
                    is_current,
                    search_value: format!("{name} {uuid}"),
                    prompt_context: prompt_context_vec,
                    recent_activity: recent_activity_vec,
                    token_usage_summary: token_usage_summary.map(str::to_string),
                    plan_progress_summary: plan_progress_summary.map(str::to_string),
                }
            })
            .collect();

        Self {
            summary_line,
            rows,
            initial_selected_idx,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multi_agents::AgentPickerThreadStatus;
    use pretty_assertions::assert_eq;

    fn thread_id(suffix: u32) -> ThreadId {
        ThreadId::from_string(&format!("00000000-0000-0000-0000-{suffix:012}"))
            .expect("valid thread id")
    }

    #[test]
    fn read_model_builds_summary_current_row_and_bounded_detail_projection() {
        let primary = thread_id(101);
        let worker = thread_id(102);
        let reviewer = thread_id(103);
        let mut navigation = AgentNavigationState::default();
        navigation.upsert(primary, None, None, /*is_closed*/ false);
        navigation.upsert(
            worker,
            None,
            Some("worker".to_string()),
            /*is_closed*/ false,
        );
        navigation.upsert(
            reviewer,
            None,
            Some("reviewer".to_string()),
            /*is_closed*/ true,
        );
        navigation.set_agent_path(worker, Some("/root/worker".to_string()));
        navigation.set_agent_path(reviewer, Some("/root/reviewer".to_string()));
        navigation.set_status(worker, AgentPickerThreadStatus::Running);

        let mut prompt_context_by_thread_id = HashMap::new();
        prompt_context_by_thread_id.insert(
            worker,
            vec!["Initial request: Inspect the parser.".to_string()],
        );
        let mut recent_activity_by_thread_id = HashMap::new();
        recent_activity_by_thread_id
            .insert(worker, vec!["Checked parser entry points.".to_string()]);
        let mut token_usage_by_thread_id = HashMap::new();
        token_usage_by_thread_id.insert(worker, "total 10".to_string());
        let mut plan_progress_by_thread_id = HashMap::new();
        plan_progress_by_thread_id.insert(worker, "1/2 complete".to_string());

        let model = AgentWorkbenchReadModel::build(
            &navigation,
            Some(primary),
            Some(worker),
            AgentWorkbenchDetailSummaries {
                prompt_context_by_thread_id: &prompt_context_by_thread_id,
                recent_activity_by_thread_id: &recent_activity_by_thread_id,
                token_usage_by_thread_id: &token_usage_by_thread_id,
                plan_progress_by_thread_id: &plan_progress_by_thread_id,
            },
        );

        assert_eq!(
            model.summary_line,
            "Agents: 2 total · 1 running · 0 waiting · 0 error · 1 closed"
        );
        assert_eq!(model.initial_selected_idx, Some(1));
        assert_eq!(model.rows.len(), 3);
        assert_eq!(model.rows[1].thread_id, worker);
        assert_eq!(
            model.rows[1].description,
            "running · role worker · /root/worker"
        );
        assert_eq!(model.rows[1].is_current, true);
        assert_eq!(model.rows[1].search_value, format!("[worker] {worker}"));
        assert!(
            model.rows[1]
                .selected_description
                .contains("Connection: connected to this thread")
        );
        assert!(
            model.rows[1]
                .selected_description
                .contains("- Enter: stay on this thread")
        );
        assert!(
            model.rows[1]
                .selected_description
                .contains("Context:\n- Initial request: Inspect the parser.")
        );
        assert!(
            model.rows[1]
                .selected_description
                .contains("Recent activity:\n- Checked parser entry points.")
        );
        assert!(
            model.rows[1]
                .selected_description
                .contains("- Tokens: total 10")
        );
        assert!(
            model.rows[1]
                .selected_description
                .contains("- Plan: 1/2 complete")
        );
    }

    #[test]
    fn read_model_excludes_hidden_agents_from_workbench_rows() {
        let primary = thread_id(101);
        let visible_worker = thread_id(102);
        let hidden_reviewer = thread_id(103);
        let mut navigation = AgentNavigationState::default();
        navigation.upsert(primary, None, None, /*is_closed*/ false);
        navigation.upsert(
            visible_worker,
            None,
            Some("worker".to_string()),
            /*is_closed*/ false,
        );
        navigation.upsert(
            hidden_reviewer,
            None,
            Some("reviewer".to_string()),
            /*is_closed*/ false,
        );
        navigation.update_thread_detail(
            hidden_reviewer,
            crate::app::agent_navigation::AgentPickerThreadDetail {
                agent_path: Some("/root/reviewer".to_string()),
                prompt_preview: None,
                thread_note: None,
                agent_hidden: true,
                retry_available: None,
                cwd: None,
                model_provider: None,
                created_at: None,
                updated_at: None,
                tool_selection_summary: None,
                action_policy_summary: None,
            },
        );

        let model = AgentWorkbenchReadModel::build(
            &navigation,
            Some(primary),
            Some(visible_worker),
            AgentWorkbenchDetailSummaries {
                prompt_context_by_thread_id: &HashMap::new(),
                recent_activity_by_thread_id: &HashMap::new(),
                token_usage_by_thread_id: &HashMap::new(),
                plan_progress_by_thread_id: &HashMap::new(),
            },
        );

        assert_eq!(
            model.summary_line,
            "Agents: 1 total · 0 running · 0 waiting · 0 error · 0 closed"
        );
        assert_eq!(
            model
                .rows
                .iter()
                .map(|row| row.thread_id)
                .collect::<Vec<_>>(),
            vec![primary, visible_worker]
        );
    }
}
