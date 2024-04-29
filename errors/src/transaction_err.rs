use crate::instruction_err::CheckInstructionError;
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
    #[error("expected instruction error: {0}, found: {1}")]
    WrongErrorCode(InstructionError, &'a TransactionError),
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

    fn check_instruction_err_at_index(
        &self,
        instruction_index: Option<u8>,
        error_code: impl Into<u32>,
    ) -> Result<&Self, TransactionErrorCheckFailure<Self::NoError>> {
        match self.get_err() {
            Err(e) => Err(TransactionErrorCheckFailure::NoError(e)),
            Ok(e) => match e {
                TransactionError::InstructionError(idx, ix_err) => {
                    let error_code = error_code.into();
                    if let Some(instruction_index) = instruction_index {
                        if *idx != instruction_index {
                            return Err(TransactionErrorCheckFailure::WrongIndex(
                                instruction_index,
                                e,
                            ));
                        }
                    }
                    let expected_ix_err = InstructionError::from(error_code);
                    match ix_err.check_instruction_err(expected_ix_err.clone()) {
                        Ok(_) => Ok(self),
                        Err(_) => Err(TransactionErrorCheckFailure::WrongErrorCode(
                            expected_ix_err,
                            e,
                        )),
                    }
                }
                _ => Err(TransactionErrorCheckFailure::NotInstructionError(e)),
            },
        }
    }

    fn map_expected_error_code<T>(
        &self,
        instruction_index: Option<u8>,
        error_code: impl Into<u32>,
        f: impl FnOnce(&InstructionError) -> T,
    ) -> Result<T, &Self> {
        if let Ok(e) = self.get_err() {
            if let TransactionError::InstructionError(idx, err) = e {
                if let Some(instruction_index) = instruction_index {
                    if *idx != instruction_index {
                        return Err(self);
                    }
                }
                if err.check_instruction_err_code(error_code).is_ok() {
                    return Ok(f(err));
                }
            }
        }
        Err(self)
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

impl CheckTransactionError for TransactionError {
    type NoError = ();

    fn get_err(&self) -> Result<&TransactionError, &Self::NoError> {
        Ok(self)
    }
}

impl CheckTransactionError for Option<TransactionError> {
    type NoError = Option<TransactionError>;

    fn get_err(&self) -> Result<&TransactionError, &Self::NoError> {
        match self {
            None => Err(self),
            Some(e) => Ok(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub enum TestError {
        Variant1,
        Variant2,
    }

    impl Into<u32> for TestError {
        fn into(self) -> u32 {
            match self {
                TestError::Variant1 => 6000,
                TestError::Variant2 => 6001,
            }
        }
    }

    #[test]
    fn transaction_error_checks() {
        let tx_err = TransactionError::InstructionError(0, InstructionError::Custom(6000));
        tx_err.check_transaction_err(tx_err.clone()).unwrap();
        tx_err
            .check_instruction_err_at_index(Some(0), TestError::Variant1)
            .unwrap();
        tx_err
            .check_instruction_err_at_index(None, TestError::Variant1)
            .unwrap();
        tx_err
            .check_instruction_err_at_index(Some(1), TestError::Variant1)
            .unwrap_err();
        tx_err
            .check_instruction_err_at_index(Some(0), TestError::Variant2)
            .unwrap_err();

        let r: Result<(), TransactionError> = Ok(());
        r.check_transaction_err(tx_err.clone()).unwrap_err();
        let r: Result<(), TransactionError> = Err(tx_err.clone());
        r.check_transaction_err(tx_err.clone()).unwrap();
        r.check_instruction_err_at_index(Some(0), TestError::Variant1)
            .unwrap();

        let opt: Option<TransactionError> = None;
        let t = opt.map_expected_error_code(Some(0), TestError::Variant1, |_| "failure");
        assert_eq!(None, *t.unwrap_err());

        opt.check_transaction_err(tx_err.clone()).unwrap_err();
        let opt: Option<TransactionError> = Some(tx_err.clone());
        opt.check_instruction_err_at_index(Some(0), TestError::Variant1)
            .unwrap();
        opt.check_transaction_err(tx_err.clone()).unwrap();

        let t = opt
            .map_expected_error_code(Some(0), TestError::Variant1, |_| "success")
            .unwrap();
        assert_eq!("success", t);
    }
}
