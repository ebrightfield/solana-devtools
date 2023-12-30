mod faucet;
mod name_service;

#[cfg(feature = "faucet")]
use crate::faucet::FaucetSubcommand;
#[cfg(feature = "name-service")]
use crate::name_service::NameServiceSubcommand;
use anchor_spl::associated_token::get_associated_token_address;
use anchor_transaction_deser::AnchorLens;
use anyhow::{anyhow, Result};
use clap::{IntoApp, Parser};
use solana_clap_v3_utils::keypair::{pubkey_from_path, signer_from_path};
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_devtools_cli_config::{CommitmentArg, KeypairArg, UrlArg};
use solana_sdk::bs58;
use solana_sdk::hash::Hasher;
use solana_sdk::message::{Message, VersionedMessage};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
use spl_memo::build_memo;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::str::FromStr;

/// CLI for an improved Solana DX
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
                    pubkey_from_path(&matches, &path, "keypair", &mut None)
                        .map_err(|_| anyhow!("Invalid pubkey or path: {}", path))?
                } else {
                    self.keypair.resolve(&matches)?.pubkey()
                };
                let mint = pubkey_from_path(&matches, &mint, "keypair", &mut None)
                    .map_err(|_| anyhow!("Invalid pubkey or path: {}", mint))?;
                println!("{}", get_associated_token_address(&owner, &mint));
            }
            #[cfg(feature = "faucet")]
            Subcommand::Faucet(subcommand) => {
                let url = self.url.resolve()?;
                let commitment = self.commitment.resolve()?;
                let client = RpcClient::new_with_commitment(url, commitment);
                subcommand.process(&client, &self.keypair, &matches)?;
            }
            #[cfg(feature = "name-service")]
            Subcommand::NameService(subcommand) => {
                let url = self.url.resolve()?;
                let commitment = self.commitment.resolve()?;
                let client = RpcClient::new_with_commitment(url, commitment);
                subcommand.process(&client, &self.keypair, &matches)?;
            }
            Subcommand::Memo {
                msg,
                signer,
                hash_file,
            } => {
                let opt = Opt::into_app();
                let matches = opt.get_matches();
                let main_signer = self.keypair.resolve(&matches)?;
                let url = self.url.resolve()?;
                let commitment = self.commitment.resolve()?;
                let client = RpcClient::new_with_commitment(url, commitment);
                let mut signers: Vec<Box<dyn Signer>> = vec![];
                for path in signer {
                    signers.push(
                        signer_from_path(&matches, &path, "keypair", &mut None)
                            .map_err(|_| anyhow!("Invalid signer path: {}", path))?,
                    );
                }
                signers.push(main_signer);
                let signer_pubkeys: Vec<Pubkey> = signers.iter().map(|s| s.pubkey()).collect();
                let pubkey_refs: Vec<&Pubkey> = signer_pubkeys.iter().map(|p| p).collect();
                let msg = if hash_file {
                    let mut hasher = Hasher::default();
                    hasher.hash(&fs::read(msg)?);
                    hasher.result().to_string()
                } else {
                    msg
                };
                let ix = build_memo(msg.as_bytes(), &pubkey_refs);
                let tx = Transaction::new_signed_with_payer(
                    &[ix],
                    Some(&signer_pubkeys.last().unwrap()),
                    &signers,
                    client.get_latest_blockhash()?,
                );
                let signature = client.send_transaction(&tx).map_err(|e| {
                    println!("{:#?}", &e);
                    e
                })?;
                println!("{}", signature);
            }
            Subcommand::GetTransaction { txid, outfile } => {
                let url = self.url.resolve()?;
                let commitment = self.commitment.resolve()?;
                let client = RpcClient::new_with_commitment(url, commitment);
                let tx = client.get_transaction_with_config(
                    &Signature::from_str(&txid)?,
                    RpcTransactionConfig {
                        commitment: Some(commitment),
                        max_supported_transaction_version: Some(0),
                        ..Default::default()
                    },
                )?;
                let json = serde_json::to_string_pretty(&tx)?;
                if let Some(outfile) = outfile {
                    let mut file = File::create(outfile)?;
                    file.write(json.as_bytes())?;
                } else {
                    println!("{}", json);
                }
            }
            Subcommand::DeserializeTransaction { txid, idl, outfile } => {
                let url = self.url.resolve()?;
                let commitment = self.commitment.resolve()?;
                let client = RpcClient::new_with_commitment(url, commitment);
                let txid = Signature::from_str(&txid)?;
                let lens = if let Some(path) = idl {
                    let pieces: Vec<&str> = path.as_str().split(":").collect();
                    if pieces.len() != 2 {
                        return Err(anyhow!(
                            "Invalid idl argument, must be <program-id>:<filepath>"
                        ));
                    }
                    let prog_id = pieces[0].to_string();
                    let path = pieces[1].to_string();
                    AnchorLens::new_with_idl(client, prog_id, path, true).expect("Invalid IDL file")
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
            }
            Subcommand::DeserializeAccount {
                address,
                outfile,
                idl,
            } => {
                let url = self.url.resolve()?;
                let commitment = self.commitment.resolve()?;
                let client = RpcClient::new_with_commitment(url, commitment);
                let lens = if let Some(path) = idl {
                    let pieces: Vec<&str> = path.as_str().split(":").collect();
                    if pieces.len() != 2 {
                        return Err(anyhow!(
                            "Invalid idl argument, must be <program-id>:<filepath>"
                        ));
                    }
                    let prog_id = pieces[0].to_string();
                    let path = pieces[1].to_string();
                    AnchorLens::new_with_idl(client, prog_id, path, true).expect("Invalid IDL file")
                } else {
                    AnchorLens::new(client)
                };
                let address =
                    Pubkey::from_str(&address).map_err(|_| anyhow!("Invalid pubkey address"))?;
                let act = lens.fetch_and_deserialize_account(&address, None)?;
                let json = serde_json::to_string_pretty(&act)?;
                if let Some(outfile) = outfile {
                    let mut file = File::create(outfile)?;
                    file.write(json.as_bytes())?;
                } else {
                    println!("{}", json);
                }
            }
            Subcommand::DeserializeMessage {
                b58_message,
                outfile,
                idl,
            } => {
                let url = self.url.resolve()?;
                let commitment = self.commitment.resolve()?;
                let client = RpcClient::new_with_commitment(url, commitment);

                let lens = if let Some(path) = idl {
                    let pieces: Vec<&str> = path.as_str().split(":").collect();
                    if pieces.len() != 2 {
                        return Err(anyhow!(
                            "Invalid idl argument, must be <program-id>:<filepath>"
                        ));
                    }
                    let prog_id = pieces[0].to_string();
                    let path = pieces[1].to_string();
                    AnchorLens::new_with_idl(client, prog_id, path, true).expect("Invalid IDL file")
                } else {
                    AnchorLens::new(client)
                };

                let message = bs58::decode(b58_message)
                    .into_vec()
                    .map_err(|e| anyhow!("Failed to deserialize base58 message: {}", e))?;
                let message: Message = bincode::deserialize(&message)?;
                let message = VersionedMessage::Legacy(message);
                let json = lens.deserialize_message(&message)?;
                let json = serde_json::to_string_pretty(&json)?;
                if let Some(outfile) = outfile {
                    let mut file = File::create(outfile)?;
                    file.write(json.as_bytes())?;
                } else {
                    println!("{}", json);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Parser)]
enum Subcommand {
    /// Display the owner's associated token address for a given mint. Owner defaults
    /// to the configured signer.
    Ata { mint: String, owner: Option<String> },
    /// Execute a memo transaction.
    Memo {
        /// Message
        msg: String,
        /// If included, reinterprets `MSG` as a filepath,
        /// and hashes the contents of the file to use as a memo message.
        #[clap(long)]
        hash_file: bool,
        /// Additional signers of the memo
        #[clap(short, long)]
        signer: Vec<String>,
    },
    /// Execute a transaction on the SPL Token Faucet program.
    /// The program is on devnet at 4bXpkKSV8swHSnwqtzuboGPaPDeEgAn4Vt8GfarV5rZt.
    /// See https://github.com/paul-schaaf/spl-token-faucet for source code.
    #[cfg(feature = "faucet")]
    Faucet(FaucetSubcommand),
    /// Execute a transaction on the SPL Name Program
    #[cfg(feature = "name-service")]
    NameService(NameServiceSubcommand),
    /// A vanilla RPC call to get a confirmed transaction.
    GetTransaction {
        /// Transaction signature
        txid: String,
        /// Optionally write the data to a file as JSON.
        outfile: Option<String>,
    },
    /// Fetch a confirmed transaction and attempt to deserialize it using Anchor IDL data.
    DeserializeTransaction {
        /// Optionally supply the IDL filepath. Otherwise, the IDL data is fetched on-chain.
        #[clap(long)]
        idl: Option<String>,
        /// Optionally write the data to a file as JSON.
        #[clap(long)]
        outfile: Option<String>,
        /// Transaction signature
        txid: String,
    },
    /// Fetch account data and attempt to deserialize it using Anchor IDL data.
    DeserializeAccount {
        /// Optionally supply the IDL filepath. Otherwise, the IDL data is fetched on-chain.
        #[clap(long)]
        idl: Option<String>,
        /// Optionally write the data to a file as JSON.
        #[clap(long)]
        outfile: Option<String>,
        /// Account address
        address: String,
    },
    /// Deserialize an unsigned transaction message encoded in Base58
    DeserializeMessage {
        /// Optionally supply the IDL filepath. Otherwise, the IDL data is fetched on-chain.
        #[clap(long)]
        idl: Option<String>,
        /// Base58-encoded transaction message.
        b58_message: String,
        /// Optionally write the data to a file as JSON.
        #[clap(long)]
        outfile: Option<String>,
    },
}

fn main() -> Result<()> {
    let opt = Opt::parse();
    opt.process()?;
    Ok(())
}
