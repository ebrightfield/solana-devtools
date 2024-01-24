use crate::error::{LocalnetConfigurationError, Result};
use crate::LocalnetAccount;
use anchor_lang::idl::IdlAccount;
use solana_devtools_anchor_utils::idl_sdk::{idl_parse, serialize_idl_account};
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;

pub struct LocalIdlAccount {
    data: Vec<u8>,
    program_id: Pubkey,
}

impl LocalIdlAccount {
    pub fn new_from_lib_rs(
        lib_rs: &str,
        version: &str,
        program_id: Pubkey,
        authority: Option<Pubkey>,
    ) -> Result<Self> {
        let idl = idl_parse(lib_rs, version.to_string(), false, false, false)
            .map_err(|e| LocalnetConfigurationError::IdlParseError(format!("{e}")))?;
        let data = serialize_idl_account(&idl, authority)
            .map_err(|e| LocalnetConfigurationError::IdlSerializationError(format!("{e}")))?;
        Ok(Self { data, program_id })
    }
}

impl Into<LocalnetAccount> for LocalIdlAccount {
    fn into(self) -> LocalnetAccount {
        LocalnetAccount {
            address: IdlAccount::address(&self.program_id),
            lamports: Rent::default().minimum_balance(self.data.len()),
            data: self.data,
            owner: self.program_id,
            executable: false,
            rent_epoch: 0,
            name: format!("{}_idl.json", self.program_id),
        }
    }
}
