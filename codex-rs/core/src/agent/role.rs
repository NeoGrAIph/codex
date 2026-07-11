//! Applies agent-role configuration layers on top of an existing session config.
//!
//! Roles are selected at spawn time and are loaded with the same config machinery as
//! `config.toml`. This module resolves built-in and user-defined role files, inserts the role as a
//! high-precedence layer, and preserves the caller's current provider and service tier unless the
//! role layer sets them. It does not decide when to spawn a sub-agent or which role to use; the
//! multi-agent tool handler owns that orchestration.

use crate::config::AgentRoleConfig;
use crate::config::Config;
use crate::config::ConfigOverrides;
use crate::config::agent_roles::agent_role_locked_settings_note;
use crate::config::deserialize_config_toml_with_base;
use anyhow::anyhow;
use codex_config::ConfigLayerEntry;
use codex_config::ConfigLayerSource;
use codex_config::ConfigLayerStack;
use codex_config::ConfigLayerStackOrdering;
use codex_config::config_toml::ConfigToml;
use codex_config::loader::resolve_relative_paths_in_config_toml;
use codex_exec_server::LOCAL_FS;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::LazyLock;
use toml::Value as TomlValue;

/// The role name used when a caller omits `agent_type`.
pub const DEFAULT_ROLE_NAME: &str = "default";
const AGENT_TYPE_UNAVAILABLE_ERROR: &str = "agent type is currently not available";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ResumeRoleOverridePolicy {
    RoleWins,
    ExplicitSessionFlagsWin,
}

/// Applies a named role layer to `config` while preserving caller-owned provider settings.
///
/// The role layer is inserted at session-flag precedence so it can override persisted config, but
/// the caller's current `model_provider` and `service_tier` remain sticky runtime choices unless
/// the role explicitly sets the corresponding top-level config key. Rebuilding the config without
/// those overrides would make a spawned agent silently fall back to default settings.
pub(crate) async fn apply_role_to_config(
    config: &mut Config,
    role_name: Option<&str>,
) -> Result<(), String> {
    let role_name = role_name.unwrap_or(DEFAULT_ROLE_NAME);

    let role = resolve_role_config(config, role_name)
        .cloned()
        .ok_or_else(|| format!("unknown agent_type '{role_name}'"))?;

    apply_role_to_config_inner(config, role_name, &role)
        .await
        .map_err(|err| {
            tracing::warn!("failed to apply role to config: {err}");
            AGENT_TYPE_UNAVAILABLE_ERROR.to_string()
        })
}

/// Reapply the current trusted role definition when a persisted thread-spawn agent is resumed.
///
/// Rollouts persist the role name, not an effective `Config` snapshot. Re-resolving that name
/// avoids trusting stale persisted permissions while preserving role-owned model and instruction
/// settings. Runtime-owned permissions, workspace roots, cwd, and base-instruction values remain
/// those selected by the current resume caller.
pub(crate) async fn reapply_role_to_resumed_agent_config(
    config: &mut Config,
    session_source: &SessionSource,
    override_policy: ResumeRoleOverridePolicy,
) -> Result<(), String> {
    let SessionSource::SubAgent(SubAgentSource::ThreadSpawn { agent_role, .. }) = session_source
    else {
        return Ok(());
    };
    let Some(agent_role) = agent_role.as_deref() else {
        // Historical full-history forks did not persist an effective role. Guessing `default`
        // here would replace their inherited configuration during resume.
        return Ok(());
    };

    let permissions = config.permissions.clone();
    let explicit_permission_profile_mode = config.explicit_permission_profile_mode;
    let custom_permission_profiles = config.custom_permission_profiles.clone();
    let approvals_reviewer = config.approvals_reviewer;
    let cwd = config.cwd.clone();
    let workspace_roots = config.workspace_roots.clone();
    let workspace_roots_explicit = config.workspace_roots_explicit;
    let base_instructions = config.base_instructions.clone();
    let preserve_explicit_session_flags =
        override_policy == ResumeRoleOverridePolicy::ExplicitSessionFlagsWin;
    let explicit_model = preserve_explicit_session_flags && session_flags_contain(config, "model");
    let explicit_model_provider =
        preserve_explicit_session_flags && session_flags_contain(config, "model_provider");
    let explicit_reasoning_effort =
        preserve_explicit_session_flags && session_flags_contain(config, "model_reasoning_effort");
    let explicit_reasoning_summary =
        preserve_explicit_session_flags && session_flags_contain(config, "model_reasoning_summary");
    let explicit_service_tier =
        preserve_explicit_session_flags && session_flags_contain(config, "service_tier");
    let explicit_developer_instructions =
        preserve_explicit_session_flags && session_flags_contain(config, "developer_instructions");
    let explicit_personality =
        preserve_explicit_session_flags && session_flags_contain(config, "personality");
    let model = config.model.clone();
    let model_provider_id = config.model_provider_id.clone();
    let model_provider = config.model_provider.clone();
    let model_reasoning_effort = config.model_reasoning_effort.clone();
    let model_reasoning_summary = config.model_reasoning_summary;
    let service_tier = config.service_tier.clone();
    let developer_instructions = config.developer_instructions.clone();
    let personality = config.personality;

    apply_role_to_config(config, Some(agent_role)).await?;

    config.permissions = permissions;
    config.explicit_permission_profile_mode = explicit_permission_profile_mode;
    config.custom_permission_profiles = custom_permission_profiles;
    config.approvals_reviewer = approvals_reviewer;
    config.cwd = cwd;
    config.workspace_roots = workspace_roots;
    config.workspace_roots_explicit = workspace_roots_explicit;
    config.base_instructions = base_instructions;
    if explicit_model {
        config.model = model;
    }
    if explicit_model_provider {
        config.model_provider_id = model_provider_id;
        config.model_provider = model_provider;
    }
    if explicit_reasoning_effort {
        config.model_reasoning_effort = model_reasoning_effort;
    }
    if explicit_reasoning_summary {
        config.model_reasoning_summary = model_reasoning_summary;
    }
    if explicit_service_tier {
        config.service_tier = service_tier;
    }
    if explicit_developer_instructions {
        config.developer_instructions = developer_instructions;
    }
    if explicit_personality {
        config.personality = personality;
    }
    Ok(())
}

