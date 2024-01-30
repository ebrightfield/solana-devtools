use solana_accounts_db::accounts_index::ZeroLamport;
use solana_program::{
    bpf_loader_upgradeable,
    bpf_loader_upgradeable::UpgradeableLoaderState,
    clock::{Clock, Slot},
    instruction::InstructionError,
    message::VersionedMessage,
    pubkey::Pubkey,
};
use solana_runtime::{
    bank::{Bank, TransactionSimulationResult},
    bank_forks::BankForks,
};
use solana_sdk::{
    account::{Account, AccountSharedData, ReadableAccount},
    signature::Signature,
    transaction::{
        MessageHash, Result as TransactionResult, SanitizedTransaction, TransactionError,
        VersionedTransaction,
    },
};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

mod program_test_private_items;
use program_test_private_items::setup_bank;

/// Simulate transactions direct from messages, skipping signature verification.
/// It is therefore not a realistic test scenario, and permits many more
/// state changes that are not possible on-chain or even with [solana_program_test].
/// Its purpose is purely for the simulation of message processing
/// by interfacing directly with a [Bank]. It is not performance optimized.
/// For more realistic simulation of transaction processing, including signature verification,
/// use [solana_program_test].
pub struct TransactionSimulator {
    bank_forks: Arc<RwLock<BankForks>>,
}

impl TransactionSimulator {
    pub fn new() -> Self {
        let bank_forks = setup_bank::<Account>([]);
        Self { bank_forks }
    }

    pub fn new_with_accounts<'a, T>(accounts: impl IntoIterator<Item = (&'a Pubkey, &'a T)>) -> Self
    where
        T: ReadableAccount + Sync + ZeroLamport + 'a,
    {
        let bank_forks = setup_bank(accounts);
        Self { bank_forks }
    }

    pub fn working_bank(&self) -> Arc<Bank> {
        self.bank_forks.read().unwrap().working_bank()
    }

    pub fn get_account(&self, pubkey: &Pubkey) -> Option<AccountSharedData> {
        self.working_bank().get_account(pubkey)
    }

    pub fn update_account(&self, pubkey: &Pubkey, account: &AccountSharedData) {
        self.working_bank().store_account(pubkey, account)
    }

    pub fn update_accounts(&self, accounts: &HashMap<Pubkey, AccountSharedData>) {
        accounts.iter().for_each(|(pubkey, act)| {
            self.update_account(pubkey, act);
        })
    }

    pub fn add_bpf(&self, program_id: &Pubkey, data: &[u8]) {
        let lamports = self
            .working_bank()
            .get_minimum_balance_for_rent_exemption(data.len());
        self.update_account(
            program_id,
            &Account {
                lamports,
                data: data.to_vec(),
                owner: solana_sdk::bpf_loader::id(),
                executable: true,
                rent_epoch: 0,
            }
            .into(),
        );
    }

    pub fn add_bpf_upgradeable(&self, program_id: Pubkey, programdata: &[u8]) {
        let programdata_address =
            Pubkey::find_program_address(&[program_id.as_ref()], &bpf_loader_upgradeable::ID).0;
        let lamports = self
            .working_bank()
            .get_minimum_balance_for_rent_exemption(36);
        let program = Account {
            lamports,
            data: bincode::serialize(&UpgradeableLoaderState::Program {
                programdata_address,
            })
            .unwrap(),
            owner: bpf_loader_upgradeable::ID,
            executable: true,
            rent_epoch: 0,
        }
        .into();
        self.update_account(&program_id, &program);

        let mut data = bincode::serialize(&UpgradeableLoaderState::ProgramData {
            slot: 0,
            upgrade_authority_address: None,
        })
        .unwrap();
        data.resize(UpgradeableLoaderState::size_of_programdata_metadata(), 0);
        data.extend_from_slice(programdata);
        let lamports = self
            .working_bank()
            .get_minimum_balance_for_rent_exemption(data.len());
        let program_data = Account {
            lamports,
            data,
            owner: bpf_loader_upgradeable::ID,
            executable: true,
            rent_epoch: 0,
        }
        .into();
        self.update_account(&programdata_address, &program_data);
    }

    #[cfg(feature = "anchor")]
    pub fn get_anchor_account<T: anchor_lang::AccountDeserialize>(
        &self,
        pubkey: &Pubkey,
    ) -> Option<anchor_lang::Result<T>> {
        self.get_account(pubkey).map(|act| {
            let mut data = act.data();
            T::try_deserialize(&mut data)
        })
    }

    pub fn get_clock(&self) -> Clock {
        self.working_bank().clock()
    }

    pub fn set_clock(&self, clock: Clock) {
        let bank = self.working_bank();
        bank.set_sysvar_for_tests(&clock);
    }

    /// Update the clock slot or unix timestamp. To update the entire [Clock], use
    /// [MockSolanaRuntime::set_clock].
    pub fn update_clock(&self, slot: Option<Slot>, unix_timestamp: Option<i64>) {
        let bank = self.working_bank();
        let mut clock = bank.clock();
        if let Some(slot) = slot {
            clock.slot = slot;
        }
        if let Some(unix_timestamp) = unix_timestamp {
            clock.unix_timestamp = unix_timestamp;
        }
        bank.set_sysvar_for_tests(&clock);
    }

