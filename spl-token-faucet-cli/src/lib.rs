use anchor_spl::token::Token;
use anchor_lang::Id;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcProgramAccountsConfig;
use solana_client::rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType};
use solana_sdk::account::Account;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::{pubkey, sysvar};
use solana_sdk::program_pack::Pack;
use solana_sdk::system_instruction::create_account;
use spl_token::instruction::initialize_mint;
use spl_token::state::Mint;
use spl_token_faucet::state::Faucet;

pub const FAUCET_PROGRAM: Pubkey = pubkey!("4bXpkKSV8swHSnwqtzuboGPaPDeEgAn4Vt8GfarV5rZt");
pub const FAUCET_MINT_AUTH: Pubkey = pubkey!("Fx1bCAyYpLMPVAjfq1pxbqKKkvDR3iYEpam1KbThRDYQ");

// TODO Close faucet

// An SPL instruction that initializes a mint configured to the required mint authority
// to become a faucet mint.
pub fn init_faucet_mint(mint: Pubkey, payer: Pubkey, decimals: u8) -> Vec<Instruction> {
    let space = Mint::LEN;
    //let rent = 696*space + 89088;
    let rent = 1461600u64;
    let ix1 = create_account(
        &payer,
        &mint,
        rent,
        space as u64,
        &Token::id(),
    );
    let ix2 = initialize_mint(&Token::id(), &mint, &FAUCET_MINT_AUTH, None, decimals).unwrap();
    vec![ix1, ix2]
}

/// Initialize a program-owned account to use in the `init_faucet` instruction.
pub fn init_faucet_account(faucet: Pubkey, payer: Pubkey) -> Instruction {
    let space = Faucet::get_packed_len();
    //let rent = 696*space + 89088;
    let rent = 1426800u64;
    create_account(
        &payer,
        &faucet,
        rent as u64,
        space as u64,
        &FAUCET_PROGRAM,
    )
}

// TODO Admin
/// Initialize a new SPL-token faucet, with a maximum
/// minted amount per request.
/// [mint.mint_authority] must equal [FAUCET_MINT_AUTH].
pub fn init_faucet_instruction(faucet_account: &Pubkey, mint: &Pubkey, amount: u64) -> Instruction {
    let mut data = Vec::with_capacity(9);
    data.push(0);
    data.extend_from_slice(&amount.to_le_bytes());

    let accounts = vec![
        AccountMeta::new_readonly(mint.clone(), false),
        AccountMeta::new(faucet_account.clone(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];
    Instruction {
        program_id: FAUCET_PROGRAM,
        accounts,
        data,
    }
}

// TODO Admin
/// Use the faucet, mint tokens.
pub fn mint_tokens_instruction(
    destination_account: &Pubkey,
    mint: &Pubkey,
    faucet: &Pubkey,
    amount: u64,
    program_id: &Option<Pubkey>,
) -> Instruction {
    let mut data = Vec::with_capacity(9);
    data.push(1);
    data.extend_from_slice(&amount.to_le_bytes());

    let program_id = program_id.unwrap_or(FAUCET_PROGRAM);
    let mint_auth = Pubkey::find_program_address(&["faucet".as_ref()], &program_id).0;

    let accounts = vec![
        AccountMeta::new_readonly(mint_auth, false),
        AccountMeta::new(mint.clone(), false),
        AccountMeta::new(destination_account.clone(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(faucet.clone(), false),
    ];
    Instruction {
        program_id,
        accounts,
        data,
    }
}

pub fn find_faucet(
    client: &RpcClient,
    mint: &Pubkey,
    program_id: &Option<Pubkey>,
) -> anyhow::Result<Vec<(Pubkey, Account)>> {
    let addresses = client.get_program_accounts_with_config(
        &program_id.unwrap_or(FAUCET_PROGRAM),
        RpcProgramAccountsConfig {
            filters: Some(vec![RpcFilterType::Memcmp(Memcmp {
                offset: 45,
                bytes: MemcmpEncodedBytes::Base58(mint.to_string()),
                encoding: None,
            })]),
            account_config: Default::default(),
            with_context: None,
        },
    )?;
    Ok(addresses)
}
