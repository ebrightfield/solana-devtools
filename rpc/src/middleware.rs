use crate::service::{RpcSenderRequest, RpcSenderResponse};
use serde_json::Value;
use solana_client::client_error::ClientError;
use solana_client::rpc_request::RpcRequest;
use std::future::{ready, Future};
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::Service;

/// Filter Solana RPC requests, and conditionally return an error.
/// Takes a function that takes the request method and params as input,
/// and returns a [Result<(), solana_client::client_error::ClientError].
/// If this function returns `Ok(())`, then the request is forwarded. Otherwise,
/// the error is returned as the response.
#[derive(Debug)]
pub struct FilterMiddleware<S, F> {
    inner: S,
    filter_func: F,
}

impl<S, F> FilterMiddleware<S, F> {
    pub fn new(s: S, f: F) -> Self {
        Self {
            inner: s,
            filter_func: f,
        }
    }
}

impl<S, F> Service<RpcSenderRequest> for FilterMiddleware<S, F>
where
    S: Service<
            RpcSenderRequest,
            Future = Pin<Box<(dyn Future<Output = RpcSenderResponse> + Send)>>,
        > + Send
        + Sync,
    F: for<'a> Fn(&'a RpcRequest, &'a Value) -> Result<(), ClientError>,
{
    type Response = Value;
    type Error = ClientError;

    type Future = Pin<Box<(dyn Future<Output = RpcSenderResponse> + Send)>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: RpcSenderRequest) -> Self::Future {
        match (self.filter_func)(&req.0, &req.1) {
            Ok(_) => self.inner.call(req),
            Err(e) => Box::pin(ready(Err(e))),
        }
    }
}
