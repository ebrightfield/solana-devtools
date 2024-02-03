extern crate proc_macro;

use proc_macro2::Ident;
use syn::parse::ParseStream;
use syn::DeriveInput;
use syn::{parse::Parse, Expr, LitStr, Token, Type};

pub(crate) struct StructFields {
    pub typename: Ident,
    pub name_field: Option<Ident>,
    pub code_field: Option<Ident>,
    pub fields: Vec<(Ident, Type)>,
}

pub(crate) fn parse_struct_fields(input: &DeriveInput) -> StructFields {
    let mut fields = Vec::new();
    let mut code_field = None;

    if let syn::Data::Struct(ref data_struct) = input.data {
        // Iterate over the fields of the struct
        for field in &data_struct.fields {
            // TODO Do the same thing with name
            if field.attrs.iter().any(|attr| attr.path().is_ident("code")) {
                // Only one code field
                if code_field.is_some() {
                    panic!("The const_data macro requires no more than one #[code] field");
                }
                // Code field must be a u32
                if let Type::Path(type_path) = &field.ty {
                    if let Some(last_segment) = type_path.path.segments.last() {
                        if last_segment.ident != "u32" {
                            panic!("The const_data macro #[code] field must be a u32");
                        }
                    }
                }
                // Must be a named field
                if field.ident.is_none() {
                    panic!("The const_data macro can only be applied to structs with named fields");
                }
                code_field = Some(field.ident.clone().unwrap());
                continue;
            }
            if let Some(ref ident) = field.ident {
                fields.push((ident.clone(), field.ty.clone()));
            } else {
                panic!("The const_data macro can only be applied to structs with named fields");
            }
        }
    } else {
        panic!("The const_data macro can only be applied to structs with named fields");
    }

    // Generate code that does nothing (for now), just return the original struct
    // This is where you'd add code generation based on parsed fields
    let typename = input.ident.clone();
    StructFields {
        typename,
        code_field,
        name_field: None,
        fields,
    }
}

pub(crate) struct ConstValue {
    pub name: LitStr,
    pub values: Vec<Expr>,
}

impl Parse for ConstValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: LitStr = input.parse()?;
        let mut values = vec![];
        loop {
            if let Ok(_) = input.parse::<Token![,]>() {
                let value: Expr = input.parse()?;
                values.push(value);
            } else {
                break;
            }
        }
        Ok(ConstValue { name, values })
    }
}
