use anchor_lang::prelude::System;
use anchor_lang::Id;
use solana_program::pubkey::Pubkey;

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

impl anchor_lang::AccountSerialize for SystemAccount {}
