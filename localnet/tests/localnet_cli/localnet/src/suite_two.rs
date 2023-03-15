use core::ops::Deref;
use solana_client::rpc_client::RpcClient;
use solana_sdk::program_option::COption;
use solana_sdk::pubkey;
use solana_sdk::pubkey::Pubkey;
use solana_devtools_localnet::from_anchor::token::{TokenAccount, Mint};
use solana_devtools_localnet::{TestTomlGenerator, LocalnetAccount, SystemAccount};


pub fn suite_2() -> TestTomlGenerator {
    TestTomlGenerator {
        save_directory: "./tests/suite-2".to_string(),
        test_file_glob: Some("./tests/suite-2/test.ts".to_string()),
        accounts: accounts(),
        ..Default::default()
    }
}

pub fn accounts() -> Vec<LocalnetAccount> {
    let test_user = LocalnetAccount::new(
        Pubkey::new_unique(),
        "test_user.json".to_string(),
        SystemAccount,
    );
    let test_mint = LocalnetAccount::new(
        Pubkey::new_unique(),
        "mint.json".to_string(),
        Mint::from(spl_token::state::Mint {
            mint_authority: COption::Some(test_user.address),
            supply: 0,
            decimals: 9,
            is_initialized: true,
            freeze_authority: COption::Some(test_user.address),
        })
    );
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
    );
    let usdc = LocalnetAccount::new_from_clone(
        &pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"),
        &RpcClient::new("https://api.mainnet-beta.solana.com".to_string()),
        "usdc_mint.json".to_string(),
        Some(|mint: Mint| {
            let mut mint: spl_token::state::Mint = mint.deref().clone();
            mint.mint_authority = COption::Some(test_user.address.clone());
            Mint::from(mint)
        })
    ).unwrap();
    vec![
        test_user,
        test_mint,
        usdc,
        test_token_account,
    ]
}
