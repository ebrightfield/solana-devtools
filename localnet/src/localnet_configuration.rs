use crate::error::{LocalnetConfigurationError, Result};
use crate::localnet_account::{LocalnetAccount, UiAccountWithAddr};
#[cfg(feature = "mock-runtime")]
use solana_mock_runtime::MockSolanaRuntime;
use solana_program::pubkey::Pubkey;
#[cfg(feature = "mock-runtime")]
use solana_sdk::{
    account::{Account, AccountSharedData},
    bpf_loader_upgradeable::{self, UpgradeableLoaderState},
};
use std::collections::{HashMap, HashSet};
#[cfg(feature = "mock-runtime")]
use std::io::Read;
use std::{
    fs::{self, read_dir, File},
    path::Path,
    process::{Child, Stdio},
};

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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_outdir(json_outdir: &str) -> Self {
        Self {
            json_outdir: Some(json_outdir.to_string()),
            ..Default::default()
        }
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

    /// Add several accounts to the configuration
    pub fn accounts(mut self, acts: impl IntoIterator<Item = LocalnetAccount>) -> Result<Self> {
        for act in acts {
            if self.accounts.contains_key(&act.address) {
                return Err(LocalnetConfigurationError::DuplicateAccountPubkey(vec![
                    act.address.to_string(),
                ]));
            } else if !self.account_names.insert(act.name.clone()) {
                return Err(LocalnetConfigurationError::DuplicateAccountName(vec![
                    act.name,
                ]));
            } else {
                self.accounts.insert(act.address, act);
            }
        }
        Ok(self)
    }

    /// If the provided program binary file path is a relative path, it is interpreted
    /// from the root of the crate being invoked with `cargo`.
    pub fn program(mut self, program_id: Pubkey, program_binary_file: &str) -> Result<Self> {
        let program_binary_file = {
            let path = Path::new(program_binary_file);
            if path.is_relative() {
                std::env::var("CARGO_MANIFEST_DIR")
                    .map(|s| s + "/")
                    .unwrap_or(String::new())
                    + program_binary_file
            } else {
                program_binary_file.to_string()
            }
        };

        if self.programs.contains_key(&program_id) {
            return Err(LocalnetConfigurationError::DuplicateProgramPubkey(
                program_id.to_string(),
            ));
        }
        if File::open(&program_binary_file).is_err() {
            eprintln!("Working directory: {:?}", std::env::current_dir());
            return Err(LocalnetConfigurationError::MissingProgramSoFile(
                program_binary_file,
            ));
        }
        self.programs.insert(program_id, program_binary_file);
        Ok(self)
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
    pub fn write_accounts_json(&self, outdir: Option<&str>, overwrite: bool) -> Result<()> {
        let path_prefix = if let Some(dir) = outdir {
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

    pub fn get_account(&self, pubkey: &Pubkey) -> Option<&LocalnetAccount> {
        self.accounts.get(pubkey)
    }

    pub fn get_program(&self, pubkey: &Pubkey) -> Option<&str> {
        self.programs.get(pubkey).map(|path| path.as_str())
    }
}

fn dedup_vec(mut vec: Vec<String>) -> Vec<String> {
    let mut set = HashSet::new();
    vec.retain(|e| set.insert(e.clone()));
    vec
}
#[cfg(feature = "mock-runtime")]
impl TryInto<MockSolanaRuntime> for &LocalnetConfiguration {
    type Error = LocalnetConfigurationError;

    fn try_into(self) -> Result<MockSolanaRuntime> {
        let mut mock_runtime = MockSolanaRuntime::new_with_spl_and_builtins()
            .map_err(|e| LocalnetConfigurationError::EbpfError(e.to_string()))?;

        //let accounts = accounts_from_localnet_configuration(self)?;
        let accounts = HashMap::from_iter(
            self.accounts
                .iter()
                .map(|(pubkey, act)| (*pubkey, act.into())),
        );
        mock_runtime.update_accounts(&accounts);
        for (program_id, path) in &self.programs {
            let mut data = vec![];
            let _ = File::open(path)
                .map(|mut f| f.read_to_end(&mut data))
                .map_err(|e| LocalnetConfigurationError::FileReadWriteError(path.clone(), e))?;
            mock_runtime.add_program_from_bytes(*program_id, &data);
        }
        Ok(mock_runtime)
    }
}
