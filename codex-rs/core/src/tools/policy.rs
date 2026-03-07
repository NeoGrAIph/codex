use crate::client_common::tools::ToolSpec;
use codex_protocol::openai_models::ConfigShellToolType;
use wildmatch::WildMatchPattern;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ToolAccessPolicy {
    allow_patterns: Option<Vec<String>>,
    deny_patterns: Vec<String>,
    shell_alias_target: Option<&'static str>,
}

impl ToolAccessPolicy {
    pub(crate) fn from_lists(
        shell_type: ConfigShellToolType,
        allow_list: Option<&[String]>,
        deny_list: Option<&[String]>,
    ) -> Option<Self> {
        if allow_list.is_none() && deny_list.is_none() {
            return None;
        }

        Some(Self {
            allow_patterns: allow_list.map(ToOwned::to_owned),
            deny_patterns: deny_list.map_or_else(Vec::new, ToOwned::to_owned),
            shell_alias_target: canonical_shell_tool_name(shell_type),
        })
    }

    pub(crate) fn allows_spec(&self, spec: &ToolSpec) -> bool {
        self.allows_tool_name(spec.name())
    }

    pub(crate) fn allows_tool_name(&self, tool_name: &str) -> bool {
        if self.matches_any_pattern(&self.deny_patterns, tool_name) {
            return false;
        }

        match &self.allow_patterns {
            Some(patterns) => self.matches_any_pattern(patterns, tool_name),
            None => true,
        }
    }

    fn matches_any_pattern(&self, patterns: &[String], tool_name: &str) -> bool {
        patterns.iter().any(|pattern| {
            matches_pattern(pattern, tool_name)
                || self
                    .canonical_tool_name(tool_name)
                    .is_some_and(|canonical| matches_pattern(pattern, canonical))
        })
    }

    fn canonical_tool_name<'a>(&'a self, tool_name: &'a str) -> Option<&'a str> {
        match tool_name {
            "shell" | "container.exec" | "local_shell" | "shell_command" | "write_stdin" => self
                .shell_alias_target
                .filter(|canonical| *canonical != tool_name),
            _ => None,
        }
    }
}

fn canonical_shell_tool_name(shell_type: ConfigShellToolType) -> Option<&'static str> {
    match shell_type {
        ConfigShellToolType::Default => Some("shell"),
        ConfigShellToolType::Local => Some("local_shell"),
        ConfigShellToolType::UnifiedExec => Some("exec_command"),
        ConfigShellToolType::Disabled => None,
        ConfigShellToolType::ShellCommand => Some("shell_command"),
    }
}

fn matches_pattern(pattern: &str, tool_name: &str) -> bool {
    WildMatchPattern::<'*', '?'>::new_case_insensitive(pattern).matches(tool_name)
}

#[cfg(test)]
mod tests {
    use super::ToolAccessPolicy;
    use codex_protocol::openai_models::ConfigShellToolType;

    fn policy(
        shell_type: ConfigShellToolType,
        allow: Option<&[&str]>,
        deny: Option<&[&str]>,
    ) -> ToolAccessPolicy {
        let allow = allow.map(|patterns| {
            patterns
                .iter()
                .map(std::string::ToString::to_string)
                .collect::<Vec<_>>()
        });
        let deny = deny.map(|patterns| {
            patterns
                .iter()
                .map(std::string::ToString::to_string)
                .collect::<Vec<_>>()
        });

        ToolAccessPolicy::from_lists(shell_type, allow.as_deref(), deny.as_deref())
            .expect("policy should exist")
    }

    #[test]
    fn deny_list_wins_over_allow_list() {
        let policy = policy(
            ConfigShellToolType::ShellCommand,
            Some(&["shell_*"]),
            Some(&["shell_command"]),
        );

        assert!(!policy.allows_tool_name("shell_command"));
    }

    #[test]
    fn allow_list_absent_allows_tools_not_denied() {
        let policy = policy(ConfigShellToolType::ShellCommand, None, Some(&["exec_*"]));

        assert!(policy.allows_tool_name("shell_command"));
        assert!(!policy.allows_tool_name("exec_command"));
    }

    #[test]
    fn wildcard_matching_is_case_insensitive() {
        let policy = policy(
            ConfigShellToolType::ShellCommand,
            Some(&["MCP__SERVER__TO?L"]),
            None,
        );

        assert!(policy.allows_tool_name("mcp__server__tool"));
        assert!(!policy.allows_tool_name("mcp__server__tool_long"));
    }

    #[test]
    fn shell_aliases_match_current_shell_capability() {
        let policy = policy(
            ConfigShellToolType::UnifiedExec,
            Some(&["exec_command"]),
            None,
        );

        assert!(policy.allows_tool_name("shell"));
        assert!(policy.allows_tool_name("container.exec"));
        assert!(policy.allows_tool_name("write_stdin"));
        assert!(!policy.allows_tool_name("view_image"));
    }

    #[test]
    fn unified_exec_write_stdin_respects_exec_command_deny() {
        let policy = policy(
            ConfigShellToolType::UnifiedExec,
            None,
            Some(&["exec_command"]),
        );

        assert!(!policy.allows_tool_name("exec_command"));
        assert!(!policy.allows_tool_name("write_stdin"));
    }

    #[test]
    fn shell_aliases_respect_deny_patterns_for_canonical_name() {
        let policy = policy(
            ConfigShellToolType::ShellCommand,
            None,
            Some(&["shell_command"]),
        );

        assert!(!policy.allows_tool_name("shell"));
        assert!(!policy.allows_tool_name("local_shell"));
        assert!(!policy.allows_tool_name("container.exec"));
    }
}
