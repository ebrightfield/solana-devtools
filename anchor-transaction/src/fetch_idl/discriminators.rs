use anchor_syn::hash::hash;
use anchor_syn::idl::{Idl, IdlInstruction, IdlTypeDefinition};
use heck::SnakeCase;
use std::collections::BTreeMap;
use std::ops::Deref;

/// Provides a means of looking up by discriminator to retrieve
/// the IDL definitions for their associated account or instruction schema.
///
/// Discriminators are calculated taking one of the following strings:
/// - Accounts -- `"account:<AccountStructName>"`
/// - Instructions -- `"global:<IxName>"` or `"state:<IxName>"`
///
/// hashing it, and keeping only the first 8 bytes.
#[derive(Debug, Clone)]
pub struct Discriminators {
    pub instructions: BTreeMap<[u8; 8], IdlInstruction>,
    pub accounts: BTreeMap<[u8; 8], IdlTypeDefinition>,
}

impl Discriminators {
    /// Calculates account and instruction discriminators, indexes
    /// hashmaps according to a key-value structure of:
    /// `(discriminator, idl_schema)`.
    pub fn from_idl(idl: Idl) -> Self {
        Self {
            instructions: idl
                .instructions
                .iter()
                .map(|ix| {
                    vec![
                        (ix_state_discriminator(&ix.name), ix.clone()),
                        (ix_discriminator(&ix.name), ix.clone()),
                    ]
                })
                .flatten()
                .collect(),
            accounts: idl
                .accounts
                .into_iter()
                .map(|act| (account_discriminator(&act.name), act))
                .collect(),
        }
    }
}

/// Calculates the discriminator for an account based on its name,
/// which would be found in an IDL.
fn account_discriminator(name: &str) -> [u8; 8] {
    hash(format!("account:{}", name).as_bytes()).to_bytes()[0..8]
        .try_into()
        .unwrap()
}

/// Calculates the discriminator for an instruction based on its name,
/// which would be found in an IDL.
fn ix_discriminator(name: &str) -> [u8; 8] {
    hash(format!("global:{}", name.to_snake_case()).as_bytes()).to_bytes()[0..8]
        .try_into()
        .unwrap()
}

/// Calculates the discriminator for a state-modifying instruction based on its name,
/// which would be found in an IDL.
fn ix_state_discriminator(name: &str) -> [u8; 8] {
    hash(format!("state:{}", name).as_bytes()).to_bytes()[0..8]
        .try_into()
        .unwrap()
}

/// A wrapped [anchor_syn::idl::Idl], with an accompanying
/// collection of lookup tables mapping every account and instruction
/// discriminator to its associated `anchor_syn` IDL type.
/// Accounts are parsed from [anchor_syn::idl::IdlTypeDefinition].
/// Instructions are parsed from an [anchor_syn::idl::IdlInstruction].
#[derive(Debug, Clone)]
pub struct IdlWithDiscriminators {
    idl: Idl,
    pub discriminators: Discriminators,
}

impl IdlWithDiscriminators {
    pub fn new(idl: Idl) -> Self {
        let discriminators = Discriminators::from_idl(idl.clone());
        Self {
            idl,
            discriminators,
        }
    }
}

impl Deref for IdlWithDiscriminators {
    type Target = Idl;

    fn deref(&self) -> &Self::Target {
        &self.idl
    }
}
