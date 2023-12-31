use crate::decompile_instructions::extract_instructions_from_versioned_message;
#[cfg(feature = "async_client")]
use solana_client::nonblocking::rpc_client::RpcClient;
#[cfg(feature = "client")]
use solana_client::rpc_client;
#[cfg(any(feature = "client", feature = "async_client"))]
use solana_client::{client_error::ClientError, rpc_config::RpcTransactionConfig};
use solana_program::instruction::CompiledInstruction;
use solana_program::message::v0::{LoadedAddresses, LoadedMessage};
use solana_program::message::VersionedMessage;
use solana_sdk::bs58;
use solana_sdk::instruction::AccountMeta;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
#[cfg(any(feature = "client", feature = "async_client"))]
use solana_sdk::signature::Signature;
use solana_sdk::transaction::TransactionError;
#[cfg(any(feature = "client", feature = "async_client"))]
use solana_transaction_status::UiTransactionEncoding;
use solana_transaction_status::{
    EncodedConfirmedTransactionWithStatusMeta, EncodedTransactionWithStatusMeta,
    UiInnerInstructions, UiInstruction, UiLoadedAddresses, UiTransactionStatusMeta,
};
use std::collections::HashMap;
use std::str::FromStr;

/// The transaction message itself, and any inner instructions extracted from it
/// by the runtime.
///
/// Since inner instructions are not encoded in a transaction message,
/// we need to pull it from the metadata sent when querying for historical
/// transaction data.
#[derive(Debug, Clone)]
pub struct HistoricalTransaction {
    pub message: VersionedMessage,
    /// Indexed by instruction number. We do not record nested inner instructions,
    /// as those are not returned from the Solana RPC `get_transaction` endpoint.
    /// Stored in a `HashMap` because sometimes an instruction will not have any inner instructions.
    pub inner_instructions: HashMap<u8, Vec<CompiledInstruction>>,

    pub loaded_addresses: Option<Vec<LoadedAddresses>>,
}

impl HistoricalTransaction {
    pub fn new(message: VersionedMessage, loaded_addresses: Option<Vec<LoadedAddresses>>) -> Self {
        Self {
            message,
            inner_instructions: Default::default(),
            loaded_addresses,
        }
    }

    #[cfg(feature = "async_client")]
    pub async fn get_nonblocking(
        client: &RpcClient,
        txid: &Signature,
    ) -> Result<Self, ClientError> {
        let tx = client
            .get_transaction_with_config(
                txid,
                RpcTransactionConfig {
                    encoding: Some(UiTransactionEncoding::Base64),
                    commitment: None,
                    max_supported_transaction_version: Some(0),
                },
            )
            .await?;
        Ok(Self::try_from(tx).unwrap())
    }

    #[cfg(feature = "client")]
    pub async fn get(
        client: &rpc_client::RpcClient,
        txid: &Signature,
    ) -> Result<Self, ClientError> {
        let tx = client.get_transaction_with_config(
            txid,
            RpcTransactionConfig {
                encoding: Some(UiTransactionEncoding::Base64),
                commitment: None,
                max_supported_transaction_version: Some(0),
            },
        )?;
        Ok(Self::try_from(tx).unwrap())
    }
}

impl TryFrom<EncodedConfirmedTransactionWithStatusMeta> for HistoricalTransaction {
    type Error = TransactionError;
    fn try_from(value: EncodedConfirmedTransactionWithStatusMeta) -> Result<Self, Self::Error> {
        let EncodedConfirmedTransactionWithStatusMeta {
            transaction:
                EncodedTransactionWithStatusMeta {
                    transaction, meta, ..
                },
            ..
        } = value;
        let (inner_instructions, loaded_addresses) = if let Some(UiTransactionStatusMeta {
            inner_instructions,
            loaded_addresses,
            ..
        }) = meta
        {
            let inner_instructions: Option<Vec<UiInnerInstructions>> = inner_instructions.into();
            let inner_instructions =
                extract_compiled_inner_instructions(inner_instructions.unwrap_or_default());
            let loaded_addresses: Option<UiLoadedAddresses> = loaded_addresses.into();
            let loaded_addresses = loaded_addresses.map(|ui_loaded_addresses| {
                vec![LoadedAddresses {
                    readonly: ui_loaded_addresses
                        .readonly
                        .iter()
                        .map(|s| Pubkey::from_str(s.as_str()).unwrap())
                        .collect(),
                    writable: ui_loaded_addresses
                        .writable
                        .iter()
                        .map(|s| Pubkey::from_str(s.as_str()).unwrap())
                        .collect(),
                }]
            });
            (inner_instructions, loaded_addresses)
        } else {
            (HashMap::<u8, Vec<CompiledInstruction>>::new(), None)
        };
        let transaction = transaction
            .decode()
            .ok_or(TransactionError::SanitizeFailure)?;
        Ok(Self {
            message: transaction.message,
            inner_instructions,
            loaded_addresses,
        })
    }
}

