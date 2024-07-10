//! Parse a `serde_json::Value` to a target type using an IDL.

// Value --(using some target type)--> Serialized Bytes

use std::{
    cmp::max,
    io::{Cursor, Write},
    str::FromStr,
};

use anchor_lang::AnchorSerialize;
use anchor_syn::idl::types::{EnumFields, IdlType, IdlTypeDefinition, IdlTypeDefinitionTy};
use anyhow::{anyhow, Context};
use serde_json::Value;
use solana_sdk::pubkey::Pubkey;

use crate::deserialize::idl_types::from_tagged_enum;

use super::IdlWithDiscriminators;

const VARIABLE_LENGTH_BORSH_PREFIX: usize = 4;
const DEFAULT_VARIABLE_LENGTH_BORSH_SPACE: usize = 0;

impl IdlWithDiscriminators {
    /// Convert a [Value] to Borsh-serialized bytes
    pub fn try_from_value_borsh(
        &self,
        type_def: &IdlTypeDefinition,
        value: &Value,
    ) -> anyhow::Result<Vec<u8>> {
        let (_, discriminator, _) = self
            .find_type_definition_by_name(&type_def.name)
            .with_context(|| format!("could not find type definition for {}", type_def.name))?;
        let mut serialized_data: Vec<u8> = discriminator.to_vec();

        let mut raw_data = Vec::with_capacity(1024);
        let mut cursor = Cursor::new(&mut raw_data);
        self.serialize_value_with_type_definition(&value, &type_def, &mut cursor)?;

        let bytes_written = cursor.position() as usize;
        serialized_data.extend(&cursor.into_inner()[..bytes_written]);
        Ok(serialized_data)
    }

    pub fn serialize_named_field<T: Write + std::fmt::Debug>(
        &self,
        field_name: &str,
        value: &Value,
        idl_type: &IdlType,
        raw_data: &mut T,
    ) -> anyhow::Result<()> {
        value
            .get(&field_name)
            .with_context(|| format!("Failed to get field {field_name} as unsigned integer"))
            .and_then(|v| self.serialize_value_as_idl_type(v, idl_type, raw_data))
    }

    pub fn serialize_value_with_type_definition<T: Write + std::fmt::Debug>(
        &self,
        value: &Value,
        type_def: &IdlTypeDefinition,
        raw_data: &mut T,
    ) -> anyhow::Result<()> {
        // println!(
        //     "Before serializing {:#?} {:?} we got {:?}",
        //     type_def, value, raw_data,
        // );
        match &type_def.ty {
            IdlTypeDefinitionTy::Struct { fields } => {
                for field in fields {
                    let inner = value
                        .get(&field.name)
                        .ok_or(anyhow!("field not found: {}", field.name))?;
                    self.serialize_value_as_idl_type(inner, &field.ty, raw_data)?;
                }
            }
            IdlTypeDefinitionTy::Enum { variants } => {
                let (variant_name, value) = from_tagged_enum(value)?;
                let (idx, variant) = variants
                    .iter()
                    .enumerate()
                    .find(|(_, item)| &item.name == variant_name)
                    .ok_or(anyhow!(
                        "could not find variant named {variant_name} in enum {}",
                        type_def.name
                    ))?;
                let idx: u8 = idx
                    .try_into()
                    .with_context(|| format!("integer out of bounds for u8: {idx}"))?;
                idx.serialize(raw_data)?;
                if let Some(fields) = &variant.fields {
                    match fields {
                        EnumFields::Named(fields) => {
                            for field in fields {
                                let inner = value
                                    .get(&field.name)
                                    .ok_or(anyhow!("enum field not found: {}", field.name))?;
                                self.serialize_named_field(
                                    &field.name,
                                    inner,
                                    &field.ty,
                                    raw_data,
                                )?;
                            }
                        }
                        EnumFields::Tuple(idl_types) => {
                            for (idx, ty) in idl_types.iter().enumerate() {
                                let inner = value.get(&idx).ok_or(anyhow!(
                                    "enum tuple field index out of bounds: {}",
                                    idx
                                ))?;
                                self.serialize_value_as_idl_type(inner, ty, raw_data)?;
                            }
                        }
                    }
                }
            }
            IdlTypeDefinitionTy::Alias { value: ty } => {
                self.serialize_value_as_idl_type(value, ty, raw_data)?;
            }
        }
        Ok(())
    }

