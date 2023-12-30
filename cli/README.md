## Solana Devtools CLI

```
$ solana-devtools --help
solana-devtools-cli 
CLI for an improved Solana DX

USAGE:
    solana-devtools [OPTIONS] <SUBCOMMAND>

OPTIONS:
    -c, --commitment <COMMITMENT>
            [possible values: processed, confirmed, finalized]

        --confirm-key
            Manually confirm the signer before proceeding

    -h, --help
            Print help information

    -k, --keypair <KEYPAIR>
            The target signer for transactions. See Solana CLI documentation on how to use this.
            Default values and usage patterns are identical to Solana CLI

        --skip-seed-phrase-validation
            Skip BIP-39 seed phrase validation (not recommended)

    -u, --url <URL>
            The target URL for the cluster. See Solana CLI documentation on how to use this. Default
            values and usage patterns are identical to Solana CLI

SUBCOMMANDS:
    ata                        Display the owner's associated token address for a given mint.
                                   Owner defaults to the configured signer
    deserialize-account        Fetch account data and attempt to deserialize it using Anchor IDL
                                   data
    deserialize-message        Deserialize an unsigned transaction message encoded in Base58
    deserialize-transaction    Fetch a confirmed transaction and attempt to deserialize it using
                                   Anchor IDL data
    faucet                     Execute a transaction on the SPL Token Faucet program. The
                                   program is on devnet at
                                   4bXpkKSV8swHSnwqtzuboGPaPDeEgAn4Vt8GfarV5rZt. See
                                   https://github.com/paul-schaaf/spl-token-faucet for source code
    get-transaction            A vanilla RPC call to get a confirmed transaction
    help                       Print this message or the help of the given subcommand(s)
    memo                       Execute a memo transaction
    name-service               Execute a transaction on the SPL Name Program
```

### A Few Useful Features
The following are features that are glaringly missing from the vanilla Solana and Anchor CLIs.

- The `deserialize-*` commands are very useful for parsing accounts and transactions
into human-readable information, provided that there is an IDL available either on-chain or locally.
- The `get-transaction` command submits an RPC request to find a historical transaction.
- The `ata` command simply prints an associated token account.
- The `memo` command submits an SPL memo transaction.
You can also submit a memo of the SHA256 hash of a file at a given path.

Other feature-flagged subcommands are detailed below.

### SPL Token Faucet CLI (build with `faucet` feature)
The `faucet` subcommand provides a CLI for the SPL Token Faucet program,
the source code of which can be found here:

https://github.com/paul-schaaf/spl-token-faucet

See examples below, or use `solana-devtools faucet --help`

##### Create new faucets
```
$ solana-keygen new -o "fake_mint.json" --no-passphrase

# Make a mint owned by the SPL Faucet mint authority (with 4 decimals)
$ solana-devtools faucet init-spl-mint fake_mint.json 4
2n7hTqsS1G5nS6i1vUJS2kcaXnJtUMFrd48ezCPqVSccCpmXMh5KXp2V1VxsBW4eu3RPzZzXEqHh9UCivftJ8qQ8

# Initialize a faucet with a maximum mint amount of 10,000,000
$ solana-devtools faucet init-faucet $(solana-keygen pubkey fake_mint.json) 10000000
Attempting to create faucet at address: <faucet-address>
CsHobJFVV52eubuXvDGBtH8XivtTBvMBKy2bK7UGFvySPnUqGCYPckXmwM1XcJB1SmecL7tPaCCC15eUWBwucjN
```

##### Airdrop tokens from faucets
```
# First time
$ solana-devtools faucet mint --init-ata <faucet-address> 1000000

# Subsequent executions
$ solana-devtools faucet mint <faucet-address> 1000000
```

##### Show human-readable faucet account data.
```
$ solana-devtools faucet show <faucet-address>                 
Faucet {
    is_initialized: true,
    admin: None,
    mint: 5SfcRksKyJBBLjqgWG8eFLLcheBi34Nn8zqgtjMHP2SJ,
    amount: 1000000000000,
}
```

### SPL Name Service Subcommand (build with `name-service` feature)
```
$ solana-devtools name-service --help
solana-devtools-name-service 
Execute a transaction on the SPL Name Program

USAGE:
    solana-devtools name-service <SUBCOMMAND>

OPTIONS:
    -h, --help    Print help information

SUBCOMMANDS:
    create            Create a new name record
    delete            Delete a name account
    derive-account    Derive the PDA address given a name, and optionally a class and parent
    help              Print this message or the help of the given subcommand(s)
    read              Read the state of a name account
    transfer          Transfer ownership of a name account
    update            Update the data stored on a name record
```