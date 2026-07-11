use super::AgentRoleConfig;
use super::deserialize_config_toml_with_base;
use codex_config::ConfigLayerStack;
use codex_config::ConfigLayerStackOrdering;
use codex_config::config_toml::AgentRoleToml;
use codex_config::config_toml::AgentsToml;
use codex_config::config_toml::ConfigToml;
use codex_exec_server::ExecutorFileSystem;
use codex_utils_absolute_path::AbsolutePathBuf;
use codex_utils_absolute_path::AbsolutePathBufGuard;
use codex_utils_path_uri::PathUri;
use futures::StreamExt;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::io::ErrorKind;
use std::path::Path;
use std::path::PathBuf;
use toml::Value as TomlValue;

const MAX_AGENT_ROLE_DEVELOPER_INSTRUCTIONS_BYTES: usize = 32 * 1_024;
const MAX_AGENT_ROLE_FILE_BYTES: u64 = 64 * 1_024;
const MAX_DISCOVERED_AGENT_ROLE_FILES: usize = 256;

#[derive(Debug, Clone, PartialEq)]
#[doc(hidden)]
pub struct MaterializedAgentRoleLayer {
    /// Validated role config, or an empty table for metadata-only role declarations.
    pub(crate) config: TomlValue,
    pub(crate) base_dir: PathBuf,
    pub(crate) catalog_priority: usize,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct LoadedAgentRoles {
    pub(crate) declarations: BTreeMap<String, AgentRoleConfig>,
    pub(crate) layers: BTreeMap<String, MaterializedAgentRoleLayer>,
}

#[derive(Debug, Clone)]
struct LoadedAgentRole {
    declaration: AgentRoleConfig,
    layer: Option<MaterializedAgentRoleLayer>,
    catalog_priority: usize,
}

#[derive(Debug)]
struct MaterializedAgentRoleFile {
    resolved: ResolvedAgentRoleFile,
    layer: MaterializedAgentRoleLayer,
}

pub(crate) async fn load_agent_roles(
    fs: &dyn ExecutorFileSystem,
    cfg: &ConfigToml,
    config_layer_stack: &ConfigLayerStack,
    startup_warnings: &mut Vec<String>,
) -> std::io::Result<LoadedAgentRoles> {
    let layers = config_layer_stack.get_layers(
        ConfigLayerStackOrdering::LowestPrecedenceFirst,
        /*include_disabled*/ false,
    );
    if layers.is_empty() {
        return load_agent_roles_without_layers(fs, cfg).await;
    }

    let mut roles: BTreeMap<String, LoadedAgentRole> = BTreeMap::new();
    for (catalog_priority, layer) in layers.into_iter().enumerate() {
        let mut layer_roles: BTreeMap<String, LoadedAgentRole> = BTreeMap::new();
        let mut declared_role_files = BTreeSet::new();
        let config_folder = layer.config_folder();
        let agents_toml = match agents_toml_from_layer(&layer.config, config_folder.as_deref()) {
            Ok(agents_toml) => agents_toml,
            Err(err) => {
                push_agent_role_warning(startup_warnings, err);
                None
            }
        };
        if let Some(agents_toml) = agents_toml {
            for (declared_role_name, role_toml) in &agents_toml.roles {
                let (role_name, role) =
                    match read_declared_role(fs, declared_role_name, role_toml).await {
                        Ok(role) => role,
                        Err(err) => {
                            push_agent_role_warning(startup_warnings, err);
                            continue;
                        }
                    };
                if let Some(config_file) = role.declaration.config_file.clone() {
                    declared_role_files.insert(config_file);
                }
                if layer_roles.contains_key(&role_name) {
                    push_agent_role_warning(
                        startup_warnings,
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            format!(
                                "duplicate agent role name `{role_name}` declared in the same config layer"
                            ),
                        ),
                    );
                    continue;
                }
                layer_roles.insert(role_name, role);
            }
        }

        if let Some(config_folder) = layer.config_folder() {
            for (role_name, role) in discover_agent_roles_in_dir(
                fs,
                &config_folder.join("agents"),
                &declared_role_files,
                startup_warnings,
            )
            .await?
            {
                if layer_roles.contains_key(&role_name) {
                    push_agent_role_warning(
                        startup_warnings,
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            format!(
                                "duplicate agent role name `{role_name}` declared in the same config layer"
                            ),
                        ),
                    );
                    continue;
                }
                layer_roles.insert(role_name, role);
            }
        }

