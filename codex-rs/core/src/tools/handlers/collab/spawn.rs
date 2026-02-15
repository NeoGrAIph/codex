// FORK COMMIT NEW FILE [SAW]: spawn handler extracted into dedicated module.
// Role: isolate spawn_agent flow to keep collab handler minimal while preserving legacy inline module.
use super::*;
use crate::agent::AgentRole;
use crate::agent::exceeds_thread_spawn_depth_limit;
use crate::agent::next_thread_spawn_depth;
use crate::agent::status::is_final;
use crate::config::Config;
use crate::protocol::SandboxPolicy;
use codex_protocol::ThreadId;
use codex_protocol::openai_models::ModelPreset;
use codex_protocol::openai_models::ReasoningEffort as ReasoningEffortConfig;
use codex_protocol::protocol::ReviewDecision;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

#[derive(Debug, Deserialize)]
// FORK COMMIT OPEN [SA]: extended spawn_agent arguments for template/persona and runtime overrides.
// Role: support role templates while allowing explicit model/reasoning selection per spawn call.
struct SpawnAgentArgs {
    message: Option<String>,
    items: Option<Vec<UserInput>>,
    // FORK COMMIT [SA]: optional working directory override for spawned agent session.
    working_directory: Option<String>,
    agent_type: Option<String>,
    agent_name: Option<String>,
    model: Option<String>,
    reasoning_effort: Option<String>,
    thread_note: Option<String>,
}
// FORK COMMIT CLOSE: extended spawn_agent arguments for template/persona and runtime overrides.

#[derive(Debug, Serialize)]
struct SpawnAgentResult {
    agent_id: String,
}

const SPAWN_STATUS_MESSAGE_PREVIEW_CHARS: usize = 160;

