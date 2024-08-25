pub mod json_rpc;

pub use json_rpc::*;

pub use json_rpc::ReqwestRpcSender;
pub use json_rpc::RpcClientSender;

pub use serde_json::Value;
pub use solana_client::rpc_request::RpcRequest;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use solana_client::client_error::ClientError;
    use solana_client::rpc_request::RpcRequest;

    use crate::middleware::{RpcSenderFilter, RpcSenderMiddleware};
    use crossbeam_channel::{unbounded, Receiver};
    use futures_util::future;
    use json_rpc::{reqwest_client::ReqwestRpcSender, RpcClientSender};
    use jsonrpc_core::{IoHandler, Params};
    use jsonrpc_http_server::{AccessControlAllowOrigin, DomainsValidation, ServerBuilder};
    use serde_json::json;
    use solana_client::nonblocking::rpc_client::RpcClient;
    use solana_client::rpc_response::{Response, RpcBlockhash, RpcResponseContext, RpcVersionInfo};
    use solana_rpc_client::rpc_sender::RpcSender;
    use solana_sdk::hash::Hash;
    use solana_sdk::pubkey;
    use solana_sdk::transport::TransportError;
    use std::net::SocketAddr;
    use std::str::FromStr;
    use std::thread::{self, JoinHandle};
    use std::time::{Duration, SystemTime};
    use tower::{BoxError, ServiceBuilder};
    use tracing_subscriber::fmt::format::FmtSpan;

    fn spawn_test_server(host: &str) -> (Receiver<SocketAddr>, JoinHandle<()>) {
        let _ = tracing_subscriber::fmt::Subscriber::builder()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_span_events(FmtSpan::FULL)
            .try_init();
        let (sender, receiver) = unbounded();
        let rpc_addr = host.parse().unwrap();
        let handle = thread::spawn(move || {
            let mut io = IoHandler::default();
            // Successful request
            io.add_method("getBalance", |_params: Params| {
                future::ok(
                    serde_json::to_value(Response {
                        context: RpcResponseContext {
                            slot: 100,
                            api_version: None,
                        },
                        value: 50,
                    })
                    .unwrap(),
                )
            });
            io.add_method("getVersion", |_params: Params| {
                future::ok(
                    serde_json::to_value(RpcVersionInfo {
                        solana_core: "1.18.21".to_string(),
                        feature_set: Some(99),
                    })
                    .unwrap(),
                )
            });
            io.add_method("getLatestBlockhash", |_params: Params| {
                future::ok(
                    serde_json::to_value(Response {
                        context: RpcResponseContext {
                            slot: 100,
                            api_version: None,
                        },
                        value: RpcBlockhash {
                            blockhash: "deadbeefXjn8o3yroDHxUtKsZZgoy4GPkPPXfouKNHh".to_string(),
                            last_valid_block_height: 100,
                        },
                    })
                    .unwrap(),
                )
            });

            let server = ServerBuilder::new(io)
                .threads(1)
                .cors(DomainsValidation::AllowOnly(vec![
                    AccessControlAllowOrigin::Any,
                ]))
                .start_http(&rpc_addr)
                .expect("Unable to start RPC server");
            sender.send(server.address().clone()).unwrap();
            server.wait();
        });
        (receiver, handle)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn http_sender_on_tokio_multi_thread() {
        let http_sender = RpcClientSender::new_reqwest("http://localhost:0".to_string());
        let _ = http_sender.send(RpcRequest::GetVersion, Value::Null).await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn http_sender_on_tokio_current_thread() {
        let http_sender = RpcClientSender::new_reqwest("http://localhost:0".to_string());
        let _ = http_sender.send(RpcRequest::GetVersion, Value::Null).await;
    }

    #[tokio::test]
    async fn test_send() {
        _test_send().await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_send_async_current_thread() {
        _test_send().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_send_async_multi_thread() {
        _test_send().await;
    }

    async fn _test_send() {
        let (rx, _) = spawn_test_server("0.0.0.0:0");
        let rpc_addr = rx.recv().unwrap();
        let rpc_addr = format!("http://{}", rpc_addr);

        let sender = RpcClientSender::new_reqwest(rpc_addr);
        let rpc_client = RpcClient::new_sender(sender, Default::default());
        // tokio::time::sleep(Duration::from_secs(1)).await;

        let _ = rpc_client.get_version().await.unwrap();
        let balance = rpc_client
            .get_balance(&pubkey!("deadbeefXjn8o3yroDHxUtKsZZgoy4GPkPPXfouKNHh"))
            .await
            .unwrap();
        assert_eq!(balance, 50);

        let blockhash = rpc_client.get_latest_blockhash().await.unwrap();
        assert_eq!(
            blockhash,
            Hash::from_str("deadbeefXjn8o3yroDHxUtKsZZgoy4GPkPPXfouKNHh").unwrap()
        );

        // Send erroneous parameter
        let blockhash: Result<String, ClientError> = rpc_client
            .send(RpcRequest::GetLatestBlockhash, json!(["parameter"]))
            .await;
        assert!(blockhash.is_err());
    }

    #[tokio::test]
    async fn generic_constructor() {
        let sender = RpcClientSender::new(
            ReqwestRpcSender::new("http://localhost:8899".to_string()),
            "http://localhost:8899".to_string(),
        );
        let _ = RpcClient::new_sender(sender, Default::default());
    }

    #[tokio::test]
    async fn service_order_doesnt_matter() {
        // Construct in a different order than below
        let sender = RpcClientSender::new_from_builder(
            "http://localhost:8899".to_string(),
            ServiceBuilder::new()
                .layer_fn(|s| {
                    RpcSenderFilter::new(s, |req: &RpcRequest, _: &Value| match &req {
                        RpcRequest::GetBalance => Ok(()),
                        RpcRequest::GetVersion => Ok(()),
                        RpcRequest::GetLatestBlockhash => Ok(()),
                        _ => Err(Box::new(ClientError::from(TransportError::Custom(
                            "RPC Method not allowed".to_string(),
                        ))) as BoxError),
                    })
                })
                .rate_limit(5, Duration::from_secs(60)),
        );
        let _ = RpcClient::new_sender(sender, Default::default());
    }

    #[tokio::test]
    async fn respects_inner_service_readiness() {
        let (rx, _) = spawn_test_server("0.0.0.0:0");
        let rpc_addr = rx.recv().unwrap();
        let rpc_addr = format!("http://{}", rpc_addr);

        let sender = RpcClientSender::new_from_builder(
            rpc_addr,
            ServiceBuilder::new()
                .rate_limit(2, Duration::from_millis(600))
                .layer_fn(|s| {
                    RpcSenderFilter::new(s, |req: &RpcRequest, _: &Value| match &req {
                        RpcRequest::GetBalance => Ok(()),
                        RpcRequest::GetVersion => Ok(()),
                        RpcRequest::GetLatestBlockhash => Ok(()),
                        _ => Err(Box::new(ClientError::from(TransportError::Custom(
                            "RPC Method not allowed".to_string(),
                        ))) as BoxError),
                    })
                }),
        );
        let rpc_client = RpcClient::new_sender(sender, Default::default());

        let before_first = SystemTime::now();
        let balance = rpc_client
            .get_balance(&pubkey!("deadbeefXjn8o3yroDHxUtKsZZgoy4GPkPPXfouKNHh"))
            .await
            .unwrap();
        let after_first = SystemTime::now();
        let elapsed_after_first = before_first.elapsed().unwrap();
        let _ = rpc_client
            .get_balance(&pubkey!("deadbeefXjn8o3yroDHxUtKsZZgoy4GPkPPXfouKNHh"))
            .await
            .unwrap();
        let elapsed_after_second = after_first.elapsed().unwrap();
        assert_eq!(balance, 50);
        assert!(
            Duration::from_millis(100) > elapsed_after_first,
            "{:?}",
            elapsed_after_first
        );
        assert!(
            Duration::from_millis(600) < elapsed_after_first + elapsed_after_second,
            "{:?}",
            elapsed_after_second
        );
    }

    #[tokio::test]
    async fn service() {
        let (rx, _) = spawn_test_server("0.0.0.0:0");
        let rpc_addr = rx.recv().unwrap();
        let rpc_addr = format!("http://{}", rpc_addr);

        let sender = RpcClientSender::new_from_builder(
            rpc_addr,
            ServiceBuilder::new()
                .rate_limit(5, Duration::from_secs(60))
                .and_then(|resp| {
                    Box::pin(async move {
                        tracing::error!(message = "from inside the `and_then` function", ?resp);
                        Ok(resp)
                    })
                })
                .filter(|res| {
                    tracing::info!("from inside the `filter` function");
                    Result::<_, BoxError>::Ok(res)
                })
                .concurrency_limit(1024)
                .layer_fn(|s| {
                    RpcSenderMiddleware::new(s, |req: &RpcRequest, v: &Value| {
                        if let RpcRequest::GetBalance = req {
                            tracing::info!(value=?v);
                            let resp = serde_json::to_value(Response {
                                context: RpcResponseContext {
                                    slot: 100,
                                    api_version: None,
                                },
                                value: 123456789,
                            })
                            .unwrap();
                            tracing::info!(?resp);
                            return Some(Ok(resp));
                        }
                        None
                    })
                })
                .layer_fn(|s| {
                    RpcSenderFilter::new(s, |req: &RpcRequest, _: &Value| match req {
                        RpcRequest::GetBalance => Ok(()),
                        RpcRequest::GetVersion => Ok(()),
                        RpcRequest::GetLatestBlockhash => Ok(()),
                        _ => Err(Box::new(ClientError::from(TransportError::Custom(
                            "RPC Method not allowed".to_string(),
                        ))) as BoxError),
                    })
                }),
        );

        let rpc_client = RpcClient::new_sender(sender, Default::default());

        let balance = rpc_client
            .get_balance(&pubkey!("deadbeefXjn8o3yroDHxUtKsZZgoy4GPkPPXfouKNHh"))
            .await
            .unwrap();
        assert_eq!(balance, 123456789);
        let result = rpc_client.get_slot().await.unwrap_err();
        assert_eq!(
            result.to_string(),
            ClientError::from(TransportError::Custom("RPC Method not allowed".to_string()))
                .to_string()
        );
    }
}
