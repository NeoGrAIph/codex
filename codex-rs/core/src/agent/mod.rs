pub(crate) mod control;
mod guards;
// === FORK: AGENT REGISTRY START ===
pub(crate) mod registry;
// === FORK: AGENT REGISTRY END ===
pub(crate) mod role;
pub(crate) mod status;

pub(crate) use codex_protocol::protocol::AgentStatus;
pub(crate) use control::AgentControl;
pub(crate) use guards::MAX_THREAD_SPAWN_DEPTH;
pub(crate) use guards::exceeds_thread_spawn_depth_limit;
pub(crate) use guards::next_thread_spawn_depth;
pub(crate) use status::agent_status_from_event;

// AgentRole kept for backward compatibility but deprecated
#[allow(unused_imports)]
pub(crate) use role::AgentRole;