pub async fn handle(
    session: Arc<Session>,
    turn: Arc<TurnContext>,
    call_id: String,
    arguments: String,
) -> Result<ToolOutput, FunctionCallError> {
    let args: SpawnAgentArgs = parse_arguments(&arguments)?;
    // FORK COMMIT OPEN [SA]: optional spawn working-directory override with mandatory approval on change.
    // Role: require explicit user consent before creating a sub-agent in a different directory.
    let requested_working_directory = args
        .working_directory
        .as_deref()
        .map(str::trim)
        .filter(|directory| !directory.is_empty())
        .map(ToString::to_string);
    let spawn_cwd = requested_working_directory
        .as_ref()
        .map(|directory| turn.resolve_path(Some(directory.clone())));
    if let Some(spawn_cwd) = spawn_cwd.as_ref()
        && spawn_cwd != &turn.cwd
    {
        let approval_command = vec![
            "spawn_agent".to_string(),
            "--working-directory".to_string(),
            spawn_cwd.display().to_string(),
        ];
        let approval_reason = Some(format!(
            "spawn_agent requested a different working_directory: {}",
            spawn_cwd.display()
        ));
        let decision = match &turn.session_source {
            SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id, ..
            }) => match session
                .services
                .agent_control
                .request_command_approval_for_thread(
                    *parent_thread_id,
                    call_id.clone(),
                    approval_command.clone(),
                    spawn_cwd.clone(),
                    approval_reason.clone(),
                    None,
                )
                .await
            {
                Ok(decision) => decision,
                Err(err) => {
                    tracing::warn!(
                        "failed to route spawn_agent working_directory approval via parent thread {}: {err}; falling back to current session",
                        parent_thread_id
                    );
                    session
                        .request_command_approval(
                            turn.as_ref(),
                            call_id.clone(),
                            approval_command,
                            spawn_cwd.clone(),
                            approval_reason,
                            None,
                        )
                        .await
                }
            },
            _ => {
                session
                    .request_command_approval(
                        turn.as_ref(),
                        call_id.clone(),
                        approval_command,
                        spawn_cwd.clone(),
                        approval_reason,
                        None,
                    )
                    .await
            }
        };
        if matches!(decision, ReviewDecision::Denied | ReviewDecision::Abort) {
            return Err(FunctionCallError::RespondToModel(
                "spawn_agent in a different working_directory was not approved".to_string(),
            ));
        }
    }
    // FORK COMMIT CLOSE: optional spawn working-directory override with mandatory approval on change.
    // FORK COMMIT OPEN [SA]: normalize role/persona selectors and explicit overrides before config build.
    // Role: apply deterministic precedence across built-in roles, templates, and call-level overrides.
    let agent_type_raw = args.agent_type;
    let agent_type = agent_type_raw.as_deref().unwrap_or("default");
    let agent_type = agent_type.trim().to_ascii_lowercase();
    let built_in_role = match agent_type.as_str() {
        "default" => Some(AgentRole::Default),
        "orchestrator" => Some(AgentRole::Orchestrator),
        "worker" => Some(AgentRole::Worker),
        "explorer" => Some(AgentRole::Explorer),
        _ => None,
    };
    let agent_name = args
        .agent_name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_ascii_lowercase);
    if agent_type == "default" && agent_name.is_some() {
        return Err(FunctionCallError::RespondToModel(
            "agent_name requires a non-default agent_type".to_string(),
        ));
    }
    let model_override = args
        .model
        .as_deref()
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .map(ToString::to_string);
    // FORK COMMIT OPEN [SA]: optional spawn-time thread note.
    // Role: allow callers to assign runtime operational label without mutating stable thread name.
    let requested_thread_note = crate::util::normalize_thread_note(args.thread_note.as_deref());
    // FORK COMMIT CLOSE: optional spawn-time thread note.
    // FORK COMMIT OPEN [SA]: preserve explicit role/persona metadata in thread source.
    // Role: keep SAW/telemetry stable for implicit spawns while surfacing explicit selections.
    let agent_type_for_source = if agent_type_raw.is_some() {
        Some(agent_type.clone())
    } else {
        None
    };
    let agent_name_for_source = if args.agent_name.is_some() {
        agent_name.clone()
    } else {
        None
    };
    let mut allow_list_for_source: Option<Vec<String>> = None;
    let mut deny_list_for_source: Option<Vec<String>> = None;
    let mut selected_agent_name_for_note: Option<String> = None;
    let mut selected_agent_description_for_note: Option<String> = None;
    // FORK COMMIT CLOSE: preserve explicit role/persona metadata in thread source.
    let input_items = parse_collab_input(args.message, args.items)?;
    let prompt = input_preview(&input_items);
    let session_source = turn.session_source.clone();
    let child_depth = next_thread_spawn_depth(&session_source);
    if exceeds_thread_spawn_depth_limit(child_depth) {
        return Err(FunctionCallError::RespondToModel(
            "Agent depth limit reached. Solve the task yourself.".to_string(),
        ));
    }

    session
        .send_event(
            &turn,
            CollabAgentSpawnBeginEvent {
                call_id: call_id.clone(),
                sender_thread_id: session.conversation_id,
                prompt: prompt.clone(),
            }
            .into(),
        )
        .await;

    let mut config = build_agent_spawn_config(
        &session.get_base_instructions().await,
        turn.as_ref(),
        child_depth,
        spawn_cwd,
    )?;

    // Keep current built-in role behavior as a baseline; templates and explicit overrides can refine.
    if let Some(role) = built_in_role {
        role.apply_to_config(&mut config)
            .map_err(FunctionCallError::RespondToModel)?;
    }

    // FORK COMMIT OPEN [SA]: template-backed role/persona configuration.
    // Role: allow `spawn_agent.agent_type` to map to templates with optional `agent_name`.
    let parsed_template = if agent_type == "default" {
        None
    } else {
        match crate::agent::role_templates::get_parsed(&agent_type) {
            Ok(parsed) => Some(parsed),
            Err(err) => {
                if built_in_role.is_some() && agent_name.is_none() {
                    None
                } else {
                    return Err(FunctionCallError::RespondToModel(err));
                }
            }
        }
    };

    if let Some(parsed) = parsed_template {
        let (selected_instructions, selected_agent_name) = match agent_name.as_deref() {
            Some(name) => parsed
                .named_instructions
                .get(name)
                .cloned()
                .map(|instructions| (instructions, Some(name)))
                .ok_or_else(|| {
                    FunctionCallError::RespondToModel(format!(
                        "unknown agent_name {name:?} for agent_type {agent_type:?}"
                    ))
                })?,
            None => {
                let default = parsed.default_instructions.trim();
                if !default.is_empty() {
                    (parsed.default_instructions.clone(), None)
                } else if parsed.named_instructions.len() == 1 {
                    let (name, instructions) = parsed
                        .named_instructions
                        .iter()
                        .next()
                        .map(|(name, instructions)| (name.as_str(), instructions.clone()))
                        .unwrap_or(("", String::new()));
                    (instructions, Some(name))
                } else {
                    return Err(FunctionCallError::RespondToModel(format!(
                        "agent_type {agent_type:?} requires agent_name selection"
                    )));
                }
            }
        };
        config.base_instructions = Some(selected_instructions);
        allow_list_for_source = parsed.meta.allow_list.clone();
        deny_list_for_source = parsed.meta.deny_list.clone();
        // FORK COMMIT [SA]: apply template read_only contract to spawned thread sandbox policy.
        if parsed.meta.read_only {
            config
                .sandbox_policy
                .set(SandboxPolicy::new_read_only_policy())
                .map_err(|err| {
                    FunctionCallError::RespondToModel(format!("sandbox_policy is invalid: {err}"))
                })?;
        }

        let selected_agent_meta = selected_agent_name.and_then(|name| {
            parsed
                .meta
                .agent_names
                .iter()
                .find(|agent| agent.name == name)
        });
        selected_agent_name_for_note = selected_agent_name.map(ToString::to_string);
        selected_agent_description_for_note = selected_agent_meta
            .and_then(|agent| agent.description.as_deref())
            .map(str::trim)
            .filter(|description| !description.is_empty())
            .map(ToString::to_string);

        if model_override.is_none()
            && let Some(model) = selected_agent_meta
                .and_then(|agent| agent.model.as_ref())
                .or(parsed.meta.model.as_ref())
        {
            config.model = Some(model.clone());
        }
        if args.reasoning_effort.is_none()
            && let Some(effort) = selected_agent_meta
                .and_then(|agent| agent.reasoning_effort)
                .or(parsed.meta.reasoning_effort)
        {
            config.model_reasoning_effort = Some(effort);
        }
    }
    // FORK COMMIT CLOSE: template-backed role/persona configuration.

    if let Some(model) = model_override {
        config.model = Some(model);
    }
    if let Some(effort) = args.reasoning_effort {
        let model = config.model.as_deref().ok_or_else(|| {
            FunctionCallError::Fatal("spawn_agent config missing model".to_string())
        })?;
        let presets = session
            .services
            .models_manager
            .try_list_models(&config)
            .map_err(|_| {
                FunctionCallError::RespondToModel(
                    "Models are being updated; try spawn_agent again in a moment.".to_string(),
                )
            })?;
        let preset = presets
            .iter()
            .find(|preset| preset.model == model)
            .ok_or_else(|| {
                let available = available_models_csv(&presets);
                FunctionCallError::RespondToModel(format!(
                    "unknown model {model:?} for spawn_agent. Available models: {available}"
                ))
            })?;
        let effort = parse_reasoning_effort_config(&effort).ok_or_else(|| {
            let supported = supported_reasoning_efforts_csv(preset);
            FunctionCallError::RespondToModel(format!(
                "reasoning_effort {effort:?} is not supported for model {model:?}. Supported efforts: {supported}"
            ))
        })?;
        config.model_reasoning_effort = Some(effort);
    }
    // FORK COMMIT OPEN [SA]: strict model/effort validation for spawned agents.
    // Role: reject unknown model slugs and unsupported reasoning levels before spawn.
    validate_spawn_model_selection(session.as_ref(), &config)?;
    // FORK COMMIT CLOSE: strict model/effort validation for spawned agents.
    // FORK COMMIT CLOSE: normalize role/persona selectors and explicit overrides before config build.

    // FORK COMMIT OPEN [SA]: encode role/type and policy metadata into spawn source.
    // Role: propagate runtime contract to child turns for tool filtering and ownership checks.
    let role_label = spawn_role_label(
        agent_type_for_source.as_deref(),
        agent_name_for_source.as_deref(),
    );
    let thread_note = requested_thread_note.or_else(|| {
        default_spawn_thread_note(
            &agent_type,
            selected_agent_name_for_note.as_deref(),
            selected_agent_description_for_note.as_deref(),
        )
    });
    let result = session
        .services
        .agent_control
        .spawn_agent(
            config,
            input_items,
            Some(thread_spawn_source(
                session.conversation_id,
                child_depth,
                agent_type_for_source,
                agent_name_for_source,
                allow_list_for_source,
                deny_list_for_source,
            )),
        )
        .await
        .map_err(collab_spawn_error);
    // FORK COMMIT CLOSE: spawn source metadata handoff.
    let (new_thread_id, status) = match &result {
        Ok(thread_id) => (
            Some(*thread_id),
            session.services.agent_control.get_status(*thread_id).await,
        ),
        Err(_) => (None, AgentStatus::NotFound),
    };
    session
        .send_event(
            &turn,
            CollabAgentSpawnEndEvent {
                call_id: call_id.clone(),
                sender_thread_id: session.conversation_id,
                new_thread_id,
                prompt,
                status,
            }
            .into(),
        )
        .await;
    let new_thread_id = result?;
    if let Some(thread_note) = thread_note
        && let Err(err) = session
            .services
            .agent_control
            .set_thread_note(new_thread_id, Some(thread_note))
            .await
    {
        tracing::warn!("failed to set spawned thread note for {new_thread_id}: {err}");
        let message = format!(
            "agent {role_label} ({new_thread_id}) spawned, but thread_note was not applied: {err}"
        );
        session.notify_background_event(&turn, message, false).await;
    }

    // FORK COMMIT OPEN [SA]: spawn background watcher for sub-agent lifecycle updates.
    // Role: emit concise progress notifications to UI status line without polluting chat history.
    spawn_agent_status_watcher(session.clone(), turn.clone(), new_thread_id, role_label);
    // FORK COMMIT CLOSE: spawn background watcher for sub-agent lifecycle updates.

    let content = serde_json::to_string(&SpawnAgentResult {
        agent_id: new_thread_id.to_string(),
    })
    .map_err(|err| {
        FunctionCallError::Fatal(format!("failed to serialize spawn_agent result: {err}"))
    })?;

    Ok(ToolOutput::Function {
        body: FunctionCallOutputBody::Text(content),
        success: Some(true),
    })
}

