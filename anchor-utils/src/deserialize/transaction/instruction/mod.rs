pub mod account_metas;
pub mod builtins;
pub mod data;
pub mod spl_programs;

use crate::deserialize::AnchorDeserializer;
pub use account_metas::*;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use solana_devtools_serde::pubkey;
use solana_program::instruction::Instruction;
use solana_program::pubkey::Pubkey;

impl AnchorDeserializer {
    /// Attempts deserialization of a given instruction and its inner instructions.
    /// The [VersionedMessage] passed in is from the same transaction.
    /// If the attempt fails, we return a JSON object indicating the
    /// reason for failure, and any other information.
    pub fn try_deserialize_instruction(
        &self,
        ix_num: usize,
        ix: &mut Instruction,
        inner_instructions: Option<Vec<Instruction>>,
    ) -> Result<DeserializedInstruction> {
        // Try to deserialize the inner instructions up front.
        let inner_ix = {
            let mut deserialized_inner_ix = vec![];
            if let Some(mut instructions) = inner_instructions {
                for (inner_ix_num, inner_ix) in instructions.iter_mut().enumerate() {
                    deserialized_inner_ix.push(self.try_deserialize_instruction(
                        inner_ix_num,
                        inner_ix,
                        None,
                    )?);
                }
            }
            deserialized_inner_ix
        };
        if let Some(ix) = DeserializedInstruction::try_compute_budget_instruction(ix, ix_num as u8)
        {
            return Ok(ix);
        }
        if let Some(ix) = DeserializedInstruction::try_system_instruction(ix, ix_num as u8) {
            return Ok(ix);
        }
        if let Some(ix) = DeserializedInstruction::try_token_program_instruction(ix, ix_num as u8) {
            return Ok(ix);
        }
        if let Some(ix) =
            DeserializedInstruction::try_associated_token_instruction(ix, ix_num as u8)
        {
            return Ok(ix);
        }
        // Get program ID, find IDL
        let idl = self.idl_cache.get(&ix.program_id);
        // Try fetching the IDL and deserializing.
        let mut deserialized = if let Some(idl) = idl {
            // If there's an IDL, we can try deserializing
            let maybe_deserialized = idl.try_deserialize_instruction_data(ix.data.as_slice());
            match maybe_deserialized {
                Ok((idl_ix, ix_data)) => {
                    // If we succeeded in deserializing the instruction data,
                    // then we can also name each account passed in to the instruction.
                    let accounts = {
                        let mut metas: Vec<DeserializedAccountMetas> = vec![];
                        let mut increment: usize = 0;
                        let account_meta_groups = AccountMetaChecker::new(&ix.accounts);
                        account_meta_groups.idl_accounts_to_json(
                            &mut increment,
                            idl_ix.accounts.clone(),
                            &mut metas,
                        );
                        metas
                    };
                    DeserializedInstruction::ok(
                        ix.program_id,
                        idl.name.to_string(),
                        ix_num as u8,
                        idl_ix.name,
                        ix_data,
                        accounts,
                    )
                }
                Err(e) => {
                    // If the IDL contains no matching discriminator,
                    // then it's not up to date or invalid.
                    DeserializedInstruction::err(
                        ix.program_id,
                        Some(idl.name.to_string()),
                        ix_num as u8,
                        format!("{}", e),
                    )
                }
            }
        } else {
            // If there's no IDL, we cannot deserialize
            DeserializedInstruction::err(
                ix.program_id,
                None,
                ix_num as u8,
                "unknown program".to_string(),
            )
        };
        // Optionally append any inner instructions
        if !inner_ix.is_empty() {
            deserialized.inner_instructions = Some(inner_ix);
        }
        Ok(deserialized)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeserializedInstruction {
    #[serde(with = "pubkey")]
    pub program_id: Pubkey,
    pub program_name: String,
    pub index: u8,
    pub parsed: DeserializedInstructionData,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inner_instructions: Option<Vec<DeserializedInstruction>>,
}

impl DeserializedInstruction {
    pub fn ok(
        program_id: Pubkey,
        program_name: String,
        index: u8,
        name: String,
        data: Value,
        accounts: Vec<DeserializedAccountMetas>,
    ) -> Self {
        Self {
            program_id,
            program_name,
            index,
            parsed: DeserializedInstructionData::Ok {
                name,
                data,
                accounts,
            },
            inner_instructions: None,
        }
    }

    pub fn err(
        program_id: Pubkey,
        program_name: Option<String>,
        index: u8,
        error_message: String,
    ) -> Self {
        Self {
            program_id,
            program_name: program_name.unwrap_or("Unknown, IDL not found".to_string()),
            index,
            parsed: DeserializedInstructionData::Err {
                deserialize_error: error_message,
            },
            inner_instructions: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", untagged)]
pub enum DeserializedInstructionData {
    Ok {
        name: String,
        data: Value,
        accounts: Vec<DeserializedAccountMetas>,
    },
    Err {
        deserialize_error: String,
    },
}
