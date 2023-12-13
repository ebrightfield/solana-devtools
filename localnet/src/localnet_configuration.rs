use crate::error::{LocalnetConfigurationError, Result};
use crate::localnet_account::{LocalnetAccount, UiAccountWithAddr};
use solana_program::pubkey::Pubkey;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::fs::{read_dir, File};
use std::hash::Hash;
use std::path::Path;
use std::process::{Child, Stdio};

/// Beginning of JS file, to construct `anchor.web3.PublicKey` instances.
const JS_ANCHOR_IMPORT: &str = "import * as anchor from \"@project-serum/anchor\";\n";

/// Generates a `Test.toml` that sets up a localnet for testing, and provides
/// other convenient setup automation for complicated state saturation.
#[derive(Debug, Clone, Default)]
pub struct LocalnetConfiguration {
    /// Any accounts to pre-load to the test validator.
    pub accounts: HashMap<Pubkey, LocalnetAccount>,
    account_names: HashSet<String>,
    /// Any programs to pre-load to the test validator.
    pub programs: HashMap<Pubkey, String>,
    /// CLI args to `solana-test-validator`. The key should not contain dashes.
    /// e.g. "rpc_port", "8899".
    pub test_validator_args: HashMap<String, String>,
    /// CLI flags to `solana-test-validator`. The flags should not contain dashes.
    /// e.g. "reset".
    pub test_validator_flags: Vec<String>,
    pub json_outdir: Option<String>,
}

impl LocalnetConfiguration {
    pub fn new(
        accounts: Vec<LocalnetAccount>,
        programs: HashMap<Pubkey, String>,
        json_outdir: Option<String>,
    ) -> Result<Self> {
        let account_names = accounts
            .iter()
            .map(|act| act.name.clone())
            .collect::<Vec<_>>();
        let duplicate_names = retain_duplicates::<String>(&account_names);
        if !duplicate_names.is_empty() {
            return Err(LocalnetConfigurationError::DuplicateAccountName(
                duplicate_names,
            ));
        }
        let pubkeys = accounts.iter().map(|p| p.address).collect::<Vec<_>>();
        let duplicate_pubkeys = retain_duplicates::<Pubkey>(&pubkeys);
        if !duplicate_pubkeys.is_empty() {
            let dup = duplicate_pubkeys.iter().map(|p| p.to_string()).collect();
            return Err(LocalnetConfigurationError::DuplicateAccountPubkey(dup));
        }
        Ok(Self {
            accounts: HashMap::from_iter(accounts.into_iter().map(|act| (act.address, act))),
            account_names: HashSet::from_iter(account_names),
            programs,
            test_validator_args: Default::default(),
            test_validator_flags: Default::default(),
            json_outdir,
        })
    }