// FORK COMMIT OPEN [SA]: spawn-time model/effort contract validation.
// Role: ensure spawned sessions use a model known to the current provider and valid reasoning effort.
fn validate_spawn_model_selection(
    session: &Session,
    config: &Config,
) -> Result<(), FunctionCallError> {
    let model = config
        .model
        .as_deref()
        .ok_or_else(|| FunctionCallError::Fatal("spawn_agent config missing model".to_string()))?;
    let presets = session
        .services
        .models_manager
        .try_list_models(config)
        .map_err(|_| {
            FunctionCallError::RespondToModel(
                "Models are being updated; try spawn_agent again in a moment.".to_string(),
            )
        })?;
    let preset = presets
        .iter()
        .find(|preset| preset.model == model)
        .ok_or_else(|| {
            let available = available_models_csv(&presets);
            FunctionCallError::RespondToModel(format!(
                "unknown model {model:?} for spawn_agent. Available models: {available}"
            ))
        })?;

    if let Some(effort) = config.model_reasoning_effort
        && effort != ReasoningEffortConfig::None
        && !preset
            .supported_reasoning_efforts
            .iter()
            .any(|supported| supported.effort == effort)
    {
        let supported = supported_reasoning_efforts_csv(preset);
        return Err(FunctionCallError::RespondToModel(format!(
            "reasoning_effort {effort:?} is not supported for model {model:?}. Supported efforts: {supported}"
        )));
    }

    Ok(())
}

