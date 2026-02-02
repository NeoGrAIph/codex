use std::collections::HashSet;

use wildmatch::WildMatchPattern;

#[derive(Debug, Clone)]
pub(crate) struct ToolAllowlist {
    matchers: Vec<WildMatchPattern<'*', '?'>>,
}

impl ToolAllowlist {
    pub(crate) fn from_patterns(patterns: &[String]) -> Self {
        let mut seen = HashSet::new();
        let mut matchers = Vec::new();
        for pattern in patterns {
            let trimmed = pattern.trim();
            if trimmed.is_empty() {
                continue;
            }
            let key = trimmed.to_ascii_lowercase();
            if !seen.insert(key) {
                continue;
            }
            matchers.push(WildMatchPattern::new_case_insensitive(trimmed));
        }
        Self { matchers }
    }

    pub(crate) fn allows(&self, tool_name: &str) -> bool {
        self.matchers
            .iter()
            .any(|pattern| pattern.matches(tool_name))
    }
}
