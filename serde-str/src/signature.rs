use serde::{self, Deserialize, Deserializer, Serializer};
pub use solana_sdk::signature::Signature;
use std::str::FromStr;

pub fn serialize<S>(signature: &Signature, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = format!("{}", signature);
    serializer.serialize_str(&s)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Signature, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Signature::from_str(&s).map_err(serde::de::Error::custom)
}
