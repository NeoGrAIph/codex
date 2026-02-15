// FORK COMMIT NEW FILE [SA]: centralized tool-policy normalization and matching.
// Role: keep allow/deny semantics consistent across specs, registry, and runtime.
use std::collections::HashSet;
use wildmatch::WildMatchPattern;

type ToolPolicyPattern = WildMatchPattern<'*', '?'>;

pub(crate) fn normalize_tool_policy_list(list: Option<Vec<String>>) -> Option<Vec<String>> {
    let mut out: Vec<String> = list
        .unwrap_or_default()
        .into_iter()
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .collect();
    out.sort();
    out.dedup();
    (!out.is_empty()).then_some(out)
}

pub(crate) fn build_allow_set(allow_list: Option<&[String]>) -> Option<HashSet<String>> {
    allow_list.and_then(|list| {
        let set: HashSet<String> = list
            .iter()
            .map(|name| name.trim().to_string())
            .filter(|name| !name.is_empty())
            .collect();
        (!set.is_empty()).then_some(set)
    })
}

pub(crate) fn build_deny_set(deny_list: Option<&[String]>) -> HashSet<String> {
    deny_list
        .unwrap_or(&[])
        .iter()
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .collect()
}

pub(crate) fn is_tool_enabled(
    tool_name: &str,
    allow_set: Option<&HashSet<String>>,
    deny_set: &HashSet<String>,
) -> bool {
    // FORK COMMIT OPEN [SA]: allow glob policy entries (`*`, `?`) in tool allow/deny lists.
    // Role: keep per-thread tool policy expressive without changing policy API shape.
    // legacy:
    // let allowed = allow_set.map(|set| set.contains(tool_name)).unwrap_or(true);
    // allowed && !deny_set.contains(tool_name)
    let allowed = allow_set
        .map(|set| matches_any_policy_entry(tool_name, set))
        .unwrap_or(true);
    allowed && !matches_any_policy_entry(tool_name, deny_set)
    // FORK COMMIT CLOSE: glob-aware allow/deny tool matching.
}

fn matches_any_policy_entry(tool_name: &str, policy_set: &HashSet<String>) -> bool {
    policy_set
        .iter()
        .any(|entry| matches_policy_entry(tool_name, entry))
}

fn matches_policy_entry(tool_name: &str, entry: &str) -> bool {
    if entry.contains('*') || entry.contains('?') {
        ToolPolicyPattern::new(entry).matches(tool_name)
    } else {
        entry == tool_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_tool_enabled_supports_literal_allow_and_deny() {
        let allow_set = build_allow_set(Some(&["wait".to_string(), "spawn_agent".to_string()]))
            .expect("allow set");
        let deny_set = build_deny_set(Some(&["wait".to_string()]));

        assert!(!is_tool_enabled("wait", Some(&allow_set), &deny_set));
        assert!(is_tool_enabled("spawn_agent", Some(&allow_set), &deny_set));
        assert!(!is_tool_enabled("close_agent", Some(&allow_set), &deny_set));
    }

    #[test]
    fn is_tool_enabled_supports_glob_allow_patterns() {
        let allow_set =
            build_allow_set(Some(&["mcp*".to_string(), "w?it".to_string()])).expect("allow set");
        let deny_set = build_deny_set(None);

        assert!(is_tool_enabled(
            "mcp__n8n_list",
            Some(&allow_set),
            &deny_set
        ));
        assert!(is_tool_enabled("wait", Some(&allow_set), &deny_set));
        assert!(!is_tool_enabled("spawn_agent", Some(&allow_set), &deny_set));
    }

    #[test]
    fn is_tool_enabled_prefers_deny_over_allow() {
        let allow_set = build_allow_set(Some(&["mcp*".to_string(), "spawn_agent".to_string()]))
            .expect("allow set");
        let deny_set = build_deny_set(Some(&[
            "mcp__danger_*".to_string(),
            "spawn_agent".to_string(),
        ]));

        assert!(!is_tool_enabled(
            "mcp__danger_run",
            Some(&allow_set),
            &deny_set
        ));
        assert!(is_tool_enabled(
            "mcp__safe_list",
            Some(&allow_set),
            &deny_set
        ));
        assert!(!is_tool_enabled("spawn_agent", Some(&allow_set), &deny_set));
    }

    #[test]
    fn is_tool_enabled_is_case_sensitive_for_globs() {
        let allow_set =
            build_allow_set(Some(&["wait".to_string(), "mcp*".to_string()])).expect("allow set");
        let deny_set = build_deny_set(None);

        assert!(is_tool_enabled("wait", Some(&allow_set), &deny_set));
        assert!(!is_tool_enabled("Wait", Some(&allow_set), &deny_set));
        assert!(!is_tool_enabled("MCP__tool", Some(&allow_set), &deny_set));
    }

    #[test]
    fn is_tool_enabled_allows_all_when_allow_list_is_absent() {
        let deny_set = build_deny_set(Some(&["close_*".to_string()]));

        assert!(is_tool_enabled("spawn_agent", None, &deny_set));
        assert!(!is_tool_enabled("close_agent", None, &deny_set));
    }
}
