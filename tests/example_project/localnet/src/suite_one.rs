use lazy_static::lazy_static;
use solana_devtools_localnet::{
    localnet_account::system_account::SystemAccount,
    localnet_account::token::{Mint, TokenAccount},
    GeneratedAccount, LocalnetAccount, LocalnetConfiguration,
};
use solana_sdk::{pubkey, pubkey::Pubkey, signature::Keypair, signature::Signer};

/// Use const values if you want to keep values fixed across test builds.
/// Otherwise `Pubkey::new_unique()` suffices.
pub const TEST_MINT: Pubkey = pubkey!("9WQV5oLq9ykMrqSj6zWrazr3SjFzbESXcVwZYttsd7XM");

lazy_static! {
    pub static ref PAYER_KEYPAIR: Keypair = Keypair::new();
}

pub struct Payer;
impl GeneratedAccount for Payer {
    type Data = SystemAccount;

    fn address(&self) -> Pubkey {
        PAYER_KEYPAIR.pubkey()
    }

    fn generate(&self) -> Self::Data {
        SystemAccount
    }

    fn name(&self) -> String {
        "payer.json".to_string()
    }
}

/// Configure different test suites with separate [LocalnetConfiguration] instances.
pub fn configuration() -> LocalnetConfiguration {
    LocalnetConfiguration::with_outdir("./tests/suite-1")
        .accounts(accounts())
        .unwrap()
        .program_binary_file(test_program::ID, "../target/deploy/test_program.so")
        .unwrap()
}

pub fn accounts() -> Vec<LocalnetAccount> {
    let test_user = LocalnetAccount::new(
        Pubkey::new_unique(),
        "test_user.json".to_string(),
        SystemAccount,
    );
    let test_mint = LocalnetAccount::new(
        TEST_MINT,
        "mint.json".to_string(),
        Mint::new(Some(test_user.address), 0, 9),
    )
    .owner(spl_token::ID);
    let test_token_account = LocalnetAccount::new(
        Pubkey::new_unique(),
        "test_user_token_act.json".to_string(),
        TokenAccount::new(test_mint.address, test_user.address, 0),
    )
    .owner(spl_token::ID);
    vec![
        Payer.to_localnet_account(),
        test_user,
        test_mint,
        test_token_account,
    ]
}
