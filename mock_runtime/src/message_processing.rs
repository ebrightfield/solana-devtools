use crate::MockSolanaRuntime;
use core::sync::atomic::Ordering;
use lazy_static::lazy_static;
use solana_program_runtime::compute_budget::ComputeBudget;
use solana_program_runtime::loaded_programs::LoadedProgramMatchCriteria;
use solana_program_runtime::loaded_programs::LoadedProgramType;
use solana_program_runtime::loaded_programs::WorkingSlot;
use solana_program_runtime::loaded_programs::{LoadedProgram, LoadedProgramsForTxBatch};
use solana_program_runtime::log_collector::LogCollector;
use solana_program_runtime::timings::ExecuteTimings;
use solana_runtime::builtins::BUILTINS;
use solana_runtime::message_processor::MessageProcessor;
use solana_sdk::account::ReadableAccount;
use solana_sdk::account::{Account, AccountSharedData};
use solana_sdk::account_utils::StateMut;
use solana_sdk::bpf_loader;
use solana_sdk::bpf_loader_deprecated;
use solana_sdk::bpf_loader_upgradeable;
use solana_sdk::bpf_loader_upgradeable::UpgradeableLoaderState;
use solana_sdk::instruction::InstructionError;
use solana_sdk::message::SanitizedMessage;
use solana_sdk::native_loader;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::slot_history::Slot;
use solana_sdk::sysvar;
use solana_sdk::sysvar::instructions::construct_instructions_data;
use solana_sdk::sysvar::rent::Rent;
use solana_sdk::transaction::Result as TransactionResult;
use solana_sdk::transaction::TransactionError;
use solana_sdk::transaction_context::TransactionContext;
use std::collections::{hash_map::Entry, HashMap};
use std::sync::Arc;

const PROGRAM_OWNERS: [Pubkey; 3] = [
    bpf_loader_upgradeable::ID,
    bpf_loader::ID,
    bpf_loader_deprecated::ID,
];

lazy_static! {
    static ref BUILTIN_PUBKEYS: Vec<Pubkey> =
        BUILTINS.iter().map(|b| b.program_id).collect::<Vec<_>>();
}

pub struct ProcessedMessage {
    pub accounts: HashMap<Pubkey, AccountSharedData>,
    pub compute_units: u64,
    pub logs: Vec<String>,
    /// If the transaction successfully loads but fails during execution,
    /// this will be a non-`None` value.
    pub execution_error: Option<TransactionError>,
}

impl ProcessedMessage {
    pub fn success(&self) -> bool {
        self.execution_error.is_none()
    }

