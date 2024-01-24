use crate::generated_account::GeneratedAccount;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;
use solana_sdk::account::{Account, WritableAccount};

pub struct SystemAccount(u64);

impl SystemAccount {
    pub fn new(lamports: u64) -> Self {
        Self(lamports)
    }

    pub fn lamports(&self) -> u64 {
        self.0
    }

    pub fn set_lamports(&mut self, lamports: u64) {
        self.0 = lamports;
    }
}

impl GeneratedAccount for SystemAccount {
    type Error = ();

    fn generate_account_data(&self) -> Result<Vec<u8>, Self::Error> {
        Ok(vec![])
    }

    fn owner(&self) -> Pubkey {
        system_program::ID
    }

    fn to_account(&self) -> Result<Account, Self::Error> {
        Ok(Account::create(
            self.0,
            vec![],
            system_program::ID,
            false,
            0,
        ))
    }
}
