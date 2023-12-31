//! By default, serde attempts to (de-)serialize [Pubkey] and [Signature] to and from byte-arrays.
//! The human-friendly way to input or read these types is via [String] values.
//! This is a serde adaptor that allows for serializing and deserializing to Base58 strings.
//!
//! Usage:
//!
//! ```
//! use solana_sdk::pubkey::Pubkey;
//! use solana_sdk::signature::Signature;
//! use crate::solana_devtools_serde::{pubkey, option_signature};
//!
//! #[derive(serde::Serialize, serde::Deserialize)]
//!  pub struct MyStruct {
//!     /// Will convert to/from strings.
//!     #[serde(with = "pubkey")]
//!     pub address: Pubkey,
//!     /// Will convert to/from strings.
//!     #[serde(with = "option_signature")]
//!     pub signature: Option<Signature>,
//! }
//! ```
pub mod error;
pub mod option_pubkey;
pub mod option_signature;
pub mod pubkey;
pub mod signature;
