use crate::error::Result;
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
        /// Optionally provide a directory path at which to build the JSON values.
        #[clap(long)]
        build_json: Option<Option<String>>,
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
                let json_outdir = output_dir.as_deref();
                cfg.write_accounts_json(json_outdir, overwrite_existing)?;
            }
            Subcommand::TestValidator {
                build_json,
                overwrite_existing,
                flags,
            } => {
                let child_process = if let Some(json_outdir) = build_json {
                    let json_outdir = json_outdir.as_deref();
                    cfg.write_accounts_json(json_outdir, overwrite_existing)?;
                    cfg
                        .start_test_validator(flags, json_outdir)
                        .expect("failed to spawn test validator")
                } else {
                    cfg
                        .start_test_validator(flags, None)
                        .expect("failed to spawn test validator")
                };


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