        for (role_name, role) in layer_roles {
            let mut merged_role = role;
            merged_role.catalog_priority = catalog_priority;
            if let Some(existing_role) = roles.get(&role_name) {
                merge_missing_role_fields(&mut merged_role.declaration, &existing_role.declaration);
                if merged_role.layer.is_none() {
                    merged_role.layer.clone_from(&existing_role.layer);
                }
            }
            if let Err(err) = validate_required_agent_role_description(
                &role_name,
                merged_role.declaration.description.as_deref(),
            ) {
                push_agent_role_warning(startup_warnings, err);
                continue;
            }
            roles.insert(role_name, merged_role);
        }
    }

    Ok(finish_loaded_agent_roles(roles))
}

fn push_agent_role_warning(startup_warnings: &mut Vec<String>, err: std::io::Error) {
    let message = format!("Ignoring malformed agent role definition: {err}");
    tracing::warn!("{message}");
    startup_warnings.push(message);
}

async fn load_agent_roles_without_layers(
    fs: &dyn ExecutorFileSystem,
    cfg: &ConfigToml,
) -> std::io::Result<LoadedAgentRoles> {
    let mut roles = BTreeMap::new();
    if let Some(agents_toml) = cfg.agents.as_ref() {
        for (declared_role_name, role_toml) in &agents_toml.roles {
            let (role_name, role) = read_declared_role(fs, declared_role_name, role_toml).await?;
            validate_required_agent_role_description(
                &role_name,
                role.declaration.description.as_deref(),
            )?;

            if roles.insert(role_name.clone(), role).is_some() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("duplicate agent role name `{role_name}` declared in config"),
                ));
            }
        }
    }

    Ok(finish_loaded_agent_roles(roles))
}

fn finish_loaded_agent_roles(roles: BTreeMap<String, LoadedAgentRole>) -> LoadedAgentRoles {
    let mut loaded = LoadedAgentRoles::default();
    let mut roles = roles.into_iter().collect::<Vec<_>>();
    roles.sort_by(|(left_name, left), (right_name, right)| {
        right
            .catalog_priority
            .cmp(&left.catalog_priority)
            .then_with(|| left_name.cmp(right_name))
    });
    for (role_name, mut role) in roles {
        if let (Some(description), Some(layer)) =
            (role.declaration.description.as_mut(), role.layer.as_ref())
        {
            let note = agent_role_locked_settings_note(&layer.config);
            if !description.ends_with(&note) {
                description.push_str(&note);
            }
        }
        loaded
            .declarations
            .insert(role_name.clone(), role.declaration);
        let mut layer = role.layer.unwrap_or_else(|| MaterializedAgentRoleLayer {
            config: TomlValue::Table(Default::default()),
            base_dir: PathBuf::new(),
            catalog_priority: role.catalog_priority,
        });
        layer.catalog_priority = role.catalog_priority;
        loaded.layers.insert(role_name, layer);
    }
    loaded
}

pub(crate) fn catalog_order(
    declarations: &BTreeMap<String, AgentRoleConfig>,
    layers: &BTreeMap<String, MaterializedAgentRoleLayer>,
) -> Vec<String> {
    let mut names = declarations.keys().cloned().collect::<Vec<_>>();
    names.sort_by(|left, right| {
        layers
            .get(right)
            .map_or(0, |layer| layer.catalog_priority)
            .cmp(&layers.get(left).map_or(0, |layer| layer.catalog_priority))
            .then_with(|| left.cmp(right))
    });
    names
}

