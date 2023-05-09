pub mod discriminators;

use anchor_lang::idl::IdlAccount;
use anyhow::anyhow;
use borsh::BorshDeserialize as AnchorDeserialize;
use flate2::read::ZlibDecoder;
use solana_client::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use solana_sdk::commitment_config::CommitmentConfig;
use std::io::Read;

pub use discriminators::IdlWithDiscriminators;

/// Fetches an IDL from on-chain account data, if it exists, and returns an
/// [IdlWithDiscriminators].
pub fn fetch_idl(client: &RpcClient, idl_addr: &Pubkey) -> anyhow::Result<IdlWithDiscriminators> {
    let mut account = client
        .get_account_with_commitment(idl_addr, CommitmentConfig::processed())?
        .value
        .map_or(Err(anyhow!("IDL account not found")), Ok)?;

    if account.executable {
        let idl_addr = IdlAccount::address(idl_addr);
        account = client
            .get_account_with_commitment(&idl_addr, CommitmentConfig::processed())?
            .value
            .map_or(Err(anyhow!("IDL account not found")), Ok)?;
    }

    if account.data.len() < 8 {
        return Err(anyhow!("IDL account is the wrong size"));
    }
    // Cut off account discriminator.
    let mut d: &[u8] = &account.data[8..];
    let idl_account: IdlAccount = AnchorDeserialize::deserialize(&mut d)?;

    let compressed_len: usize = idl_account.data_len.try_into().unwrap();
    let compressed_bytes = &account.data[44..44 + compressed_len];
    let mut z = ZlibDecoder::new(compressed_bytes);
    let mut s = Vec::new();
    z.read_to_end(&mut s)?;
    let idl = serde_json::from_slice(&s[..])
        .map_err(|_| anyhow!("Could not deserialize decompressed IDL data"))?;
    Ok(IdlWithDiscriminators::new(idl))
}
