
use solana_devtools_localnet::cli::SolanaLocalnetCli;
use test_localnet::suite_two::suite_2;

fn main() -> anyhow::Result<()> {
    // Alternatively, you can convert this binary to a CLI
    // with a command to build, or execute a single test suite's localnet.
    SolanaLocalnetCli::with_config(suite_2())?;
    Ok(())
}