async fn read_declared_role(
    fs: &dyn ExecutorFileSystem,
    declared_role_name: &str,
    role_toml: &AgentRoleToml,
) -> std::io::Result<(String, LoadedAgentRole)> {
    let mut role = agent_role_config_from_toml(fs, declared_role_name, role_toml).await?;
    let mut role_name = declared_role_name.to_string();
    let mut layer = None;
    if let Some(config_file) = role.config_file.as_deref() {
        let config_file = AbsolutePathBuf::from_absolute_path(config_file)?;
        let materialized =
            read_materialized_agent_role_file(fs, &config_file, Some(declared_role_name)).await?;
        role_name = materialized.resolved.role_name;
        role.description = materialized.resolved.description.or(role.description);
        role.nickname_candidates = materialized
            .resolved
            .nickname_candidates
            .or(role.nickname_candidates);
        layer = Some(materialized.layer);
    }

    Ok((
        role_name,
        LoadedAgentRole {
            declaration: role,
            layer,
            catalog_priority: 0,
        },
    ))
}

fn merge_missing_role_fields(role: &mut AgentRoleConfig, fallback: &AgentRoleConfig) {
    role.description = role.description.clone().or(fallback.description.clone());
    role.config_file = role.config_file.clone().or(fallback.config_file.clone());
    role.nickname_candidates = role
        .nickname_candidates
        .clone()
        .or(fallback.nickname_candidates.clone());
}

fn agents_toml_from_layer(
    layer_toml: &TomlValue,
    config_base_dir: Option<&Path>,
) -> std::io::Result<Option<AgentsToml>> {
    let Some(agents_toml) = layer_toml.get("agents") else {
        return Ok(None);
    };

    // AbsolutePathBufGuard resolves relative paths while it remains in scope.
    let _guard = config_base_dir.map(AbsolutePathBufGuard::new);
    agents_toml
        .clone()
        .try_into()
        .map(Some)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))
}

async fn agent_role_config_from_toml(
    fs: &dyn ExecutorFileSystem,
    role_name: &str,
    role: &AgentRoleToml,
) -> std::io::Result<AgentRoleConfig> {
    let config_file = role
        .config_file
        .as_ref()
        .map(AbsolutePathBuf::from_absolute_path)
        .transpose()?;
    validate_agent_role_config_file(fs, role_name, config_file.as_ref()).await?;
    let description = normalize_agent_role_description(
        &format!("agents.{role_name}.description"),
        role.description.as_deref(),
    )?;
    let nickname_candidates = normalize_agent_role_nickname_candidates(
        &format!("agents.{role_name}.nickname_candidates"),
        role.nickname_candidates.as_deref(),
    )?;

    Ok(AgentRoleConfig {
        description,
        config_file: config_file.map(AbsolutePathBuf::into_path_buf),
        nickname_candidates,
    })
}

