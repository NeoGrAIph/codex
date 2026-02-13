pub(crate) mod control;
mod guards;
pub(crate) mod role;
// [SA] COMMIT OPEN: agent templates module
// Role: support template-backed roles/personas for `spawn_agent` without bloating upstream modules.
pub(crate) mod role_templates;
// [SA] COMMIT CLOSE: agent templates module
pub(crate) mod status;

pub(crate) use codex_protocol::protocol::AgentStatus;
pub(crate) use control::AgentControl;
pub(crate) use guards::MAX_THREAD_SPAWN_DEPTH;
pub(crate) use guards::exceeds_thread_spawn_depth_limit;
pub(crate) use guards::next_thread_spawn_depth;
pub(crate) use role::AgentRole;
pub(crate) use status::agent_status_from_event;
