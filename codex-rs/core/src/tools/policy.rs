use codex_tools::AgentToolPolicyConfig;
use codex_tools::ToolName;
use codex_tools::ToolSpec;
use wildmatch::WildMatchPattern;

type ToolPattern = WildMatchPattern<'*', '?'>;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ToolAccessPolicy {
    layers: Vec<ToolAccessPolicyLayer>,
    shell_aliases: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct ToolAccessPolicyLayer {
    allow_list: Option<Vec<ToolPattern>>,
    deny_list: Vec<ToolPattern>,
}

impl ToolAccessPolicy {
    pub(crate) fn from_config(config: &Option<AgentToolPolicyConfig>) -> Option<Self> {
        let config = config.as_ref()?;
        let mut layers = Vec::new();
        collect_layers(config, &mut layers);
        Some(Self {
            layers,
            shell_aliases: shell_family_aliases(),
        })
    }

    pub(crate) fn allows(&self, tool_name: &ToolName) -> bool {
        let candidates = comparable_tool_names(&tool_name.display(), &self.shell_aliases);
        self.layers
            .iter()
            .all(|layer| layer.allows_candidates(&candidates))
    }

    pub(crate) fn filter_spec(&self, spec: ToolSpec) -> Option<ToolSpec> {
        match spec {
            ToolSpec::Function(tool) => self
                .allows(&ToolName::plain(tool.name.as_str()))
                .then_some(ToolSpec::Function(tool)),
            ToolSpec::Freeform(tool) => self
                .allows(&ToolName::plain(tool.name.as_str()))
                .then_some(ToolSpec::Freeform(tool)),
            ToolSpec::LocalShell {} => self
                .allows(&ToolName::plain("local_shell"))
                .then_some(ToolSpec::LocalShell {}),
            ToolSpec::Namespace(mut namespace) => {
                let namespace_name = namespace.name.clone();
                namespace.tools.retain(|tool| match tool {
                    codex_tools::ResponsesApiNamespaceTool::Function(tool) => self.allows(
                        &ToolName::namespaced(namespace_name.as_str(), tool.name.as_str()),
                    ),
                });
                (!namespace.tools.is_empty()).then_some(ToolSpec::Namespace(namespace))
            }
            spec => self.allows(&ToolName::plain(spec.name())).then_some(spec),
        }
    }
}

impl ToolAccessPolicyLayer {
    fn allows_candidates(&self, candidates: &[String]) -> bool {
        if self.deny_list.iter().any(|pattern| {
            candidates
                .iter()
                .any(|candidate| pattern.matches(candidate))
        }) {
            return false;
        }

        match &self.allow_list {
            Some(allow_list) => allow_list.iter().any(|pattern| {
                candidates
                    .iter()
                    .any(|candidate| pattern.matches(candidate))
            }),
            None => true,
        }
    }
}

fn collect_layers(config: &AgentToolPolicyConfig, layers: &mut Vec<ToolAccessPolicyLayer>) {
    if let Some(inherited) = &config.inherited {
        collect_layers(inherited, layers);
    }
    layers.push(ToolAccessPolicyLayer {
        allow_list: config.allow_list.clone().map(compile_patterns),
        deny_list: compile_patterns(config.deny_list.clone().unwrap_or_default()),
    });
}

fn compile_patterns(patterns: Vec<String>) -> Vec<ToolPattern> {
    patterns
        .into_iter()
        .map(|pattern| pattern.trim().to_ascii_lowercase())
        .filter(|pattern| !pattern.is_empty())
        .map(|pattern| ToolPattern::new(&pattern))
        .collect()
}

fn comparable_tool_names(tool_name: &str, shell_aliases: &[String]) -> Vec<String> {
    let normalized = tool_name.trim().to_ascii_lowercase();
    if shell_aliases.iter().any(|alias| alias == &normalized) {
        return shell_aliases.to_vec();
    }
    vec![normalized]
}

fn shell_family_aliases() -> Vec<String> {
    vec![
        "exec_command".to_string(),
        "shell".to_string(),
        "container.exec".to_string(),
        "local_shell".to_string(),
        "shell_command".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn deny_beats_allow_for_shell_aliases() {
        let policy = ToolAccessPolicy {
            layers: vec![ToolAccessPolicyLayer {
                allow_list: Some(compile_patterns(vec!["shell*".to_string()])),
                deny_list: compile_patterns(vec!["exec_command".to_string()]),
            }],
            shell_aliases: shell_family_aliases(),
        };

        assert_eq!(policy.allows(&ToolName::plain("shell_command")), false);
        assert_eq!(policy.allows(&ToolName::plain("local_shell")), false);
    }

    #[test]
    fn empty_allow_means_allow_unless_denied() {
        let policy = ToolAccessPolicy {
            layers: vec![ToolAccessPolicyLayer {
                allow_list: None,
                deny_list: compile_patterns(vec!["mcp__secret__*".to_string()]),
            }],
            shell_aliases: shell_family_aliases(),
        };

        assert_eq!(policy.allows(&ToolName::plain("web_search")), true);
        assert_eq!(
            policy.allows(&ToolName::namespaced("mcp__secret__", "read")),
            false
        );
    }

    #[test]
    fn allow_list_filters_hosted_specs() {
        let policy = ToolAccessPolicy {
            layers: vec![ToolAccessPolicyLayer {
                allow_list: Some(compile_patterns(vec!["shell*".to_string()])),
                deny_list: Vec::new(),
            }],
            shell_aliases: shell_family_aliases(),
        };

        assert_eq!(
            policy.filter_spec(ToolSpec::WebSearch {
                external_web_access: None,
                filters: None,
                user_location: None,
                search_context_size: None,
                search_content_types: None,
            }),
            None
        );
    }

    #[test]
    fn inherited_layers_cannot_be_relaxed_by_child_policy() {
        let policy = ToolAccessPolicy::from_config(&Some(AgentToolPolicyConfig {
            allow_list: Some(vec!["*".to_string()]),
            deny_list: None,
            inherited: Some(Box::new(AgentToolPolicyConfig {
                allow_list: Some(vec!["shell*".to_string()]),
                deny_list: Some(vec!["exec_command".to_string()]),
                inherited: None,
            })),
        }))
        .expect("policy should compile");

        assert_eq!(policy.allows(&ToolName::plain("web_search")), false);
        assert_eq!(policy.allows(&ToolName::plain("shell_command")), false);
    }
}
