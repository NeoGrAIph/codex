use crate::types::StoredThread;
use codex_protocol::AgentPath;
use codex_protocol::ThreadId;
use codex_protocol::protocol::RolloutItem;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;

const MAX_PERSISTED_AGENT_DEPTH: i32 = 256;
const MAX_PERSISTED_AGENT_PATH_BYTES: usize = 4 * 1_024;

impl StoredThread {
    /// Projects current store-owned identity metadata into the canonical session metadata item.
    ///
    /// SQLite-backed metadata can contain a compatibility backfill that is newer than an older
    /// rollout header. Resume callers consume history, so they must see the same source, parent,
    /// role, nickname, and canonical path as metadata-only readers.
    pub fn project_identity_into_history(&mut self) -> Result<(), String> {
        let thread_id = self.thread_id;
        let Some(history) = self.history.as_mut() else {
            return Ok(());
        };
        let Some(RolloutItem::SessionMeta(meta_line)) = history.items.iter_mut().find(
            |item| matches!(item, RolloutItem::SessionMeta(meta) if meta.meta.id == thread_id),
        ) else {
            if matches!(
                &self.source,
                SessionSource::SubAgent(SubAgentSource::ThreadSpawn { .. })
            ) || self.parent_thread_id.is_some()
                || self.agent_path.is_some()
                || self.agent_nickname.is_some()
                || self.agent_role.is_some()
            {
                return Err(format!(
                    "stored agent identity for thread {thread_id} has no matching session metadata"
                ));
            }
            return Ok(());
        };
        let metadata_parent_thread_id = self.parent_thread_id;
        if self
            .agent_path
            .as_ref()
            .is_some_and(|path| path.len() > MAX_PERSISTED_AGENT_PATH_BYTES)
        {
            return Err(format!(
                "stored agent path for thread {thread_id} exceeds {MAX_PERSISTED_AGENT_PATH_BYTES} bytes"
            ));
        }
        let metadata_agent_path = self
            .agent_path
            .as_deref()
            .map(AgentPath::try_from)
            .transpose()
            .map_err(|err| format!("invalid stored agent path for thread {thread_id}: {err}"))?;
        let metadata_agent_nickname = self.agent_nickname.clone();
        let metadata_agent_role = self.agent_role.clone();
        let canonical_source = match &self.source {
            SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth,
                agent_path,
                agent_nickname,
                agent_role,
            }) => {
                if metadata_parent_thread_id.is_some_and(|metadata_parent_thread_id| {
                    metadata_parent_thread_id != *parent_thread_id
                }) {
                    return Err(format!(
                        "stored parent metadata disagrees with source for thread {thread_id}"
                    ));
                }
                if let (Some(metadata_agent_path), Some(source_agent_path)) =
                    (metadata_agent_path.as_ref(), agent_path.as_ref())
                    && metadata_agent_path != source_agent_path
                {
                    return Err(format!(
                        "stored path metadata disagrees with source for thread {thread_id}"
                    ));
                }
                let canonical_agent_path = metadata_agent_path.or_else(|| agent_path.clone());
                validate_thread_spawn_identity(
                    thread_id,
                    *parent_thread_id,
                    *depth,
                    canonical_agent_path.as_ref(),
                )?;
                SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                    parent_thread_id: *parent_thread_id,
                    depth: *depth,
                    agent_path: canonical_agent_path,
                    agent_nickname: metadata_agent_nickname
                        .clone()
                        .or_else(|| agent_nickname.clone()),
                    agent_role: metadata_agent_role.clone().or_else(|| agent_role.clone()),
                })
            }
            SessionSource::Unknown => match (metadata_parent_thread_id, metadata_agent_path) {
                (Some(parent_thread_id), Some(agent_path)) => {
                    let depth = thread_spawn_depth_from_path(&agent_path).ok_or_else(|| {
                        format!(
                            "stored agent path has no thread-spawn depth for thread {thread_id}"
                        )
                    })?;
                    validate_thread_spawn_identity(
                        thread_id,
                        parent_thread_id,
                        depth,
                        Some(&agent_path),
                    )?;
                    SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                        parent_thread_id,
                        depth,
                        agent_path: Some(agent_path),
                        agent_nickname: metadata_agent_nickname.clone(),
                        agent_role: metadata_agent_role.clone(),
                    })
                }
                _ => SessionSource::Unknown,
            },
            source => source.clone(),
        };
        self.parent_thread_id = self
            .parent_thread_id
            .or_else(|| canonical_source.parent_thread_id());
        self.agent_nickname = self
            .agent_nickname
            .clone()
            .or_else(|| canonical_source.get_nickname());
        self.agent_role = self
            .agent_role
            .clone()
            .or_else(|| canonical_source.get_agent_role());
        self.agent_path = canonical_source
            .get_agent_path()
            .map(Into::into)
            .or_else(|| self.agent_path.clone());
        self.source = canonical_source.clone();
        meta_line.meta.parent_thread_id =
            metadata_parent_thread_id.or_else(|| canonical_source.parent_thread_id());
        meta_line.meta.agent_nickname =
            metadata_agent_nickname.or_else(|| canonical_source.get_nickname());
        meta_line.meta.agent_role =
            metadata_agent_role.or_else(|| canonical_source.get_agent_role());
        meta_line.meta.agent_path = canonical_source
            .get_agent_path()
            .map(Into::into)
            .or_else(|| self.agent_path.clone());
        meta_line.meta.source = canonical_source;
        Ok(())
    }
}

