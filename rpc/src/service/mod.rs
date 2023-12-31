pub mod json_rpc;

use crate::json_rpc::stats_updater::TransportStats;
use json_rpc::HttpClientService;
use serde_json::Value;
use solana_client::client_error::{ClientError, ClientErrorKind};
use solana_client::rpc_request::RpcRequest;
use solana_rpc_client::rpc_sender::{RpcSender, RpcTransportStats};
use std::fmt::Debug;
use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::{Layer, Service, ServiceBuilder, ServiceExt};

/// The data types sent to `RpcSender::send`, grouped into a tuple.
pub type RpcSenderRequest = (RpcRequest, Value);
/// The response type to `RpcSender::send`.
pub type RpcSenderResponse = Result<Value, ClientError>;

/// Implements both the [solana_rpc_client::rpc_sender::RpcSender] [tower::Service] traits.
/// By default, it behaves the same as a vanilla [solana_rpc_client::http_sender::HttpSender],
/// but the `tower::Service` trait provides a richer interface for custom configuration.
#[derive(Debug)]
pub struct HttpSenderService<T> {
    service: RwLock<T>,
    url: String,
    /// If the underlying `T` is [HttpClientService], then this is a shared ownership
    /// of the same stats. This allows the [HttpClientService] to modify the stats,
    /// while the outer [HttpSenderService] can implement [solana_rpc_client::rpc_sender::RpcSender]
    /// and return the inner value in `get_transport_stats`.
    stats: Arc<std::sync::RwLock<TransportStats>>,
}

impl HttpSenderService<HttpClientService> {
    /// A default constructor,
    /// which behaves identically to [solana_rpc_client::http_sender::HttpSender].
    pub fn new<U: ToString>(url: U) -> Self {
        let service = HttpClientService::new(url);
        Self::from(service)
    }
}

impl From<HttpClientService> for HttpSenderService<HttpClientService> {
    fn from(value: HttpClientService) -> Self {
        let url = value.url.clone();
        let stats = value.stats.clone();
        Self {
            service: RwLock::new(value),
            url,
            stats,
        }
    }
}

impl<T> HttpSenderService<T> {
    /// The preferred way to customize behavior. For default behavior, use [HttpSenderService::new].
    /// This constructor will create and wrap an inner [HttpClientService] with the configuration
    /// specified on a [tower::ServiceBuilder].
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use serde_json::Value;
    /// use solana_client::client_error::ClientError;
    /// use solana_client::rpc_request::RpcRequest;
    /// use solana_rpc_client::rpc_client::RpcClient;
    /// use solana_sdk::transport::TransportError;
    /// use solana_devtools_rpc::{HttpSenderService, middleware::FilterMiddleware};
    /// use tower::ServiceBuilder;
    ///
    /// // Return a custom client with rate limiting and a filter on which methods are allowed.
    /// fn my_custom_client(url: &str) -> RpcClient {
    ///     let sender = HttpSenderService::new_from_builder(
    ///         url,
    ///         ServiceBuilder::new()
    ///             .layer_fn(|s| {
    ///                 FilterMiddleware::new(s, |req: &RpcRequest, _: &Value| match req {
    ///                     RpcRequest::GetBalance => Ok(()),
    ///                     RpcRequest::GetVersion => Ok(()),
    ///                     RpcRequest::GetLatestBlockhash => Ok(()),
    ///                     _ => Err(ClientError::from(TransportError::Custom(
    ///                         "RPC Method not allowed".to_string(),
    ///                     ))),
    ///                 })
    ///             })
    ///             .rate_limit(5, Duration::from_secs(60)),
    ///     );
    ///     RpcClient::new_sender(sender, Default::default())
    /// }
    /// ```
    pub fn new_from_builder<U, L>(url: U, builder: ServiceBuilder<L>) -> Self
    where
        U: ToString,
        L: Layer<HttpClientService, Service = T>,
    {
        let inner = HttpClientService::new(url);
        let url = inner.url.clone();
        let stats = inner.stats.clone();
        let service = builder.service(inner);
        Self {
            service: RwLock::new(service),
            url,
            stats,
        }
    }

    /// Since [HttpSenderService] doesn't do anything with the `stats` or `url` field on its own
    /// other than expose them to `impl solana_client::rpc_sender::RpcSender`,
    /// it is up to the caller as to whether the inner service `T` does anything with `stats` and
    /// whether the inner service is using the same `url` to make requests.
    /// This is therefore not a generally recommended way to instantiate customized instances.
    /// The recommended way to customize the behavior of [HttpSenderService]
    /// is with [HttpSenderService::new_from_builder].
    pub fn new_from_service<U: ToString>(
        service: T,
        url: U,
        stats: Arc<std::sync::RwLock<TransportStats>>,
    ) -> Self {
        Self {
            service: RwLock::new(service),
            url: url.to_string(),
            stats,
        }
    }
}

