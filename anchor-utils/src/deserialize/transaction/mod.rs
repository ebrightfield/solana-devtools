pub mod instruction;

use crate::deserialize::AnchorDeserializer;
use anyhow::Result;
pub use instruction::*;
use serde::{Deserialize, Serialize};
use solana_devtools_tx::inner_instructions::{DecompiledMessageAndInnerIx, HistoricalTransaction};
use solana_program::message::v0::LoadedAddresses;
use solana_program::message::VersionedMessage;

impl AnchorDeserializer {
    /// Deserializes a historical transaction's instructions, and any inner instructions.
    ///
    /// Provides instruction names, deserialized args, and decoded / validated
    /// account metas.
    ///
    /// There is a special [instruction::AccountMetaStatus] variant
    /// that flags whether a message's account meta signer + mutable flag disagrees with the IDL.
    /// This is not necessarily a privilege escalation error, unless the IDL calls for a higher
    /// privilege than the message grants on the account in question.
    pub fn try_deserialize_transaction(
        &self,
        tx: HistoricalTransaction,
    ) -> Result<DeserializedTransaction> {
        let mut instructions_deserialized = vec![];
        let mut decompiled: DecompiledMessageAndInnerIx = tx.into();

        for (ix_num, ix) in decompiled.top_level_instructions.iter_mut().enumerate() {
            let inner_ixs = decompiled.inner_instructions.get(&(ix_num as u8)).cloned();
            instructions_deserialized
                .push(self.try_deserialize_instruction(ix_num, ix, inner_ixs)?);
        }
        Ok(DeserializedTransaction(instructions_deserialized))
    }

    /// Deserialize just a transaction message, no inner instructions.
    pub fn try_deserialize_message(
        &self,
        message: VersionedMessage,
        loaded_addresses: Option<Vec<LoadedAddresses>>,
    ) -> Result<DeserializedTransaction> {
        let historical_tx = HistoricalTransaction::new(message, loaded_addresses);
        self.try_deserialize_transaction(historical_tx)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeserializedTransaction(Vec<DeserializedInstruction>);
