use crate::deserialize::field::{deserialize_idl_type, deserialize_idl_types};
use anchor_syn::idl::{
    EnumFields, IdlEnumVariant, IdlField, IdlTypeDefinition, IdlTypeDefinitionTy,
};
use anyhow::anyhow;
use serde_json::{json, Value};

/// Performs a top-down, iterated deserialization over the type-tree
/// of a given [IdlTypeDefinition]. We include a Vec of additional
/// [IdlTypeDefinition] objects to resolve any `T` in `IdlType::Defined<T>`.
pub struct TypeDefinitionDeserializer {
    pub idl_type_defs: Vec<IdlTypeDefinition>,
    pub curr_type: IdlTypeDefinition,
}

impl TypeDefinitionDeserializer {
    /// Deserialize a data according to a custom type definition defined
    /// in the IDL. This includes accounts, instructions, and auxiliary defined types.
    pub fn deserialize(self, data: &mut &[u8]) -> anyhow::Result<Value> {
        match self.curr_type.ty.clone() {
            IdlTypeDefinitionTy::Struct { fields } => {
                self.deserialize_idl_fields(&fields.clone(), data)
            }
            IdlTypeDefinitionTy::Enum { variants } => {
                for variant in variants {
                    let IdlEnumVariant { name, fields } = variant;
                    if let Ok(value) = self.deserialize_enum_field(name, &fields.clone(), data) {
                        return Ok(value);
                    }
                }
                return Err(anyhow!(
                    "Couldn't deserialize using any of the available enum variants"
                ));
            }
        }
    }

    fn deserialize_idl_fields(
        &self,
        fields: &Vec<IdlField>,
        data: &mut &[u8],
    ) -> anyhow::Result<Value> {
        let mut map = serde_json::Map::default();
        for field in fields {
            map.insert(
                field.name.clone(),
                deserialize_idl_type(&field.ty, &self.idl_type_defs, data)?,
            );
        }
        return Ok(Value::Object(map.into()));
    }

    fn deserialize_enum_field(
        &self,
        name: String,
        fields: &Option<EnumFields>,
        data: &mut &[u8],
    ) -> anyhow::Result<Value> {
        if let Some(enum_fields) = fields {
            match enum_fields {
                // A variant with struct fields.
                EnumFields::Named(idl_fields) => Ok(self.deserialize_idl_fields(idl_fields, data)?),
                // A variant with unnamed tuple fields.
                EnumFields::Tuple(idl_types) => {
                    let mut data = data.clone();
                    let deserialized =
                        deserialize_idl_types(idl_types, &self.idl_type_defs, &mut data)?;
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
