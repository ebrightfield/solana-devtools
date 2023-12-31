use anchor_syn::codegen::program::common::{sighash, SIGHASH_GLOBAL_NAMESPACE};
use anchor_syn::hash::hash;
use heck::SnakeCase;

pub type Discriminator = [u8; 8];

/// Calculates the discriminator for an account based on its name,
/// which would be found in an IDL.
pub fn account_discriminator(name: &str) -> Discriminator {
    hash(format!("account:{}", name).as_bytes()).to_bytes()[0..8]
        .try_into()
        .unwrap()
}

/// Calculates the discriminator for an instruction based on its name,
/// which would be found in an IDL.
pub fn ix_discriminator(name: &str) -> Discriminator {
    sighash(SIGHASH_GLOBAL_NAMESPACE, &name.to_snake_case())
}

/// Calculates the discriminator for a state-modifying instruction based on its name,
/// which would be found in an IDL.
pub fn ix_state_discriminator(name: &str) -> Discriminator {
    hash(format!("state:{}", name).as_bytes()).to_bytes()[0..8]
        .try_into()
        .unwrap()
}

pub fn partition_discriminator_from_data(data: &[u8]) -> ([u8; 8], Vec<u8>) {
    let mut first_eight_array = [0u8; 8];
    let len = data.len().min(8);

    // Copy up to the first 8 bytes into the array
    first_eight_array[..len].copy_from_slice(&data[..len]);

    let data = data[len..].to_vec();

    (first_eight_array, data)
}
