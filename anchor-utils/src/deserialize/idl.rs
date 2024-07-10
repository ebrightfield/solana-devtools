use crate::deserialize::discriminator;
use crate::deserialize::discriminator::Discriminator;
use crate::idl_sdk::account::deserialize_idl_account;
use anchor_syn::idl::types::{
    Idl, IdlField, IdlInstruction, IdlTypeDefinition, IdlTypeDefinitionTy,
};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use solana_sdk::account::Account;
use std::collections::BTreeMap;
use std::fs;
use std::ops::Deref;
use std::path::Path;

const ENUM_VARIANT_FIELD_NAME: &'static str = "__enum_variant";

/// IDL Definitions indexed by discriminator
///
/// Discriminators are calculated taking one of the following strings:
/// - Accounts -- `"account:<AccountStructName>"`
/// - Instructions -- `"global:<IxName>"` or `"state:<IxName>"`
///
/// hashing it, and keeping only the first 8 bytes.
#[derive(Debug, Clone)]
pub struct IdlDefinitions {
    pub instructions: BTreeMap<Discriminator, IdlInstruction>,
    pub accounts: BTreeMap<Discriminator, IdlTypeDefinition>,
    pub types: BTreeMap<Discriminator, IdlTypeDefinition>,
    // TODO events
}

impl From<&Idl> for IdlDefinitions {
    fn from(idl: &Idl) -> Self {
        Self {
            instructions: idl
                .instructions
                .iter()
                .map(|ix| {
                    vec![
                        (discriminator::ix_state_discriminator(&ix.name), ix.clone()),
                        (discriminator::ix_discriminator(&ix.name), ix.clone()),
                    ]
                })
                .flatten()
                .collect(),
            types: idl
                .types
                .iter()
                .map(|ty_def| {
                    (
                        discriminator::account_discriminator(&ty_def.name),
                        ty_def.clone(),
                    )
                })
                .collect(),
            accounts: idl
                .accounts
                .iter()
                .map(|act| (discriminator::account_discriminator(&act.name), act.clone()))
                .collect(),
        }
    }
}

/// A marker enum to help with tracking the origin of an [IdlTypeDefinition]
/// being used in a deserialization attempt when the [IdlTypeDefinition] is obtained by name.
/// Primarily for debugging purposes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum IdlSection {
    Instructions,
    Accounts,
    Types,
    // TODO Events
}

/// A wrapped [anchor_syn::idl::Idl], with an accompanying
/// collection of lookup tables mapping every account and instruction
/// discriminator to its associated `anchor_syn` IDL type.
/// Accounts are parsed from [anchor_syn::idl::IdlTypeDefinition].
/// Instructions are parsed from an [anchor_syn::idl::IdlInstruction].
#[derive(Debug, Clone)]
pub struct IdlWithDiscriminators {
    idl: Idl,
    pub instruction_definitions: BTreeMap<Discriminator, IdlInstruction>,
    pub account_definitions: BTreeMap<Discriminator, IdlTypeDefinition>,
    pub type_definitions: BTreeMap<Discriminator, IdlTypeDefinition>,
    pub event_definitions: BTreeMap<Discriminator, IdlTypeDefinition>,
    pub enum_variant_field_name: String,
}

impl IdlWithDiscriminators {
    pub fn new(idl: Idl) -> Self {
        Self::from(idl)
    }

    pub fn from_file(p: impl AsRef<Path>) -> anyhow::Result<Self> {
        let idl = fs::read_to_string(&p)?;
        let idl: Idl = serde_json::from_str(&idl)
            .map_err(|_| anyhow!("Could not deserialize decompressed IDL data"))?;
        Ok(idl.into())
    }

    /// Find any type definition, whether under accounts, types, or events.
    /// Also returns an enum marking the section in which it was found.
    pub fn find_type_definition_by_name(
        &self,
        name: &str,
    ) -> Option<(IdlSection, &[u8; 8], &IdlTypeDefinition)> {
        if let Some((discriminator, ty_def)) = self.get_type_definition_by_name(name) {
            return Some((IdlSection::Types, discriminator, ty_def));
        }
        if let Some((discriminator, ty_def)) = self.get_account_definition_by_name(name) {
            return Some((IdlSection::Accounts, discriminator, ty_def));
        }
        // TODO Events
        None
    }

    pub fn get_type_definition(&self, discriminator: &Discriminator) -> Option<&IdlTypeDefinition> {
        self.type_definitions.get(discriminator)
    }

    pub fn get_type_definition_by_name(
        &self,
        name: &str,
    ) -> Option<(&[u8; 8], &IdlTypeDefinition)> {
        self.type_definitions
            .iter()
            .find(|entry| entry.1.name == name)
    }

    pub fn get_account_definition(
        &self,
        discriminator: &Discriminator,
    ) -> Option<&IdlTypeDefinition> {
        self.account_definitions.get(discriminator)
    }

    pub fn get_account_definition_by_name(
        &self,
        name: &str,
    ) -> Option<(&[u8; 8], &IdlTypeDefinition)> {
        self.account_definitions
            .iter()
            .find(|entry| entry.1.name == name)
    }

    pub fn get_event_definition_by_name(
        &self,
        name: &str,
    ) -> Option<(&[u8; 8], &IdlTypeDefinition)> {
        self.event_definitions
            .iter()
            .find(|entry| entry.1.name == name)
    }
    // TODO Events
}

impl Deref for IdlWithDiscriminators {
    type Target = Idl;

    fn deref(&self) -> &Self::Target {
        &self.idl
    }
}

impl From<Idl> for IdlWithDiscriminators {
    fn from(idl: Idl) -> Self {
        Self {
            instruction_definitions: idl
                .instructions
                .iter()
                .map(|ix| {
                    vec![
                        (discriminator::ix_state_discriminator(&ix.name), ix.clone()),
                        (discriminator::ix_discriminator(&ix.name), ix.clone()),
                    ]
                })
                .flatten()
                .collect(),
            type_definitions: idl
                .types
                .iter()
                .map(|ty_def| {
                    (
                        discriminator::account_discriminator(&ty_def.name),
                        ty_def.clone(),
                    )
                })
                .collect(),
            account_definitions: idl
                .accounts
                .iter()
                .map(|act| (discriminator::account_discriminator(&act.name), act.clone()))
                .collect(),
            event_definitions: idl
                .events
                .as_ref()
                .unwrap_or(&vec![])
                .iter()
                .map(|event| {
                    (
                        discriminator::account_discriminator(&event.name),
                        IdlTypeDefinition {
                            name: event.name.clone(),
                            docs: None,
                            generics: None,
                            ty: IdlTypeDefinitionTy::Struct {
                                fields: event
                                    .fields
                                    .iter()
                                    .map(|field| IdlField {
                                        name: field.name.clone(),
                                        docs: None,
                                        ty: field.ty.clone(),
                                    })
                                    .collect(),
                            },
                        },
                    )
                })
                .collect(),
            idl,
            enum_variant_field_name: ENUM_VARIANT_FIELD_NAME.to_string(),
        }
    }
}

impl TryFrom<Account> for IdlWithDiscriminators {
    type Error = anyhow::Error;

    fn try_from(account: Account) -> Result<Self, Self::Error> {
        let idl = deserialize_idl_account(&account.data)
            .map_err(|e| anyhow!("failed to deserialize IDL: {e}"))?;
        Ok(Self::from(idl))
    }
}
