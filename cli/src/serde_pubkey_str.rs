//! By default, serde attempts to (de-)serialize [Pubkey] to and from byte-arrays.
//! The preferred means for humans input or read pubkeys is via [String] values.
//! This is a serde adaptor that allows for serializing and deserializing to Base58 strings.
//! Usage:
//!
//! ```
//! use solana_sdk::pubkey::Pubkey;
//! use crate::serde_pubkey_str;
//!
//! #[derive(serde::Serialize, serde::Deserialize)]
//!  pub struct ImportantThing {
//!     #[serde(with = "serde_pubkey_str")]
//!     pub address: Pubkey,  // Will convert to/from strings.
//! }
//! ```
///
use std::fmt::Formatter;
use std::str::FromStr;
use serde::{Deserialize, Deserializer, Serializer};
use serde::de::{Expected, Unexpected};
use solana_sdk::pubkey::Pubkey;

pub fn serialize<S>(pubkey: &Pubkey, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
{
    let s = pubkey.to_string();
    serializer.serialize_str(&s)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Pubkey, D::Error>
    where
        D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Pubkey::from_str(&s).map_err( |_|
        serde::de::Error::invalid_value(
            Unexpected::Str(&s),
            &InvalidPubkey::new(s.to_owned()),
        )
    )
}

pub struct InvalidPubkey {
    addr: String,
}

impl InvalidPubkey {
    pub fn new(addr: String) -> Self {
        Self {
            addr: addr,
        }
    }
}

impl Expected for InvalidPubkey {
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str(&format!("{} is not a valid public key", &self.addr))
    }
}