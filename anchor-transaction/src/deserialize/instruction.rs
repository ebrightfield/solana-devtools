// TODO Top level function for deserializing the entire instruction,
// return metadata like name, accounts, return value

use crate::deserialize::field::deserialize_idl_fields;
use crate::fetch_idl::discriminators::IdlWithDiscriminators;
use anchor_syn::idl::{IdlAccountItem, IdlAccounts, IdlInstruction};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use solana_program::message::{MessageHeader, VersionedMessage};
use solana_program::pubkey::Pubkey;

/// Deserializes just the data portion of an instruction.
/// We peel off the discriminator and use [crate::deserialize::field::deserialize_idl_fields].
pub fn deser_ix_data_from_idl(
    idl: &IdlWithDiscriminators,
    ix_data: Vec<u8>,
) -> anyhow::Result<(IdlInstruction, Value)> {
    let mut first_eight = ix_data.to_vec();
    first_eight.resize(8, 0);
    let first_eight: [u8; 8] = first_eight.try_into().unwrap();
    let ix = idl
        .discriminators
        .instructions
        .get(&first_eight)
        .ok_or(anyhow!(
            "Could not match instruction against any discriminator"
        ))
        .map_err(|e| {
            println!("{:?}", &e);
            e
        })?;
    Ok((
        ix.clone(),
        deserialize_idl_fields(&ix.args, &idl, &mut &ix_data[8..])?,
    ))
}

/// For iterating over both a transaction message and an IDL account item,
/// building a list of JSON values, and potentially recursively stepping into
/// a nested account object in the IDL.
///
/// This also verifies the account signer and mutability privilege escalations,
/// making sure the instruction's account metas match what is stipulated in the IDL.
pub struct AccountMetaGroups {
    all_accounts: Vec<Pubkey>,
    signers: Vec<Pubkey>,
    signer_mut: Vec<Pubkey>,
    nonsigner_readonly: Vec<Pubkey>,
    instruction_account_indices: Vec<u8>,
}

impl AccountMetaGroups {
    pub fn new_from_message(msg: VersionedMessage, instruction_account_indices: Vec<u8>) -> Self {
        let MessageHeader {
            num_required_signatures,
            num_readonly_signed_accounts,
            num_readonly_unsigned_accounts,
        } = msg.header();
        let all_accounts = msg.static_account_keys().to_vec();
        let signers = all_accounts[0..(*num_required_signatures) as usize].to_vec();
        let signer_mut = signers
            [0..(*num_required_signatures - *num_readonly_signed_accounts) as usize]
            .to_vec();
        let acts_len = all_accounts.len();
        let nonsigner_readonly =
            all_accounts[(acts_len - (*num_readonly_unsigned_accounts as usize))..].to_vec();
        Self {
            all_accounts,
            signers,
            signer_mut,
            nonsigner_readonly,
            instruction_account_indices,
        }
    }

    /// Breaks down the [IdlAccountItem], with possible recursion due to
    /// nested account structs.
    pub fn idl_accounts_to_json(
        &self,
        instruction_account_index: &mut usize,
        items: Vec<IdlAccountItem>,
        json_values: &mut Vec<Value>,
    ) {
        for item in items {
            match item {
                IdlAccountItem::IdlAccount(act) => {
                    let idx = self.instruction_account_indices[*instruction_account_index];
                    let pubkey = self.all_accounts[idx as usize].clone();
                    let json = json!({
                        "name": act.name,
                        "pubkey": pubkey.to_string(),
                        "is_signer": self.check_pubkey_signer(&pubkey, act.is_signer),
                        "is_mut": self.check_pubkey_is_mut(&pubkey, act.is_mut)
                    });
                    json_values.push(json);
                    *instruction_account_index += 1;
                }
                IdlAccountItem::IdlAccounts(IdlAccounts { name, accounts }) => {
                    let mut nested_values = vec![];
                    for account_item in accounts {
                        self.idl_accounts_to_json(
                            instruction_account_index,
                            vec![account_item],
                            &mut nested_values,
                        );
                    }
                    json_values.push(json!({ name: Value::Array(nested_values) }));
                }
            }
        }
    }