/// Convert a collectino of [UiInnerInstruction] to a compiled [CompiledInstruction].
pub fn extract_compiled_inner_instructions(
    ui_inner_instructions: Vec<UiInnerInstructions>,
) -> HashMap<u8, Vec<CompiledInstruction>> {
    HashMap::from_iter(ui_inner_instructions.into_iter().map(|inner_ix| {
        (
            inner_ix.index,
            inner_ix
                .instructions
                .into_iter()
                .map(|ix| match ix {
                    UiInstruction::Compiled(ix) => Some(CompiledInstruction {
                        program_id_index: ix.program_id_index,
                        accounts: ix.accounts,
                        data: bs58::decode(ix.data).into_vec().unwrap(),
                    }),
                    _ => None,
                })
                .into_iter()
                .flatten()
                .collect::<Vec<_>>(),
        )
    }))
}

#[derive(Debug, Clone)]
pub struct DecompiledMessageAndInnerIx {
    pub top_level_instructions: Vec<Instruction>,
    pub inner_instructions: HashMap<u8, Vec<Instruction>>,
    pub loaded_addresses: LoadedAddresses,
}

impl DecompiledMessageAndInnerIx {
    pub fn programs(&self) -> Vec<Pubkey> {
        let mut program_ids: Vec<Pubkey> = self
            .top_level_instructions
            .iter()
            .map(|ix| ix.program_id)
            .collect();
        self.inner_instructions.iter().for_each(|(_, inner_ixs)| {
            program_ids.extend(inner_ixs.iter().map(|ix| ix.program_id))
        });
        program_ids
    }
}

impl From<HistoricalTransaction> for DecompiledMessageAndInnerIx {
    fn from(value: HistoricalTransaction) -> Self {
        let loaded_addresses =
            LoadedAddresses::from_iter(value.loaded_addresses.unwrap_or_default());
        let addrs: Vec<Pubkey> = match &value.message {
            VersionedMessage::Legacy(message) => message.account_keys.clone(),
            VersionedMessage::V0(message) => {
                let message = LoadedMessage::new_borrowed(message, &loaded_addresses);
                message.account_keys().iter().map(|p| *p).collect()
            }
        };
        let is_writable = |idx| match &value.message {
            VersionedMessage::Legacy(m) => m.is_writable(idx),
            VersionedMessage::V0(m) => {
                LoadedMessage::new_borrowed(m, &loaded_addresses).is_writable(idx)
            }
        };
        let is_signer = |idx| match &value.message {
            VersionedMessage::Legacy(m) => m.is_signer(idx),
            VersionedMessage::V0(m) => {
                LoadedMessage::new_borrowed(m, &loaded_addresses).is_signer(idx)
            }
        };

        let top_level_instructions =
            extract_instructions_from_versioned_message(&value.message, &loaded_addresses);

        let mut inner_instructions = HashMap::new();
        for (idx, compiled_instructions) in value.inner_instructions {
            let inner_ix = compiled_instructions
                .iter()
                .map(|ix| {
                    let mut account_metas = vec![];
                    for idx in &ix.accounts {
                        let idx = *idx as usize;
                        let is_signer = is_signer(idx);
                        if is_writable(idx) {
                            account_metas
                                .push(AccountMeta::new(*addrs.get(idx).unwrap(), is_signer));
                        } else {
                            account_metas.push(AccountMeta::new_readonly(
                                *addrs.get(idx).unwrap(),
                                is_signer,
                            ));
                        }
                    }
                    let program = addrs.get(ix.program_id_index as usize).unwrap();
                    Instruction::new_with_bytes(*program, &ix.data, account_metas)
                })
                .collect();
            inner_instructions.insert(idx, inner_ix);
        }

        DecompiledMessageAndInnerIx {
            top_level_instructions,
            inner_instructions,
            loaded_addresses,
        }
    }
}
