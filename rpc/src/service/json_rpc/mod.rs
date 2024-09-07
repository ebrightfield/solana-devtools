use std::{
    future::Future,
    sync::atomic::{AtomicU64, Ordering},
    task::{Context, Poll},
    time::Duration,
};

use futures::future::BoxFuture;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE},
    Method, Url,
};
use serde::Deserialize;
use serde_json::{json, Value};
use tower::{BoxError, Layer, Service};

pub mod reqwest_client;
pub mod rpc_client_sender;
pub mod stats_updater;

pub use reqwest_client::ReqwestRpcSender;
pub use rpc_client_sender::RpcClientSender;
use solana_client::{
    rpc_custom_error::{
        NodeUnhealthyErrorData, JSON_RPC_SERVER_ERROR_NODE_UNHEALTHY as NODE_UNHEALTHY,
        JSON_RPC_SERVER_ERROR_SEND_TRANSACTION_PREFLIGHT_FAILURE as PREFLIGHT_FAILURE,
    },
    rpc_request::{RpcError, RpcRequest, RpcResponseErrorData},
    rpc_response::RpcSimulateTransactionResult,
};

/// The data types sent to `RpcSender::send`, grouped into a tuple.
pub type RpcSenderRequest = (RpcRequest, Value);
pub use solana_client::client_error::ClientError as SolanaClientError;

use tower::BoxError as ClientError;

pub type RpcSenderResult<T> = Result<T, ClientError>;
/// The response type to `RpcSender::send`.
pub type RpcSenderResponse = RpcSenderResult<Value>;
/// The return type of an RpcSenderService
pub type RpcSenderResponseFuture =
    BoxFuture<'static, dyn Future<Output = RpcSenderResponse> + Send>;

/// Marker trait for anything that implements the [tower::Service] trait with
/// the appropriate request and response types.
///
/// Any type that implements this trait can be wrapped in an [RpcClientSender]
/// and inherit the [RpcSender] trait as a consequence.
/// This allows one to make full use of the tower Service interface to compose
/// custom middleware, mocked return values, caches, retry mechanisms, and much more.
///
/// See [ReqwestRpcSender] for an example implementation of the [tower::Service] trait.
pub trait RpcSenderService:
    tower::Service<
        RpcSenderRequest,
        Error = BoxError,
        Future = BoxFuture<'static, Result<Value, BoxError>>,
    > + Send
    + Sync
{
}

impl<T> RpcSenderService for T where
    T: tower::Service<
            RpcSenderRequest,
            Error = BoxError,
            Future = BoxFuture<'static, Result<Value, BoxError>>,
        > + Send
        + Sync
{
}

pub(crate) const JSON_RPC: &'static str = "2.0";
pub(crate) const APPLICATION_JSON: &'static str = "application/json";
pub(crate) const SOLANA_CLIENT: &'static str = "solana-client";

pub(crate) fn rust_version() -> String {
    format!("rust/{}", solana_version::Version::default())
}

pub(crate) fn jsonrpc_request_body(method: String, params: Value, request_id: u64) -> String {
    json!({
       "jsonrpc": JSON_RPC,
       "id": request_id,
       "method": method.to_string(),
       "params": params,
    })
    .to_string()
}

/// Helper struct for easier decoding of the `"error"` field in an RPC response.
#[derive(Deserialize, Debug)]
struct RpcErrorObject {
    pub code: i64,
    pub message: String,
}

