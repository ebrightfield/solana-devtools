extern crate proc_macro;
extern crate core;

use core::panic;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, LitStr};
use solana_devtools_anchor_utils::pubkey::{get_named_pubkey, NamedPubkeyError};

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

    // Generate the fake public key string
    match get_named_pubkey(input_value) {
        Ok(key) => {
            let key_str = key.to_string();
            let expanded = quote! {
                solana_sdk::pubkey!(#key_str)
            };
            TokenStream::from(expanded)
        }
        Err(NamedPubkeyError::InvalidIdentifier) =>
            panic!("The identifier provided is longer than 32 chars"),
        Err(NamedPubkeyError::PubkeyInvalidEncoding) =>
            panic!("The provided identifier cannot be turned into a base58 address and contains invalid special chars"),
        Err(NamedPubkeyError::PubkeyWrongSize) =>
            panic!("Unable to generate a valid sized public key with the provided identifier")
    }
}