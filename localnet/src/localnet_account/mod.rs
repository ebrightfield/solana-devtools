use crate::error::{LocalnetConfigurationError, Result};
use anchor_lang::{system_program, AccountDeserialize, AccountSerialize};
use base64::{engine::general_purpose::STANDARD, Engine};
use inflector::Inflector;
use serde::{Deserialize, Serialize};
use solana_accounts_db::accounts_index::ZeroLamport;
use solana_client::rpc_client::RpcClient;
use solana_devtools_serde::pubkey;
use solana_sdk::{
    account::{Account, AccountSharedData, ReadableAccount, WritableAccount},
    bs58,
    clock::Epoch,
    pubkey::Pubkey,
};
use std::fs::{File, OpenOptions};

#[cfg(feature = "idl")]
pub mod idl;
pub mod system_account;
pub mod token;
pub mod trait_based;

pub use system_account::SystemAccount;
pub use token::{Mint, TokenAccount};

pub const THOUSAND_SOL: u64 = 1_000_000_000_000;

/// Builds JSON files consumable by `solana-test-validator`.
#[derive(Debug, Clone, Default)]
pub struct LocalnetAccount {
    pub address: Pubkey,
    pub lamports: u64,
    pub data: Vec<u8>,
    pub owner: Pubkey,
    pub executable: bool,
    pub rent_epoch: Epoch,
    pub name: String,
}

impl LocalnetAccount {
    pub fn new<T: AccountSerialize + AccountDeserialize>(
        address: Pubkey,
        name: String,
        data: T,
    ) -> Self {
        let mut serialized = Vec::new();
        data.try_serialize(&mut serialized).unwrap();
        Self {
            address,
            lamports: THOUSAND_SOL,
            name,
            data: serialized,
            owner: system_program::ID,
            executable: false,
            rent_epoch: 0,
        }
    }

    pub fn new_unnamed<T: AccountSerialize + AccountDeserialize>(address: Pubkey, data: T) -> Self {
        Self::new(address, address.to_string(), data)
    }

    pub fn new_raw(address: Pubkey, name: String, account_data: Vec<u8>) -> Self {
        Self {
            address,
            lamports: THOUSAND_SOL,
            name,
            data: account_data,
            owner: system_program::ID,
            executable: false,
            rent_epoch: 0,
        }
    }

    pub fn new_raw_unnamed(address: Pubkey, account_data: Vec<u8>) -> Self {
        Self::new_raw(address, address.to_string(), account_data)
    }

    pub fn new_from_readable_account(address: Pubkey, account: impl ReadableAccount) -> Self {
        Self {
            address,
            lamports: account.lamports(),
            data: account.data().to_vec(),
            owner: *account.owner(),
            executable: account.executable(),
            rent_epoch: account.rent_epoch(),
            name: address.to_string(),
        }
    }

    /// Copy and potentially modify an on-chain account.
    pub fn new_from_clone<T: AccountSerialize + AccountDeserialize, F: FnOnce(T) -> T>(
        address: &Pubkey,
        client: &RpcClient,
        name: String,
        modify: Option<F>,
    ) -> Result<Self> {
        let info = client
            .get_account(address)
            .map_err(|e| LocalnetConfigurationError::ClonedAccountRpcError(e))?;
        // Even if there is no modify function, deserialization verifies the expected account type
        let mut deserialized = T::try_deserialize(&mut info.data.as_slice())
            .map_err(|e| LocalnetConfigurationError::AnchorAccountError(e))?;
        // Maybe modify the account data.
        if let Some(func) = modify {
            deserialized = func(deserialized);
        }
        let mut serialized = Vec::new();
        deserialized
            .try_serialize(&mut serialized)
            .map_err(|e| LocalnetConfigurationError::AnchorAccountError(e))?;
        Ok(Self {
            address: address.clone(),
            lamports: info.lamports,
            name,
            data: serialized,
            owner: info.owner,
            executable: info.executable,
            rent_epoch: info.rent_epoch,
        })
    }

    /// There is no modification on this constructor, but also no deserialization.
    /// This is useful for blindly cloning accounts without having access to
    /// any type to which the data can deserialize.
    pub fn new_from_clone_unchecked(
        address: &Pubkey,
        client: &RpcClient,
        name: String,
    ) -> Result<Self> {
        let info = client
            .get_account(address)
            .map_err(|e| LocalnetConfigurationError::ClonedAccountRpcError(e))?;
        Ok(Self {
            address: address.clone(),
            lamports: info.lamports,
            name,
            data: info.data,
            owner: info.owner,
            executable: info.executable,
            rent_epoch: info.rent_epoch,
        })
    }

    pub fn from_ui_account(account: UiAccountWithAddr, name: String) -> Result<Self> {
        Ok(Self {
            address: account.pubkey,
            lamports: account.account.lamports,
            data: account.account.data.to_vec()?,
            owner: account.account.owner,
            executable: account.account.executable,
            rent_epoch: account.account.rent_epoch,
            name,
        })
    }

    pub fn lamports(mut self, balance: u64) -> Self {
        self.lamports = balance;
        self
    }

    pub fn owner(mut self, owner: Pubkey) -> Self {
        self.owner = owner;
        self
    }

    pub fn executable(mut self, executable: bool) -> Self {
        self.executable = executable;
        self
    }

    pub fn rent_epoch(mut self, rent_epoch: Epoch) -> Self {
        self.rent_epoch = rent_epoch;
        self
    }