fn available_models_csv(presets: &[ModelPreset]) -> String {
    let models = presets
        .iter()
        .map(|preset| preset.model.as_str())
        .collect::<Vec<_>>();
    if models.is_empty() {
        "<none>".to_string()
    } else {
        models.join(", ")
    }
}

fn supported_reasoning_efforts_csv(preset: &ModelPreset) -> String {
    let mut efforts = vec!["none".to_string()];
    efforts.extend(
        preset
            .supported_reasoning_efforts
            .iter()
            .map(|supported| supported.effort.to_string())
            .filter(|effort| effort != "none"),
    );
    efforts.join(", ")
}

fn parse_reasoning_effort_config(effort: &str) -> Option<ReasoningEffortConfig> {
    match effort.trim().to_ascii_lowercase().as_str() {
        "none" => Some(ReasoningEffortConfig::None),
        "minimal" => Some(ReasoningEffortConfig::Minimal),
        "low" => Some(ReasoningEffortConfig::Low),
        "medium" => Some(ReasoningEffortConfig::Medium),
        "high" => Some(ReasoningEffortConfig::High),
        "xhigh" => Some(ReasoningEffortConfig::XHigh),
        _ => None,
    }
}

// FORK COMMIT OPEN [SA]: default spawn-time thread note composer.
// Role: auto-label child threads with role/persona metadata when caller omits thread_note.
fn default_spawn_thread_note(
    agent_type: &str,
    agent_name: Option<&str>,
    agent_description: Option<&str>,
) -> Option<String> {
    let agent_type = agent_type.trim();
    if agent_type.is_empty() {
        return None;
    }

    let mut parts = vec![format!("agent_type={agent_type}")];
    if let Some(agent_name) = agent_name.map(str::trim).filter(|value| !value.is_empty()) {
        parts.push(format!("agent_name={agent_name}"));
    }
    if let Some(agent_description) = agent_description
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        parts.push(format!("agent_description={agent_description}"));
    }
    crate::util::normalize_thread_note(Some(&parts.join("; ")))
}
// FORK COMMIT CLOSE: default spawn-time thread note composer.

