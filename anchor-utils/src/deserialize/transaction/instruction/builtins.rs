use anyhow::Result;
use solana_sdk::{
    borsh0_10::try_from_slice_unchecked,
    compute_budget::{self, ComputeBudgetInstruction},
    instruction::Instruction,
    system_instruction::SystemInstruction,
    system_program,
};

use super::DeserializedInstruction;

pub fn compute_budget_instruction(data: &[u8]) -> Result<ComputeBudgetInstruction> {
    Ok(try_from_slice_unchecked(data)?)
}

pub fn compute_budget_instruction_name(ix: &ComputeBudgetInstruction) -> &'static str {
    match ix {
        ComputeBudgetInstruction::RequestUnitsDeprecated { .. } => "request_units_deprecated",
        ComputeBudgetInstruction::RequestHeapFrame(_) => "request_heap_frame",
        ComputeBudgetInstruction::SetComputeUnitLimit(_) => "set_compute_unit_limit",
        ComputeBudgetInstruction::SetComputeUnitPrice(_) => "set_compute_unit_price",
        ComputeBudgetInstruction::SetLoadedAccountsDataSizeLimit(_) => {
            "set_loaded_accounts_data_size_limit"
        }
    }
}

pub fn system_instruction(data: &[u8]) -> Result<SystemInstruction> {
    Ok(bincode1::deserialize(data)?)
}

pub fn system_instruction_name(ix: &SystemInstruction) -> &'static str {
    match ix {
        SystemInstruction::CreateAccount { .. } => "create_account",
        SystemInstruction::Assign { .. } => "assign",
        SystemInstruction::Transfer { .. } => "transfer",
        SystemInstruction::CreateAccountWithSeed { .. } => "create_account_with_seed",
        SystemInstruction::AdvanceNonceAccount => "advance_nonce_account",
        SystemInstruction::WithdrawNonceAccount(_) => "withdraw_nonce_account",
        SystemInstruction::InitializeNonceAccount(_) => "initialize_nonce_account",
        SystemInstruction::AuthorizeNonceAccount(_) => "authorize_nonce_account",
        SystemInstruction::Allocate { .. } => "allocate",
        SystemInstruction::AllocateWithSeed { .. } => "allocate_with_seed",
        SystemInstruction::AssignWithSeed { .. } => "assign_with_seed",
        SystemInstruction::TransferWithSeed { .. } => "transfer_with_seed",
        SystemInstruction::UpgradeNonceAccount => "upgrade_nonce_account",
    }
}

impl DeserializedInstruction {
    pub fn try_compute_budget_instruction(ix: &Instruction, ix_num: u8) -> Option<Self> {
        if ix.program_id == compute_budget::ID {
            if let Ok(ix) = compute_budget_instruction(&ix.data) {
                let ix_data = serde_json::to_value(&ix).ok()?;
                return Some(DeserializedInstruction::ok(
                    compute_budget::ID,
                    "compute_budget_program".to_string(),
                    ix_num as u8,
                    compute_budget_instruction_name(&ix).to_string(),
                    ix_data,
                    vec![],
                ));
            }
        }
        None
    }

    pub fn try_system_instruction(ix: &Instruction, ix_num: u8) -> Option<Self> {
        if ix.program_id == system_program::ID {
            if let Ok(ix) = system_instruction(&ix.data) {
                let ix_data = serde_json::to_value(&ix).ok()?;
                return Some(DeserializedInstruction::ok(
                    system_program::ID,
                    "system_program".to_string(),
                    ix_num as u8,
                    system_instruction_name(&ix).to_string(),
                    ix_data,
                    vec![],
                ));
            }
        }
        None
    }
}
