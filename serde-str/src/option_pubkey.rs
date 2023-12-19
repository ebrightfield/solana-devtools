use serde::{self, Deserialize, Deserializer, Serializer};
pub use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

pub fn serialize<S>(pubkey: &Option<Pubkey>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if let Some(pubkey) = pubkey {
        let s = format!("{}", pubkey);
        serializer.serialize_str(&s)
    } else {
        serializer.serialize_none()
    }
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Pubkey>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = Option::<String>::deserialize(deserializer)?;
    if let Some(s) = s {
        Ok(Some(
            Pubkey::from_str(&s).map_err(serde::de::Error::custom)?,
        ))
    } else {
        Ok(None)
    }
}
