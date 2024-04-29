# Solana DevTools

This is a suite of crates to assist with Rust development for Solana and Anchor.

## Crates

- `solana-devtools-anchor-utils` -- Dynamic deserialization Anchor instructions and accounts using IDLs, and other QoL tooling for Anchor.
- `solana-devtools-cli-config` -- Structs and functions to make it easier to build Solana CLIs with Clap, implementing a super-set of the Solana CLI config behavior.
- `solana-devtools-cli` -- A CLI binary with useful dev/admin features that don't exist on the vanilla Solana and Anchor CLI tools.
- `solana-devtools-errors` -- Extract or map error codes from highly nested enum types returned from RPC clients, etc.
- `solana-devtools-localnet` -- (DEPRECATED) see `solana-devtools-anchor-utils` and `solana-devtools-simulator` instead.
- `solana-devtools-macros` -- Macros for named fake pubkeys, and for constants which associate metadata with addresses.
- `solana-devtools-simulator` -- Simulate the BPF execution of transactions locally with arbitrary account data and pubkeys, without the need to sign. You can choose whether or not to persist account data mutations across simulations.
- `solana-devnet-monitoring` -- Functions for tracking events, similar to Anchor's log subscribe approach, but with a trait based interface.
- `solana-devtools-signers` -- Useful structs that `impl Signer`.
- `solana-devtools-rpc` -- RPC client utilities. Add headers to RPC requests, print transaction logs from simulation errors.
- `solana-devtools-serde` -- (De-)serialize pubkeys and signatures to/from strings instead of byte-arrays.
- `solana-devtools-tx` -- A library for constructing and processing transactions in various ways.