#[derive(Deserialize, Debug, Clone, Default, PartialEq)]
#[serde(deny_unknown_fields)]
struct RawAgentRoleFileToml {
    name: Option<String>,
    description: Option<String>,
    nickname_candidates: Option<Vec<String>>,
    #[serde(flatten)]
    config: ConfigToml,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ResolvedAgentRoleFile {
    pub(crate) role_name: String,
    pub(crate) description: Option<String>,
    pub(crate) nickname_candidates: Option<Vec<String>>,
    pub(crate) config: TomlValue,
}

pub(crate) fn parse_agent_role_file_contents(
    contents: &str,
    role_file_label: &Path,
    config_base_dir: &Path,
    role_name_hint: Option<&str>,
) -> std::io::Result<ResolvedAgentRoleFile> {
    let role_file_toml: TomlValue = toml::from_str(contents).map_err(|err| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "failed to parse agent role file at {}: {err}",
                role_file_label.display()
            ),
        )
    })?;
    let _guard = AbsolutePathBufGuard::new(config_base_dir);
    let parsed: RawAgentRoleFileToml = role_file_toml.clone().try_into().map_err(|err| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "failed to deserialize agent role file at {}: {err}",
                role_file_label.display()
            ),
        )
    })?;
    let description = normalize_agent_role_description(
        &format!("agent role file {}.description", role_file_label.display()),
        parsed.description.as_deref(),
    )?;
    validate_agent_role_file_developer_instructions(
        role_file_label,
        parsed.config.developer_instructions.as_deref(),
        role_name_hint.is_none(),
    )?;

    let role_name = parsed
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| role_name_hint.map(ToOwned::to_owned))
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "agent role file at {} must define a non-empty `name`",
                    role_file_label.display()
                ),
            )
        })?;

    let nickname_candidates = normalize_agent_role_nickname_candidates(
        &format!(
            "agent role file {}.nickname_candidates",
            role_file_label.display()
        ),
        parsed.nickname_candidates.as_deref(),
    )?;

    let mut config = role_file_toml;
    let Some(config_table) = config.as_table_mut() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "agent role file at {} must contain a TOML table",
                role_file_label.display()
            ),
        ));
    };
    config_table.remove("name");
    config_table.remove("description");
    config_table.remove("nickname_candidates");

    Ok(ResolvedAgentRoleFile {
        role_name,
        description,
        nickname_candidates,
        config,
    })
}

pub(crate) fn agent_role_locked_settings_note(config: &TomlValue) -> String {
    let model = config.get("model").and_then(TomlValue::as_str);
    let reasoning_effort = config
        .get("model_reasoning_effort")
        .and_then(TomlValue::as_str);
    let service_tier = config.get("service_tier").and_then(TomlValue::as_str);
    let model_and_reasoning_note = match (model, reasoning_effort) {
        (Some(model), Some(reasoning_effort)) => format!(
            "\n- This role's model is set to `{model}` and its reasoning effort is set to `{reasoning_effort}`. These settings cannot be changed."
        ),
        (Some(model), None) => {
            format!("\n- This role's model is set to `{model}` and cannot be changed.")
        }
        (None, Some(reasoning_effort)) => format!(
            "\n- This role's reasoning effort is set to `{reasoning_effort}` and cannot be changed."
        ),
        (None, None) => String::new(),
    };
    let service_tier_note = service_tier
        .map(|service_tier| {
            format!(
                "\n- This role's service tier is set to `{service_tier}`. If it is supported by the resolved model, it takes precedence over a valid spawn request service tier."
            )
        })
        .unwrap_or_default();
    format!("{model_and_reasoning_note}{service_tier_note}")
}

async fn read_materialized_agent_role_file(
    fs: &dyn ExecutorFileSystem,
    path: &AbsolutePathBuf,
    role_name_hint: Option<&str>,
) -> std::io::Result<MaterializedAgentRoleFile> {
    let path_uri = PathUri::from_abs_path(path);
    let metadata = fs.get_metadata(&path_uri, /*sandbox*/ None).await?;
    if !metadata.is_file || metadata.size > MAX_AGENT_ROLE_FILE_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "agent role file at {} must be a file no larger than {MAX_AGENT_ROLE_FILE_BYTES} bytes",
                path.as_path().display()
            ),
        ));
    }
    let contents = read_bounded_agent_role_file(fs, &path_uri, path.as_path()).await?;
    let config_base_dir = path.parent().unwrap_or_else(|| path.clone());
    let resolved = parse_agent_role_file_contents(
        &contents,
        path.as_path(),
        config_base_dir.as_path(),
        role_name_hint,
    )?;
    deserialize_config_toml_with_base(resolved.config.clone(), config_base_dir.as_path())
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
    Ok(MaterializedAgentRoleFile {
        layer: MaterializedAgentRoleLayer {
            config: resolved.config.clone(),
            base_dir: config_base_dir.into_path_buf(),
            catalog_priority: 0,
        },
        resolved,
    })
}

