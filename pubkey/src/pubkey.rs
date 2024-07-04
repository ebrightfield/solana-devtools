use solana_sdk::{bs58, ed25519_instruction::PUBKEY_SERIALIZED_SIZE, pubkey::Pubkey};

use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum NamedPubkeyError {
    #[error("Pubkey name prefix contains invalid Base58 characters: {0}")]
    InvalidEncoding(String),
    #[error("Pubkey name prefix is too long when Base58 encoded, encodes to {0} bytes")]
    TooLong(usize),
}

pub fn get_named_pubkey(prefix: String) -> Result<Pubkey, NamedPubkeyError> {
    let sanitized = prefix
        .replace('I', "i")
        .replace('O', "o")
        .replace('l', "L")
        .replace('0', "o");
    let prefix = bs58::decode(&sanitized)
        .into_vec()
        .map_err(|_| NamedPubkeyError::InvalidEncoding(prefix))?;

    let prefix_len = prefix.len();
    if prefix_len > PUBKEY_SERIALIZED_SIZE {
        return Err(NamedPubkeyError::TooLong(prefix_len));
    }
    let mut arr = [0u8; 32];
    arr[..prefix_len].copy_from_slice(&prefix[..]);

    Ok(Pubkey::new_from_array(arr))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::distributions::Alphanumeric;
    use rand::{thread_rng, Rng};

    #[test]
    fn check_base58_pubkey_validity() {
        for _ in 0..1000 {
            let identifier: String = thread_rng()
                .sample_iter(&Alphanumeric)
                .take(24)
                .map(|x| x as char)
                .collect();

            get_named_pubkey(identifier).expect("Invalid base58 pubkey");
        }
    }

    #[test]
    fn various_names() {
        let _ = get_named_pubkey("myname".to_string()).unwrap();
        let _ = get_named_pubkey("my_name".to_string()).unwrap_err();
        let _ = get_named_pubkey("myname123".to_string()).unwrap();
        let _ = get_named_pubkey("myreallylongname123456789iiiiiii".to_string()).unwrap();
    }
}
