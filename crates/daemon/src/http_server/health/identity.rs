use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::Json;
use http::StatusCode;
use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};

use crate::http_server::api::client::ApiRequest;
use crate::ServiceState;

/// Request type for the identity endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityResponse {
    /// The node's public identity (NodeId)
    pub node_id: String,
}

impl ApiRequest for IdentityRequest {
    type Response = IdentityResponse;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let full_url = base_url.join("/_status/identity").unwrap();
        client.get(full_url)
    }
}

#[tracing::instrument(skip(state))]
pub async fn handler(State(state): State<ServiceState>) -> Response {
    let node_id = state.peer().id().to_string();
    (StatusCode::OK, Json(IdentityResponse { node_id })).into_response()
}
