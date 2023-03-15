/// Copied and modified [solana-client::http_sender::HttpSender], adds the ability
/// to provide default HTTP headers such (Bearer auth tokens) to RpcClient.
/// This has become a feature of using GenesysGo infrastructure.
/// There are also structs that assist in authenticating with a GenesysGo
/// authentication server.
use solana_client;
use solana_client::client_error::reqwest;
use solana_client::client_error::reqwest::header::HeaderMap;
use solana_client::rpc_sender::{RpcSender, RpcTransportStats};
use serde_json::{json, Value};
use solana_client::rpc_request::{RpcError, RpcRequest, RpcResponseErrorData};
use solana_client::rpc_response::RpcSimulateTransactionResult;
use solana_client::rpc_custom_error as custom_error;
use {
    async_trait::async_trait,
    log::*,
    reqwest::{
        header::{self, CONTENT_TYPE, RETRY_AFTER},
        StatusCode,
    },
    std::{
        sync::{
            Arc,
            atomic::{AtomicU64, Ordering}, RwLock,
        },
        time::{Duration, Instant},
    },
    tokio::time::sleep,
};
use serde::Deserialize ;

/// Supporting struct for the [impl RpcSender for HttpSenderWithHeaders] block below.
#[derive(Deserialize, Debug)]
pub struct RpcErrorObject {
    pub code: i64,
    pub message: String,
}

/// Nonblocking [`RpcSender`] over HTTP, with optional custom headers.
/// Modified version of [solana_client::http_sender::HttpSender].
pub struct HttpSenderWithHeaders {
    client: Arc<reqwest::Client>,
    url: String,
    request_id: AtomicU64,
    stats: RwLock<RpcTransportStats>,
}


impl HttpSenderWithHeaders {
    /// Create an HTTP RPC sender.
    ///
    /// The URL is an HTTP URL, usually for port 8899, as in
    /// "http://localhost:8899". The sender has a default timeout of 30 seconds.
    pub fn new<U: ToString>(url: U, headers: Option<HeaderMap>) -> Self {
        Self::new_with_timeout(url, Duration::from_secs(30), headers)
    }

    /// Create an HTTP RPC sender.
    ///
    /// The URL is an HTTP URL, usually for port 8899.
    pub fn new_with_timeout<U: ToString>(url: U, timeout: Duration, headers: Option<HeaderMap>) -> Self {
        let mut default_headers = HeaderMap::new();
        default_headers.append(
            header::HeaderName::from_static("solana-client"),
            header::HeaderValue::from_str(
                format!("rust/{}", solana_version::Version::default()).as_str(),
            )
                .unwrap(),
        );
        if let Some(headers) = headers {
            default_headers.extend(headers);
        }

        let client = Arc::new(
            reqwest::Client::builder()
                .default_headers(default_headers)
                .timeout(timeout)
                .pool_idle_timeout(timeout)
                .build()
                .expect("build rpc client"),
        );

        Self {
            client,
            url: url.to_string(),
            request_id: AtomicU64::new(0),
            stats: RwLock::new(RpcTransportStats::default()),
        }
    }
}

/// Supporting struct for the [impl RpcSender for HttpSenderWithHeaders] block below.
struct StatsUpdater<'a> {
    stats: &'a RwLock<RpcTransportStats>,
    request_start_time: Instant,
    rate_limited_time: Duration,
}

impl<'a> StatsUpdater<'a> {
    fn new(stats: &'a RwLock<RpcTransportStats>) -> Self {
        Self {
            stats,
            request_start_time: Instant::now(),
            rate_limited_time: Duration::default(),
        }
    }

    fn add_rate_limited_time(&mut self, duration: Duration) {
        self.rate_limited_time += duration;
    }
}

