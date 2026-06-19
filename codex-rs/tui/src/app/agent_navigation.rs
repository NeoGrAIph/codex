//! Multi-agent picker navigation and labeling state for the TUI app.
//!
//! This module exists to keep the pure parts of multi-agent navigation out of [`crate::app::App`].
//! It owns the stable spawn-order cache used by the `/agent` picker, keyboard next/previous
//! navigation, and the contextual footer label for the thread currently being watched.
//!
//! Responsibilities here are intentionally narrow:
//! - remember picker entries and their first-seen order
//! - answer traversal questions like "what is the next thread?"
//! - derive user-facing picker/footer text from cached thread metadata
//!
//! Responsibilities that stay in `App`:
//! - discovering threads from the backend
//! - deciding which thread is currently displayed
//! - mutating UI state such as switching threads or updating the footer widget
//!
//! The key invariant is that traversal follows first-seen spawn order rather than thread-id sort
//! order. Once a thread id is observed it keeps its place in the cycle even if the entry is later
//! updated or marked closed.

use crate::multi_agents::AgentPickerThreadEntry;
use crate::multi_agents::AgentPickerThreadStatus;
use crate::multi_agents::AgentRoleRuntimeUsage;
use crate::multi_agents::SubAgentActivityDisplay;
use crate::multi_agents::format_agent_picker_item_name;
use crate::multi_agents::next_agent_shortcut;
use crate::multi_agents::previous_agent_shortcut;
use codex_protocol::ThreadId;
use ratatui::text::Span;
use std::collections::BTreeMap;
use std::collections::HashMap;

/// Small state container for multi-agent picker ordering and labeling.
///
/// `App` owns thread lifecycle and UI side effects. This type keeps the pure rules for stable
/// spawn-order traversal, picker copy, and active-agent labels together and separately testable.
///
/// The core invariant is that `order` records first-seen thread ids exactly once, while `threads`
/// stores the latest metadata for those ids. Mutation is intentionally funneled through `upsert`,
/// `mark_closed`, and `clear` so those two collections do not drift semantically even if they are
/// temporarily out of sync during teardown races.
#[derive(Debug, Default)]
pub(crate) struct AgentNavigationState {
    /// Latest picker metadata for each tracked thread id.
    threads: HashMap<ThreadId, AgentPickerThreadEntry>,
    /// Stable first-seen traversal order for picker rows and keyboard cycling.
    order: Vec<ThreadId>,
}

/// Direction of keyboard traversal through the stable picker order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AgentNavigationDirection {
    /// Move toward the entry that was seen earlier in spawn order, wrapping at the front.
    Previous,
    /// Move toward the entry that was seen later in spawn order, wrapping at the end.
    Next,
}

pub(crate) struct AgentPickerThreadDetail {
    pub(crate) agent_path: Option<String>,
    pub(crate) prompt_preview: Option<String>,
    pub(crate) thread_note: Option<String>,
    pub(crate) agent_hidden: bool,
    pub(crate) retry_available: Option<bool>,
    pub(crate) cwd: Option<String>,
    pub(crate) model_provider: Option<String>,
    pub(crate) created_at: Option<i64>,
    pub(crate) updated_at: Option<i64>,
    pub(crate) tool_selection_summary: Option<String>,
    pub(crate) action_policy_summary: Option<String>,
}

impl AgentNavigationState {
    /// Returns the cached picker entry for a specific thread id.
    ///
    /// Callers use this when they already know which thread they care about and need the last
    /// metadata captured for picker or footer rendering. If a caller assumes every tracked thread
    /// must be present here, shutdown races can turn that assumption into a panic elsewhere, so
    /// this stays optional.
    pub(crate) fn get(&self, thread_id: &ThreadId) -> Option<&AgentPickerThreadEntry> {
        self.threads.get(thread_id)
    }

    /// Returns whether the picker cache currently knows about any threads.
    ///
    /// This is the cheapest way for `App` to decide whether opening the picker should show "No
    /// agents available yet." rather than constructing picker rows from an empty state.
    pub(crate) fn is_empty(&self) -> bool {
        self.threads.is_empty()
    }

