use crate::error::{LocalnetConfigurationError, Result};
use crate::LocalnetAccount;
use anchor_lang::AccountSerialize;
use anchor_syn::idl::Idl;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use solana_program::pubkey::Pubkey;
use std::io::Write;
use std::path::Path;

pub fn idl_account_from_lib_rs(
    lib_rs: &str,
    program_id: &Pubkey,
    json_filename: &str,
    idl_authority: Option<Pubkey>,
) -> Result<LocalnetAccount> {
    let idl = parse_idl_from_lib_rs(lib_rs)?;
    let account_data = serialize_idl_for_on_chain(&idl, idl_authority)?;
    Ok(LocalnetAccount::new_raw(
        anchor_lang::idl::IdlAccount::address(program_id),
        json_filename.to_string(),
        account_data,
    )
    .set_owner(*program_id))
}

pub fn parse_idl_from_lib_rs<P: AsRef<Path>>(lib_rs: P) -> Result<Idl> {
    anchor_syn::idl::file::parse(&lib_rs, "0.0.0".to_string(), false, false, false)
        .map_err(|e| LocalnetConfigurationError::IdlParseError(e.to_string()))?
        .ok_or(LocalnetConfigurationError::IdlParseError(
            "parse function returned no IDL".to_string(),
        ))
}

pub fn serialize_idl_for_on_chain(idl: &Idl, authority: Option<Pubkey>) -> Result<Vec<u8>> {
    let json_bytes = serde_json::to_vec(idl)
        .map_err(|e| LocalnetConfigurationError::IdlSerializationError(e))?;
    let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
    e.write_all(&json_bytes)
        .map_err(|e| LocalnetConfigurationError::IdlCompressionError(e))?;
    let idl_data = e
        .finish()
        .map_err(|e| LocalnetConfigurationError::IdlCompressionError(e))?;
    let header = anchor_lang::idl::IdlAccount {
        authority: authority.unwrap_or(Pubkey::new_unique()),
        data_len: idl_data.len() as u32,
    };
    let mut account_data = Vec::new();
    header
        .try_serialize(&mut account_data)
        .map_err(|e| LocalnetConfigurationError::AnchorAccountError(e))?;
    account_data.extend(idl_data);
    Ok(account_data)
}