    pub fn check_error_code<T: Into<u32>>(&self, instruction_index: u8, error_code: T) -> Result<(), &Option<TransactionError>> {
        if let Some(TransactionError::InstructionError(
            idx,
            err,
        )) = &self.execution_error {
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

    #[cfg(feature = "anchor")]
    pub fn get_account_as<T: anchor_lang::AccountDeserialize>(&self, pubkey: &Pubkey) -> Option<anchor_lang::Result<T>> {
        self.accounts
            .get(pubkey)
            .map(|act| {
                let mut data = act.data();
                T::try_deserialize(&mut data)
            })
    }
}

impl MockSolanaRuntime {
    pub fn process(&mut self, message: &SanitizedMessage) -> TransactionResult<ProcessedMessage> {
        let slot = self.sysvar_cache.get_clock().unwrap().slot;
        let (accounts, program_indices) = self.load_accounts(message)?;
        let loaded_programs = self.load_programs(slot, &message)?;
        let mut transaction_context = TransactionContext::new(accounts, None, 10, usize::MAX);

        let mut compute_units = 0;
        let mut timing = ExecuteTimings::default();

        let mut p1 = LoadedProgramsForTxBatch::new(slot);
        let mut p2 = LoadedProgramsForTxBatch::new(slot);
        //let log_collector = LogCollector::new_ref();
        let mut error = None;
        if let Err(e) = MessageProcessor::process_message(
            message,
            &program_indices,
            &mut transaction_context,
            Rent::default(),
            Some(self.logger.clone()),
            &loaded_programs,
            &mut p1,
            &mut p2,
            self.feature_set.clone(),
            ComputeBudget::default(),
            &mut timing,
            &self.sysvar_cache,
            *message.recent_blockhash(),
            0,
            0,
            &mut compute_units,
        ) {
            error = Some(e);
        }

        let keys = message.account_keys().iter().copied().collect::<Vec<_>>();
        let post_transaction_data: Vec<_> = transaction_context
            .deconstruct_without_keys()
            .map_err(|_| TransactionError::CallChainTooDeep)?;
        let accounts = HashMap::from_iter(keys.into_iter().zip(post_transaction_data));

        let logs = self.logger.borrow().get_recorded_content().to_vec();
        self.logger = LogCollector::new_ref();
        Ok(ProcessedMessage {
            accounts,
            compute_units,
            logs,
            execution_error: error,
        })
    }

    /// Will only update the accounts on execution success.
    pub fn process_and_update_accounts(
        &mut self,
        message: &SanitizedMessage,
    ) -> TransactionResult<ProcessedMessage> {
        let result = self.process(message)?;
        if result.success() {
            self.update_accounts(&result.accounts);
        }
        Ok(result)
    }

    fn load_accounts(
        &mut self,
        msg: &SanitizedMessage,
    ) -> TransactionResult<(Vec<(Pubkey, AccountSharedData)>, Vec<Vec<u16>>)> {
        let mut accounts =
            Vec::with_capacity(msg.account_keys().len() + msg.instructions().len() * 2);

        for &key in msg.account_keys().iter() {
            // Sysvars
            if solana_sdk::sysvar::instructions::check_id(&key) {
                let acc = Account {
                    data: construct_instructions_data(&msg.decompile_instructions()).into(),
                    owner: sysvar::id(),
                    ..Default::default()
                };
                accounts.push((key, acc.into()));
                continue;
            }
            if solana_sdk::sysvar::clock::check_id(&key) {
                let data = self.sysvar_cache.get_clock().unwrap();
                let acc = Account {
                    data: bincode::serialize(&data).unwrap(),
                    owner: sysvar::id(),
                    ..Default::default()
                };
                accounts.push((key, acc.into()));
                continue;
            }
            if solana_sdk::sysvar::rent::check_id(&key) {
                let data = self.sysvar_cache.get_rent().unwrap();
                let acc = Account {
                    lamports: 1009200,
                    data: bincode::serialize(&data).unwrap(),
                    owner: sysvar::id(),
                    rent_epoch: 361,
                    executable: false,
                };
                accounts.push((key, acc.into()));
                continue;
            }

            let account = self.get_account_or_default(&key);

            accounts.push((key, account));
        }

        let builtins_start_index = accounts.len();
        let mut program_indices = Vec::with_capacity(msg.instructions().len());
        'OUTER: for ix in msg.instructions() {
            let mut account_indices = Vec::new();
            let mut program_index = ix.program_id_index as usize;

            // In five iterations, we should bottom out at the native loader.
            for _ in 0..5 {
                let (program_id, program_account) = accounts
                    .get(program_index)
                    .ok_or(TransactionError::ProgramAccountNotFound)?;

                // push nothing if the program is native_loader
                if native_loader::check_id(program_id) {
                    program_indices.push(account_indices);
                    continue 'OUTER;
                }

                // push the program
                account_indices.insert(0, program_index as u16);

                // if the program owner is native loader, we're done
                let owner_id = program_account.owner();
                if native_loader::check_id(owner_id) {
                    program_indices.push(account_indices);
                    continue 'OUTER;
                }
                // otherwise look for the program owner,
                // which should probably be one of the BPF loaders,
                // which in turn should be owned by the native loader.
                program_index = match accounts
                    .get(builtins_start_index..)
                    .ok_or(TransactionError::ProgramAccountNotFound)?
                    .iter()
                    .position(|(key, _)| key == owner_id)
                {
                    Some(owner_index) => builtins_start_index.saturating_add(owner_index),
                    None => {
                        let owner_index = accounts.len();
                        let owner_account = self.get_account_or_default(owner_id);

                        accounts.push((*owner_id, owner_account));
                        owner_index
                    }
                };
            }

            // chain of program owners went too far
            return Err(TransactionError::CallChainTooDeep);
        }

        Ok((accounts, program_indices))
    }

    pub fn load_programs(
        &mut self,
        slot: Slot,
        message: &SanitizedMessage,
    ) -> TransactionResult<LoadedProgramsForTxBatch> {
        let mut programs_and_slots = HashMap::new();

        // Queue up any BPF loader programs for loading
        for &key in message.account_keys().iter() {
            let acc = self.get_account_or_default(&key);
            if PROGRAM_OWNERS.contains(&acc.owner()) {
                match programs_and_slots.entry(key) {
                    Entry::Vacant(e) => {
                        e.insert((LoadedProgramMatchCriteria::NoCriteria, 0));
                    }
                    Entry::Occupied(mut e) => e.get_mut().1 += 1,
                }
            }
        }
        // Queue up all built-in programs for loading
        for builtin_program in BUILTIN_PUBKEYS.iter() {
            programs_and_slots.insert(
                *builtin_program,
                (LoadedProgramMatchCriteria::NoCriteria, 0),
            );
        }

        // Load anything already
        let (mut loaded_programs_for_txs, missing_programs) = {
            self.loaded_programs
                .extract(&WrappedSlot(slot), programs_and_slots.into_iter())
        };

        // Load missing programs while global cache is unlocked
        let mut loaded_missing_programs = vec![];
        for (key, count) in missing_programs {
            let program = self.load_program(slot, &key)?;
            program.tx_usage_counter.store(count, Ordering::Relaxed);
            loaded_missing_programs.push((key, program))
        }

        // Lock the global cache again to replenish the missing programs
        for (key, program) in loaded_missing_programs {
            let (_, entry) = self.loaded_programs.replenish(key, program);
            // Use the returned entry as that might have been deduplicated globally
            loaded_programs_for_txs.replenish(key, entry);
        }

        Ok(loaded_programs_for_txs)
    }

    pub fn load_program(
        &mut self,
        slot: Slot,
        pubkey: &Pubkey,
    ) -> TransactionResult<Arc<LoadedProgram>> {
        let program = self.get_account_or_default(pubkey);

        let mut transaction_accounts = vec![(*pubkey, program)];
        let is_upgradeable_loader =
            bpf_loader_upgradeable::check_id(transaction_accounts[0].1.owner());
        if is_upgradeable_loader {
            let programdata_address = match transaction_accounts[0].1.state() {
                Ok(UpgradeableLoaderState::Program {
                    programdata_address,
                }) => programdata_address,
                _ => {
                    return Ok(Arc::new(LoadedProgram::new_tombstone(
                        slot,
                        LoadedProgramType::Closed,
                    )));
                }
            };

            let programdata_account = self.get_account_or_default(&programdata_address);

            transaction_accounts.push((programdata_address, programdata_account));
        }

        let mut transaction_context = TransactionContext::new(
            transaction_accounts,
            Some(sysvar::rent::Rent::default()),
            1,
            1,
        );
        let instruction_context = transaction_context.get_next_instruction_context().unwrap();
        instruction_context.configure(if is_upgradeable_loader { &[0, 1] } else { &[0] }, &[], &[]);
        transaction_context.push().unwrap();
        let instruction_context = transaction_context
            .get_current_instruction_context()
            .unwrap();
        let program = instruction_context
            .try_borrow_program_account(&transaction_context, 0)
            .unwrap();
        let programdata = if is_upgradeable_loader {
            Some(
                instruction_context
                    .try_borrow_program_account(&transaction_context, 1)
                    .unwrap(),
            )
        } else {
            None
        };
        let loaded_program = solana_bpf_loader_program::load_program_from_account(
            &self.feature_set,
            Some(self.logger.clone()), // log_collector
            &program,
            programdata.as_ref().unwrap_or(&program),
            self.environment.clone(),
        )
        .map(|(loaded_program, _)| loaded_program)
        .unwrap_or_else(|_| {
            Arc::new(LoadedProgram::new_tombstone(
                slot,
                LoadedProgramType::FailedVerification(self.environment.clone()),
            ))
        });
        Ok(loaded_program)
    }
}

pub struct WrappedSlot(pub Slot);
impl WorkingSlot for WrappedSlot {
    fn current_slot(&self) -> Slot {
        self.0
    }

