use crate::account_data::ToAnchorAccount;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;
use solana_sdk::{
    account::{Account, WritableAccount},
    signature::Keypair,
    signer::Signer,
};

/// Current minimum balance for accounts with `data_len = 0`.
pub const MINIMUM_BALANCE_FOR_RENT_EXEMPTION: u64 = 890880;

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

impl ToAnchorAccount for SystemAccount {
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

/// A system account an associated designated keypair.
pub struct SigningSystemAccount(Keypair);

impl SigningSystemAccount {
    pub fn new() -> Self {
        Self(Keypair::new())
    }

    pub fn address(&self) -> Pubkey {
        self.0.pubkey()
    }
}

impl ToAnchorAccount for SigningSystemAccount {
    type Error = ();
    fn generate_account_data(&self) -> Result<Vec<u8>, Self::Error> {
        Ok(vec![])
    }

    fn owner(&self) -> Pubkey {
        system_program::ID
    }

    fn to_account(&self) -> Result<Account, Self::Error> {
        Ok(Account::create(
            MINIMUM_BALANCE_FOR_RENT_EXEMPTION,
            vec![],
            system_program::ID,
            false,
            0,
        ))
    }

    fn to_account_with_lamports(&self, lamports: u64) -> Result<Account, Self::Error> {
        Ok(Account::create(
            lamports,
            vec![],
            system_program::ID,
            false,
            0,
        ))
    }

    fn to_keyed_account(&self) -> Result<(Pubkey, Account), Self::Error> {
        Ok((
            self.address(),
            Account::create(
                MINIMUM_BALANCE_FOR_RENT_EXEMPTION,
                vec![],
                system_program::ID,
                false,
                0,
            ),
        ))
    }
}

impl Signer for SigningSystemAccount {
    fn try_pubkey(&self) -> Result<Pubkey, solana_sdk::signer::SignerError> {
        self.0.try_pubkey()
    }

    fn try_sign_message(
        &self,
        message: &[u8],
    ) -> Result<solana_sdk::signature::Signature, solana_sdk::signer::SignerError> {
        self.0.try_sign_message(message)
    }

    fn is_interactive(&self) -> bool {
        self.0.is_interactive()
    }
}
