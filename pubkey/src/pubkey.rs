use solana_sdk::pubkey::{ParsePubkeyError, Pubkey};
use std::str::FromStr;

use thiserror::Error;

const TARGET_KEY_LENGTH: usize = 44;
const MIN_PAD_LENGTH: usize = 12;
const NAMED_ADDRESS_PAD_CHAR: char = '2';

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum NamedPubkeyError {
    #[error("Identifier contains invalid base58 chars that were not converted")]
    PubkeyInvalidEncoding,
    #[error("Generated pubkey is of an invalid length")]
    PubkeyWrongSize,
    #[error("Identifier is greater than 32 chars long")]
    InvalidIdentifier,
}

pub fn get_named_pubkey(identifier: String) -> Result<Pubkey, NamedPubkeyError> {
    let pad_length = TARGET_KEY_LENGTH - identifier.len();
    if pad_length < MIN_PAD_LENGTH {
        return Err(NamedPubkeyError::InvalidIdentifier);
    }

    let mut result = identifier
        .replace('I', "i")
        .replace('O', "o")
        .replace('l', "L")
        .replace('0', "o");

    result.reserve(pad_length);
    for _ in 0..pad_length {
        result.push(NAMED_ADDRESS_PAD_CHAR);
    }

    for pad_trim in 0..=MIN_PAD_LENGTH {
        let new_result = &result[..(TARGET_KEY_LENGTH - pad_trim)];
        match Pubkey::from_str(new_result) {
            Ok(key) => return Ok(key),
            Err(ParsePubkeyError::WrongSize) => continue,
            Err(_) => return Err(NamedPubkeyError::PubkeyInvalidEncoding),
        }
    }
    Err(NamedPubkeyError::PubkeyWrongSize)
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
                .take(TARGET_KEY_LENGTH - MIN_PAD_LENGTH)
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
