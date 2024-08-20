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
use tower::{Layer, Service, ServiceBuilder, ServiceExt};

use super::reqwest_client::ReqwestRpcSender;
use super::{RpcSenderFuture, RpcSenderService};

#[tracing::instrument(skip_all)]
async fn process_requests<T>(
    mut inner: T,
    stats: Arc<RwLock<TransportStats>>,
    mut rx: mpsc::UnboundedReceiver<(RpcSenderRequest, oneshot::Sender<RpcSenderResponse>)>,
) where
    T: Service<RpcSenderRequest, Error = ClientError, Future = RpcSenderFuture>
        + Send
        + Sync
        + 'static,
{
    loop {
        match rx.recv().await {
            Some((value, tx)) => {
                let (method, params) = value;
                if let Err(e) = inner.ready().await {
                    tracing::error!(err=?e);
                    return;
                }
                let fut = inner.call((method, params));
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
}

// Top level service struct.
pub struct RpcClientSender {
    pub request_processing_handle: JoinHandle<()>,
    stats: Arc<RwLock<TransportStats>>,
    url: String,
    transmitter: UnboundedSender<(RpcSenderRequest, oneshot::Sender<RpcSenderResponse>)>,
}
impl RpcClientSender {
    pub fn new<T>(inner: T, url: String) -> Self
    where
        T: Service<RpcSenderRequest, Error = ClientError, Future = RpcSenderFuture>
            + Send
            + Sync
            + 'static,
    {
        let (tx, rx) =
            mpsc::unbounded_channel::<(RpcSenderRequest, oneshot::Sender<RpcSenderResponse>)>();
        let stats = Arc::new(RwLock::new(TransportStats::default()));
        let handle = tokio::spawn(process_requests(inner, stats.clone(), rx));
        Self {
            request_processing_handle: handle,
            url: url.clone(),
            stats,
            transmitter: tx,
        }
    }
    pub fn new_from_builder<U, L, T>(url: U, builder: ServiceBuilder<L>) -> Self
    where
        U: ToString,
        L: Layer<ReqwestRpcSender, Service = T>,
        T: RpcSenderService + 'static, // T: Service<RpcSenderRequest, Error = ClientError, Future = RpcSenderFuture>
                                       //     + Send
                                       //     + Sync
                                       //     + 'static,
    {
        let inner = ReqwestRpcSender::new(url.to_string());
        let url = url.to_string();
        let inner = builder.service(inner);
        let (tx, rx) =
            mpsc::unbounded_channel::<(RpcSenderRequest, oneshot::Sender<RpcSenderResponse>)>();
        let stats = Arc::new(RwLock::new(TransportStats::default()));
        let handle = tokio::spawn(process_requests(inner, stats.clone(), rx));
        Self {
            request_processing_handle: handle,
            url,
            stats: Arc::new(RwLock::new(TransportStats::default())),
            transmitter: tx,
        }
    }

    pub fn foo() {
        // Start a routine that waits for messages, and processes them using &mut able things
    }
}

impl RpcClientSender {
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
            transmitter: tx,
        }
    }
}

#[async_trait::async_trait]
impl RpcSender for RpcClientSender {
    async fn send(
        &self,
        request: RpcRequest,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, ClientError> {
        let (tx, rx) = oneshot::channel();
        self.transmitter
            .send(((request, params), tx))
            .map_err(|e| {
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