    fn is_ancestor(&self, _: Slot) -> bool {
        true
    }
}


#[cfg(test)]
mod tests {
    use anchor_lang::error::ErrorCode;
    use solana_program::instruction::InstructionError;
    use solana_sdk::transaction::TransactionError;
    use crate::message_processing::ProcessedMessage;

    #[test]
    fn check_error_code() {
        let mut processed_message = ProcessedMessage {
            accounts: Default::default(),
            compute_units: 0,
            logs: vec![],
            execution_error: None,
        };
        assert_eq!(
            processed_message.check_error_code(0, 1000u32),
            Err(&None),
        );

        let err = TransactionError::CallChainTooDeep;
        processed_message.execution_error = Some(err.clone());
        assert_eq!(
            processed_message.check_error_code(0, 1000u32),
            Err(&Some(err)),
        );

        let err = TransactionError::InstructionError(1, InstructionError::Custom(1000));
        processed_message.execution_error = Some(err.clone());
        assert_eq!(
            processed_message.check_error_code(0, 1000u32),
            Err(&Some(err)),
        );

        let err = TransactionError::InstructionError(0, InstructionError::AccountAlreadyInitialized);
        processed_message.execution_error = Some(err.clone());
        assert_eq!(
            processed_message.check_error_code(0, 1000u32),
            Err(&Some(err)),
        );

        let err = TransactionError::InstructionError(0, InstructionError::Custom(1001));
        processed_message.execution_error = Some(err.clone());
        assert_eq!(
            processed_message.check_error_code(0, 1000u32),
            Err(&Some(err)),
        );

        let err = TransactionError::InstructionError(0, InstructionError::Custom(ErrorCode::AccountOwnedByWrongProgram.into()));
        processed_message.execution_error = Some(err.clone());
        assert_eq!(
            processed_message.check_error_code(0, 1000u32),
            Err(&Some(err)),
        );

        let err = TransactionError::InstructionError(0, InstructionError::Custom(1000));
        processed_message.execution_error = Some(err.clone());
        assert_eq!(
            processed_message.check_error_code(0, 1000u32),
            Ok(()),
        );
    }
}