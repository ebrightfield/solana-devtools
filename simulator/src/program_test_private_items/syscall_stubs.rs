//! Directly copied from private items in [solana_program_test].
pub use tokio;
pub use {
    solana_banks_client::{BanksClient, BanksClientError},
    solana_banks_interface::BanksTransactionResultWithMetadata,
    solana_program_runtime::invoke_context::InvokeContext,
    solana_sdk::transaction_context::IndexOfAccount,
};
use {
    solana_program_runtime::{ic_msg, stable_log, timings::ExecuteTimings},
    solana_sdk::{
        account_info::AccountInfo,
        entrypoint::{ProgramResult, SUCCESS},
        instruction::{Instruction, InstructionError},
        program_error::{ProgramError, UNSUPPORTED_SYSVAR},
        pubkey::Pubkey,
        stable_layout::stable_instruction::StableInstruction,
        sysvar::Sysvar,
    },
    std::{cell::RefCell, convert::TryFrom, mem::transmute, sync::Arc},
};

thread_local! {
    static INVOKE_CONTEXT: RefCell<Option<usize>> = RefCell::new(None);
}
fn get_invoke_context<'a, 'b>() -> &'a mut InvokeContext<'b> {
    let ptr = INVOKE_CONTEXT.with(|invoke_context| match *invoke_context.borrow() {
        Some(val) => val,
        None => panic!("Invoke context not set!"),
    });
    unsafe { transmute::<usize, &mut InvokeContext>(ptr) }
}

fn get_sysvar<T: Default + Sysvar + Sized + serde::de::DeserializeOwned + Clone>(
    sysvar: Result<Arc<T>, InstructionError>,
    var_addr: *mut u8,
) -> u64 {
    let invoke_context = get_invoke_context();
    if invoke_context
        .consume_checked(invoke_context.get_compute_budget().sysvar_base_cost + T::size_of() as u64)
        .is_err()
    {
        panic!("Exceeded compute budget");
    }

    match sysvar {
        Ok(sysvar_data) => unsafe {
            *(var_addr as *mut _ as *mut T) = T::clone(&sysvar_data);
            SUCCESS
        },
        Err(_) => UNSUPPORTED_SYSVAR,
    }
}

pub struct SyscallStubs {}
impl solana_sdk::program_stubs::SyscallStubs for SyscallStubs {
    fn sol_log(&self, message: &str) {
        let invoke_context = get_invoke_context();
        ic_msg!(invoke_context, "Program log: {}", message);
    }

