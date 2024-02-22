use solana_sdk::instruction::InstructionError;
use solana_sdk::transaction::TransactionError;
use std::fmt::Debug;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransactionErrorCheckFailure<'a, NoError: Debug> {
    #[error("no error: {0:?}")]
    NoError(&'a NoError),
    #[error("expected transaction error: {0}, found: {1}")]
    WrongVariant(TransactionError, &'a TransactionError),
    #[error("expected failure at instruction index: {0}, found: {1}")]
    WrongIndex(u8, &'a TransactionError),
    #[error("expected error code: {0}, found: {1}")]
    WrongErrorCode(u32, &'a TransactionError),
    #[error("expected an instruction error, found: {0}")]
    NotInstructionError(&'a TransactionError),
}

pub trait CheckTransactionError {
    type NoError: Debug;

    fn get_err(&self) -> Result<&TransactionError, &Self::NoError>;

    fn check_transaction_err(
        &self,
        expected: TransactionError,
    ) -> Result<&Self, TransactionErrorCheckFailure<Self::NoError>> {
        match self.get_err() {
            Err(e) => Err(TransactionErrorCheckFailure::NoError(e)),
            Ok(e) => {
                if *e != expected {
                    Err(TransactionErrorCheckFailure::WrongVariant(expected, e))
                } else {
                    Ok(self)
                }
            }
        }
    }

    fn check_instruction_err(
        &self,
        instruction_index: Option<u8>,
        error_code: impl Into<u32>,
    ) -> Result<&Self, TransactionErrorCheckFailure<Self::NoError>> {
        match self.get_err() {
            Err(e) => Err(TransactionErrorCheckFailure::NoError(e)),
            Ok(e) => match e {
                TransactionError::InstructionError(idx, InstructionError::Custom(code)) => {
                    let error_code = error_code.into();
                    if let Some(instruction_index) = instruction_index {
                        if *idx != instruction_index {
                            return Err(TransactionErrorCheckFailure::WrongIndex(
                                instruction_index,
                                e,
                            ));
                        }
                    }
                    if *code != error_code {
                        Err(TransactionErrorCheckFailure::WrongErrorCode(error_code, e))
                    } else {
                        Ok(self)
                    }
                }
                _ => Err(TransactionErrorCheckFailure::NotInstructionError(e)),
            },
        }
    }

    // fn on_instruction_err(
    //     &self,
    //     instruction_index: Option<u8>,
    //     error_code: impl Into<u32>,
    //     f: impl FnOnce() -> (),
    // ) -> &Self {
    //     if let Ok(e) = self.get_err() {
    //         if let TransactionError::InstructionError(idx, InstructionError::Custom(code)) = e {
    //             let error_code = error_code.into();
    //             let index_matches = if let Some(instruction_index) = instruction_index {
    //                 *idx == instruction_index
    //             } else {
    //                 true
    //             };
    //             if *code == error_code && index_matches {
    //                 f()
    //             }
    //         }
    //     }
    //     self
    // }
    //
    fn on_transaction_err(
        &self,
        expected: &TransactionError,
        f: impl FnOnce(&TransactionError) -> (),
    ) -> &Self {
        if let Ok(e) = self.get_err() {
            if *e == *expected {
                f(e)
            }
        }
        self
    }

    fn map_expected_transaction_err<T>(
        &self,
        expected: &TransactionError,
        f: impl FnOnce(&TransactionError) -> T,
    ) -> Result<T, &Self> {
        if let Ok(e) = self.get_err() {
            if *e == *expected {
                return Ok(f(e));
            }
        }
        Err(self)
    }
}

impl<T: Debug> CheckTransactionError for Result<T, TransactionError> {
    type NoError = T;

    fn get_err(&self) -> Result<&TransactionError, &Self::NoError> {
        match self {
            Ok(t) => Err(t),
            Err(e) => Ok(e),
        }
    }
}

impl CheckTransactionError for &TransactionError {
    type NoError = ();

    fn get_err(&self) -> Result<&TransactionError, &Self::NoError> {
        Ok(self)
    }
}

impl CheckTransactionError for &Option<TransactionError> {
    type NoError = Option<TransactionError>;

    fn get_err(&self) -> Result<&TransactionError, &Self::NoError> {
        match self {
            None => Err(self),
            Some(e) => Ok(e),
        }
    }
}
