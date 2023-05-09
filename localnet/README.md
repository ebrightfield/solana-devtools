## Anchor Localnet Tools

Automates the creation of arbitrarily complex localnet setups, including
both test validator configuration as well as account and program loading.

Provides a Rust API for defining the account data in pre-loaded accounts,
creates a JS file that allows for their easy import into a test script,
and writes a `Test.toml` that consolidates the configuration.

### Why Use this Crate?

This crate is useful for creating localnet instances from which one can
test frontend applications locally without deploying to devnet.

It is also very useful for testing CLI crates, JS/TS SDKs, etc.

These localnet instances are also more easily probed and explored
than the usual `BankClient` based means of testing locally.
