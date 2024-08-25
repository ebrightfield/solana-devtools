use crate::service::json_rpc::{RpcSenderRequest, RpcSenderResponse};
use reqwest::header::RETRY_AFTER;
use reqwest::StatusCode;
use serde_json::Value;
use solana_client::client_error::{ClientError, ClientErrorKind};
use solana_client::rpc_request::RpcRequest;
use std::future::{ready, Future};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::time::sleep;
use tower::{retry, BoxError, Service};

/// Filter Solana RPC requests, and conditionally return an error.
/// Takes a function that takes the request method and params as input,
/// and returns a [Result<(), solana_client::client_error::ClientError].
/// If this function returns `Ok(())`, then the request is forwarded. Otherwise,
/// the error is returned as the response.
#[derive(Debug)]
pub struct RpcSenderFilter<S, F> {
    inner: S,
    filter_func: F,
}

impl<S, F> RpcSenderFilter<S, F> {
    pub fn new(s: S, f: F) -> Self {
        Self {
            inner: s,
            filter_func: f,
        }
    }
}

impl<S, F> Service<RpcSenderRequest> for RpcSenderFilter<S, F>
where
    S: Service<
            RpcSenderRequest,
            Future = Pin<Box<(dyn Future<Output = RpcSenderResponse> + Send)>>,
        > + Send
        + Sync,
    F: for<'a> Fn(&'a RpcRequest, &'a Value) -> Result<(), BoxError>,
{
    type Response = Value;
    type Error = BoxError;

    type Future = Pin<Box<(dyn Future<Output = Result<Value, BoxError>> + Send)>>;

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
    S: Service<
            RpcSenderRequest,
            Future = Pin<Box<(dyn Future<Output = RpcSenderResponse> + Send)>>,
        > + Send
        + Sync,
    F: for<'a> Fn(&'a RpcRequest, &'a Value) -> Option<RpcSenderResponse>,
{
    type Response = Value;
    type Error = BoxError;

    type Future = Pin<Box<(dyn Future<Output = Result<Value, BoxError>> + Send)>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: RpcSenderRequest) -> Self::Future {
        match (self.f)(&req.0, &req.1) {
            None => self.inner.call(req),
            Some(result) => Box::pin(ready(result)),
        }
    }
}

pub struct TooManyRequestsRetry {
    pub num_retries: usize,
    curr_attempt: usize,
}

// impl retry::Policy<RpcSenderRequest, Value, BoxError> for TooManyRequestsRetry {
//     type Future = Pin<Box<(dyn Future<Output = Result<Value, BoxError>> + Send)>>;

//     fn retry(
//         &mut self,
//         req: &mut RpcSenderRequest,
//         result: &mut Result<Value, BoxError>,
//     ) -> Option<Self::Future> {
//         match result {
//             Ok(res) => return None,
//             Err(e) => {
//                 if let Some(ClientError {
//                     kind: ClientErrorKind::Reqwest(http_err),
//                     ..
//                 }) = e.downcast_ref::<ClientError>()
//                 {
//                     if http_err.status().unwrap_or_default() != StatusCode::TOO_MANY_REQUESTS {
//                         return None;
//                     }
//                     if let Some(retry_after) = http_err.headers().get(RETRY_AFTER) {
//                         if let Ok(retry_after) = retry_after.to_str() {
//                             if let Ok(retry_after) = retry_after.parse::<u64>() {
//                                 if retry_after < 120 {
//                                     duration = Duration::from_secs(retry_after);
//                                 }
//                             }
//                         }
//                     }

//                     self.curr_attempt += 1;
//                     tracing::debug!(
//                             "Too many requests: server responded with {:?}, {} retries left, pausing for {:?}",
//                             response, self.curr_attempt, duration
//                         );

//                     Some(sleep(duration))
//                 }
//             }
//         }
//         None
//         // if !response.status().is_success() {
//         //     if response.status() == StatusCode::TOO_MANY_REQUESTS
//         //         && too_many_requests_retries > 0
//         //     {
//         //         let mut duration = Duration::from_millis(500);
//         //         if let Some(retry_after) = response.headers().get(RETRY_AFTER) {
//         //             if let Ok(retry_after) = retry_after.to_str() {
//         //                 if let Ok(retry_after) = retry_after.parse::<u64>() {
//         //                     if retry_after < 120 {
//         //                         duration = Duration::from_secs(retry_after);
//         //                     }
//         //                 }
//         //             }
//         //         }

//         //         too_many_requests_retries -= 1;
//         //         debug!(
//         //                     "Too many requests: server responded with {:?}, {} retries left, pausing for {:?}",
//         //                     response, too_many_requests_retries, duration
//         //                 );

//         //         sleep(duration).await;
//         //         stats_updater.add_rate_limited_time(duration);
//         //         continue;
//         //     }
//         //     return Err(response.error_for_status().unwrap_err().into());
//         // }
//     }

//     fn clone_request(&mut self, req: &RpcSenderRequest) -> Option<RpcSenderRequest> {
//         todo!()
//     }
// }
