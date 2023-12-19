use crate::error::InvalidPubkey;
use serde::de::Unexpected;
use serde::{Deserialize, Deserializer, Serializer};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

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
    Pubkey::from_str(&s).map_err(|_| {
        serde::de::Error::invalid_value(Unexpected::Str(&s), &InvalidPubkey::new(s.to_owned()))
    })
}
