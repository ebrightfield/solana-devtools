use anchor_syn::idl::{IdlAccountItem, IdlAccounts};
use serde::{Deserialize, Serialize};
use solana_devtools_serde::pubkey;
use solana_program::instruction::AccountMeta;
use solana_program::pubkey::Pubkey;

/// For iterating over both a transaction message and an IDL account item,
/// building a list of JSON values, and potentially recursively stepping into
/// a nested account object in the IDL.
///
/// This also verifies the account signer and mutability privilege escalations,
/// making sure the instruction's account metas match what is stipulated in the IDL.
pub struct AccountMetaChecker<'a>(&'a [AccountMeta]);

impl<'a> AccountMetaChecker<'a> {
    pub fn new(account_metas: &'a [AccountMeta]) -> Self {
        Self(account_metas)
    }

    /// Breaks down the [IdlAccountItem], with possible recursion due to
    /// nested account structs.
    pub fn idl_accounts_to_json(
        &self,
        instruction_account_index: &mut usize,
        items: Vec<IdlAccountItem>,
        json_values: &mut Vec<DeserializedAccountMetas>,
    ) {
        for item in items {
            match item {
                IdlAccountItem::IdlAccount(act) => {
                    let act_meta = &self.0[*instruction_account_index];
                    let pubkey = act_meta.pubkey;
                    let account_meta = DeserializedAccountMetas::One(DeserializedAccountMeta {
                        name: act.name,
                        pubkey,
                        is_signer: self.check_pubkey_signer(act.is_signer, act_meta.is_signer),
                        is_mut: self.check_pubkey_is_mut(act.is_mut, act.is_mut),
                    });
                    json_values.push(account_meta);
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
                    json_values.push(DeserializedAccountMetas::Nested {
                        name,
                        accounts: nested_values,
                    });
                }
            }
        }
    }

    /// Check that an account was signed appropriately according to what is
    /// stipulated in the IDL.
    fn check_pubkey_signer(&self, is_signer: bool, act_meta_is_signer: bool) -> AccountMetaStatus {
        match (is_signer, act_meta_is_signer) {
            (true, true) => AccountMetaStatus::True,
            (true, false) => AccountMetaStatus::FailedToEscalatePrivilege,
            (false, true) => AccountMetaStatus::UnnecessaryPrivilegeEscalation,
            (false, false) => AccountMetaStatus::False,
        }
    }

    /// Check that an account was marked mutable appropriately according to what is
    /// stipulated in the IDL.
    fn check_pubkey_is_mut(&self, is_mut: bool, act_meta_is_mut: bool) -> AccountMetaStatus {
        match (is_mut, act_meta_is_mut) {
            (true, true) => AccountMetaStatus::True,
            (true, false) => AccountMetaStatus::FailedToEscalatePrivilege,
            (false, true) => AccountMetaStatus::UnnecessaryPrivilegeEscalation,
            (false, false) => AccountMetaStatus::False,
        }
    }
}

/// Reports privilege escalations as "true" or "false" in the correct case,
/// and an error variant in the mismatched cases.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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
    /// This is not necessarily an error, because another instruction in the same
    /// transaction might have required the privilege escalation.
    UnnecessaryPrivilegeEscalation,
}

impl Into<bool> for AccountMetaStatus {
    fn into(self) -> bool {
        match self {
            AccountMetaStatus::True => true,
            AccountMetaStatus::False => false,
            AccountMetaStatus::FailedToEscalatePrivilege => false,
            AccountMetaStatus::UnnecessaryPrivilegeEscalation => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeserializedAccountMeta {
    name: String,
    #[serde(with = "pubkey")]
    pubkey: Pubkey,
    is_signer: AccountMetaStatus,
    is_mut: AccountMetaStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DeserializedAccountMetas {
    One(DeserializedAccountMeta),
    Nested {
        name: String,
        accounts: Vec<DeserializedAccountMetas>,
    },
}
