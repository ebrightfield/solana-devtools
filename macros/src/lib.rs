extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, LitStr};

const TARGET_KEY_LENGTH: usize = 43;
const MIN_IDENTIFIER_LENGTH: usize = 8;
const FAKE_ADDRESS_PAD_CHAR: char = '2';

/// Creates a fake base58 public key via the solana_sdk::pubkey! proc macro, padding
/// the passed in string literal with 2's up to a 43 char address.
///
/// The last 8 charts are reserved for the identifer leaving 35 chars for the user to define.
///
/// The solana runtime seems to use 43 char base58 pubkeys for their system accounts, we will do the same.
#[proc_macro]
pub fn fake_pubkey(input: TokenStream) -> TokenStream {
    let input_str = parse_macro_input!(input as LitStr);
    let input_value = input_str.value();

    let pad_length = TARGET_KEY_LENGTH - input_value.len();
    if pad_length < MIN_IDENTIFIER_LENGTH {
        panic!("Input address literal length is greater than target key and minimum identifier length");
    }

    // Generate the fake public key string
    let fake_pubkey_str = format!("{}{}", input_str.value(), FAKE_ADDRESS_PAD_CHAR.to_string().repeat(pad_length));

    let expanded = quote! {
        solana_sdk::pubkey!(#fake_pubkey_str)
    };
    TokenStream::from(expanded)
}