    /// Simulate the execution of a transaction message, bypassing signature verification.
    pub fn process_message(
        &self,
        mut message: VersionedMessage,
    ) -> TransactionResult<ProcessedMessage> {
        match &mut message {
            VersionedMessage::Legacy(m) => {
                m.recent_blockhash = self.working_bank().confirmed_last_blockhash();
            }
            VersionedMessage::V0(m) => {
                m.recent_blockhash = self.working_bank().confirmed_last_blockhash();
            }
        }
        let tx = VersionedTransaction {
            signatures: vec![],
            message,
        };
        let (bank, result) = self.simulate_transaction_unchecked(tx)?;
        let accounts = HashMap::from_iter(
            result
                .post_simulation_accounts
                .into_iter()
                .map(|a| (a.0, a.1)),
        );
        let execution_error = match result.result {
            Ok(_) => None,
            Err(e) => Some(e),
        };
        Ok(ProcessedMessage {
            accounts,
            compute_units: result.units_consumed,
            logs: result.logs,
            execution_error,
            slot: bank.slot(),
        })
    }

    /// Simulate the execution of a transaction message, bypassing signature verification,
    /// and if successful, update account state on the bank accordingly.
    /// This does not take the more realistic path to commit transactions to a bank,
    /// and instead just updates all non-executable accounts directly with [Bank::store_account].
    pub fn process_message_and_update_accounts(
        &self,
        message: VersionedMessage,
    ) -> TransactionResult<ProcessedMessage> {
        let result = self.process_message(message)?;
        if result.success() {
            result.accounts.iter().for_each(|act| {
                // Loaded transactions store a dummy account for executable accounts.
                // We therefore cannot update data based on this.
                if !act.1.executable() {
                    self.update_account(act.0, act.1);
                }
            });
        }
        Ok(result)
    }

    /// Skips signature verification. This is obviously not realistic,
    /// but makes it easier to test a wider array of situations. Use with caution.
    pub fn simulate_transaction_unchecked(
        &self,
        transaction: VersionedTransaction,
    ) -> TransactionResult<(Arc<Bank>, TransactionSimulationResult)> {
        let bank = self.working_bank();
        let sanitized_transaction = try_sanitize_unsigned_transaction(transaction, &*bank)?;
        let result = bank.simulate_transaction_unchecked(sanitized_transaction);
        Ok((bank, result))
    }
}

/// The result of a simulated transaction execution.
#[derive(Debug, Clone)]
pub struct ProcessedMessage {
    pub accounts: HashMap<Pubkey, AccountSharedData>,
    pub compute_units: u64,
    pub logs: Vec<String>,
    /// If the transaction successfully loads but fails during execution,
    /// this will be a non-`None` value.
    pub execution_error: Option<TransactionError>,
    pub slot: u64,
}

impl ProcessedMessage {
    pub fn success(&self) -> bool {
        self.execution_error.is_none()
    }

    pub fn check_error_code<T: Into<u32>>(
        &self,
        instruction_index: u8,
        error_code: T,
    ) -> Result<(), &Option<TransactionError>> {
        if let Some(TransactionError::InstructionError(idx, err)) = &self.execution_error {
            if *idx != instruction_index {
                return Err(&self.execution_error);
            }
            if let InstructionError::Custom(code) = err {
                if *code != error_code.into() {
                    return Err(&self.execution_error);
                }
            } else {
                return Err(&self.execution_error);
            }
            Ok(())
        } else {
            Err(&self.execution_error)
        }
    }

    pub fn get_account(&self, pubkey: &Pubkey) -> Option<&AccountSharedData> {
        self.accounts.get(pubkey)
    }

    #[cfg(feature = "anchor")]
    pub fn get_anchor_account<T: anchor_lang::AccountDeserialize>(
        &self,
        pubkey: &Pubkey,
    ) -> Option<anchor_lang::Result<T>> {
        self.accounts.get(pubkey).map(|act| {
            let mut data = act.data();
            T::try_deserialize(&mut data)
        })
    }
}

pub fn try_sanitize_unsigned_transaction(
    mut transaction: VersionedTransaction,
    bank: &Bank,
) -> TransactionResult<SanitizedTransaction> {
    match SanitizedTransaction::try_create(
        transaction.clone(),
        MessageHash::Compute,
        Some(false), // is_simple_vote_tx
        bank,
    ) {
        Err(e) => {
            // enforce the proper vec length for transaction.signatures.
            let len = transaction.message.header().num_required_signatures as usize;
            if len > 0 {
                let mut signatures = vec![Signature::default(); len];
                // add dummy signatures where applicable and try sanitizing again
                for i in 0..len {
                    let sig = transaction.signatures.get(i);
                    signatures[i] = if let Some(sig) = sig {
                        if *sig == Signature::default() {
                            Signature::new_unique()
                        } else {
                            *sig
                        }
                    } else {
                        Signature::new_unique()
                    }
                }
                transaction.signatures = signatures;
                // Every transaction should have at least one signature
                if transaction.signatures.is_empty() {
                    transaction.signatures = vec![Signature::new_unique()];
                }
                SanitizedTransaction::try_create(
                    transaction,
                    MessageHash::Compute,
                    Some(false), // is_simple_vote_tx
                    bank,
                )
            } else {
                return Err(e);
            }
        }
        Ok(tx) => Ok(tx),
    }
}
