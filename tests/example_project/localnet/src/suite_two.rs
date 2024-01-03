use solana_client::rpc_client::RpcClient;
use solana_devtools_localnet::localnet_account::system_account::SystemAccount;
use solana_devtools_localnet::localnet_account::token::{Mint, TokenAccount};
use solana_devtools_localnet::{LocalnetAccount, LocalnetConfiguration};
use solana_sdk::pubkey;
use solana_sdk::pubkey::Pubkey;

pub fn configuration() -> LocalnetConfiguration {
    LocalnetConfiguration::with_outdir("./tests/suite-2")
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
        Pubkey::new_unique(),
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

    let usdc = LocalnetAccount::new_from_clone(
        &pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"),
        &RpcClient::new("https://api.mainnet-beta.solana.com".to_string()),
        "usdc_mint.json".to_string(),
        Some(|mint: Mint| mint.mint_authority(Some(test_user.address))),
    )
    .unwrap();

    // Instead of always cloning, you could obtain data with the Solana CLI
    // and include the data at build time:
    // solana -um account --output json --output-file usdt.json \
    //   Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB
    let usdt_str = include_str!("usdt.json");
    let data = serde_json::from_str(usdt_str).unwrap();
    let usdt = LocalnetAccount::from_ui_account(data, "usdt.json".to_string()).unwrap();
    vec![test_user, test_mint, usdc, usdt, test_token_account]
}
