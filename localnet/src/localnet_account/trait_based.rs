use anchor_lang::{AccountDeserialize, AccountSerialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::account::Account;
use solana_program::clock::Epoch;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;
use crate::localnet_account::THOUSAND_SOL;
use crate::LocalnetAccount;
use crate::error::{LocalnetConfigurationError, Result};

/// Create account data wholecloth, from any type that implements
/// [anchor_lang::AccountSerialize] and [anchor_lang::AccountDeserialize].
pub trait GeneratedAccount {
    type Data: AccountSerialize + AccountDeserialize;

    fn address(&self) -> Pubkey;

    fn generate(&self) -> Self::Data;

    fn lamports(&self) -> u64 {
        THOUSAND_SOL
    }

    fn owner(&self) -> Pubkey {
        system_program::id()
    }

    fn executable(&self) -> bool {
        false
    }

    fn rent_epoch(&self) -> Epoch {
        0
    }

    fn name(&self) -> String {
        format!("{}.json", self.address().to_string())
    }

    fn to_localnet_account(&self) -> LocalnetAccount {
        let data = self.generate();
        let mut buf = vec![];
        data.try_serialize(&mut buf).unwrap();
        LocalnetAccount {
            address: self.address(),
            lamports: self.lamports(),
            data: buf,
            owner: self.owner(),
            executable: self.executable(),
            rent_epoch: self.rent_epoch(),
            name: self.name(),
        }
    }
}

impl<T: GeneratedAccount> From<&T> for LocalnetAccount {
    fn from(value: &T) -> Self {
        let data = value.generate();
        let mut buf = vec![];
        data.try_serialize(&mut buf).unwrap();
        LocalnetAccount {
            address: value.address(),
            lamports: value.lamports(),
            data: buf,
            owner: value.owner(),
            executable: value.executable(),
            rent_epoch: value.rent_epoch(),
            name: value.name(),
        }
    }
}

/// Clone an account from a cluster, and optionally modify it.
/// Only works on account types that implement [anchor_lang::AccountSerialize]
/// and [anchor_lang::AccountDeserialize].
pub trait ClonedAccount {
    type Data: AccountSerialize + AccountDeserialize;

    fn address(&self) -> Pubkey;

    fn name(&self) -> String {
        format!("{}.json", self.address().to_string())
    }

    /// Default implementation performs no modification
    fn modify(&self, deserialized: Self::Data) -> Self::Data {
        deserialized
    }

    fn fetch_and_modify_data(&self, client: &RpcClient) -> Result<(Account, Self::Data)> {
        let address = self.address();
        let info = client
            .get_account(&address)
            .map_err(|e| LocalnetConfigurationError::ClonedAccountRpcError(e))?;
        let deserialized = Self::Data::try_deserialize(
            &mut info.data.as_slice())
            .map_err(|e| LocalnetConfigurationError::AnchorAccountError(e))?;
        Ok((info, self.modify(deserialized)))
    }

    fn to_localnet_account(&self, client: &RpcClient) -> Result<LocalnetAccount> {
        let (act, data) = self.fetch_and_modify_data(client)?;
        let mut buf = vec![];
        data.try_serialize(&mut buf).unwrap();
        Ok(LocalnetAccount {
            address: self.address(),
            lamports: act.lamports,
            data: buf,
            owner: act.owner,
            executable: act.executable,
            rent_epoch: act.rent_epoch,
            name: self.name()
        })
    }
}