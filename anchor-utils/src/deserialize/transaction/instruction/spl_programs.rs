use anchor_lang::AnchorDeserialize;
use anyhow::Result;
use serde_json::{json, Value};
use solana_sdk::instruction::Instruction;
use spl_associated_token_account::instruction::AssociatedTokenAccountInstruction;
use spl_token::{self, instruction::TokenInstruction};

use super::DeserializedInstruction;

pub fn token_program_instruction(ix_data: &[u8]) -> Result<TokenInstruction> {
    Ok(TokenInstruction::unpack(ix_data)?)
}

pub fn token_program_instruction_name<'a>(ix: &'a TokenInstruction<'a>) -> &'static str {
    match ix {
        TokenInstruction::InitializeMint { .. } => "initialize_mint",
        TokenInstruction::InitializeAccount => "initialize_account",
        TokenInstruction::InitializeMultisig { .. } => "initialize_multisig",
        TokenInstruction::Transfer { .. } => "transfer",
        TokenInstruction::Approve { .. } => "approve",
        TokenInstruction::Revoke => "revoke",
        TokenInstruction::SetAuthority { .. } => "set_authority",
        TokenInstruction::MintTo { .. } => "mint_to",
        TokenInstruction::Burn { .. } => "burn",
        TokenInstruction::CloseAccount => "close_account",
        TokenInstruction::FreezeAccount => "freeze_account",
        TokenInstruction::ThawAccount => "thaw_account",
        TokenInstruction::TransferChecked { .. } => "transfer_checked",
        TokenInstruction::ApproveChecked { .. } => "approve_checked",
        TokenInstruction::MintToChecked { .. } => "mint_to_checked",
        TokenInstruction::BurnChecked { .. } => "burn_checked",
        TokenInstruction::InitializeAccount2 { .. } => "initialize_account2",
        TokenInstruction::SyncNative => "sync_native",
        TokenInstruction::InitializeAccount3 { .. } => "initialize_account3",
        TokenInstruction::InitializeMultisig2 { .. } => "initialize_multisig2",
        TokenInstruction::InitializeMint2 { .. } => "initialize_mint2",
        TokenInstruction::GetAccountDataSize => "get_account_data_size",
        TokenInstruction::InitializeImmutableOwner => "initialize_immutable_owner",
        TokenInstruction::AmountToUiAmount { .. } => "amount_to_ui_amount",
        TokenInstruction::UiAmountToAmount { .. } => "ui_amount_to_amount",
    }
}

pub fn token_program_ix_to_value<'a>(ix: &'a TokenInstruction) -> Value {
    match ix {
        TokenInstruction::InitializeMint {
            decimals,
            mint_authority,
            freeze_authority,
        } => {
            json!({
                "decimals": *decimals,
                "mint_authority": mint_authority.to_string(),
                "freeze_authority": freeze_authority.map_or(None, |p| Some(p.to_string())),
            })
        }
        TokenInstruction::InitializeAccount => Value::Null,
        TokenInstruction::InitializeMultisig { m } => {
            json!({ "m": m })
        }
        TokenInstruction::Transfer { amount } => {
            json!({ "amount": amount})
        }
        TokenInstruction::Approve { amount } => {
            json!({ "amount": amount})
        }
        TokenInstruction::Revoke => Value::Null,
        TokenInstruction::SetAuthority {
            authority_type,
            new_authority,
        } => {
            json!({
                "authority_type": format!("{:?}", authority_type),
                "new_authority": new_authority.map_or(None, |p| Some(p.to_string())),
            })
        }
        TokenInstruction::MintTo { amount } => {
            json!({ "amount": amount })
        }
        TokenInstruction::Burn { amount } => {
            json!({ "amount": amount })
        }
        TokenInstruction::CloseAccount => Value::Null,
        TokenInstruction::FreezeAccount => Value::Null,
        TokenInstruction::ThawAccount => Value::Null,
        TokenInstruction::TransferChecked { amount, decimals } => {
            json!({
                "amount": amount,
                "decimals": decimals,
            })
        }
        TokenInstruction::ApproveChecked { amount, decimals } => {
            json!({
                "amount": amount,
                "decimals": decimals,
            })
        }
        TokenInstruction::MintToChecked { amount, decimals } => {
            json!({
                "amount": amount,
                "decimals": decimals,
            })
        }
        TokenInstruction::BurnChecked { amount, decimals } => {
            json!({
                "amount": amount,
                "decimals": decimals,
            })
        }
        TokenInstruction::InitializeAccount2 { owner } => {
            json!({ "owner": owner.to_string() })
        }
        TokenInstruction::SyncNative => Value::Null,
        TokenInstruction::InitializeAccount3 { owner } => {
            json!({ "owner": owner.to_string() })
        }
        TokenInstruction::InitializeMultisig2 { m } => {
            json!({"m": m })
        }
        TokenInstruction::InitializeMint2 {
            decimals,
            mint_authority,
            freeze_authority,
        } => {
            json!({
                "decimals": *decimals,
                "mint_authority": mint_authority.to_string(),
                "freeze_authority": freeze_authority.map_or(None, |p| Some(p.to_string())),
            })
        }
        TokenInstruction::GetAccountDataSize => Value::Null,
        TokenInstruction::InitializeImmutableOwner => Value::Null,
        TokenInstruction::AmountToUiAmount { amount } => {
            json!({ "amount": amount })
        }
        TokenInstruction::UiAmountToAmount { ui_amount } => {
            json!({ "ui_amount": ui_amount })
        }
    }
}

pub fn associated_token_instruction(ix_data: &[u8]) -> Result<AssociatedTokenAccountInstruction> {
    Ok(AssociatedTokenAccountInstruction::deserialize(
        &mut &ix_data[..],
    )?)
}

pub fn associated_token_instruction_name(ix: &AssociatedTokenAccountInstruction) -> &'static str {
    match ix {
        AssociatedTokenAccountInstruction::Create => "create",
        AssociatedTokenAccountInstruction::CreateIdempotent => "create_idempotent",
        AssociatedTokenAccountInstruction::RecoverNested => "recover_nested",
    }
}

impl DeserializedInstruction {
    pub fn try_token_program_instruction(ix: &Instruction, ix_num: u8) -> Option<Self> {
        if ix.program_id == spl_token::ID {
            if let Ok(ix) = token_program_instruction(&ix.data) {
                let ix_data = token_program_ix_to_value(&ix);
                return Some(DeserializedInstruction::ok(
                    spl_token::ID,
                    "spl_token_program".to_string(),
                    ix_num as u8,
                    token_program_instruction_name(&ix).to_string(),
                    ix_data,
                    vec![],
                ));
            }
        }
        None
    }

    pub fn try_associated_token_instruction(ix: &Instruction, ix_num: u8) -> Option<Self> {
        if ix.program_id == spl_associated_token_account::ID {
            if let Ok(ix) = associated_token_instruction(&ix.data) {
                return Some(DeserializedInstruction::ok(
                    spl_associated_token_account::ID,
                    "spl_associated_token_program".to_string(),
                    ix_num as u8,
                    associated_token_instruction_name(&ix).to_string(),
                    Value::Null,
                    vec![],
                ));
            }
        }
        None
    }
}
