use crate::transaction_err::CheckTransactionError;
use solana_program_test::BanksClientError;
use solana_sdk::transaction::TransactionError;

impl CheckTransactionError for BanksClientError {
    type NoError = Self;

    fn get_err(&self) -> Result<&TransactionError, &Self::NoError> {
        match self {
            BanksClientError::TransactionError(err)
            | BanksClientError::SimulationError { err, .. } => Ok(err),
            _ => Err(self),
        }
    }
}
