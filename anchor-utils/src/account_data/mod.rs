pub mod associated_token;
pub mod idl;
pub mod system_account;
pub mod token;

use anchor_lang::{error::Error, AccountDeserialize, AccountSerialize, Owner};
use solana_program::rent::Rent;
use solana_sdk::{
    account::{Account, AccountSharedData, ReadableAccount, WritableAccount},
    pubkey::Pubkey,
};

pub use associated_token::AssociatedTokenAccount;
pub use system_account::{SigningSystemAccount, SystemAccount};
pub use token::{Mint, TokenAccount};

pub trait ToAnchorAccount {
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

    fn to_keyed_account_with_lamports(
        &self,
        lamports: u64,
    ) -> Result<(Pubkey, Account), Self::Error> {
        let (address, mut act) = self.to_keyed_account()?;
        act.set_lamports(lamports);
        Ok((address, act))
    }
}

impl<T: AccountSerialize + Owner> ToAnchorAccount for T {
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

pub trait FromAnchorAccount: Sized {
    type Error;

    fn from_account_data(data: &mut &[u8]) -> Result<Self, Self::Error>;

    fn from_account(account: &Account) -> Result<Self, Self::Error> {
        Self::from_account_data(&mut &account.data[..])
    }

    fn from_account_shared_data(account: &AccountSharedData) -> Result<Self, Self::Error> {
        Self::from_account_data(&mut account.data())
    }
}

impl<T: AccountDeserialize> FromAnchorAccount for T {
    type Error = Error;

    fn from_account_data(data: &mut &[u8]) -> Result<T, Self::Error> {
        T::try_deserialize(data)
    }
}
