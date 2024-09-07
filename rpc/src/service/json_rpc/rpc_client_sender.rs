use crate::json_rpc::stats_updater::{StatsUpdater, TransportStats};
use crate::middleware::TooManyRequestsRetry;
use crate::service::json_rpc::{RpcSenderRequest, RpcSenderResponse};
use reqwest::Url;
use serde_json::Value;
use solana_client::client_error::{ClientError, ClientErrorKind};
use solana_client::rpc_request::RpcRequest;
use solana_client::rpc_sender::{RpcSender, RpcTransportStats};
use std::ops::Deref;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tower::retry::Retry;
use tower::{BoxError, Layer, Service, ServiceBuilder, ServiceExt};

use super::parse_response_body::{ParseResponseBody, ParseResponseBodyLayer};
use super::{HttpRequestBuilderLayer, HttpRequestBuilderService};

#[tracing::instrument(skip_all)]
async fn process_requests<S>(
    mut rpc_sender_service: S,
    stats: Arc<RwLock<TransportStats>>,
    mut rx: mpsc::UnboundedReceiver<(RpcSenderRequest, oneshot::Sender<RpcSenderResponse>)>,
) -> S
where
    S: tower::Service<RpcSenderRequest, Response = Value, Error = BoxError>,
    S::Future: Send + 'static,
{
    loop {
        match rx.recv().await {
            Some((value, tx)) => {
                let (method, params) = value;
                if let Err(e) = rpc_sender_service.ready().await {
                    tracing::error!(err=?e);
                    return rpc_sender_service;
                }
                let fut = rpc_sender_service.call((method, params));
                let stats_clone = stats.clone();
                // On Drop::drop(), time is recorded
                tokio::spawn(async move {
                    let _stats_updater = StatsUpdater::new(stats_clone);
                    if let Err(e) = tx.send(fut.await) {
                        tracing::error!(err=?e);
                        return;
                    }
                });
            }
            None => break,
        }
    }
    tracing::error!("terminating request processing routine");
    rpc_sender_service
}

// Top level service struct.
pub struct RpcClientSender<T> {
    pub handle: JoinHandle<T>,
    stats: Arc<RwLock<TransportStats>>,
    url: String,
    tx: UnboundedSender<(RpcSenderRequest, oneshot::Sender<RpcSenderResponse>)>,
}

impl RpcClientSender<DefaultHttpService> {
    pub fn new_http(url: Url) -> Self {
        let service = default_http_service(url.clone());
        let (tx, rx) =
            mpsc::unbounded_channel::<(RpcSenderRequest, oneshot::Sender<RpcSenderResponse>)>();
        let stats = Arc::new(RwLock::new(TransportStats::default()));
        let handle = tokio::spawn(process_requests(service, stats.clone(), rx));
        Self {
            handle,
            url: url.to_string(),
            stats,
            tx,
        }
    }
}

impl<S> RpcClientSender<S>
where
    S: Service<RpcSenderRequest, Response = Value, Error = BoxError> + Send + 'static,
    S::Future: Send + 'static,
{
    pub fn new_with_service(url: String, service: S) -> Self {
        let (tx, rx) =
            mpsc::unbounded_channel::<(RpcSenderRequest, oneshot::Sender<RpcSenderResponse>)>();
        let stats = Arc::new(RwLock::new(TransportStats::default()));
        let handle = tokio::spawn(process_requests(service, stats.clone(), rx));
        Self {
            handle,
            url: url.clone(),
            stats,
            tx,
        }
    }

    pub fn new_from_builder<L, U>(url: String, builder: ServiceBuilder<L>, inner: U) -> Self
    where
        L: Layer<U, Service = S>,
    {
        let service = builder.service(inner);
        Self::new_with_service(url, service)
    }

    pub fn new_http_from_builder<L>(builder: ServiceBuilder<L>, url: Url) -> Self
    where
        L: Layer<DefaultHttpService, Service = S>,
    {
        let service = builder.service(default_http_service(url.clone()));
        let (tx, rx) =
            mpsc::unbounded_channel::<(RpcSenderRequest, oneshot::Sender<RpcSenderResponse>)>();
        let stats = Arc::new(RwLock::new(TransportStats::default()));
        let handle = tokio::spawn(process_requests(service, stats.clone(), rx));
        Self {
            handle,
            url: url.to_string(),
            stats,
            tx,
        }
    }
}

#[async_trait::async_trait]
impl<T> RpcSender for RpcClientSender<T>
where
    T: tower::Service<RpcSenderRequest, Response = Value, Error = BoxError> + Send + 'static,
    T::Future: Send + 'static,
{
    async fn send(
        &self,
        request: RpcRequest,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, ClientError> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(((request, params), tx)).map_err(|e| {
            ClientError::new_with_request(ClientErrorKind::Custom(format!("{e}")), request)
        })?;
        let resp = rx.await.map_err(|e| {
            ClientError::new_with_request(ClientErrorKind::Custom(format!("{e}")), request)
        })?;
        let client_resp = resp.map_err(|e| match e.downcast::<ClientError>() {
            Ok(client_error) => *client_error,
            Err(e) => {
                ClientError::new_with_request(ClientErrorKind::Custom(format!("{e}")), request)
            }
        });
        tracing::info!(rpc_sender_return=?client_resp);
        client_resp
    }

    fn get_transport_stats(&self) -> RpcTransportStats {
        self.stats.read().unwrap().deref().into()
    }

    fn url(&self) -> String {
        self.url.clone()
    }
}
pub type DefaultHttpService =
    ParseResponseBody<HttpRequestBuilderService<Retry<TooManyRequestsRetry, reqwest::Client>>>;

pub fn default_http_service(url: Url) -> DefaultHttpService {
    ServiceBuilder::new()
        .layer(ParseResponseBodyLayer)
        .layer(HttpRequestBuilderLayer::new(url))
        .retry(TooManyRequestsRetry::new(4))
        .service(reqwest::Client::builder().build().unwrap())
}
