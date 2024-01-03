mod syscall_stubs;

use solana_runtime::accounts_index::ZeroLamport;
use solana_sdk::account::ReadableAccount;
use syscall_stubs::*;
use {
    log::*,
    solana_program_test::programs,
    solana_runtime::{
        bank::Bank, bank_forks::BankForks, genesis_utils::create_genesis_config_with_leader_ex,
        runtime_config::RuntimeConfig,
    },
    solana_sdk::{
        fee_calculator::{FeeRateGovernor, DEFAULT_TARGET_LAMPORTS_PER_SIGNATURE},
        genesis_config::ClusterType,
        native_token::sol_to_lamports,
        poh_config::PohConfig,
        pubkey::Pubkey,
        rent::Rent,
        signature::{Keypair, Signer},
    },
    solana_vote_program::vote_state::VoteState,
    std::{sync::Arc, time::Duration},
};
pub use {
    solana_banks_client::{BanksClient, BanksClientError},
    solana_banks_interface::BanksTransactionResultWithMetadata,
    solana_program_runtime::invoke_context::InvokeContext,
    solana_sdk::transaction_context::IndexOfAccount,
};

/// Copied from private method [ProgramTest::setup_bank],
/// but only returns a [BankForks] and is less configurable. These limitations
/// are due to the fact that we cannot directly use many private fields on a [ProgramTest].
/// Specifically, no feature deactivation, no runtime config, and no user built-ins.
/// User provided programs must be BPF programs added directly as account data.
pub fn setup_bank<'a, T>(accounts: impl IntoIterator<Item = (&'a Pubkey, &'a T)>) -> BankForks
where
    T: ReadableAccount + Sync + ZeroLamport + 'a,
{
    {
        use std::sync::Once;
        static ONCE: Once = Once::new();

        ONCE.call_once(|| {
            solana_sdk::program_stubs::set_syscall_stubs(Box::new(SyscallStubs {}));
        });
    }

    let rent = Rent::default();
    let fee_rate_governor = FeeRateGovernor {
        // Initialize with a non-zero fee
        lamports_per_signature: DEFAULT_TARGET_LAMPORTS_PER_SIGNATURE / 2,
        ..FeeRateGovernor::default()
    };
    let bootstrap_validator_pubkey = Pubkey::new_unique();
    let bootstrap_validator_stake_lamports =
        rent.minimum_balance(VoteState::size_of()) + sol_to_lamports(1_000_000.0);

    let mint_keypair = Keypair::new();
    let voting_keypair = Keypair::new();

    let mut genesis_config = create_genesis_config_with_leader_ex(
        sol_to_lamports(1_000_000.0),
        &mint_keypair.pubkey(),
        &bootstrap_validator_pubkey,
        &voting_keypair.pubkey(),
        &Pubkey::new_unique(),
        bootstrap_validator_stake_lamports,
        42,
        fee_rate_governor,
        rent,
        ClusterType::Development,
        vec![],
    );

    let target_tick_duration = Duration::from_micros(100);
    genesis_config.poh_config = PohConfig::new_sleep(target_tick_duration);
    debug!("Payer address: {}", mint_keypair.pubkey());
    debug!("Genesis config: {}", genesis_config);

    let bank = Bank::new_with_runtime_config_for_tests(
        &genesis_config,
        Arc::new(RuntimeConfig::default()),
    );

    // Add commonly-used SPL programs as a convenience to the user
    for (program_id, account) in programs::spl_programs(&Rent::default()).iter() {
        bank.store_account(program_id, account);
    }

    for (pubkey, account) in accounts {
        bank.store_account(pubkey, account);
    }

    bank.set_capitalization();
    // Advance beyond slot 0 for a slightly more realistic test environment
    let bank = {
        let bank = Arc::new(bank);
        bank.fill_bank_with_ticks_for_tests();
        let bank = Bank::new_from_parent(&bank, bank.collector_id(), bank.slot() + 1);
        debug!("Bank slot: {}", bank.slot());
        bank
    };
    BankForks::new(bank)
}
