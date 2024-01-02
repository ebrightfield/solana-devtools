pub mod decompile_instructions;
pub mod inner_instructions;

use solana_program::message::CompileError;
/// Define a struct representing a transaction schema.
/// Implementing [TransactionSchema] allows for a number of
/// approaches to processing the transaction.
use solana_sdk::address_lookup_table_account::AddressLookupTableAccount;
use solana_sdk::hash::Hash;
use solana_sdk::instruction::Instruction;
use solana_sdk::message::{v0, Message, SanitizedMessage, VersionedMessage};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::SignerError;
use solana_sdk::signers::Signers;
use solana_sdk::transaction::{Transaction, VersionedTransaction};

/// Facilitates the creation of (un-)signed transactions, potentially serialized,
/// or lists of serialized instructions.
/// Any type `T` where `&T: Into<Vec<Instruction>>` implements this trait. By extension,
/// `&[Instruction]` and `Vec<Instruction>` also implements this trait.
pub trait TransactionSchema: Sized {
    /// Return an unsigned transaction
    fn unsigned_transaction(self, payer: Option<&Pubkey>) -> VersionedTransaction {
        let ixs: Vec<Instruction> = self.instructions();
        VersionedTransaction::from(Transaction::new_unsigned(Message::new(&ixs, payer)))
    }

    /// Return an unsigned transaction, serialized.
    /// Good for sending over the wire to request a signature.
    fn unsigned_serialized(self, payer: Option<&Pubkey>) -> Vec<u8> {
        let tx = self.unsigned_transaction(payer);
        tx.message.serialize()
    }

    fn message(self, payer: Option<&Pubkey>) -> VersionedMessage {
        let tx = self.unsigned_transaction(payer);
        tx.message
    }

    fn message_v0(
        self,
        payer: &Pubkey,
        lookups: &[AddressLookupTableAccount],
        recent_blockhash: Hash,
    ) -> Result<v0::Message, CompileError> {
        let instructions = self.instructions();
        v0::Message::try_compile(payer, &instructions, lookups, recent_blockhash)
    }

    fn sanitized_message(self, payer: Option<&Pubkey>) -> Option<SanitizedMessage> {
        let message = Message::new(&self.instructions(), payer);
        SanitizedMessage::try_from(message).ok()
    }

    /// Return a signed transaction.
    fn transaction(
        self,
        blockhash: Hash,
        payer: Option<&Pubkey>,
        signers: &impl Signers,
    ) -> VersionedTransaction {
        let ixs: Vec<Instruction> = self.instructions();
        VersionedTransaction::from(Transaction::new_signed_with_payer(
            &ixs, payer, signers, blockhash,
        ))
    }

    fn transaction_v0(
        self,
        blockhash: Hash,
        payer: &Pubkey,
        signers: &impl Signers,
        lookups: &[AddressLookupTableAccount],
    ) -> Result<VersionedTransaction, SignerError> {
        let message_v0 = self
            .message_v0(payer, lookups, blockhash)
            .map_err(|e| SignerError::Custom(format!("message failed to compile {}", e)))?;
        VersionedTransaction::try_new(VersionedMessage::V0(message_v0), signers)
    }

    /// Return a signed transaction, serialized
    fn signed_serialized(
        self,
        blockhash: Hash,
        payer: Option<&Pubkey>,
        signers: &impl Signers,
    ) -> Vec<u8> {
        let tx = self.transaction(blockhash, payer, signers);
        bincode::serialize(&tx).expect("transaction failed to serialize")
    }

    /// Return the instructions.
    fn instructions(self) -> Vec<Instruction>;

    /// Return the instructions in serialized form.
    fn instructions_serialized(self) -> Vec<Vec<u8>> {
        let ixs: Vec<Instruction> = self.instructions();
        ixs.iter()
            .map(|ix| bincode::serialize(ix).expect("instruction failed to serialize"))
            .collect()
    }

    fn programs(self) -> Vec<Pubkey> {
        let ixs: Vec<Instruction> = self.instructions();
        ixs.into_iter().map(|ix| ix.program_id).collect()
    }
}

