use anchor_lang::{InstructionData, ToAccountMetas};
use lazy_static::lazy_static;
use solana_devtools_localnet::{GeneratedAccount, LocalnetConfiguration, TransactionSimulator};
use solana_devtools_tx::TransactionSchema;
use solana_program_test::ProgramTest;
use solana_sdk::instruction::{Instruction, InstructionError};
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::TransactionError;
use solana_sdk::{pubkey, system_instruction, sysvar};
use spl_associated_token_account::get_associated_token_address;
use spl_token::solana_program::pubkey::Pubkey;
use std::ops::Deref;
use std::sync::RwLock;
use test_localnet::suite_one::{configuration, Payer, PAYER_KEYPAIR, TEST_MINT};

// We can load the configuration just once and re-use it for many tests.
// It is wrapped in a RwLock because cargo tests will execute multi-threaded.
lazy_static! {
    static ref CONFIGURATION: RwLock<LocalnetConfiguration> = RwLock::new(configuration());
}

/// Used below to demonstrate the control over whether to persist changes to account state
/// after processing messages.
const REUSED_PUBKEY: Pubkey = pubkey!("FgoH9wNBfW18Teg1aN7H3uhUiu8QkPK3jbC3MRJioh26");

#[test]
fn transaction_simulators_conditionally_save_account_mutations() {
    let mock_runtime: TransactionSimulator = CONFIGURATION.read().unwrap().deref().into();
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
    .message(Some(&Payer.address()));
    let result = mock_runtime.process_message(msg.clone()).unwrap();
    assert!(result.execution_error.is_none());
    let result = mock_runtime
        .process_message_and_update_accounts(msg.clone())
        .unwrap();
    assert!(result.execution_error.is_none());

    // But this one will fail
    let result = mock_runtime
        .process_message_and_update_accounts(msg)
        .unwrap();
    assert_eq!(
        TransactionError::InstructionError(0, InstructionError::Custom(0)),
        result.execution_error.unwrap(),
    );
}

#[test]
fn transaction_simulators_are_independent() {
    let mock_runtime: TransactionSimulator = CONFIGURATION.read().unwrap().deref().into();
    let msg = [system_instruction::create_account(
        &Payer.address(),
        &REUSED_PUBKEY,
        1_000_000,
        10,
        &Pubkey::default(),
    )]
    .message(Some(&Payer.address()));
    // This test uses an independent runtime instance,
    // so no matter the order of the cargo test execution, this will not fail.
    let result = mock_runtime.process_message(msg).unwrap();
    assert!(result.execution_error.is_none());
}

#[test]
fn failed_simulations_never_update_account_state() {
    let mock_runtime: TransactionSimulator = CONFIGURATION.read().unwrap().deref().into();
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
    .message(Some(&Payer.address()));
    let result = mock_runtime
        .process_message_and_update_accounts(msg)
        .unwrap();
    // `process_and_update_accounts` will only update account state on successful transactions
    assert!(!result.success());
    assert_eq!(mock_runtime.get_account(&REUSED_PUBKEY), None);
}

#[tokio::test]
async fn using_program_test_instance() {
    let program_test: ProgramTest = CONFIGURATION.read().unwrap().deref().into();
    let (mut banks_client, _, hash) = program_test.start().await;

    let test_mint = Keypair::new();

    let msg = [
        system_instruction::create_account(
            &Payer.address(),
            &test_mint.pubkey(),
            2_000_000,
            82,
            &spl_token::ID,
        ),
        spl_token::instruction::initialize_mint2(
            &spl_token::ID,
            &test_mint.pubkey(),
            &Payer.address(),
            None,
            5,
        )
        .unwrap(),
        Instruction::new_with_bytes(
            test_program::ID,
            &test_program::instruction::Initialize {}.data(),
            test_program::accounts::Initialize {
                mint: test_mint.pubkey(),
                new_account: get_associated_token_address(&Payer.address(), &test_mint.pubkey()),
                owner: Payer.address(),
                token_program: spl_token::ID,
                associated_token_program: spl_associated_token_account::ID,
                system_program: Pubkey::default(),
                rent: sysvar::rent::ID,
            }
            .to_account_metas(None),
        ),
    ]
    .transaction(
        hash,
        Some(&Payer.address()),
        &vec![PAYER_KEYPAIR.deref(), &test_mint],
    );

    banks_client.send_transaction(msg).await.unwrap();
}

#[tokio::test]
async fn same_tx_with_new_program_test_instance() {
    let program_test: ProgramTest = CONFIGURATION.read().unwrap().deref().into();
    let (mut banks_client, _, hash) = program_test.start().await;

    let test_mint = Keypair::new();

    let msg = [
        system_instruction::create_account(
            &Payer.address(),
            &test_mint.pubkey(),
            2_000_000,
            82,
            &spl_token::ID,
        ),
        spl_token::instruction::initialize_mint2(
            &spl_token::ID,
            &test_mint.pubkey(),
            &Payer.address(),
            None,
            5,
        )
        .unwrap(),
        Instruction::new_with_bytes(
            test_program::ID,
            &test_program::instruction::Initialize {}.data(),
            test_program::accounts::Initialize {
                mint: test_mint.pubkey(),
                new_account: get_associated_token_address(&Payer.address(), &test_mint.pubkey()),
                owner: Payer.address(),
                token_program: spl_token::ID,
                associated_token_program: spl_associated_token_account::ID,
                system_program: Pubkey::default(),
                rent: sysvar::rent::ID,
            }
            .to_account_metas(None),
        ),
    ]
    .transaction(
        hash,
        Some(&Payer.address()),
        &vec![PAYER_KEYPAIR.deref(), &test_mint],
    );

    banks_client.send_transaction(msg).await.unwrap();
}
