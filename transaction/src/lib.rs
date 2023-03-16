/// Define a struct representing a transaction schema.
/// Implementing [TransactionSchema] allows for a number of
/// approaches to processing the transaction, from the most common
/// case of signing and sending, to more niche cases of printing instruction
/// data to use as a multisig proposal.
use serde_json::{Map, Value};
use solana_sdk::hash::Hash;
use solana_sdk::instruction::Instruction;
use solana_sdk::message::Message;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;

pub trait TransactionSchema: Into<Vec<Instruction>> {
    /// Return an unsigned transaction
    fn unsigned_transaction(&self) -> Transaction {
        Transaction::new_unsigned(
            Message::new(
                &ixs,
                payer,
            ),
        )
    }

    /// Return an unsigned transaction, serialized.
    fn unsigned_serialized(&self) -> Vec<u8> {
        let tx = self.unsigned_transaction();
        tx.message.serialize()
    }

    /// Return a signed transaction.
    fn transaction(&self, blockhash: Hash, payer: Option<&Pubkey>, signers: &[Box<dyn Signer>]) -> Transaction {
        let ixs: Vec<Instruction> = self.into();
        Transaction::new_signed_with_payer(
            &ixs,
            payer,
            signers,
            blockhash,
        )
    }

    /// Return a signed transaction, serialized
    fn signed_serialized(&self, blockhash: Hash, payer: Option<&Pubkey>, signers: &[Box<dyn Signer>]) -> Vec<u8> {
        let tx = self.transaction(blockhash, payer, signers);
        bincode::serialize(&tx)
             .expect("transaction failed to serialize")
    }

    /// Return the instruction set in serialized form.
    fn instructions_serialized(&self) -> Vec<Vec<u8>> {
        let ixs: Vec<Instruction> = self.into();
        ixs.iter().map(
            bincode::serialize(ix).expect("instruction failed to serialize")
        ).collect()
    }
}