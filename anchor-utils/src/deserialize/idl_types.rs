use crate::deserialize::IdlWithDiscriminators;
use anchor_syn::idl::types::{
    EnumFields, IdlEnumVariant, IdlField, IdlType, IdlTypeDefinition, IdlTypeDefinitionTy,
};
use anyhow::anyhow;
use serde_json::{json, Value};
use solana_program::pubkey::Pubkey;

/// Deserialize a data according to a type definition defined
/// in the IDL. This includes accounts, instructions, and auxiliary defined types.
/// See [IdlWithDiscriminators::deserialize_struct_or_enum].
impl IdlWithDiscriminators {
    /// Top level deserialization routine for some data against a target type.
    pub fn deserialize_struct_or_enum(
        &self,
        type_definition: &IdlTypeDefinition,
        data: &mut &[u8],
    ) -> anyhow::Result<Value> {
        match &type_definition.ty {
            IdlTypeDefinitionTy::Struct { fields } => {
                self.deserialize_named_fields(&fields, &mut &data[..])
            }
            IdlTypeDefinitionTy::Enum { variants } => {
                for variant in variants {
                    let IdlEnumVariant { name, fields } = variant;
                    if let Ok(value) = self.deserialize_enum_variant(
                        name.as_str(),
                        &fields.clone(),
                        &mut &data[..],
                    ) {
                        return Ok(value);
                    }
                }
                return Err(anyhow!(
                    "Couldn't deserialize using any of the available enum variants"
                ));
            }
            IdlTypeDefinitionTy::Alias { value } => self.deserialize_idl_type(value, data),
        }
    }

    /// Try to deserialize from raw byte data based on a given [IdlType].
    pub fn deserialize_idl_type(
        &self,
        idl_type: &IdlType,
        raw_data: &mut &[u8],
    ) -> anyhow::Result<Value> {
        match &idl_type {
            IdlType::Bool => {
                let value: bool = borsh::BorshDeserialize::deserialize(raw_data)?;
                return Ok(Value::Bool(value));
            }
            IdlType::U8 => {
                let value: u8 = borsh::BorshDeserialize::deserialize(raw_data)?;
                return Ok(Value::Number(value.into()));
            }
            IdlType::I8 => {
                let value: i8 = borsh::BorshDeserialize::deserialize(raw_data)?;
                return Ok(Value::Number(value.into()));
            }
            IdlType::U16 => {
                let value: u16 = borsh::BorshDeserialize::deserialize(raw_data)?;
                return Ok(Value::Number(value.into()));
            }
            IdlType::I16 => {
                let value: i16 = borsh::BorshDeserialize::deserialize(raw_data)?;
                return Ok(Value::Number(value.into()));
            }
            IdlType::U32 => {
                let value: u32 = borsh::BorshDeserialize::deserialize(raw_data)?;
                return Ok(Value::Number(value.into()));
            }
            IdlType::I32 => {
                let value: i32 = borsh::BorshDeserialize::deserialize(raw_data)?;
                return Ok(Value::Number(value.into()));
            }
            IdlType::F32 => {
                let value: f32 = borsh::BorshDeserialize::deserialize(raw_data)?;
                return Ok(Value::String(value.to_string()));
            }
            IdlType::U64 => {
                let value: u64 = borsh::BorshDeserialize::deserialize(raw_data)?;
                return Ok(Value::Number(value.into()));
            }
            IdlType::I64 => {
                let value: i64 = borsh::BorshDeserialize::deserialize(raw_data)?;
                return Ok(Value::Number(value.into()));
            }
            IdlType::F64 => {
                let value: f64 = borsh::BorshDeserialize::deserialize(raw_data)?;
                return Ok(Value::String(value.to_string()));
            }
            IdlType::U128 => {
                let value: u128 = borsh::BorshDeserialize::deserialize(raw_data)?;
                return Ok(Value::String(value.to_string()));
            }
            IdlType::I128 => {
                let value: i128 = borsh::BorshDeserialize::deserialize(raw_data)?;
                return Ok(Value::String(value.to_string()));
            }
            IdlType::Bytes => {
                let value: Vec<u8> = borsh::BorshDeserialize::deserialize(raw_data)?;
                return Ok(Value::Array(
                    value.iter().map(|v| Value::Number((*v).into())).collect(),
                ));
            }
            IdlType::String => {
                let value: String = borsh::BorshDeserialize::deserialize(raw_data)?;
                return Ok(Value::String(value));
            }
            IdlType::PublicKey => {
                let value: Pubkey = borsh::BorshDeserialize::deserialize(raw_data)?;
                return Ok(Value::String(value.to_string()));
            }
            IdlType::Defined(defined_type) => {
                if let Some((_, ty_def)) = self.find_type_definition_by_name(defined_type) {
                    return self.deserialize_struct_or_enum(ty_def, raw_data);
                }
                return Err(anyhow!("Couldn't find defined type: {}", &defined_type));
            }
            IdlType::Option(idl_type) => {
                let is_some: bool = borsh::BorshDeserialize::deserialize(raw_data)?;
                return if is_some {
                    let value = self.deserialize_idl_type(idl_type, raw_data)?;
                    Ok(Some(value).into())
                } else {
                    Ok(None::<Value>.into())
                };
            }
            IdlType::Vec(idl_type) => {
                let arr_len: u32 = borsh::BorshDeserialize::deserialize(raw_data)?;
                let mut values = vec![];
                for _ in 0..arr_len {
                    values.push(self.deserialize_idl_type(idl_type, raw_data)?);
                }
                return Ok(values.into());
            }
            IdlType::Array(idl_type, arr_len) => {
                let mut values = vec![];
                for _ in 0..*arr_len {
                    values.push(self.deserialize_idl_type(idl_type, raw_data)?);
                }
                return Ok(values.into());
            }
            _ => {
                return Err(anyhow!("U256 and I256 not yet supported"));
            }
        }
    }

    /// Deserialize a collection of named fields,
    /// for example on an Vec, array, or enum tuple-variant.
    pub fn deserialize_named_fields(
        &self,
        fields: &[IdlField],
        data: &mut &[u8],
    ) -> anyhow::Result<Value> {
        let mut map = serde_json::Map::default();
        for field in fields {
            map.insert(
                field.name.clone(),
                self.deserialize_idl_type(&field.ty, data)?,
            );
        }
        return Ok(Value::Object(map.into()));
    }

    /// Deserialize an enum variant,
    /// whether it is a struct variant, a tuple variant, or unit variant.
    pub fn deserialize_enum_variant(
        &self,
        name: &str,
        fields: &Option<EnumFields>,
        data: &mut &[u8],
    ) -> anyhow::Result<Value> {
        if let Some(enum_fields) = fields {
            match enum_fields {
                // A variant with struct fields.
                EnumFields::Named(idl_fields) => {
                    Ok(self.deserialize_named_fields(idl_fields, data)?)
                }
                // A variant with unnamed tuple fields.
                EnumFields::Tuple(idl_types) => {
                    let deserialized = idl_types
                        .iter()
                        .map(|idl_type| self.deserialize_idl_type(idl_type, data))
                        .collect::<anyhow::Result<Vec<_>>>()?;
                    Ok(json!({
                        "name": name,
                        "fields": Value::Array(deserialized)
                    }))
                }
            }
        } else {
            // A variant with no fields.
            Ok(json!({
                "name": name,
                "fields": Value::Null
            }))
        }
    }
}
