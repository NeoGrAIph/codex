use crate::auth::AuthProvider;
use crate::auth::add_auth_headers;
use crate::common::MemoryTraceSummarizeInput;
use crate::common::MemoryTraceSummaryOutput;
use crate::error::ApiError;
use crate::provider::Provider;
use crate::telemetry::run_with_request_telemetry;
use codex_client::HttpTransport;
use codex_client::RequestTelemetry;
use http::HeaderMap;
use http::Method;
use serde::Deserialize;
use serde_json::to_value;
use std::sync::Arc;

pub struct MemoriesClient<T: HttpTransport, A: AuthProvider> {
    transport: T,
    provider: Provider,
    auth: A,
    request_telemetry: Option<Arc<dyn RequestTelemetry>>,
}

impl<T: HttpTransport, A: AuthProvider> MemoriesClient<T, A> {
    pub fn new(transport: T, provider: Provider, auth: A) -> Self {
        Self {
            transport,
            provider,
            auth,
            request_telemetry: None,
        }
    }

    pub fn with_telemetry(self, request: Option<Arc<dyn RequestTelemetry>>) -> Self {
        Self {
            request_telemetry: request,
            ..self
        }
    }

    fn path() -> &'static str {
        "memories/trace_summarize"
    }

    pub async fn trace_summarize(
        &self,
        body: serde_json::Value,
        extra_headers: HeaderMap,
    ) -> Result<Vec<MemoryTraceSummaryOutput>, ApiError> {
        let builder = || {
            let mut req = self.provider.build_request(Method::POST, Self::path());
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
        let parsed: TraceSummarizeResponse =
            serde_json::from_slice(&resp.body).map_err(|e| ApiError::Stream(e.to_string()))?;
        Ok(parsed.output)
    }

    pub async fn trace_summarize_input(
        &self,
        input: &MemoryTraceSummarizeInput,
        extra_headers: HeaderMap,
    ) -> Result<Vec<MemoryTraceSummaryOutput>, ApiError> {
        let body = to_value(input).map_err(|e| {
            ApiError::Stream(format!(
                "failed to encode memory trace summarize input: {e}"
            ))
        })?;
        self.trace_summarize(body, extra_headers).await
    }
}

#[derive(Debug, Deserialize)]
struct TraceSummarizeResponse {
    output: Vec<MemoryTraceSummaryOutput>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use codex_client::Request;
    use codex_client::Response;
    use codex_client::StreamResponse;
    use codex_client::TransportError;

    #[derive(Clone, Default)]
    struct DummyTransport;

    #[async_trait]
    impl HttpTransport for DummyTransport {
        async fn execute(&self, _req: Request) -> Result<Response, TransportError> {
            Err(TransportError::Build("execute should not run".to_string()))
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

    #[test]
    fn path_is_memories_trace_summarize() {
        assert_eq!(
            MemoriesClient::<DummyTransport, DummyAuth>::path(),
            "memories/trace_summarize"
        );
    }
}
