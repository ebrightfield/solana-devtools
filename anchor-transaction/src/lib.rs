//! # Solana Anchor Lens
//!
//! A deserializer for accounts and instructions that come from Anchor programs.
//!
//! ```rust
//! use solana_client::rpc_client::RpcClient;
//! use solana_sdk::pubkey;
//! use solana_anchor_lens::AnchorLens;
//!
//! fn main() {
//!   let client = RpcClient::new("https://api.mainnet-beta.solana.com");
//!   // This type is the most convenient way to interact with the library,
//!   // but every step of the process is exposed if you need more fine-grained control.
//!   // See `AnchorDeserializer::new_with_caching` for IDL caching to save on RPC calls.
//!   let deser = AnchorLens::new(client);
//!   // The Marinade Finance mSOL state account.
//!   let key = pubkey!("8szGkuLTAux9XMgZ2vtY39jVSowEcpBfFfD8hXSEqdGC");
//!   let (prog_id, ix_name, json) = deser.fetch_and_deserialize_account_without_idl(&key);
//!   println!("Found program: {}", program_name);
//!   println!("Found account type: {}", act_type);
//!   println!("{}", serde_json::to_string_pretty(&value)?);
//!
//!   // But instead of one big, monolithic call, you can break it up
//!   // and save your IDL object for subsequent calls.
//!   let idl = deser.get_idl(pubkey!("MarBmsSgKXdrN1egZf5sqe1TMai9K1rChYNDJgjq7aD"))?;
//!   let (ix_name, json) = deser.deserialize_account_from_idl(&idl, &key);
//! }
//! ```
//!
pub mod deserialize;
pub mod fetch_idl;

pub use deserialize::AnchorLens;
