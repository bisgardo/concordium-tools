use anyhow::{Context, Result};
use concordium_contracts_common::{
    from_bytes,
    schema::{
        ContractV0, ContractV1, ContractV2, ContractV3, FunctionV1, FunctionV2, Type,
        VersionedModuleSchema,
    },
};
use rocket::serde::{Serialize, Serializer};
use serde_json::Value;
use std::collections::BTreeMap;

struct SerializableType(Type);

// TODO serialize into '{"kind":"...", <params>}'.
impl Serialize for SerializableType {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        match &self.0 {
            Type::Unit => serializer.serialize_str("unit"),
            Type::Bool => serializer.serialize_str("bool"),
            Type::U8 => serializer.serialize_str("u8"),
            Type::U16 => serializer.serialize_str("u16"),
            Type::U32 => serializer.serialize_str("u32"),
            Type::U64 => serializer.serialize_str("u64"),
            Type::U128 => serializer.serialize_str("u128"),
            Type::I8 => serializer.serialize_str("i8"),
            Type::I16 => serializer.serialize_str("i16"),
            Type::I32 => serializer.serialize_str("i32"),
            Type::I64 => serializer.serialize_str("i64"),
            Type::I128 => serializer.serialize_str("i128"),
            Type::Amount => serializer.serialize_str("amount"),
            Type::AccountAddress => serializer.serialize_str("account_address"),
            Type::ContractAddress => serializer.serialize_str("contract_address"),
            Type::Timestamp => serializer.serialize_str("timestamp"),
            Type::Duration => serializer.serialize_str("duration"),
            Type::Pair(_, _) => serializer.serialize_str("pair..."),
            Type::List(_, _) => serializer.serialize_str("list..."),
            Type::Set(_, _) => serializer.serialize_str("set..."),
            Type::Map(_, _, _) => serializer.serialize_str("map..."),
            Type::Array(_, _) => serializer.serialize_str("array..."),
            Type::Struct(_) => serializer.serialize_str("struct..."),
            Type::Enum(_) => serializer.serialize_str("enum..."),
            Type::String(_) => serializer.serialize_str("string..."),
            Type::ContractName(_) => serializer.serialize_str("contract_name..."),
            Type::ReceiveName(_) => serializer.serialize_str("receive_name..."),
            Type::ULeb128(_) => serializer.serialize_str("uleb128..."),
            Type::ILeb128(_) => serializer.serialize_str("ileb128..."),
            Type::ByteList(_) => serializer.serialize_str("byte_list..."),
            Type::ByteArray(_) => serializer.serialize_str("byte_array..."),
            Type::TaggedEnum(_) => serializer.serialize_str("tagged_enum..."),
        }
    }
}

/// Versioned schemas always start with two fully set bytes.
/// This is used to determine whether we are looking at a versioned or
/// unversioned (old) schemas.
const VERSIONED_SCHEMA_MAGIC_HASH: &[u8] = &[0xff, 0xff];

#[derive(Debug, Clone, Copy)]
pub enum WasmVersion {
    V0,
    V1,
}

pub fn parse_schema(
    wasm_version: Option<WasmVersion>,
    bytes: &[u8],
) -> Result<VersionedModuleSchema> {
    Ok(if bytes.starts_with(VERSIONED_SCHEMA_MAGIC_HASH) {
        from_bytes::<VersionedModuleSchema>(bytes)?
    } else if let Some(wv) = wasm_version {
        match wv {
            WasmVersion::V0 => from_bytes(bytes).map(VersionedModuleSchema::V0)?,
            WasmVersion::V1 => from_bytes(bytes).map(VersionedModuleSchema::V1)?,
        }
    } else {
        anyhow::bail!("Legacy unversioned schema was supplied, but no version was provided.");
    })
}

pub fn schema_to_json(schema: &VersionedModuleSchema) -> Result<Value> {
    let map = match schema {
        VersionedModuleSchema::V0(module_schema) => {
            try_map_values(&module_schema.contracts, schema_to_json_v0)
        }
        VersionedModuleSchema::V1(module_schema) => {
            try_map_values(&module_schema.contracts, schema_to_json_v1)
        }
        VersionedModuleSchema::V2(module_schema) => {
            try_map_values(&module_schema.contracts, schema_to_json_v2)
        }
        VersionedModuleSchema::V3(module_schema) => {
            try_map_values(&module_schema.contracts, schema_to_json_v3)
        }
    }?;
    serde_json::to_value(map).context("cannot convert result to JSON")
}

fn try_map_values<K: Ord, V, W>(
    map: &BTreeMap<K, V>,
    f: fn(&V) -> Result<W>,
) -> Result<BTreeMap<&K, W>> {
    map.into_iter().map(|(k, v)| Ok((k, f(v)?))).collect()
}

