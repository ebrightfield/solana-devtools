use std::collections::HashSet;

use std::fs;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use anchor_cli::config::{Config, ConfigOverride, TestConfig, TestValidator, WithPath};
use solana_client::rpc_client::RpcClient;
use anyhow::{anyhow, Result};
use solana_program::bpf_loader_upgradeable;
use solana_program::bpf_loader_upgradeable::UpgradeableLoaderState;
use solana_program::pubkey::Pubkey;
use solana_sdk::commitment_config::CommitmentConfig;
use crate::from_anchor::cli::{start_test_validator, stream_logs, test_validator_rpc_url};


/// Returns the solana-test-validator flags. This will embed the workspace
/// programs in the genesis block.
/// It also provides control of other solana-test-validator features.
///
/// This function is the same as the one in the CLI crate,
/// but it handles the IDL accounts differently.
/// It could be DRYed, but it's not straightforward
fn validator_flags(
    test_validator: &Option<TestValidator>,
) -> Result<Vec<String>> {

    // On-chain IDL accounts are written here.
    if !PathBuf::from("target/idl-account").exists() {
        fs::create_dir("target/idl-account")?;
    }

    let mut flags = Vec::new();

    if let Some(test) = test_validator.as_ref() {
        if let Some(genesis) = &test.genesis {
            for entry in genesis {
                let program_path = Path::new(&entry.program);
                if !program_path.exists() {
                    return Err(anyhow!(
                        "Program in genesis configuration does not exist at path: {}",
                        program_path.display()
                    ));
                }
                flags.push("--bpf-program".to_string());
                flags.push(entry.address.clone());
                flags.push(entry.program.clone());
            }
        }
        if let Some(validator) = &test.validator {
            let entries = serde_json::to_value(validator)?;
            for (key, value) in entries.as_object().unwrap() {
                if key == "ledger" {
                    // Ledger flag is a special case as it is passed separately to the rest of
                    // these validator flags.
                    continue;
                };
                if key == "account" {
                    for entry in value.as_array().unwrap() {
                        // Push the account flag for each array entry
                        flags.push("--account".to_string());
                        flags.push(entry["address"].as_str().unwrap().to_string());
                        flags.push(entry["filename"].as_str().unwrap().to_string());
                    }
                } else if key == "clone" {
                    // Client for fetching accounts data
                    let client = if let Some(url) = entries["url"].as_str() {
                        RpcClient::new(url.to_string())
                    } else {
                        return Err(anyhow!(
                    "Validator url for Solana's JSON RPC should be provided in order to clone accounts from   it"
                ));
                    };

                    let mut pubkeys = value
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|entry| {
                            let address = entry["address"].as_str().unwrap();
                            Pubkey::from_str(address)
                                .map_err(|_| anyhow!("Invalid pubkey {}", address))
                        })
                        .collect::<anyhow::Result<HashSet<Pubkey>>>()?;

                    let accounts_keys = pubkeys.iter().cloned().collect::<Vec<_>>();
                    let accounts = client
                        .get_multiple_accounts_with_commitment(
                            &accounts_keys,
                            CommitmentConfig::default(),
                        )?
                        .value;

                    // Check if there are program accounts
                    for (account, acc_key) in accounts.iter().zip(accounts_keys) {
                        if let Some(account) = account {
                            if account.owner == bpf_loader_upgradeable::id() {
                                let upgradable: UpgradeableLoaderState = account
                                    .deserialize_data()
                                    .map_err(|_| anyhow!("Invalid program account {}", acc_key))?;

                                if let UpgradeableLoaderState::Program {
                                    programdata_address,
                                } = upgradable
                                {
                                    pubkeys.insert(programdata_address);
                                }
                            }
                        } else {
                            return Err(anyhow!("Account {} not found", acc_key));
                        }
                    }

                    for pubkey in &pubkeys {
                        // Push the clone flag for each array entry
                        flags.push("--clone".to_string());
                        flags.push(pubkey.to_string());
                    }
                } else {
                    // Remaining validator flags are non-array types
                    flags.push(format!("--{}", key.replace('_', "-")));
                    if let serde_json::Value::String(v) = value {
                        flags.push(v.to_string());
                    } else {
                        flags.push(value.to_string());
                    }
                }
            }
        }
    }
    Ok(flags)
}


pub fn localnet_from_test_config(test_config: TestConfig, flags: Vec<String>) -> Result<()> {
    for (_, test_toml) in &*test_config {
        // Copy the test suite into the Anchor [Config].
        // Set the startup_wait to zero, since it's irrelevant when we aren't running tests.
        let mut anchor_cfg = Config::discover(
            &ConfigOverride::default(),
        )?.unwrap();
        let mut test_validator = test_toml.test.clone();
        if let Some(inner) = test_validator {
            let mut with_no_wait = inner.clone();
            with_no_wait.startup_wait = 0;
            test_validator = Some(with_no_wait);
        } else {
            let mut with_no_wait = TestValidator::default();
            with_no_wait.startup_wait = 0;
            test_validator = Some(with_no_wait);
        }
        anchor_cfg.test_validator = test_validator;
        let with_path = &WithPath::new(
            anchor_cfg, PathBuf::from("./Anchor.toml"));
        // Gather the CLI flags
        let mut cfg_flags = validator_flags(&test_toml.test)?;
        cfg_flags.extend(flags);
        // Start the validator
        let mut validator_handle = start_test_validator(
            &with_path,
            &test_toml.test,
            Some(cfg_flags),
            false,
        )?;

        let url = test_validator_rpc_url(&test_toml.test);
        let log_streams = stream_logs(
            &with_path,
            &url,
        );

        std::io::stdin().lock().lines().next().unwrap().unwrap();

        // Check all errors and shut down.
        if let Err(err) = validator_handle.kill() {
            println!(
                "Failed to kill subprocess {}: {}",
                validator_handle.id(),
                err
            );
        }

        for mut child in log_streams? {
            if let Err(err) = child.kill() {
                println!("Failed to kill subprocess {}: {}", child.id(), err);
            }
        }
        return Ok(())
    }
    Ok(())
}