    /// Inserts or updates a picker entry while preserving first-seen traversal order.
    ///
    /// The key invariant of this module is enforced here: a thread id is appended to `order` only
    /// the first time it is seen. Later updates may change nickname, role, or closed state, but
    /// they must not move the thread in the cycle or keyboard navigation would feel unstable.
    pub(crate) fn upsert(
        &mut self,
        thread_id: ThreadId,
        agent_nickname: Option<String>,
        agent_role: Option<String>,
        is_closed: bool,
    ) {
        if !self.threads.contains_key(&thread_id) {
            self.order.push(thread_id);
        }
        let previous_entry = self.threads.get(&thread_id);
        let previous_agent_path = previous_entry.and_then(|entry| entry.agent_path.clone());
        let previous_prompt_preview = previous_entry.and_then(|entry| entry.prompt_preview.clone());
        let previous_thread_note = previous_entry.and_then(|entry| entry.thread_note.clone());
        let previous_agent_hidden = previous_entry
            .map(|entry| entry.agent_hidden)
            .unwrap_or(false);
        let previous_retry_available = previous_entry
            .map(|entry| entry.retry_available)
            .unwrap_or(false);
        let previous_cwd = previous_entry.and_then(|entry| entry.cwd.clone());
        let previous_model_provider = previous_entry.and_then(|entry| entry.model_provider.clone());
        let previous_created_at = previous_entry.and_then(|entry| entry.created_at);
        let previous_updated_at = previous_entry.and_then(|entry| entry.updated_at);
        let previous_model = previous_entry.and_then(|entry| entry.model.clone());
        let previous_reasoning_effort =
            previous_entry.and_then(|entry| entry.reasoning_effort.clone());
        let previous_service_tier = previous_entry.and_then(|entry| entry.service_tier.clone());
        let previous_tool_selection_summary =
            previous_entry.and_then(|entry| entry.tool_selection_summary.clone());
        let previous_action_policy_summary =
            previous_entry.and_then(|entry| entry.action_policy_summary.clone());
        let previous_status = previous_entry
            .map(|entry| entry.status)
            .unwrap_or(AgentPickerThreadStatus::Idle);
        let status = if is_closed {
            AgentPickerThreadStatus::Closed
        } else if previous_status.is_closed() {
            AgentPickerThreadStatus::Idle
        } else {
            previous_status
        };
        self.threads.insert(
            thread_id,
            AgentPickerThreadEntry {
                agent_nickname,
                agent_role,
                agent_path: previous_agent_path,
                prompt_preview: previous_prompt_preview,
                thread_note: previous_thread_note,
                agent_hidden: previous_agent_hidden,
                retry_available: previous_retry_available,
                cwd: previous_cwd,
                model_provider: previous_model_provider,
                created_at: previous_created_at,
                updated_at: previous_updated_at,
                model: previous_model,
                reasoning_effort: previous_reasoning_effort,
                service_tier: previous_service_tier,
                tool_selection_summary: previous_tool_selection_summary,
                action_policy_summary: previous_action_policy_summary,
                status,
            },
        );
    }

    pub(crate) fn record_sub_agent_activity(&mut self, activity: SubAgentActivityDisplay) {
        if !self.threads.contains_key(&activity.thread_id) {
            self.order.push(activity.thread_id);
        }
        let entry =
            self.threads
                .entry(activity.thread_id)
                .or_insert_with(|| AgentPickerThreadEntry {
                    agent_nickname: None,
                    agent_role: None,
                    agent_path: None,
                    prompt_preview: None,
                    thread_note: None,
                    agent_hidden: false,
                    retry_available: false,
                    cwd: None,
                    model_provider: None,
                    created_at: None,
                    updated_at: None,
                    model: None,
                    reasoning_effort: None,
                    service_tier: None,
                    tool_selection_summary: None,
                    action_policy_summary: None,
                    status: AgentPickerThreadStatus::Idle,
                });
        entry.agent_path = Some(activity.agent_path);
        entry.status = if activity.is_running_hint {
            AgentPickerThreadStatus::Running
        } else {
            AgentPickerThreadStatus::Idle
        };
    }

