use solana_cli_output::CliAccount;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_response::RpcKeyedAccount;
use solana_devtools_anchor_utils::client::account::get_state_blocking;
use solana_devtools_anchor_utils::generated_account::{
    AssociatedTokenAccount, GeneratedAccount, Mint, SystemAccount,
};
use solana_sdk::account::Account;
use solana_sdk::pubkey;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;

const USDC_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");

pub fn accounts() -> HashMap<Pubkey, Account> {
    let test_user_address = Pubkey::new_unique();
    let test_user = SystemAccount::new(1_000_000_000);
    let test_mint_address = Pubkey::new_unique();
    let test_mint = Mint::new(Some(test_user_address), 0, 9);
    let user_ata = AssociatedTokenAccount::new(test_mint_address, test_user_address, 0);

    // Example of fetching data from RPC
    let client = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
    let usdc_mint: Mint = get_state_blocking(&USDC_MINT, &client).unwrap();

    // Instead of always cloning, you could obtain data with the Solana CLI
    // and include the data at build time:
    // solana -um account --output json --output-file usdt.json \
    //   Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB
    let usdt_str = include_str!("usdt.json");
    let CliAccount {
        keyed_account:
            RpcKeyedAccount {
                pubkey: usdt_mint,
                account,
            },
        ..
    } = serde_json::from_str(usdt_str).unwrap();
    let usdt_mint = Pubkey::from_str(&usdt_mint).unwrap();
    let usdt_account: Account = account.decode().unwrap();

    HashMap::from([
        (test_user_address, test_user.to_account().unwrap()),
        (test_mint_address, test_mint.to_account().unwrap()),
        (user_ata.address(), user_ata.to_account().unwrap()),
        (USDC_MINT, usdc_mint.to_account().unwrap()),
        (usdt_mint, usdt_account),
    ])
}
