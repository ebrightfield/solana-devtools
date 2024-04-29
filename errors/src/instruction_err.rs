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

    /// If the expected error is found, execute a function that
    /// maps it to `T`. Otherwise, pass through `self`.
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

impl CheckInstructionError for InstructionError {
    type NoError = ();

    fn get_err(&self) -> Result<&InstructionError, &Self::NoError> {
        Ok(self)
    }
}

impl CheckInstructionError for Option<InstructionError> {
    type NoError = Option<InstructionError>;

    fn get_err(&self) -> Result<&InstructionError, &Self::NoError> {
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
    fn instruction_error_checks() {
        let err = InstructionError::Custom(6000);
        err.check_instruction_err(err.clone()).unwrap();
        err.check_instruction_err_code(TestError::Variant1).unwrap();
        err.check_instruction_err_code(TestError::Variant1).unwrap();
        err.check_instruction_err_code(TestError::Variant2)
            .unwrap_err();

        InstructionError::AccountAlreadyInitialized
            .check_instruction_err(InstructionError::AccountAlreadyInitialized)
            .unwrap();
        InstructionError::AccountAlreadyInitialized
            .check_instruction_err(InstructionError::AccountDataTooSmall)
            .unwrap_err();

        let r: Result<(), InstructionError> = Ok(());
        r.check_instruction_err(err.clone()).unwrap_err();
        let r: Result<(), InstructionError> = Err(err.clone());
        r.check_instruction_err_code(TestError::Variant1).unwrap();

        let opt: Option<InstructionError> = None;
        opt.check_instruction_err(err.clone()).unwrap_err();
        let opt: Option<InstructionError> = Some(err.clone());
        opt.check_instruction_err_code(TestError::Variant1).unwrap();
        opt.check_instruction_err(err.clone()).unwrap();
    }
}
