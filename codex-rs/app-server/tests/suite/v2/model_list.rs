use std::time::Duration;

use anyhow::Error;
use anyhow::Result;
use app_test_support::ChatGptAuthFixture;
use app_test_support::TestAppServer;
use app_test_support::to_response;
use app_test_support::write_chatgpt_auth;
use app_test_support::write_models_cache;
use app_test_support::write_models_cache_with_models;
use codex_app_server_protocol::ConfigWriteResponse;
use codex_app_server_protocol::JSONRPCError;
use codex_app_server_protocol::JSONRPCResponse;
use codex_app_server_protocol::Model;
use codex_app_server_protocol::ModelListParams;
use codex_app_server_protocol::ModelListResponse;
use codex_app_server_protocol::ModelProviderAuthRemoveParams;
use codex_app_server_protocol::ModelProviderAuthStatus;
use codex_app_server_protocol::ModelProviderAuthWriteParams;
use codex_app_server_protocol::ModelProviderAuthWriteResponse;
use codex_app_server_protocol::ModelProviderConfigWriteParams;
use codex_app_server_protocol::ModelProviderListParams;
use codex_app_server_protocol::ModelProviderListResponse;
use codex_app_server_protocol::ModelServiceTier;
use codex_app_server_protocol::ModelUpgradeInfo;
use codex_app_server_protocol::ReasoningEffortOption;
use codex_app_server_protocol::RequestId;
use codex_config::types::AuthCredentialsStoreMode;
use codex_model_provider_info::DEEPSEEK_PROVIDER_ID;
use codex_model_provider_info::LMSTUDIO_OSS_PROVIDER_ID;
use codex_model_provider_info::OLLAMA_OSS_PROVIDER_ID;
use codex_model_provider_info::OPENAI_PROVIDER_ID;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::openai_models::ModelPreset;
use codex_protocol::openai_models::ModelsResponse;
use core_test_support::responses::mount_models_once;
use pretty_assertions::assert_eq;
use serde_json::json;
use tempfile::TempDir;
use tokio::time::timeout;
use wiremock::MockServer;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const INVALID_REQUEST_ERROR_CODE: i64 = -32600;

fn model_from_preset(preset: &ModelPreset) -> Model {
    Model {
        id: preset.id.clone(),
        model_provider: preset.model_provider.clone(),
        model: preset.model.clone(),
        upgrade: preset.upgrade.as_ref().map(|upgrade| upgrade.id.clone()),
        upgrade_info: preset.upgrade.as_ref().map(|upgrade| ModelUpgradeInfo {
            model: upgrade.id.clone(),
            upgrade_copy: upgrade.upgrade_copy.clone(),
            model_link: upgrade.model_link.clone(),
            migration_markdown: upgrade.migration_markdown.clone(),
        }),
        availability_nux: preset.availability_nux.clone().map(Into::into),
        display_name: preset.display_name.clone(),
        description: preset.description.clone(),
        hidden: !preset.show_in_picker,
        supported_reasoning_efforts: preset
            .supported_reasoning_efforts
            .iter()
            .map(|preset| ReasoningEffortOption {
                reasoning_effort: preset.effort.clone(),
                description: preset.description.clone(),
            })
            .collect(),
        default_reasoning_effort: preset.default_reasoning_effort.clone(),
        input_modalities: preset.input_modalities.clone(),
        // `write_models_cache()` round-trips through a simplified ModelInfo fixture that does not
        // preserve personality placeholders in base instructions, so app-server list results from
        // cache report `supports_personality = false`.
        // todo(sayan): fix, maybe make roundtrip use ModelInfo only
        supports_personality: false,
        additional_speed_tiers: preset.additional_speed_tiers.clone(),
        service_tiers: preset
            .service_tiers
            .iter()
            .map(|service_tier| ModelServiceTier {
                id: service_tier.id.clone(),
                name: service_tier.name.clone(),
                description: service_tier.description.clone(),
            })
            .collect(),
        default_service_tier: preset.default_service_tier.clone(),
        is_default: preset.is_default,
    }
}