    fn sol_invoke_signed(
        &self,
        instruction: &Instruction,
        account_infos: &[AccountInfo],
        signers_seeds: &[&[&[u8]]],
    ) -> ProgramResult {
        let instruction = StableInstruction::from(instruction.clone());
        let invoke_context = get_invoke_context();
        let log_collector = invoke_context.get_log_collector();
        let transaction_context = &invoke_context.transaction_context;
        let instruction_context = transaction_context
            .get_current_instruction_context()
            .unwrap();
        let caller = instruction_context
            .get_last_program_key(transaction_context)
            .unwrap();

        stable_log::program_invoke(
            &log_collector,
            &instruction.program_id,
            invoke_context.get_stack_height(),
        );

        let signers = signers_seeds
            .iter()
            .map(|seeds| Pubkey::create_program_address(seeds, caller).unwrap())
            .collect::<Vec<_>>();

        let (instruction_accounts, program_indices) = invoke_context
            .prepare_instruction(&instruction, &signers)
            .unwrap();

        // Copy caller's account_info modifications into invoke_context accounts
        let transaction_context = &invoke_context.transaction_context;
        let instruction_context = transaction_context
            .get_current_instruction_context()
            .unwrap();
        let mut account_indices = Vec::with_capacity(instruction_accounts.len());
        for instruction_account in instruction_accounts.iter() {
            let account_key = transaction_context
                .get_key_of_account_at_index(instruction_account.index_in_transaction)
                .unwrap();
            let account_info_index = account_infos
                .iter()
                .position(|account_info| account_info.unsigned_key() == account_key)
                .ok_or(InstructionError::MissingAccount)
                .unwrap();
            let account_info = &account_infos[account_info_index];
            let mut borrowed_account = instruction_context
                .try_borrow_instruction_account(
                    transaction_context,
                    instruction_account.index_in_caller,
                )
                .unwrap();
            if borrowed_account.get_lamports() != account_info.lamports() {
                borrowed_account
                    .set_lamports(account_info.lamports())
                    .unwrap();
            }
            let account_info_data = account_info.try_borrow_data().unwrap();
            // The redundant check helps to avoid the expensive data comparison if we can
            match borrowed_account
                .can_data_be_resized(account_info_data.len())
                .and_then(|_| borrowed_account.can_data_be_changed())
            {
                Ok(()) => borrowed_account
                    .set_data_from_slice(&account_info_data)
                    .unwrap(),
                Err(err) if borrowed_account.get_data() != *account_info_data => {
                    panic!("{err:?}");
                }
                _ => {}
            }
            // Change the owner at the end so that we are allowed to change the lamports and data before
            if borrowed_account.get_owner() != account_info.owner {
                borrowed_account
                    .set_owner(account_info.owner.as_ref())
                    .unwrap();
            }
            if instruction_account.is_writable {
                account_indices.push((instruction_account.index_in_caller, account_info_index));
            }
        }

        let mut compute_units_consumed = 0;
        invoke_context
            .process_instruction(
                &instruction.data,
                &instruction_accounts,
                &program_indices,
                &mut compute_units_consumed,
                &mut ExecuteTimings::default(),
            )
            .map_err(|err| ProgramError::try_from(err).unwrap_or_else(|err| panic!("{}", err)))?;

        // Copy invoke_context accounts modifications into caller's account_info
        let transaction_context = &invoke_context.transaction_context;
        let instruction_context = transaction_context
            .get_current_instruction_context()
            .unwrap();
        for (index_in_caller, account_info_index) in account_indices.into_iter() {
            let borrowed_account = instruction_context
                .try_borrow_instruction_account(transaction_context, index_in_caller)
                .unwrap();
            let account_info = &account_infos[account_info_index];
            **account_info.try_borrow_mut_lamports().unwrap() = borrowed_account.get_lamports();
            if account_info.owner != borrowed_account.get_owner() {
                // TODO Figure out a better way to allow the System Program to set the account owner
                #[allow(clippy::transmute_ptr_to_ptr)]
                #[allow(mutable_transmutes)]
                let account_info_mut =
                    unsafe { transmute::<&Pubkey, &mut Pubkey>(account_info.owner) };
                *account_info_mut = *borrowed_account.get_owner();
            }

            let new_data = borrowed_account.get_data();
            let new_len = new_data.len();

            // Resize account_info data
            if account_info.data_len() != new_len {
                account_info.realloc(new_len, false)?;
            }

            // Clone the data
            let mut data = account_info.try_borrow_mut_data()?;
            data.clone_from_slice(new_data);
        }

        stable_log::program_success(&log_collector, &instruction.program_id);
        Ok(())
    }

    fn sol_get_clock_sysvar(&self, var_addr: *mut u8) -> u64 {
        get_sysvar(
            get_invoke_context().get_sysvar_cache().get_clock(),
            var_addr,
        )
    }

    fn sol_get_epoch_schedule_sysvar(&self, var_addr: *mut u8) -> u64 {
        get_sysvar(
            get_invoke_context().get_sysvar_cache().get_epoch_schedule(),
            var_addr,
        )
    }

    #[allow(deprecated)]
    fn sol_get_fees_sysvar(&self, var_addr: *mut u8) -> u64 {
        get_sysvar(get_invoke_context().get_sysvar_cache().get_fees(), var_addr)
    }

    fn sol_get_rent_sysvar(&self, var_addr: *mut u8) -> u64 {
        get_sysvar(get_invoke_context().get_sysvar_cache().get_rent(), var_addr)
    }

    fn sol_get_return_data(&self) -> Option<(Pubkey, Vec<u8>)> {
        let (program_id, data) = get_invoke_context().transaction_context.get_return_data();
        Some((*program_id, data.to_vec()))
    }

    fn sol_set_return_data(&self, data: &[u8]) {
        let invoke_context = get_invoke_context();
        let transaction_context = &mut invoke_context.transaction_context;
        let instruction_context = transaction_context
            .get_current_instruction_context()
            .unwrap();
        let caller = *instruction_context
            .get_last_program_key(transaction_context)
            .unwrap();
        transaction_context
            .set_return_data(caller, data.to_vec())
            .unwrap();
    }

    fn sol_get_stack_height(&self) -> u64 {
        let invoke_context = get_invoke_context();
        invoke_context.get_stack_height().try_into().unwrap()
    }
}
