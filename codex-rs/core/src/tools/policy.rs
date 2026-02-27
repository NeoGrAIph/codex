use wildmatch::WildMatchPattern;

pub(crate) type ToolPolicyPattern = WildMatchPattern<'*', '?'>;

pub(crate) fn normalize_tool_policy_list(list: Option<&[String]>) -> Option<Vec<String>> {
    let Some(list) = list else {
        return None;
    };

    let mut normalized = list
        .iter()
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();

    (!normalized.is_empty()).then_some(normalized)
}

pub(crate) fn build_allow_set(allow_list: Option<&[String]>) -> Option<Vec<ToolPolicyPattern>> {
    normalize_tool_policy_list(allow_list).map(|allow_list| {
        allow_list
            .into_iter()
            .map(|name| ToolPolicyPattern::new_case_insensitive(&name))
            .collect()
    })
}

pub(crate) fn build_deny_set(deny_list: Option<&[String]>) -> Vec<ToolPolicyPattern> {
    normalize_tool_policy_list(deny_list)
        .unwrap_or_default()
        .into_iter()
        .map(|name| ToolPolicyPattern::new_case_insensitive(&name))
        .collect()
}

pub(crate) fn is_tool_enabled(
    tool_name: &str,
    allow_set: Option<&[ToolPolicyPattern]>,
    deny_set: &[ToolPolicyPattern],
) -> bool {
    if deny_set.iter().any(|pattern| pattern.matches(tool_name)) {
        return false;
    }

    allow_set.is_none_or(|allow_set| allow_set.iter().any(|pattern| pattern.matches(tool_name)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn normalize_tool_policy_list_trims_sorts_and_dedups() {
        let list = vec![
            "  mcp__rmcp__echo ".to_string(),
            "mcp__rmcp__*".to_string(),
            "".to_string(),
            "mcp__rmcp__echo".to_string(),
        ];

        assert_eq!(
            normalize_tool_policy_list(Some(&list)),
            Some(vec![
                "mcp__rmcp__*".to_string(),
                "mcp__rmcp__echo".to_string(),
            ])
        );
    }

    #[test]
    fn deny_has_priority_over_allow() {
        let allow_list = vec!["mcp__rmcp__*".to_string()];
        let deny_list = vec!["mcp__rmcp__danger*".to_string()];
        let allow_set = build_allow_set(Some(&allow_list));
        let deny_set = build_deny_set(Some(&deny_list));

        assert!(is_tool_enabled(
            "mcp__rmcp__echo",
            allow_set.as_deref(),
            &deny_set
        ));
        assert!(!is_tool_enabled(
            "mcp__rmcp__danger_exec",
            allow_set.as_deref(),
            &deny_set
        ));
    }

    #[test]
    fn allow_absent_keeps_everything_not_denied() {
        let deny_list = vec!["mcp__rmcp__danger*".to_string()];
        let deny_set = build_deny_set(Some(&deny_list));

        assert!(is_tool_enabled("mcp__rmcp__echo", None, &deny_set));
        assert!(!is_tool_enabled("mcp__rmcp__danger_exec", None, &deny_set));
    }

    #[test]
    fn wildcard_question_mark_matches_single_character() {
        let allow_list = vec!["mcp__rmcp__tool_?".to_string()];
        let allow_set = build_allow_set(Some(&allow_list));

        assert!(is_tool_enabled(
            "mcp__rmcp__tool_a",
            allow_set.as_deref(),
            &[]
        ));
        assert!(!is_tool_enabled(
            "mcp__rmcp__tool_ab",
            allow_set.as_deref(),
            &[]
        ));
    }
}