fn validate_thread_spawn_identity(
    thread_id: ThreadId,
    parent_thread_id: ThreadId,
    depth: i32,
    agent_path: Option<&AgentPath>,
) -> Result<(), String> {
    if parent_thread_id == thread_id {
        return Err(format!(
            "stored agent lineage for thread {thread_id} points to itself as parent"
        ));
    }
    if !(1..=MAX_PERSISTED_AGENT_DEPTH).contains(&depth) {
        return Err(format!(
            "stored agent lineage for thread {thread_id} has invalid depth {depth}"
        ));
    }
    if let Some(agent_path) = agent_path {
        if agent_path.as_str().len() > MAX_PERSISTED_AGENT_PATH_BYTES {
            return Err(format!(
                "stored agent path for thread {thread_id} exceeds {MAX_PERSISTED_AGENT_PATH_BYTES} bytes"
            ));
        }
        if thread_spawn_depth_from_path(agent_path) != Some(depth) {
            return Err(format!(
                "stored agent path depth disagrees with lineage for thread {thread_id}"
            ));
        }
    }
    Ok(())
}

fn thread_spawn_depth_from_path(agent_path: &AgentPath) -> Option<i32> {
    agent_path
        .as_str()
        .strip_prefix("/root/")
        .and_then(|path| i32::try_from(path.split('/').count()).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thread_spawn_identity_rejects_invalid_lineage_boundaries() {
        let thread_id = ThreadId::new();
        let parent_thread_id = ThreadId::new();
        let depth_one_path = AgentPath::try_from("/root/worker").expect("valid agent path");
        let depth_two_path =
            AgentPath::try_from("/root/worker/reviewer").expect("valid agent path");

        let self_parent_err =
            validate_thread_spawn_identity(thread_id, thread_id, 1, Some(&depth_one_path))
                .expect_err("self-parent lineage must fail closed");
        assert!(self_parent_err.contains("points to itself as parent"));

        for invalid_depth in [0, MAX_PERSISTED_AGENT_DEPTH + 1] {
            let depth_err =
                validate_thread_spawn_identity(thread_id, parent_thread_id, invalid_depth, None)
                    .expect_err("out-of-range lineage depth must fail closed");
            assert!(depth_err.contains("has invalid depth"));
        }

        let path_depth_err =
            validate_thread_spawn_identity(thread_id, parent_thread_id, 1, Some(&depth_two_path))
                .expect_err("path-depth disagreement must fail closed");
        assert!(path_depth_err.contains("path depth disagrees with lineage"));
    }

    #[test]
    fn thread_spawn_identity_enforces_agent_path_byte_boundary() {
        let thread_id = ThreadId::new();
        let parent_thread_id = ThreadId::new();
        let path_at_limit = AgentPath::try_from(format!(
            "/root/{}",
            "a".repeat(MAX_PERSISTED_AGENT_PATH_BYTES - "/root/".len())
        ))
        .expect("agent path at byte limit");
        validate_thread_spawn_identity(thread_id, parent_thread_id, 1, Some(&path_at_limit))
            .expect("agent path at byte limit must be accepted");

        let path_over_limit = AgentPath::try_from(format!(
            "/root/{}",
            "a".repeat(MAX_PERSISTED_AGENT_PATH_BYTES + 1 - "/root/".len())
        ))
        .expect("syntactically valid agent path over persistence limit");
        let path_err =
            validate_thread_spawn_identity(thread_id, parent_thread_id, 1, Some(&path_over_limit))
                .expect_err("agent path over byte limit must fail closed");
        assert!(path_err.contains("exceeds 4096 bytes"));
    }
}
