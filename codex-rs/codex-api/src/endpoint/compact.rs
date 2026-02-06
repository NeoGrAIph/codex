use crate::auth::AuthProvider;
use crate::auth::add_auth_headers;
use crate::common::CompactionInput;
use crate::error::ApiError;
use crate::provider::Provider;
use crate::telemetry::run_with_request_telemetry;
use codex_client::HttpTransport;
use codex_client::RequestTelemetry;
use codex_protocol::models::ResponseItem;
use http::HeaderMap;
use http::Method;
use serde::Deserialize;
use serde_json::to_value;
use std::sync::Arc;

pub struct CompactClient<T: HttpTransport, A: AuthProvider> {
    transport: T,
    provider: Provider,
    auth: A,
    request_telemetry: Option<Arc<dyn RequestTelemetry>>,
}

impl<T: HttpTransport, A: AuthProvider> CompactClient<T, A> {
    pub fn new(transport: T, provider: Provider, auth: A) -> Self {
        Self {
            transport,
            provider,
            auth,
            request_telemetry: None,
        }
    }

    pub fn with_telemetry(mut self, request: Option<Arc<dyn RequestTelemetry>>) -> Self {
        self.request_telemetry = request;
        self
    }

    fn path(&self) -> &'static str {
        "responses/compact"
    }

    pub async fn compact(
        &self,
        body: serde_json::Value,
        extra_headers: HeaderMap,
    ) -> Result<Vec<ResponseItem>, ApiError> {
        let path = self.path();
        let builder = || {
            let mut req = self.provider.build_request(Method::POST, path);
            req.headers.extend(extra_headers.clone());
            req.body = Some(body.clone());
            add_auth_headers(&self.auth, req)
        };

        let resp = run_with_request_telemetry(
            self.provider.retry.to_policy(),
            self.request_telemetry.clone(),
            builder,
            |req| self.transport.execute(req),
        )
        .await?;
        let parsed: CompactHistoryResponse =
            serde_json::from_slice(&resp.body).map_err(|e| ApiError::Stream(e.to_string()))?;
        Ok(parsed.output)
    }

    pub async fn compact_input(
        &self,
        input: &CompactionInput<'_>,
        extra_headers: HeaderMap,
    ) -> Result<Vec<ResponseItem>, ApiError> {
        let body = to_value(input)
            .map_err(|e| ApiError::Stream(format!("failed to encode compaction input: {e}")))?;
        self.compact(body, extra_headers).await
    }
}

#[derive(Debug, Deserialize)]
struct CompactHistoryResponse {
    output: Vec<ResponseItem>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::RetryConfig;
    use async_trait::async_trait;
    use codex_client::Request;
    use codex_client::Response;
    use codex_client::StreamResponse;
    use codex_client::TransportError;
    use http::HeaderMap;
    use http::StatusCode;
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::time::Duration;

    #[derive(Clone, Default)]
    struct DummyTransport {
        last_request: Arc<Mutex<Option<Request>>>,
    }

    #[async_trait]
    impl HttpTransport for DummyTransport {
        async fn execute(&self, req: Request) -> Result<Response, TransportError> {
            *self.last_request.lock().unwrap() = Some(req);
            Ok(Response {
                status: StatusCode::OK,
                headers: HeaderMap::new(),
                body: serde_json::to_vec(&serde_json::json!({ "output": [] }))
                    .unwrap()
                    .into(),
            })
        }

        async fn stream(&self, _req: Request) -> Result<StreamResponse, TransportError> {
            Err(TransportError::Build("stream should not run".to_string()))
        }
    }

    #[derive(Clone, Default)]
    struct DummyAuth;

    impl AuthProvider for DummyAuth {
        fn bearer_token(&self) -> Option<String> {
            None
        }
    }

    fn provider() -> Provider {
        Provider {
            name: "test".to_string(),
            base_url: "https://example.com/v1".to_string(),
            query_params: None,
            headers: HeaderMap::new(),
            retry: RetryConfig {
                max_attempts: 1,
                base_delay: Duration::from_millis(1),
                retry_429: false,
                retry_5xx: true,
                retry_transport: true,
            },
            stream_idle_timeout: Duration::from_secs(1),
        }
    }

    #[tokio::test]
    async fn compact_uses_responses_compact_path() {
        let transport = DummyTransport::default();
        let client = CompactClient::new(transport.clone(), provider(), DummyAuth);
        let input = CompactionInput {
            model: "gpt-test",
            input: &[],
            instructions: "inst",
        };
        let output = client
            .compact_input(&input, HeaderMap::new())
            .await
            .expect("request should succeed");

        assert_eq!(output, Vec::<ResponseItem>::new());

        let url = transport
            .last_request
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .url
            .clone();
        assert!(
            url.ends_with("/responses/compact"),
            "expected url to end with /responses/compact, got {url}"
        );
    }
}
