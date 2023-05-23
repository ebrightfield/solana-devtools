mod suite_one;
mod suite_two;

use clap::Parser;
use solana_devtools_localnet::cli::SolanaLocalnetCli;
use crate::suite_one::suite_1;
use crate::suite_two::suite_2;

fn main() -> anyhow::Result<()> {

    let toml1 = suite_1();
    let toml2 = suite_2();

    // You could just build directly and be done with it.
    // toml1.build()?;
    // toml2.build()?;

    // Alternatively, you can convert this binary to a CLI
    // with a command to build, or execute a single test suite's localnet.
    let opts = SolanaLocalnetCli::parse();
    opts.process(vec![
        toml1,
        toml2,
    ])?;
    Ok(())
}
