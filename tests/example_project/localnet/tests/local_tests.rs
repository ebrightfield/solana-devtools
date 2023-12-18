use anchor_lang::InstructionData;
use lazy_static::lazy_static;
use solana_mock_runtime::MockSolanaRuntime;
use solana_devtools_localnet::{GeneratedAccount, LocalnetConfiguration};
use solana_devtools_tx::TransactionSchema;
use solana_sdk::instruction::{Instruction, InstructionError};
use solana_sdk::transaction::TransactionError;
use solana_sdk::{pubkey, system_instruction};
use spl_token::solana_program::pubkey::Pubkey;
use test_localnet::suite_one::{configuration, Payer};

// We can load the configuration just once and re-use it for many tests.
lazy_static! {
    static ref CONFIGURATION: LocalnetConfiguration = {
        // We need to move up a directory since this test will execute from the localnet crate folder
        configuration()
    };
}

const REUSED_PUBKEY: Pubkey = pubkey!("FgoH9wNBfW18Teg1aN7H3uhUiu8QkPK3jbC3MRJioh26");

#[test]
fn test1() {
    let suite = configuration();
    let mut mock_runtime: MockSolanaRuntime = (&suite).try_into().unwrap();
    let msg = [
        system_instruction::create_account(
            &Payer.address(),
            &REUSED_PUBKEY,
            1000,
            100,
            &Pubkey::default(),
        ),
        Instruction::new_with_bytes(
            test_program::ID,
            &test_program::instruction::Initialize {}.data(),
            vec![],
        ),
    ]
    .sanitized_message(Some(&Payer.address()));
    let result = mock_runtime.process(&msg).unwrap();
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
    let msg = [
        system_instruction::create_account(
            &Payer.address(),
            &REUSED_PUBKEY,
            1000,
            100,
            &Pubkey::default(),
        ),
        Instruction::new_with_bytes(
            test_program::ID,
            &test_program::instruction::Initialize {}.data(),
            vec![],
        ),
    ]
    .sanitized_message(Some(&Payer.address()));
    // This test uses an independent runtime instance,
    // so no matter the order of the cargo test execution, this will not fail.
    let result = mock_runtime.process(&msg).unwrap();
    assert!(result.execution_error.is_none());
}
