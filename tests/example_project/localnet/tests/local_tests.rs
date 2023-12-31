use anchor_lang::{InstructionData, ToAccountMetas};
use lazy_static::lazy_static;
use solana_devtools_localnet::{GeneratedAccount, LocalnetConfiguration};
use solana_devtools_tx::TransactionSchema;
use solana_mock_runtime::MockSolanaRuntime;
use solana_sdk::instruction::{Instruction, InstructionError};
use solana_sdk::transaction::TransactionError;
use solana_sdk::{pubkey, system_instruction, sysvar};
use spl_associated_token_account::get_associated_token_address;
use spl_token::solana_program::pubkey::Pubkey;
use test_localnet::suite_one::{configuration, Payer, TEST_MINT};

// We can load the configuration just once and re-use it for many tests.
lazy_static! {
    static ref CONFIGURATION: LocalnetConfiguration = {
        // We need to move up a directory since this test will execute from the localnet crate folder
        configuration()
    };
}

/// Used below to demonstrate the control over whether to persist changes to account state
/// after processing messages.
const REUSED_PUBKEY: Pubkey = pubkey!("FgoH9wNBfW18Teg1aN7H3uhUiu8QkPK3jbC3MRJioh26");

#[test]
fn test1() {
    let suite = configuration();
    let mut mock_runtime: MockSolanaRuntime = (&suite).try_into().unwrap();
    let msg = [
        system_instruction::create_account(
            &Payer.address(),
            &REUSED_PUBKEY,
            2_000_000,
            82,
            &spl_token::ID,
        ),
        spl_token::instruction::initialize_mint2(
            &spl_token::ID,
            &REUSED_PUBKEY,
            &Payer.address(),
            None,
            5,
        )
        .unwrap(),
        Instruction::new_with_bytes(
            test_program::ID,
            &test_program::instruction::Initialize {}.data(),
            test_program::accounts::Initialize {
                mint: TEST_MINT,
                new_account: get_associated_token_address(&Payer.address(), &TEST_MINT),
                owner: Payer.address(),
                token_program: spl_token::ID,
                associated_token_program: spl_associated_token_account::ID,
                system_program: Pubkey::default(),
                rent: sysvar::rent::ID,
            }
            .to_account_metas(None),
        ),
    ]
    .sanitized_message(Some(&Payer.address()))
    .unwrap();
    let result = mock_runtime.process(&msg).unwrap();
    println!("{:#?}", result.logs);
    assert!(result.execution_error.is_none());
    // Since the previous call does not save account mutations,
    // this second call will not fail with "AccountAlreadyExists".
    let result = mock_runtime.process_and_update_accounts(&msg).unwrap();
    assert!(result.execution_error.is_none());

    // But this one will fail
    let result = mock_runtime.process_and_update_accounts(&msg).unwrap();
    assert_eq!(
        TransactionError::InstructionError(0, InstructionError::Custom(0)),
        result.execution_error.unwrap(),
    );
}

#[test]
fn test2() {
    let suite = configuration();
    let mut mock_runtime: MockSolanaRuntime = (&suite).try_into().unwrap();
    let msg = [system_instruction::create_account(
        &Payer.address(),
        &REUSED_PUBKEY,
        1_000_000,
        10,
        &Pubkey::default(),
    )]
    .sanitized_message(Some(&Payer.address()))
    .unwrap();
    // This test uses an independent runtime instance,
    // so no matter the order of the cargo test execution, this will not fail.
    let result = mock_runtime.process(&msg).unwrap();
    assert!(result.execution_error.is_none());
}

#[test]
fn test3() {
    let suite = configuration();
    let mut mock_runtime: MockSolanaRuntime = (&suite).try_into().unwrap();
    let msg = [
        system_instruction::create_account(
            &Payer.address(),
            &REUSED_PUBKEY,
            1_000_000,
            5,
            &spl_token::ID,
        ),
        spl_token::instruction::initialize_mint2(
            &spl_token::ID,
            &REUSED_PUBKEY,
            &Payer.address(),
            None,
            5,
        )
        .unwrap(),
    ]
    .sanitized_message(Some(&Payer.address()))
    .unwrap();
    let result = mock_runtime.process_and_update_accounts(&msg).unwrap();
    // `process_and_update_accounts` will only update account state on successful transactions
    assert!(!result.success());
    assert_eq!(mock_runtime.get_account(&REUSED_PUBKEY), None);
}
