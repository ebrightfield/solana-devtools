# Solana DevTools

This is a suite of crates to assist with Rust development for Solana and Anchor.

## Crates

- `solana-devtools-anchor-utils` -- Dynamic deserialization Anchor instructions and accounts using IDLs, and other QoL tooling for Anchor.
- `solana-devtools-cli-config` -- Structs and functions to make it easier to build Solana CLIs with Clap, implementing a super-set of the Solana CLI config behavior.
- `solana-devtools-cli` -- A CLI binary with useful dev/admin features that don't exist on the vanilla Solana and Anchor CLI tools.
- `solana-devtools-localnet` -- A library for creating and executing highly configured localnets.
- `solana-mock-runtime` -- Simulate the BPF execution of transactions locally with arbitrary account data and pubkeys, without the need to sign. This is because it processes `solana_sdk::message::Message` types instead of `Transactions`. You can choose whether or not to persist account data mutations across simulations.
- `solana-devnet-monitoring` -- Functions for tracking events, similar to Anchor's log subscribe approach, but with a trait based interface.
- `solana-devtools-signers` -- Useful structs that `impl Signer`.
- `solana-devtools-rpc` -- RPC client utilities. Add headers to RPC requests, print transaction logs from simulation errors.
- `serde-pubkey-str` -- (De-)serialize pubkeys to/from strings instead of byte-arrays.
- `solana-devtools-tx` -- A library for constructing and processing transactions in various ways.
