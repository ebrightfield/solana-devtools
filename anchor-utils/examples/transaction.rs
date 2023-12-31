use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::signature::Signature;
use std::str::FromStr;
use solana_sdk::pubkey;
use solana_devtools_anchor_utils::deserialize::AnchorDeserializer;
use solana_devtools_tx::inner_instructions::HistoricalTransaction;

#[tokio::main]
async fn main() -> Result<()> {
    let client = RpcClient::new("https://api.devnet.solana.com".to_string());

    // Since deserializing a transaction might involve several programs (and thus several IDLs),
    // the preferred way is through this object which caches multiple IDLs.
    let mut deser = AnchorDeserializer::new();

    let marinade_program = pubkey!("MarBmsSgKXdrN1egZf5sqe1TMai9K1rChYNDJgjq7aD");
    deser.fetch_and_cache_idl_for_program(&client, &marinade_program).await?;

    let sig = Signature::from_str(
        "5u3CjSy612jJpbfsqGGJNZjRi9APyLdnNHXVdXi5TKa8rESSp73jt85UL8ZpDPc7fiNaEsX5SXBmXUjZb5y68r4E",
    )?;
    let tx = HistoricalTransaction::get_nonblocking(&client, &sig).await?;
    println!("Attempting to parse transaction {}", sig.to_string());

    // If you only have the message object because it's not a historical
    // transaction, you can use this method.
    let _ = deser.try_deserialize_message(&tx.message)?;

    // Or, to process inner instructions on a historical transaction,
    // you can use this method instead.
    let deserialized = deser.try_deserialize_transaction(tx)?;
    println!("{}", serde_json::to_string_pretty(&deserialized)?);
    Ok(())
}
