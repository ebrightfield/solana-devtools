use std::str::FromStr;
use solana_program::pubkey::{ParsePubkeyError, Pubkey};
use thiserror::Error;

const TARGET_KEY_LENGTH: usize = 44;
const MIN_IDENTIFIER_LENGTH: usize = 12;
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

pub fn get_named_pubkey(
    identifier: String
) -> Result<Pubkey, NamedPubkeyError> {
    let pad_length = TARGET_KEY_LENGTH - identifier.len();
    if pad_length < MIN_IDENTIFIER_LENGTH {
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

    for pad_trim in 0..=MIN_IDENTIFIER_LENGTH {
        let new_result = &result[..(TARGET_KEY_LENGTH - pad_trim)];
        match Pubkey::from_str(new_result) {
            Ok(key) => return Ok(key),
            Err(ParsePubkeyError::WrongSize) => continue,
            Err(_) => return Err(NamedPubkeyError::PubkeyInvalidEncoding)
        }
    }
    Err(NamedPubkeyError::PubkeyWrongSize)
}

#[cfg(test)]
mod tests {
    use rand::{Rng, thread_rng};
    use rand::distributions::Alphanumeric;
    use super::*;

    #[test]
    fn check_base58_pubkey_validity() {
        for _ in 0..1000 {
            let identifier: String = thread_rng()
                .sample_iter(&Alphanumeric)
                .take(TARGET_KEY_LENGTH - MIN_IDENTIFIER_LENGTH)
                .map(|x| x as char)
                .collect();

            get_named_pubkey(identifier).expect("Invalid base58 pubkey");
        }
    }
}