use crate::log_parsing::{
    check_for_program_error, parse_transaction_logs, LoggedTransactionFailure,
};
use anchor_lang;
use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use log::{error, info, warn};
use solana_client::{
    nonblocking::pubsub_client::PubsubClient,
    rpc_config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter},
};
use solana_sdk::{
    clock::Slot, commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signature,
};
use std::{future::Future, marker::PhantomData, pin::Pin};
use tokio::{
    runtime::Handle,
    sync::mpsc::{unbounded_channel, UnboundedReceiver},
    task::JoinHandle,
};

type UnsubscribeFn = Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send>;

/// Allows for a graceful unsubscribe,
/// or wait for the server to terminate the Websocket connection.
pub struct ProgramEventSubscription<'a> {
    pub handle: JoinHandle<Result<()>>,
    rx: UnboundedReceiver<UnsubscribeFn>,
    _lifetime_marker: PhantomData<&'a Handle>,
}

impl<'a> ProgramEventSubscription<'a> {
    pub async fn wait_until_server_disconnects(self) -> Result<()> {
        self.handle.await.map_err(|e| {
            anyhow!(
                "Error occurred while waiting for the server to terminate WS connection: {}",
                e
            )
        })?
    }

    /// Unsubscribe gracefully.
    pub async fn unsubscribe(mut self) {
        if let Some(unsubscribe) = self.rx.recv().await {
            unsubscribe().await;
        }

        let _ = self.handle.await;
    }
}

#[async_trait::async_trait]
pub trait ProgramEventLogSubscriber: Clone + Send + Sync + 'static {
    type Event: anchor_lang::Event + anchor_lang::AnchorDeserialize + Send;
    fn target_program() -> Pubkey;

    fn ws_url(&self) -> String;

    /// Typically just sleep some amount of seconds,
    /// but if necessary, maybe also do some logging or cleanup.
    async fn on_reconnect(&self) -> Result<()>;

    /// Define what to do with each new incoming event.
    async fn on_event(
        self,
        signature: Signature,
        slot: Slot,
        execution_error: Option<LoggedTransactionFailure>,
        raw_event: String,
        event: Self::Event,
    );

    /// Continually watch for the emission of SSLv2 program events,
    /// by using the Solana RPC Websockets `logs_subscribe`.
    ///
    /// Whenever an event is found, process the event for DB insertion.
    fn watch_for_events(&self) -> JoinHandle<()> {
        let state = self.clone();
        tokio::spawn(async move {
            let reconnect_state = state.clone();
            loop {
                let state = state.clone();
                let event_subscription = match state.subscribe_to_events().await {
                    Ok(event_subscription) => event_subscription,
                    Err(e) => {
                        error!("failed to subscribe to events: {:?}", e);
                        return;
                    }
                };
                if let Err(e) = event_subscription.wait_until_server_disconnects().await {
                    error!("program event subscriber exited abnormally: {:?}", e);
                } else {
                    warn!("websocket subscription closed by the RPC server")
                }
                reconnect_state.on_reconnect().await.unwrap();
            }
        })
    }

    async fn subscribe_to_events<'a>(self) -> Result<ProgramEventSubscription<'a>> {
        let (tx, rx) = unbounded_channel::<_>();
        let program_id_str = Self::target_program().to_string();
        let ws_url = self.ws_url();
        let sub_client = PubsubClient::new(&ws_url)
            .await
            .map_err(|e| anyhow!("error creating pubsub client: {:?}", e))?;

        let handle = tokio::spawn(async move {
            let filter = RpcTransactionLogsFilter::Mentions(vec![program_id_str.clone()]);
            let config = RpcTransactionLogsConfig {
                commitment: Some(CommitmentConfig::finalized()),
            };
            info!("attempting to connect to client WS {}", ws_url);
            let (mut notifications, unsubscribe) = sub_client
                .logs_subscribe(filter, config)
                .await
                .map_err(|e| anyhow!("Error subscribing to Solana program logs: {:?}", e))?;

            info!(
                "connected to client WS {}, subscribed to program logs",
                ws_url
            );

            tx.send(unsubscribe).map_err(|e| {
                anyhow!(
                    "error sending the unsubscribe function through a Rust channel: {:?}",
                    e
                )
            })?;

            while let Some(response) = notifications.next().await {
                let signature: Signature = response.value.signature.parse().unwrap();
                let slot = response.context.slot;
                let logs = response.value.logs;
                let execution_error = logs.last().map(|l| check_for_program_error(l)).flatten();
                let events = parse_transaction_logs(logs, &program_id_str);
                for (raw_event, event) in events {
                    self.clone()
                        .on_event(
                            signature.clone(),
                            slot.clone(),
                            execution_error.clone(),
                            raw_event,
                            event,
                        )
                        .await;
                }
            }
            warn!("websockets log subscription closed");
            Ok::<(), anyhow::Error>(())
        });

        Ok(ProgramEventSubscription {
            handle,
            rx,
            _lifetime_marker: PhantomData,
        })
    }
}
