use solana_sdk::instruction::InstructionError;
use std::fmt::Debug;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InstructionErrorCheckFailure<'a, NoError: Debug> {
    #[error("no error: {0:?}")]
    NoError(&'a NoError),
    #[error("expected instruction error: {0}, found: {1}")]
    WrongVariant(InstructionError, &'a InstructionError),
    #[error("expected error: {0}, found: {1}")]
    WrongErrorCode(InstructionError, &'a InstructionError),
}

pub trait CheckInstructionError {
    type NoError: Debug;

    fn get_err(&self) -> Result<&InstructionError, &Self::NoError>;

    fn check_instruction_err(
        &self,
        expected: InstructionError,
    ) -> Result<&Self, InstructionErrorCheckFailure<Self::NoError>> {
        match self.get_err() {
            Err(e) => Err(InstructionErrorCheckFailure::NoError(e)),
            Ok(e) => {
                if *e != expected {
                    Err(InstructionErrorCheckFailure::WrongVariant(expected, e))
                } else {
                    Ok(self)
                }
            }
        }
    }

    fn check_instruction_err_code(
        &self,
        error_code: impl Into<u32>,
    ) -> Result<&Self, InstructionErrorCheckFailure<Self::NoError>> {
        match self.get_err() {
            Err(e) => Err(InstructionErrorCheckFailure::NoError(e)),
            Ok(e) => {
                let expected_error = InstructionError::from(Into::<u32>::into(error_code));
                if *e != expected_error {
                    Err(InstructionErrorCheckFailure::WrongErrorCode(
                        expected_error,
                        e,
                    ))
                } else {
                    Ok(self)
                }
            }
        }
    }

    fn map_expected_instruction_err<T>(
        &self,
        expected: &InstructionError,
        f: impl FnOnce(&InstructionError) -> T,
    ) -> Result<T, &Self> {
        if let Ok(e) = self.get_err() {
            if *e == *expected {
                return Ok(f(e));
            }
        }
        Err(self)
    }
}

impl<T: Debug> CheckInstructionError for Result<T, InstructionError> {
    type NoError = T;

    fn get_err(&self) -> Result<&InstructionError, &Self::NoError> {
        match self {
            Ok(t) => Err(t),
            Err(e) => Ok(e),
        }
    }
}

impl CheckInstructionError for &InstructionError {
    type NoError = ();

    fn get_err(&self) -> Result<&InstructionError, &Self::NoError> {
        Ok(self)
    }
}

impl CheckInstructionError for &Option<InstructionError> {
    type NoError = Option<InstructionError>;

    fn get_err(&self) -> Result<&InstructionError, &Self::NoError> {
        match self {
            None => Err(self),
            Some(e) => Ok(e),
        }
    }
}
