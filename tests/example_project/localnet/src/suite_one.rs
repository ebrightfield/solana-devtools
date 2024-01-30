use lazy_static::lazy_static;
use solana_devtools_anchor_utils::account_data::{
    Mint, SystemAccount, ToAnchorAccount, TokenAccount,
};
use solana_sdk::{
    account::Account, pubkey, pubkey::Pubkey, signature::Keypair, signature::Signer, system_program,
};
use std::collections::HashMap;

/// Use const or static values if you want to keep values fixed across test builds
/// or use them multiple places in your code.
/// Otherwise `Pubkey::new_unique()` suffices.
pub const TEST_MINT: Pubkey = pubkey!("9WQV5oLq9ykMrqSj6zWrazr3SjFzbESXcVwZYttsd7XM");

lazy_static! {
    pub static ref PAYER_KEYPAIR: Keypair = Keypair::new();
}

/// This is just a trivial (not-useful) example of how you can
/// use your own structs to create locally generated accounts.
pub struct Payer;

impl Payer {
    pub fn address(&self) -> Pubkey {
        PAYER_KEYPAIR.pubkey()
    }
}

impl ToAnchorAccount for Payer {
    type Error = ();
    fn generate_account_data(&self) -> Result<Vec<u8>, Self::Error> {
        Ok(vec![])
    }

    fn owner(&self) -> Pubkey {
        system_program::ID
    }
}
pub fn accounts() -> HashMap<Pubkey, Account> {
    let test_user_address = Pubkey::new_unique();
    let test_user = SystemAccount::new(1_000_000_000).to_account().unwrap();

    // Custom SPL types for conversion to an `Account`.
    let test_mint = Mint::new(Some(test_user_address), 0, 9)
        .to_account()
        .unwrap();

    HashMap::from([
        // User-defined impls
        (
            Payer.address(),
            Payer.to_account_with_lamports(10_000_000_000).unwrap(),
        ),
        (test_user_address, test_user),
        // Using constants
        (TEST_MINT, test_mint),
        // Or let the `AnchorAccount` trait generate a unique pubkey
        TokenAccount::new(TEST_MINT, test_user_address, 0)
            .to_keyed_account()
            .unwrap(),
    ])
}
