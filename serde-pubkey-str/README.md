## Serde Pubkey Str

This library contains a simple implementation of (de-)serialization to/from String types,
rather than the default array (de-)serialization provided by default by the Solana SDK `Pubkey` type.

This is most useful for situations where one is storing/specifying Pubkeys in databases or JSON files,
and for other similar use cases related to databases or configuration in general.
