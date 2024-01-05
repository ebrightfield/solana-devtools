use crate::error::{LocalnetConfigurationError, Result};
use crate::localnet_account::{LocalnetAccount, UiAccountWithAddr};
#[cfg(feature = "solana-devtools-simulator")]
pub use crate::TransactionSimulator;
use solana_program_test::ProgramTest;
use solana_sdk::{
    account::AccountSharedData, bpf_loader_upgradeable,
    bpf_loader_upgradeable::UpgradeableLoaderState, pubkey::Pubkey,
};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::{
    fs::{self, read_dir, File},
    path::Path,
    process::{Child, Stdio},
};

/// Beginning of JS file, to construct `anchor.web3.PublicKey` instances.
const JS_ANCHOR_IMPORT: &str = "import * as anchor from \"@project-serum/anchor\";\n";

/// Defines a configuration of a set of accounts, programs, etc.
/// Can be used to generate a [ProgramTest], a [TransactionSimulator],
/// and a CLI binary that indirectly calls `solana-test-validator`
/// with the accounts built as JSON and passed as args.
#[derive(Debug, Clone, Default)]
pub struct LocalnetConfiguration {
    /// Any accounts to pre-load to the test validator.
    pub accounts: HashMap<Pubkey, LocalnetAccount>,
    /// Used to enforce no duplicate account names.
    account_names: HashSet<String>,
    /// Paths to programs are retained only for use with a test validator.
    pub programs: HashMap<Pubkey, String>,
    /// BPF Upgradeable program data pubkeys. Pubkeys in this and `self.programs`
    /// are filtered out.
    program_data_accounts: HashSet<Pubkey>,
    /// CLI args to `solana-test-validator`. The key should not contain dashes.
    /// e.g. "rpc_port", "8899".
    pub test_validator_args: HashMap<String, String>,
    /// CLI flags to `solana-test-validator`. The flags should not contain dashes.
    /// e.g. "reset".
    pub test_validator_flags: Vec<String>,
    /// Output directory to write JSON files before starting a `solana-test-validator`.
    pub json_outdir: Option<String>,
}

impl LocalnetConfiguration {
    pub fn new() -> Self {
        Self::default()
    }

    /// Primarily useful for cases where you're going to use a [LocalnetConfiguration]
    /// to instantiate a `solana-test-validator`.
    pub fn with_outdir(json_outdir: &str) -> Self {
        Self {
            json_outdir: Some(json_outdir.to_string()),
            ..Default::default()
        }
    }

    /// Load JSON files into a [LocalnetConfiguration].
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
                                    LocalnetAccount::from_ui_account(ui_account, name)?,
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

    /// Add raw binary program data as a BPF upgradeable program. For programs that are not
    /// going to change, like dependency programs your program relies on, this is the preferred
    /// way to add programs, because you can use `include_bytes!` and place your binaries
    /// in relation to source code in a way that is easier to configure than with relative paths
    /// that might change depending on where you execute your tests.
    pub fn program_binary_data(
        mut self,
        program_binary_name: &str,
        program_id: Pubkey,
        program_data: &[u8],
    ) -> Result<Self> {
        let programdata_address = Pubkey::new_unique();
        let program = LocalnetAccount {
            address: program_id,
            name: program_binary_name.to_string(),
            lamports: 1,
            data: bincode::serialize(&UpgradeableLoaderState::Program {
                programdata_address,
            })
            .unwrap(),
            owner: bpf_loader_upgradeable::ID,
            executable: true,
            rent_epoch: 0,
        }
        .into();

        let mut data = bincode::serialize(&UpgradeableLoaderState::ProgramData {
            slot: 0,
            upgrade_authority_address: None,
        })
        .unwrap();
        data.resize(UpgradeableLoaderState::size_of_programdata_metadata(), 0);
        data.extend_from_slice(program_data);
        let program_data = LocalnetAccount {
            address: programdata_address,
            name: format!("{}_programdata", program_binary_name),
            lamports: 1,
            data,
            owner: bpf_loader_upgradeable::ID,
            executable: true,
            rent_epoch: 0,
        }
        .into();
        self.program_data_accounts.insert(programdata_address);
        let this = self.accounts([program, program_data])?;
        Ok(this)
    }

    /// If the provided program binary file path is a relative path, it is interpreted
    /// from the root of the crate when executed through `cargo`.
    /// The program is added as a BPF upgradeable program in `self.accounts`.
    /// The filepath is only retained for the case where a `solana-test-validator` will be created.
    pub fn program_binary_file(
        mut self,
        program_id: Pubkey,
        program_binary_file: &str,
    ) -> Result<Self> {
        let path = {
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
        let mut file = File::open(&path).map_err(|e| {
            eprintln!("Working directory: {:?}", std::env::current_dir());
            LocalnetConfigurationError::FileReadWriteError(path.clone(), e)
        })?;
        let name = Path::new(&path)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        let mut data = vec![];
        let _ = file
            .read_to_end(&mut data)
            .map_err(|e| LocalnetConfigurationError::FileReadWriteError(path.clone(), e))?;
        self.programs.insert(program_id, path);
        self.program_binary_data(&name, program_id, &data)
    }

    /// Add a `solana-test-validator` CLI argument to include on every startup.
    pub fn add_test_validator_arg(&mut self, key: String, value: String) {
        self.test_validator_args.insert(key, value);
    }

    /// Add a `solana-test-validator` CLI flag to include on every startup.
    pub fn add_test_validator_flag(&mut self, flag: String) {
        self.test_validator_flags.push(flag);
    }

    pub fn pubkey_is_program(&self, pubkey: &Pubkey) -> bool {
        self.programs.contains_key(pubkey) || self.program_data_accounts.contains(pubkey)
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
        for (pubkey, act) in &self.accounts {
            if !self.pubkey_is_program(pubkey) {
                act.write_to_validator_json_file(&path_prefix, overwrite)?;
            }
        }
        Ok(())
    }

    /// Create a file that allows for easy import of the files in this test suite.
    pub fn write_js_import_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let mut script = vec![JS_ANCHOR_IMPORT.to_string()];
        script.extend(
            self.accounts
                .iter()
                .filter(|(pubkey, _)| !self.pubkey_is_program(pubkey))
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
        let mut args: Vec<String> = {
            // dedupe
            let set: HashSet<_> = self.test_validator_flags.clone().into_iter().collect();
            set.into_iter().collect()
        };
        for (k, v) in &self.test_validator_args {
            args.push(k.clone());
            args.push(v.clone());
        }
        args.extend(additional_args);
        for (pubkey, account) in &self.accounts {
            if !self.pubkey_is_program(pubkey) {
                args.push("--account".to_string());
                args.push(pubkey.to_string());
                args.push(account.json_output_path(&path_prefix));
            }
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

    /// Also reads BPF program binaries into the accounts.
    pub fn dump_accounts(&self) -> HashMap<Pubkey, AccountSharedData> {
        HashMap::from_iter(self.accounts.iter().map(|(p, act)| (*p, act.into())))
    }
}

#[cfg(feature = "solana-devtools-simulator")]
impl Into<TransactionSimulator> for &LocalnetConfiguration {
    fn into(self) -> TransactionSimulator {
        TransactionSimulator::new_with_accounts(&self.accounts)
    }
}

impl Into<ProgramTest> for &LocalnetConfiguration {
    fn into(self) -> ProgramTest {
        let mut program_test = ProgramTest::default();

        for (pubkey, act) in &self.accounts {
            program_test.add_account(*pubkey, act.into());
        }

        program_test
    }
}
