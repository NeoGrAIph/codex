use codex_tools::JsonSchema;
use codex_tools::ResponsesApiTool;
use codex_tools::ToolEnvironmentMode;
use codex_tools::ToolSpec;
use std::collections::BTreeMap;

pub fn create_run_skill_script_tool(environment_mode: ToolEnvironmentMode) -> ToolSpec {
    let mut properties = BTreeMap::from([
        (
            "skill".to_string(),
            JsonSchema::string(Some(
                "Enabled skill name, or path to its SKILL.md file.".to_string(),
            )),
        ),
        (
            "script".to_string(),
            JsonSchema::string(Some(
                "Relative path under the skill's scripts directory.".to_string(),
            )),
        ),
        (
            "args".to_string(),
            JsonSchema::array(
                JsonSchema::string(/*description*/ None),
                Some("Arguments passed to the skill script.".to_string()),
            ),
        ),
        (
            "yield_time_ms".to_string(),
            JsonSchema::integer(Some(
                "Wait before yielding output. Defaults to exec_command behavior.".to_string(),
            )),
        ),
        (
            "max_output_tokens".to_string(),
            JsonSchema::integer(Some(
                "Output token budget. Defaults to exec_command behavior.".to_string(),
            )),
        ),
    ]);

    if matches!(environment_mode, ToolEnvironmentMode::Multiple) {
        properties.insert(
            "environment_id".to_string(),
            JsonSchema::string(Some(
                "Target turn environment. First iteration supports only the primary local environment; omit this unless you are selecting that same primary environment.".to_string(),
            )),
        );
    }

    ToolSpec::Function(ResponsesApiTool {
        name: "run_skill_script".to_string(),
        description:
            "Runs a helper script from an enabled skill's scripts directory through unified exec on the primary local environment, preserving exec approval, sandbox, Bash hooks, and telemetry behavior."
                .to_string(),
        strict: false,
        defer_loading: None,
        parameters: JsonSchema::object(
            properties,
            Some(vec!["skill".to_string(), "script".to_string()]),
            Some(false.into()),
        ),
        output_schema: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn function_properties(tool: ToolSpec) -> BTreeMap<String, JsonSchema> {
        let ToolSpec::Function(tool) = tool else {
            panic!("expected function tool");
        };
        tool.parameters
            .properties
            .expect("function tool parameters should have properties")
    }

    #[test]
    fn environment_id_is_only_present_for_multiple_environments() {
        let single = function_properties(create_run_skill_script_tool(ToolEnvironmentMode::Single));
        assert!(!single.contains_key("environment_id"));

        let multiple =
            function_properties(create_run_skill_script_tool(ToolEnvironmentMode::Multiple));
        assert!(multiple.contains_key("environment_id"));
    }
}