#[async_trait::async_trait]
impl<T, E> RpcSender for HttpSenderService<T>
where
    E: Send,
    T: Service<
            RpcSenderRequest,
            Error = E,
            Future = Pin<Box<(dyn Future<Output = RpcSenderResponse> + Send)>>,
        > + Send
        + Sync,
{
    async fn send(
        &self,
        request: RpcRequest,
        params: Value,
    ) -> solana_client::client_error::Result<Value> {
        let mut lock = self.service.write().await;
        match lock.deref_mut().ready().await {
            Ok(service) => {
                let fut = service.call((request, params));
                fut.await
            }
            Err(_) => Err(ClientError::new_with_request(
                ClientErrorKind::Custom("Failed to poll RPC service for readiness".to_string()),
                request,
            )),
        }
    }

    fn get_transport_stats(&self) -> RpcTransportStats {
        self.stats.read().unwrap().deref().into()
    }

    fn url(&self) -> String {
        self.url.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::FilterMiddleware;
    use crossbeam_channel::{unbounded, Receiver};
    use futures_util::future;
    use jsonrpc_core::{IoHandler, Params};
    use jsonrpc_http_server::{AccessControlAllowOrigin, DomainsValidation, ServerBuilder};
    use serde_json::json;
    use solana_client::client_error::ClientError;
    use solana_client::nonblocking::rpc_client::RpcClient;
    use solana_client::rpc_request::RpcRequest;
    use solana_client::rpc_response::{Response, RpcBlockhash, RpcResponseContext, RpcVersionInfo};
    use solana_sdk::hash::Hash;
    use solana_sdk::pubkey;
    use solana_sdk::transport::TransportError;
    use std::net::SocketAddr;
    use std::str::FromStr;
    use std::thread;
    use std::time::{Duration, SystemTime};
    use tower::ServiceBuilder;

    fn spawn_test_server(host: &str) -> Receiver<SocketAddr> {
        let (sender, receiver) = unbounded();
        let rpc_addr = host.parse().unwrap();
        thread::spawn(move || {
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
                        solana_core: "1.16.23".to_string(),
                        feature_set: None,
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
        return receiver;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn http_sender_on_tokio_multi_thread() {
        let http_sender = HttpSenderService::new("http://localhost:1234".to_string());
        let _ = http_sender.send(RpcRequest::GetVersion, Value::Null).await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn http_sender_on_tokio_current_thread() {
        let http_sender = HttpSenderService::new("http://localhost:1234".to_string());
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
        let rpc_addr = spawn_test_server("0.0.0.0:0").recv().unwrap();
        let rpc_addr = format!("http://{}", rpc_addr);

        let sender = HttpSenderService::new(rpc_addr);
        let rpc_client = RpcClient::new_sender(sender, Default::default());

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
        let sender = HttpSenderService::new_from_service(
            HttpClientService::new("http://localhost:8899"),
            "http://localhost:8899",
            Default::default(),
        );
        let _ = RpcClient::new_sender(sender, Default::default());
    }

    #[tokio::test]
    async fn service_order_doesnt_matter() {
        // Construct in a different order than below
        let sender = HttpSenderService::new_from_builder(
            "http://localhost:8899",
            ServiceBuilder::new()
                .layer_fn(|s| {
                    FilterMiddleware::new(s, |req: &RpcRequest, _: &Value| match &req {
                        RpcRequest::GetBalance => Ok(()),
                        RpcRequest::GetVersion => Ok(()),
                        RpcRequest::GetLatestBlockhash => Ok(()),
                        _ => Err(ClientError::from(TransportError::Custom(
                            "RPC Method not allowed".to_string(),
                        ))),
                    })
                })
                .rate_limit(5, Duration::from_secs(60)),
        );
        let _ = RpcClient::new_sender(sender, Default::default());
    }

    #[tokio::test]
    async fn respects_inner_service_readiness() {
        let rpc_addr = spawn_test_server("0.0.0.0:0").recv().unwrap();
        let rpc_addr = format!("http://{}", rpc_addr);

        let sender = HttpSenderService::new_from_builder(
            rpc_addr,
            ServiceBuilder::new().rate_limit(2, Duration::from_millis(600)),
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
        let rpc_addr = spawn_test_server("0.0.0.0:0").recv().unwrap();
        let rpc_addr = format!("http://{}", rpc_addr);

        let sender = HttpSenderService::new_from_builder(
            rpc_addr,
            ServiceBuilder::new()
                .rate_limit(5, Duration::from_secs(60))
                .layer_fn(|s| {
                    FilterMiddleware::new(s, |req: &RpcRequest, _: &Value| match req {
                        RpcRequest::GetBalance => Ok(()),
                        RpcRequest::GetVersion => Ok(()),
                        RpcRequest::GetLatestBlockhash => Ok(()),
                        _ => Err(ClientError::from(TransportError::Custom(
                            "RPC Method not allowed".to_string(),
                        ))),
                    })
                }),
        );

        let rpc_client = RpcClient::new_sender(sender, Default::default());

        let balance = rpc_client
            .get_balance(&pubkey!("deadbeefXjn8o3yroDHxUtKsZZgoy4GPkPPXfouKNHh"))
            .await
            .unwrap();
        assert_eq!(balance, 50);
        let result = rpc_client.get_slot().await.unwrap_err();
        assert_eq!(
            result.to_string(),
            ClientError::from(TransportError::Custom("RPC Method not allowed".to_string()))
                .to_string()
        );
    }
}
