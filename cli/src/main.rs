use anchor_spl::associated_token::get_associated_token_address;
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use clap::{IntoApp, Parser};
use solana_account_decoder::{UiAccount, UiAccountEncoding};
use solana_clap_v3_utils::keypair::{pubkey_from_path, signer_from_path};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_devtools_anchor_utils::deserialize::AnchorDeserializer;
use solana_devtools_cli_config::{CommitmentArg, KeypairArg, UrlArg};
use solana_devtools_tx::decompile_instructions::lookup_addresses;
use solana_devtools_tx::inner_instructions::HistoricalTransaction;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::hash::Hasher;
use solana_sdk::instruction::Instruction;
use solana_sdk::message::VersionedMessage;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::{Transaction, VersionedTransaction};
use solana_sdk::{borsh0_10, bs58};
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
    pub async fn process(self) -> Result<()> {
        let app = Opt::into_app();
        let matches = app.get_matches();
        let main_signer = self.keypair.resolve(None)?;
        let url = self.url.resolve(None)?;
        let commitment = self.commitment.resolve(None)?;
        match self.cmd {
            Subcommand::Address => {
                println!("{}", main_signer.pubkey());
            }
            Subcommand::DeserializeComputeIx { hex_data } => {
                let bytes = hex::decode(&hex_data.as_bytes())?;
                let ix: ComputeBudgetInstruction = borsh0_10::try_from_slice_unchecked(&bytes)?;
                println!("{:?}", ix);
            }
            Subcommand::CalculatePriorityFee {
                microlamports,
                budget,
            } => {
                println!("{}", microlamports * budget / 1_000_000);
            }
            Subcommand::Ata { mint, owner } => {
                let owner = if let Some(path) = owner {
                    pubkey_from_path(&matches, &path, "keypair", &mut None)
                        .map_err(|_| anyhow!("Invalid pubkey or path: {}", path))?
                } else {
                    main_signer.pubkey()
                };
                let mint = pubkey_from_path(&matches, &mint, "keypair", &mut None)
                    .map_err(|_| anyhow!("Invalid pubkey or path: {}", mint))?;
                println!("{}", get_associated_token_address(&owner, &mint));
            }
            Subcommand::Memo {
                msg,
                signer,
                hash_file,
            } => {
                let client = RpcClient::new_with_commitment(url, commitment);
                let mut signers: Vec<Box<dyn Signer>> = vec![];
                for path in signer {
                    signers.push(
                        signer_from_path(&matches, &path, "keypair", &mut None)
                            .map_err(|_| anyhow!("Invalid signer path: {}", path))?,
                    );
                }
                signers.push(Box::new(main_signer));
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
                    client.get_latest_blockhash().await?,
                );
                let signature = client.send_transaction(&tx).await.map_err(|e| {
                    println!("{:#?}", &e);
                    e
                })?;
                println!("{}", signature);
            }
            Subcommand::GetTransaction { txid, outfile } => {
                let client = RpcClient::new_with_commitment(url, commitment);
                let tx = client
                    .get_transaction_with_config(
                        &Signature::from_str(&txid)?,
                        RpcTransactionConfig {
                            commitment: Some(commitment),
                            max_supported_transaction_version: Some(0),
                            ..Default::default()
                        },
                    )
                    .await?;
                let json = serde_json::to_string_pretty(&tx)?;
                if let Some(outfile) = outfile {
                    let mut file = File::create(outfile)?;
                    file.write(json.as_bytes())?;
                } else {
                    println!("{}", json);
                }
            }
            Subcommand::DeserializeTransaction { txid, idl, outfile } => {
                let client = RpcClient::new_with_commitment(url, commitment);
                let txid = Signature::from_str(&txid)?;
                let mut deser = if let Some(path) = idl {
                    let pieces: Vec<&str> = path.as_str().split(":").collect();
                    if pieces.len() != 2 {
                        return Err(anyhow!(
                            "Invalid idl argument, must be <program-id>:<filepath>"
                        ));
                    }
                    let prog_id = Pubkey::from_str(pieces[0])?;
                    let path = pieces[1].to_string();
                    let mut deser = AnchorDeserializer::new();
                    deser.cache_idl_from_file(prog_id, path)?;
                    deser
                } else {
                    AnchorDeserializer::new()
                };
                let tx = HistoricalTransaction::get_nonblocking(&client, &txid).await?;
                deser.fetch_and_cache_any_idls(&client, tx.clone()).await?;
                let json = deser.try_deserialize_transaction(tx)?;
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
                let client = RpcClient::new_with_commitment(url, commitment);
                let deser = if let Some(path) = idl {
                    let pieces: Vec<&str> = path.as_str().split(":").collect();
                    if pieces.len() != 2 {
                        return Err(anyhow!(
                            "Invalid idl argument, must be <program-id>:<filepath>"
                        ));
                    }
                    let prog_id = Pubkey::from_str(pieces[0])?;
                    let path = pieces[1].to_string();
                    let mut deser = AnchorDeserializer::new();
                    deser.cache_idl_from_file(prog_id, path)?;
                    deser
                } else {
                    AnchorDeserializer::new()
                };
                let pubkey =
                    Pubkey::from_str(&address).map_err(|_| anyhow!("Invalid pubkey address"))?;
                let account = client.get_account(&pubkey).await?;

                let mut act = deser
                    .try_account_data_to_value(&account)
                    .map(|act| serde_json::to_value(act).expect("struct serializes"))?;

                let ui_account =
                    UiAccount::encode(&pubkey, &account, UiAccountEncoding::Base64, None, None);
                act.as_object_mut()
                    .unwrap()
                    .insert("ui_account".to_string(), serde_json::to_value(ui_account)?);

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
                base64,
                as_transaction,
            } => {
                let client = RpcClient::new_with_commitment(url, commitment);
                let deser = if let Some(path) = idl {
                    let pieces: Vec<&str> = path.as_str().split(":").collect();
                    if pieces.len() != 2 {
                        return Err(anyhow!(
                            "Invalid idl argument, must be <program-id>:<filepath>"
                        ));
                    }
                    let prog_id = Pubkey::from_str(pieces[0])?;
                    let path = pieces[1].to_string();
                    let mut deser = AnchorDeserializer::new();
                    deser
                        .cache_idl_from_file(prog_id, &path)
                        .map_err(|e| anyhow!("could not add IDL from filepath {}: {}", path, e))?;
                    deser
                } else {
                    AnchorDeserializer::new()
                };

                let message = if base64 {
                    STANDARD
                        .decode(b58_message)
                        .map_err(|e| anyhow!("Failed to deserialize base64 message: {e}"))?
                } else {
                    bs58::decode(b58_message)
                        .into_vec()
                        .map_err(|e| anyhow!("Failed to deserialize base58 message: {}", e))?
                };
                println!("Deserializing message");
                let message: VersionedMessage = if as_transaction {
                    let tx: VersionedTransaction = bincode::deserialize(&message)?;
                    tx.message
                } else {
                    bincode::deserialize(&message)?
                };
                let loaded_addresses = lookup_addresses(&client, &message).await?;

                let historical_tx = HistoricalTransaction::new(message, Some(loaded_addresses));

                let json = deser.try_deserialize_transaction(historical_tx)?;
                let json = serde_json::to_string_pretty(&json)?;
                if let Some(outfile) = outfile {
                    let mut file = File::create(outfile)?;
                    file.write(json.as_bytes())?;
                } else {
                    println!("{}", json);
                }
            }
            Subcommand::DeserializeInstruction {
                b58_instruction,
                outfile,
                idl,
            } => {
                let ix = bs58::decode(b58_instruction)
                    .into_vec()
                    .map_err(|e| anyhow!("Failed to deserialize base58 instruction: {}", e))?;
                let mut ix: Instruction = bincode::deserialize(&ix)?;

                let deser = if let Some(path) = idl {
                    let pieces: Vec<&str> = path.as_str().split(":").collect();
                    if pieces.len() != 2 {
                        return Err(anyhow!(
                            "Invalid idl argument, must be <program-id>:<filepath>"
                        ));
                    }
                    let prog_id = Pubkey::from_str(pieces[0])?;
                    let path = pieces[1].to_string();
                    let mut deser = AnchorDeserializer::new();
                    deser.cache_idl_from_file(prog_id, path)?;
                    deser
                } else {
                    let client = RpcClient::new_with_commitment(url, commitment);
                    // TODO Fetch an IDL from the program ID of the instruction
                    let mut deser = AnchorDeserializer::new();
                    deser
                        .fetch_and_cache_idl_for_program(&client, &ix.program_id)
                        .await?;
                    deser
                };

                let json = deser.try_deserialize_instruction(0, &mut ix, None)?;

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
    Address,
    /// Display the owner's associated token address for a given mint. Owner defaults
    /// to the configured signer.
    Ata {
        mint: String,
        owner: Option<String>,
    },
    DeserializeComputeIx {
        hex_data: String,
    },
    CalculatePriorityFee {
        microlamports: u64,
        budget: u64,
    },
    // TODO Pubkey subcommand,
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
        /// Optionally parse the message data as base64
        #[clap(long)]
        base64: bool,
        /// Optionally parse the message data as a serialized transaction, instead of a message
        #[clap(long)]
        as_transaction: bool,
    },
    /// Deserialize an instruction encoded in Base58
    DeserializeInstruction {
        /// Optionally supply the IDL filepath. Otherwise, the IDL data is fetched on-chain.
        #[clap(long)]
        idl: Option<String>,
        /// Base58-encoded instruction.
        b58_instruction: String,
        /// Optionally write the data to a file as JSON.
        #[clap(long)]
        outfile: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt = Opt::parse();
    opt.process().await?;
    Ok(())
}