fn expected_visible_models() -> Vec<Model> {
    // Filter by supported_in_api to support testing with both ChatGPT and non-ChatGPT auth modes.
    let mut presets = ModelPreset::filter_by_auth(
        codex_core::test_support::all_model_presets().clone(),
        /*chatgpt_mode*/ false,
    );

    // Mirror `ModelsManager::build_available_models()` default selection after auth filtering.
    ModelPreset::mark_default_by_picker_visibility(&mut presets);

    presets
        .iter()
        .filter(|preset| preset.show_in_picker)
        .map(model_from_preset)
        .collect()
}

#[tokio::test]
async fn list_models_returns_all_models_with_large_limit() -> Result<()> {
    let codex_home = TempDir::new()?;
    write_models_cache(codex_home.path())?;
    let mut mcp = TestAppServer::new(codex_home.path()).await?;

    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_list_models_request(ModelListParams {
            limit: Some(100),
            cursor: None,
            include_hidden: None,
            include_configured_providers: None,
        })
        .await?;

    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;

    let ModelListResponse {
        data: items,
        next_cursor,
    } = to_response::<ModelListResponse>(response)?;

    let expected_models = expected_visible_models();

    assert_eq!(items, expected_models);
    assert!(next_cursor.is_none());
    Ok(())
}

#[tokio::test]
async fn list_models_includes_hidden_models() -> Result<()> {
    let codex_home = TempDir::new()?;
    write_models_cache(codex_home.path())?;
    let mut mcp = TestAppServer::new(codex_home.path()).await?;

    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_list_models_request(ModelListParams {
            limit: Some(100),
            cursor: None,
            include_hidden: Some(true),
            include_configured_providers: None,
        })
        .await?;

    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;

    let ModelListResponse {
        data: items,
        next_cursor,
    } = to_response::<ModelListResponse>(response)?;

    assert!(items.iter().any(|item| item.hidden));
    assert!(next_cursor.is_none());
    Ok(())
}

#[tokio::test]
async fn list_models_uses_chatgpt_remote_catalog_as_source_of_truth() -> Result<()> {
    let server = MockServer::start().await;
    let remote_model: ModelInfo = serde_json::from_value(json!({
        "slug": "chatgpt-remote-only",
        "display_name": "ChatGPT Remote Only",
        "description": "Remote-only model for app-server model/list coverage",
        "default_reasoning_level": "max",
        "supported_reasoning_levels": [
            {"effort": "max", "description": "Maximum"},
            {"effort": "low", "description": "Low"},
            {"effort": "focused", "description": "Focused"}
        ],
        "shell_type": "shell_command",
        "visibility": "list",
        "minimal_client_version": [0, 1, 0],
        "supported_in_api": true,
        "priority": 0,
        "upgrade": null,
        "base_instructions": "base instructions",
        "supports_reasoning_summaries": false,
        "support_verbosity": false,
        "default_verbosity": null,
        "apply_patch_tool_type": null,
        "truncation_policy": {"mode": "bytes", "limit": 10_000},
        "supports_parallel_tool_calls": false,
        "supports_image_detail_original": false,
        "context_window": 272_000,
        "max_context_window": 272_000,
        "experimental_supported_tools": [],
    }))?;
    let models_mock = mount_models_once(
        &server,
        ModelsResponse {
            models: vec![remote_model.clone()],
        },
    )
    .await;

    let codex_home = TempDir::new()?;
    let server_uri = server.uri();
    std::fs::write(
        codex_home.path().join("config.toml"),
        format!(
            r#"
model = "mock-model"
approval_policy = "never"
sandbox_mode = "read-only"
openai_base_url = "{server_uri}/v1"
"#
        ),
    )?;
    write_chatgpt_auth(
        codex_home.path(),
        ChatGptAuthFixture::new("chatgpt-access-token").plan_type("pro"),
        AuthCredentialsStoreMode::File,
    )?;

    let mut mcp =
        TestAppServer::new_with_env(codex_home.path(), &[("OPENAI_API_KEY", None)]).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_list_models_request(ModelListParams {
            limit: Some(100),
            cursor: None,
            include_hidden: None,
            include_configured_providers: None,
        })
        .await?;

    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;

    let ModelListResponse {
        data: items,
        next_cursor,
    } = to_response::<ModelListResponse>(response)?;
    let mut expected_presets: Vec<ModelPreset> = vec![remote_model.into()];
    ModelPreset::mark_default_by_picker_visibility(&mut expected_presets);
    let mut expected_items = expected_presets
        .iter()
        .map(model_from_preset)
        .collect::<Vec<_>>();
    expected_items[0].supported_reasoning_efforts = vec![
        ReasoningEffortOption {
            reasoning_effort: "max".parse().map_err(Error::msg)?,
            description: "Maximum".to_string(),
        },
        ReasoningEffortOption {
            reasoning_effort: "low".parse().map_err(Error::msg)?,
            description: "Low".to_string(),
        },
        ReasoningEffortOption {
            reasoning_effort: "focused".parse().map_err(Error::msg)?,
            description: "Focused".to_string(),
        },
    ];

    assert_eq!(items, expected_items);
    assert!(next_cursor.is_none());
    assert_eq!(
        models_mock.requests().len(),
        1,
        "expected a single /models request"
    );
    Ok(())
}

