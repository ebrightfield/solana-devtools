## Solana Devtools Signers

Provides two useful objects that implement `Signer`.

### Threadsafe Signer
A `solana_sdk::signer::Signer` that wraps a normal `Signer` type
and implements `Clone + Send + Sync`, and is therefore threadsafe.

### Concrete Signer
A signer that can be derived from the same multitude of string values
that are parsed in the Solana CLI, but with two main benefits:
1. The signer is not a Boxed type, and is instead generic to `T: Signer`.
2. There is no reliance on Clap `ArgMatches` to do the parsing.
