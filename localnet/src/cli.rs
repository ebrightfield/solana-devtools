use crate::error::{LocalnetConfigurationError, Result};
use crate::LocalnetConfiguration;
use clap::Parser;

#[derive(Debug, Parser)]
pub enum Subcommand {
    /// Write the accounts to JSON files
    BuildJson {
        /// Directory where JSON files will be written.
        /// If no value is passed, defaults to the destination configured in code.
        #[clap(long)]
        output_dir: Option<String>,
        /// Overwrite existing JSON files.
        #[clap(long)]
        overwrite_existing: bool,
    },
    /// Start a `solana-test-validator` preloaded with accounts, programs,
    /// and additional flags.
    TestValidator {
        /// If specified, rebuild the account JSON files
        /// before starting the local validator.
        #[clap(long)]
        build_json: Option<String>,
        /// Overwrite existing JSON files. Has no effect if `build-json` arg is not provided.
        #[clap(long)]
        overwrite_existing: bool,
        /// Additional flags to pass to the test validator.
        flags: Vec<String>,
    },
    BuildJsImports {
        /// Filepath in which to write JS import statements.
        #[clap(long)]
        outfile: String,
    },
}

#[derive(Debug, Parser)]
pub struct SolanaLocalnetCli {
    #[clap(subcommand)]
    command: Subcommand,
}

impl SolanaLocalnetCli {
    pub fn with_config(cfg: LocalnetConfiguration) -> Result<()> {
        match Self::parse().command {
            Subcommand::BuildJson {
                output_dir,
                overwrite_existing,
            } => {
                build_json_and_check_programs(&cfg, output_dir.as_deref(), overwrite_existing)?;
            }
            Subcommand::TestValidator {
                build_json,
                overwrite_existing,
                flags,
            } => {
                let missing_programs = cfg.missing_programs();
                if !missing_programs.is_empty() {
                    for path in cfg.missing_programs() {
                        eprintln!("WARNING: Could not find program binary at path: {}", path);
                    }
                    return Err(LocalnetConfigurationError::MissingProgramSoFile(
                        missing_programs.first().unwrap().to_string(),
                    ));
                }
                let child_process = if let Some(output_dir) = build_json {
                    build_json_and_check_programs(&cfg, Some(&output_dir), overwrite_existing)?;
                    cfg.start_test_validator(flags, Some(&output_dir))
                } else {
                    cfg.start_test_validator(flags, None)
                }
                .expect("failed to spawn test validator");
                let output = child_process
                    .wait_with_output()
                    .expect("Test validator failed unexpectedly");
                if output.status.success() {
                    println!("test validator exited successfully");
                } else {
                    let code = output.status.code();
                    eprintln!("test validator exited with error code: {:?}", code);
                }
            }
            Subcommand::BuildJsImports { outfile } => {
                cfg.write_js_import_file(outfile)?;
            }
        }
        Ok(())
    }
}

fn build_json_and_check_programs(
    cfg: &LocalnetConfiguration,
    output_dir: Option<&str>,
    overwrite_existing: bool,
) -> Result<()> {
    cfg.write_accounts_json(output_dir, overwrite_existing)?;
    for path in cfg.missing_programs() {
        eprintln!("WARNING: Could not find program binary at path: {}", path);
    }
    Ok(())
}
