use crate::account_data::ToAnchorAccount;
use crate::idl_sdk::{serialize_idl_account, AnchorIdlSdkError};
use anchor_lang::idl::IdlAccount;
use anchor_syn::idl::types::Idl;
use solana_sdk::pubkey::Pubkey;

/// Represents an IDL account as it is stored on-chain,
/// allowing local transaction parsing.
pub struct OnChainIdl {
    idl: Idl,
    authority: Option<Pubkey>,
    program_id: Pubkey,
}

impl OnChainIdl {
    pub fn address(&self) -> Pubkey {
        IdlAccount::address(&self.program_id)
    }
}

impl ToAnchorAccount for OnChainIdl {
    type Error = AnchorIdlSdkError;

    fn generate_account_data(&self) -> Result<Vec<u8>, Self::Error> {
        serialize_idl_account(&self.idl, self.authority)
    }

    fn owner(&self) -> Pubkey {
        self.program_id
    }
}
