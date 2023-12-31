//! Put these Clap arg structs (flattened) at the top level of a Clap CLI
//! made with the Derive API to add the `-u/--url`, `--commitment`, and
//! `-k/--keypair` CLI args as they behave in the Solana CLI.
use clap::{Parser, ValueEnum};
use solana_cli_config::Config;
use solana_devtools_signers::concrete_signer::ConcreteSigner;
use solana_sdk::commitment_config::CommitmentConfig;
use std::io;
use std::str::FromStr;

fn normalize_to_url_if_moniker<T: AsRef<str>>(url_or_moniker: T) -> String {
    match url_or_moniker.as_ref() {
        "m" | "mainnet-beta" => "https://api.mainnet-beta.solana.com",
        "t" | "testnet" => "https://api.testnet.solana.com",
        "d" | "devnet" => "https://api.devnet.solana.com",
        "l" | "localhost" => "http://localhost:8899",
        url => url,
    }
    .to_string()
}

/// Specify an RPC URL for RPC client requests.
#[derive(Debug, Parser)]
pub struct UrlArg {
    #[clap(short, long)]
    #[cfg_attr(feature = "env", clap(env = "SOLANA_RPC_URL"))]
    pub url: Option<String>,
}

impl UrlArg {
    /// Resolve to a `String` if specified in a [clap] `-u/--url` CLI argument,
    /// or from the `commitment` field in a [Config] struct.
    /// Looks at `~/.config/solana/cli/config.json` if `None` is provided.
    pub fn resolve(&self, config: Option<Config>) -> Result<String, io::Error> {
        if let Some(url) = self.url.clone() {
            let url = normalize_to_url_if_moniker(url);
            return Ok(url);
        }
        let config = config.unwrap_or(load_default_solana_cli_config()
            .map_err(|e| io::Error::new(
                io::ErrorKind::Other,
                format!("could not locate Solana CLI config file at its default location ~/.config/solana/cli/config.yaml: {}", e)))?);
        return Ok(normalize_to_url_if_moniker(config.json_rpc_url));
    }
}

#[derive(ValueEnum, Debug, Default, Clone)]
pub enum CommitmentLevel {
    Processed,
    #[default]
    Confirmed,
    Finalized,
}

impl Into<CommitmentConfig> for CommitmentLevel {
    fn into(self) -> CommitmentConfig {
        match self {
            CommitmentLevel::Processed => CommitmentConfig::processed(),
            CommitmentLevel::Confirmed => CommitmentConfig::confirmed(),
            CommitmentLevel::Finalized => CommitmentConfig::finalized(),
        }
    }
}

/// Specify a commitment level for RPC client requests.
#[derive(Debug, Parser)]
pub struct CommitmentArg {
    #[clap(short, long, value_enum)]
    #[cfg_attr(feature = "env", clap(env = "SOLANA_RPC_COMMITMENT"))]
    pub commitment: Option<CommitmentLevel>,
}

impl CommitmentArg {
    /// Resolve to a [CommitmentConfig] if specified in a [clap] `--commitment` CLI argument,
    /// or from the `commitment` field in a [Config] struct.
    /// Looks at `~/.config/solana/cli/config.json` if `None` is provided.
    pub fn resolve(self, config: Option<Config>) -> Result<CommitmentConfig, io::Error> {
        Into::<Option<CommitmentConfig>>::into(self).map_or_else (
            || {
                let config = config.unwrap_or(load_default_solana_cli_config()?);
                CommitmentConfig::from_str(&config.commitment)
                    .map_err(|e| io::Error::new(
                        io::ErrorKind::Other,
                        format!("could not locate Solana CLI config file at its default location ~/.config/solana/cli/config.yaml: {}", e)
                    ))
            },
            |commitment| Ok(commitment)
        )
    }
}

impl Default for CommitmentArg {
    fn default() -> Self {
        Self { commitment: Some(CommitmentLevel::Finalized) }
    }
}

impl Into<Option<CommitmentConfig>> for CommitmentArg {
    fn into(self) -> Option<CommitmentConfig> {
        self.commitment.map(|commitment| commitment.into())
    }
}

/// Specify a keypair according to `file://`, `usb://`, `stdin://`, `prompt://`, `presign://` URIs,
/// and an optional BIP-44 derivation path as the URI query param`?key={account}/{change}`.
/// URI parsing behavior is a super-set of the Solana CLI interface.
#[derive(Debug, Parser)]
pub struct KeypairArg {
    /// The target signer for transactions. See Solana CLI documentation on how to use this.
    /// Default values and usage patterns are identical to Solana CLI.
    #[clap(short, long)]
    #[cfg_attr(feature = "env", clap(env = "SOLANA_KEYPAIR_URI"))]
    pub keypair: Option<String>,
}

impl KeypairArg {
    /// Resolve to a [ConcreteSigner] from a URI path specified in a `-k/--keypair`,
    /// or from a URI path in the `keypair_path` field of a [Config] struct.
    /// Looks at `~/.config/solana/cli/config.json` if `None` is provided.
    pub fn resolve(self, config: Option<Config>) -> Result<ConcreteSigner, io::Error> {
        if let Some(keypair_path) = self.keypair {
            ConcreteSigner::from_str(&keypair_path).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::Other,
                    format!(
                        "could not interpret supplied keypair URI: {} error: {}",
                        keypair_path, e
                    ),
                )
            })
        } else {
            let config = config.unwrap_or(load_default_solana_cli_config()
                .map_err(|e| io::Error::new(io::ErrorKind::Other,
                    format!("could not locate Solana CLI config file at its default location ~/.config/solana/cli/config.yaml: {}", e)))?
            );
            ConcreteSigner::from_str(&config.keypair_path)
                .map_err(|e| io::Error::new(io::ErrorKind::Other,
                    format!("could not interpret keypair URI {} in ~/.config/solana/cli/config.yaml error: {}",
                            config.keypair_path, e)))
        }
    }
}

/// Load configuration from the standard Solana CLI config path.
/// For other filepaths, use [Config::load] directly.
pub fn load_default_solana_cli_config() -> Result<Config, io::Error> {
    let config_file = solana_cli_config::CONFIG_FILE
        .as_ref()
        .ok_or(io::Error::new(
            io::ErrorKind::Other,
            "unable to determine a config file path: no home directory on this OS or user",
        ))?;
    Config::load(&config_file)
}