#[tokio::test]
async fn list_models_can_include_configured_provider_catalogs() -> Result<()> {
    let codex_home = TempDir::new()?;
    write_models_cache(codex_home.path())?;
    let mut mcp = TestAppServer::new(codex_home.path()).await?;

    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_list_models_request(ModelListParams {
            limit: Some(100),
            cursor: None,
            include_hidden: None,
            include_configured_providers: Some(true),
        })
        .await?;

    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;

    let ModelListResponse {
        data: items,
        next_cursor,
    } = to_response::<ModelListResponse>(response)?;

    assert!(
        items
            .iter()
            .any(|item| item.model_provider == DEEPSEEK_PROVIDER_ID
                && item.model.starts_with("deepseek-v4"))
    );
    assert!(
        !items
            .iter()
            .any(|item| item.model_provider == OLLAMA_OSS_PROVIDER_ID
                || item.model_provider == LMSTUDIO_OSS_PROVIDER_ID),
        "inactive local OSS providers must not expose bundled OpenAI models in provider-aware catalog"
    );
    assert!(next_cursor.is_none());
    Ok(())
}

#[tokio::test]
async fn disabled_provider_is_hidden_from_provider_aware_model_list() -> Result<()> {
    let codex_home = TempDir::new()?;
    std::fs::write(
        codex_home.path().join("config.toml"),
        r#"
model = "gpt-5.4"
approval_policy = "never"
sandbox_mode = "read-only"
"#,
    )?;
    write_models_cache(codex_home.path())?;
    let mut mcp = TestAppServer::new(codex_home.path()).await?;

    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let config_write_request_id = mcp
        .send_model_provider_config_write_request(ModelProviderConfigWriteParams {
            provider_id: DEEPSEEK_PROVIDER_ID.to_string(),
            enabled_in_picker: Some(false),
            set_active: false,
        })
        .await?;
    let config_write_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(config_write_request_id)),
    )
    .await??;
    let _ = to_response::<ConfigWriteResponse>(config_write_response)?;

    let request_id = mcp
        .send_list_models_request(ModelListParams {
            limit: Some(100),
            cursor: None,
            include_hidden: None,
            include_configured_providers: Some(true),
        })
        .await?;

    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let ModelListResponse { data: items, .. } = to_response::<ModelListResponse>(response)?;

    assert!(
        !items
            .iter()
            .any(|item| item.model_provider == DEEPSEEK_PROVIDER_ID),
        "disabled provider must be hidden from provider-aware model/list"
    );
    Ok(())
}

