use solana_client::nonblocking::rpc_client::RpcClient;
use solana_devtools_anchor_utils::deserialize::IdlWithDiscriminators;
use solana_sdk::pubkey;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());

    // Get the IDL from on-chain, parse it for discriminators to allow for account lookup
    let marinade_program = pubkey!("MarBmsSgKXdrN1egZf5sqe1TMai9K1rChYNDJgjq7aD");
    let idl = IdlWithDiscriminators::fetch_for_program(&client, &marinade_program).await?;

    // The Marinade Finance mSOL state account.
    let marinade_state = pubkey!("8szGkuLTAux9XMgZ2vtY39jVSowEcpBfFfD8hXSEqdGC");
    let account = client.get_account(&marinade_state).await?;

    // Deserialize
    let value = idl.try_deserialize_account(&marinade_state, &account)?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}
