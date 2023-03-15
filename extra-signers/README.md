## Solana Devtools Signers

Provides two useful objects that implement `Signer`.

### Threadsafe Signer
A `solana_sdk::signer::Signer` that implements `Clone + Send + Sync`, and is therefore threadsafe.

### Concrete Signer
A signer that can be derived from the same multitude of string values
that are parsed in the Solana CLI, converted to `T: Signer` rather
than the `Box<dyn Signer>` that the Solana CLI libraries
return in `parse_from_signer`.
