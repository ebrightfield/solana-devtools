
## Solana Client Transaction Processor

This library exposes a trait that allows one to define transactions by describing how to
construct their instruction data. It then offers a number of ways to process the transaction.

This includes:

1. Signing and sending.
2. Signing and simulating.
3. Signing and serializing (but not sending).
4. Serializing unsigned (so that it can be sent to be signed).
5. Serializing the instruction set (so that it can be used as instruction data for a multisig proposal).

It also includes hooks for offline versions of the above where applicable.