#[tokio::test]
async fn openai_provider_is_native_and_not_manageable() -> Result<()> {
    let codex_home = TempDir::new()?;
    write_models_cache(codex_home.path())?;
    let mut mcp = TestAppServer::new(codex_home.path()).await?;

    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let list_request_id = mcp
        .send_model_provider_list_request(ModelProviderListParams {})
        .await?;
    let list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(list_request_id)),
    )
    .await??;
    let providers = to_response::<ModelProviderListResponse>(list_response)?;
    assert!(
        !providers
            .data
            .iter()
            .any(|provider| provider.id == OPENAI_PROVIDER_ID),
        "OpenAI is native and must not appear in the provider management list"
    );

    let disable_request_id = mcp
        .send_model_provider_config_write_request(ModelProviderConfigWriteParams {
            provider_id: OPENAI_PROVIDER_ID.to_string(),
            enabled_in_picker: Some(false),
            set_active: false,
        })
        .await?;
    let error: JSONRPCError = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(disable_request_id)),
    )
    .await??;
    assert_eq!(error.id, RequestId::Integer(disable_request_id));
    assert_eq!(error.error.code, INVALID_REQUEST_ERROR_CODE);
    assert_eq!(
        error.error.message,
        "OpenAI is a native model provider and cannot be disabled"
    );

    let auth_write_request_id = mcp
        .send_model_provider_auth_write_request(ModelProviderAuthWriteParams {
            provider_id: OPENAI_PROVIDER_ID.to_string(),
            api_key: "openai-secret".to_string(),
        })
        .await?;
    let error: JSONRPCError = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(auth_write_request_id)),
    )
    .await??;
    assert_eq!(error.id, RequestId::Integer(auth_write_request_id));
    assert_eq!(error.error.code, INVALID_REQUEST_ERROR_CODE);
    assert_eq!(
        error.error.message,
        "OpenAI is a native model provider; use account login to manage its auth"
    );

    let auth_remove_request_id = mcp
        .send_model_provider_auth_remove_request(ModelProviderAuthRemoveParams {
            provider_id: OPENAI_PROVIDER_ID.to_string(),
        })
        .await?;
    let error: JSONRPCError = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(auth_remove_request_id)),
    )
    .await??;
    assert_eq!(error.id, RequestId::Integer(auth_remove_request_id));
    assert_eq!(error.error.code, INVALID_REQUEST_ERROR_CODE);
    assert_eq!(
        error.error.message,
        "OpenAI is a native model provider; use account login to manage its auth"
    );
    Ok(())
}

#[tokio::test]
async fn model_provider_set_active_writes_compatible_default_model() -> Result<()> {
    let codex_home = TempDir::new()?;
    std::fs::write(
        codex_home.path().join("config.toml"),
        r#"
model = "gpt-5.4"
approval_policy = "never"
sandbox_mode = "read-only"
"#,
    )?;
    write_models_cache(codex_home.path())?;
    let mut mcp = TestAppServer::new(codex_home.path()).await?;

    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_model_provider_config_write_request(ModelProviderConfigWriteParams {
            provider_id: DEEPSEEK_PROVIDER_ID.to_string(),
            enabled_in_picker: None,
            set_active: true,
        })
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let _ = to_response::<ConfigWriteResponse>(response)?;

    let config = std::fs::read_to_string(codex_home.path().join("config.toml"))?;
    assert!(config.contains("model_provider = \"deepseek\""));
    assert!(
        config.contains("model = \"deepseek-v4-pro\"")
            || config.contains("model = \"deepseek-v4-flash\""),
        "expected set_active to persist a DeepSeek-compatible default model, got:\n{config}"
    );
    Ok(())
}

#[tokio::test]
async fn model_provider_cannot_be_set_active_and_hidden() -> Result<()> {
    let codex_home = TempDir::new()?;
    write_models_cache(codex_home.path())?;
    let mut mcp = TestAppServer::new(codex_home.path()).await?;

    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_model_provider_config_write_request(ModelProviderConfigWriteParams {
            provider_id: DEEPSEEK_PROVIDER_ID.to_string(),
            enabled_in_picker: Some(false),
            set_active: true,
        })
        .await?;
    let error: JSONRPCError = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(request_id)),
    )
    .await??;
    assert_eq!(error.id, RequestId::Integer(request_id));
    assert_eq!(error.error.code, INVALID_REQUEST_ERROR_CODE);
    assert_eq!(
        error.error.message,
        "model provider cannot be made active and hidden from /model in the same request"
    );
    Ok(())
}

#[tokio::test]
async fn stale_disabled_openai_config_does_not_hide_openai_models() -> Result<()> {
    let codex_home = TempDir::new()?;
    std::fs::write(
        codex_home.path().join("config.toml"),
        r#"
model = "deepseek-v4"
model_provider = "deepseek"
disabled_model_providers = ["openai"]
approval_policy = "never"
sandbox_mode = "read-only"
"#,
    )?;
    write_models_cache(codex_home.path())?;
    let mut mcp = TestAppServer::new(codex_home.path()).await?;

    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_list_models_request(ModelListParams {
            limit: Some(100),
            cursor: None,
            include_hidden: None,
            include_configured_providers: Some(true),
        })
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let ModelListResponse { data: items, .. } = to_response::<ModelListResponse>(response)?;

    assert!(
        items
            .iter()
            .any(|item| item.model_provider == OPENAI_PROVIDER_ID),
        "OpenAI models must remain available even if stale config lists OpenAI as disabled"
    );
    Ok(())
}

