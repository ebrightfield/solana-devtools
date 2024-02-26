use crate::transaction_err::CheckTransactionError;
use solana_program_test::BanksClientError;
use solana_sdk::transaction::TransactionError;
use std::fmt::Debug;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BanksClientErrorCheckFailure<'a, NoError: Debug> {
    #[error("no error: {0:?}")]
    NoError(&'a NoError),
    #[error("expected transaction error: {0}, found: {1}")]
    NotTransactionError(TransactionError, &'a BanksClientError),
    #[error("expected simulation error: {0}, found: {1}")]
    NotSimulationError(TransactionError, &'a BanksClientError),
    #[error("failed transaction check: {0}, found: {1}")]
    TransactionErrorCheckFailure(TransactionError, &'a BanksClientError),
    #[error("failed instruction check, found: {0}")]
    InstructionErrorCheckFailure(&'a BanksClientError),
    #[error("failed instruction check for instruction index {0:?} and code {1}, found: {2}")]
    NotInstructionError(Option<u8>, u32, &'a BanksClientError),
}

pub trait CheckBanksClientError {
    type NoError: Debug;

    fn get_err(&self) -> Result<&BanksClientError, &Self::NoError>;

    fn check_transaction_err(
        &self,
        expected: TransactionError,
    ) -> Result<&Self, BanksClientErrorCheckFailure<Self::NoError>> {
        match self.get_err() {
            Err(e) => Err(BanksClientErrorCheckFailure::NoError(e)),
            Ok(e) => match e {
                BanksClientError::TransactionError(tx_err) => tx_err
                    .check_transaction_err(expected.clone())
                    .map(|_| self)
                    .map_err(|_| {
                        BanksClientErrorCheckFailure::TransactionErrorCheckFailure(expected, e)
                    }),
                _ => Err(BanksClientErrorCheckFailure::NotTransactionError(
                    expected, e,
                )),
            },
        }
    }

    fn check_simulation_err(
        &self,
        expected: TransactionError,
    ) -> Result<&Self, BanksClientErrorCheckFailure<Self::NoError>> {
        match self.get_err() {
            Err(e) => Err(BanksClientErrorCheckFailure::NoError(e)),
            Ok(e) => match e {
                BanksClientError::SimulationError { err: tx_err, .. } => tx_err
                    .check_transaction_err(expected.clone())
                    .map(|_| self)
                    .map_err(|_| {
                        BanksClientErrorCheckFailure::TransactionErrorCheckFailure(expected, e)
                    }),
                _ => Err(BanksClientErrorCheckFailure::NotSimulationError(
                    expected, e,
                )),
            },
        }
    }

    fn check_transaction_err_code(
        &self,
        instruction_index: Option<u8>,
        error_code: impl Into<u32>,
    ) -> Result<&Self, BanksClientErrorCheckFailure<Self::NoError>> {
        match self.get_err() {
            Err(e) => Err(BanksClientErrorCheckFailure::NoError(e)),
            Ok(e) => match e {
                BanksClientError::TransactionError(tx_err) => tx_err
                    .check_instruction_err_at_index(instruction_index, error_code)
                    .map(|_| self)
                    .map_err(|_| BanksClientErrorCheckFailure::InstructionErrorCheckFailure(e)),
                _ => Err(BanksClientErrorCheckFailure::NotInstructionError(
                    instruction_index,
                    error_code.into(),
                    e,
                )),
            },
        }
    }

    fn check_simulation_err_code(
        &self,
        instruction_index: Option<u8>,
        error_code: impl Into<u32>,
    ) -> Result<&Self, BanksClientErrorCheckFailure<Self::NoError>> {
        match self.get_err() {
            Err(e) => Err(BanksClientErrorCheckFailure::NoError(e)),
            Ok(e) => match e {
                BanksClientError::SimulationError { err: tx_err, .. } => tx_err
                    .check_instruction_err_at_index(instruction_index, error_code)
                    .map(|_| self)
                    .map_err(|_| BanksClientErrorCheckFailure::InstructionErrorCheckFailure(e)),
                _ => Err(BanksClientErrorCheckFailure::NotInstructionError(
                    instruction_index,
                    error_code.into(),
                    e,
                )),
            },
        }
    }
}

impl<T: Debug> CheckBanksClientError for Result<T, BanksClientError> {
    type NoError = T;

    fn get_err(&self) -> Result<&BanksClientError, &Self::NoError> {
        match self {
            Ok(t) => Err(t),
            Err(e) => Ok(e),
        }
    }
}