    /// Check that an account was signed appropriately according to what is
    /// stipulated in the IDL.
    fn check_pubkey_signer(&self, pubkey: &Pubkey, is_signer: bool) -> AccountMetaStatus {
        match (is_signer, self.signers.contains(pubkey)) {
            (true, true) => AccountMetaStatus::True,
            (true, false) => AccountMetaStatus::FailedToEscalatePrivilege,
            (false, true) => AccountMetaStatus::UnnecessaryPrivilegeEscalation,
            (false, false) => AccountMetaStatus::False,
        }
    }

    /// Check that an account was marked mutable appropriately according to what is
    /// stipulated in the IDL.
    fn check_pubkey_is_mut(&self, pubkey: &Pubkey, is_mut: bool) -> AccountMetaStatus {
        match (
            is_mut,
            (self.signer_mut.contains(pubkey) || !self.nonsigner_readonly.contains(pubkey)),
        ) {
            (true, true) => AccountMetaStatus::True,
            (true, false) => AccountMetaStatus::FailedToEscalatePrivilege,
            (false, true) => AccountMetaStatus::UnnecessaryPrivilegeEscalation,
            (false, false) => AccountMetaStatus::False,
        }
    }
}

/// Reports privilege escalations as "true" or "false" in the correct case,
/// and an error variant in the mismatched cases.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountMetaStatus {
    /// The privilege escalation was required, and fulfilled.
    True,
    /// The privilege escalation was not required, and not performed.
    False,
    /// The privilege escalation was required, and not fulfilled. e.g.
    /// a required signer did not sign the transaction, or an account was
    /// not marked as mutable but should have been.
    FailedToEscalatePrivilege,
    /// The privilege escalation was not required, and unnecessarily performed.
    /// e.g. a signer that did not need to sign, or an account marked
    /// mutable despite the instruction only reading it.
    UnnecessaryPrivilegeEscalation,
}

// /// Attempts deserialization of a given transaction instruction.
// /// The [VersionedMessage] passed in must be from the same transaction.
// /// If the attempt fails, we return a JSON object indicating the
// /// reason for failure, and any other information.
// pub fn deserialize_instruction(
//     idl: &IdlWithDiscriminators,
//     instruction_num: usize,
//     ix: &CompiledInstruction,
//     message: &VersionedMessage,
//     inner_instructions: Option<&Vec<CompiledInstruction>>,
// ) -> Value {
//     let idx = ix.program_id_index;
//     let program_id = message.static_account_keys()[idx as usize];
//     let maybe_deserialized = deser_ix_data_from_idl(&idl, ix.data.clone());
//     if let Ok((idl_ix, ix_data)) = maybe_deserialized {
//         let account_metas = {
//             let mut metas: Vec<Value> = vec![];
//             let mut increment: usize = 0;
//             let account_meta_groups =
//                 AccountMetaGroups::new_from_message(message.clone(), ix.accounts.clone());
//             account_meta_groups.idl_accounts_to_json(
//                 &mut increment,
//                 idl_ix.accounts.clone(),
//                 &mut metas,
//             );
//             metas
//         };
//         if let Some(instructions) = inner_instructions {
//             for (i, ix) in instructions.iter().enumerate() {
//
//             }
//         }
//         json!({
//            "program_id": program_id.to_string(),
//            "program_name": idl.name,
//            "instruction": {
//                "name": idl_ix.name,
//                "data": ix_data,
//                "accounts": account_metas
//             }
//         })
//     } else {
//         // TODO Maybe add account metas and raw ix data?
//         json!({
//            "program_id": program_id.to_string(),
//            "unknown_discriminator": format!("instruction {}", instruction_num)
//         })
//     }
// }
