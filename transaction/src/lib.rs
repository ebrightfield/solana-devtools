/// Define a struct representing a transaction schema.
/// Implementing [TransactionSchema] allows for a number of
/// approaches to processing the transaction, from the most common
/// case of signing and sending, to more niche cases of printing instruction
/// data to use as a multisig proposal.
use solana_sdk::hash::Hash;
use solana_sdk::instruction::Instruction;
use solana_sdk::message::Message;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signers::Signers;
use solana_sdk::transaction::Transaction;

pub trait TransactionSchema: Sized {
    /// Return an unsigned transaction
    fn unsigned_transaction(self, payer: Option<&Pubkey>) -> Transaction;

    /// Return an unsigned transaction, serialized.
    fn unsigned_serialized(self, payer: Option<&Pubkey>) -> Vec<u8> {
        let tx = self.unsigned_transaction(payer);
        tx.message.serialize()
    }

    /// Return a signed transaction.
    fn transaction<S: Signers>(self, blockhash: Hash, payer: Option<&Pubkey>, signers: &S) -> Transaction;

    /// Return a signed transaction, serialized
    fn signed_serialized<S: Signers>(self, blockhash: Hash, payer: Option<&Pubkey>, signers: &S) -> Vec<u8> {
        let tx = self.transaction(blockhash, payer, signers);
        bincode::serialize(&tx)
            .expect("transaction failed to serialize")
    }

    /// Return the instructions.
    fn instructions(self) -> Vec<Instruction>;

    /// Return the instructions in serialized form.
    fn instructions_serialized(self) -> Vec<Vec<u8>> {
        let ixs: Vec<Instruction> = self.instructions();
        ixs.iter().map(
            |ix| bincode::serialize(ix).expect("instruction failed to serialize")
        ).collect()
    }
}

impl<T: Into<Vec<Instruction>>> TransactionSchema for T {
    /// Return an unsigned transaction
    fn unsigned_transaction(self, payer: Option<&Pubkey>) -> Transaction {
        let ixs: Vec<Instruction> = self.into();
        Transaction::new_unsigned(
            Message::new(
                &ixs,
                payer,
            ),
        )
    }

    /// Return a signed transaction.
    fn transaction<S: Signers>(self, blockhash: Hash, payer: Option<&Pubkey>, signers: &S) -> Transaction {
        let ixs: Vec<Instruction> = self.into();
        Transaction::new_signed_with_payer(
            &ixs,
            payer,
            signers,
            blockhash,
        )
    }

    /// Return the instructions.
    fn instructions(self) -> Vec<Instruction> {
        self.into()
    }
}

#[cfg(test)]
mod tests {
    use solana_sdk::signature::Keypair;
    use spl_memo::build_memo;
    use super::*;

    struct MemoType(String);

    impl Into<Vec<Instruction>> for &MemoType {
        fn into(self) -> Vec<Instruction> {
            vec![build_memo(self.message.as_bytes(), &[])]
        }
    }

    #[test]
    fn memo_type() {
        let memo = MemoType(String::from("foo"));
        let tx = (&memo).transaction(Hash::new_unique(), None, &vec![
            Box::new(Keypair::new())
        ]);
        let tx = (&memo).signed_serialized(Hash::new_unique(), None, &vec![
            Box::new(Keypair::new())
        ]);
        let ixs = (&memo).unsigned_transaction(None);
        let ixs = (&memo).unsigned_serialized(None);
        let ixs = (&memo).instructions();
        let ixs = (&memo).instructions_serialized();
    }
}