    pub fn from_dir<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let mut accounts = HashMap::new();
        let mut duplicate_pubkeys: Vec<UiAccountWithAddr> = vec![];
        for path in read_dir(&dir).map_err(|e| {
            let path = dir.as_ref().to_str().unwrap().to_string();
            LocalnetConfigurationError::FileReadWriteError(path, e)
        })? {
            if let Ok(p) = path {
                let path = p.path();
                let path_str = path.display().to_string();
                let stripped = path_str.strip_suffix(".json");
                if path.is_file() && stripped.is_some() {
                    let file = File::open(path).map_err(|e| {
                        LocalnetConfigurationError::FileReadWriteError(path_str.to_string(), e)
                    })?;
                    let ui_account = serde_json::from_reader::<_, UiAccountWithAddr>(file);
                    match ui_account {
                        Ok(ui_account) => {
                            let name = stripped.unwrap().to_string();
                            if accounts.contains_key(&ui_account.pubkey) {
                                duplicate_pubkeys.push(ui_account);
                            } else {
                                accounts.insert(
                                    ui_account.pubkey,
                                    ui_account.to_localnet_account(name)?,
                                );
                            }
                        }
                        Err(e) => {
                            return Err(LocalnetConfigurationError::InvalidAccountJson(e));
                        }
                    }
                }
            }
        }
        if !duplicate_pubkeys.is_empty() {
            let dup = duplicate_pubkeys
                .iter()
                .map(|p| p.pubkey.to_string())
                .collect();
            return Err(LocalnetConfigurationError::DuplicateAccountPubkey(dup));
        }
        Ok(Self {
            accounts,
            json_outdir: Some(dir.as_ref().to_str().unwrap().to_string()),
            ..Default::default()
        })
    }

    pub fn add_account(&mut self, act: LocalnetAccount) -> Result<()> {
        if self.accounts.contains_key(&act.address) {
            Err(LocalnetConfigurationError::DuplicateAccountPubkey(vec![
                act.address.to_string(),
            ]))
        } else if !self.account_names.insert(act.name.clone()) {
            Err(LocalnetConfigurationError::DuplicateAccountName(vec![
                act.name,
            ]))
        } else {
            self.accounts.insert(act.address, act);
            Ok(())
        }
    }

    pub fn add_accounts(&mut self, acts: Vec<LocalnetAccount>) -> Result<()> {
        for act in acts {
            self.add_account(act)?;
        }
        Ok(())
    }

    pub fn add_program(&mut self, program_id: Pubkey, program_binary_file: String) -> Result<()> {
        if self.programs.contains_key(&program_id) {
            return Err(LocalnetConfigurationError::DuplicateProgramPubkey(vec![
                program_id.to_string(),
            ]));
        }
        self.programs.insert(program_id, program_binary_file);
        Ok(())
    }

    pub fn add_test_validator_arg(&mut self, key: String, value: String) {
        self.test_validator_args.insert(key, value);
    }

    pub fn add_test_validator_flag(&mut self, flag: String) {
        self.test_validator_flags.push(flag);
    }

    /// Write configured accounts out to JSON files in the same format
    /// as the Solana CLI `account` subcommand when using the `--output-format json` arg.
    /// Also the same as the `getAccountInfo` RPC endpoint:
    /// https://docs.solana.com/api/http#getaccountinfo
    pub fn write_accounts_json(&self, path_prefix: Option<&str>, overwrite: bool) -> Result<()> {
        let path_prefix = if let Some(dir) = path_prefix {
            dir
        } else {
            if let Some(ref dir) = self.json_outdir {
                dir
            } else {
                return Err(LocalnetConfigurationError::NoOutputDirectory);
            }
        };
        for (_, act) in &self.accounts {
            act.write_to_validator_json_file(&path_prefix, overwrite)?;
        }
        Ok(())
    }

    pub fn missing_programs(&self) -> Vec<&str> {
        self.programs
            .iter()
            .filter_map(|(_, path)| {
                if File::open(path).is_err() {
                    Some(path.as_str())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Create a file that allows for easy import of the files in this test suite.
    pub fn write_js_import_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let mut script = vec![JS_ANCHOR_IMPORT.to_string()];
        script.extend(
            self.accounts
                .iter()
                .map(|(_, act)| act.js_import())
                .collect::<Vec<String>>(),
        );
        let script: String = script.join("\n");
        fs::write(path.as_ref(), script).map_err(|e| {
            let path = path.as_ref().to_str().unwrap().to_string();
            LocalnetConfigurationError::FileReadWriteError(path, e)
        })
    }

    pub fn start_test_validator(
        &self,
        additional_args: Vec<String>,
        json_outdir: Option<&str>,
    ) -> std::io::Result<Child> {
        let path_prefix = self
            .json_outdir
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or_else(|| {
                json_outdir.expect("no json_outdir specified, cannot load localnet accounts")
            });
        let mut args = dedup_vec(self.test_validator_flags.clone());
        for (k, v) in &self.test_validator_args {
            args.push(k.clone());
            args.push(v.clone());
        }
        args.extend(additional_args);
        for (pubkey, account) in &self.accounts {
            args.push("--account".to_string());
            args.push(pubkey.to_string());
            args.push(account.json_output_path(&path_prefix));
        }
        for (pubkey, path) in &self.programs {
            args.push("--bpf-program".to_string());
            args.push(pubkey.to_string());
            args.push(path.to_string());
        }
        std::process::Command::new("solana-test-validator")
            .args(args)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
    }
}

fn dedup_vec(mut vec: Vec<String>) -> Vec<String> {
    let mut set = HashSet::new();
    vec.retain(|e| set.insert(e.clone()));
    vec
}

fn retain_duplicates<T: Clone + PartialEq + Eq + Hash>(items: &Vec<T>) -> Vec<T> {
    let mut counts = HashMap::new();

    // Count the occurrences of each element
    for item in items.iter() {
        *counts.entry(item.clone()).or_insert(0u32) += 1;
    }

    // Retain only elements that have more than one occurrence
    let mut items = items.clone();
    items.retain(|item| counts.get(item).map_or(false, |&count| count > 1));
    items
}
