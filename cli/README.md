## Solana Devtools CLI

### SPL Token Faucet CLI
The `faucet` subcommand provides a CLI for the SPL Token Faucet program,
the source code of which can be found here:

https://github.com/paul-schaaf/spl-token-faucet

With this subcommand, you can:

- Create new faucets
- Airdrop tokens from faucets.
- Show human-readable faucet account data.
- Find a faucet by its mint using `get_program_accounts`.
- Create new SPL mints prepared for use in a new faucet.


### Example Faucet Scripts
You can either run:
```
./example_scripts/full.sh
```

Or break the walkthrough into sections,
and executing the sections in order, and observing
what happens:

```
./example_scripts/01_init_local_faucet.sh
# ... you should sleep ...
./example_scripts/02_airdrop_local_faucet.sh
# ... you should sleep ...
./example_scripts/02_airdrop_local_faucet.sh
```
