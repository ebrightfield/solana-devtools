use std::time::Duration;
use log::error;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use tokio::task::JoinHandle;
use tokio::time::sleep;

#[async_trait::async_trait]
pub trait PolledAccount: Clone + Send + 'static {
    type AccountType: anchor_lang::AccountDeserialize + Send;
    type Error: std::fmt::Debug;

    fn rpc_url(&self) -> String;
    fn address(&self) -> Pubkey;

    fn poll_every(&self) -> Duration;

    async fn on_account(self, account: Self::AccountType) -> Result<(), Self::Error>;

    fn poll_account(&self) -> JoinHandle<()> {
        let client = RpcClient::new_with_commitment(self.rpc_url(), CommitmentConfig::finalized());
        let state = self.clone();
        tokio::spawn(async move {
            loop {
                let address = state.address();
                let duration = state.poll_every();
                let state = state.clone();
                match client.get_account_data(&address).await {
                    Ok(account) => {
                        let account: anchor_lang::Result<Self::AccountType> =
                            anchor_lang::AccountDeserialize::try_deserialize(
                                &mut account.as_slice(),
                            );
                        match account {
                            Ok(account) => {
                                if let Err(e) = state.on_account(account).await {
                                    error!(
                                        "Uncaught error during account polling for {}: {:?}",
                                        address, e
                                    );
                                }
                            }
                            Err(e) => {
                                error!(
                                    "error parsing account data for {}: {}",
                                    address,
                                    e.to_string()
                                );
                            }
                        }
                    }
                    Err(e) => {
                        error!("failed to fetch account data for {}: {:?}", address, e);
                    }
                }
                sleep(duration).await;
            }
        })
    }
}
