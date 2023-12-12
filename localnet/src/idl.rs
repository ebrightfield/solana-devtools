use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use anchor_cli::config::Program;
use anchor_syn::idl::Idl;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

/// Serialize and compress the idl.
pub fn on_chain_idl_account_data(idl_file: &str) -> Result<Vec<u8>> {
    let file = shellexpand::tilde(idl_file);
    let idl = anchor_syn::idl::file::parse(
        &*file,
        "0.0.0".to_string(),
        false,
        false,
        false,
    )?
        .ok_or(anyhow!("Failed to parse idl: {}", file))?;
    let json_bytes = serde_json::to_vec(&idl)?;
    let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
    e.write_all(&json_bytes)?;
    e.finish().map_err(Into::into)
}

/// Used to write an "address" field to the IDL file.
#[derive(Debug, Serialize, Deserialize)]
pub struct IdlTestMetadata {
    pub address: String,
}

impl IdlTestMetadata {
    pub fn from_program(program: &Program) -> Result<Self> {
        let mut file = File::open(&format!("target/idl/{}.json", program.lib_name))?;
        let mut contents = vec![];
        file.read_to_end(&mut contents)?;
        let idl: Idl = serde_json::from_slice(&contents)?;
        let metadata = idl.metadata.ok_or_else(|| {
            anyhow!(
                "Metadata property not found in IDL of program: {}",
                program.lib_name
            )
        })?;
        Ok(serde_json::from_value(metadata)?)
    }

    pub fn write_to_file(&self, idl: &mut Idl) -> Result<()> {
        // Add program address to the IDL.
        idl.metadata = Some(serde_json::to_value(self)?);

        // Persist it.
        let idl_out = PathBuf::from("target/idl")
            .join(&idl.name)
            .with_extension("json");
        let idl_json = serde_json::to_string_pretty(idl)?;
        fs::write(idl_out, idl_json)
            .map_err(|e| anyhow!("Failed to write IDL to file: {}", e))?;
        Ok(())
    }
}
