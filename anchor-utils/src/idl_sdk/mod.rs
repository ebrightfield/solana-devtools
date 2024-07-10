pub use anchor_syn::idl::parse::file::parse as idl_parse;
use anchor_syn::idl::types::Idl;
use anyhow::anyhow;
use std::collections::HashMap;

use solana_program::instruction::Instruction;
use solana_program::message::{Message, VersionedMessage};
use solana_program::pubkey::Pubkey;
use solana_sdk::account::Account;
use thiserror::Error;

pub mod account;
pub mod instructions;

use crate::deserialize::AnchorDeserializer;
pub use account::{deserialize_idl_account, serialize_idl_account};

/// Verify that an IDL successfully deserializes a set of instructions and accounts.
/// This is useful in tests to ensure that your data types are all accurately represented
/// in your IDL for all instructions and account types that you intend to expose.
pub fn verify_idl(
    idl: Idl,
    program_id: Pubkey,
    instructions: impl IntoIterator<Item = Instruction>,
    accounts: impl IntoIterator<Item = (Pubkey, Account)>,
) -> anyhow::Result<()> {
    let idls = HashMap::from_iter([(program_id, idl)]);
    let deser = AnchorDeserializer::new_with_idls(idls);
    for ix in instructions {
        deser
            .try_deserialize_message(
                VersionedMessage::Legacy(Message::new(&[ix.clone()], None)),
                None,
            )
            .map_err(|e| anyhow!("failed to deserialize instruction: {e}, {:?}", ix))?;
    }
    for (pubkey, account) in accounts {
        deser
            .try_account_data_to_value(&account)
            .map_err(|e| anyhow!("failed to deserialize account {pubkey}: {e}"))?;
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum AnchorIdlSdkError {
    #[error("failed to serialize IDL or IdlAccount")]
    SerializeError,
    #[error("failed to deserialize IDL or IdlAccount")]
    DeserializeError,
    #[error("failed to compress on-chain IDL account data")]
    CompressionError,
    #[error("failed to decompress on-chain IDL account data")]
    DecompressionError,
}
