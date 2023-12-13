use std::collections::HashMap;
use solana_sdk::pubkey;
use solana_devtools_localnet::{LocalnetAccount, LocalnetConfiguration};
use solana_devtools_localnet::localnet_account::system_account::SystemAccount;
use solana_devtools_localnet::localnet_account::token::{Mint, TokenAccount};
use solana_sdk::pubkey::Pubkey;
use spl_token::solana_program::program_option::COption;

const TEST_MINT: Pubkey = pubkey!("9WQV5oLq9ykMrqSj6zWrazr3SjFzbESXcVwZYttsd7XM");

pub fn suite_1() -> LocalnetConfiguration {
    let mut programs = HashMap::new();
    programs.insert(test_program::ID, "target/deploy/test_program.so".to_string());
    LocalnetConfiguration::new(
        accounts(),
        programs,
        Some("./tests/suite-1".to_string()),
    ).unwrap()
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
        test_user,
        test_mint,
        test_token_account,
    ]
}
