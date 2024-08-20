// use serde::{de, ser::Error, Deserialize, Deserializer, Serialize, Serializer};
// use solana_sdk::account::Account;

// use super::account::DeserializedAccount;

// pub fn serialize<S>(my_type: &DeserializedAccount, serializer: S) -> Result<S::Ok, S::Error>
// where
//     S: Serializer,
// {
//     // my_type
//     //     .serialize()
//     //     .map_err(|e| S::Error::custom(e))?
//     //     .serialize(serializer)
//     todo!()
// }

// pub fn deserialize<'de, D>(deserializer: D) -> Result<, D::Error>
// where
//     D: Deserializer<'de>,
// {
//     // let s: Account = Deserialize::deserialize(deserializer)?;
//     // DeserializedAccount::try_account_data_to_value(s)
//     //     .map_err(|e| de::Error::custom(format!("Parse error: {e:?}")))
//     todo!()
// }
