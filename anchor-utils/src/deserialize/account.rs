use crate::deserialize::discriminator::partition_discriminator_from_data;
use crate::deserialize::{AnchorDeserializer, IdlWithDiscriminators};
use anchor_syn::idl::types::IdlTypeDefinition;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use solana_sdk::account::{Account, ReadableAccount};

/// A superset of [solana-account-decoder::UiAccount].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeserializedAccount {
    #[serde(flatten)]
    pub program_name: String,
    pub account_type: String,
    pub deserialized: Value,
}

impl IdlWithDiscriminators {
    /// Deserialize a slice of bytes to a [Value] by inferring its type from the account discriminator,
    /// and according to this Anchor IDL's type specification.
    pub fn try_account_data_to_value_inner<'a>(
        &'a self,
        data: &[u8],
    ) -> Result<(&'a IdlTypeDefinition, Value)> {
        let mut idl_type_defs = self.types.clone();
        idl_type_defs.extend_from_slice(&self.accounts);
        let (discriminator, data) = partition_discriminator_from_data(data);
        let type_def = self.account_definitions.get(&discriminator).ok_or(anyhow!(
            "Could not match account data against any discriminator"
        ))?;
        Ok((
            type_def,
            self.deserialize_struct_or_enum(type_def, &mut &data[..])?,
        ))
    }

    /// Deserialize account data, returning a [DeserializedAccount] containing a [Value] with parsed fields.
    pub fn try_account_data_to_value(
        &self,
        account: &Account,
    ) -> anyhow::Result<DeserializedAccount> {
        let (account_type, deserialized) = self.try_account_data_to_value_inner(account.data())?;
        Ok(DeserializedAccount {
            program_name: self.name.clone(),
            account_type: account_type.name.clone(),
            deserialized,
        })
    }
}

impl AnchorDeserializer {
    /// Tries to deserialize an account, first trying with any IDL cached from the account's owner,
    /// and failing that, tries to deserialize using all other caches IDLs (order is indeterminate).
    pub fn try_account_data_to_value(&self, account: &Account) -> Result<DeserializedAccount> {
        if let Some(idl) = self.idl_cache.get(&account.owner) {
            if let Ok(json) = idl.try_account_data_to_value(account) {
                return Ok(json);
            }
        }
        // Brute force search all cached IDLs, trying to deserialize
        for (_, idl) in &self.idl_cache {
            if let Ok(json) = idl.try_account_data_to_value(account) {
                return Ok(json);
            }
        }
        return Err(anyhow!(
            "could not deserialize account from any cached IDLs"
        ));
    }
}
