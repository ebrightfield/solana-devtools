### Solana SPL Token Faucet CLI

An easy-to-use CLI for the SPL Token Faucet program, found here:

https://github.com/paul-schaaf/spl-token-faucet

- Create new faucets
- Airdrop tokens from faucets.
- Show human-readable faucet account data.
- Find a faucet by its mint using `get_program_accounts`.
- Create new SPL mints prepared for use in a new faucet.
- Specify RPC url and signer with the same file and argument interface as the official Solana CLI.


### Example Scripts
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