async fn read_bounded_agent_role_file(
    fs: &dyn ExecutorFileSystem,
    path_uri: &PathUri,
    path_label: &Path,
) -> std::io::Result<String> {
    let mut stream = fs.read_file_stream(path_uri, /*sandbox*/ None).await?;
    let mut contents = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if chunk.len() > MAX_AGENT_ROLE_FILE_BYTES as usize - contents.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "agent role file at {} must be a file no larger than {MAX_AGENT_ROLE_FILE_BYTES} bytes",
                    path_label.display()
                ),
            ));
        }
        contents.extend_from_slice(&chunk);
    }
    String::from_utf8(contents)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))
}

#[cfg(test)]
pub(crate) async fn materialize_agent_role_for_test(
    config: &mut super::Config,
    role_name: &str,
) -> std::io::Result<()> {
    let config_file = config
        .agent_roles
        .get(role_name)
        .and_then(|role| role.config_file.as_deref())
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("test agent role `{role_name}` has no config file"),
            )
        })?;
    let config_file = AbsolutePathBuf::from_absolute_path(config_file)?;
    let materialized = read_materialized_agent_role_file(
        codex_exec_server::LOCAL_FS.as_ref(),
        &config_file,
        Some(role_name),
    )
    .await?;
    config
        .materialized_agent_role_layers
        .insert(role_name.to_string(), materialized.layer);
    Ok(())
}

fn normalize_agent_role_description(
    field_label: &str,
    description: Option<&str>,
) -> std::io::Result<Option<String>> {
    match description.map(str::trim) {
        Some("") => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{field_label} cannot be blank"),
        )),
        Some(description) => Ok(Some(description.to_string())),
        None => Ok(None),
    }
}

fn validate_required_agent_role_description(
    role_name: &str,
    description: Option<&str>,
) -> std::io::Result<()> {
    if description.is_some() {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("agent role `{role_name}` must define a description"),
        ))
    }
}

fn validate_agent_role_file_developer_instructions(
    role_file_label: &Path,
    developer_instructions: Option<&str>,
    require_present: bool,
) -> std::io::Result<()> {
    match developer_instructions.map(str::trim) {
        Some("") => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "agent role file at {}.developer_instructions cannot be blank",
                role_file_label.display()
            ),
        )),
        Some(developer_instructions)
            if developer_instructions.len() > MAX_AGENT_ROLE_DEVELOPER_INSTRUCTIONS_BYTES =>
        {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "agent role file at {}.developer_instructions must be at most {MAX_AGENT_ROLE_DEVELOPER_INSTRUCTIONS_BYTES} bytes",
                    role_file_label.display()
                ),
            ))
        }
        Some(_) => Ok(()),
        None if require_present => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "agent role file at {} must define `developer_instructions`",
                role_file_label.display()
            ),
        )),
        None => Ok(()),
    }
}

async fn validate_agent_role_config_file(
    fs: &dyn ExecutorFileSystem,
    role_name: &str,
    config_file: Option<&AbsolutePathBuf>,
) -> std::io::Result<()> {
    let Some(config_file) = config_file else {
        return Ok(());
    };

    let config_file_uri = PathUri::from_abs_path(config_file);
    let metadata = fs
        .get_metadata(&config_file_uri, /*sandbox*/ None)
        .await
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "agents.{role_name}.config_file must point to an existing file at {}: {e}",
                    config_file.as_path().display()
                ),
            )
        })?;
    if metadata.is_file && metadata.size <= MAX_AGENT_ROLE_FILE_BYTES {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "agents.{role_name}.config_file must point to a file no larger than {MAX_AGENT_ROLE_FILE_BYTES} bytes: {}",
                config_file.as_path().display()
            ),
        ))
    }
}