fn session_flags_contain(config: &Config, key: &str) -> bool {
    config
        .config_layer_stack
        .get_layers(
            ConfigLayerStackOrdering::HighestPrecedenceFirst,
            /*include_disabled*/ false,
        )
        .into_iter()
        .filter(|layer| matches!(layer.name, ConfigLayerSource::SessionFlags))
        .any(|layer| {
            layer
                .config
                .as_table()
                .is_some_and(|table| table.contains_key(key))
        })
}

async fn apply_role_to_config_inner(
    config: &mut Config,
    role_name: &str,
    role: &AgentRoleConfig,
) -> anyhow::Result<()> {
    let is_built_in = !config.agent_roles.contains_key(role_name);
    let Some(config_file) = role.config_file.as_ref() else {
        return Ok(());
    };
    let role_layer_toml = load_role_layer_toml(config, config_file, is_built_in, role_name).await?;
    if role_layer_toml
        .as_table()
        .is_some_and(toml::map::Map::is_empty)
    {
        return Ok(());
    }
    let preserve_current_provider = role_layer_toml.get("model_provider").is_none();
    let preserve_current_service_tier = role_layer_toml.get("service_tier").is_none();
    let preserve_current_model = role_layer_toml.get("model").is_none();
    let preserve_current_reasoning_effort = role_layer_toml.get("model_reasoning_effort").is_none();
    let preserve_current_reasoning_summary =
        role_layer_toml.get("model_reasoning_summary").is_none();
    let preserve_current_base_instructions = role_layer_toml.get("base_instructions").is_none();
    let preserve_current_developer_instructions =
        role_layer_toml.get("developer_instructions").is_none();
    let current_model = config.model.clone();
    let current_reasoning_effort = config.model_reasoning_effort.clone();
    let current_reasoning_summary = config.model_reasoning_summary;
    let current_base_instructions = config.base_instructions.clone();
    let current_developer_instructions = config.developer_instructions.clone();

    let mut next_config = reload::build_next_config(
        config,
        role_layer_toml,
        preserve_current_provider,
        preserve_current_service_tier,
    )
    .await?;
    if preserve_current_model {
        next_config.model = current_model;
    }
    if preserve_current_reasoning_effort {
        next_config.model_reasoning_effort = current_reasoning_effort;
    }
    if preserve_current_reasoning_summary {
        next_config.model_reasoning_summary = current_reasoning_summary;
    }
    if preserve_current_base_instructions {
        next_config.base_instructions = current_base_instructions;
    }
    if preserve_current_developer_instructions {
        next_config.developer_instructions = current_developer_instructions;
    }
    *config = next_config;
    Ok(())
}

