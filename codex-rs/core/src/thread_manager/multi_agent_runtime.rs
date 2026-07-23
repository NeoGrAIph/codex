use crate::agent::control::MultiAgentRuntimeIntent;
use crate::config::Config;
use codex_protocol::ThreadId;
use codex_protocol::error::CodexErr;
use codex_protocol::error::Result as CodexResult;
use codex_protocol::protocol::InitialHistory;
use codex_protocol::protocol::MultiAgentVersion;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use codex_protocol::protocol::ThreadHistoryMode;
use codex_protocol::protocol::ThreadSource;

#[allow(clippy::too_many_arguments)]
pub(crate) fn normalize_multi_agent_runtime_intent(
    requested: MultiAgentRuntimeIntent,
    initial_history: &InitialHistory,
    history_mode: Option<ThreadHistoryMode>,
    session_source: &SessionSource,
    parent_thread_id: Option<ThreadId>,
    forked_from_thread_id: Option<ThreadId>,
    thread_source: Option<&ThreadSource>,
    config: &Config,
) -> CodexResult<MultiAgentRuntimeIntent> {
    if config.multi_agent_version_override() == Some(MultiAgentVersion::Disabled) {
        return match requested {
            MultiAgentRuntimeIntent::Inherit => Ok(requested),
            MultiAgentRuntimeIntent::ExactV1Spawn | MultiAgentRuntimeIntent::ExactV1Resume => {
                Err(CodexErr::InvalidRequest(
                    "exact V1 runtime is unavailable while multi-agent support is disabled"
                        .to_string(),
                ))
            }
        };
    }

    let pathless_parent = match session_source {
        SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id,
            agent_path: None,
            ..
        }) => Some(*parent_thread_id),
        _ => None,
    };
    let resolved_history_version =
        crate::agent::control::resolve_persisted_multi_agent_version_for_exact_v1(
            initial_history,
            history_mode
                .unwrap_or_else(|| initial_history.get_history_mode(ThreadHistoryMode::Legacy)),
        );
    let persisted_pathless_thread_spawn = initial_history
        .get_resumed_session_sources()
        .is_some_and(|(persisted_source, _)| {
            matches!(
                persisted_source,
                SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                    agent_path: None,
                    ..
                })
            )
        });
    let requested = if requested == MultiAgentRuntimeIntent::Inherit
        && matches!(initial_history, InitialHistory::Resumed(_))
        && pathless_parent.is_some()
        && persisted_pathless_thread_spawn
        && resolved_history_version == Some(MultiAgentVersion::V1)
        && config.multi_agent_version_override() == Some(MultiAgentVersion::V2)
    {
        MultiAgentRuntimeIntent::ExactV1Resume
    } else {
        requested
    };

    match requested {
        MultiAgentRuntimeIntent::Inherit => Ok(requested),
        MultiAgentRuntimeIntent::ExactV1Spawn => {
            let Some(source_parent_thread_id) = pathless_parent else {
                return Err(CodexErr::InvalidRequest(
                    "exact V1 spawn runtime requires a pathless thread-spawn source".to_string(),
                ));
            };
            let valid_history = match initial_history {
                InitialHistory::New => forked_from_thread_id.is_none(),
                InitialHistory::Forked(_) => forked_from_thread_id == Some(source_parent_thread_id),
                InitialHistory::Cleared | InitialHistory::Resumed(_) => false,
            };
            if !valid_history
                || parent_thread_id != Some(source_parent_thread_id)
                || !matches!(thread_source, Some(ThreadSource::Subagent))
            {
                return Err(CodexErr::InvalidRequest(
                    "exact V1 spawn runtime requires consistent pathless subagent creation"
                        .to_string(),
                ));
            }
            Ok(requested)
        }
        MultiAgentRuntimeIntent::ExactV1Resume => {
            if !matches!(initial_history, InitialHistory::Resumed(_))
                || pathless_parent.is_none()
                || resolved_history_version != Some(MultiAgentVersion::V1)
            {
                return Err(CodexErr::InvalidRequest(
                    "exact V1 resume runtime requires resumed V1 history and a pathless caller thread-spawn source".to_string(),
                ));
            }
            Ok(requested)
        }
    }
}
