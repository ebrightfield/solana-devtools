use crate::json_rpc::{
    jsonrpc_request_body, rust_version, RpcErrorObject, APPLICATION_JSON, SOLANA_CLIENT,
};
use crate::service::json_rpc::RpcSenderRequest;
use futures::future::BoxFuture;
use futures::FutureExt;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE};
use reqwest::Client;
use serde_json::Value;
use solana_client::client_error::ClientError;
use solana_client::rpc_request::RpcRequest;
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;
use tower::{BoxError, Layer, Service};
use tracing::Instrument;

use super::jsonrpc_to_solanarpc;

/// A vanilla [reqwest] based [RpcSender].
#[derive(Debug, Default)]
pub struct ReqwestRpcSender {
    request_id: AtomicU64,
    headers: HeaderMap,
    timeout: Duration,
    url: String,
}

impl Clone for ReqwestRpcSender {
    fn clone(&self) -> Self {
        Self {
            request_id: AtomicU64::new(self.request_id.load(Ordering::Relaxed)),
            url: self.url.clone(),
            headers: self.headers.clone(),
            timeout: self.timeout.clone(),
        }
    }
}

impl ReqwestRpcSender {
    pub fn new(url: String) -> Self {
        Self {
            request_id: AtomicU64::new(0),
            headers: HeaderMap::new(),
            timeout: Duration::from_secs(30),
            url,
        }
    }

    fn call_inner(
        &self,
        method: RpcRequest,
        params: Value,
    ) -> impl Future<Output = Result<Value, BoxError>> + Send + 'static {
        let request_id = self.request_id.fetch_add(1, Ordering::Relaxed);
        let timeout = Duration::from_secs(30);
        let mut headers = HeaderMap::new();
        headers.append(
            HeaderName::from_static(SOLANA_CLIENT),
            HeaderValue::from_str(&rust_version()).unwrap(),
        );
        headers.append(CONTENT_TYPE, HeaderValue::from_static(APPLICATION_JSON));
        headers.extend(self.headers.clone());
        let url = self.url.clone();
        let client = Client::builder()
            .default_headers(headers)
            .timeout(timeout)
            .pool_idle_timeout(timeout)
            .build()
            .expect("reqwest client");
        let span = tracing::info_span!("http_jsonrpc_request", ?method, ?params, request_id);
        async move {
            let jsonrpc_request = jsonrpc_request_body(method.to_string(), params, request_id);
            tracing::info!(?jsonrpc_request);
            let http_response = Box::pin(client.post(&url).body(jsonrpc_request).send());
            WrappedReqwestFuture {
                http_response,
                http_response_body: None,
            }
            .await
        }
        .instrument(span)
    }
}
impl Service<RpcSenderRequest> for ReqwestRpcSender {
    type Response = Value;
    type Error = BoxError;

    type Future = BoxFuture<'static, Result<Value, BoxError>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: RpcSenderRequest) -> Self::Future {
        let (req, params) = req;
        let fut = self.call_inner(req, params);
        Box::pin(fut)
    }
}

pub struct WrappedReqwestFuture {
    // The response body is awaited and parsed as JSON-RPC output after this
    http_response: Pin<Box<dyn Future<Output = Result<reqwest::Response, reqwest::Error>> + Send>>,
    // The error here is converted before the future returns
    http_response_body: Option<Pin<Box<dyn Future<Output = Result<Value, reqwest::Error>> + Send>>>,
}

impl Future for WrappedReqwestFuture {
    type Output = Result<Value, BoxError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(resp) = &mut self.http_response_body {
            match resp.poll_unpin(cx) {
                Poll::Pending => {
                    return Poll::Pending;
                }
                Poll::Ready(r) => {
                    return Poll::Ready(match r {
                        Ok(value) => jsonrpc_to_solanarpc(value),
                        Err(e) => Err(Box::new(ClientError::from(e)) as BoxError),
                    });
                }
            }
        }
        match self.http_response.poll_unpin(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(r) => match r {
                Ok(r) => {
                    tracing::info!("{:?}", r);
                    self.http_response_body = Some(Box::pin(r.json()));
                    self.poll(cx)
                }
                Err(e) => {
                    tracing::error!(jsonrpc_error=?e);
                    Poll::Ready(Err(e.into()))
                }
            },
        }
    }
}

pub async fn parse_response_body(response: reqwest::Response) -> Result<Value, BoxError> {
    tracing::info!("{:?}", response);
    match response.json().await {
        Err(e) => {
            tracing::error!(http_error=?e);
            Err(Box::new(e) as BoxError)
        }
        Result::<Value, _>::Ok(mut json) => {
            if json["error"].is_object() {
                tracing::error!(jsonrpc_error = ?json);
                return RpcErrorObject::parse_value(json["error"].take());
            }
            tracing::info!(jsonrpc_response=?json);
            Ok(json["result"].take())
        }
    }
}

pub struct ParseResponseBodyLayer;

impl<S> Layer<S> for ParseResponseBodyLayer {
    type Service = ParseResponseBody<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ParseResponseBody { inner }
    }
}

#[derive(Debug, Clone)]
pub struct ParseResponseBody<T> {
    inner: T,
}

impl<S, Request, E, F> Service<Request> for ParseResponseBody<S>
where
    S: Service<Request, Error = E, Future = F>,
    S::Error: std::error::Error + Send + Sync + 'static,
    F: Future<Output = Result<reqwest::Response, reqwest::Error>> + Send,
{
    type Response = Value;
    type Error = BoxError;
    type Future = ParseResponseFuture<F>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner
            .poll_ready(cx)
            .map_err(|e| Box::new(e) as BoxError)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let fut = self.inner.call(request);
        ParseResponseFuture::new(fut)
    }
}

pub struct ParseResponseFuture<F> {
    // The response body is awaited and parsed as JSON-RPC output after this
    inner_fut: Pin<Box<F>>,
    http_response_body: Option<Pin<Box<dyn Future<Output = Result<Value, reqwest::Error>> + Send>>>,
}

impl<F> ParseResponseFuture<F> {
    pub fn new(fut: F) -> Self {
        Self {
            inner_fut: Box::pin(fut),
            http_response_body: None,
        }
    }
}

impl<F> Future for ParseResponseFuture<F>
where
    F: Future<Output = Result<reqwest::Response, reqwest::Error>> + Send,
{
    type Output = Result<Value, BoxError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(resp) = &mut self.http_response_body {
            match resp.poll_unpin(cx) {
                Poll::Pending => {
                    return Poll::Pending;
                }
                Poll::Ready(r) => {
                    return Poll::Ready(match r {
                        Err(e) => {
                            tracing::error!(http_error=?e);
                            Err(Box::new(e) as BoxError)
                        }
                        Result::<Value, _>::Ok(mut r) => {
                            if r["error"].is_object() {
                                tracing::error!(jsonrpc_error = ?r);
                                return Poll::Ready(RpcErrorObject::parse_value(r["error"].take()));
                            }
                            tracing::info!(jsonrpc_response=?r);
                            Ok(r["result"].take())
                        }
                    });
                }
            }
        }
        match self.inner_fut.poll_unpin(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(r) => match r {
                Ok(r) => {
                    tracing::info!("{:?}", r);
                    self.http_response_body = Some(Box::pin(r.json()));
                    self.poll(cx)
                }
                Err(e) => {
                    tracing::error!(jsonrpc_error=?e);
                    Poll::Ready(Err(e.into()))
                }
            },
        }
    }
}
