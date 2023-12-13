use std::str::FromStr;
use serde::{Deserialize, Deserializer, Serializer};
use serde::de::Unexpected;
use solana_sdk::pubkey::Pubkey;
use crate::error::InvalidPubkey;

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
