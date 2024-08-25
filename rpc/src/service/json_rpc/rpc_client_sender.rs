use crate::json_rpc::stats_updater::{StatsUpdater, TransportStats};
use crate::service::json_rpc::{RpcSenderRequest, RpcSenderResponse};
use solana_client::client_error::{ClientError, ClientErrorKind};
use solana_client::rpc_request::RpcRequest;
use solana_client::rpc_sender::{RpcSender, RpcTransportStats};
use std::ops::Deref;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tower::{Layer, ServiceBuilder, ServiceExt};

use super::reqwest_client::ReqwestRpcSender;
use super::RpcSenderService;

#[tracing::instrument(skip_all)]
async fn process_requests<S>(
    mut rpc_sender_service: S,
    stats: Arc<RwLock<TransportStats>>,
    mut rx: mpsc::UnboundedReceiver<(RpcSenderRequest, oneshot::Sender<RpcSenderResponse>)>,
) -> S
where
    S: RpcSenderService + 'static,
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
    pub request_processing_handle: JoinHandle<T>,
    stats: Arc<RwLock<TransportStats>>,
    url: String,
    tx: UnboundedSender<(RpcSenderRequest, oneshot::Sender<RpcSenderResponse>)>,
}

impl<T> RpcClientSender<T>
where
    T: RpcSenderService + 'static,
{
    pub fn new(service: T, url: String) -> Self {
        let (tx, rx) =
            mpsc::unbounded_channel::<(RpcSenderRequest, oneshot::Sender<RpcSenderResponse>)>();
        let stats = Arc::new(RwLock::new(TransportStats::default()));
        let handle = tokio::spawn(process_requests(service, stats.clone(), rx));
        Self {
            request_processing_handle: handle,
            url: url.clone(),
            stats,
            tx,
        }
    }
    pub fn new_from_builder<L>(url: String, builder: ServiceBuilder<L>) -> Self
    where
        L: Layer<ReqwestRpcSender, Service = T>,
    {
        let service = ReqwestRpcSender::new(url.clone());
        let service = builder.service(service);
        let (tx, rx) =
            mpsc::unbounded_channel::<(RpcSenderRequest, oneshot::Sender<RpcSenderResponse>)>();
        let stats = Arc::new(RwLock::new(TransportStats::default()));
        let handle = tokio::spawn(process_requests(service, stats.clone(), rx));
        Self {
            request_processing_handle: handle,
            url,
            stats: Arc::new(RwLock::new(TransportStats::default())),
            tx,
        }
    }
}

impl RpcClientSender<ReqwestRpcSender> {
    pub fn new_reqwest(url: String) -> Self {
        let inner = ReqwestRpcSender::new(url.clone());
        let (tx, rx) =
            mpsc::unbounded_channel::<(RpcSenderRequest, oneshot::Sender<RpcSenderResponse>)>();
        let stats = Arc::new(RwLock::new(TransportStats::default()));
        let handle = tokio::spawn(process_requests(inner, stats.clone(), rx));
        Self {
            request_processing_handle: handle,
            url,
            stats: Arc::new(RwLock::new(TransportStats::default())),
            tx,
        }
    }
}

#[async_trait::async_trait]
impl<T> RpcSender for RpcClientSender<T>
where
    T: RpcSenderService,
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
        tracing::info!(rpc_sender_return=?resp);
        resp
    }

    fn get_transport_stats(&self) -> RpcTransportStats {
        self.stats.read().unwrap().deref().into()
    }

    fn url(&self) -> String {
        self.url.clone()
    }
}
