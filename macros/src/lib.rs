extern crate core;
extern crate proc_macro;

mod const_data;

use const_data::{ConstValue, StructFields};

use core::panic;
use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use solana_devtools_pubkey::{get_named_pubkey, NamedPubkeyError};
use syn::{parse_macro_input, DeriveInput, LitStr, Token};

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
                solana_program::pubkey!(#key_str)
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

#[proc_macro_attribute]
pub fn const_data(attr: TokenStream, item: TokenStream) -> TokenStream {
    let const_values = parse_macro_input!(attr with syn::punctuated::Punctuated::<ConstValue, Token![;]>::parse_terminated);

    let input = parse_macro_input!(item as DeriveInput);
    let StructFields {
        typename,
        code_field,
        name_field,
        fields,
    } = const_data::parse_struct_fields(&input);

    let count = const_values.len(); // Get the count of inputs

    let mut const_declarations = vec![];
    for (index, const_value) in const_values.iter().enumerate() {
        let name = Ident::new(&const_value.name.value(), const_value.name.span());

        let mut field_initializers = fields
            .iter()
            .zip(&const_value.values)
            .map(|((field_name, _), expr)| quote!(#field_name: #expr))
            .collect::<Vec<_>>();

        if let Some(ref field) = code_field {
            let mint_code = index as u32;
            field_initializers.push(quote!(#field: #mint_code));
        }

        if let Some(ref field) = name_field {
            let name_str = &const_value.name.value();
            field_initializers.push(quote!(#field: #name_str));
        }

        const_declarations.push(quote! {
            pub const #name: #typename = #typename {
                #(#field_initializers),*
            };
        });
    }

    let consts_count_name = Ident::new("NUM_CONSTS", proc_macro2::Span::call_site());
    let count_const = quote! {
        pub const #consts_count_name: usize = #count;
    };

    TokenStream::from(quote! {
        #input
        #(#const_declarations)*
        #count_const
    })
}