impl RpcErrorObject {
    /// Certain special values get dedicating checking and parsing routines.
    fn parse_value(json: Value) -> RpcSenderResponse {
        let rpc_error_object =
            serde_json::from_value::<RpcErrorObject>(json.clone()).map_err(|e| {
                RpcError::RpcRequestError(format!(
                    "Failed to deserialize RPC error response: {} [{}]",
                    serde_json::to_string(&json).unwrap(),
                    e
                ))
            })?;
        let data = match rpc_error_object.code {
            PREFLIGHT_FAILURE => {
                match serde_json::from_value::<RpcSimulateTransactionResult>(json["data"].clone()) {
                    Ok(data) => RpcResponseErrorData::SendTransactionPreflightFailure(data),
                    Err(err) => {
                        tracing::debug!(
                            "Failed to deserialize RpcSimulateTransactionResult: {:?}",
                            err
                        );
                        RpcResponseErrorData::Empty
                    }
                }
            }
            NODE_UNHEALTHY => {
                let err_data: Result<NodeUnhealthyErrorData, _> =
                    serde_json::from_value(json["data"].clone());
                if let Ok(NodeUnhealthyErrorData { num_slots_behind }) = err_data {
                    RpcResponseErrorData::NodeUnhealthy { num_slots_behind }
                } else {
                    RpcResponseErrorData::Empty
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
}

/// Parse a generic JSON-RPC response by either:
/// - Extracting the "result" field from a successful response, or
/// - Parsing the "error" field from an error response
#[tracing::instrument]
pub fn jsonrpc_to_solanarpc(mut json: Value) -> RpcSenderResponse {
    if json["error"].is_object() {
        tracing::error!(jsonrpc_error = ?json);
        return RpcErrorObject::parse_value(json["error"].take());
    }
    tracing::info!(jsonrpc_response=?json);
    Ok(json["result"].take())
}

/// Parse the error value from a `Reqwest`
pub struct JsonRpcToSolanaRpc;

// 0. Reqwest Clients already implement Service! That's dope.
// 1. A layer to build `Request` objects with JSON-RPC payload, headers, etc.
//    - Stores Request ID incrementer, additional headers, timeout, URL
// 2. A layer to convert JSON-RPC Error, and start working with a BoxError.

/// Converts a Solana RPC request + params into an HTTP JSON-RPC request,
/// served with [Rewqest]
#[derive(Debug, Clone)]
pub struct HttpRequestConfigLayer {
    pub headers: HeaderMap,
    pub timeout: Duration,
    pub url: Url,
}

impl HttpRequestConfigLayer {
    pub fn new(url: impl AsRef<str>) -> Result<Self, BoxError> {
        let url = Url::parse(url.as_ref())?;

        let timeout = Duration::from_secs(30);

        let mut headers = HeaderMap::new();
        headers.append(
            HeaderName::from_static(SOLANA_CLIENT),
            HeaderValue::from_str(&rust_version()).unwrap(),
        );
        headers.append(CONTENT_TYPE, HeaderValue::from_static(APPLICATION_JSON));
        Ok(Self {
            headers,
            timeout,
            url,
        })
    }
}

impl<S> Layer<S> for HttpRequestConfigLayer {
    type Service = HttpRequestBuilderService<S>;

    fn layer(&self, service: S) -> Self::Service {
        HttpRequestBuilderService {
            service,
            request_id: AtomicU64::new(0),
            headers: self.headers.clone(),
            timeout: self.timeout.clone(),
            url: self.url.clone(),
        }
    }
}

pub struct HttpRequestBuilderLayer {
    request_id: AtomicU64,
    headers: HeaderMap,
    timeout: Duration,
    url: Url,
}

impl HttpRequestBuilderLayer {
    pub fn new(url: Url) -> Self {
        let timeout = Duration::from_secs(30);

        let mut headers = HeaderMap::new();
        headers.append(
            HeaderName::from_static(SOLANA_CLIENT),
            HeaderValue::from_str(&rust_version()).unwrap(),
        );
        headers.append(CONTENT_TYPE, HeaderValue::from_static(APPLICATION_JSON));
        Self {
            request_id: AtomicU64::new(0),
            headers,
            timeout,
            url,
        }
    }
}

impl<S> Layer<S> for HttpRequestBuilderLayer {
    type Service = HttpRequestBuilderService<S>;

    fn layer(&self, service: S) -> Self::Service {
        HttpRequestBuilderService {
            service,
            request_id: AtomicU64::new(0),
            headers: self.headers.clone(),
            timeout: self.timeout.clone(),
            url: self.url.clone(),
        }
    }
}

/// Service for layering in configuration to a [reqwest::Request]
/// and constructing the JSON-RPC body.
pub struct HttpRequestBuilderService<S> {
    service: S,
    request_id: AtomicU64,
    headers: HeaderMap,
    timeout: Duration,
    url: Url,
}

impl<S> HttpRequestBuilderService<S> {
    pub fn new(
        service: S,
        url: Url,
        timeout: Option<Duration>,
        headers: Option<HeaderMap>,
    ) -> Self {
        let mut headers = headers.unwrap_or_default();
        if headers.get(SOLANA_CLIENT).is_none() {
            headers.append(
                HeaderName::from_static(SOLANA_CLIENT),
                HeaderValue::from_str(&rust_version()).unwrap(),
            );
        }
        if headers.get(CONTENT_TYPE).is_none() {
            headers.append(CONTENT_TYPE, HeaderValue::from_static(APPLICATION_JSON));
        }
        Self {
            service,
            request_id: AtomicU64::new(0),
            headers,
            timeout: timeout.unwrap_or(Duration::from_secs(30)),
            url,
        }
    }
}

impl<S> Service<RpcSenderRequest> for HttpRequestBuilderService<S>
where
    S: Service<reqwest::Request>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, request: RpcSenderRequest) -> Self::Future {
        let (method, params) = request;
        let request_id = self.request_id.fetch_add(1, Ordering::Relaxed);
        let body = jsonrpc_request_body(method.to_string(), params, request_id);

        let mut headers = HeaderMap::new();
        headers.extend(self.headers.clone());
        let timeout = self.timeout.clone();

        let mut request = reqwest::Request::new(Method::POST, self.url.clone());
        *request.headers_mut() = headers;
        *request.timeout_mut() = Some(timeout);
        *request.body_mut() = Some(body.into());
        self.service.call(request)
    }
}

// pub struct ParseClientErrorLayer;

// impl<S> Layer<S> for ParseClientErrorLayer {
//     type Service = ParseJsonRpcResponseService<S>;

//     fn layer(&self, service: S) -> Self::Service {
//         ParseJsonRpcResponseService { service }
//     }
// }

// // pub struct ParseJsonRpcResponseService<S> {
// //     service: S,
// // }

// // impl<S, T> Service<T> for ParseJsonRpcResponseService<S>
// // where
// //     S: Service<
// //         T,
// //         Response = reqwest::Response,
// //         // Error = BoxError,
// //         // Future = BoxFuture<'static, Result<reqwest::Response, BoxError>>,
// //     >,
// // {
// //     type Response = Value;
// //     type Error = S::Error;
// //     type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

// //     fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
// //         self.service.poll_ready(cx)
// //     }

// //     fn call(&mut self, request: T) -> Self::Future {
// //         // self.service.poll_ready(cx)?;
// //         // Box::pin(async move {
// //         //     let response = self.service.call(request).await?;
// //         //     Ok(Value::Null)
// //         // })
// //         // let (method, params) = request;
// //         // let request_id = self.request_id.fetch_add(1, Ordering::Relaxed);
// //         // let body = jsonrpc_request_body(method.to_string(), params, request_id);

// //         // let mut headers = HeaderMap::new();
// //         // headers.append(
// //         //     HeaderName::from_static(SOLANA_CLIENT),
// //         //     HeaderValue::from_str(&rust_version()).unwrap(),
// //         // );
// //         // headers.append(CONTENT_TYPE, HeaderValue::from_static(APPLICATION_JSON));
// //         // headers.extend(self.headers.clone());
// //         // let timeout = self.timeout.clone();

// //         // let url = Url::from_str(&self.url).expect("RPC URL should parse");
// //         // let mut request = reqwest::Request::new(Method::POST, url);
// //         // *request.headers_mut() = headers;
// //         // *request.timeout_mut() = Some(timeout);
// //         // *request.body_mut() = Some(body.into());
// //         // self.service.call(request)
// //     }
// // }

// // pub struct ParseJsonRpcResponseFuture {
// //     // The response body is awaited and parsed as JSON-RPC output after this
// //     http_response: Pin<Box<dyn Future<Output = Result<reqwest::Response, reqwest::Error>> + Send>>,
// // }

// // impl Future for ParseJsonRpcResponseFuture {
// //     type Output = Result<Value, reqwest::Error>;

// //     fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
// //         match self.http_response.poll_unpin(cx) {
// //             Poll::Pending => {
// //                 return Poll::Pending;
// //             }
// //             Poll::Ready(r) => {
// //                 return Poll::Ready(match r {
// //                     Ok(value) => jsonrpc_to_solanarpc(value),
// //                     Err(e) => Err(e),
// //                 });
// //             }
// //         }
// //     }
// // }
