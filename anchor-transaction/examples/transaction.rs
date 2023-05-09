use anyhow::Result;
use solana_anchor_lens::AnchorLens;
use solana_client::rpc_client::RpcClient;
use solana_sdk::signature::Signature;
use std::str::FromStr;

fn main() -> Result<()> {
    let client = RpcClient::new("https://api.devnet.solana.com");
    let deser = AnchorLens::new(client);

    let sig = Signature::from_str(
        "5u3CjSy612jJpbfsqGGJNZjRi9APyLdnNHXVdXi5TKa8rESSp73jt85UL8ZpDPc7fiNaEsX5SXBmXUjZb5y68r4E",
    )?;
    println!("Attempting to parse transaction {}", sig.to_string());

    // This is the same as `self.client.get_transaction`, but with some preset
    // configuration and verbose unpacking of the RPC response.
    let tx = deser.get_versioned_transaction(&sig).unwrap();

    // If you only have the message object because it's not a historical
    // transaction, you can use this method.
    let _ = deser.deserialize_message(&tx.message)?;
    // Or, to process inner instructions on a historical transaction,
    // you can use this method instead.
    let deserialized = deser.deserialize_transaction(tx)?;
    println!("{}", serde_json::to_string_pretty(&deserialized)?);
    Ok(())
}
