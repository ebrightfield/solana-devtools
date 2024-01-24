pub mod associated_token;
pub mod idl;
pub mod system_account;
pub mod token;

use anchor_lang::{error::Error, AccountSerialize, Owner};
use solana_program::rent::Rent;
use solana_sdk::account::Account;
use solana_sdk::account::WritableAccount;
use solana_sdk::pubkey::Pubkey;

pub use associated_token::AssociatedTokenAccount;
pub use system_account::SystemAccount;
pub use token::{Mint, TokenAccount};

pub trait GeneratedAccount {
    type Error;

    fn generate_account_data(&self) -> Result<Vec<u8>, Self::Error>;

    fn owner(&self) -> Pubkey;

    fn to_account(&self) -> Result<Account, Self::Error> {
        let data = self.generate_account_data()?;
        let lamports = Rent::default().minimum_balance(data.len());
        Ok(Account::create(lamports, data, self.owner(), false, 0))
    }

    fn to_keyed_account(&self) -> Result<(Pubkey, Account), Self::Error> {
        Ok((Pubkey::new_unique(), self.to_account()?))
    }

    fn to_account_with_lamports(&self, lamports: u64) -> Result<Account, Self::Error> {
        let mut act = self.to_account()?;
        act.set_lamports(lamports);
        Ok(act)
    }
}

impl<T: AccountSerialize + Owner> GeneratedAccount for T {
    type Error = Error;

    fn generate_account_data(&self) -> Result<Vec<u8>, Self::Error> {
        let mut data = vec![];
        self.try_serialize(&mut data)?;
        Ok(data)
    }

    fn owner(&self) -> Pubkey {
        T::owner()
    }
}
