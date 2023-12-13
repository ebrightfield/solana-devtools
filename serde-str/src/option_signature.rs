use serde::{self, Deserialize, Deserializer, Serializer};
pub use solana_sdk::signature::Signature;
use std::str::FromStr;

pub fn serialize<S>(signature: &Option<Signature>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
{
    if let Some(s) = signature {
        let s = format!("{}", s);
        serializer.serialize_str(&s)
    } else {
        serializer.serialize_none()
    }
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Signature>, D::Error>
    where
        D: Deserializer<'de>,
{
    let s: Option<String> = Option::<String>::deserialize(deserializer)?;
    if let Some(s) = s {
        Ok(Some(
            Signature::from_str(&s).map_err(serde::de::Error::custom)?,
        ))
    } else {
        Ok(None)
    }
}
