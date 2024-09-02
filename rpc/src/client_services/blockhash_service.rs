use std::{
    borrow::BorrowMut,
    collections::VecDeque,
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
    time::{Duration, Instant},
};

use anyhow::anyhow;
use futures::future;
use serde_json::json;
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_request::RpcRequest,
    rpc_response::{Response, RpcBlockhash},
};
use solana_sdk::{clock::Slot, commitment_config::CommitmentConfig, hash::Hash};
use tokio::{task::JoinHandle, time::sleep};

#[derive(Debug, Clone, PartialEq)]
pub struct CachedBlockhash {
    pub hash: Hash,
    pub last_valid_block_height: Option<u64>,
    pub slot: Option<Slot>,
    pub blocktime: Option<i64>,
    received_at: Instant,
}

impl CachedBlockhash {
    pub fn new(
        hash: Hash,
        last_valid_block_height: Option<u64>,
        slot: Option<Slot>,
        blocktime: Option<i64>,
    ) -> Self {
        Self {
            hash,
            last_valid_block_height,
            slot,
            blocktime,
            received_at: Instant::now(),
        }
    }

    pub async fn new_from_rpc(
        client: &RpcClient,
        commitment: CommitmentConfig,
    ) -> anyhow::Result<Self> {
        let response = client
            .send::<Response<RpcBlockhash>>(RpcRequest::GetLatestBlockhash, json!([commitment]))
            .await?;
        let hash: Hash = response.value.blockhash.parse().map_err(|e| {
            tracing::error!(
                last_valid_block_height = response.value.last_valid_block_height,
                slot = response.context.slot,
                rpc_api_version = ?response.context.api_version,
                blockhash = response.value.blockhash,
                "rpc client returned a bad blockhash"
            );
            anyhow!(
                "RPC client returned a bad blockhash: {}",
                response.value.blockhash
            )
            .context(e)
        })?;
        Ok(CachedBlockhash {
            hash,
            last_valid_block_height: Some(response.value.last_valid_block_height),
            slot: Some(response.context.slot),
            blocktime: None,
            received_at: Instant::now(),
        })
    }

    pub fn age_since(&self, now: &Instant) -> Duration {
        now.duration_since(self.received_at)
    }
}

#[derive(Clone)]
pub struct BlockHashService {
    client: Arc<RpcClient>,
    retain_no_older_than: Duration,
    cache_size: usize,
    check_every: Duration,
    stagger_refresh_requests: Duration,
    hashes: Arc<RwLock<VecDeque<CachedBlockhash>>>,
    commitment_config: CommitmentConfig,
}

impl BlockHashService {
    pub fn get_latest<'a>(&'a self, n: usize) -> anyhow::Result<Vec<CachedBlockhash>> {
        let mut hashes = write_lock_cache(&self.hashes)?;
        hashes.make_contiguous();
        let (h, _) = hashes.as_slices();
        let (h, _) = h.split_at(n);
        Ok(h.to_vec())
    }

    pub fn spawn_autorefresh_task(&self) -> JoinHandle<anyhow::Result<()>> {
        let client = self.client.clone();
        let hashes = self.hashes.clone();
        let check_every = self.check_every;
        let max_age = self.retain_no_older_than;
        let max_size = self.cache_size;
        let sleep_between_requests = self.stagger_refresh_requests;
        let commitment = self.commitment_config;
        let handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(check_every).await;
                let guard = write_lock_cache(&hashes)?;
                prune_blockhashes_older_than(guard, max_age)?;
                let cache_size = read_lock_cache(&hashes)?.len();
                if cache_size < max_size {
                    let handles = (0..max_size - cache_size)
                        .map(|i| {
                            let hashes = hashes.clone();
                            let client = client.clone();
                            tokio::spawn(async move {
                                sleep(sleep_between_requests * i as u32).await;
                                let recent_blockhash =
                                    CachedBlockhash::new_from_rpc(&client, commitment)
                                        .await
                                        .map_err(|e| {
                                            anyhow!("failed to get blockhash from rpc").context(e)
                                        })?;
                                write_lock_cache(&hashes)?.push_front(recent_blockhash);
                                Ok(())
                            })
                        })
                        .collect::<Vec<JoinHandle<anyhow::Result<()>>>>();
                    let _ = future::join_all(handles).await;
                }
            }
        });
        handle
    }

    pub fn retain_no_older_than(mut self, duration: Duration) -> Self {
        self.retain_no_older_than = duration;
        self
    }

    pub fn check_every(mut self, duration: Duration) -> Self {
        self.check_every = duration;
        self
    }
}

pub fn prune_blockhashes_older_than(
    mut hashes: RwLockWriteGuard<VecDeque<CachedBlockhash>>,
    max_age: Duration,
) -> anyhow::Result<()> {
    let now = Instant::now();
    while hashes
        .back()
        .filter(|item| now.duration_since(item.received_at) < max_age)
        .is_some()
    {
        hashes.borrow_mut().pop_back();
    }
    Ok(())
}

fn write_lock_cache(
    hashes: &RwLock<VecDeque<CachedBlockhash>>,
) -> anyhow::Result<RwLockWriteGuard<VecDeque<CachedBlockhash>>> {
    hashes
        .write()
        .map_err(|e| anyhow!("failed to acquire blockhash cache writelock").context(format!("{e}")))
}

fn read_lock_cache(
    hashes: &RwLock<VecDeque<CachedBlockhash>>,
) -> anyhow::Result<RwLockReadGuard<VecDeque<CachedBlockhash>>> {
    hashes
        .read()
        .map_err(|e| anyhow!("failed to acquire blockhash cache readlock").context(format!("{e}")))
}
