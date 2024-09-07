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
use serde_json::{json, Value};
use tower::{BoxError, Layer, Service};

pub mod parse_response_body;
pub mod rpc_client_sender;
pub mod stats_updater;

pub use rpc_client_sender::RpcClientSender;
use solana_client::rpc_request::RpcRequest;

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

pub struct HttpRequestBuilderLayer {
    headers: HeaderMap,
    timeout: Duration,
    url: Url,
}

impl HttpRequestBuilderLayer {
    pub fn new(url: Url) -> Self {
        Self {
            headers: Default::default(),
            timeout: Duration::from_secs(30),
            url,
        }
    }

    pub fn with_header(mut self, k: HeaderName, v: HeaderValue) -> Self {
        self.headers.append(k, v);
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

impl<S> Layer<S> for HttpRequestBuilderLayer {
    type Service = HttpRequestBuilderService<S>;

    fn layer(&self, service: S) -> Self::Service {
        HttpRequestBuilderService::new(
            service,
            self.url.clone(),
            Some(self.timeout),
            Some(self.headers.clone()),
        )
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
