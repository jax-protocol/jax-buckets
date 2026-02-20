use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};

use crate::http_server::api::client::ApiRequest;

/// Request type for the liveness probe endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LivezRequest {}

/// Response type for the liveness probe endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LivezResponse {
    pub status: String,
}

impl ApiRequest for LivezRequest {
    type Response = LivezResponse;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let full_url = base_url.join("/_status/livez").unwrap();
        client.get(full_url)
    }
}

/// This is a very simple handler that always returns with a valid response. It's intended to be
/// used by external healthchecks to see whether the service is "alive". Failing this check for any
/// reason generally leads to immediate termination of the service.
///
/// If you're looking for how to report a service issue, please refer to
/// [`crate::health_check::readiness::handler`].
#[tracing::instrument]
pub async fn handler() -> Response {
    let msg = serde_json::json!({"status": "ok"});
    (StatusCode::OK, Json(msg)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_handler_direct() {
        let response = handler().await;
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();

        assert_eq!(&body[..], b"{\"status\":\"ok\"}");
    }
}
