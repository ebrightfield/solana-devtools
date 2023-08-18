use crate::deserialize::idl_type_deserializer::TypeDefinitionDeserializer;
use anchor_syn::idl::{Idl, IdlField, IdlType, IdlTypeDefinition};
use anyhow::anyhow;
use serde_json::Value;
use solana_program::pubkey::Pubkey;

/// Matches over the `idl_type`, deserializing each field into a [serde_json::Value].
/// Concrete types are pretty straightforward, but there are caveats:
///
/// - Most numbers deserialize as such, but floats, and [IdlType::U128] and [IdlType::I128]
/// will convert to strings since Borsh can't handle them natively.
/// - We convert [IdlType::Bytes] to a [serde_json::Value::Array] of [serde_json::Value::Number].
/// - Public keys convert to strings.
/// - The generic types Option, Vec, and Array recurse directly back into this function.
/// - The generic type [IdlType::Defined] recurses *indirectly*, because it creates a
///  new [TypeDefinitionDeserializer] and calls the deserialize method on that, which in turn
///  makes potentially many calls into this function. There are *in theory* IDLs that one
///  could structure in a circular fashion and cause infinite recursion here. However,
///  this would have to be quite deliberate, as any such IDL would never correspond to
///  a program that was compilable, as Rust definitions can't be circular.
pub fn deserialize_idl_type(
    idl_type: &IdlType,
    type_defs: &Vec<IdlTypeDefinition>,
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
            for type_def in type_defs {
                if type_def.name == *defined_type {
                    return Ok(TypeDefinitionDeserializer {
                        idl_type_defs: type_defs.clone(),
                        curr_type: type_def.clone(),
                    }
                    .deserialize(raw_data)?);
                }
            }
            return Err(anyhow!("Couldn't find defined type: {}", &defined_type));
        }
        IdlType::Option(idl_type) => {
            let is_some: bool = borsh::BorshDeserialize::deserialize(raw_data)?;
            return if is_some {
                let value = deserialize_idl_type(idl_type, type_defs, raw_data)?;
                Ok(Some(value).into())
            } else {
                Ok(None::<Value>.into())
            }
        }
        IdlType::Vec(idl_type) => {
            let arr_len: u32 = borsh::BorshDeserialize::deserialize(raw_data)?;
            let mut values = vec![];
            for _ in 0..arr_len {
                values.push(deserialize_idl_type(idl_type, type_defs, raw_data)?);
            }
            return Ok(values.into());
        }
        IdlType::Array(idl_type, arr_len) => {
            let mut values = vec![];
            for _ in 0..*arr_len {
                values.push(deserialize_idl_type(idl_type, type_defs, raw_data)?);
            }
            return Ok(values.into());
        },
        _ => {
            return Err(anyhow!("U256 and I256 not yet supported"));
        }
    }
}

/// Mainly for use in an enum tuple-variant, to deserialize
/// its containing data.
pub fn deserialize_idl_types(
    types: &Vec<IdlType>,
    type_defs: &Vec<IdlTypeDefinition>,
    raw_data: &mut &[u8],
) -> anyhow::Result<Vec<Value>> {
    Ok(types
        .iter()
        .map(|idl_type| deserialize_idl_type(idl_type, type_defs, raw_data))
        .into_iter()
        .flatten()
        .collect())
}

/// Deserializes many named fields, indexing them into a [serde_json::map::Map]
/// provided by the caller.
pub fn deserialize_idl_fields(
    fields: &Vec<IdlField>,
    idl: &Idl,
    data: &mut &[u8],
) -> anyhow::Result<Value> {
    let mut map = serde_json::Map::default();
    for field in fields {
        let deserialized = deserialize_idl_type(&field.ty, &idl.types, data)
            .map_err(|e| anyhow!("Failed to deserialize field {:?}, {}", field, e))?;
        map.insert(
            field.name.clone(),
            deserialized,
        );
    }
    return Ok(Value::Object(map.into()));
}