    pub(crate) fn set_running(&mut self, thread_id: ThreadId, is_running: bool) {
        if let Some(entry) = self.threads.get_mut(&thread_id) {
            entry.status = if is_running {
                AgentPickerThreadStatus::Running
            } else {
                AgentPickerThreadStatus::Idle
            };
        }
    }

    pub(crate) fn set_status(&mut self, thread_id: ThreadId, status: AgentPickerThreadStatus) {
        if let Some(entry) = self.threads.get_mut(&thread_id) {
            entry.status = status;
        }
    }

    pub(crate) fn set_agent_path(&mut self, thread_id: ThreadId, agent_path: Option<String>) {
        if let Some(agent_path) = agent_path
            && let Some(entry) = self.threads.get_mut(&thread_id)
        {
            entry.agent_path = Some(agent_path);
        }
    }

    pub(crate) fn update_thread_detail(
        &mut self,
        thread_id: ThreadId,
        detail: AgentPickerThreadDetail,
    ) {
        if let Some(entry) = self.threads.get_mut(&thread_id) {
            if let Some(agent_path) = detail.agent_path.filter(|value| !value.trim().is_empty()) {
                entry.agent_path = Some(agent_path);
            }
            if let Some(prompt_preview) = detail
                .prompt_preview
                .filter(|value| !value.trim().is_empty())
            {
                entry.prompt_preview = Some(prompt_preview);
            }
            entry.thread_note = detail.thread_note.filter(|value| !value.trim().is_empty());
            entry.agent_hidden = detail.agent_hidden;
            if let Some(retry_available) = detail.retry_available {
                entry.retry_available = retry_available;
            }
            if let Some(cwd) = detail.cwd.filter(|value| !value.trim().is_empty()) {
                entry.cwd = Some(cwd);
            }
            if let Some(model_provider) = detail
                .model_provider
                .filter(|value| !value.trim().is_empty())
            {
                entry.model_provider = Some(model_provider);
            }
            if let Some(created_at) = detail.created_at.filter(|value| *value > 0) {
                entry.created_at = Some(created_at);
            }
            if let Some(updated_at) = detail.updated_at.filter(|value| *value > 0) {
                entry.updated_at = Some(updated_at);
            }
            entry.tool_selection_summary = detail
                .tool_selection_summary
                .filter(|value| !value.trim().is_empty());
            entry.action_policy_summary = detail
                .action_policy_summary
                .filter(|value| !value.trim().is_empty());
        }
    }

    pub(crate) fn update_thread_session_detail(
        &mut self,
        thread_id: ThreadId,
        model_provider: Option<String>,
        model: Option<String>,
        reasoning_effort: Option<String>,
        service_tier: Option<String>,
    ) {
        if let Some(entry) = self.threads.get_mut(&thread_id) {
            if let Some(model_provider) = model_provider.filter(|value| !value.trim().is_empty()) {
                entry.model_provider = Some(model_provider);
            }
            if let Some(model) = model.filter(|value| !value.trim().is_empty()) {
                entry.model = Some(model);
            }
            if let Some(reasoning_effort) =
                reasoning_effort.filter(|value| !value.trim().is_empty())
            {
                entry.reasoning_effort = Some(reasoning_effort);
            }
            if let Some(service_tier) = service_tier.filter(|value| !value.trim().is_empty()) {
                entry.service_tier = Some(service_tier);
            }
        }
    }