// FORK COMMIT OPEN [SA]: background status observer helpers for spawned agents.
// Role: keep spawn-agent lifecycle surfaced as concise background UI events.
fn spawn_role_label(agent_type: Option<&str>, agent_name: Option<&str>) -> String {
    let agent_type = agent_type
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("default");
    let agent_name = agent_name.map(str::trim).filter(|value| !value.is_empty());
    match agent_name {
        Some(agent_name) => format!("{agent_type}/{agent_name}"),
        None => agent_type.to_string(),
    }
}

fn spawn_agent_status_watcher(
    session: Arc<Session>,
    turn: Arc<TurnContext>,
    agent_id: ThreadId,
    role: String,
) {
    tokio::spawn(async move {
        let mut status_rx = match session
            .services
            .agent_control
            .subscribe_status(agent_id)
            .await
        {
            Ok(status_rx) => status_rx,
            Err(err) => {
                tracing::debug!("failed to subscribe to spawned agent status ({agent_id}): {err}");
                return;
            }
        };

        let started_at = Instant::now();
        let mut last_reported: Option<AgentStatus> = None;
        loop {
            let status = status_rx.borrow().clone();
            if last_reported.as_ref() != Some(&status) {
                let final_status = is_final(&status);
                let message =
                    format_spawn_status_message(&role, agent_id, started_at.elapsed(), &status);
                session
                    .notify_background_event(&turn, message, final_status)
                    .await;
                last_reported = Some(status.clone());
            }

            if status_rx.changed().await.is_err() {
                let latest = session.services.agent_control.get_status(agent_id).await;
                if last_reported.as_ref() != Some(&latest) {
                    let final_status = is_final(&latest);
                    let message =
                        format_spawn_status_message(&role, agent_id, started_at.elapsed(), &latest);
                    session
                        .notify_background_event(&turn, message, final_status)
                        .await;
                }
                break;
            }
        }
    });
}

fn format_spawn_status_message(
    role: &str,
    agent_id: ThreadId,
    elapsed: Duration,
    status: &AgentStatus,
) -> String {
    let elapsed = format_elapsed_compact(elapsed);
    match status {
        AgentStatus::PendingInit => format!("agent {role} ({agent_id}) initializing ({elapsed})"),
        AgentStatus::Running => format!("agent {role} ({agent_id}) running ({elapsed})"),
        AgentStatus::Completed(message) => {
            let preview = message
                .as_deref()
                .map(|message| {
                    sanitize_preview_one_line(message, SPAWN_STATUS_MESSAGE_PREVIEW_CHARS)
                })
                .filter(|message| !message.is_empty())
                .unwrap_or_else(|| "no message".to_string());
            format!("agent {role} ({agent_id}) completed in {elapsed}: {preview}")
        }
        AgentStatus::Errored(reason) => {
            let preview = sanitize_preview_one_line(reason, SPAWN_STATUS_MESSAGE_PREVIEW_CHARS);
            format!("agent {role} ({agent_id}) errored in {elapsed}: {preview}")
        }
        AgentStatus::Shutdown => format!("agent {role} ({agent_id}) stopped in {elapsed}"),
        AgentStatus::NotFound => format!("agent {role} ({agent_id}) not found in {elapsed}"),
    }
}

