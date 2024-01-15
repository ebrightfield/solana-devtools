use anchor_syn::idl::types::Idl;
use solana_program::pubkey::Pubkey;
use std::collections::HashMap;
use std::path::Path;

pub mod account;
#[cfg(feature = "client")]
pub mod client;
pub mod discriminator;
pub mod idl;
pub mod idl_types;
pub mod transaction;

pub use idl::IdlWithDiscriminators;

/// Wraps client calls and optionally caches the IDLs that it fetches.
/// This is the preferred means of fetching on-chain IDLs.
/// It's also an easy entrypoint to deserialize accounts
/// and transactions, although for finer grained control there are
/// separate functions for each step of the process.
///
/// Deserializes accounts and instructions, relying on the help
/// of program IDL accounts. These are found on chain, and they store
/// an Anchor IDL JSON file in compressed form.
pub struct AnchorDeserializer {
    pub idl_cache: HashMap<Pubkey, IdlWithDiscriminators>,
}

impl AnchorDeserializer {
    /// Initializes with caching turned off. This will make [AnchorDeserializer::fetch_idl]
    /// make an RPC call on every call.
    pub fn new() -> Self {
        Self {
            idl_cache: HashMap::new(),
        }
    }

    pub fn new_with_idls(idls: HashMap<Pubkey, Idl>) -> Self {
        let idl_cache = HashMap::from_iter(
            idls.into_iter()
                .map(|(pubkey, idl)| (pubkey, IdlWithDiscriminators::new(idl))),
        );
        Self { idl_cache }
    }

    pub fn cache_idl(&mut self, program_id: Pubkey, idl: IdlWithDiscriminators) {
        self.idl_cache.insert(program_id, idl);
    }

    pub fn cache_idl_from_file(
        &mut self,
        program_id: Pubkey,
        path: impl AsRef<Path>,
    ) -> anyhow::Result<()> {
        let idl = IdlWithDiscriminators::from_file(path)?;
        self.cache_idl(program_id, idl);
        Ok(())
    }
}
