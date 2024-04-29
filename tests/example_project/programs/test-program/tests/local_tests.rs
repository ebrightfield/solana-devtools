mod suite_one;

use anchor_lang::{InstructionData, ToAccountMetas};
use anchor_spl::token::spl_token;
use solana_devtools_simulator::TransactionSimulator;
use solana_devtools_tx::TransactionSchema;
use solana_program_test::ProgramTest;
use solana_sdk::instruction::{Instruction, InstructionError};
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::TransactionError;
use solana_sdk::{pubkey, system_instruction, sysvar};
use spl_associated_token_account::get_associated_token_address;
use spl_token::solana_program::pubkey::Pubkey;
use std::fs::File;
use std::io::Read;
use suite_one::{PAYER, TEST_MINT};

/// Configure different test suites with separate [TransactionSimulator] instances.
/// You could share instances with `lazy_static` or `OnceCell`.
pub fn transaction_simulator() -> TransactionSimulator {
    let simulator = TransactionSimulator::new_with_accounts(&suite_one::accounts());
    let mut test_program = vec![];
    let mut so_file = File::open("tests/fixtures/test_program.so").unwrap();
    so_file.read_to_end(&mut test_program).unwrap();
    simulator.add_bpf_upgradeable(test_program::ID, &test_program);
    simulator
}

/// We can also use `solana-program-test`.
pub fn program_test() -> ProgramTest {
    suite_one::accounts()
        .into_iter()
        .fold(ProgramTest::default(), |mut p, (pubkey, act)| {
            p.add_account(pubkey, act);
            p
        })
}

/// Used below to demonstrate the control over whether to persist changes to account state
/// after processing messages.
const REUSED_PUBKEY: Pubkey = pubkey!("FgoH9wNBfW18Teg1aN7H3uhUiu8QkPK3jbC3MRJioh26");

#[test]
fn transaction_simulators_conditionally_save_account_mutations() {
    let mock_runtime: TransactionSimulator = transaction_simulator();
    let msg = [
        system_instruction::create_account(
            &PAYER.address(),
            &REUSED_PUBKEY,
            2_000_000,
            82,
            &spl_token::ID,
        ),
        spl_token::instruction::initialize_mint2(
            &spl_token::ID,
            &REUSED_PUBKEY,
            &PAYER.address(),
            None,
            5,
        )
        .unwrap(),
        Instruction::new_with_bytes(
            test_program::ID,
            &test_program::instruction::Initialize {}.data(),
            test_program::accounts::Initialize {
                mint: TEST_MINT,
                new_account: get_associated_token_address(&PAYER.address(), &TEST_MINT),
                owner: PAYER.address(),
                token_program: spl_token::ID,
                associated_token_program: spl_associated_token_account::ID,
                system_program: Pubkey::default(),
                rent: sysvar::rent::ID,
            }
            .to_account_metas(None),
        ),
    ]
    .message(Some(&PAYER.address()));
    let result = mock_runtime.process_message(msg.clone()).unwrap();
    assert!(result.execution_error.is_none(), "{:#?}", result.logs);
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
    let mock_runtime: TransactionSimulator = transaction_simulator();
    mock_runtime.update_clock(Some(1000), None);

    let msg = [system_instruction::create_account(
        &PAYER.address(),
        &REUSED_PUBKEY,
        1_000_000,
        10,
        &Pubkey::default(),
    )]
    .message(Some(&PAYER.address()));
    // This test uses an independent runtime instance,
    // so no matter the order of the cargo test execution, this will not fail.
    let result = mock_runtime.process_message(msg).unwrap();
    assert!(result.execution_error.is_none());
}

#[test]
fn failed_simulations_never_update_account_state() {
    let mock_runtime: TransactionSimulator = transaction_simulator();
    let msg = [
        system_instruction::create_account(
            &PAYER.address(),
            &REUSED_PUBKEY,
            1_000_000,
            5,
            &spl_token::ID,
        ),
        spl_token::instruction::initialize_mint2(
            &spl_token::ID,
            &REUSED_PUBKEY,
            &PAYER.address(),
            None,
            5,
        )
        .unwrap(),
    ]
    .message(Some(&PAYER.address()));
    let result = mock_runtime
        .process_message_and_update_accounts(msg)
        .unwrap();
    // `process_and_update_accounts` will only update account state on successful transactions
    assert!(!result.success());
    assert_eq!(mock_runtime.get_account(&REUSED_PUBKEY), None);
}

#[tokio::test]
async fn using_program_test_instance() {
    let program_test: ProgramTest = program_test();
    let (mut banks_client, _, hash) = program_test.start().await;

    let test_mint = Keypair::new();

    let msg = [
        system_instruction::create_account(
            &PAYER.address(),
            &test_mint.pubkey(),
            2_000_000,
            82,
            &spl_token::ID,
        ),
        spl_token::instruction::initialize_mint2(
            &spl_token::ID,
            &test_mint.pubkey(),
            &PAYER.address(),
            None,
            5,
        )
        .unwrap(),
        Instruction::new_with_bytes(
            test_program::ID,
            &test_program::instruction::Initialize {}.data(),
            test_program::accounts::Initialize {
                mint: test_mint.pubkey(),
                new_account: get_associated_token_address(&PAYER.address(), &test_mint.pubkey()),
                owner: PAYER.address(),
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
        Some(&PAYER.address()),
        &vec![&PAYER as &dyn Signer, &test_mint as &dyn Signer],
    );

    banks_client.send_transaction(msg).await.unwrap();
}

#[tokio::test]
async fn account_mutations_isolated() {
    let program_test: ProgramTest = program_test();
    let (mut banks_client, _, hash) = program_test.start().await;

    let test_mint = Keypair::new();

    let msg = [
        system_instruction::create_account(
            &PAYER.address(),
            &test_mint.pubkey(),
            2_000_000,
            82,
            &spl_token::ID,
        ),
        spl_token::instruction::initialize_mint2(
            &spl_token::ID,
            &test_mint.pubkey(),
            &PAYER.address(),
            None,
            5,
        )
        .unwrap(),
        Instruction::new_with_bytes(
            test_program::ID,
            &test_program::instruction::Initialize {}.data(),
            test_program::accounts::Initialize {
                mint: test_mint.pubkey(),
                new_account: get_associated_token_address(&PAYER.address(), &test_mint.pubkey()),
                owner: PAYER.address(),
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
        Some(&PAYER.address()),
        &vec![&PAYER as &dyn Signer, &test_mint as &dyn Signer],
    );

    banks_client.send_transaction(msg).await.unwrap();
}
