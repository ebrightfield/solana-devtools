use std::time::{Duration, Instant};
use std::sync::{Arc, RwLock};
use solana_rpc_client::rpc_sender::RpcTransportStats;

#[derive(Default, Clone, Debug)]
pub struct TransportStats {
    /// Number of RPC requests issued
    pub request_count: usize,

    /// Total amount of time spent transacting with the RPC server
    pub elapsed_time: Duration,

    /// Total amount of waiting time due to RPC server rate limiting
    /// (a subset of `elapsed_time`)
    pub rate_limited_time: Duration,
}

impl Into<RpcTransportStats> for &TransportStats {
    fn into(self) -> RpcTransportStats {
        RpcTransportStats {
            request_count: self.request_count,
            elapsed_time: self.elapsed_time.clone(),
            rate_limited_time: self.rate_limited_time.clone(),
        }
    }
}

pub struct StatsUpdater {
    stats: Arc<RwLock<TransportStats>>,
    request_start_time: Instant,
    rate_limited_time: Duration,
}

impl StatsUpdater {
    pub fn new(stats: Arc<RwLock<TransportStats>>) -> Self {
        Self {
            stats,
            request_start_time: Instant::now(),
            rate_limited_time: Duration::default(),
        }
    }

    pub fn add_rate_limited_time(&mut self, duration: Duration) {
        self.rate_limited_time += duration;
    }
}

impl Drop for StatsUpdater {
    fn drop(&mut self) {
        let mut stats = self.stats.write().unwrap();
        stats.request_count += 1;
        stats.elapsed_time += Instant::now().duration_since(self.request_start_time);
        stats.rate_limited_time += self.rate_limited_time;
    }
}
