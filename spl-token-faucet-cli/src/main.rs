mod config;

use anchor_lang::Id;
use anchor_spl::associated_token::get_associated_token_address;
use anchor_spl::token::Token;
use anyhow::{anyhow, Result};
use clap::{IntoApp, Parser};
use solana_clap_v3_utils::keypair::{keypair_from_path, pubkey_from_path};
use solana_client::rpc_client::RpcClient;
use solana_sdk::program_pack::Pack;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
use spl_associated_token_account::instruction::create_associated_token_account;
use spl_token::instruction::set_authority;
use spl_token_faucet::state::Faucet;
use spl_token_faucet_cli::{init_faucet_account, init_faucet_instruction, init_faucet_mint, mint_tokens_instruction};
use crate::config::{KeypairArg, UrlArg};

// TODO Close instruction
// TODO Admin option on creation and mint and close

#[derive(Debug, Parser)]
struct Opt {
    #[clap(flatten)]
    url: UrlArg,
    #[clap(flatten)]
    keypair: KeypairArg,
    #[clap(subcommand)]
    cmd: Subcommand,
}

impl Opt {
    pub fn process(self) -> Result<()> {
        let url = self.url.resolve(None)?;
        let client = RpcClient::new(url);
        match self.cmd {
            Subcommand::ConfigureSplMint { mint_pubkey } => {
                let mint = pubkey_from_path(
                    &matches,
                    &mint_pubkey,
                    "keypair",
                    &mut None,
                ).map_err(|_| anyhow!("Invalid pubkey: {}", mint_pubkey))?;
                // let ix = set_authority(
                //
                // );
            },
            Subcommand::InitSplMint { mint_keypair, decimals } => {
                // TODO If the mint exists already, then just configure it, assuming
                //   signer is the current authority over the mint.
                let app = Opt::into_app();
                let matches = app.get_matches();
                let signer = self.keypair.resolve(&matches, None)?;
                let mint = keypair_from_path(
                    &matches,
                    &mint_keypair, "keypair",
                    false,
                ).map_err(|_| anyhow!("Invalid keypair path: {}", mint_keypair))?;
                let ixs = init_faucet_mint(mint.pubkey(), signer.pubkey(), decimals);
                let tx = Transaction::new_signed_with_payer(
                    &ixs,
                    Some(&signer.pubkey()),
                    &vec![signer, Box::new(mint)],
                    client.get_latest_blockhash()?
                );
                let signature = client.send_transaction(&tx)
                    .map_err(|e| {
                        println!("{:#?}", &e);
                        e
                    })?;
                println!("{}", signature);
            },
            Subcommand::InitFaucet { mint_pubkey, amount, faucet_keypair } => {
                let app = Opt::into_app();
                let matches = app.get_matches();
                let signer = self.keypair.resolve(&matches, None)?;
                let mint = pubkey_from_path(
                    &matches,
                    &mint_pubkey,
                    "keypair",
                    &mut None,
                ).map_err(|_| anyhow!("Invalid pubkey: {}", mint_pubkey))?;
                let faucet = if let Some(path) = faucet_keypair {
                    keypair_from_path(
                        &matches,
                        &path,
                        "faucet",
                        false,
                    ).map_err(|_| anyhow!("Invalid faucet path: {}", path))?
                } else {
                  Keypair::new()
                };
                println!("Faucet mint: {}", &mint);
                println!("Attempting to create faucet at address: {}", faucet.pubkey());
                let ix1 = init_faucet_account(faucet.pubkey(), signer.pubkey());
                let ix2 = init_faucet_instruction(
                    &faucet.pubkey(),
                    &mint,
                    amount,
                );
                let tx = Transaction::new_signed_with_payer(
                    &[ix1, ix2],
                    Some(&signer.pubkey()),
                    &vec![signer, Box::new(faucet)],
                    client.get_latest_blockhash()?
                );
                let signature = client.send_transaction(&tx)
                    .map_err(|e| {
                        println!("{:#?}", &e);
                        e
                    })?;
                println!("{}", signature);
            },
            Subcommand::InitAta { faucet } => {
                let app = Opt::into_app();
                let matches = app.get_matches();
                let signer = self.keypair.resolve(&matches, None)?;
                let faucet = pubkey_from_path(
                    &matches,
                    &faucet,
                    "keypair",
                    &mut None,
                ).map_err(|_| anyhow!("Invalid pubkey: {}", faucet))?;
                let faucet_data = client.get_account_data(&faucet)?;
                let faucet_data = Faucet::unpack(&faucet_data)?;
                let mint = faucet_data.mint;
                let ix = create_associated_token_account(
                    &signer.pubkey(),
                    &signer.pubkey(),
                    &mint,
                    &Token::id(),
                );
                let tx = Transaction::new_signed_with_payer(
                    &[ix],
                    Some(&signer.pubkey()),
                    &vec![signer],
                    client.get_latest_blockhash()?
                );
                let signature = client.send_transaction(&tx)
                    .map_err(|e| {
                        println!("{:#?}", &e);
                        e
                    })?;
                println!("{}", signature);
            }
            Subcommand::Mint { faucet, amount } => {
                let app = Opt::into_app();
                let matches = app.get_matches();
                let signer = self.keypair.resolve(&matches, None)?;
                let faucet = pubkey_from_path(
                    &matches,
                    &faucet,
                    "keypair",
                    &mut None,
                ).map_err(|_| anyhow!("Invalid pubkey: {}", faucet))?;
                let faucet_data = client.get_account_data(&faucet)?;
                let faucet_data = Faucet::unpack(&faucet_data)?;
                let mint = faucet_data.mint;
                let ata = get_associated_token_address(&signer.pubkey(), &mint);

                let ata_rent_balance = client.get_balance(&ata).unwrap_or(0);
                if ata_rent_balance == 0 {
                    println!("Associated token account found. \
                    Create one with the `init-ata` subcommand, then rerun this subcommand.");
                    let ix = create_associated_token_account(
                        &signer.pubkey(),
                        &signer.pubkey(),
                        &mint,
                        &Token::id(),
                    );
                    let tx = Transaction::new_signed_with_payer(
                        &[ix],
                        Some(&signer.pubkey()),
                        &vec![signer],
                        client.get_latest_blockhash()?
                    );
                    let signature = client.send_transaction(&tx)
                        .map_err(|e| {
                            println!("{:#?}", &e);
                            e
                        })?;
                    println!("{}", signature);
                    return Ok(());
                }

                let ix = mint_tokens_instruction(
                    &ata,
                    &mint,
                    &faucet,
                    amount,
                    &None,
                );
                let tx = Transaction::new_signed_with_payer(
                    &[ix],
                    Some(&signer.pubkey()),
                    &vec![signer],
                    client.get_latest_blockhash()?
                );
                let signature = client.send_transaction(&tx)
                    .map_err(|e| {
                        println!("{:#?}", &e);
                        e
                    })?;
                println!("{}", signature);
            },
            Subcommand::Show { faucet } => {
                let app = Opt::into_app();
                let matches = app.get_matches();
                let faucet = pubkey_from_path(
                    &matches,
                    &faucet,
                    "keypair",
                    &mut None,
                ).map_err(|_| anyhow!("Invalid pubkey: {}", faucet))?;
                let faucet_data = client.get_account_data(&faucet)?;
                let faucet_data = Faucet::unpack(&faucet_data)?;
                println!("{:#?}", faucet_data);
            },
        }
        Ok(())
    }
}

