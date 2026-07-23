use crate::agent::exceeds_thread_spawn_depth_limit;
use crate::agent::next_thread_spawn_depth;
use crate::session::turn_context::TurnContext;
use crate::tools::handlers::multi_agents_spec::MULTI_AGENT_V1_NAMESPACE;
use crate::tools::registry::ToolExposure;
use codex_protocol::dynamic_tools::DynamicToolSpec;
use codex_protocol::protocol::MultiAgentVersion;

pub(crate) const NAMESPACE_COLLISION_WARNING: &str = "The multi_agent_v1 tools are unavailable in this thread because the multi_agent_v1 namespace is already in use.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProjectedV1Availability {
    Ineligible,
    NamespaceInUse,
    Available { base_exposure: ToolExposure },
}

pub(crate) fn multi_agent_v2_base_exposure(turn_context: &TurnContext) -> ToolExposure {
    if turn_context.config.multi_agent_v2.non_code_mode_only {
        ToolExposure::DirectModelOnly
    } else {
        ToolExposure::Direct
    }
}

pub(crate) fn projected_v1_availability(
    turn_context: &TurnContext,
    dynamic_tools: &[DynamicToolSpec],
    runtime_namespace_in_use: bool,
) -> ProjectedV1Availability {
    if turn_context.multi_agent_version != MultiAgentVersion::V2
        || crate::guardian::is_guardian_reviewer_source(&turn_context.session_source)
        || !turn_context.provider.capabilities().namespace_tools
        || exceeds_thread_spawn_depth_limit(
            next_thread_spawn_depth(&turn_context.session_source),
            turn_context.config.agent_max_depth,
        )
    {
        return ProjectedV1Availability::Ineligible;
    }

    let configured_namespace_in_use = turn_context.config.multi_agent_v2.tool_namespace.as_deref()
        == Some(MULTI_AGENT_V1_NAMESPACE);
    let dynamic_namespace_in_use = dynamic_tools.iter().any(|tool| {
        matches!(
            tool,
            DynamicToolSpec::Namespace(namespace)
                if namespace.name == MULTI_AGENT_V1_NAMESPACE
        )
    });
    if configured_namespace_in_use || dynamic_namespace_in_use || runtime_namespace_in_use {
        return ProjectedV1Availability::NamespaceInUse;
    }

    ProjectedV1Availability::Available {
        base_exposure: multi_agent_v2_base_exposure(turn_context),
    }
}
