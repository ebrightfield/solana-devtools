use solana_program::pubkey::Pubkey;
use anchor_lang::prelude::System;
use std::io::Write;
use anchor_lang::Id;

/// Use this struct as type T for any [GeneratedAccount] or [ClonedAccount]
/// owned by `SystemProgram` (e.g. typical user accounts).
pub struct SystemAccount;

impl SystemAccount {
    pub const LEN: usize = 0;
}

impl anchor_lang::Owner for SystemAccount {
    fn owner() -> Pubkey {
        System::id()
    }
}

impl anchor_lang::AccountDeserialize for SystemAccount {
    fn try_deserialize_unchecked(_buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        Ok(SystemAccount)
    }
}

impl anchor_lang::AccountSerialize for SystemAccount {
    fn try_serialize<W: Write>(&self, _writer: &mut W) -> anchor_lang::Result<()> {
        Ok(())
    }
}
