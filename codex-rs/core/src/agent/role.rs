//! Legacy agent role definitions.
//!
//! This module is preserved for backward compatibility. New code should use
//! agent names from the registry system in `registry.rs`.

#![allow(dead_code)]

use serde::Deserialize;
use serde::Serialize;

/// Legacy agent role enum for backward compatibility.
///
/// New code should use agent names from registry instead of these hard-coded roles.
/// The registry provides dynamic agent definitions loaded from YAML files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    /// Inherit the parent agent's configuration unchanged.
    Default,
    /// Coordination-only agent that delegates to workers.
    Orchestrator,
    /// Task-executing agent with a fixed model override.
    Worker,
    /// Fast codebase exploration agent.
    Explorer,
    /// Reviews code for correctness, security, and maintainability.
    Reviewer,
    /// Designs architecture and proposes implementation plans.
    Architect,
    /// Hunts for bugs, regressions, and edge cases.
    BugHunter,
}

impl AgentRole {
    /// Converts legacy role to agent name for registry lookup.
    ///
    /// Returns `None` for `Default` role as it should inherit parent config.
    pub fn to_agent_name(self) -> Option<&'static str> {
        match self {
            AgentRole::Default => None,
            AgentRole::Orchestrator => Some("orchestrator"),
            AgentRole::Worker => Some("worker"),
            AgentRole::Explorer => Some("explorer"),
            AgentRole::Reviewer => Some("reviewer"),
            AgentRole::Architect => Some("architect"),
            AgentRole::BugHunter => Some("bug-hunter"),
        }
    }
}
