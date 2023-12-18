use anchor_lang::Id;
use anchor_spl::associated_token::get_associated_token_address;
use anchor_spl::token::Token;
use anyhow::anyhow;
use clap::{ArgMatches, Parser};
use solana_clap_v3_utils::keypair::{keypair_from_path, pubkey_from_path};
use solana_client::rpc_client::RpcClient;
use solana_devtools_cli_config::config::KeypairArg;
use solana_devtools_faucet::{
    init_faucet_account, init_faucet_instruction, init_faucet_mint, mint_tokens_instruction,
    FAUCET_MINT_AUTH,
};
use solana_sdk::program_pack::Pack;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
use spl_associated_token_account::instruction::create_associated_token_account;
use spl_token::instruction::{set_authority, AuthorityType};
use spl_token_faucet::state::Faucet;

// TODO Close instruction
// TODO Admin option on creation and mint and close

#[derive(Debug, Parser)]
pub enum FaucetSubcommand {
    ConfigureSplMint {
        /// Path to a signer (JSON private key, prompt://, etc)
        mint_pubkey: String,
    },
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
    InitAtaForFaucet {
        /// The faucet address
        faucet: String,
    },
    /// Airdrop tokens from an SPL faucet
    Mint {
        #[clap(long)]
        init_ata: bool,
        /// The faucet address
        faucet: String,
        /// The amount to airdrop. Not a decimal value. It's in "lamports" of the SPL mint.
        amount: u64,
    },
    /// Display information about a particular SPL Faucet
    Show {
        /// The faucet address
        faucet: String,
    }, //Close,
}

impl FaucetSubcommand {
    pub fn process(
        self,
        client: &RpcClient,
        keypair: &KeypairArg,
        matches: &ArgMatches,
    ) -> anyhow::Result<()> {
        match self {
            FaucetSubcommand::ConfigureSplMint { mint_pubkey } => {
                let signer = keypair.resolve(&matches)?;
                let mint = pubkey_from_path(&matches, &mint_pubkey, "keypair", &mut None)
                    .map_err(|_| anyhow!("Invalid pubkey: {}", mint_pubkey))?;
                let ix = set_authority(
                    &Token::id(),
                    &mint,
                    Some(&FAUCET_MINT_AUTH),
                    AuthorityType::MintTokens,
                    &signer.pubkey(),
                    &[],
                )
                .unwrap();
                let tx = Transaction::new_signed_with_payer(
                    &[ix],
                    Some(&signer.pubkey()),
                    &vec![signer],
                    client.get_latest_blockhash()?,
                );
                let signature = client.send_transaction(&tx).map_err(|e| {
                    println!("{:#?}", &e);
                    e
                })?;
                println!("{}", signature);
            }
            FaucetSubcommand::InitSplMint {
                mint_keypair,
                decimals,
            } => {
                let signer = keypair.resolve(&matches)?;
                let mint = keypair_from_path(&matches, &mint_keypair, "keypair", false)
                    .map_err(|_| anyhow!("Invalid keypair path: {}", mint_keypair))?;
                let ixs = init_faucet_mint(mint.pubkey(), signer.pubkey(), decimals);
                let tx = Transaction::new_signed_with_payer(
                    &ixs,
                    Some(&signer.pubkey()),
                    &vec![signer, Box::new(mint)],
                    client.get_latest_blockhash()?,
                );
                let signature = client.send_transaction(&tx).map_err(|e| {
                    println!("{:#?}", &e);
                    e
                })?;
                println!("{}", signature);
            }
            FaucetSubcommand::InitFaucet {
                mint_pubkey,
                amount,
                faucet_keypair,
            } => {
                let signer = keypair.resolve(&matches)?;
                let mint = pubkey_from_path(&matches, &mint_pubkey, "keypair", &mut None)
                    .map_err(|_| anyhow!("Invalid pubkey: {}", mint_pubkey))?;
                let faucet = if let Some(path) = faucet_keypair {
                    keypair_from_path(&matches, &path, "faucet", false)
                        .map_err(|_| anyhow!("Invalid faucet path: {}", path))?
                } else {
                    Keypair::new()
                };
                println!("Faucet mint: {}", &mint);
                println!(
                    "Attempting to create faucet at address: {}",
                    faucet.pubkey()
                );
                let ix1 = init_faucet_account(faucet.pubkey(), signer.pubkey());
                let ix2 = init_faucet_instruction(&faucet.pubkey(), &mint, amount);
                let tx = Transaction::new_signed_with_payer(
                    &[ix1, ix2],
                    Some(&signer.pubkey()),
                    &vec![signer, Box::new(faucet)],
                    client.get_latest_blockhash()?,
                );
                let signature = client.send_transaction(&tx).map_err(|e| {
                    println!("{:#?}", &e);
                    e
                })?;
                println!("{}", signature);
            }
            FaucetSubcommand::InitAtaForFaucet { faucet } => {
                let signer = keypair.resolve(&matches)?;
                let faucet = pubkey_from_path(&matches, &faucet, "keypair", &mut None)
                    .map_err(|_| anyhow!("Invalid pubkey: {}", faucet))?;
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
                    client.get_latest_blockhash()?,
                );
                let signature = client.send_transaction(&tx).map_err(|e| {
                    println!("{:#?}", &e);
                    e
                })?;
                println!("{}", signature);
            }
            FaucetSubcommand::Mint {
                faucet,
                amount,
                init_ata,
            } => {
                let signer = keypair.resolve(&matches)?;
                let faucet = pubkey_from_path(&matches, &faucet, "keypair", &mut None)
                    .map_err(|_| anyhow!("Invalid pubkey: {}", faucet))?;
                let faucet_data = client.get_account_data(&faucet)?;
                let faucet_data = Faucet::unpack(&faucet_data)?;
                let mint = faucet_data.mint;
                let ata = get_associated_token_address(&signer.pubkey(), &mint);

                let mut ixs = vec![];
                if init_ata {
                    ixs.push(create_associated_token_account(
                        &signer.pubkey(),
                        &signer.pubkey(),
                        &mint,
                        &Token::id(),
                    ));
                }

                ixs.push(mint_tokens_instruction(&ata, &mint, &faucet, amount, &None));
                let tx = Transaction::new_signed_with_payer(
                    &ixs,
                    Some(&signer.pubkey()),
                    &vec![signer],
                    client.get_latest_blockhash()?,
                );
                let signature = client.send_transaction(&tx).map_err(|e| {
                    println!("{:#?}", &e);
                    e
                })?;
                println!("{}", signature);
            }
            FaucetSubcommand::Show { faucet } => {
                let faucet = pubkey_from_path(&matches, &faucet, "keypair", &mut None)
                    .map_err(|_| anyhow!("Invalid pubkey: {}", faucet))?;
                let faucet_data = client.get_account_data(&faucet)?;
                let faucet_data = Faucet::unpack(&faucet_data)?;
                println!("{:#?}", faucet_data);
            }
        }
        Ok(())
    }
}
