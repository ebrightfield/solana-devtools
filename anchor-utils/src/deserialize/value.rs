//! Parse a `serde_json::Value` to a target type using an IDL.

// Value --(using some target type)--> Serialized Bytes

use std::{cmp::max, str::FromStr};

use anchor_lang::AnchorSerialize;
use anchor_syn::idl::types::{EnumFields, IdlType, IdlTypeDefinition, IdlTypeDefinitionTy};
use anyhow::{anyhow, Context};
use serde_json::Value;
use solana_sdk::pubkey::Pubkey;

use super::IdlWithDiscriminators;

impl IdlWithDiscriminators {
    /// Convert a [Value] to Borsh-serialized bytes
    pub fn try_from_value_borsh(&self, type_def: &IdlTypeDefinition, value: &Value) -> Vec<u8> {
        todo!()
    }

    pub fn serialize_named_field(
        &self,
        field_name: &str,
        value: &Value,
        idl_type: &IdlType,
        raw_data: &mut [u8],
    ) -> anyhow::Result<()> {
        value
            .get(&field_name)
            .with_context(|| format!("Failed to get field {field_name} as unsigned integer"))
            .and_then(|v| self.serialize_value_as_idl_type(v, idl_type, raw_data))
    }

    pub fn serialize_value_with_type_definition(
        &self,
        value: &Value,
        type_def: &IdlTypeDefinition,
    ) -> anyhow::Result<()> {
        let mut raw_data = vec![];
        Ok(match &type_def.ty {
            IdlTypeDefinitionTy::Struct { fields } => {
                for field in fields {
                    self.serialize_value_as_idl_type(value, &field.ty, &mut raw_data)?;
                }
            }
            IdlTypeDefinitionTy::Enum { variants } => {
                // TODO Need to decide on a canonical way to represent enums as [Value] types.
                // for variant in variants {
                //     if let Some(fields) = &variant.fields {
                //         match fields {
                //             EnumFields::Named(fields) => {
                //                 let len = fields.iter().fold(Ok(0usize), |sum, t| {
                //                     sum.and_then(|s| self.len_of_idl_type(&t.ty).map(|l| l + s))
                //                 })?;
                //                 largest_variant_len = max(len, largest_variant_len);
                //             }
                //             EnumFields::Tuple(idl_types) => {
                //                 let len = idl_types.iter().fold(Ok(0usize), |sum, t| {
                //                     sum.and_then(|s| self.len_of_idl_type(t).map(|l| l + s))
                //                 })?;
                //                 largest_variant_len = max(len, largest_variant_len);
                //             }
                //         }
                //     }
                // }
                todo!()
            }
            IdlTypeDefinitionTy::Alias { value } => self.len_of_idl_type(value)?,
        })
    }

