use anchor_lang::idl::IdlAccount;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use crate::deserialize::AnchorDeserializer;
use crate::deserialize::IdlWithDiscriminators;
use anyhow::{anyhow, Result};
use solana_devtools_tx::inner_instructions::{DecompiledMessageAndInnerIx, HistoricalTransaction};
use crate::deserialize::account::DeserializedAccount;

impl AnchorDeserializer {
    pub async fn fetch_and_cache_idl_for_program(&mut self, client: &RpcClient, program_id: &Pubkey) -> Result<()> {
        let idl = IdlWithDiscriminators::fetch_for_program(client, program_id).await?;
        self.cache_idl(*program_id, idl);
        Ok(())
    }

    pub async fn fetch_and_cache_idl(&mut self, client: &RpcClient, idl_account: &Pubkey, program_id: &Pubkey) -> Result<()> {
        let idl = IdlWithDiscriminators::fetch_from_account(client, idl_account).await?;
        self.cache_idl(*program_id, idl);
        Ok(())
    }

    /// Fails quietly for any programs it doesn't find.
    pub async fn fetch_and_cache_any_idls(&mut self, client: &RpcClient, message_and_inner_ix: HistoricalTransaction) -> Result<()> {
        let decompiled = DecompiledMessageAndInnerIx::from(message_and_inner_ix);
        for program in decompiled.programs() {
            if let Err(_) = self.fetch_and_cache_idl_for_program(client, &program).await {
                // TODO debug log
            }
        }
        Ok(())
    }
}

impl IdlWithDiscriminators {
    pub async fn fetch_from_account(client: &RpcClient, idl_addr: &Pubkey) -> anyhow::Result<IdlWithDiscriminators> {
        let account = client
            .get_account(idl_addr)
            .await
            .map_err(|_| anyhow!("IDL account not found"))?;
        Self::try_from(account)
    }

    pub async fn fetch_for_program(client: &RpcClient, program_id: &Pubkey) -> Result<IdlWithDiscriminators> {
        let idl_addr = IdlAccount::address(program_id);
        let account = client
            .get_account(&idl_addr)
            .await
            .map_err(|_| anyhow!("IDL account not found"))?;
        Self::try_from(account)
    }

    pub async fn get_deserialized_account(&self, client: &RpcClient, pubkey: &Pubkey) -> Result<DeserializedAccount> {
        let account = client.get_account(pubkey).await?;
        self.try_deserialize_account_to_json(pubkey, &account)
    }
}