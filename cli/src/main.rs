mod faucet;

use anchor_spl::associated_token::get_associated_token_address;
use anyhow::{anyhow, Result};
use clap::{IntoApp, Parser};
use solana_clap_v3_utils::keypair::pubkey_from_path;
use solana_client::rpc_client::RpcClient;
use solana_devtools_cli_config::{KeypairArg, UrlArg};
use crate::faucet::FaucetSubcommand;

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
                let client = RpcClient::new(url);
                subcommand.process(&client, &self.keypair, &matches)?;
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
}

fn main() -> Result<()> {
    let opt = Opt::parse();
    opt.process()?;
    Ok(())
}