    /// Marks a thread as closed without removing it from the traversal cache.
    ///
    /// Closed threads stay in the picker and in spawn order so users can still review them and so
    /// next/previous navigation does not reshuffle around disappearing entries. If a caller "cleans
    /// this up" by deleting the entry instead, wraparound navigation will silently change shape
    /// mid-session.
    pub(crate) fn mark_closed(&mut self, thread_id: ThreadId) {
        if let Some(entry) = self.threads.get_mut(&thread_id) {
            entry.status = AgentPickerThreadStatus::Closed;
        } else {
            self.upsert(
                thread_id, /*agent_nickname*/ None, /*agent_role*/ None,
                /*is_closed*/ true,
            );
        }
    }

    /// Drops all cached picker state.
    ///
    /// This is used when `App` tears down thread event state and needs the picker cache to return
    /// to a pristine single-session state.
    pub(crate) fn clear(&mut self) {
        self.threads.clear();
        self.order.clear();
    }

    /// Removes a tracked thread entirely from picker metadata and traversal order.
    ///
    /// This is reserved for entries that were only discovered opportunistically and never became
    /// replayable local threads. Keeping those around after the backend confirms they are gone
    /// would leave ghost rows in `/agent`.
    pub(crate) fn remove(&mut self, thread_id: ThreadId) {
        self.threads.remove(&thread_id);
        self.order.retain(|candidate| *candidate != thread_id);
    }

    /// Returns whether there is at least one tracked thread other than the primary one.
    ///
    /// `App` uses this to decide whether the picker should be available even when the collaboration
    /// feature flag is currently disabled, because already-existing sub-agent threads should remain
    /// inspectable.
    pub(crate) fn has_non_primary_thread(&self, primary_thread_id: Option<ThreadId>) -> bool {
        self.threads
            .keys()
            .any(|thread_id| Some(*thread_id) != primary_thread_id)
    }

    /// Returns live picker rows in the same order users cycle through them.
    ///
    /// The `order` vector is intentionally historical and may briefly contain thread ids that no
    /// longer have cached metadata, so this filters through the map instead of assuming both
    /// collections are perfectly synchronized.
    pub(crate) fn ordered_threads(&self) -> Vec<(ThreadId, &AgentPickerThreadEntry)> {
        self.order
            .iter()
            .filter_map(|thread_id| self.threads.get(thread_id).map(|entry| (*thread_id, entry)))
            .collect()
    }

    pub(crate) fn ordered_path_backed_subagent_threads(
        &self,
        primary_thread_id: Option<ThreadId>,
    ) -> Vec<(ThreadId, &AgentPickerThreadEntry)> {
        self.ordered_threads()
            .into_iter()
            .filter(|(thread_id, entry)| {
                Some(*thread_id) != primary_thread_id
                    && entry
                        .agent_path
                        .as_deref()
                        .is_some_and(|agent_path| !agent_path.trim().is_empty())
            })
            .collect()
    }

    /// Returns the ordered threads that should be visible in Agent Window.
    pub(crate) fn ordered_workbench_threads(&self) -> Vec<(ThreadId, &AgentPickerThreadEntry)> {
        self.ordered_threads()
            .into_iter()
            .filter(|(_thread_id, entry)| !entry.agent_hidden)
            .collect()
    }

    pub(crate) fn role_runtime_usage(
        &self,
        primary_thread_id: Option<ThreadId>,
        active_thread_id: Option<ThreadId>,
    ) -> BTreeMap<String, AgentRoleRuntimeUsage> {
        let mut usage = BTreeMap::<String, AgentRoleRuntimeUsage>::new();
        for (thread_id, entry) in self.ordered_threads() {
            if Some(thread_id) == primary_thread_id {
                continue;
            }
            let Some(role) = entry
                .agent_role
                .as_deref()
                .map(str::trim)
                .filter(|role| !role.is_empty())
            else {
                continue;
            };
            let usage = usage.entry(role.to_string()).or_default();
            usage.total += 1;
            usage.current_view |= Some(thread_id) == active_thread_id;
            match entry.status {
                AgentPickerThreadStatus::Idle => {}
                AgentPickerThreadStatus::Running => usage.running += 1,
                AgentPickerThreadStatus::WaitingApproval | AgentPickerThreadStatus::WaitingUser => {
                    usage.waiting += 1;
                }
                AgentPickerThreadStatus::Error => usage.errors += 1,
                AgentPickerThreadStatus::Closed => usage.closed += 1,
            }
        }
        usage
    }

