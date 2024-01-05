use anchor_lang::prelude::{AccountDeserialize, AccountSerialize};
use solana_client::client_error::{ClientError, ClientErrorKind};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_client;
use solana_program::pubkey::Pubkey;
use std::thread::sleep;
use std::time::Duration;

pub async fn get_state<T: AccountDeserialize>(
    address: &Pubkey,
    client: &RpcClient,
) -> Result<T, ClientError> {
    let data = client.get_account_data(address).await?;
    T::try_deserialize(&mut data.as_slice()).map_err(|_| {
        ClientError::from(ClientErrorKind::Custom(
            "account did not deserialize".to_string(),
        ))
    })
}

pub fn get_state_blocking<T: AccountDeserialize>(
    address: &Pubkey,
    client: &rpc_client::RpcClient,
) -> Result<T, ClientError> {
    let data = client.get_account_data(address)?;
    T::try_deserialize(&mut data.as_slice()).map_err(|_| {
        ClientError::from(ClientErrorKind::Custom(
            "account did not deserialize".to_string(),
        ))
    })
}

/// Uses `RpcClient::get_multiple_accounts` to fetch accounts, deserialize them,
/// and for each account, calls a function, in case data needs to be extracted, etc.
pub async fn get_anchor_accounts<T: AccountSerialize + AccountDeserialize>(
    addresses: &[Pubkey],
    client: &RpcClient,
    sleep_between_requests: Option<Duration>,
    mut for_each_account: impl FnMut(&T),
) -> Result<Vec<T>, ClientError> {
    let mut accounts = vec![];
    for pubkeys in addresses.chunks(5) {
        let deserialized_accounts = client
            .get_multiple_accounts(pubkeys)
            .await?
            .iter()
            .map(|opt_act| {
                opt_act
                    .as_ref()
                    .map(|act| {
                        let act = T::try_deserialize(&mut &act.data[..]).map_err(|_| {
                            ClientError::from(ClientErrorKind::Custom(
                                "failed to deserialize account".to_string(),
                            ))
                        })?;
                        for_each_account(&act);
                        Ok(act)
                    })
                    .ok_or(ClientError::from(ClientErrorKind::Custom(format!(
                        "one or more accounts not found from {:?}",
                        pubkeys
                    ))))
            })
            .flatten()
            .collect::<Result<Vec<T>, ClientError>>()?;
        accounts.extend(deserialized_accounts);
        if let Some(dur) = sleep_between_requests {
            sleep(dur);
        }
    }
    Ok(accounts)
}