#[tokio::test]
async fn model_provider_auth_write_and_remove_updates_managed_key_status() -> Result<()> {
    let codex_home = TempDir::new()?;
    write_models_cache(codex_home.path())?;
    let mut mcp =
        TestAppServer::new_with_env(codex_home.path(), &[("DEEPSEEK_API_KEY", None)]).await?;

    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let write_request_id = mcp
        .send_model_provider_auth_write_request(ModelProviderAuthWriteParams {
            provider_id: DEEPSEEK_PROVIDER_ID.to_string(),
            api_key: "deepseek-secret".to_string(),
        })
        .await?;
    let write_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(write_request_id)),
    )
    .await??;
    let write = to_response::<ModelProviderAuthWriteResponse>(write_response)?;
    assert_eq!(
        write,
        ModelProviderAuthWriteResponse {
            provider_id: DEEPSEEK_PROVIDER_ID.to_string(),
            managed_key_present: true,
        }
    );

    let list_request_id = mcp
        .send_model_provider_list_request(ModelProviderListParams {})
        .await?;
    let list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(list_request_id)),
    )
    .await??;
    let providers = to_response::<ModelProviderListResponse>(list_response)?;
    let deepseek = providers
        .data
        .iter()
        .find(|provider| provider.id == DEEPSEEK_PROVIDER_ID)
        .expect("deepseek provider listed");
    assert_eq!(
        deepseek.auth_status,
        ModelProviderAuthStatus::ManagedKeyPresent
    );

    let remove_request_id = mcp
        .send_model_provider_auth_remove_request(ModelProviderAuthRemoveParams {
            provider_id: DEEPSEEK_PROVIDER_ID.to_string(),
        })
        .await?;
    let _: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(remove_request_id)),
    )
    .await??;

    let list_request_id = mcp
        .send_model_provider_list_request(ModelProviderListParams {})
        .await?;
    let list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(list_request_id)),
    )
    .await??;
    let providers = to_response::<ModelProviderListResponse>(list_response)?;
    let deepseek = providers
        .data
        .iter()
        .find(|provider| provider.id == DEEPSEEK_PROVIDER_ID)
        .expect("deepseek provider listed after remove");
    assert_eq!(deepseek.auth_status, ModelProviderAuthStatus::EnvKeyMissing);
    Ok(())
}

#[tokio::test]
async fn model_provider_env_key_takes_precedence_over_managed_key_status() -> Result<()> {
    let codex_home = TempDir::new()?;
    write_models_cache(codex_home.path())?;
    let mut mcp = TestAppServer::new_with_env(
        codex_home.path(),
        &[("DEEPSEEK_API_KEY", Some("deepseek-env-secret"))],
    )
    .await?;

    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let write_request_id = mcp
        .send_model_provider_auth_write_request(ModelProviderAuthWriteParams {
            provider_id: DEEPSEEK_PROVIDER_ID.to_string(),
            api_key: "deepseek-managed-secret".to_string(),
        })
        .await?;
    let _: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(write_request_id)),
    )
    .await??;

    let list_request_id = mcp
        .send_model_provider_list_request(ModelProviderListParams {})
        .await?;
    let list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(list_request_id)),
    )
    .await??;
    let providers = to_response::<ModelProviderListResponse>(list_response)?;
    let deepseek = providers
        .data
        .iter()
        .find(|provider| provider.id == DEEPSEEK_PROVIDER_ID)
        .expect("deepseek provider listed");

    assert_eq!(deepseek.auth_status, ModelProviderAuthStatus::EnvKeyPresent);
    Ok(())
}