    /// Try to serialize a [Value] to raw byte data based on a given [IdlType].
    pub fn serialize_value_as_idl_type<T: Write + std::fmt::Debug>(
        &self,
        value: &Value,
        idl_type: &IdlType,
        raw_data: &mut T,
    ) -> anyhow::Result<()> {
        // println!(
        //     "Before serializing {:#?} {:?} we got {:?}",
        //     idl_type, value, raw_data,
        // );
        match &idl_type {
            IdlType::Bool => {
                let field = value
                    .as_bool()
                    .with_context(|| format!("Failed to read value as bool"))?;
                field.serialize(raw_data)?;
            }
            IdlType::U8 => serialize_unsigned_int::<u8>(value, raw_data, "u8")?,
            IdlType::U16 => serialize_unsigned_int::<u16>(value, raw_data, "u16")?,
            IdlType::U32 => serialize_unsigned_int::<u32>(value, raw_data, "u32")?,
            IdlType::I8 => serialize_signed_int::<i8>(value, raw_data, "i8")?,
            IdlType::I16 => serialize_signed_int::<i16>(value, raw_data, "i16")?,
            IdlType::I32 => serialize_unsigned_int::<i32>(value, raw_data, "i32")?,
            IdlType::U64 => {
                let field = value
                    .as_u64()
                    .with_context(|| format!("Failed to read value as u64"))?;
                field.serialize(raw_data)?;
            }
            IdlType::I64 => {
                let field = value
                    .as_i64()
                    .with_context(|| format!("Failed to read value as i64"))?;
                field.serialize(raw_data)?;
            }
            IdlType::F32 => {
                let field = value
                    .as_f64()
                    .with_context(|| format!("Failed to read value as f64"))?;
                (field as f32).serialize(raw_data)?;
            }
            IdlType::F64 => {
                let field = value
                    .as_f64()
                    .with_context(|| format!("Failed to read value as f64"))?;
                field.serialize(raw_data)?;
            }
            IdlType::U128 => {
                let field = value
                    .as_str()
                    .with_context(|| format!("Failed to read value as string (u128)"))
                    .and_then(|f| {
                        u128::from_str(f)
                            .with_context(|| format!("failed to parse u128 from &str: {f}"))
                    })?;
                field.serialize(raw_data)?;
            }
            IdlType::I128 => {
                let field = value
                    .as_str()
                    .with_context(|| format!("Failed to read value as string (i128)"))
                    .and_then(|f| {
                        i128::from_str(f)
                            .with_context(|| format!("failed to parse i128 from &str: {f}"))
                    })?;
                field.serialize(raw_data)?;
            }
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
                field.serialize(raw_data)?;
            }
            IdlType::PublicKey => {
                let field = value
                    .as_str()
                    .with_context(|| format!("Failed to read value as string (pubkey)"))
                    .and_then(|f| {
                        Pubkey::from_str(f)
                            .with_context(|| format!("failed to parse pubkey from &str: {f}"))
                    })?;
                field.serialize(raw_data)?;
            }
            IdlType::Defined(defined_type) => {
                let (_, _, type_def) =
                    self.find_type_definition_by_name(&defined_type)
                        .ok_or(anyhow!(
                            "Failed to find type definition for type named: {defined_type}"
                        ))?;
                self.serialize_value_with_type_definition(value, type_def, raw_data)?;
            }
            IdlType::Option(idl_type) => {
                let is_none = value.as_null().is_some();
                if is_none {
                    // println!("Doing the none");
                    self.serialize_none_variant_of_option_t(idl_type, raw_data)?;
                    // println!("{:?}", raw_data);
                } else {
                    true.serialize(raw_data)?;
                    self.serialize_value_as_idl_type(value, idl_type, raw_data)?;
                }
            }
            IdlType::Vec(idl_type) => {
                let arr = value
                    .as_array()
                    .with_context(|| format!("Failed to read value as array"))?;
                let len = arr.len() as u32;
                len.serialize(raw_data)?;
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
        raw_data: &mut impl Write,
    ) -> anyhow::Result<()> {
        false.serialize(raw_data)?;
        raw_data
            .write_all(&vec![0u8; self.len_of_idl_type(idl_type)?])
            .map_err(|e| anyhow!("failed to write zeroed bytes for none variant: {e}"))?;
        // vec![0u8; self.len_of_idl_type(idl_type)?].serialize(raw_data)?;
        Ok(())
    }

    pub fn len_of_idl_type(&self, idl_type: &IdlType) -> anyhow::Result<usize> {
        Ok(match idl_type {
            IdlType::Bool | IdlType::U8 | IdlType::I8 => 1,
            IdlType::U16 | IdlType::I16 => 2,
            IdlType::U32 | IdlType::I32 | IdlType::F32 => 4,
            IdlType::U64 | IdlType::I64 | IdlType::F64 => 8,
            IdlType::U128 | IdlType::I128 => 16,
            //IdlType::U256 | IdlType::I256 => todo!(),
            // Surprisingly, Borsh serializes a `None`-valued `Option<Pubkey>` as a single-byte 0
            IdlType::PublicKey => 0, // 32 seems sane, but in fact does not work,
            IdlType::Option(ty) => 1 + self.len_of_idl_type(ty)?,
            IdlType::Array(ty, len) => len * self.len_of_idl_type(ty)?,
            IdlType::Defined(name) => {
                let (_, ty) = self.get_type_definition_by_name(name).ok_or(anyhow!(
                    "unknown defined type {name} in IDL for program: {}",
                    self.name
                ))?;
                self.len_from_type_definition(ty)?
            }
            IdlType::Vec(ty) => {
                VARIABLE_LENGTH_BORSH_PREFIX
                    + DEFAULT_VARIABLE_LENGTH_BORSH_SPACE * self.len_of_idl_type(ty)?
            }
            // IdlType::Bytes => todo!(),
            IdlType::String => VARIABLE_LENGTH_BORSH_PREFIX + DEFAULT_VARIABLE_LENGTH_BORSH_SPACE,
            // IdlType::GenericLenArray(_, _) => todo!(),
            IdlType::Generic(typename) => {
                self.len_of_generic_type(&typename)? + self.len_of_idl_type(idl_type)?
            }
            // IdlType::DefinedWithTypeArgs { name, args } => {}
            _ => unimplemented!("Type length calculation yet supported: {:?}", idl_type),
        })
    }

    pub fn len_of_generic_type(&self, typename: &str) -> anyhow::Result<usize> {
        if let Ok(ty) = IdlType::from_str(typename) {
            return self.len_of_idl_type(&ty);
        }
        let (_, _, type_def) = self
            .find_type_definition_by_name(typename)
            .ok_or(anyhow!("unknown type named {typename}"))?;
        self.len_from_type_definition(&type_def)
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
                largest_variant_len
            }
            IdlTypeDefinitionTy::Alias { value } => self.len_of_idl_type(value)?,
        })
    }
}

fn serialize_unsigned_int<T: TryFrom<u64> + AnchorSerialize>(
    value: &Value,
    raw_data: &mut impl Write,
    ty_name: &'static str,
) -> anyhow::Result<()> {
    let field = value
        .as_u64()
        .with_context(|| format!("Failed to read value as {ty_name}"))
        .and_then(|f| {
            TryInto::<T>::try_into(f)
                .map_err(|_| anyhow!("integer {f} out of bounds for {ty_name}"))
        })?;
    field.serialize(raw_data)?;
    Ok(())
}

fn serialize_signed_int<T: TryFrom<i64> + AnchorSerialize>(
    value: &Value,
    raw_data: &mut impl Write,
    ty_name: &'static str,
) -> anyhow::Result<()> {
    let field = value
        .as_i64()
        .with_context(|| format!("Failed to read value as {ty_name}"))
        .and_then(|f| {
            TryInto::<T>::try_into(f)
                .map_err(|_| anyhow!("integer {f} out of bounds for {ty_name}"))
        })?;
    field.serialize(raw_data)?;
    Ok(())
}