    /// Returns tracked thread ids in the same stable order used by the picker.
    pub(crate) fn tracked_thread_ids(&self) -> Vec<ThreadId> {
        self.ordered_threads()
            .into_iter()
            .map(|(thread_id, _)| thread_id)
            .collect()
    }

    /// Returns the adjacent thread id for keyboard navigation in stable spawn order.
    ///
    /// The caller must pass the thread whose transcript is actually being shown to the user, not
    /// just whichever thread bookkeeping most recently marked active. If the wrong current thread
    /// is supplied, next/previous navigation will jump in a way that feels nondeterministic even
    /// though the cache itself is correct.
    pub(crate) fn adjacent_thread_id(
        &self,
        current_displayed_thread_id: Option<ThreadId>,
        direction: AgentNavigationDirection,
    ) -> Option<ThreadId> {
        let ordered_threads = self.ordered_threads();
        if ordered_threads.len() < 2 {
            return None;
        }

        let current_thread_id = current_displayed_thread_id?;
        let current_idx = ordered_threads
            .iter()
            .position(|(thread_id, _)| *thread_id == current_thread_id)?;
        let next_idx = match direction {
            AgentNavigationDirection::Next => (current_idx + 1) % ordered_threads.len(),
            AgentNavigationDirection::Previous => {
                if current_idx == 0 {
                    ordered_threads.len() - 1
                } else {
                    current_idx - 1
                }
            }
        };
        Some(ordered_threads[next_idx].0)
    }

    /// Derives the contextual footer label for the currently displayed thread.
    ///
    /// This intentionally returns `None` until there is more than one tracked thread so
    /// single-thread sessions do not waste footer space restating the obvious. When metadata for
    /// the displayed thread is missing, the label falls back to the same generic naming rules used
    /// by the picker.
    pub(crate) fn active_agent_label(
        &self,
        current_displayed_thread_id: Option<ThreadId>,
        primary_thread_id: Option<ThreadId>,
    ) -> Option<String> {
        if self.threads.len() <= 1 {
            return None;
        }

        let thread_id = current_displayed_thread_id?;
        let is_primary = primary_thread_id == Some(thread_id);
        Some(
            self.threads
                .get(&thread_id)
                .map(|entry| {
                    if !is_primary
                        && let Some(agent_path) = entry
                            .agent_path
                            .as_deref()
                            .filter(|agent_path| !agent_path.trim().is_empty())
                    {
                        return format!("`{agent_path}`");
                    }
                    format_agent_picker_item_name(
                        entry.agent_nickname.as_deref(),
                        entry.agent_role.as_deref(),
                        is_primary,
                    )
                })
                .unwrap_or_else(|| {
                    format_agent_picker_item_name(
                        /*agent_nickname*/ None, /*agent_role*/ None, is_primary,
                    )
                }),
        )
    }

    /// Builds the `/agent` picker subtitle from the same canonical bindings used by key handling.
    ///
    /// Keeping this text derived from the actual shortcut helpers prevents the picker copy from
    /// drifting if the bindings ever change on one platform.
    pub(crate) fn picker_subtitle() -> String {
        let previous: Span<'static> = previous_agent_shortcut().into();
        let next: Span<'static> = next_agent_shortcut().into();
        format!(
            "Select a thread to watch. {} previous, {} next.",
            previous.content, next.content
        )
    }