#[derive(Debug, Parser)]
enum Subcommand {
    /// Initialize a new SPL mint that's configured to
    /// the necessary mint authority to be a faucet mint.
    InitSplMint {
        /// Path to a signer (JSON private key, prompt://, etc)
        mint_keypair: String,
        /// The number of decimals for the new mint.
        decimals: u8,
    },
    /// Create a new faucet.
    InitFaucet {
        /// Takes a pubkey or path to a signer. Must be initialized,
        /// and be configured to the necessary mint authority to become a faucet.
        mint_pubkey: String,
        /// Maximum airdrop amount
        amount: u64,
        /// Optional path to a signer for the faucet address. If not provided,
        /// a random keypair will be generated.
        faucet_keypair: Option<String>,
    },
    /// Initialize an associated token account for whatever mint
    /// is associated with some faucet address.
    ///
    /// This is equivalent to `spl-token create-account <faucet-mint>`
    InitAta {
        /// The faucet address
        faucet: String,
    },
    /// Airdrop tokens from an SPL faucet
    Mint {
        /// The faucet address
        faucet: String,
        /// The amount to airdrop. Not a decimal value. It's in "lamports" of the SPL mint.
        amount: u64,
    },
    Show {
        faucet: String,
    }
    //Close,
}

fn main() -> Result<()> {
    let opt = Opt::parse();
    opt.process()?;
    Ok(())
}