    /// Try to deserialize from raw byte data based on a given [IdlType].
    pub fn serialize_value_as_idl_type(
        &self,
        value: &Value,
        idl_type: &IdlType,
        mut raw_data: &mut [u8],
    ) -> anyhow::Result<()> {
        match &idl_type {
            IdlType::Bool => {
                let field = value
                    .as_bool()
                    .with_context(|| format!("Failed to read value as bool"))?;
                field.serialize(&mut raw_data)?;
            }
            IdlType::U8 => {
                let field = value
                    .as_u64()
                    .with_context(|| format!("Failed to read value as u8"))
                    .and_then(|f| {
                        TryInto::<u8>::try_into(f)
                            .with_context(|| format!("integer out of bounds for u8: {f}"))
                    })?;
                field.serialize(&mut raw_data)?;
            }
            // IdlType::I8 => {
            //     let value: i8 = borsh::BorshDeserialize::deserialize(raw_data)?;
            //     return Ok(Value::Number(value.into()));
            // }
            // IdlType::U16 => {
            //     let value: u16 = borsh::BorshDeserialize::deserialize(raw_data)?;
            //     return Ok(Value::Number(value.into()));
            // }
            // IdlType::I16 => {
            //     let value: i16 = borsh::BorshDeserialize::deserialize(raw_data)?;
            //     return Ok(Value::Number(value.into()));
            // }
            // IdlType::U32 => {
            //     let value: u32 = borsh::BorshDeserialize::deserialize(raw_data)?;
            //     return Ok(Value::Number(value.into()));
            // }
            // IdlType::I32 => {
            //     let value: i32 = borsh::BorshDeserialize::deserialize(raw_data)?;
            //     return Ok(Value::Number(value.into()));
            // }
            // IdlType::F32 => {
            //     let value: f32 = borsh::BorshDeserialize::deserialize(raw_data)?;
            //     return Ok(Value::String(value.to_string()));
            // }
            IdlType::U64 => {
                let field = value
                    .as_u64()
                    .with_context(|| format!("Failed to read value as u64"))?;
                field.serialize(&mut raw_data)?;
            }
            // IdlType::I64 => {
            //     let value: i64 = borsh::BorshDeserialize::deserialize(raw_data)?;
            //     return Ok(Value::Number(value.into()));
            // }
            // IdlType::F64 => {
            //     let value: f64 = borsh::BorshDeserialize::deserialize(raw_data)?;
            //     return Ok(Value::String(value.to_string()));
            // }
            IdlType::U128 => {
                let field = value
                    .as_str()
                    .with_context(|| format!("Failed to read value as string (u128)"))
                    .and_then(|f| {
                        u128::from_str(f)
                            .with_context(|| format!("failed to parse u128 from &str: {f}"))
                    })?;
                field.serialize(&mut raw_data)?;
            }
            // IdlType::I128 => {
            //     let value: i128 = borsh::BorshDeserialize::deserialize(raw_data)?;
            //     return Ok(Value::String(value.to_string()));
            // }
            // IdlType::Bytes => {
            //     let value: Vec<u8> = borsh::BorshDeserialize::deserialize(raw_data)?;
            //     return Ok(Value::Array(
            //         value.iter().map(|v| Value::Number((*v).into())).collect(),
            //     ));
            // }
            IdlType::String => {
                let field = value
                    .as_u64()
                    .with_context(|| format!("Failed to read value as u64"))?;
                field.serialize(&mut raw_data)?;
            }
            IdlType::PublicKey => {
                let field = value
                    .as_str()
                    .with_context(|| format!("Failed to read value as string (pubkey)"))
                    .and_then(|f| {
                        Pubkey::from_str(f)
                            .with_context(|| format!("failed to parse pubkey from &str: {f}"))
                    })?;
                field.serialize(&mut raw_data)?;
            }
            IdlType::Defined(defined_type) => {
                let (_, type_def) =
                    self.find_type_definition_by_name(&defined_type)
                        .ok_or(anyhow!(
                            "Failed to find type definition for type named: {defined_type}"
                        ))?;
                self.serialize_value_with_type_definition(value, type_def)?;
            }
            IdlType::Option(idl_type) => {
                let is_none = value.as_null().is_some();
                if is_none {
                    self.serialize_none_variant_of_option_t(idl_type, raw_data)?;
                } else {
                    true.serialize(&mut raw_data)?;
                    self.serialize_value_as_idl_type(value, idl_type, raw_data)?;
                }
            }
            IdlType::Vec(idl_type) => {
                let arr = value
                    .as_array()
                    .with_context(|| format!("Failed to read value as array"))?;
                let len = arr.len() as u32;
                len.serialize(&mut raw_data)?;
                for element in arr {
                    self.serialize_value_as_idl_type(element, idl_type, raw_data)?;
                }
            }
            IdlType::Array(idl_type, arr_len) => {
                let arr = value
                    .as_array()
                    .with_context(|| format!("Failed to read value as array"))?;
                if arr.len() != *arr_len {
                    return Err(anyhow!("expected array length {arr_len}"));
                }
                for i in 0..*arr_len {
                    let value = arr.get(i).unwrap();
                    self.serialize_value_as_idl_type(value, idl_type, raw_data)?;
                }
            }
            _ => {
                return Err(anyhow!(
                    "serialization of type {:?} not supported",
                    idl_type
                ));
            }
        }
        Ok(())
    }

    pub fn serialize_none_variant_of_option_t(
        &self,
        idl_type: &IdlType,
        mut raw_data: &mut [u8],
    ) -> anyhow::Result<()> {
        false.serialize(&mut raw_data)?;
        vec![0u8; self.len_of_idl_type(idl_type)?].serialize(&mut raw_data)?;
        Ok(())
    }

