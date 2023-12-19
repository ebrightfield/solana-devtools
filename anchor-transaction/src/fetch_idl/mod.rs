pub mod discriminators;

use anyhow::anyhow;
use borsh::BorshDeserialize;
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
    let idl_account: IdlAccount = BorshDeserialize::deserialize(&mut d)?;

    let compressed_len: usize = idl_account.data_len.try_into().unwrap();
    let compressed_bytes = &account.data[44..44 + compressed_len];
    let mut z = ZlibDecoder::new(compressed_bytes);
    let mut s = Vec::new();
    z.read_to_end(&mut s)?;
    let idl = serde_json::from_slice(&s[..])
        .map_err(|_| anyhow!("Could not deserialize decompressed IDL data"))?;
    Ok(IdlWithDiscriminators::new(idl))
}

#[derive(BorshDeserialize)]
pub struct IdlAccount {
    // Address that can modify the IDL.
    pub authority: Pubkey,
    // Length of compressed idl bytes.
    pub data_len: u32,
    // Followed by compressed idl bytes.
}

impl IdlAccount {
    pub fn address(program_id: &Pubkey) -> Pubkey {
        let program_signer = Pubkey::find_program_address(&[], program_id).0;
        Pubkey::create_with_seed(&program_signer, IdlAccount::seed(), program_id)
            .expect("Seed is always valid")
    }
    pub fn seed() -> &'static str {
        "anchor:idl"
    }
}
