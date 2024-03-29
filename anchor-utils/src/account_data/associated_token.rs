use crate::account_data::{ToAnchorAccount, TokenAccount};
use anchor_lang::prelude::Error;
use solana_program::pubkey::Pubkey;
use solana_sdk::account::Account;
use spl_associated_token_account::get_associated_token_address;

pub struct AssociatedTokenAccount {
    pub mint: Pubkey,
    pub owner: Pubkey,
    pub balance: u64,
}

impl AssociatedTokenAccount {
    pub fn new(mint: Pubkey, owner: Pubkey, balance: u64) -> Self {
        Self {
            mint,
            owner,
            balance,
        }
    }

    pub fn address(&self) -> Pubkey {
        get_associated_token_address(&self.owner, &self.mint)
    }
}

impl ToAnchorAccount for AssociatedTokenAccount {
    type Error = Error;

    fn generate_account_data(&self) -> Result<Vec<u8>, Self::Error> {
        TokenAccount::new(self.mint, self.owner, self.balance).generate_account_data()
    }

    fn to_keyed_account(&self) -> Result<(Pubkey, Account), Self::Error> {
        Ok((self.address(), self.to_account()?))
    }

    fn owner(&self) -> Pubkey {
        spl_token::ID
    }
}
