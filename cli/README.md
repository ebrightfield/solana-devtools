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
    help                       Print this message or the help of the given subcommand(s)
    memo                       Execute a memo transaction
    name-service               Execute a transaction on the SPL Name Program
```

### A Few Useful Features
The following features that are missing from the vanilla Solana and Anchor CLIs.

- The `deserialize-*` commands are very useful for parsing accounts and transactions
into human-readable information, provided that there is an IDL available either on-chain or locally.
- The `get-transaction` command submits an RPC request to find a historical transaction.
- The `ata` command simply prints an associated token account.
- The `memo` command submits an SPL memo transaction.
You can also submit a memo of the SHA256 hash of a file at a given path.

