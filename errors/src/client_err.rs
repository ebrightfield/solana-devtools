use crate::transaction_err::CheckTransactionError;
use solana_client::client_error::{ClientError, ClientErrorKind};
use solana_client::rpc_request::RpcError::RpcResponseError;
use solana_client::rpc_request::RpcResponseErrorData::SendTransactionPreflightFailure;
use solana_client::rpc_response::RpcSimulateTransactionResult;
use solana_sdk::transaction::TransactionError;

impl CheckTransactionError for ClientError {
    type NoError = Self;

    fn get_err(&self) -> Result<&TransactionError, &Self::NoError> {
        match &self.kind {
            ClientErrorKind::RpcError(RpcResponseError {
                data:
                    SendTransactionPreflightFailure(RpcSimulateTransactionResult {
                        err: Some(tx_err),
                        ..
                    }),
                ..
            }) => Ok(tx_err),
            ClientErrorKind::TransactionError(tx_err) => Ok(tx_err),
            _ => Err(self),
        }
    }
}