async fn load_role_layer_toml(
    config: &Config,
    config_file: &Path,
    is_built_in: bool,
    role_name: &str,
) -> anyhow::Result<TomlValue> {
    let (role_config_toml, role_config_base) = if is_built_in {
        let role_config_contents = built_in::config_file_contents(config_file)
            .map(str::to_owned)
            .ok_or(anyhow!("No corresponding config content"))?;
        let role_config_toml: TomlValue = toml::from_str(&role_config_contents)?;
        (role_config_toml, config.codex_home.to_path_buf())
    } else {
        let layer = config
            .materialized_agent_role_layers
            .get(role_name)
            .ok_or(anyhow!("No materialized role content"))?;
        (layer.config.clone(), layer.base_dir.clone())
    };

    deserialize_config_toml_with_base(role_config_toml.clone(), &role_config_base)?;
    Ok(resolve_relative_paths_in_config_toml(
        role_config_toml,
        &role_config_base,
    )?)
}

pub(crate) fn resolve_role_config<'a>(
    config: &'a Config,
    role_name: &str,
) -> Option<&'a AgentRoleConfig> {
    config
        .agent_roles
        .get(role_name)
        .or_else(|| built_in::configs().get(role_name))
}

mod reload {
    use super::*;

    pub(super) async fn build_next_config(
        config: &Config,
        role_layer_toml: TomlValue,
        preserve_current_provider: bool,
        preserve_current_service_tier: bool,
    ) -> anyhow::Result<Config> {
        let config_layer_stack = build_config_layer_stack(config, &role_layer_toml)?;
        let merged_config = deserialize_effective_config(config, &config_layer_stack)?;
        let loading_layer_stack = without_agent_role_declarations(&config_layer_stack)?;

        let mut next_config = Config::load_config_with_layer_stack(
            LOCAL_FS.as_ref(),
            merged_config,
            reload_overrides(
                config,
                preserve_current_provider,
                preserve_current_service_tier,
            ),
            config.codex_home.clone(),
            loading_layer_stack,
        )
        .await?;
        next_config.config_layer_stack = config_layer_stack;
        next_config.agent_roles.clone_from(&config.agent_roles);
        next_config
            .materialized_agent_role_layers
            .clone_from(&config.materialized_agent_role_layers);
        Ok(next_config)
    }

    fn without_agent_role_declarations(
        config_layer_stack: &ConfigLayerStack,
    ) -> anyhow::Result<ConfigLayerStack> {
        let mut layers = config_layer_stack
            .get_layers(
                ConfigLayerStackOrdering::LowestPrecedenceFirst,
                /*include_disabled*/ true,
            )
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        for layer in &mut layers {
            if let Some(table) = layer.config.as_table_mut() {
                table.remove("agents");
            }
        }
        Ok(ConfigLayerStack::new(
            layers,
            config_layer_stack.requirements().clone(),
            config_layer_stack.requirements_toml().clone(),
        )?)
    }

    fn build_config_layer_stack(
        config: &Config,
        role_layer_toml: &TomlValue,
    ) -> anyhow::Result<ConfigLayerStack> {
        let mut layers = existing_layers(config);
        insert_layer(&mut layers, role_layer(role_layer_toml.clone()));
        Ok(ConfigLayerStack::new(
            layers,
            config.config_layer_stack.requirements().clone(),
            config.config_layer_stack.requirements_toml().clone(),
        )?)
    }

    fn deserialize_effective_config(
        config: &Config,
        config_layer_stack: &ConfigLayerStack,
    ) -> anyhow::Result<ConfigToml> {
        Ok(deserialize_config_toml_with_base(
            config_layer_stack.effective_config(),
            &config.codex_home,
        )?)
    }

    fn existing_layers(config: &Config) -> Vec<ConfigLayerEntry> {
        config
            .config_layer_stack
            .get_layers(
                ConfigLayerStackOrdering::LowestPrecedenceFirst,
                /*include_disabled*/ true,
            )
            .into_iter()
            .cloned()
            .collect()
    }

    fn insert_layer(layers: &mut Vec<ConfigLayerEntry>, layer: ConfigLayerEntry) {
        let insertion_index =
            layers.partition_point(|existing_layer| existing_layer.name <= layer.name);
        layers.insert(insertion_index, layer);
    }

    fn role_layer(role_layer_toml: TomlValue) -> ConfigLayerEntry {
        ConfigLayerEntry::new(ConfigLayerSource::SessionFlags, role_layer_toml)
    }

