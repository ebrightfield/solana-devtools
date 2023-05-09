## Solana Devtools CLI Config

This library provides a Clap `UrlArg`, `KeypairArg` and `CommitmentArg`
that behave exactly like the vanilla Solana CLI.

This allows one to easily create CLI crates that integrate with
the vanilla Solana CLI configuration space.

The usage is therefore done in one of two ways:
- Creating args in your CLI such as `-u/--url`, `-k/--keypair`, or `--commitment`.
- Using `solana config set` to persistently set your CLI configuration.

### Example Usage:

Using the Clap derive API:

```
#[derive(Debug, Parser)]
struct Opt {
    #[clap(flatten)]
    url: UrlArg,
    #[clap(flatten)]
    keypair: KeypairArg,
    #[clap(flatten)]
    commitment: CommitmentArg,

    #[clap(subcommand)]
    cmd: Subcommand,
}
```

The above code snippet would allow one to pass optional args like:
```
your-cli-binary -ul -k some_keypair.json --commitment processed <subcommand>
```

or omit them and default to what is configured as shown when you `solana config get`.

The arg values are retrievable in code as follows:
```
// Get the URL
let url = self.url.resolve()?;
// Get the commitment
let commitment = self.commitment.resolve()?;

// Now you can create an `RpcClient` with the given configuration.
let client = RpcClient::new_with_commitment(url, commitment);

// Getting the keypair requires getting the Clap `ArgMatches`
let app = Opt::into_app();
let matches = app.get_matches();
let payer = self.keypair.resolve(&matches)?;
```
