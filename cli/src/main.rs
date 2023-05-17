mod faucet;

use std::fs::File;
use std::io::Write;
use std::str::FromStr;
use anchor_spl::associated_token::get_associated_token_address;
use anyhow::{anyhow, Result};
use clap::{IntoApp, Parser};
use solana_clap_v3_utils::keypair::pubkey_from_path;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use anchor_transaction_deser::AnchorLens;
use solana_devtools_cli_config::{CommitmentArg, KeypairArg, UrlArg};
use crate::faucet::FaucetSubcommand;

#[derive(Debug, Parser)]
struct Opt {
    #[clap(flatten)]
    url: UrlArg,
    #[clap(flatten)]
    keypair: KeypairArg,
    #[clap(flatten)]
    commitment: CommitmentArg,
    #[clap(subcommand)]
    cmd: Subcommand,
}

impl Opt {
    pub fn process(self) -> Result<()> {
        let app = Opt::into_app();
        let matches = app.get_matches();
        match self.cmd {
            Subcommand::Ata { mint, owner } => {
                let owner = if let Some(path) = owner {
                    pubkey_from_path(
                        &matches,
                        &mint,
                        "keypair",
                        &mut None
                    ).map_err(|_| anyhow!("Invalid pubkey or path: {}", path))?
                } else {
                    self.keypair.resolve(&matches)?.pubkey()
                };
                let mint = pubkey_from_path(
                    &matches,
                    &mint,
                    "keypair",
                    &mut None
                ).map_err(|_| anyhow!("Invalid pubkey or path: {}", mint))?;
                println!("{}", get_associated_token_address(&owner, &mint));
            },
            Subcommand::Faucet(subcommand) => {
                let url = self.url.resolve()?;
                let commitment = self.commitment.resolve()?;
                let client = RpcClient::new_with_commitment(url, commitment);
                subcommand.process(&client, &self.keypair, &matches)?;
            },
            Subcommand::GetTransaction { txid, outfile } => {
                let url = self.url.resolve()?;
                let commitment = self.commitment.resolve()?;
                let client = RpcClient::new_with_commitment(url, commitment);
                let tx = client.get_transaction_with_config(
                    &Signature::from_str(&txid)?,
                    RpcTransactionConfig {
                        commitment: Some(commitment),
                        //encoding: Some(),
                        ..Default::default()
                    }
                )?;
                let json = serde_json::to_string_pretty(&tx)?;
                if let Some(outfile) = outfile {
                    let mut file = File::create(outfile)?;
                    file.write(json.as_bytes())?;
                } else {
                    println!("{}", json);
                }
            },
            Subcommand::DeserializeTransaction { txid, idl, outfile } => {
                let url = self.url.resolve()?;
                let commitment = self.commitment.resolve()?;
                let client = RpcClient::new_with_commitment(url, commitment);
                let txid = Signature::from_str(&txid)?;
                let lens = if let Some(path) = idl {
                    let pieces: Vec<&str> = path.as_str().split(":").collect();
                    if pieces.len() != 2 {
                        return Err(anyhow!("Invalid idl argument, must be <program-id>:<filepath>"));
                    }
                    let prog_id = pieces[0].to_string();
                    let path = pieces[1].to_string();
                    AnchorLens::new_with_idl(client, prog_id, path, true)
                        .expect("Invalid IDL file")
                } else {
                    AnchorLens::new(client)
                };
                let tx = lens.get_versioned_transaction(&txid)?;
                let json = lens.deserialize_transaction(tx)?;
                let json = serde_json::to_string_pretty(&json)?;
                if let Some(outfile) = outfile {
                    let mut file = File::create(outfile)?;
                    file.write(json.as_bytes())?;
                } else {
                    println!("{}", json);
                }
            },
            Subcommand::DeserializeAccount { address, outfile, idl } => {
                let url = self.url.resolve()?;
                let commitment = self.commitment.resolve()?;
                let client = RpcClient::new_with_commitment(url, commitment);
                let lens = if let Some(path) = idl {
                    let pieces: Vec<&str> = path.as_str().split(":").collect();
                    if pieces.len() != 2 {
                        return Err(anyhow!("Invalid idl argument, must be <program-id>:<filepath>"));
                    }
                    let prog_id = pieces[0].to_string();
                    let path = pieces[1].to_string();
                    AnchorLens::new_with_idl(client, prog_id, path, true)
                        .expect("Invalid IDL file")
                } else {
                    AnchorLens::new(client)
                };
                let address = Pubkey::from_str(&address)
                    .map_err(|_| anyhow!("Invalid pubkey address"))?;
                let act = lens.fetch_and_deserialize_account(&address, None)?;
                let json = serde_json::to_string_pretty(&act)?;
                if let Some(outfile) = outfile {
                    let mut file = File::create(outfile)?;
                    file.write(json.as_bytes())?;
                } else {
                    println!("{}", json);
                }
            },
        }
        Ok(())
    }
}

#[derive(Debug, Parser)]
enum Subcommand {
    /// Display the owner's associated token address for a given mint. Owner defaults
    /// to the configured signer.
    Ata { mint: String, owner: Option<String> },
    #[clap(subcommand)]
    Faucet(FaucetSubcommand),
    GetTransaction { txid: String, outfile: Option<String> },
    DeserializeTransaction {
        #[clap(long)]
        idl: Option<String>,
        #[clap(long)]
        outfile: Option<String>,
        txid: String,
    },
    DeserializeAccount {
        #[clap(long)]
        idl: Option<String>,
        #[clap(long)]
        outfile: Option<String>,
        address: String,
    },
}

fn main() -> Result<()> {
    let opt = Opt::parse();
    opt.process()?;
    Ok(())
}
