use crate::deserialize::{AnchorDeserializer, IdlWithDiscriminators};
use anchor_syn::idl::types::IdlTypeDefinition;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use solana_sdk::account::{Account, ReadableAccount};

use super::discriminator::partition_discriminator_from_data;

/// Deserialized account data, tagged by program_name and account_type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountDataValue {
    pub program_name: String,
    pub docs: Option<Vec<String>>,
    pub discriminator: Vec<u8>,
    pub account_type: String,
    pub deserialized: Value,
}

impl AnchorDeserializer {
    /// Tries to deserialize an account, first trying with any IDL cached from the account's owner,
    /// and failing that, tries to deserialize using all other caches IDLs (order is indeterminate).
    pub fn try_account_data_to_value(&self, account: &Account) -> Result<AccountDataValue> {
        if let Some(idl) = self.idl_cache.get(&account.owner) {
            if let Ok((json, _)) = idl.try_account_to_value(account) {
                return Ok(json);
            }
        }
        // Brute force search all cached IDLs, trying to deserialize
        for (_, idl) in &self.idl_cache {
            if let Ok((json, _)) = idl.try_account_to_value(account) {
                return Ok(json);
            }
        }
        return Err(anyhow!(
            "could not deserialize account from any cached IDLs"
        ));
    }
}

pub trait AccountDataParser {
    type IdlTypeDef;

    fn try_account_data_to_value<'a>(
        &'a self,
        data: impl AsRef<[u8]>,
    ) -> anyhow::Result<(AccountDataValue, &Self::IdlTypeDef)>;

    /// Deserialize account data, returning a [DeserializedAccount] containing a [Value] with parsed fields.
    fn try_account_to_value(
        &self,
        account: &impl ReadableAccount,
    ) -> anyhow::Result<(AccountDataValue, &Self::IdlTypeDef)> {
        self.try_account_data_to_value(account.data())
    }
}

impl AccountDataParser for IdlWithDiscriminators {
    type IdlTypeDef = IdlTypeDefinition;

    fn try_account_data_to_value(
        &self,
        data: impl AsRef<[u8]>,
    ) -> anyhow::Result<(AccountDataValue, &Self::IdlTypeDef)> {
        let mut idl_type_defs = self.types.clone();
        idl_type_defs.extend_from_slice(&self.accounts);
        let (discriminator, data) = partition_discriminator_from_data::<8>(data.as_ref());
        let type_def = self.account_definitions.get(&discriminator).ok_or(anyhow!(
            "Could not match account data against any discriminator"
        ))?;
        let deserialized = self.deserialize_struct_or_enum(type_def, &mut &data[..])?;
        Ok((
            AccountDataValue {
                program_name: self.name.clone(),
                docs: type_def.docs.clone(),
                discriminator: discriminator.to_vec(),
                account_type: type_def.name.clone(),
                deserialized,
            },
            type_def,
        ))
    }
}
