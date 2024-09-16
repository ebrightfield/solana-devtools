use futures::FutureExt;
use serde::Deserialize;
use serde_json::Value;
use solana_client::{
    rpc_custom_error::{
        NodeUnhealthyErrorData, JSON_RPC_SERVER_ERROR_NODE_UNHEALTHY as NODE_UNHEALTHY,
        JSON_RPC_SERVER_ERROR_SEND_TRANSACTION_PREFLIGHT_FAILURE as PREFLIGHT_FAILURE,
    },
    rpc_request::{RpcError, RpcResponseErrorData},
    rpc_response::RpcSimulateTransactionResult,
};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::{BoxError, Layer, Service};

use super::rpc_sender_impl::SolanaClientResponse;

/// Helper struct for easier decoding of the `"error"` field in an RPC response.
#[derive(Deserialize, Debug)]
struct RpcErrorObject {
    pub code: i64,
    pub message: String,
}

/// Certain special values get dedicating checking and parsing routines.
fn parse_rpc_error(json: Value) -> SolanaClientResponse {
    let rpc_error_object = serde_json::from_value::<RpcErrorObject>(json.clone()).map_err(|e| {
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
                    tracing::warn!(
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

/// Parse a generic JSON-RPC response by either:
/// - Extracting the "result" field from a successful response, or
/// - Parsing the "error" field from an error response
#[tracing::instrument]
pub fn jsonrpc_to_solanarpc(mut json: Value) -> SolanaClientResponse {
    if json["error"].is_object() {
        tracing::error!(jsonrpc_error = ?json);
        return parse_rpc_error(json["error"].take());
    }
    tracing::info!(jsonrpc_response=?json);
    Ok(json["result"].take())
}

pub async fn parse_response_body(response: reqwest::Response) -> Result<Value, BoxError> {
    tracing::info!("{:?}", response);
    match response.json().await {
        Err(e) => {
            tracing::error!(http_error=?e);
            Err(Box::new(e) as BoxError)
        }
        Result::<Value, _>::Ok(json) => jsonrpc_to_solanarpc(json),
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
    http_response_body_fut:
        Option<Pin<Box<dyn Future<Output = Result<Value, reqwest::Error>> + Send>>>,
}

impl<F> ParseResponseFuture<F> {
    pub fn new(fut: F) -> Self {
        Self {
            inner_fut: Box::pin(fut),
            http_response_body_fut: None,
        }
    }
}

impl<F> Future for ParseResponseFuture<F>
where
    F: Future<Output = Result<reqwest::Response, reqwest::Error>> + Send,
{
    type Output = Result<Value, BoxError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(resp) = &mut self.http_response_body_fut {
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
                        Result::<Value, _>::Ok(value) => jsonrpc_to_solanarpc(value),
                    });
                }
            }
        }
        match self.inner_fut.poll_unpin(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(r) => match r {
                Ok(r) => {
                    tracing::info!("{:?}", r);
                    self.http_response_body_fut = Some(Box::pin(r.json()));
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