    fn reload_overrides(
        config: &Config,
        preserve_current_provider: bool,
        preserve_current_service_tier: bool,
    ) -> ConfigOverrides {
        ConfigOverrides {
            cwd: Some(config.cwd.to_path_buf()),
            model_provider: preserve_current_provider.then(|| config.model_provider_id.clone()),
            service_tier: preserve_current_service_tier.then(|| config.service_tier.clone()),
            codex_linux_sandbox_exe: config.codex_linux_sandbox_exe.clone(),
            main_execve_wrapper_exe: config.main_execve_wrapper_exe.clone(),
            ..Default::default()
        }
    }
}

#[path = "role/spawn_tool_spec.rs"]
pub(crate) mod spawn_tool_spec;

mod built_in {
    use super::*;

    /// Returns the cached built-in role declarations defined in this module.
    pub(super) fn configs() -> &'static BTreeMap<String, AgentRoleConfig> {
        static CONFIG: LazyLock<BTreeMap<String, AgentRoleConfig>> = LazyLock::new(|| {
            BTreeMap::from([
                (
                    DEFAULT_ROLE_NAME.to_string(),
                    AgentRoleConfig {
                        description: Some("Default agent.".to_string()),
                        config_file: None,
                        nickname_candidates: None,
                    }
                ),
                (
                    "explorer".to_string(),
                    AgentRoleConfig {
                        description: Some(r#"Use `explorer` for specific codebase questions.
Explorers are fast and authoritative.
They must be used to ask specific, well-scoped questions on the codebase.
Rules:
- In order to avoid redundant work, you should avoid exploring the same problem that explorers have already covered. Typically, you should trust the explorer results without additional verification. You are still allowed to inspect the code yourself to gain the needed context!
- You are encouraged to spawn up multiple explorers in parallel when you have multiple distinct questions to ask about the codebase that can be answered independently. This allows you to get more information faster without waiting for one question to finish before asking the next. While waiting for the explorer results, you can continue working on other local tasks that do not depend on those results. This parallelism is a key advantage of delegation, so use it whenever you have multiple questions to ask.
- Reuse existing explorers for related questions."#.to_string()),
                        config_file: Some("explorer.toml".to_string().parse().unwrap_or_default()),
                        nickname_candidates: None,
                    }
                ),
                (
                    "worker".to_string(),
                    AgentRoleConfig {
                        description: Some(r#"Use for execution and production work.
Typical tasks:
- Implement part of a feature
- Fix tests or bugs
- Split large refactors into independent chunks
Rules:
- Explicitly assign **ownership** of the task (files / responsibility). When the subtask involves code changes, you should clearly specify which files or modules the worker is responsible for. This helps avoid merge conflicts and ensures accountability. For example, you can say "Worker 1 is responsible for updating the authentication module, while Worker 2 will handle the database layer." By defining clear ownership, you can delegate more effectively and reduce coordination overhead.
- Always tell workers they are **not alone in the codebase**, and they should not revert the edits made by others, and they should adjust their implementation to accommodate the changes made by others. This is important because there may be multiple workers making changes in parallel, and they need to be aware of each other's work to avoid conflicts and ensure a cohesive final product."#.to_string()),
                        config_file: None,
                        nickname_candidates: None,
                    }
                ),
                // Awaiter is temp removed
//                 (
//                     "awaiter".to_string(),
//                     AgentRoleConfig {
//                         description: Some(r#"Use an `awaiter` agent EVERY TIME you must run a command that will take some very long time.
// This includes, but not only:
// * testing
// * monitoring of a long running process
// * explicit ask to wait for something
//
// Rules:
// - When an awaiter is running, you can work on something else. If you need to wait for its completion, use the largest possible timeout.
// - Be patient with the `awaiter`.
// - Do not use an awaiter for every compilation/test if it won't take time. Only use if for long running commands.
// - Close the awaiter when you're done with it."#.to_string()),
//                         config_file: Some("awaiter.toml".to_string().parse().unwrap_or_default()),
//                     }
//                 )
            ])
        });
        &CONFIG
    }

    /// Resolves a built-in role `config_file` path to embedded content.
    pub(super) fn config_file_contents(path: &Path) -> Option<&'static str> {
        const EXPLORER: &str = include_str!("builtins/explorer.toml");
        const AWAITER: &str = include_str!("builtins/awaiter.toml");
        match path.to_str()? {
            "explorer.toml" => Some(EXPLORER),
            "awaiter.toml" => Some(AWAITER),
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "role_tests.rs"]
mod tests;
