// use crate::transaction_err::CheckTransactionError;
// use solana_client::client_error::{ClientError, ClientErrorKind};
// use solana_sdk::transaction::TransactionError;
// use std::fmt::Debug;
// use solana_client::rpc_request::{RpcError, RpcResponseErrorData};
// use solana_client::rpc_response::RpcSimulateTransactionResult;
// use thiserror::Error;
//
// #[derive(Debug, Error)]
// pub enum ClientErrorCheckFailure<'a, NoError: Debug> {
//     #[error("no error: {0:?}")]
//     NoError(&'a NoError),
//     #[error("expected transaction error: {0}, found: {1}")]
//     NotTransactionError(TransactionError, &'a ClientError),
//     #[error("expected simulation error: {0}, found: {1}")]
//     NotSimulationError(TransactionError, &'a ClientError),
//     #[error("failed transaction check: {0}, found: {1}")]
//     TransactionErrorCheckFailure(TransactionError, &'a ClientError),
//     #[error("failed instruction check, found: {0}")]
//     InstructionErrorCheckFailure(&'a ClientError),
//     #[error("failed instruction check for instruction index {0:?} and code {1}, found: {2}")]
//     NotInstructionError(Option<u8>, u32, &'a ClientError),
// }
//
// pub trait CheckClientError {
//     type NoError: Debug;
//
//     fn get_err(&self) -> Result<&ClientError, &Self::NoError>;
//
//     fn check_transaction_err(
//         &self,
//         expected: TransactionError,
//     ) -> Result<&Self, ClientErrorCheckFailure<Self::NoError>> {
//         match self.get_err() {
//             Err(e) => Err(ClientErrorCheckFailure::NoError(e)),
//             Ok(e) => match e.get_transaction_error() {
//                 Some(tx_err) => tx_err
//                     .check_transaction_err(expected.clone())
//                     .map(|_| self)
//                     .map_err(|_| {
//                         ClientErrorCheckFailure::TransactionErrorCheckFailure(expected, e)
//                     }),
//                 None => Err(ClientErrorCheckFailure::NotTransactionError(expected, e)),
//             },
//         }
//     }
//
//     fn check_transaction_err_code(
//         &self,
//         instruction_index: Option<u8>,
//         error_code: impl Into<u32>,
//     ) -> Result<&Self, ClientErrorCheckFailure<Self::NoError>> {
//         match self.get_err() {
//             Err(e) => Err(ClientErrorCheckFailure::NoError(e)),
//             Ok(e) => match e {
//                 ClientError::TransactionError(tx_err) => tx_err
//                     .check_instruction_err_at_index(instruction_index, error_code)
//                     .map(|_| self)
//                     .map_err(|_| ClientErrorCheckFailure::InstructionErrorCheckFailure(e)),
//                 _ => Err(ClientErrorCheckFailure::NotInstructionError(
//                     instruction_index,
//                     error_code.into(),
//                     e,
//                 )),
//             },
//         }
//     }
//
//     fn check_simulation_err_code(
//         &self,
//         instruction_index: Option<u8>,
//         error_code: impl Into<u32>,
//     ) -> Result<&Self, ClientErrorCheckFailure<Self::NoError>> {
//         match self.get_err() {
//             Err(e) => Err(ClientErrorCheckFailure::NoError(e)),
//             Ok(e) => match e {
//                 ClientError::SimulationError { err: tx_err, .. } => tx_err
//                     .check_instruction_err_at_index(instruction_index, error_code)
//                     .map(|_| self)
//                     .map_err(|_| ClientErrorCheckFailure::InstructionErrorCheckFailure(e)),
//                 _ => Err(ClientErrorCheckFailure::NotInstructionError(
//                     instruction_index,
//                     error_code.into(),
//                     e,
//                 )),
//             },
//         }
//     }
// }
//
// impl<T: Debug> CheckClientError for Result<T, ClientError> {
//     type NoError = T;
//
//     fn get_err(&self) -> Result<&ClientError, &Self::NoError> {
//         match self {
//             Ok(t) => Err(t),
//             Err(e) => Ok(e),
//         }
//     }
// }