#[tokio::test]
async fn list_models_does_not_reuse_active_provider_cache_for_inactive_provider() -> Result<()> {
    let codex_home = TempDir::new()?;
    let cached_active_model: ModelInfo = serde_json::from_value(json!({
        "slug": "active-cache-only",
        "display_name": "Active Cache Only",
        "description": "Model seeded into the active provider global cache",
        "default_reasoning_level": "medium",
        "supported_reasoning_levels": [
            {"effort": "medium", "description": "Medium"}
        ],
        "shell_type": "shell_command",
        "visibility": "list",
        "minimal_client_version": [0, 1, 0],
        "supported_in_api": true,
        "priority": 0,
        "upgrade": null,
        "base_instructions": "base instructions",
        "supports_reasoning_summaries": false,
        "support_verbosity": false,
        "default_verbosity": null,
        "apply_patch_tool_type": null,
        "truncation_policy": {"mode": "bytes", "limit": 10_000},
        "supports_parallel_tool_calls": false,
        "supports_image_detail_original": false,
        "context_window": 272_000,
        "max_context_window": 272_000,
        "experimental_supported_tools": [],
    }))?;
    write_models_cache_with_models(codex_home.path(), vec![cached_active_model])?;
    std::fs::write(
        codex_home.path().join("config.toml"),
        r#"
model = "gpt-5.4"
approval_policy = "never"
sandbox_mode = "read-only"

[model_providers.inactive]
name = "Inactive Provider"
base_url = "https://inactive.example/v1"
env_key = "INACTIVE_API_KEY"
wire_api = "responses"
"#,
    )?;
    let mut mcp = TestAppServer::new(codex_home.path()).await?;

    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_list_models_request(ModelListParams {
            limit: Some(100),
            cursor: None,
            include_hidden: Some(true),
            include_configured_providers: Some(true),
        })
        .await?;

    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;

    let ModelListResponse {
        data: items,
        next_cursor,
    } = to_response::<ModelListResponse>(response)?;

    assert!(
        items
            .iter()
            .any(|item| item.model_provider == "openai" && item.model == "active-cache-only"),
        "seeded active provider cache should remain visible for active OpenAI"
    );
    assert!(
        !items
            .iter()
            .any(|item| item.model_provider == "inactive" && item.model == "active-cache-only"),
        "inactive provider must not inherit the active provider cache"
    );
    assert!(
        !items.iter().any(|item| item.model_provider == "inactive"),
        "inactive provider without an authoritative catalog must not expose bundled OpenAI models"
    );
    assert!(next_cursor.is_none());
    Ok(())
}

#[tokio::test]
async fn list_models_pagination_works() -> Result<()> {
    let codex_home = TempDir::new()?;
    write_models_cache(codex_home.path())?;
    let mut mcp = TestAppServer::new(codex_home.path()).await?;

    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let expected_models = expected_visible_models();
    let mut cursor = None;
    let mut items = Vec::new();

    for _ in 0..expected_models.len() {
        let request_id = mcp
            .send_list_models_request(ModelListParams {
                limit: Some(1),
                cursor: cursor.clone(),
                include_hidden: None,
                include_configured_providers: None,
            })
            .await?;

        let response: JSONRPCResponse = timeout(
            DEFAULT_TIMEOUT,
            mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
        )
        .await??;

        let ModelListResponse {
            data: page_items,
            next_cursor,
        } = to_response::<ModelListResponse>(response)?;

        assert_eq!(page_items.len(), 1);
        items.extend(page_items);

        if let Some(next_cursor) = next_cursor {
            cursor = Some(next_cursor);
        } else {
            assert_eq!(items, expected_models);
            return Ok(());
        }
    }

    panic!(
        "model pagination did not terminate after {} pages",
        expected_models.len()
    );
}

#[tokio::test]
async fn list_models_rejects_invalid_cursor() -> Result<()> {
    let codex_home = TempDir::new()?;
    write_models_cache(codex_home.path())?;
    let mut mcp = TestAppServer::new(codex_home.path()).await?;

    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_list_models_request(ModelListParams {
            limit: None,
            cursor: Some("invalid".to_string()),
            include_hidden: None,
            include_configured_providers: None,
        })
        .await?;

    let error: JSONRPCError = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(request_id)),
    )
    .await??;

    assert_eq!(error.id, RequestId::Integer(request_id));
    assert_eq!(error.error.code, INVALID_REQUEST_ERROR_CODE);
    assert_eq!(error.error.message, "invalid cursor: invalid");
    Ok(())
}