impl<'a> Drop for StatsUpdater<'a> {
    fn drop(&mut self) {
        let mut stats = self.stats.write().unwrap();
        stats.request_count += 1;
        stats.elapsed_time += Instant::now().duration_since(self.request_start_time);
        stats.rate_limited_time += self.rate_limited_time;
    }
}

/// Simple way to put together our RPC request for sign-in
pub fn build_request_json(req: &RpcRequest, id: u64, params: Value) -> Value {
    let jsonrpc = "2.0";
    json!({
           "jsonrpc": jsonrpc,
           "id": id,
           "method": format!("{}", req),
           "params": params,
        })
}

/// Allows use in [solana_client::rpc_client::RpcClient::with_rpc_client] by initializing
/// an RpcClient with [nonblocking::rpc_client::RpcClient::new_sender] using the
/// [HttpSenderWithHeaders].
#[async_trait]
impl RpcSender for HttpSenderWithHeaders {
    async fn send(
        &self,
        request: RpcRequest,
        params: Value,
    ) -> solana_client::client_error::Result<Value> {
        let mut stats_updater = StatsUpdater::new(&self.stats);

        let request_id = self.request_id.fetch_add(1, Ordering::Relaxed);
        let request_json = build_request_json(&request, request_id, params).to_string();

        let mut too_many_requests_retries = 5;
        loop {
            let response = {
                let client = self.client.clone();
                let request_json = request_json.clone();
                client
                    .post(&self.url)
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

            let mut json = response.json::<Value>().await?;
            if json["error"].is_object() {
                return match serde_json::from_value::<RpcErrorObject>(json["error"].clone()) {
                    Ok(rpc_error_object) => {
                        let data = match rpc_error_object.code {
                            custom_error::JSON_RPC_SERVER_ERROR_SEND_TRANSACTION_PREFLIGHT_FAILURE => {
                                match serde_json::from_value::<RpcSimulateTransactionResult>(json["error"]["data"].clone()) {
                                    Ok(data) => RpcResponseErrorData::SendTransactionPreflightFailure(data),
                                    Err(err) => {
                                        debug!("Failed to deserialize RpcSimulateTransactionResult: {:?}", err);
                                        RpcResponseErrorData::Empty
                                    }
                                }
                            },
                            custom_error::JSON_RPC_SERVER_ERROR_NODE_UNHEALTHY => {
                                match serde_json::from_value::<custom_error::NodeUnhealthyErrorData>(json["error"]["data"].clone()) {
                                    Ok(custom_error::NodeUnhealthyErrorData {num_slots_behind}) => RpcResponseErrorData::NodeUnhealthy {num_slots_behind},
                                    Err(_err) => {
                                        RpcResponseErrorData::Empty
                                    }
                                }
                            },
                            _ => RpcResponseErrorData::Empty
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
    }

    fn get_transport_stats(&self) -> RpcTransportStats {
        self.stats.read().unwrap().clone()
    }

    fn url(&self) -> String {
        self.url.clone()
    }
}

/// Same tests as in the original [solana_client] crate.
#[cfg(test)]
mod tests {
    use std::thread;
    use reqwest::header::HeaderValue;
    use crossbeam_channel::unbounded;
    use solana_client::rpc_client::RpcClient;
    use jsonrpc_core::{Error, IoHandler, Params};
    use jsonrpc_http_server::{AccessControlAllowOrigin, DomainsValidation, RequestMiddleware, RequestMiddlewareAction, ServerBuilder};
    use serde_json::Number;
    use futures_util::future;
    use jsonrpc_http_server::hyper::{Body, Request};
    use solana_client::client_error::ClientError;
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn http_sender_on_tokio_multi_thread() {
        let http_sender = HttpSenderWithHeaders::new("http://localhost:1234".to_string(), None);
        let _ = http_sender
            .send(RpcRequest::GetVersion, Value::Null)
            .await;
        // And again with headers
        let mut headers = HeaderMap::new();
        headers.insert("foo", HeaderValue::from_str("bar").unwrap());
        let http_sender = HttpSenderWithHeaders::new(
            "http://localhost:1234".to_string(), Some(headers));
        let _ = http_sender
            .send(RpcRequest::GetVersion, Value::Null)
            .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn http_sender_on_tokio_current_thread() {
        let http_sender = HttpSenderWithHeaders::new("http://localhost:1234".to_string(), None);
        let _ = http_sender
            .send(RpcRequest::GetVersion, Value::Null)
            .await;
        // And again with headers
        let mut headers = HeaderMap::new();
        headers.insert("foo", HeaderValue::from_str("bar").unwrap());
        let http_sender = HttpSenderWithHeaders::new(
            "http://localhost:1234".to_string(), Some(headers));
        let _ = http_sender
            .send(RpcRequest::GetVersion, Value::Null)
            .await;
    }

    struct CheckHeaderMiddleware;

    impl RequestMiddleware for CheckHeaderMiddleware {
        fn on_request(&self, request: Request<Body>) -> RequestMiddlewareAction {
            assert_eq!(request.headers().get("foo"), Some(&HeaderValue::from_str("bar").unwrap()));
            RequestMiddlewareAction::Proceed {
                should_continue_on_invalid_cors: false,
                request
            }
        }
    }

    #[test]
    fn test_send() {
        _test_send();
    }

    #[tokio::test(flavor = "current_thread")]
    #[should_panic(expected = "can call blocking only when running on the multi-threaded runtime")]
    async fn test_send_async_current_thread() {
        _test_send();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_send_async_multi_thread() {
        _test_send();
    }

    fn _test_send() {
        let (sender, receiver) = unbounded();
        thread::spawn(move || {
            let rpc_addr = "0.0.0.0:0".parse().unwrap();
            let mut io = IoHandler::default();
            // Successful request
            io.add_method("getBalance", |_params: Params| {
                future::ok(Value::Number(Number::from(50)))
            });
            // Failed request
            io.add_method("getRecentBlockhash", |params: Params| {
                if params != Params::None {
                    future::err(Error::invalid_request())
                } else {
                    future::ok(Value::String(
                        "deadbeefXjn8o3yroDHxUtKsZZgoy4GPkPPXfouKNHhx".to_string(),
                    ))
                }
            });

            let server = ServerBuilder::new(io)
                .threads(1)
                .cors(DomainsValidation::AllowOnly(vec![
                    AccessControlAllowOrigin::Any,
                ]))
                .request_middleware(CheckHeaderMiddleware)
                .start_http(&rpc_addr)
                .expect("Unable to start RPC server");
            sender.send(*server.address()).unwrap();
            server.wait();
        });

        let rpc_addr = receiver.recv().unwrap();
        let rpc_addr = format!("http://{}", rpc_addr);

        let mut default_headers = HeaderMap::new();
        default_headers.insert("foo", HeaderValue::from_str("bar").unwrap());
        let sender = HttpSenderWithHeaders::new(
            rpc_addr,
            Some(default_headers)
        );
        let rpc_client = RpcClient::new_sender(sender, Default::default());

        let balance: u64 = rpc_client
            .send(
                RpcRequest::GetBalance,
                json!(["deadbeefXjn8o3yroDHxUtKsZZgoy4GPkPPXfouKNHhx"]),
            )
            .unwrap();
        assert_eq!(balance, 50);

        #[allow(deprecated)]
            let blockhash: String = rpc_client
            .send(RpcRequest::GetRecentBlockhash, Value::Null)
            .unwrap();
        assert_eq!(blockhash, "deadbeefXjn8o3yroDHxUtKsZZgoy4GPkPPXfouKNHhx");

        // Send erroneous parameter
        #[allow(deprecated)]
            let blockhash: Result<String, ClientError> =
            rpc_client.send(RpcRequest::GetRecentBlockhash, json!(["parameter"]));
        assert!(blockhash.is_err());
    }
}
