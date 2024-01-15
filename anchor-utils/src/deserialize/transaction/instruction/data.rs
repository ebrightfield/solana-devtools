use crate::deserialize::discriminator::partition_discriminator_from_data;
use crate::deserialize::IdlWithDiscriminators;
use anchor_syn::idl::types::IdlInstruction;
use anyhow::anyhow;
use serde_json::Value;

impl IdlWithDiscriminators {
    /// Deserialize the arguments passed to an instruction as instruction data.
    pub fn try_deserialize_instruction_data(
        &self,
        ix_data: &[u8],
    ) -> anyhow::Result<(IdlInstruction, Value)> {
        let (discriminator, data) = partition_discriminator_from_data(ix_data);
        let ix = self
            .instruction_definitions
            .get(&discriminator)
            .ok_or(anyhow!(
                "Could not match instruction against any discriminator"
            ))?;
        Ok((
            ix.clone(),
            self.deserialize_named_fields(&ix.args, &mut &data[..])?,
        ))
    }
}
