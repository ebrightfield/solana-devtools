use std::{future::Future, pin::Pin};

use serde::Deserialize;
use serde_json::{json, Value};

pub mod reqwest_client;
pub mod rpc_client_sender;
pub mod stats_updater;

pub use reqwest_client::ReqwestRpcSender;
pub use rpc_client_sender::RpcClientSender;
use solana_client::{
    rpc_custom_error::{
        NodeUnhealthyErrorData, JSON_RPC_SERVER_ERROR_NODE_UNHEALTHY as NODE_UNHEALTHY,
        JSON_RPC_SERVER_ERROR_SEND_TRANSACTION_PREFLIGHT_FAILURE as PREFLIGHT_FAILURE,
    },
    rpc_request::{RpcError, RpcRequest, RpcResponseErrorData},
    rpc_response::RpcSimulateTransactionResult,
};

/// The data types sent to `RpcSender::send`, grouped into a tuple.
pub type RpcSenderRequest = (RpcRequest, Value);
pub use solana_client::client_error::ClientError;

pub type RpcSenderResult<T> = Result<T, ClientError>;
/// The response type to `RpcSender::send`.
pub type RpcSenderResponse = RpcSenderResult<Value>;
/// The return type of an RpcSenderService
pub type RpcSenderFuture = Pin<Box<dyn Future<Output = RpcSenderResponse> + Send>>;

pub trait RpcSenderService:
    tower::Service<RpcSenderRequest, Error = ClientError, Future = RpcSenderFuture> + Send + Sync
{
}

impl<T> RpcSenderService for T where
    T: tower::Service<RpcSenderRequest, Error = ClientError, Future = RpcSenderFuture>
        + Send
        + Sync
{
}

pub(crate) const JSON_RPC: &'static str = "2.0";
pub(crate) const APPLICATION_JSON: &'static str = "application/json";
pub(crate) const SOLANA_CLIENT: &'static str = "solana-client";

pub(crate) fn rust_version() -> String {
    format!("rust/{}", solana_version::Version::default())
}

pub(crate) fn jsonrpc_request(method: String, params: Value, request_id: u64) -> String {
    json!({
       "jsonrpc": JSON_RPC,
       "id": request_id,
       "method": method.to_string(),
       "params": params,
    })
    .to_string()
}

/// Helper struct for easier decoding of the `"error"` field in an RPC response.
#[derive(Deserialize, Debug)]
struct RpcErrorObject {
    pub code: i64,
    pub message: String,
}

impl RpcErrorObject {
    fn parse_value(json: Value) -> RpcSenderResponse {
        let rpc_error_object =
            serde_json::from_value::<RpcErrorObject>(json.clone()).map_err(|e| {
                RpcError::RpcRequestError(format!(
                    "Failed to deserialize RPC error response: {} [{}]",
                    serde_json::to_string(&json).unwrap(),
                    e
                ))
            })?;
        let data = match rpc_error_object.code {
            PREFLIGHT_FAILURE => {
                match serde_json::from_value::<RpcSimulateTransactionResult>(json["data"].clone()) {
                    Ok(data) => RpcResponseErrorData::SendTransactionPreflightFailure(data),
                    Err(err) => {
                        tracing::debug!(
                            "Failed to deserialize RpcSimulateTransactionResult: {:?}",
                            err
                        );
                        RpcResponseErrorData::Empty
                    }
                }
            }
            NODE_UNHEALTHY => {
                let err_data: Result<NodeUnhealthyErrorData, _> =
                    serde_json::from_value(json["data"].clone());
                if let Ok(NodeUnhealthyErrorData { num_slots_behind }) = err_data {
                    RpcResponseErrorData::NodeUnhealthy { num_slots_behind }
                } else {
                    RpcResponseErrorData::Empty
                }
            }
            _ => RpcResponseErrorData::Empty,
        };
        Err(RpcError::RpcResponseError {
            code: rpc_error_object.code,
            message: rpc_error_object.message,
            data,
        }
        .into())
    }
}

#[tracing::instrument]
pub fn to_solana_rpc_result(mut json: Value) -> RpcSenderResponse {
    if json["error"].is_object() {
        tracing::error!(jsonrpc_error = ?json);
        return RpcErrorObject::parse_value(json["error"].take());
    }
    tracing::info!(jsonrpc_response=?json);
    Ok(json["result"].take())
}
