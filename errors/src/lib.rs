#[cfg(feature = "anchor-lang")]
pub mod anchor_lang_err;
pub mod instruction_err;
pub mod transaction_err;

#[cfg(feature = "solana-program-test")]
pub mod banks_client_err;
#[cfg(feature = "solana-client")]
pub mod client_err;

#[cfg(feature = "solana-program")]
use solana_program;

// TODO Macro for calculating the number of error code variants,
//     and impl TryFrom<u32>

/// ```rust
/// use anchor_lang::prelude::error_code;
/// use solana_devtools_errors::ErrorCause;
///
/// #[error_code]
/// pub enum MyProgramError {
///     Variant1,
///     Variant2,
/// }
///
/// impl ErrorCause for MyProgramError {}
///
/// /// The provided cause will be logged in Solana program logs
/// /// and via the `log` crate.
/// pub fn foo(bar: bool) -> Result<(), MyProgramError> {
///     if bar {
///         return Err(MyProgramError::Variant1.with_cause("some detail"));
///     }
///     Ok(())
/// }
/// ```
pub trait ErrorCause: Sized {
    /// Useful when returning Anchor `#[error]` variants, which are untagged enums.
    /// Used with `Result::map_err` and similar call-sites.
    #[allow(unused_variables)]
    #[inline]
    fn with_operands(self, op1: impl std::fmt::Display, op2: impl std::fmt::Display) -> Self {
        self.with_cause(format!("{op1}, {op2}"))
    }

    /// Useful when returning Anchor `#[error]` variants, which are untagged enums.
    /// Used with `Result::map_err` and similar call-sites.
    #[allow(unused_variables)]
    #[inline]
    fn with_cause(self, cause: impl std::fmt::Display) -> Self {
        #[cfg(feature = "program-log")]
        solana_program::msg!("{}", cause);
        #[cfg(feature = "log")]
        log::error!("{}", cause);
        self
    }
}
