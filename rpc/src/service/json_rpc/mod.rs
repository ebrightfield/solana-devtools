use crate::json_rpc::stats_updater::TransportStats;
use crate::service::{RpcSenderRequest, RpcSenderResponse};
use log::debug;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE, RETRY_AFTER};
use reqwest::{Client, Response, StatusCode};
use serde::Deserialize;
use serde_json::{json, Value};
use solana_client::client_error::ClientError;
use solana_client::rpc_custom_error;
use solana_client::rpc_request::{RpcError, RpcResponseErrorData};
use solana_client::rpc_response::RpcSimulateTransactionResult;
use stats_updater::StatsUpdater;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::time::sleep;
use tower::Service;

pub mod stats_updater;

const JSON_RPC: &'static str = "2.0";

/// Helper struct for easier decoding of the `"error"` field in an RPC response.
#[derive(Deserialize, Debug)]
struct RpcErrorObject {
    pub code: i64,
    pub message: String,
}

/// The innermost service for a layered service that implements `RpcSender`.
/// This contains the basic implementation of `solana_rpc_client::http_sender::HttpSender`.
#[derive(Debug)]
pub struct HttpClientService {
    pub client: Arc<Client>,
    pub url: String,
    pub request_id: AtomicU64,
    pub stats: Arc<RwLock<TransportStats>>,
}

impl HttpClientService {
    pub fn new<U: ToString>(url: U) -> Self {
        Self::new_with_timeout(url, Duration::from_secs(30), None)
    }

    pub fn new_with_client<U: ToString>(url: U, client: Client) -> Self {
        Self {
            client: Arc::new(client),
            url: url.to_string(),
            request_id: AtomicU64::new(0),
            stats: Default::default(),
        }
    }

    pub fn new_with_headers<U: ToString>(url: U, headers: Option<HeaderMap>) -> Self {
        Self::new_with_timeout(url, Duration::from_secs(30), headers)
    }

    pub fn new_with_timeout<U: ToString>(
        url: U,
        timeout: Duration,
        headers: Option<HeaderMap>,
    ) -> Self {
        let mut default_headers = HeaderMap::new();
        default_headers.append(
            HeaderName::from_static("solana-client"),
            HeaderValue::from_str(format!("rust/{}", solana_version::Version::default()).as_str())
                .unwrap(),
        );
        if let Some(headers) = headers {
            default_headers.extend(headers);
        }

        let client = Arc::new(
            Client::builder()
                .default_headers(default_headers)
                .timeout(timeout)
                .pool_idle_timeout(timeout)
                .build()
                .expect("reqwest client"),
        );

        Self {
            client,
            url: url.to_string(),
            request_id: AtomicU64::new(0),
            stats: Default::default(),
        }
    }
}

impl Service<RpcSenderRequest> for HttpClientService {
    type Response = Value;
    type Error = ClientError;

    type Future = Pin<Box<(dyn Future<Output = RpcSenderResponse> + Send)>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: RpcSenderRequest) -> Self::Future {
        let (request, params) = req;
        let mut stats_updater = StatsUpdater::new(self.stats.clone());

        let request_id = self.request_id.fetch_add(1, Ordering::Relaxed);
        let request_json = json!({
           "jsonrpc": JSON_RPC,
           "id": request_id,
           "method": format!("{}", request),
           "params": params,
        })
        .to_string();
        let client = self.client.clone();
        let url = self.url.clone();

        Box::pin(async move {
            let mut too_many_requests_retries = 5;
            loop {
                let response = {
                    let request_json = request_json.clone();
                    client
                        .post(&url)
                        .header(CONTENT_TYPE, "application/json")
                        .body(request_json)
                        .send()
                        .await
                }?;

                if !response.status().is_success() {
                    if response.status() == StatusCode::TOO_MANY_REQUESTS
                        && too_many_requests_retries > 0
                    {
                        let mut duration = Duration::from_millis(500);
                        if let Some(retry_after) = response.headers().get(RETRY_AFTER) {
                            if let Ok(retry_after) = retry_after.to_str() {
                                if let Ok(retry_after) = retry_after.parse::<u64>() {
                                    if retry_after < 120 {
                                        duration = Duration::from_secs(retry_after);
                                    }
                                }
                            }
                        }

                        too_many_requests_retries -= 1;
                        debug!(
                                "Too many requests: server responded with {:?}, {} retries left, pausing for {:?}",
                                response, too_many_requests_retries, duration
                            );

                        sleep(duration).await;
                        stats_updater.add_rate_limited_time(duration);
                        continue;
                    }
                    return Err(response.error_for_status().unwrap_err().into());
                }
                return to_solana_rpc_result(response).await;
            }
        })
    }
}

/// Convert Reqwest responses and errors to the types
/// required by higher-level Solana client code.
pub async fn to_solana_rpc_result(resp: Response) -> RpcSenderResponse {
    let mut json = resp.json::<Value>().await?;
    if json["error"].is_object() {
        return match serde_json::from_value::<RpcErrorObject>(json["error"].clone()) {
            Ok(rpc_error_object) => {
                let data = match rpc_error_object.code {
                    rpc_custom_error::JSON_RPC_SERVER_ERROR_SEND_TRANSACTION_PREFLIGHT_FAILURE => {
                        match serde_json::from_value::<RpcSimulateTransactionResult>(
                            json["error"]["data"].clone(),
                        ) {
                            Ok(data) => RpcResponseErrorData::SendTransactionPreflightFailure(data),
                            Err(err) => {
                                debug!(
                                    "Failed to deserialize RpcSimulateTransactionResult: {:?}",
                                    err
                                );
                                RpcResponseErrorData::Empty
                            }
                        }
                    }
                    rpc_custom_error::JSON_RPC_SERVER_ERROR_NODE_UNHEALTHY => {
                        match serde_json::from_value::<rpc_custom_error::NodeUnhealthyErrorData>(
                            json["error"]["data"].clone(),
                        ) {
                            Ok(rpc_custom_error::NodeUnhealthyErrorData { num_slots_behind }) => {
                                RpcResponseErrorData::NodeUnhealthy { num_slots_behind }
                            }
                            Err(_err) => RpcResponseErrorData::Empty,
                        }
                    }
                    _ => RpcResponseErrorData::Empty,
                };

                Err(RpcError::RpcResponseError {
                    code: rpc_error_object.code,
                    message: rpc_error_object.message,
                    data,
                }
                .into())
            }
            Err(err) => Err(RpcError::RpcRequestError(format!(
                "Failed to deserialize RPC error response: {} [{}]",
                serde_json::to_string(&json["error"]).unwrap(),
                err
            ))
            .into()),
        };
    }
    return Ok(json["result"].take());
}
