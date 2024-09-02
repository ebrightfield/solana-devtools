use crate::service::json_rpc::{RpcSenderRequest, RpcSenderResponse};
use futures::future::BoxFuture;
use reqwest::header::RETRY_AFTER;
use reqwest::StatusCode;
use serde_json::Value;
use solana_client::rpc_request::RpcRequest;
use std::future::ready;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::time::Sleep;
use tower::{retry, BoxError, Service};

#[derive(Debug)]
pub struct RpcSenderMiddleware<S, F> {
    inner: S,
    f: F,
}

impl<S, F> RpcSenderMiddleware<S, F> {
    pub fn new(s: S, f: F) -> Self {
        Self { inner: s, f }
    }
}

impl<S, F> Service<RpcSenderRequest> for RpcSenderMiddleware<S, F>
where
    S: Service<RpcSenderRequest, Response = Value, Error = BoxError>,
    S::Future: Send + 'static,
    F: for<'a> Fn(&'a RpcRequest, &'a Value) -> Option<RpcSenderResponse>,
{
    type Response = Value;
    type Error = BoxError;

    type Future = BoxFuture<'static, Result<Value, BoxError>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // self.inner.poll_ready(cx)
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: RpcSenderRequest) -> Self::Future {
        match (self.f)(&req.0, &req.1) {
            None => Box::pin(self.inner.call(req)),
            Some(result) => Box::pin(ready(result)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TooManyRequestsRetry {
    retries_remaining: usize,
}

impl TooManyRequestsRetry {
    pub fn new(num_retries: usize) -> Self {
        Self {
            retries_remaining: num_retries,
        }
    }
}

impl retry::Policy<reqwest::Request, reqwest::Response, reqwest::Error> for TooManyRequestsRetry {
    type Future = Sleep;

    fn retry(
        &mut self,
        _req: &mut reqwest::Request,
        result: &mut Result<reqwest::Response, reqwest::Error>,
    ) -> Option<Self::Future> {
        if let Ok(response) = result {
            if !response.status().is_success() {
                if response.status() == StatusCode::TOO_MANY_REQUESTS && self.retries_remaining > 0
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

                    self.retries_remaining -= 1;
                    tracing::debug!(
                                "Too many requests: server responded with {:?}, {} retries left, pausing for {:?}",
                                response, self.retries_remaining, duration
                            );

                    // stats_updater.add_rate_limited_time(duration);
                    return Some(tokio::time::sleep(duration));
                }
                // return Err(response.error_for_status().unwrap_err().into());
            }
        }
        None
    }

    fn clone_request(&mut self, req: &reqwest::Request) -> Option<reqwest::Request> {
        let mut request = reqwest::Request::new(req.method().clone(), req.url().clone());
        *request.headers_mut() = req.headers().clone();
        *request.timeout_mut() = req.timeout().copied().clone();
        *request.body_mut() = req
            .body()
            .and_then(|b| b.as_bytes())
            .map(|bytes| bytes.to_vec())
            .map(Into::into);
        Some(request)
    }
}

// // This is cool because then I can name weird shit in my services LOL
// pub type DoThingFuture = impl Future<Output = u64>;

// pub async fn do_thing() -> u64 {
//     0
// }

// pub fn do_thing_inner() -> DoThingFuture {
//     do_thing()
// }