    pub fn address(mut self, address: Pubkey) -> Self {
        self.address = address;
        self
    }

    pub fn data(mut self, data: Vec<u8>) -> Self {
        self.data = data;
        self
    }

    /// For inclusion in autogenerated imports that can be used
    /// in testing.
    pub fn js_import(&self) -> String {
        js_test_import(&self.name)
    }

    // TODO Use `Path` for cleaner handling of this, and maybe check is dir and exists
    pub fn json_output_path(&self, path_prefix: &str) -> String {
        if path_prefix.ends_with("/") {
            format!("{}{}", path_prefix, &self.name)
        } else {
            format!("{}/{}", path_prefix, &self.name)
        }
    }

    /// Write to a JSON file that can be consumed by `--account` flags in
    /// `solana-test-validator`.
    pub fn write_to_validator_json_file(&self, path_prefix: &str, overwrite: bool) -> Result<()> {
        let path = self.json_output_path(path_prefix);
        let file = if overwrite {
            File::create(&path)
        } else {
            OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .open(&path)
        }
        .map_err(|e| LocalnetConfigurationError::FileReadWriteError(path.clone(), e))?;
        let ui_act = UiAccount::from_localnet_account(&self);
        serde_json::to_writer_pretty(
            file,
            &UiAccountWithAddr {
                pubkey: self.address,
                account: ui_act,
            },
        )
        .map_err(|e| LocalnetConfigurationError::SerdeFileReadWriteFailure(path, e))?;
        Ok(())
    }
}

/// Takes a filepath to a JSON file, and produces a source code string
/// that both imports the JSON as well as extracts the public key object.
/// JS identifier for each pubkey is based off the JSON filename.
pub fn js_test_import(location: &str) -> String {
    let location = if !location.ends_with(".json") {
        location.to_string()
    } else {
        let (location, _) = location.split_at(location.len() - 5);
        location.to_string()
    };
    let name = {
        let mut pieces = location.rsplit('/');
        match pieces.next() {
            Some(p) => p.to_string(),
            None => location.to_string(),
        }
    };
    // Turn it into "camelCase" ending in "Json", e.g. i_mint.json -> iMintJson.
    let name = name.to_string().to_camel_case();
    // Output an import statement
    // and its subsequent extraction of the Typescript `PublicKey` object.
    format!("import * as {}Json from \"./{}.json\";\nexport const {} = new anchor.web3.PublicKey({}Json.pubkey);", &name, &location, &name, &name)
}

/// Conforms to the JSON output format from RPC endpoint `getAccount`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UiAccountWithAddr {
    #[serde(with = "pubkey")]
    pub pubkey: Pubkey,
    pub account: UiAccount,
}

/// The inner data for [UiAccountWithAddr].
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UiAccount {
    pub lamports: u64,
    pub data: UiAccountData,
    #[serde(with = "pubkey")]
    pub owner: Pubkey,
    pub executable: bool,
    pub rent_epoch: Epoch,
    pub space: Option<u64>,
}

impl UiAccount {
    pub fn from_localnet_account(act: &LocalnetAccount) -> Self {
        Self {
            lamports: act.lamports,
            data: UiAccountData::Binary(STANDARD.encode(&act.data), UiAccountEncoding::Base64),
            owner: act.owner,
            executable: act.executable,
            rent_epoch: act.rent_epoch,
            space: Some(act.data.len() as u64),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum UiAccountData {
    Binary(String, UiAccountEncoding),
}

impl UiAccountData {
    pub fn to_vec(&self) -> Result<Vec<u8>> {
        match &self {
            UiAccountData::Binary(data, encoding) => match encoding {
                UiAccountEncoding::Base58 => bs58::decode(data)
                    .into_vec()
                    .map_err(|e| LocalnetConfigurationError::InvalidBase58AccountData(e)),
                UiAccountEncoding::Base64 => STANDARD
                    .decode(data)
                    .map_err(|e| LocalnetConfigurationError::InvalidBase64AccountData(e)),
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum UiAccountEncoding {
    Base58,
    Base64,
}

impl Into<AccountSharedData> for LocalnetAccount {
    fn into(self) -> AccountSharedData {
        AccountSharedData::create(
            self.lamports,
            self.data,
            self.owner,
            self.executable,
            self.rent_epoch,
        )
    }
}

impl Into<AccountSharedData> for &LocalnetAccount {
    fn into(self) -> AccountSharedData {
        AccountSharedData::create(
            self.lamports,
            self.data.clone(),
            self.owner,
            self.executable,
            self.rent_epoch,
        )
    }
}

impl Into<Account> for LocalnetAccount {
    fn into(self) -> Account {
        Account::create(
            self.lamports,
            self.data,
            self.owner,
            self.executable,
            self.rent_epoch,
        )
    }
}

impl Into<Account> for &LocalnetAccount {
    fn into(self) -> Account {
        Account::create(
            self.lamports,
            self.data.clone(),
            self.owner,
            self.executable,
            self.rent_epoch,
        )
    }
}

impl ReadableAccount for LocalnetAccount {
    fn lamports(&self) -> u64 {
        self.lamports
    }
    fn data(&self) -> &[u8] {
        &self.data
    }
    fn owner(&self) -> &Pubkey {
        &self.owner
    }
    fn executable(&self) -> bool {
        self.executable
    }
    fn rent_epoch(&self) -> Epoch {
        self.rent_epoch
    }
}

impl ZeroLamport for LocalnetAccount {
    fn is_zero_lamport(&self) -> bool {
        self.lamports == 0
    }
}
