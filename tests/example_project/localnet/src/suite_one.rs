use solana_sdk::pubkey;
use solana_devtools_localnet::{GeneratedAccount, LocalnetAccount, LocalnetConfiguration};
use solana_devtools_localnet::localnet_account::system_account::SystemAccount;
use solana_devtools_localnet::localnet_account::token::{Mint, TokenAccount};
use solana_sdk::pubkey::Pubkey;
use spl_token::solana_program::program_option::COption;

/// Use const values if you want to keep values fixed across test builds.
/// Otherwise `Pubkey::new_unique()` suffices.
pub const TEST_MINT: Pubkey = pubkey!("9WQV5oLq9ykMrqSj6zWrazr3SjFzbESXcVwZYttsd7XM");

pub struct Payer;
impl GeneratedAccount for Payer {
    type Data = SystemAccount;

    fn address(&self) -> spl_token::solana_program::pubkey::Pubkey {
        pubkey!("9VegcRe98qziwbKWdMQjrraUQB5HFCTW7M2vGbRScpUx")
    }

    fn generate(&self) -> Self::Data {
        SystemAccount
    }
}

/// Configure different test suites with separate [LocalnetConfiguration] instances.
pub fn configuration() -> LocalnetConfiguration {
    LocalnetConfiguration::with_outdir("./tests/suite-1")
        .accounts(accounts())
        .unwrap()
        .program(test_program::ID, "../target/deploy/test_program.so")
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
        Mint::from(spl_token::state::Mint {
            mint_authority: COption::Some(test_user.address),
            supply: 0,
            decimals: 9,
            is_initialized: true,
            freeze_authority: COption::Some(test_user.address),
        })
    ).set_owner(spl_token::ID);
    let test_token_account = LocalnetAccount::new(
        Pubkey::new_unique(),
        "test_user_token_act.json".to_string(),
        TokenAccount::from(spl_token::state::Account {
            mint: test_mint.address,
            owner: test_user.address,
            amount: 0,
            delegate: COption::None,
            state: spl_token::state::AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::Some(test_user.address)
        })
    ).set_owner(spl_token::ID);
    vec![
        Payer.to_localnet_account(),
        test_user,
        test_mint,
        test_token_account,
    ]
}