/// Converts the ContractV0 schema of the given contract_name to JSON and writes
/// it to a file named after the smart contract name at the specified location.
fn schema_to_json_v0(contract_schema: &ContractV0) -> Result<Value> {
    // create empty schema_json
    let mut schema_json: Value = Value::Object(serde_json::Map::new());

    // add init schema
    if let Some(init_schema) = &contract_schema.init {
        schema_json["init"] = type_to_json(init_schema)?;
    }

    // add state schema
    if let Some(state_schema) = &contract_schema.state {
        schema_json["state"] = type_to_json(state_schema)?;
    }

    // add receive entrypoints
    if !contract_schema.receive.is_empty() {
        // create empty entrypoints
        let mut entrypoints: Value = Value::Object(serde_json::Map::new());

        // iterate through the entrypoints and add their schemas
        for (method_name, receive_schema) in contract_schema.receive.iter() {
            // add `method_name` entrypoint
            entrypoints[method_name] = type_to_json(receive_schema)?;
        }

        // add all receive entrypoints
        schema_json["entrypoints"] = entrypoints;
    }

    Ok(schema_json)
}

fn function_v1_schema(schema: &FunctionV1) -> Result<Value> {
    // create empty function object
    let mut function_object: Value = Value::Object(serde_json::Map::new());

    // add parameter schema to function object
    if let Some(parameter_schema) = &schema.parameter() {
        function_object["parameter"] = type_to_json(parameter_schema)?;
    }

    // add return_value schema to function object
    if let Some(return_value_schema) = &schema.return_value() {
        function_object["returnValue"] = type_to_json(return_value_schema)?;
    }
    Ok(function_object)
}

/// Converts the ContractV1 schema of the given contract_name to JSON and writes
/// it to a file named after the smart contract name at the specified location.
fn schema_to_json_v1(contract_schema: &ContractV1) -> Result<Value> {
    // create empty schema_json
    let mut schema_json: Value = Value::Object(serde_json::Map::new());

    // add init schema
    if let Some(init_schema) = &contract_schema.init {
        schema_json["init"] = function_v1_schema(init_schema)?;
    }

    // add receive entrypoints
    if !contract_schema.receive.is_empty() {
        // create empty entrypoints
        let mut entrypoints: Value = Value::Object(serde_json::Map::new());

        // iterate through the entrypoints and add their schemas
        for (method_name, receive_schema) in contract_schema.receive.iter() {
            // add `method_name` entrypoint
            entrypoints[method_name] = function_v1_schema(receive_schema)?;
        }

        // add all receive entrypoints
        schema_json["entrypoints"] = entrypoints;
    }

    Ok(schema_json)
}

fn type_to_json(t: &Type) -> Result<Value> {
    serde_json::to_value(SerializableType(t.clone()))
        .context(format!("cannot serialize type {:?} into JSON", t))
}

/// Convert a [`FunctionV2`] schema to a JSON representation.
fn function_v2_schema(schema: &FunctionV2) -> Result<Value> {
    // create empty object
    let mut function_object: Value = Value::Object(serde_json::Map::new());

    // add parameter schema
    if let Some(parameter_schema) = &schema.parameter {
        function_object["parameter"] = type_to_json(parameter_schema)?;
    }

    // add return_value schema
    if let Some(return_value_schema) = &schema.return_value {
        function_object["returnValue"] = type_to_json(return_value_schema)?;
    }

    // add error schema
    if let Some(error_schema) = &schema.error {
        function_object["error"] = type_to_json(error_schema)?;
    }
    Ok(function_object)
}

/// Converts the ContractV2 schema of the given contract_name to JSON and writes
/// it to a file named after the smart contract name at the specified location.
fn schema_to_json_v2(contract_schema: &ContractV2) -> Result<Value> {
    // create empty schema_json
    let mut schema_json: Value = Value::Object(serde_json::Map::new());

    // add init schema
    if let Some(init_schema) = &contract_schema.init {
        schema_json["init"] = function_v2_schema(init_schema)?;
    }

    // add receive entrypoints
    if !contract_schema.receive.is_empty() {
        // create empty entrypoints
        let mut entrypoints: Value = Value::Object(serde_json::Map::new());

        // iterate through the entrypoints and add their schemas
        for (method_name, receive_schema) in contract_schema.receive.iter() {
            // add `method_name` entrypoint
            entrypoints[method_name] = function_v2_schema(receive_schema)?;
        }

        // add all receive entrypoints
        schema_json["entrypoints"] = entrypoints;
    }

    Ok(schema_json)
}

fn schema_to_json_v3(contract_schema: &ContractV3) -> Result<Value> {
    // create empty schema_json
    let mut schema_json: Value = Value::Object(serde_json::Map::new());

    // add init schema
    if let Some(init_schema) = &contract_schema.init {
        schema_json["init"] = function_v2_schema(init_schema)?;
    }

    // add event schema
    if let Some(event_schema) = &contract_schema.event {
        schema_json["event"] = type_to_json(event_schema)?;
    }

    // add receive entrypoints
    if !contract_schema.receive.is_empty() {
        // create empty entrypoints
        let mut entrypoints: Value = Value::Object(serde_json::Map::new());

        // iterate through the entrypoints and add their schemas
        for (method_name, receive_schema) in contract_schema.receive.iter() {
            // add `method_name` entrypoint
            entrypoints[method_name] = function_v2_schema(receive_schema)?;
        }

        // add all receive entrypoints
        schema_json["entrypoints"] = entrypoints;
    }

    Ok(schema_json)
}