    #[cfg(test)]
    /// Returns only the ordered thread ids for focused tests of traversal invariants.
    ///
    /// This helper exists so tests can assert on ordering without embedding the full picker entry
    /// payload in every expectation.
    pub(crate) fn ordered_thread_ids(&self) -> Vec<ThreadId> {
        self.ordered_threads()
            .into_iter()
            .map(|(thread_id, _)| thread_id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn populated_state() -> (AgentNavigationState, ThreadId, ThreadId, ThreadId) {
        let mut state = AgentNavigationState::default();
        let main_thread_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000101").expect("valid thread");
        let first_agent_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000102").expect("valid thread");
        let second_agent_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000103").expect("valid thread");

        state.upsert(
            main_thread_id,
            /*agent_nickname*/ None,
            /*agent_role*/ None,
            /*is_closed*/ false,
        );
        state.upsert(
            first_agent_id,
            Some("Robie".to_string()),
            Some("explorer".to_string()),
            /*is_closed*/ false,
        );
        state.upsert(
            second_agent_id,
            Some("Bob".to_string()),
            Some("worker".to_string()),
            /*is_closed*/ false,
        );

        (state, main_thread_id, first_agent_id, second_agent_id)
    }

    #[test]
    fn upsert_preserves_first_seen_order() {
        let (mut state, main_thread_id, first_agent_id, second_agent_id) = populated_state();

        state.upsert(
            first_agent_id,
            Some("Robie".to_string()),
            Some("worker".to_string()),
            /*is_closed*/ true,
        );

        assert_eq!(
            state.ordered_thread_ids(),
            vec![main_thread_id, first_agent_id, second_agent_id]
        );
    }

    #[test]
    fn adjacent_thread_id_wraps_in_spawn_order() {
        let (state, main_thread_id, first_agent_id, second_agent_id) = populated_state();

        assert_eq!(
            state.adjacent_thread_id(Some(second_agent_id), AgentNavigationDirection::Next),
            Some(main_thread_id)
        );
        assert_eq!(
            state.adjacent_thread_id(Some(second_agent_id), AgentNavigationDirection::Previous),
            Some(first_agent_id)
        );
        assert_eq!(
            state.adjacent_thread_id(Some(main_thread_id), AgentNavigationDirection::Previous),
            Some(second_agent_id)
        );
    }

    #[test]
    fn picker_subtitle_mentions_shortcuts() {
        let previous: Span<'static> = previous_agent_shortcut().into();
        let next: Span<'static> = next_agent_shortcut().into();
        let subtitle = AgentNavigationState::picker_subtitle();

        assert!(subtitle.contains(previous.content.as_ref()));
        assert!(subtitle.contains(next.content.as_ref()));
    }

    #[test]
    fn active_agent_label_tracks_current_thread() {
        let (state, main_thread_id, first_agent_id, _) = populated_state();

        assert_eq!(
            state.active_agent_label(Some(first_agent_id), Some(main_thread_id)),
            Some("Robie [explorer]".to_string())
        );
        assert_eq!(
            state.active_agent_label(Some(main_thread_id), Some(main_thread_id)),
            Some("Main [default]".to_string())
        );
    }

    #[test]
    fn role_runtime_usage_counts_non_primary_thread_roles() {
        let (mut state, main_thread_id, first_agent_id, second_agent_id) = populated_state();
        state.set_running(first_agent_id, /*is_running*/ true);
        state.set_status(second_agent_id, AgentPickerThreadStatus::WaitingUser);
        let closed_agent_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000104").expect("valid thread");
        state.upsert(
            closed_agent_id,
            Some("Done".to_string()),
            Some("explorer".to_string()),
            /*is_closed*/ true,
        );

        let usage = state.role_runtime_usage(Some(main_thread_id), Some(first_agent_id));

        assert_eq!(
            usage.get("explorer"),
            Some(&AgentRoleRuntimeUsage {
                total: 2,
                running: 1,
                waiting: 0,
                errors: 0,
                closed: 1,
                current_view: true,
            })
        );
        assert_eq!(
            usage.get("worker"),
            Some(&AgentRoleRuntimeUsage {
                total: 1,
                running: 0,
                waiting: 1,
                errors: 0,
                closed: 0,
                current_view: false,
            })
        );
        assert!(!usage.contains_key("default"));
    }
}
