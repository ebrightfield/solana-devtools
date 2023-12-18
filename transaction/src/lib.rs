/// Define a struct representing a transaction schema.
/// Implementing [TransactionSchema] allows for a number of
/// approaches to processing the transaction.
use solana_sdk::hash::Hash;
use solana_sdk::instruction::Instruction;
use solana_sdk::message::{Message, SanitizedMessage, VersionedMessage};
use solana_sdk::pubkey::Pubkey;
#[cfg(feature = "client")]
use solana_sdk::signature::Signature;
use solana_sdk::signers::Signers;
use solana_sdk::transaction::{Transaction, VersionedTransaction};

/// Facilitates the creation of (un-)signed transactions, potentially serialized,
/// or lists of serialized instructions.
/// Any type `T` where `&T: Into<Vec<Instruction>>` implements this trait. By extension,
/// `&[Instruction]` and `Vec<Instruction>` also implements this trait.
#[cfg_attr(feature = "async_client", async_trait::async_trait)]
pub trait TransactionSchema {
    /// Return an unsigned transaction
    fn unsigned_transaction(&self, payer: Option<&Pubkey>) -> VersionedTransaction;

    /// Return an unsigned transaction, serialized.
    /// Good for sending over the wire to request a signature.
    fn unsigned_serialized(&self, payer: Option<&Pubkey>) -> Vec<u8> {
        let tx = self.unsigned_transaction(payer);
        tx.message.serialize()
    }

    fn message(&self, payer: Option<&Pubkey>) -> VersionedMessage {
        let tx = self.unsigned_transaction(payer);
        tx.message
    }

    fn sanitized_message(&self, payer: Option<&Pubkey>) -> SanitizedMessage {
        let message = Message::new(&self.instructions(), payer);
        SanitizedMessage::try_from(message).unwrap()
    }

    /// Return a signed transaction.
    fn transaction<S: Signers>(
        &self,
        blockhash: Hash,
        payer: Option<&Pubkey>,
        signers: &S,
    ) -> VersionedTransaction;

    /// Return a signed transaction, serialized
    fn signed_serialized<S: Signers>(
        &self,
        blockhash: Hash,
        payer: Option<&Pubkey>,
        signers: &S,
    ) -> Vec<u8> {
        let tx = self.transaction(blockhash, payer, signers);
        bincode::serialize(&tx).expect("transaction failed to serialize")
    }

    /// Return the instructions.
    fn instructions(&self) -> Vec<Instruction>;

    /// Return the instructions in serialized form.
    fn instructions_serialized(&self) -> Vec<Vec<u8>> {
        let ixs: Vec<Instruction> = self.instructions();
        ixs.iter()
            .map(|ix| bincode::serialize(ix).expect("instruction failed to serialize"))
            .collect()
    }

    #[cfg(feature = "client")]
    fn sign_and_send<S: Signers>(
        &self,
        client: &solana_client::rpc_client::RpcClient,
        payer: &Pubkey,
        signers: &S,
        blockhash: Option<Hash>,
    ) -> solana_client::client_error::Result<Signature>;

    #[cfg(feature = "async_client")]
    async fn sign_and_send_nonblocking<S: Signers>(
        &self,
        client: &solana_client::nonblocking::rpc_client::RpcClient,
        payer: &Pubkey,
        signers: &S,
        blockhash: Option<Hash>,
    ) -> solana_client::client_error::Result<Signature>;
}

impl<T: ?Sized> TransactionSchema for T
where
    for<'a> &'a T: Into<Vec<Instruction>>,
{
    /// Return an unsigned transaction
    fn unsigned_transaction(&self, payer: Option<&Pubkey>) -> VersionedTransaction {
        let ixs: Vec<Instruction> = self.instructions();
        VersionedTransaction::from(Transaction::new_unsigned(Message::new(&ixs, payer)))
    }

    /// Return a signed transaction.
    fn transaction<S: Signers>(
        &self,
        blockhash: Hash,
        payer: Option<&Pubkey>,
        signers: &S,
    ) -> VersionedTransaction {
        let ixs: Vec<Instruction> = self.instructions();
        VersionedTransaction::from(Transaction::new_signed_with_payer(
            &ixs, payer, signers, blockhash,
        ))
    }

    /// Return the instructions.
    fn instructions(&self) -> Vec<Instruction> {
        self.into()
    }

    #[cfg(feature = "client")]
    fn sign_and_send<S: Signers>(
        &self,
        client: &solana_client::rpc_client::RpcClient,
        payer: &Pubkey,
        signers: &S,
        blockhash: Option<Hash>,
    ) -> solana_client::client_error::Result<Signature> {
        let blockhash = blockhash.unwrap_or(client.get_latest_blockhash()?);
        let transaction = self.transaction(blockhash, Some(payer), signers);
        client.send_transaction(&transaction)
    }

    #[cfg(feature = "async_client")]
    async fn sign_and_send_nonblocking<S: Signers>(
        &self,
        client: &solana_client::nonblocking::rpc_client::RpcClient,
        payer: &Pubkey,
        signers: &S,
        blockhash: Option<Hash>,
    ) -> solana_client::client_error::Result<Signature> {
        let blockhash = blockhash.unwrap_or(client.get_latest_blockhash().await?);
        let transaction = self.transaction(blockhash, Some(payer), signers);
        client.send_transaction(&transaction).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn memo_type() {
        let memo = MemoType(String::from("foo"));
        let key = Keypair::new();
        let _ = (&memo).transaction(Hash::new_unique(), Some(&key.pubkey()), &vec![&key]);
        let _ = (&memo).signed_serialized(Hash::new_unique(), Some(&key.pubkey()), &vec![&key]);
        let _ = (&memo).message(None);
        let _ = (&memo).unsigned_transaction(None);
        let _ = (&memo).unsigned_serialized(None);
        let _ = (&memo).instructions();
        let _ = (&memo).instructions_serialized();
    }

    #[test]
    fn ix() {
        let instructions = [
            build_memo(b"hello world", &[]),
            build_memo(b"hola mundo", &[]),
        ];
        let key = Keypair::new();

        let _ = (&instructions).transaction(Hash::new_unique(), Some(&key.pubkey()), &vec![&key]);
        let _ =
            (&instructions).signed_serialized(Hash::new_unique(), Some(&key.pubkey()), &vec![&key]);
        let _ = (&instructions).message(None);
        let _ = (&instructions).unsigned_transaction(None);
        let _ = (&instructions).unsigned_serialized(None);
        let _ = (&instructions).instructions();
        let _ = (&instructions).instructions_serialized();
    }

    #[test]
    fn unit_struct() {
        let unit_struct = UnitStruct;
        let key = Keypair::new();

        let _ = (&unit_struct).transaction(Hash::new_unique(), Some(&key.pubkey()), &vec![&key]);
        let _ =
            (&unit_struct).signed_serialized(Hash::new_unique(), Some(&key.pubkey()), &vec![&key]);
        let _ = (&unit_struct).message(None);
        let _ = (&unit_struct).unsigned_transaction(None);
        let _ = (&unit_struct).unsigned_serialized(None);
        let _ = (&unit_struct).instructions();
        let _ = (&unit_struct).instructions_serialized();
    }
}