fn format_elapsed_compact(elapsed: Duration) -> String {
    let total_seconds = elapsed.as_secs();
    if total_seconds < 60 {
        return format!("{total_seconds}s");
    }
    if total_seconds < 3600 {
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        return format!("{minutes}m {seconds:02}s");
    }
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    format!("{hours}h {minutes:02}m {seconds:02}s")
}

fn sanitize_preview_one_line(text: &str, max_chars: usize) -> String {
    let mut normalized = String::new();
    let mut last_was_space = true;
    for ch in text.chars() {
        let ch = if ch.is_control() { ' ' } else { ch };
        if ch.is_whitespace() {
            if !last_was_space {
                normalized.push(' ');
            }
            last_was_space = true;
            continue;
        }
        normalized.push(ch);
        last_was_space = false;
    }
    let normalized = normalized.trim();
    if normalized.is_empty() {
        return String::new();
    }

    if normalized.chars().count() <= max_chars {
        return normalized.to_string();
    }

    let keep = max_chars.saturating_sub(1);
    let mut out: String = normalized.chars().take(keep).collect();
    out.push('…');
    out
}
// FORK COMMIT CLOSE: background status observer helpers for spawned agents.
// FORK COMMIT CLOSE: spawn-time model/effort contract validation.
// FORK COMMIT CLOSE: spawn handler extracted into dedicated module.

#[cfg(test)]
mod tests {
    use super::*;
    use codex_protocol::ThreadId;
    use pretty_assertions::assert_eq;

    #[test]
    fn spawn_role_label_uses_default_and_optional_name() {
        assert_eq!(spawn_role_label(None, None), "default".to_string());
        assert_eq!(
            spawn_role_label(Some("worker"), Some("deep")),
            "worker/deep".to_string()
        );
        assert_eq!(
            spawn_role_label(Some("explorer"), Some("   ")),
            "explorer".to_string()
        );
    }

    #[test]
    fn sanitize_preview_one_line_normalizes_and_truncates() {
        let source = " one\n\n two\tthree\r\n";
        assert_eq!(
            sanitize_preview_one_line(source, SPAWN_STATUS_MESSAGE_PREVIEW_CHARS),
            "one two three".to_string()
        );

        assert_eq!(sanitize_preview_one_line("abcdef", 4), "abc…".to_string());
        assert_eq!(sanitize_preview_one_line("   \n\t", 10), String::new());
    }

    #[test]
    fn format_spawn_status_message_uses_required_completed_format() {
        let agent_id = ThreadId::new();
        let message = format_spawn_status_message(
            "worker/deep",
            agent_id,
            Duration::from_secs(65),
            &AgentStatus::Completed(Some("done\nwith\nresult".to_string())),
        );
        assert_eq!(
            message,
            format!("agent worker/deep ({agent_id}) completed in 1m 05s: done with result")
        );
    }

    #[test]
    fn format_spawn_status_message_handles_non_completed_finals() {
        let agent_id = ThreadId::new();
        let errored = format_spawn_status_message(
            "default",
            agent_id,
            Duration::from_secs(2),
            &AgentStatus::Errored("boom\nfail".to_string()),
        );
        assert_eq!(
            errored,
            format!("agent default ({agent_id}) errored in 2s: boom fail")
        );

        let shutdown = format_spawn_status_message(
            "default",
            agent_id,
            Duration::from_secs(2),
            &AgentStatus::Shutdown,
        );
        assert_eq!(
            shutdown,
            format!("agent default ({agent_id}) stopped in 2s")
        );
    }

    #[test]
    fn default_spawn_thread_note_contains_agent_metadata() {
        assert_eq!(
            default_spawn_thread_note("worker", None, None),
            Some("agent_type=worker".to_string())
        );
        assert_eq!(
            default_spawn_thread_note("worker", Some("implementer"), Some("Fast fixes")),
            Some(
                "agent_type=worker; agent_name=implementer; agent_description=Fast fixes"
                    .to_string()
            )
        );
    }
}
