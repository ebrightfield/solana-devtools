use crate::idl_sdk::AnchorIdlSdkError;
use anchor_lang::{idl::IdlAccount, AccountDeserialize, AccountSerialize};
use anchor_syn::idl::types::Idl;
use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use solana_program::pubkey::Pubkey;
use std::io::{Read, Write};

/// Deserialize (and decompress) an IDL account, excluding its header.
pub fn deserialize_idl_account(data: &[u8]) -> Result<Idl, AnchorIdlSdkError> {
    if data.len() < 8 {
        return Err(AnchorIdlSdkError::DeserializeError);
    }

    let idl_account: IdlAccount = AccountDeserialize::try_deserialize(&mut &data[..])
        .map_err(|_| AnchorIdlSdkError::DeserializeError)?;
    let compressed_len: usize = idl_account.data_len.try_into().unwrap();
    let compressed_bytes = &data[44..44 + compressed_len];
    let mut z = ZlibDecoder::new(compressed_bytes);
    let mut s = Vec::new();
    z.read_to_end(&mut s)
        .map_err(|_| AnchorIdlSdkError::DecompressionError)?;
    let idl: Idl =
        serde_json::from_slice(&s[..]).map_err(|_| AnchorIdlSdkError::DeserializeError)?;
    Ok(idl)
}

/// Serialize an IDL account, including the header.
pub fn serialize_idl_account(
    idl: &Idl,
    authority: Option<Pubkey>,
) -> Result<Vec<u8>, AnchorIdlSdkError> {
    let idl_data = serialize_and_compress_idl(idl)?;
    let header = IdlAccount {
        authority: authority.unwrap_or(Pubkey::new_unique()),
        data_len: idl_data.len() as u32,
    };
    let mut account_data = Vec::new();
    header
        .try_serialize(&mut account_data)
        .map_err(|_| AnchorIdlSdkError::SerializeError)?;
    account_data.extend(idl_data);
    Ok(account_data)
}

/// Serialize and compress an [Idl] (not the entire account data, excludes the header).
pub fn serialize_and_compress_idl(idl: &Idl) -> Result<Vec<u8>, AnchorIdlSdkError> {
    let json_bytes = serde_json::to_vec(idl).map_err(|_| AnchorIdlSdkError::SerializeError)?;
    let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
    e.write_all(&json_bytes)
        .map_err(|_| AnchorIdlSdkError::CompressionError)?;
    let data = e
        .finish()
        .map_err(|_| AnchorIdlSdkError::CompressionError)?;
    Ok(data)
}
