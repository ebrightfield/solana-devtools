use anyhow::{anyhow, Result};
use clap::{ArgMatches, Parser, ValueEnum};
use solana_clap_v3_utils::input_validators::normalize_to_url_if_moniker;
use solana_clap_v3_utils::keypair::signer_from_path;
use solana_cli_config::Config;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::Signer;
use std::str::FromStr;

/// Put this (flattened) at the top level of a Clap CLI made with the Derive API to add the
/// `-u/--url` CLI arg as it functions in the official Solana CLI.
/// This allows for manual specification of a cluster url,
/// or otherwise defaulting to the Solana CLI config file.
#[derive(Debug, Parser)]
pub struct UrlArg {
    /// The target URL for the cluster. See Solana CLI documentation on how to use this.
    /// Default values and usage patterns are identical to Solana CLI.
    #[clap(short, long)]
    pub url: Option<String>,
}

impl UrlArg {
    pub fn resolve(&self) -> Result<String> {
        if let Some(url) = self.url.clone() {
            let url = normalize_to_url_if_moniker(url);
            return Ok(url);
        }
        let config = get_solana_cli_config()?;
        return Ok(normalize_to_url_if_moniker(config.json_rpc_url));
    }

    pub fn resolve_with_config(&self, config: &Config) -> Result<String> {
        if let Some(url) = self.url.clone() {
            let url = normalize_to_url_if_moniker(url);
            return Ok(url);
        }
        return Ok(normalize_to_url_if_moniker(config.json_rpc_url.clone()));
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

#[derive(Debug, Parser)]
pub struct CommitmentArg {
    #[clap(short, long, value_enum)]
    pub commitment: Option<CommitmentLevel>,
}

impl CommitmentArg {
    pub fn resolve(&self) -> Result<CommitmentConfig> {
        if let Some(commitment) = self.commitment.clone() {
            return Ok(commitment.into());
        }
        let config = get_solana_cli_config()?;
        return Ok(CommitmentConfig::from_str(&config.commitment)?);
    }

    pub fn resolve_with_config(&self, config: &Config) -> Result<CommitmentConfig> {
        if let Some(commitment) = self.commitment.clone() {
            return Ok(commitment.into());
        }
        return Ok(CommitmentConfig::from_str(&config.commitment)?);
    }
}

/// Put this (flattened) at the top level of a Clap CLI made with the Derive API to add the
/// `-k/--keypair` CLI arg as it functions in the Solana CLI.
/// This allows for manual specification of a signing keypair,
/// or otherwise defaulting to the Solana CLI config file.
/// `--skip_phrase_validation` and `--confirm-key` are necessary because
/// signer resolution uses [solana_clap_v3_utils::keypair::signer_from_path].
#[derive(Debug, Parser)]
pub struct KeypairArg {
    /// The target signer for transactions. See Solana CLI documentation on how to use this.
    /// Default values and usage patterns are identical to Solana CLI.
    #[clap(short, long)]
    pub keypair: Option<String>,
    /// Skip BIP-39 seed phrase validation (not recommended)
    #[clap(long, name = "skip_seed_phrase_validation")]
    pub skip_seed_phrase_validation: bool,
    /// Manually confirm the signer before proceeding
    #[clap(long, name = "confirm_key")]
    pub confirm_key: bool,
}

impl KeypairArg {
    pub fn resolve(&self, matches: &ArgMatches) -> Result<Box<dyn Signer>> {
        if let Some(keypair_path) = self.keypair.clone() {
            return parse_signer(matches, keypair_path.as_str());
        }
        let config = get_solana_cli_config()?;
        parse_signer(matches, &config.keypair_path)
    }

    pub fn resolve_with_config(
        &self,
        matches: &ArgMatches,
        config: &Config,
    ) -> Result<Box<dyn Signer>> {
        if let Some(keypair_path) = self.keypair.clone() {
            return parse_signer(matches, keypair_path.as_str());
        }
        return parse_signer(matches, &config.keypair_path);
    }
}

/// Branch over the possible ways that signers can be specified via user input.
/// This basically does what `-k/--keypair` does, on a specific input string,
/// with disregard to filesystem configuration. It is useful for situations
/// where additional signers may be specified, e.g. grinding for an address and using
/// it as a signer when creating a multisig account.
fn parse_signer(matches: &ArgMatches, path: &str) -> Result<Box<dyn Signer>> {
    let mut wallet_manager = None;
    let signer = signer_from_path(matches, path, "keypair", &mut wallet_manager)
        .map_err(|e| anyhow!("Could not resolve signer: {:?}", e))?;
    Ok(signer)
}

/// Load configuration from the standard Solana CLI config path.
/// Those config values are used as defaults at runtime whenever
/// keypair and/or url are not explicitly passed in.
/// This can possibly fail if there is no Solana CLI installed, nor a config file
/// at the expected location.
pub fn get_solana_cli_config() -> Result<Config> {
    let config_file = solana_cli_config::CONFIG_FILE
        .as_ref()
        .ok_or_else(|| anyhow!("unable to determine a config file path on this OS or user"))?;
    Config::load(&config_file).map_err(|e| anyhow!("unable to load config file: {}", e.to_string()))
}
