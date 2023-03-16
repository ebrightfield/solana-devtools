# Solana DevTools

This is a suite of crates to assist with Rust development for Solana and Anchor.

## Crates

- `solana-devtools-cli-config` -- Structs and functions to make it easier to build CLIs
- `solana-devtools-faucet` -- An SDK for the `spl-token-faucet` program. See `solana-devtools-cli` for a CLI subcommand that uses this SDK.
that use the Solana CLI config files and interface for keypairs and RPC URLs.
- `solana-devtools-localnet` -- A library for creating and executing highly configured localnets.
- `solana-devtools-signers` -- Useful structs that `impl Signer`.
- `solana-devtools-rpc` -- RPC client utilities. Add headers to RPC requests, print transaction logs from simulation errors.
- `solana-devtools-cli` -- Interact with the faucet program, get the address of an ATA, and other useful dev/admin features.
- `serde-pubkey-str` -- (De-)serialize pubkeys to/from strings instead of byte-arrays.
- `solana-devtools-transaction` -- A library for constructing and processing transactions in various ways.
