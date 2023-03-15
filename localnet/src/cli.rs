use anchor_cli::config::TestConfig;
use anyhow::anyhow;
use clap::Parser;
use crate::test_validator::localnet_from_test_config;
use crate::TestTomlGenerator;

#[derive(Debug, Parser)]
pub enum Subcommand {
    Build,
    FromTestConfig {
        #[clap(long)]
        skip_project_programs: bool,
        cfg: String,
        flags: Vec<String>,
    },
}

#[derive(Debug, Parser)]
pub struct SolanaLocalnetCli {
    #[clap(subcommand)]
    command: Option<Subcommand>,
}

impl SolanaLocalnetCli {
    pub fn process(self, test_toml_generators: Vec<TestTomlGenerator>) -> anyhow::Result<()> {
        if let Some(subcommand) = self.command {
            match subcommand {
                Subcommand::FromTestConfig { cfg, flags, skip_project_programs } => {
                    let test_config = TestConfig::discover(&cfg, vec![])?;
                    if let Some(test_config) = test_config {
                        localnet_from_test_config(test_config, flags, skip_project_programs)?;
                        return Ok(())
                    }
                    return Err(anyhow!(
                        "Could not find {}, you might need to build the localnet first.", &cfg));
                },
                Subcommand::Build => {
                    build_test_toml_files(test_toml_generators)?;
                }
            }
        } else {
            // Default to [Subcommand::Build],
            build_test_toml_files(test_toml_generators)?;
        }
        Ok(())
    }
}

pub fn build_test_toml_files(test_toml_generators: Vec<TestTomlGenerator>) -> anyhow::Result<()> {
    println!("Building Test.toml and associated files");
    test_toml_generators
        .iter()
        .for_each(|test_toml| {
            println!("Building: {}/Test.toml", test_toml.save_directory);
            test_toml.build().unwrap();
        });
    println!("Localnet configuration setup complete.");
    Ok(())
}

