use super::*;
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PersistedAgentLineage {
    pub(crate) parent_thread_id: ThreadId,
    pub(crate) depth: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AuthorizedAgentTarget {
    Live,
    Persisted(PersistedAgentLineage),
}

impl AgentControl {
    /// Authorize a live or persisted target against this root agent-control tree.
    ///
    /// Thread ids are locators, not capabilities. Live targets must be registered in this
    /// session-scoped registry. Closed targets may be resumed only when the authoritative
    /// persisted spawn graph proves that they descend from this control's root thread.
    pub(crate) async fn ensure_agent_known_or_persisted_descendant(
        &self,
        agent_id: ThreadId,
    ) -> CodexResult<AuthorizedAgentTarget> {
        self.ensure_no_pending_lifecycle_cleanup(agent_id)?;
        let state = self.upgrade()?;
        if self.state.agent_metadata_for_thread(agent_id).is_some()
            && state.get_thread(agent_id).await.is_ok()
        {
            return Ok(AuthorizedAgentTarget::Live);
        }

        self.persisted_agent_lineage(agent_id)
            .await
            .map(AuthorizedAgentTarget::Persisted)
    }

    pub(super) async fn persisted_agent_lineage(
        &self,
        agent_id: ThreadId,
    ) -> CodexResult<PersistedAgentLineage> {
        let Some(root_thread_id) = self.state.agent_id_for_path(&AgentPath::root()) else {
            return Err(CodexErr::ThreadNotFound(agent_id));
        };
        if agent_id == root_thread_id {
            return Err(CodexErr::ThreadNotFound(agent_id));
        }

        let state = self.upgrade()?;
        let agent_graph_store = state.agent_graph_store().ok_or_else(|| {
            CodexErr::Fatal(
                "authoritative agent graph is unavailable; refusing persisted agent authorization"
                    .to_string(),
            )
        })?;
        let mut visited = HashSet::from([agent_id]);
        let mut current_thread_id = agent_id;
        let mut immediate_parent_thread_id = None;
        let mut absolute_depth = 0_usize;
        let mut control_root_seen = false;
        loop {
            let edge = agent_graph_store
                .get_thread_spawn_edge(current_thread_id)
                .await
                .map_err(|err| {
                    warn!(%err, %agent_id, "failed to verify persisted agent ownership");
                    CodexErr::Fatal("failed to verify persisted agent ownership".to_string())
                })?;
            let Some(edge) = edge else {
                if !control_root_seen {
                    return Err(CodexErr::ThreadNotFound(agent_id));
                }
                let depth = i32::try_from(absolute_depth).map_err(|_| {
                    CodexErr::Fatal("persisted agent graph depth is invalid".to_string())
                })?;
                let parent_thread_id = immediate_parent_thread_id.ok_or_else(|| {
                    CodexErr::Fatal("persisted agent graph is missing target lineage".to_string())
                })?;
                return Ok(PersistedAgentLineage {
                    parent_thread_id,
                    depth,
                });
            };
            match edge.status {
                codex_agent_graph_store::ThreadSpawnEdgeStatus::PendingActivation => {
                    return Err(CodexErr::ThreadNotFound(agent_id));
                }
                codex_agent_graph_store::ThreadSpawnEdgeStatus::Open
                | codex_agent_graph_store::ThreadSpawnEdgeStatus::Closed => {}
            }
            let parent_thread_id = edge.parent_thread_id;
            immediate_parent_thread_id.get_or_insert(parent_thread_id);
            if parent_thread_id == root_thread_id {
                control_root_seen = true;
            }
            if visited.contains(&parent_thread_id) {
                return Err(CodexErr::Fatal(format!(
                    "persisted agent graph contains a cycle while authorizing {agent_id}"
                )));
            }
            if absolute_depth == MAX_AGENT_GRAPH_DEPTH as usize {
                return Err(CodexErr::Fatal(format!(
                    "persisted agent graph exceeds the authorization depth limit of {MAX_AGENT_GRAPH_DEPTH}"
                )));
            }
            visited.insert(parent_thread_id);
            absolute_depth += 1;
            current_thread_id = parent_thread_id;
        }
    }
}
