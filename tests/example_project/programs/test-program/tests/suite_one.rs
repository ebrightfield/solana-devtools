use lazy_static::lazy_static;
use solana_cli_output::CliAccount;
use solana_client::rpc_response::RpcKeyedAccount;
use solana_devtools_anchor_utils::account_data::{
    AssociatedTokenAccount, Mint, SigningSystemAccount, SystemAccount, ToAnchorAccount,
    TokenAccount,
};
use solana_devtools_macros::named_pubkey;
use solana_sdk::{account::Account, pubkey, pubkey::Pubkey};
use std::collections::HashMap;
use std::str::FromStr;

pub const TEST_MINT: Pubkey = pubkey!("9WQV5oLq9ykMrqSj6zWrazr3SjFzbESXcVwZYttsd7XM");

lazy_static! {
    pub static ref PAYER: SigningSystemAccount = SigningSystemAccount::new();
}

pub fn accounts() -> HashMap<Pubkey, Account> {
    let test_user_address = named_pubkey!("testuser");
    let test_user = SystemAccount::new(1_000_000_000).to_account().unwrap();

    // Custom SPL types for conversion to an `Account`.
    let test_mint = Mint::new(Some(test_user_address), 0, 9)
        .to_account()
        .unwrap();

    // Fetching data from RPC
    // let client = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
    // let usdc_mint: Mint = get_state_blocking(&USDT_MINT, &client).unwrap();

    // Or obtain data with the Solana CLI, and import as static data
    let usdt_str = include_str!("fixtures/usdt.json");
    let CliAccount {
        keyed_account:
            RpcKeyedAccount {
                pubkey: usdt_addr,
                account: usdt_account,
            },
        ..
    } = serde_json::from_str(usdt_str).unwrap();
    let usdt_addr = Pubkey::from_str(&usdt_addr).unwrap();
    let usdt_account: Account = usdt_account.decode().unwrap();

    HashMap::from([
        (
            PAYER.address(),
            PAYER.to_account_with_lamports(1_000_000_000).unwrap(),
        ),
        (test_user_address, test_user),
        (TEST_MINT, test_mint),
        (usdt_addr, usdt_account),
        AssociatedTokenAccount::new(TEST_MINT, test_user_address, 0)
            .to_keyed_account()
            .unwrap(),
        (
            named_pubkey!("testUserAltTokenAct"),
            TokenAccount::new(TEST_MINT, test_user_address, 0)
                .to_account()
                .unwrap(),
        ),
    ])
}