    pub fn len_of_idl_type(&self, idl_type: &IdlType) -> anyhow::Result<usize> {
        Ok(match idl_type {
            IdlType::Bool | IdlType::U8 | IdlType::I8 => 1,
            IdlType::U16 | IdlType::I16 => 2,
            IdlType::U32 | IdlType::I32 | IdlType::F32 => 4,
            IdlType::U64 | IdlType::I64 | IdlType::F64 => 8,
            IdlType::U128 | IdlType::I128 => 16,
            IdlType::U256 | IdlType::I256 | IdlType::PublicKey => 32,
            IdlType::Option(ty) => 1 + self.len_of_idl_type(ty)?,
            IdlType::Array(ty, len) => len * self.len_of_idl_type(ty)?,
            IdlType::Defined(name) => {
                let ty = self.get_type_definition_by_name(name).ok_or(anyhow!(
                    "unknown defined type {name} in IDL for program: {}",
                    self.name
                ))?;
                self.len_from_type_definition(ty)?
            }
            IdlType::Vec(_) => todo!(),
            IdlType::Bytes => todo!(),
            IdlType::String => todo!(),
            IdlType::GenericLenArray(_, _) => todo!(),
            IdlType::Generic(_) => todo!(),
            // IdlType::DefinedWithTypeArgs { name, args } => todo!(),
            _ => unimplemented!("Type length calculation yet supported: {:?}", idl_type),
        })
    }

    pub fn len_from_type_definition(&self, type_def: &IdlTypeDefinition) -> anyhow::Result<usize> {
        Ok(match &type_def.ty {
            IdlTypeDefinitionTy::Struct { fields } => {
                fields.iter().fold(Ok(0usize), |sum, t| {
                    sum.and_then(|s| self.len_of_idl_type(&t.ty).map(|l| l + s))
                })?
            }
            IdlTypeDefinitionTy::Enum { variants } => {
                let mut largest_variant_len = 0usize;
                for variant in variants {
                    if let Some(fields) = &variant.fields {
                        match fields {
                            EnumFields::Named(fields) => {
                                let len = fields.iter().fold(Ok(0usize), |sum, t| {
                                    sum.and_then(|s| self.len_of_idl_type(&t.ty).map(|l| l + s))
                                })?;
                                largest_variant_len = max(len, largest_variant_len);
                            }
                            EnumFields::Tuple(idl_types) => {
                                let len = idl_types.iter().fold(Ok(0usize), |sum, t| {
                                    sum.and_then(|s| self.len_of_idl_type(t).map(|l| l + s))
                                })?;
                                largest_variant_len = max(len, largest_variant_len);
                            }
                        }
                    }
                }
                4 + largest_variant_len
            }
            IdlTypeDefinitionTy::Alias { value } => self.len_of_idl_type(value)?,
        })
    }
}

// Hmm, could I do this as like a serde serializer / deserializer?
// I should be able to, like, loop over the type def struct fields, and for each, look up the name in the value.
// And then given some type

/*
               "i8" | "u8" | "bool" => quote!(1),
               "i16" | "u16" => quote!(2),
               "i32" | "u32" | "f32" => quote!(4),
               "i64" | "u64" | "f64" => quote!(8),
               "i128" | "u128" => quote!(16),
               "String" => {
                   let max_len = get_next_arg(ident, attrs);
                   quote!((4 + #max_len))
               }
               "Pubkey" => quote!(32),
               "Option" => {
                   if let Some(ty) = first_ty {
                       let type_len = len_from_type(ty, attrs);

                       quote!((1 + #type_len))
                   } else {
                       quote_spanned!(ident.span() => compile_error!("Invalid argument in Vec"))
                   }
               }
               "Vec" => {
                   if let Some(ty) = first_ty {
                       let max_len = get_next_arg(ident, attrs);
                       let type_len = len_from_type(ty, attrs);

                       quote!((4 + #type_len * #max_len))
                   } else {
                       quote_spanned!(ident.span() => compile_error!("Invalid argument in Vec"))
                   }
               }
               _ => {
                   let ty = &ty_path.path;
                   quote!(<#ty as anchor_lang::Space>::INIT_SPACE)
               }
*/

// fn serialize_as<T: AnchorSerialize>(
//     mut raw_data: &mut [u8],
//     value: &Value,
//     conversion_fn: impl Fn(&Value) -> Option<T>,
//     typename: &str,
// ) -> anyhow::Result<()> {
//     let field =
//         conversion_fn(value).with_context(|| format!("Failed to read value as {typename}"))?;
//     field.serialize(&mut raw_data)?;
//     Ok(())
// }

// fn serialize_as_and_then<T: AnchorSerialize, U>(
//     mut raw_data: &mut [u8],
//     value: &Value,
//     conversion_fn: impl Fn(&Value) -> Option<U>,
//     conversion_fn2: impl Fn(U) -> anyhow::Result<T>,
//     typename: &str,
// ) -> anyhow::Result<()> {
//     let field = conversion_fn(value)
//         .with_context(|| format!("Failed to read value as {typename}"))
//         .and_then(|f| conversion_fn2(f))?;
//     field.serialize(&mut raw_data)?;

//     Ok(())
// }
