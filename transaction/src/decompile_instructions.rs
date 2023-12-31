#[cfg(any(feature = "async_client", feature = "client"))]
use solana_address_lookup_table_program::state::AddressLookupTable;
#[cfg(any(feature = "async_client", feature = "client"))]
use solana_client::client_error::{ClientError, ClientErrorKind};
#[cfg(feature = "async_client")]
use solana_client::nonblocking::rpc_client;
#[cfg(feature = "client")]
use solana_client::rpc_client::RpcClient;
use solana_program::message::v0::{LoadedAddresses, LoadedMessage};
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::message::{Message, SanitizedMessage, VersionedMessage};
use solana_sdk::pubkey::Pubkey;

/// Decompile a [VersionedMessage] back into its instructions.
pub fn extract_instructions_from_versioned_message(
    message: &VersionedMessage,
    loaded_addresses: &LoadedAddresses,
) -> Vec<Instruction> {
    match &message {
        VersionedMessage::Legacy(message) => extract_instructions_from_message(message),
        VersionedMessage::V0(message) => {
            let loaded_message = LoadedMessage::new_borrowed(message, loaded_addresses);
            let addrs: Vec<Pubkey> = loaded_message.account_keys().iter().map(|p| *p).collect();
            message
                .instructions
                .iter()
                .map(|ix| {
                    let mut account_metas = vec![];
                    for idx in &ix.accounts {
                        let idx = *idx as usize;
                        let is_signer = loaded_message.is_signer(idx);
                        if loaded_message.is_writable(idx) {
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
                .collect()
        }
    }
}

/// Decompile a [Message] back into its instructions.
pub fn extract_instructions_from_message(message: &Message) -> Vec<Instruction> {
    let message = SanitizedMessage::try_from(message.clone()).unwrap();
    message
        .decompile_instructions()
        .iter()
        .map(|ix| {
            Instruction::new_with_bytes(
                *ix.program_id,
                ix.data,
                ix.accounts
                    .iter()
                    .map(|act| AccountMeta {
                        pubkey: *act.pubkey,
                        is_signer: act.is_signer,
                        is_writable: act.is_writable,
                    })
                    .collect(),
            )
        })
        .collect()
}

#[cfg(feature = "async_client")]
pub async fn lookup_addresses(
    client: &rpc_client::RpcClient,
    message: &VersionedMessage,
) -> Result<Vec<LoadedAddresses>, ClientError> {
    match message {
        VersionedMessage::Legacy(_) => Ok(vec![]),
        VersionedMessage::V0(m) => {
            let mut loaded_addresses = vec![];
            for lookup in &m.address_table_lookups {
                let account = client.get_account(&lookup.account_key).await?;
                let lookup_table =
                    AddressLookupTable::deserialize(&account.data).map_err(|_| {
                        ClientError::from(ClientErrorKind::Custom(
                            "failed to deserialize account lookup table".to_string(),
                        ))
                    })?;
                loaded_addresses.push(LoadedAddresses {
                    writable: lookup
                        .writable_indexes
                        .iter()
                        .map(|idx| {
                            lookup_table.addresses.get(*idx as usize).map(|p| *p).ok_or(
                                ClientError::from(ClientErrorKind::Custom(
                                    "account lookup went out of bounds of address lookup table"
                                        .to_string(),
                                )),
                            )
                        })
                        .collect::<Result<_, _>>()?,
                    readonly: lookup
                        .readonly_indexes
                        .iter()
                        .map(|idx| {
                            lookup_table.addresses.get(*idx as usize).map(|p| *p).ok_or(
                                ClientError::from(ClientErrorKind::Custom(
                                    "account lookup went out of bounds of address lookup table"
                                        .to_string(),
                                )),
                            )
                        })
                        .collect::<Result<_, _>>()?,
                });
            }
            Ok(loaded_addresses)
        }
    }
}

#[cfg(feature = "client")]
pub fn lookup_addresses_blocking(
    client: &RpcClient,
    message: &VersionedMessage,
) -> Result<Vec<LoadedAddresses>, ClientError> {
    match message {
        VersionedMessage::Legacy(_) => Ok(vec![]),
        VersionedMessage::V0(m) => {
            let mut loaded_addresses = vec![];
            for lookup in &m.address_table_lookups {
                let account = client.get_account(&lookup.account_key)?;
                let lookup_table =
                    AddressLookupTable::deserialize(&account.data).map_err(|_| {
                        ClientError::from(ClientErrorKind::Custom(
                            "failed to deserialize account lookup table".to_string(),
                        ))
                    })?;
                loaded_addresses.push(LoadedAddresses {
                    writable: lookup
                        .writable_indexes
                        .iter()
                        .map(|idx| {
                            lookup_table.addresses.get(*idx as usize).map(|p| *p).ok_or(
                                ClientError::from(ClientErrorKind::Custom(
                                    "account lookup went out of bounds of address lookup table"
                                        .to_string(),
                                )),
                            )
                        })
                        .collect::<Result<_, _>>()?,
                    readonly: lookup
                        .readonly_indexes
                        .iter()
                        .map(|idx| {
                            lookup_table.addresses.get(*idx as usize).map(|p| *p).ok_or(
                                ClientError::from(ClientErrorKind::Custom(
                                    "account lookup went out of bounds of address lookup table"
                                        .to_string(),
                                )),
                            )
                        })
                        .collect::<Result<_, _>>()?,
                });
            }
            Ok(loaded_addresses)
        }
    }
}