impl<T: Sized> TransactionSchema for T
where
    T: Into<Vec<Instruction>>,
{
    fn instructions(self) -> Vec<Instruction> {
        self.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decompile_instructions::extract_instructions_from_message;
    use solana_sdk::signature::Keypair;
    use solana_sdk::signer::Signer;
    use spl_memo::build_memo;

    struct MemoType(String);

    impl Into<Vec<Instruction>> for &MemoType {
        fn into(self) -> Vec<Instruction> {
            vec![build_memo(self.0.as_bytes(), &[])]
        }
    }

    struct UnitStruct;

    impl Into<Vec<Instruction>> for &UnitStruct {
        fn into(self) -> Vec<Instruction> {
            vec![build_memo(b"hello world", &[])]
        }
    }

    impl Into<Vec<Instruction>> for UnitStruct {
        fn into(self) -> Vec<Instruction> {
            vec![build_memo(b"hello world", &[])]
        }
    }

    fn _test_func<'a, T>(t: &'a T)
    where
        &'a T: TransactionSchema + Copy,
    {
        let lookups = vec![AddressLookupTableAccount {
            key: Pubkey::new_unique(),
            addresses: vec![
                Pubkey::new_unique(),
                Pubkey::new_unique(),
                Pubkey::new_unique(),
            ],
        }];
        let key = Keypair::new();
        let _ = t.transaction(Hash::new_unique(), Some(&key.pubkey()), &vec![&key]);
        let _ = t.signed_serialized(Hash::new_unique(), Some(&key.pubkey()), &vec![&key]);
        let _ = t.message(None);
        let _ = t.message_v0(&key.pubkey(), &lookups, Hash::new_unique());
        let _ = t.transaction_v0(Hash::new_unique(), &key.pubkey(), &[&key], &lookups);
        let _ = t.unsigned_transaction(None);
        let _ = t.unsigned_serialized(None);
        let _ = t.instructions();
        let _ = t.instructions_serialized();
    }

    #[test]
    fn memo_type() {
        let memo = &MemoType(String::from("foo"));
        _test_func(memo);
        let key = Keypair::new();
        let _ = memo.transaction(Hash::new_unique(), Some(&key.pubkey()), &vec![&key]);
        let _ = memo.signed_serialized(Hash::new_unique(), Some(&key.pubkey()), &vec![&key]);
        let _ = memo.message(None);
        let _ = memo.unsigned_transaction(None);
        let _ = memo.unsigned_serialized(None);
        let _ = memo.instructions();
        let _ = memo.instructions_serialized();
    }

    #[test]
    fn ix() {
        let instructions = [
            build_memo(b"hello world", &[]),
            build_memo(b"hola mundo", &[]),
        ];
        let key = Keypair::new();

        let _ =
            instructions
                .clone()
                .transaction(Hash::new_unique(), Some(&key.pubkey()), &vec![&key]);
        let _ = instructions.clone().signed_serialized(
            Hash::new_unique(),
            Some(&key.pubkey()),
            &vec![&key],
        );
        let _ = instructions.clone().message(None);
        let _ = instructions.clone().unsigned_transaction(None);
        let _ = instructions.clone().unsigned_serialized(None);
        let _ = instructions.clone().instructions();
        let _ = instructions.clone().instructions_serialized();
    }

    #[test]
    fn unit_struct() {
        _test_func(&UnitStruct);
        let key = Keypair::new();

        let _ = UnitStruct.transaction(Hash::new_unique(), Some(&key.pubkey()), &vec![&key]);
        let _ = UnitStruct.signed_serialized(Hash::new_unique(), Some(&key.pubkey()), &vec![&key]);
        let _ = UnitStruct.message(None);
        let _ = UnitStruct.unsigned_transaction(None);
        let _ = UnitStruct.unsigned_serialized(None);
        let _ = UnitStruct.instructions();
        let _ = UnitStruct.instructions_serialized();
    }

    #[test]
    fn tx() {
        let keypair = Keypair::new();
        let tx = Transaction::new_signed_with_payer(
            &vec![build_memo(b"hello world", &[])],
            Some(&keypair.pubkey()),
            &[&keypair],
            Hash::new_unique(),
        );
        let ixs = extract_instructions_from_message(&tx.message);

        let new_signer = Keypair::new();

        let _ = ixs.clone().transaction(
            Hash::new_unique(),
            Some(&new_signer.pubkey()),
            &vec![&new_signer],
        );
        let _ = ixs.clone().signed_serialized(
            Hash::new_unique(),
            Some(&new_signer.pubkey()),
            &vec![&new_signer],
        );
        let _ = ixs.clone().message(None);
        let _ = ixs.clone().unsigned_transaction(None);
        let _ = ixs.clone().unsigned_serialized(None);
        let _ = ixs.clone().instructions();
        let _ = ixs.clone().instructions_serialized();
    }
}
