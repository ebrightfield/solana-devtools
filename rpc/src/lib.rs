//! Tower Service based approach to types that implement `solana_client::rpc_sender::RpcSender`,
//! which can then be used to create `RpcClient` instances using `RpcClient::new_sender`.
//! This gives a greater degree of low-level configurability to a RPC client behavior,
//! including rate limiting, request filtering, retry logic, and more.
pub mod service;
pub mod middleware;

pub use service::*;
