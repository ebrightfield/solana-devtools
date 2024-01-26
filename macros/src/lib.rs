extern crate proc_macro;
extern crate core;

use core::panic;
use proc_macro::TokenStream;
use std::str::FromStr;
use quote::quote;
use solana_sdk::pubkey::{ParsePubkeyError, Pubkey};
use syn::{parse_macro_input, LitStr};

const TARGET_KEY_LENGTH: usize = 44;
const MIN_IDENTIFIER_LENGTH: usize = 12;
const FAKE_ADDRESS_PAD_CHAR: char = '2';

/// Creates a fake base58 public key via the solana_sdk::pubkey! proc macro, padding
/// the passed in string literal with 2's up to a 44 char address.
///
/// Due to some chars not being included in the base58 encoding, the following chars are
/// converted to their counterpart.
/// ```text
/// 'I' => 'i'
/// 'O' => 'o'
/// 'l' => 'L'
/// '0' => 'o'
/// ```
///
/// The last 12 chars are reserved for the identifier, in a case where the resulting
/// byte array from a pubkey of 44 chars exceeds 32 bytes. The macro will reduce the
/// length of the reserved space until the correct key size is created.
///
/// A valid base58 address is anywhere between 32 and 44 chars specified on the
/// [Solana CLI Docs](https://docs.solana.com/cli/transfer-tokens). The minimum length
/// pubkey that will be produced is 32.
#[proc_macro]
pub fn named_pubkey(input: TokenStream) -> TokenStream {
    let input_str = parse_macro_input!(input as LitStr);
    let input_value = input_str.value();

    let pad_length = TARGET_KEY_LENGTH - input_value.len();
    if pad_length < MIN_IDENTIFIER_LENGTH {
        panic!("Input address literal and minimum identifier length is greater than target key");
    }

    // Generate the fake public key string
    let fake_pubkey_str = get_named_base58_str(input_value, pad_length);

    let expanded = quote! {
        solana_sdk::pubkey!(#fake_pubkey_str)
    };
    TokenStream::from(expanded)
}

fn get_named_base58_str(identifier: String, max_pad_length: usize) -> String {
    let mut result = identifier
        .replace('I', "i")
        .replace('O', "o")
        .replace('l', "L")
        .replace('0', "o");

    result.reserve(max_pad_length);
    for _ in 0..max_pad_length {
        result.push(FAKE_ADDRESS_PAD_CHAR);
    }

    let len = result.len();
    for pad_trim in 0..=MIN_IDENTIFIER_LENGTH {
        let new_result = &result[..(len - pad_trim)];
        match Pubkey::from_str(new_result) {
            Ok(_) => return new_result.to_string(),
            Err(ParsePubkeyError::WrongSize) => continue,
            Err(ParsePubkeyError::Invalid) => panic!("Invalid base58 encoding, invalid chars")
        }
    }

    println!("{}", result);

    panic!("Unable to create 32 byte array from padded identifier");
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

            let maybe_valid_pubkey = get_named_base58_str(identifier, MIN_IDENTIFIER_LENGTH);

            Pubkey::from_str(&maybe_valid_pubkey)
                .expect("Invalid base58 pubkey");
        }
    }
}