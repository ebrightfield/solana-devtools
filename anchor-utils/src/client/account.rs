use solana_client::client_error::{ClientError, ClientErrorKind};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_client;
use solana_program::pubkey::Pubkey;
use anchor_lang::prelude::AccountDeserialize;

pub async fn get_state<T: AccountDeserialize>(
    address: &Pubkey,
    client: &RpcClient,
) -> Result<T, ClientError> {
    let data = client
        .get_account_data(address)
        .await?;
    T::try_deserialize(&mut data.as_slice())
        .map_err(|_| ClientError::from(ClientErrorKind::Custom("account did not deserialize".to_string())))
}

pub fn get_state_blocking<T: AccountDeserialize>(
    address: &Pubkey,
    client: &rpc_client::RpcClient,
) -> Result<T, ClientError> {
    let data = client
        .get_account_data(address)?;
    T::try_deserialize(&mut data.as_slice())
        .map_err(|_| ClientError::from(ClientErrorKind::Custom("account did not deserialize".to_string())))
}