fn normalize_agent_role_nickname_candidates(
    field_label: &str,
    nickname_candidates: Option<&[String]>,
) -> std::io::Result<Option<Vec<String>>> {
    let Some(nickname_candidates) = nickname_candidates else {
        return Ok(None);
    };

    if nickname_candidates.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{field_label} must contain at least one name"),
        ));
    }

    let mut normalized_candidates = Vec::with_capacity(nickname_candidates.len());
    let mut seen_candidates = BTreeSet::new();

    for nickname in nickname_candidates {
        let normalized_nickname = nickname.trim();
        if normalized_nickname.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("{field_label} cannot contain blank names"),
            ));
        }

        if !seen_candidates.insert(normalized_nickname.to_owned()) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("{field_label} cannot contain duplicates"),
            ));
        }

        if !normalized_nickname
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '_'))
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "{field_label} may only contain ASCII letters, digits, spaces, hyphens, and underscores"
                ),
            ));
        }

        normalized_candidates.push(normalized_nickname.to_owned());
    }

    Ok(Some(normalized_candidates))
}

async fn discover_agent_roles_in_dir(
    fs: &dyn ExecutorFileSystem,
    agents_dir: &AbsolutePathBuf,
    declared_role_files: &BTreeSet<PathBuf>,
    startup_warnings: &mut Vec<String>,
) -> std::io::Result<BTreeMap<String, LoadedAgentRole>> {
    let mut roles = BTreeMap::new();

    for agent_file in collect_agent_role_files(fs, agents_dir).await? {
        if declared_role_files.contains(agent_file.as_path()) {
            continue;
        }
        let materialized =
            match read_materialized_agent_role_file(fs, &agent_file, /*role_name_hint*/ None).await
            {
                Ok(materialized) => materialized,
                Err(err) => {
                    push_agent_role_warning(startup_warnings, err);
                    continue;
                }
            };
        let role_name = materialized.resolved.role_name.clone();
        if roles.contains_key(&role_name) {
            push_agent_role_warning(
                startup_warnings,
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!(
                        "duplicate agent role name `{role_name}` discovered in {}",
                        agents_dir.as_path().display()
                    ),
                ),
            );
            continue;
        }
        roles.insert(
            role_name,
            LoadedAgentRole {
                declaration: AgentRoleConfig {
                    description: materialized.resolved.description,
                    config_file: Some(agent_file.to_path_buf()),
                    nickname_candidates: materialized.resolved.nickname_candidates,
                },
                layer: Some(materialized.layer),
                catalog_priority: 0,
            },
        );
    }

    Ok(roles)
}

async fn collect_agent_role_files(
    fs: &dyn ExecutorFileSystem,
    dir: &AbsolutePathBuf,
) -> std::io::Result<Vec<AbsolutePathBuf>> {
    let mut files = Vec::new();
    let mut dirs = vec![dir.clone()];
    while let Some(dir) = dirs.pop() {
        let dir_uri = PathUri::from_abs_path(&dir);
        let entries = match fs.read_directory(&dir_uri, /*sandbox*/ None).await {
            Ok(entries) => entries,
            Err(err) if err.kind() == ErrorKind::NotFound => continue,
            Err(err) => return Err(err),
        };

        for entry in entries {
            let path = dir.join(entry.file_name);
            if entry.is_directory {
                dirs.push(path);
                continue;
            }
            if entry.is_file
                && path
                    .as_path()
                    .extension()
                    .is_some_and(|extension| extension == "toml")
            {
                files.push(path);
                if files.len() > MAX_DISCOVERED_AGENT_ROLE_FILES {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!(
                            "agent role discovery exceeds the limit of {MAX_DISCOVERED_AGENT_ROLE_FILES} files under {}",
                            dir.as_path().display()
                        ),
                    ));
                }
            }
        }
    }

    files.sort();
    Ok(files)
}
