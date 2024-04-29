use anchor_lang::error::Error;
use std::fmt::Debug;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AnchorLangErrorCheckFailure<'a, NoError: Debug> {
    #[error("no error: {0:?}")]
    NoError(&'a NoError),
    #[error("expected error code: {0}, found: {1}")]
    WrongErrorCode(u32, &'a Error),
}

pub trait CheckAnchorLangError {
    type NoError: Debug;

    fn get_err(&self) -> Result<&Error, &Self::NoError>;

    /// Check an [Error].
    fn check_anchor_lang_error_code(
        &self,
        error_code: impl Into<u32>,
    ) -> Result<&Self, AnchorLangErrorCheckFailure<Self::NoError>> {
        match self.get_err() {
            Err(e) => Err(AnchorLangErrorCheckFailure::NoError(e)),
            Ok(e) => match e {
                Error::AnchorError(error) => {
                    let error_code = error_code.into();
                    if error.error_code_number != error_code {
                        Err(AnchorLangErrorCheckFailure::WrongErrorCode(error_code, e))
                    } else {
                        Ok(self)
                    }
                }
                Error::ProgramError(error) => {
                    let program_error_code: u64 = error.program_error.clone().into();
                    let error_code = error_code.into();
                    if program_error_code as u32 != error_code {
                        Err(AnchorLangErrorCheckFailure::WrongErrorCode(error_code, e))
                    } else {
                        Ok(self)
                    }
                }
            },
        }
    }

    /// Similar to a typical `map` method, but called conditionally
    /// only if a matching error code is found.
    fn map_anchor_lang_error_code<T>(
        &self,
        error_code: impl Into<u32>,
        f: impl FnOnce(&Self) -> T,
    ) -> Result<T, &Self> {
        self.check_anchor_lang_error_code(error_code)
            .map(f)
            .map_err(|_| self)
    }
}

impl<T: Debug> CheckAnchorLangError for Result<T, Error> {
    type NoError = T;

    fn get_err(&self) -> Result<&Error, &Self::NoError> {
        match self {
            Ok(t) => Err(t),
            Err(e) => Ok(e),
        }
    }
}

impl CheckAnchorLangError for Error {
    type NoError = ();

    fn get_err(&self) -> Result<&Error, &Self::NoError> {
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::error::{AnchorError, Error};
    use anchor_lang::prelude::ProgramError;

    #[test]
    fn check_anchor_lang_err() {
        let err: Error = ProgramError::IncorrectProgramId.into();
        err.check_anchor_lang_error_code(Into::<u64>::into(ProgramError::IncorrectProgramId) as u32).unwrap();

        let err: Error = AnchorError {
            error_name: "foo".to_string(),
            error_code_number: 6001,
            error_msg: "bar".to_string(),
            error_origin: None,
            compared_values: None,
        }
        .into();
        (&err).check_anchor_lang_error_code(6001u32).unwrap();
    }
}
