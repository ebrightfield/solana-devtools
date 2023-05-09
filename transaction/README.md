
## Solana Devtools: Transaction

This library contains a `TransactionSchema` trait that allows one to
define a struct that implements a transaction schema, from which one can then:
- Create `Transaction` objects
- Create serialized unsigned transactions
- Create signed and serialized transactions
- Create `Vec<Instruction>` of the transaction's instruction set.
- Create a `Vec` of serialized instructions.
