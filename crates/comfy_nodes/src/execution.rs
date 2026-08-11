use crate::{
    NativeDescriptorSchemaMetadata, NativeInputSchemaMetadata, NativeSchemaValue,
    NativeStoredPayload, NativeStoredPayloadError,
};
use comfy_media::{
    MetadataWritePolicy, PngError, PngLimits, encode_png_frame_with_policy_and_context,
};
use comfy_tensor::{
    CpuBackend, ExecutionContext, ImageTensor, MAX_SHADER_OUTPUTS, MAX_SHADER_PASSES,
    NativeShaderError, NativeShaderExecutor, NativeShaderRequest, NativeShaderResult,
    ScratchBindingIdentity, ScratchReservation, StreamId, TensorError,
};
use comfy_types::{ApiPrompt, AttemptId, CancellationToken, NodeId, PromptId};
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    sync::Arc,
};
use thiserror::Error;
use uuid::Uuid;

pub const NATIVE_NODE_CONTRACT_SCHEMA_VERSION: u16 = 2;
pub const LEGACY_NATIVE_NODE_CONTRACT_SCHEMA_VERSION: u16 = 1;
pub const NATIVE_OPAQUE_HANDLE_SCHEMA_VERSION: u16 = 1;
pub const NATIVE_STRUCTURED_VALUE_SCHEMA_VERSION: u16 = 1;

const MAX_IDENTIFIER_BYTES: usize = 4_096;
const MAX_TEXT_BYTES: usize = 1024 * 1024;
const MAX_UNKNOWN_BYTES: usize = 1024 * 1024;
const MAX_VALUE_DEPTH: usize = 32;
const MAX_LIST_VALUES: usize = 1_000_000;
const MAX_PORTS: usize = 65_536;
const MAX_TYPE_UNION_MEMBERS: usize = 128;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativePrimitiveType {
    Null,
    Boolean,
    Integer,
    Number,
    String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum NativePrimitive {
    Null,
    Boolean(bool),
    Integer(i64),
    UnsignedInteger(u64),
    Number(f64),
    String(String),
}

impl NativePrimitive {
    pub const fn primitive_type(&self) -> NativePrimitiveType {
        match self {
            Self::Null => NativePrimitiveType::Null,
            Self::Boolean(_) => NativePrimitiveType::Boolean,
            Self::Integer(_) | Self::UnsignedInteger(_) => NativePrimitiveType::Integer,
            Self::Number(_) => NativePrimitiveType::Number,
            Self::String(_) => NativePrimitiveType::String,
        }
    }

    fn validate(&self) -> Result<(), NativeNodeContractError> {
        match self {
            Self::Number(value) if !value.is_finite() => {
                Err(NativeNodeContractError::NonFiniteNumber)
            }
            Self::String(value) => validate_workflow_text("primitive string", value),
            _ => Ok(()),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeHandleKind {
    Tensor,
    Model,
    Clip,
    Vae,
    ControlNet,
    Conditioning,
    Latent,
    Image,
    Mask,
    Audio,
    Video,
    ThreeD,
    Artifact,
    ProviderTask,
    StructuredCompute,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeHandleType {
    pub kind: NativeHandleKind,
    pub type_id: String,
}

impl NativeHandleType {
    pub fn new(
        kind: NativeHandleKind,
        type_id: impl Into<String>,
    ) -> Result<Self, NativeNodeContractError> {
        let handle_type = Self {
            kind,
            type_id: type_id.into(),
        };
        handle_type.validate()?;
        Ok(handle_type)
    }

    pub fn validate(&self) -> Result<(), NativeNodeContractError> {
        if self.type_id.is_empty()
            || self.type_id.len() > MAX_IDENTIFIER_BYTES
            || !self.type_id.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':' | b'@')
            })
        {
            return Err(NativeNodeContractError::InvalidHandleType);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeHandleStoreIdentity {
    pub store_id: Uuid,
    pub generation_id: Uuid,
}

impl NativeHandleStoreIdentity {
    pub fn new(store_id: Uuid, generation_id: Uuid) -> Result<Self, NativeNodeContractError> {
        let identity = Self {
            store_id,
            generation_id,
        };
        identity.validate()?;
        Ok(identity)
    }

    pub fn validate(&self) -> Result<(), NativeNodeContractError> {
        if self.store_id.is_nil() || self.generation_id.is_nil() {
            return Err(NativeNodeContractError::InvalidHandleStoreIdentity);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeOpaqueHandle {
    schema_version: u16,
    handle_type: NativeHandleType,
    store_identity: NativeHandleStoreIdentity,
    identifier: String,
    generation: u64,
    digest_sha256: Option<String>,
}

impl NativeOpaqueHandle {
    pub fn new(
        handle_type: NativeHandleType,
        store_identity: NativeHandleStoreIdentity,
        identifier: impl Into<String>,
        generation: u64,
        digest_sha256: Option<String>,
    ) -> Result<Self, NativeNodeContractError> {
        let handle = Self {
            schema_version: NATIVE_OPAQUE_HANDLE_SCHEMA_VERSION,
            handle_type,
            store_identity,
            identifier: identifier.into(),
            generation,
            digest_sha256,
        };
        handle.validate()?;
        Ok(handle)
    }

    pub fn handle_type(&self) -> &NativeHandleType {
        &self.handle_type
    }

    pub const fn store_identity(&self) -> NativeHandleStoreIdentity {
        self.store_identity
    }

    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    pub const fn generation(&self) -> u64 {
        self.generation
    }

    pub fn digest_sha256(&self) -> Option<&str> {
        self.digest_sha256.as_deref()
    }

    pub fn validate(&self) -> Result<(), NativeNodeContractError> {
        if self.schema_version != NATIVE_OPAQUE_HANDLE_SCHEMA_VERSION {
            return Err(NativeNodeContractError::UnsupportedHandleSchema(
                self.schema_version,
            ));
        }
        self.handle_type.validate()?;
        self.store_identity.validate()?;
        validate_identifier("opaque handle identifier", &self.identifier)?;
        if self.generation == 0 {
            return Err(NativeNodeContractError::InvalidHandleGeneration);
        }
        if let Some(digest) = &self.digest_sha256
            && !valid_sha256(digest)
        {
            return Err(NativeNodeContractError::InvalidDigest);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NativeValue {
    Primitive { value: NativePrimitive },
    Handle { value: NativeOpaqueHandle },
    List { values: Vec<NativeValue> },
    PreservedUnknown { type_name: String, value: Value },
}

#[derive(Clone, Debug, PartialEq)]
pub struct NativeStructuredValue {
    type_name: String,
    fields: BTreeMap<String, NativeValue>,
}

impl NativeStructuredValue {
    const MARKER: &'static str = "sim.native-structured-value@1";

    pub fn checked(
        type_name: impl Into<String>,
        fields: BTreeMap<String, NativeValue>,
    ) -> Result<Self, NativeNodeContractError> {
        let value = Self {
            type_name: type_name.into(),
            fields,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn type_name(&self) -> &str {
        &self.type_name
    }

    pub fn fields(&self) -> &BTreeMap<String, NativeValue> {
        &self.fields
    }

    pub fn get(&self, name: &str) -> Option<&NativeValue> {
        self.fields.get(name)
    }

    pub fn validate(&self) -> Result<(), NativeNodeContractError> {
        validate_identifier("structured value type", &self.type_name)?;
        if self.fields.len() > MAX_LIST_VALUES {
            return Err(NativeNodeContractError::TooManyListValues);
        }
        for (name, value) in &self.fields {
            validate_identifier("structured value field", name)?;
            value.validate()?;
        }
        Ok(())
    }

    pub fn into_native_value(self) -> NativeValue {
        let fields = self
            .fields
            .into_iter()
            .map(|(name, value)| NativeValue::List {
                values: vec![
                    NativeValue::Primitive {
                        value: NativePrimitive::String(name),
                    },
                    value,
                ],
            })
            .collect();
        NativeValue::List {
            values: vec![
                NativeValue::Primitive {
                    value: NativePrimitive::String(Self::MARKER.to_owned()),
                },
                NativeValue::Primitive {
                    value: NativePrimitive::String(self.type_name),
                },
                NativeValue::List { values: fields },
            ],
        }
    }

    pub fn from_native_value(value: &NativeValue) -> Result<Option<Self>, NativeNodeContractError> {
        let NativeValue::List { values } = value else {
            return Ok(None);
        };
        let marker = match values.first() {
            Some(NativeValue::Primitive {
                value: NativePrimitive::String(marker),
            }) => marker,
            _ => return Ok(None),
        };
        if marker != Self::MARKER {
            return Ok(None);
        }
        if values.len() != 3 {
            return Err(NativeNodeContractError::InvalidStructuredValue);
        }
        let type_name = match values.get(1) {
            Some(NativeValue::Primitive {
                value: NativePrimitive::String(type_name),
            }) => type_name.clone(),
            _ => return Err(NativeNodeContractError::InvalidStructuredValue),
        };
        let field_values = match values.get(2) {
            Some(NativeValue::List { values }) => values,
            _ => return Err(NativeNodeContractError::InvalidStructuredValue),
        };
        let mut fields = BTreeMap::new();
        for field in field_values {
            let NativeValue::List { values } = field else {
                return Err(NativeNodeContractError::InvalidStructuredValue);
            };
            let name = match values.as_slice() {
                [
                    NativeValue::Primitive {
                        value: NativePrimitive::String(name),
                    },
                    _,
                ] => name.clone(),
                _ => return Err(NativeNodeContractError::InvalidStructuredValue),
            };
            let value = values
                .get(1)
                .cloned()
                .ok_or(NativeNodeContractError::InvalidStructuredValue)?;
            if fields.insert(name, value).is_some() {
                return Err(NativeNodeContractError::InvalidStructuredValue);
            }
        }
        Self::checked(type_name, fields).map(Some)
    }

    pub fn into_runtime_value(self) -> Result<NativeValue, NativeNodeContractError> {
        if let Some(value) = self.as_json_value()? {
            return Ok(NativeValue::PreservedUnknown {
                type_name: self.type_name,
                value,
            });
        }
        Ok(self.into_native_value())
    }

    fn as_json_value(&self) -> Result<Option<Value>, NativeNodeContractError> {
        let mut fields = serde_json::Map::new();
        for (name, value) in &self.fields {
            let Some(value) = native_value_as_json(value)? else {
                return Ok(None);
            };
            fields.insert(name.clone(), value);
        }
        Ok(Some(Value::Object(fields)))
    }
}

fn native_value_as_json(value: &NativeValue) -> Result<Option<Value>, NativeNodeContractError> {
    Ok(Some(match value {
        NativeValue::Primitive { value } => match value {
            NativePrimitive::Null => Value::Null,
            NativePrimitive::Boolean(value) => Value::Bool(*value),
            NativePrimitive::Integer(value) => Value::Number((*value).into()),
            NativePrimitive::UnsignedInteger(value) => Value::Number((*value).into()),
            NativePrimitive::Number(value) => {
                let Some(value) = serde_json::Number::from_f64(*value) else {
                    return Err(NativeNodeContractError::InvalidStructuredValue);
                };
                Value::Number(value)
            }
            NativePrimitive::String(value) => Value::String(value.clone()),
        },
        NativeValue::Handle { .. } => return Ok(None),
        NativeValue::List { values } => {
            if let Some(structured) = NativeStructuredValue::from_native_value(value)? {
                let Some(value) = structured.as_json_value()? else {
                    return Ok(None);
                };
                value
            } else {
                let mut result = Vec::with_capacity(values.len());
                for value in values {
                    let Some(value) = native_value_as_json(value)? else {
                        return Ok(None);
                    };
                    result.push(value);
                }
                Value::Array(result)
            }
        }
        NativeValue::PreservedUnknown { value, .. } => value.clone(),
    }))
}

impl NativeValue {
    pub fn validate(&self) -> Result<(), NativeNodeContractError> {
        self.validate_at_depth(0)
    }

    fn validate_at_depth(&self, depth: usize) -> Result<(), NativeNodeContractError> {
        if depth > MAX_VALUE_DEPTH {
            return Err(NativeNodeContractError::ValueNestingTooDeep);
        }
        match self {
            Self::Primitive { value } => value.validate(),
            Self::Handle { value } => value.validate(),
            Self::List { values } => {
                if values.len() > MAX_LIST_VALUES {
                    return Err(NativeNodeContractError::TooManyListValues);
                }
                for value in values {
                    value.validate_at_depth(depth.saturating_add(1))?;
                }
                Ok(())
            }
            Self::PreservedUnknown { type_name, value } => {
                validate_identifier("preserved unknown type", type_name)?;
                let encoded = serde_json::to_vec(value)
                    .map_err(NativeNodeContractError::EncodePreservedUnknown)?;
                if encoded.len() > MAX_UNKNOWN_BYTES {
                    return Err(NativeNodeContractError::PreservedUnknownTooLarge);
                }
                Ok(())
            }
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum NativeValueType {
    Any,
    Primitive(NativePrimitiveType),
    Handle(NativeHandleType),
    PreservedUnknown,
    NamedPreservedUnknown(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NativeTypeUnion(Vec<NativeValueType>);

impl NativeTypeUnion {
    pub fn new(
        members: impl IntoIterator<Item = NativeValueType>,
    ) -> Result<Self, NativeNodeContractError> {
        let members = members.into_iter().collect::<Vec<_>>();
        let value = Self(members);
        value.validate()?;
        Ok(value)
    }

    pub fn members(&self) -> &[NativeValueType] {
        &self.0
    }

    pub fn accepts(&self, value: &NativeValue) -> bool {
        self.0.iter().any(|member| match (member, value) {
            (NativeValueType::Any, _) => true,
            (NativeValueType::Primitive(expected), NativeValue::Primitive { value }) => {
                *expected == value.primitive_type()
            }
            (NativeValueType::Handle(expected), NativeValue::Handle { value }) => {
                crate::native_handle_type_accepts(expected, value.handle_type())
            }
            (NativeValueType::PreservedUnknown, NativeValue::PreservedUnknown { .. }) => true,
            (
                NativeValueType::NamedPreservedUnknown(expected),
                NativeValue::PreservedUnknown { type_name, .. },
            ) => expected == type_name,
            (NativeValueType::PreservedUnknown, value) => {
                NativeStructuredValue::from_native_value(value)
                    .ok()
                    .flatten()
                    .is_some()
            }
            (NativeValueType::NamedPreservedUnknown(expected), value) => {
                NativeStructuredValue::from_native_value(value)
                    .ok()
                    .flatten()
                    .is_some_and(|structured| structured.type_name == *expected)
            }
            _ => false,
        })
    }

    pub fn validate(&self) -> Result<(), NativeNodeContractError> {
        if self.0.is_empty() || self.0.len() > MAX_TYPE_UNION_MEMBERS {
            return Err(NativeNodeContractError::InvalidTypeUnion);
        }
        if !self.0.windows(2).all(|pair| pair[0] < pair[1]) {
            return Err(NativeNodeContractError::InvalidTypeUnion);
        }
        if self.0.contains(&NativeValueType::Any) && self.0.len() != 1 {
            return Err(NativeNodeContractError::InvalidTypeUnion);
        }
        for member in &self.0 {
            if let NativeValueType::NamedPreservedUnknown(type_name) = member {
                validate_identifier("preserved unknown value type", type_name)?;
            }
        }
        Ok(())
    }
}

pub fn native_value_matches_input_schema(
    value: &NativeValue,
    schema: &NativeInputSchemaMetadata,
) -> bool {
    match value {
        NativeValue::List { values } => values
            .iter()
            .all(|value| native_value_matches_input_schema(value, schema)),
        _ => {
            let Some(schema_value) = native_value_as_schema_value(value) else {
                return schema.choices.is_empty()
                    && schema.minimum.is_none()
                    && schema.maximum.is_none();
            };
            if !schema.choices.is_empty()
                && !schema
                    .choices
                    .iter()
                    .any(|choice| schema_values_equal(&schema_value, choice))
            {
                return false;
            }
            schema.minimum.as_ref().is_none_or(|minimum| {
                // Preserved source expressions are provenance, not executable numeric bounds.
                matches!(minimum, NativeSchemaValue::PreservedExpression { .. })
                    || schema_number_at_least(&schema_value, minimum)
            }) && schema.maximum.as_ref().is_none_or(|maximum| {
                matches!(maximum, NativeSchemaValue::PreservedExpression { .. })
                    || schema_number_at_most(&schema_value, maximum)
            })
        }
    }
}

fn schema_values_equal(left: &NativeSchemaValue, right: &NativeSchemaValue) -> bool {
    left == right
        || compare_schema_numbers(left, right)
            .is_some_and(|ordering| ordering == std::cmp::Ordering::Equal)
}

fn native_value_as_schema_value(value: &NativeValue) -> Option<NativeSchemaValue> {
    let NativeValue::Primitive { value } = value else {
        return None;
    };
    match value {
        NativePrimitive::Null => Some(NativeSchemaValue::Null),
        NativePrimitive::Boolean(value) => Some(NativeSchemaValue::Boolean { value: *value }),
        NativePrimitive::Integer(value) => Some(NativeSchemaValue::SignedInteger { value: *value }),
        NativePrimitive::UnsignedInteger(value) => {
            Some(NativeSchemaValue::UnsignedInteger { value: *value })
        }
        NativePrimitive::Number(value) if value.is_finite() => {
            Some(NativeSchemaValue::FiniteDecimal {
                value: value.to_string(),
            })
        }
        NativePrimitive::Number(_) => None,
        NativePrimitive::String(value) => Some(NativeSchemaValue::String {
            value: value.clone(),
        }),
    }
}

fn native_value_from_schema_value(value: &NativeSchemaValue) -> Option<NativeValue> {
    Some(match value {
        NativeSchemaValue::Null => NativeValue::Primitive {
            value: NativePrimitive::Null,
        },
        NativeSchemaValue::Boolean { value } => NativeValue::Primitive {
            value: NativePrimitive::Boolean(*value),
        },
        NativeSchemaValue::SignedInteger { value } => NativeValue::Primitive {
            value: NativePrimitive::Integer(*value),
        },
        NativeSchemaValue::UnsignedInteger { value } => NativeValue::Primitive {
            value: NativePrimitive::UnsignedInteger(*value),
        },
        NativeSchemaValue::FiniteDecimal { value } => NativeValue::Primitive {
            value: NativePrimitive::Number(
                value
                    .parse::<f64>()
                    .ok()
                    .filter(|value| value.is_finite())?,
            ),
        },
        NativeSchemaValue::String { value } => NativeValue::Primitive {
            value: NativePrimitive::String(value.clone()),
        },
        NativeSchemaValue::List { values } => NativeValue::List {
            values: values
                .iter()
                .map(native_value_from_schema_value)
                .collect::<Option<Vec<_>>>()?,
        },
        NativeSchemaValue::Object { .. } | NativeSchemaValue::PreservedExpression { .. } => {
            return None;
        }
    })
}

fn native_type_union_accepts_value(union: &NativeTypeUnion, value: &NativeValue) -> bool {
    union.accepts(value)
        || union.members() == [NativeValueType::Any]
        || matches!(
            value,
            NativeValue::Primitive {
                value: NativePrimitive::Integer(_) | NativePrimitive::UnsignedInteger(_)
            }
        ) && union
            .members()
            .contains(&NativeValueType::Primitive(NativePrimitiveType::Number))
}

fn schema_number_at_least(value: &NativeSchemaValue, minimum: &NativeSchemaValue) -> bool {
    compare_schema_numbers(value, minimum).is_some_and(|ordering| !ordering.is_lt())
}

fn schema_number_at_most(value: &NativeSchemaValue, maximum: &NativeSchemaValue) -> bool {
    compare_schema_numbers(value, maximum).is_some_and(|ordering| !ordering.is_gt())
}

fn compare_schema_numbers(
    left: &NativeSchemaValue,
    right: &NativeSchemaValue,
) -> Option<std::cmp::Ordering> {
    use std::cmp::Ordering;
    match (left, right) {
        (
            NativeSchemaValue::SignedInteger { value: left },
            NativeSchemaValue::SignedInteger { value: right },
        ) => Some(left.cmp(right)),
        (
            NativeSchemaValue::UnsignedInteger { value: left },
            NativeSchemaValue::UnsignedInteger { value: right },
        ) => Some(left.cmp(right)),
        (
            NativeSchemaValue::SignedInteger { value: left },
            NativeSchemaValue::UnsignedInteger { value: right },
        ) => {
            if *left < 0 {
                Some(Ordering::Less)
            } else {
                Some((*left as u64).cmp(right))
            }
        }
        (
            NativeSchemaValue::UnsignedInteger { value: left },
            NativeSchemaValue::SignedInteger { value: right },
        ) => {
            if *right < 0 {
                Some(Ordering::Greater)
            } else {
                Some(left.cmp(&(*right as u64)))
            }
        }
        (NativeSchemaValue::FiniteDecimal { value: left }, right) => {
            compare_finite_decimal(left, right)
        }
        (left, NativeSchemaValue::FiniteDecimal { value: right }) => {
            compare_finite_decimal(right, left).map(Ordering::reverse)
        }
        _ => None,
    }
}

fn compare_finite_decimal(value: &str, other: &NativeSchemaValue) -> Option<std::cmp::Ordering> {
    const MAX_EXACT_F64_INTEGER: u64 = 1 << 53;
    let value = value
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())?;
    let other = match other {
        NativeSchemaValue::SignedInteger { value } => {
            if value.unsigned_abs() > MAX_EXACT_F64_INTEGER {
                return None;
            }
            *value as f64
        }
        NativeSchemaValue::UnsignedInteger { value } => {
            if *value > MAX_EXACT_F64_INTEGER {
                return None;
            }
            *value as f64
        }
        NativeSchemaValue::FiniteDecimal { value } => value
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())?,
        _ => return None,
    };
    value.partial_cmp(&other)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativePortCardinality {
    Scalar,
    List,
    Mapped,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeInputDescriptor {
    pub name: String,
    pub accepted_types: NativeTypeUnion,
    pub required: bool,
    pub hidden: bool,
    pub lazy: bool,
    pub cardinality: NativePortCardinality,
    pub allows_literal: bool,
}

impl NativeInputDescriptor {
    fn validate(&self) -> Result<(), NativeNodeContractError> {
        validate_identifier("input name", &self.name)?;
        self.accepted_types.validate()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeDynamicInputDescriptor {
    pub name_template: String,
    pub start_index: u32,
    pub minimum_count: u32,
    pub maximum_count: u32,
    pub input: NativeInputDescriptor,
}

impl NativeDynamicInputDescriptor {
    fn validate(&self) -> Result<(), NativeNodeContractError> {
        validate_text(
            "dynamic input name template",
            &self.name_template,
            MAX_IDENTIFIER_BYTES,
            false,
        )?;
        let indexed = self.name_template.matches("{index}").count() == 1
            && !self.name_template.contains("{name}");
        let named = self.name_template == "{name}";
        if !(indexed || named) || self.minimum_count > self.maximum_count || self.maximum_count == 0
        {
            return Err(NativeNodeContractError::InvalidDynamicInput);
        }
        if indexed {
            self.start_index
                .checked_add(self.maximum_count)
                .ok_or(NativeNodeContractError::InvalidDynamicInput)?;
        }
        self.input.validate()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeOutputDescriptor {
    pub name: String,
    pub produced_type: NativeValueType,
    pub is_list: bool,
}

impl NativeOutputDescriptor {
    fn validate(&self) -> Result<(), NativeNodeContractError> {
        validate_identifier("output name", &self.name)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeEffectClass {
    Pure,
    ReadsArtifact,
    WritesArtifact,
    Provider,
    ExclusiveDevice,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeCachePolicy {
    InputIdentity,
    Never,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeNodeDescriptor {
    pub schema_version: u16,
    pub class_type: String,
    pub implementation_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_schema: Option<NativeDescriptorSchemaMetadata>,
    pub inputs: Vec<NativeInputDescriptor>,
    pub dynamic_inputs: Vec<NativeDynamicInputDescriptor>,
    pub outputs: Vec<NativeOutputDescriptor>,
    pub output_node: bool,
    pub effect: NativeEffectClass,
    pub cache: NativeCachePolicy,
}

impl NativeNodeDescriptor {
    pub fn validate(&self) -> Result<(), NativeNodeContractError> {
        match (self.schema_version, self.source_schema.as_ref()) {
            (LEGACY_NATIVE_NODE_CONTRACT_SCHEMA_VERSION, None) => {}
            (NATIVE_NODE_CONTRACT_SCHEMA_VERSION, Some(source_schema)) => source_schema
                .validate()
                .map_err(|error| NativeNodeContractError::InvalidSourceSchema(error.to_string()))?,
            (LEGACY_NATIVE_NODE_CONTRACT_SCHEMA_VERSION, Some(_))
            | (NATIVE_NODE_CONTRACT_SCHEMA_VERSION, None) => {
                return Err(NativeNodeContractError::InvalidSourceSchema(
                    "descriptor version and source metadata do not agree".to_owned(),
                ));
            }
            _ => {
                return Err(NativeNodeContractError::UnsupportedContractSchema(
                    self.schema_version,
                ));
            }
        }
        validate_identifier("node class type", &self.class_type)?;
        validate_identifier("node implementation version", &self.implementation_version)?;
        if self.inputs.len() > MAX_PORTS
            || self.dynamic_inputs.len() > MAX_PORTS
            || self.outputs.len() > MAX_PORTS
        {
            return Err(NativeNodeContractError::InvalidPortCount);
        }
        if let Some(source_schema) = &self.source_schema {
            if source_schema.inputs.len() != self.inputs.len()
                || source_schema.dynamic_inputs.len() != self.dynamic_inputs.len()
                || source_schema.outputs.len() != self.outputs.len()
                || !source_schema
                    .inputs
                    .iter()
                    .zip(&self.inputs)
                    .all(|(schema, input)| schema.name == input.name)
                || !source_schema
                    .outputs
                    .iter()
                    .zip(&self.outputs)
                    .all(|(schema, output)| schema.name == output.name)
                || !source_schema
                    .dynamic_inputs
                    .iter()
                    .zip(&self.dynamic_inputs)
                    .all(|(schema, input)| {
                        schema.identity == input.name_template
                            && schema.start_index == input.start_index
                            && schema.minimum_count == input.minimum_count
                            && schema.maximum_count == input.maximum_count
                            && schema.input.name == input.input.name
                            && (input.name_template != "{name}"
                                || (schema.names.len() >= input.minimum_count as usize
                                    && schema.names.len() <= input.maximum_count as usize))
                    })
            {
                return Err(NativeNodeContractError::InvalidSourceSchema(
                    "schema port order does not match the execution descriptor".to_owned(),
                ));
            }
        }
        let mut input_names = BTreeSet::new();
        for input in &self.inputs {
            input.validate()?;
            if !input_names.insert(input.name.as_str()) {
                return Err(NativeNodeContractError::DuplicatePort(input.name.clone()));
            }
        }
        if let Some(source_schema) = &self.source_schema {
            if matches!(
                source_schema.node.provenance,
                crate::NativeSchemaProvenance::SourceV1 | crate::NativeSchemaProvenance::SourceV3
            ) {
                for (input, schema) in self.inputs.iter().zip(&source_schema.inputs) {
                    let expected =
                        crate::native_value_types_for_input_schema(schema).map_err(|error| {
                            NativeNodeContractError::InvalidSourceSchema(error.to_string())
                        })?;
                    if input.accepted_types != expected {
                        return Err(NativeNodeContractError::InvalidSourceSchema(format!(
                            "execution type for input `{}` does not match its source schema",
                            input.name
                        )));
                    }
                }
                for (input, schema) in self
                    .dynamic_inputs
                    .iter()
                    .zip(&source_schema.dynamic_inputs)
                {
                    let expected = crate::native_value_types_for_input_schema(&schema.input)
                        .map_err(|error| {
                            NativeNodeContractError::InvalidSourceSchema(error.to_string())
                        })?;
                    if input.input.accepted_types != expected {
                        return Err(NativeNodeContractError::InvalidSourceSchema(format!(
                            "execution type for dynamic input `{}` does not match its source schema",
                            input.name_template
                        )));
                    }
                }
                for (output, schema) in self.outputs.iter().zip(&source_schema.outputs) {
                    let expected =
                        crate::native_value_type_for_output_schema(schema).map_err(|error| {
                            NativeNodeContractError::InvalidSourceSchema(error.to_string())
                        })?;
                    if output.produced_type != expected {
                        return Err(NativeNodeContractError::InvalidSourceSchema(format!(
                            "execution type for output `{}` does not match its source schema",
                            output.name
                        )));
                    }
                }
            }
            for (input, schema) in self.inputs.iter().zip(&source_schema.inputs) {
                if let Some(default) = schema
                    .default
                    .as_ref()
                    .and_then(native_value_from_schema_value)
                    && (!native_type_union_accepts_value(&input.accepted_types, &default)
                        || !native_value_matches_input_schema(&default, schema))
                {
                    return Err(NativeNodeContractError::InvalidSourceSchema(format!(
                        "default for input `{}` is incompatible with its execution type or constraints",
                        input.name
                    )));
                }
            }
        }
        let mut templates = BTreeSet::new();
        for input in &self.dynamic_inputs {
            input.validate()?;
            if !templates.insert(input.name_template.as_str()) {
                return Err(NativeNodeContractError::DuplicatePort(
                    input.name_template.clone(),
                ));
            }
        }
        let mut output_names = BTreeSet::new();
        for output in &self.outputs {
            output.validate()?;
            if !output_names.insert(output.name.as_str()) {
                return Err(NativeNodeContractError::DuplicatePort(output.name.clone()));
            }
        }
        Ok(())
    }

    pub fn validate_exact_schema_v2(&self) -> Result<(), NativeNodeContractError> {
        self.validate()?;
        if self.schema_version != NATIVE_NODE_CONTRACT_SCHEMA_VERSION
            || self.source_schema.is_none()
        {
            return Err(NativeNodeContractError::InvalidSourceSchema(
                "an exact schema-v2 descriptor is required".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeNodePresentation {
    pub display_name: String,
    pub category: String,
    pub description: String,
    pub output_names: Vec<String>,
    pub search_aliases: Vec<String>,
    pub is_deprecated: bool,
    pub is_experimental: bool,
}

impl NativeNodePresentation {
    pub fn validate(&self) -> Result<(), NativeNodeContractError> {
        validate_text(
            "node display name",
            &self.display_name,
            MAX_IDENTIFIER_BYTES,
            false,
        )?;
        validate_text("node category", &self.category, MAX_IDENTIFIER_BYTES, true)?;
        validate_text("node description", &self.description, MAX_TEXT_BYTES, true)?;
        if self.output_names.len() > MAX_PORTS {
            return Err(NativeNodeContractError::InvalidPresentationOutputs);
        }
        let mut output_names = BTreeSet::new();
        for output_name in &self.output_names {
            validate_identifier("node presentation output name", output_name)?;
            if !output_names.insert(output_name.as_str()) {
                return Err(NativeNodeContractError::InvalidPresentationOutputs);
            }
        }
        let mut aliases = BTreeSet::new();
        for alias in &self.search_aliases {
            validate_text("node search alias", alias, MAX_IDENTIFIER_BYTES, false)?;
            if !aliases.insert(alias.as_str()) {
                return Err(NativeNodeContractError::DuplicateSearchAlias(alias.clone()));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum NativeHandleStoreError {
    #[error("native handle operation was cancelled")]
    Cancelled,
    #[error("native handle belongs to a different store")]
    WrongStore,
    #[error("native handle belongs to a different store generation")]
    WrongGeneration,
    #[error("native handle type `{actual}` does not match `{expected}`")]
    WrongType { expected: String, actual: String },
    #[error("native handle `{0}` is absent")]
    Missing(String),
    #[error("native handle digest does not match the stored object")]
    DigestMismatch,
    #[error("native handle store rejected the operation: {0}")]
    Rejected(String),
    #[error("native handle contract is invalid: {0}")]
    InvalidHandle(#[from] NativeNodeContractError),
    #[error("native stored payload is invalid: {0}")]
    InvalidPayload(#[from] NativeStoredPayloadError),
}

pub trait NativeResolvedPayloadRetention: fmt::Debug + Send + Sync {}

#[derive(Clone)]
pub struct NativeResolvedPayload {
    payload: Arc<NativeStoredPayload>,
    _retention: Arc<dyn NativeResolvedPayloadRetention>,
}

impl NativeResolvedPayload {
    pub fn checked(
        payload: Arc<NativeStoredPayload>,
        retention: Arc<dyn NativeResolvedPayloadRetention>,
    ) -> Result<Self, NativeStoredPayloadError> {
        payload.validate()?;
        Ok(Self {
            payload,
            _retention: retention,
        })
    }
}

impl AsRef<NativeStoredPayload> for NativeResolvedPayload {
    fn as_ref(&self) -> &NativeStoredPayload {
        self.payload.as_ref()
    }
}

impl std::ops::Deref for NativeResolvedPayload {
    type Target = NativeStoredPayload;

    fn deref(&self) -> &Self::Target {
        self.payload.as_ref()
    }
}

impl fmt::Debug for NativeResolvedPayload {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeResolvedPayload")
            .finish_non_exhaustive()
    }
}

pub trait NativeHandleStore: Send + Sync + fmt::Debug {
    fn identity(&self) -> NativeHandleStoreIdentity;
    fn attempt_id(&self) -> AttemptId;

    fn resolve(
        &self,
        handle: &NativeOpaqueHandle,
        expected_type: &NativeHandleType,
        cancellation: &CancellationToken,
    ) -> Result<NativeResolvedPayload, NativeHandleStoreError>;

    fn publish(
        &self,
        payload: NativeStoredPayload,
        cancellation: &CancellationToken,
    ) -> Result<NativeOpaqueHandle, NativeHandleStoreError>;

    fn revoke(
        &self,
        handle: &NativeOpaqueHandle,
        cancellation: &CancellationToken,
    ) -> Result<(), NativeHandleStoreError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeNodeServiceIdentity {
    service_id: Uuid,
    attempt_id: AttemptId,
    node_id: NodeId,
}

impl NativeNodeServiceIdentity {
    pub fn checked(
        service_id: Uuid,
        attempt_id: AttemptId,
        node_id: NodeId,
    ) -> Result<Self, NativeNodeContractError> {
        if service_id.is_nil() || attempt_id.0.is_nil() {
            return Err(NativeNodeContractError::InvalidNodeServiceIdentity);
        }
        validate_identifier("native node service node ID", &node_id.0)?;
        Ok(Self {
            service_id,
            attempt_id,
            node_id,
        })
    }

    pub const fn service_id(&self) -> Uuid {
        self.service_id
    }

    pub const fn attempt_id(&self) -> AttemptId {
        self.attempt_id
    }

    pub const fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    fn matches(&self, attempt_id: AttemptId, node_id: &NodeId) -> bool {
        self.attempt_id == attempt_id && &self.node_id == node_id
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeAssetReference {
    service_id: Uuid,
    reference_id: Uuid,
    source_type_id: String,
    byte_length: u64,
    sha256: String,
}

impl NativeAssetReference {
    pub fn checked(
        service_id: Uuid,
        reference_id: Uuid,
        source_type_id: impl Into<String>,
        byte_length: u64,
        sha256: impl Into<String>,
    ) -> Result<Self, NativeAssetServiceError> {
        let source_type_id = source_type_id.into();
        let sha256 = sha256.into();
        if service_id.is_nil()
            || reference_id.is_nil()
            || byte_length == 0
            || byte_length > 2 * 1024 * 1024 * 1024
            || validate_identifier("native asset source type", &source_type_id).is_err()
            || !valid_sha256(&sha256)
        {
            return Err(NativeAssetServiceError::InvalidReference);
        }
        Ok(Self {
            service_id,
            reference_id,
            source_type_id,
            byte_length,
            sha256,
        })
    }

    pub const fn service_id(&self) -> Uuid {
        self.service_id
    }

    pub const fn reference_id(&self) -> Uuid {
        self.reference_id
    }

    pub fn source_type_id(&self) -> &str {
        &self.source_type_id
    }

    pub const fn byte_length(&self) -> u64 {
        self.byte_length
    }

    pub fn sha256(&self) -> &str {
        &self.sha256
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeAssetReadRequest {
    reference: NativeAssetReference,
    maximum_bytes: u64,
}

impl NativeAssetReadRequest {
    pub fn checked(
        reference: NativeAssetReference,
        maximum_bytes: u64,
    ) -> Result<Self, NativeAssetServiceError> {
        if maximum_bytes == 0
            || maximum_bytes > 2 * 1024 * 1024 * 1024
            || reference.byte_length() > maximum_bytes
        {
            return Err(NativeAssetServiceError::InvalidRequest);
        }
        Ok(Self {
            reference,
            maximum_bytes,
        })
    }

    pub const fn reference(&self) -> &NativeAssetReference {
        &self.reference
    }

    pub const fn maximum_bytes(&self) -> u64 {
        self.maximum_bytes
    }
}

#[derive(Clone, Debug)]
pub struct NativeResolvedAsset {
    reference: NativeAssetReference,
    bytes: Arc<[u8]>,
    byte_length: u64,
    sha256: String,
}

impl NativeResolvedAsset {
    pub fn checked(
        reference: NativeAssetReference,
        bytes: Arc<[u8]>,
        sha256: impl Into<String>,
    ) -> Result<Self, NativeAssetServiceError> {
        let sha256 = sha256.into();
        let byte_length =
            u64::try_from(bytes.len()).map_err(|_| NativeAssetServiceError::TooLarge)?;
        if byte_length > 2 * 1024 * 1024 * 1024
            || byte_length != reference.byte_length()
            || !valid_sha256(&sha256)
            || sha256 != reference.sha256()
            || sha256 != format!("{:x}", Sha256::digest(&bytes))
        {
            return Err(NativeAssetServiceError::DigestMismatch);
        }
        Ok(Self {
            reference,
            bytes,
            byte_length,
            sha256,
        })
    }

    pub const fn reference(&self) -> &NativeAssetReference {
        &self.reference
    }

    pub fn bytes(&self) -> &Arc<[u8]> {
        &self.bytes
    }

    pub const fn byte_length(&self) -> u64 {
        self.byte_length
    }

    pub fn sha256(&self) -> &str {
        &self.sha256
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum NativeAssetServiceError {
    #[error("native asset service is unavailable")]
    Unavailable,
    #[error("native asset operation was cancelled")]
    Cancelled,
    #[error("native asset reference is invalid")]
    InvalidReference,
    #[error("native asset read request is invalid")]
    InvalidRequest,
    #[error("native asset permission was denied")]
    PermissionDenied,
    #[error("native asset is missing")]
    Missing,
    #[error("native asset exceeds the authorized byte limit")]
    TooLarge,
    #[error("native asset digest changed or did not match")]
    DigestMismatch,
    #[error("native asset changed during the verified read")]
    ChangedDuringRead,
    #[error("native asset service rejected the operation")]
    Rejected,
}

pub trait NativeAssetResolver: Send + Sync + fmt::Debug {
    fn identity(&self) -> &NativeNodeServiceIdentity;

    fn read_verified(
        &self,
        request: &NativeAssetReadRequest,
        cancellation: &CancellationToken,
    ) -> Result<NativeResolvedAsset, NativeAssetServiceError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeOutputNamespace {
    Output,
    Temporary,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeOutputShape {
    File,
    Image { width: u32, height: u32 },
}

#[derive(Clone, Debug)]
pub struct NativeOutputEffectRequest {
    namespace: NativeOutputNamespace,
    filename_prefix: String,
    extension: String,
    batch_index: u32,
    shape: NativeOutputShape,
    content: Arc<[u8]>,
    request_digest_sha256: String,
}

impl NativeOutputEffectRequest {
    pub fn checked(
        namespace: NativeOutputNamespace,
        filename_prefix: impl Into<String>,
        extension: impl Into<String>,
        batch_index: u32,
        shape: NativeOutputShape,
        content: Arc<[u8]>,
        maximum_bytes: u64,
    ) -> Result<Self, NativeEffectServiceError> {
        let filename_prefix = filename_prefix.into();
        let extension = extension.into();
        if filename_prefix.is_empty()
            || filename_prefix.len() > MAX_IDENTIFIER_BYTES
            || filename_prefix
                .bytes()
                .any(|byte| byte.is_ascii_control() || matches!(byte, b'/' | b'\\'))
            || extension.is_empty()
            || extension.len() > 32
            || !extension
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
            || content.is_empty()
            || u64::try_from(content.len()).map_or(true, |length| {
                length > maximum_bytes || length > 2 * 1024 * 1024 * 1024
            })
            || matches!(shape, NativeOutputShape::Image { width: 0, .. })
            || matches!(shape, NativeOutputShape::Image { height: 0, .. })
        {
            return Err(NativeEffectServiceError::InvalidRequest);
        }
        let request_digest_sha256 = output_request_digest(
            namespace,
            &filename_prefix,
            &extension,
            batch_index,
            shape,
            &content,
        );
        Ok(Self {
            namespace,
            filename_prefix,
            extension,
            batch_index,
            shape,
            content,
            request_digest_sha256,
        })
    }

    pub const fn namespace(&self) -> NativeOutputNamespace {
        self.namespace
    }

    pub fn filename_prefix(&self) -> &str {
        &self.filename_prefix
    }

    pub fn extension(&self) -> &str {
        &self.extension
    }

    pub const fn batch_index(&self) -> u32 {
        self.batch_index
    }

    pub const fn shape(&self) -> NativeOutputShape {
        self.shape
    }

    pub fn content(&self) -> &Arc<[u8]> {
        &self.content
    }

    pub fn request_digest_sha256(&self) -> &str {
        &self.request_digest_sha256
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativePreparedEffectKind {
    Output,
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum NativeEffectServiceError {
    #[error("native prepared-effect service is unavailable")]
    Unavailable,
    #[error("native prepared-effect operation was cancelled")]
    Cancelled,
    #[error("native output effect request is invalid")]
    InvalidRequest,
    #[error("native prepared-effect service rejected the operation")]
    Rejected,
    #[error("native effect ticket is duplicated or belongs to another node attempt")]
    InvalidTicket,
}

#[derive(Debug, Error)]
pub enum NativeImagePreviewError {
    #[error("native image preview compute session is invalid: {0}")]
    Contract(#[from] NativeNodeContractError),
    #[error("native image preview tensor is invalid: {0}")]
    Tensor(#[from] TensorError),
    #[error("native image preview PNG encoding failed: {0}")]
    Png(#[from] PngError),
    #[error("native image preview effect preparation failed: {0}")]
    Effect(#[from] NativeEffectServiceError),
    #[error("native image preview dimensions or batch index exceeded the output contract")]
    DimensionOverflow,
}

#[derive(Clone, Debug)]
pub struct NativePreparedImagePreview {
    effects: Vec<NativePreparedEffectRequest>,
    ui: Value,
}

impl NativePreparedImagePreview {
    pub fn effects(&self) -> &[NativePreparedEffectRequest] {
        &self.effects
    }

    pub fn ui(&self) -> &Value {
        &self.ui
    }

    pub fn into_parts(self) -> (Vec<NativePreparedEffectRequest>, Value) {
        (self.effects, self.ui)
    }
}

pub trait NativePreparedEffectService: Send + Sync + fmt::Debug {
    fn identity(&self) -> &NativeNodeServiceIdentity;
    fn maximum_output_bytes(&self) -> u64;

    fn prepare_output(
        &self,
        request: NativeOutputEffectRequest,
        cancellation: &CancellationToken,
    ) -> Result<NativePreparedEffectRequest, NativeEffectServiceError>;

    fn rollback_prepared(
        &self,
        request: &NativePreparedEffectRequest,
    ) -> Result<(), NativeEffectServiceError>;

    fn rollback_all_prepared(&self) -> Result<(), NativeEffectServiceError>;
}

#[derive(Debug, Error)]
pub enum NativeShaderServiceError {
    #[error("native shader execution service is unavailable")]
    Unavailable,
    #[error("native shader compute session is invalid: {0}")]
    Contract(#[from] NativeNodeContractError),
    #[error("native shader execution failed: {0}")]
    Shader(#[from] NativeShaderError),
    #[error("native shader executor returned an invalid result projection")]
    InvalidProjection,
}

#[derive(Debug, Error)]
pub enum NativeShaderPreviewError {
    #[error("native shader execution failed: {0}")]
    Shader(#[from] NativeShaderServiceError),
    #[error("native shader preview preparation failed: {0}")]
    Preview(#[from] NativeImagePreviewError),
    #[error("native shader preview effect rollback failed: {0}")]
    Effect(#[from] NativeEffectServiceError),
}

#[derive(Clone, Debug)]
pub struct NativePreparedShaderResult {
    shader: NativeShaderResult,
    effects: Vec<NativePreparedEffectRequest>,
    ui: Value,
}

impl NativePreparedShaderResult {
    pub fn shader(&self) -> &NativeShaderResult {
        &self.shader
    }

    pub fn effects(&self) -> &[NativePreparedEffectRequest] {
        &self.effects
    }

    pub fn ui(&self) -> &Value {
        &self.ui
    }

    pub fn into_parts(self) -> (NativeShaderResult, Vec<NativePreparedEffectRequest>, Value) {
        (self.shader, self.effects, self.ui)
    }
}

#[derive(Clone, Debug)]
pub struct NativeNodeComputeSession {
    identity: NativeNodeServiceIdentity,
    backend: Arc<CpuBackend>,
    stream: StreamId,
    scratch_binding: ScratchBindingIdentity,
}

impl NativeNodeComputeSession {
    pub fn checked(
        identity: NativeNodeServiceIdentity,
        backend: Arc<CpuBackend>,
        stream: StreamId,
        scratch: &ScratchReservation,
    ) -> Result<Self, NativeNodeContractError> {
        backend
            .validate_scratch_reservation(scratch)
            .map_err(|_| NativeNodeContractError::InvalidComputeSession)?;
        Ok(Self {
            identity,
            backend,
            stream,
            scratch_binding: scratch.binding_identity(),
        })
    }

    pub const fn identity(&self) -> &NativeNodeServiceIdentity {
        &self.identity
    }

    pub fn backend(&self) -> &CpuBackend {
        &self.backend
    }

    pub const fn stream(&self) -> StreamId {
        self.stream
    }
}

#[derive(Clone, Debug, Default)]
pub struct NativeNodeServices {
    assets: Option<Arc<dyn NativeAssetResolver>>,
    effects: Option<Arc<dyn NativePreparedEffectService>>,
    compute: Option<NativeNodeComputeSession>,
    shader: Option<Arc<dyn NativeShaderExecutor>>,
}

impl NativeNodeServices {
    pub fn checked(
        assets: Option<Arc<dyn NativeAssetResolver>>,
        effects: Option<Arc<dyn NativePreparedEffectService>>,
        compute: Option<NativeNodeComputeSession>,
    ) -> Result<Self, NativeNodeContractError> {
        for identity in assets
            .as_deref()
            .map(NativeAssetResolver::identity)
            .into_iter()
            .chain(
                effects
                    .as_deref()
                    .map(NativePreparedEffectService::identity),
            )
            .chain(compute.as_ref().map(NativeNodeComputeSession::identity))
        {
            if identity.service_id().is_nil() {
                return Err(NativeNodeContractError::InvalidNodeServiceIdentity);
            }
        }
        Ok(Self {
            assets,
            effects,
            compute,
            shader: None,
        })
    }

    pub fn with_shader(mut self, shader: Arc<dyn NativeShaderExecutor>) -> Self {
        self.shader = Some(shader);
        self
    }
}

#[derive(Clone, Debug)]
pub struct NativeNodeContext {
    pub prompt_id: PromptId,
    pub attempt_id: AttemptId,
    pub node_id: NodeId,
    pub cancellation: CancellationToken,
    pub scratch: ScratchReservation,
    handle_store: Arc<dyn NativeHandleStore>,
    services: NativeNodeServices,
}

impl NativeNodeContext {
    pub fn new(
        prompt_id: PromptId,
        attempt_id: AttemptId,
        node_id: NodeId,
        cancellation: CancellationToken,
        scratch: ScratchReservation,
        handle_store: Arc<dyn NativeHandleStore>,
    ) -> Result<Self, NativeNodeContractError> {
        Self::new_with_services(
            prompt_id,
            attempt_id,
            node_id,
            cancellation,
            scratch,
            handle_store,
            NativeNodeServices::default(),
        )
    }

    pub fn new_with_services(
        prompt_id: PromptId,
        attempt_id: AttemptId,
        node_id: NodeId,
        cancellation: CancellationToken,
        scratch: ScratchReservation,
        handle_store: Arc<dyn NativeHandleStore>,
        services: NativeNodeServices,
    ) -> Result<Self, NativeNodeContractError> {
        let context = Self {
            prompt_id,
            attempt_id,
            node_id,
            cancellation,
            scratch,
            handle_store,
            services,
        };
        context.validate()?;
        Ok(context)
    }

    pub fn handle_store(&self) -> &dyn NativeHandleStore {
        self.handle_store.as_ref()
    }

    pub fn asset_resolver(&self) -> Result<&dyn NativeAssetResolver, NativeAssetServiceError> {
        self.services
            .assets
            .as_deref()
            .ok_or(NativeAssetServiceError::Unavailable)
    }

    pub fn prepared_effects(
        &self,
    ) -> Result<&dyn NativePreparedEffectService, NativeEffectServiceError> {
        self.services
            .effects
            .as_deref()
            .ok_or(NativeEffectServiceError::Unavailable)
    }

    pub fn compute_session(&self) -> Result<&NativeNodeComputeSession, NativeNodeContractError> {
        self.services
            .compute
            .as_ref()
            .ok_or(NativeNodeContractError::InvalidComputeSession)
    }

    pub fn execute_shader(
        &self,
        request: &NativeShaderRequest,
    ) -> Result<NativeShaderResult, NativeShaderServiceError> {
        self.cancellation
            .check()
            .map_err(|_| NativeShaderError::Cancelled)?;
        let compute = self.compute_session()?;
        let execution_context = compute.execution_context(self)?;
        let shader = self
            .services
            .shader
            .as_deref()
            .ok_or(NativeShaderServiceError::Unavailable)?;
        let result = shader
            .execute(request, compute.backend(), &execution_context)
            .map_err(NativeShaderServiceError::from)?;
        let (batch, _, _, _) = request
            .images
            .first()
            .ok_or(NativeShaderServiceError::InvalidProjection)?
            .dimensions()
            .map_err(|_| NativeShaderServiceError::InvalidProjection)?;
        if result.outputs.len() != MAX_SHADER_OUTPUTS
            || result.pass_count == 0
            || result.pass_count > MAX_SHADER_PASSES
        {
            return Err(NativeShaderServiceError::InvalidProjection);
        }
        for output in &result.outputs {
            let dimensions = output
                .dimensions()
                .map_err(|_| NativeShaderServiceError::InvalidProjection)?;
            if dimensions
                != (
                    batch,
                    u64::from(request.height),
                    u64::from(request.width),
                    4,
                )
            {
                return Err(NativeShaderServiceError::InvalidProjection);
            }
        }
        Ok(result)
    }

    pub fn execute_shader_with_previews(
        &self,
        request: &NativeShaderRequest,
    ) -> Result<NativePreparedShaderResult, NativeShaderPreviewError> {
        let shader = self.execute_shader(request)?;
        let effects_service = self.prepared_effects()?;
        let mut effects = Vec::new();
        let mut input_images = Vec::new();

        for (image_index, image) in request.images.iter().enumerate() {
            let prefix = format!("GLSLShader_input_{image_index}");
            let preview = match self.prepare_image_preview(image, &prefix) {
                Ok(preview) => preview,
                Err(error) => {
                    for effect in effects.iter().rev() {
                        effects_service.rollback_prepared(effect)?;
                    }
                    return Err(error.into());
                }
            };
            let (preview_effects, ui) = preview.into_parts();
            if let Some(images) = ui.get("images").and_then(Value::as_array) {
                input_images.extend(images.iter().cloned());
            }
            effects.extend(preview_effects);
        }

        let output = shader
            .outputs
            .first()
            .ok_or(NativeShaderServiceError::InvalidProjection)?;
        let preview = match self.prepare_image_preview(output, "GLSLShader_output") {
            Ok(preview) => preview,
            Err(error) => {
                for effect in effects.iter().rev() {
                    effects_service.rollback_prepared(effect)?;
                }
                return Err(error.into());
            }
        };
        let (preview_effects, ui) = preview.into_parts();
        let output_images = ui
            .get("images")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        effects.extend(preview_effects);

        if self.cancellation.is_cancelled() {
            for effect in effects.iter().rev() {
                effects_service.rollback_prepared(effect)?;
            }
            return Err(NativeShaderServiceError::Shader(NativeShaderError::Cancelled).into());
        }
        Ok(NativePreparedShaderResult {
            shader,
            effects,
            ui: json!({"input_images": input_images, "images": output_images}),
        })
    }

    pub fn prepare_image_preview(
        &self,
        image: &ImageTensor,
        filename_prefix: &str,
    ) -> Result<NativePreparedImagePreview, NativeImagePreviewError> {
        self.cancellation
            .check()
            .map_err(|_| NativeImagePreviewError::Effect(NativeEffectServiceError::Cancelled))?;
        let compute = self.compute_session()?;
        let execution_context = compute.execution_context(self)?;
        let effects_service = self.prepared_effects()?;
        let (batch, height, width, channels) = image.dimensions()?;
        let pixels = image.as_f32_slice()?;
        let metadata = BTreeMap::new();
        let mut effects = Vec::new();
        let mut ui_images = Vec::new();
        for batch_index in 0..batch {
            let prepared = (|| {
                self.cancellation.check().map_err(|_| {
                    NativeImagePreviewError::Effect(NativeEffectServiceError::Cancelled)
                })?;
                let encoded = encode_png_frame_with_policy_and_context(
                    compute.backend(),
                    &execution_context,
                    pixels,
                    batch,
                    height,
                    width,
                    channels,
                    batch_index,
                    &metadata,
                    MetadataWritePolicy {
                        metadata_enabled: false,
                    },
                    PngLimits::default(),
                )?;
                let output_index = u32::try_from(batch_index)
                    .map_err(|_| NativeImagePreviewError::DimensionOverflow)?;
                let request = NativeOutputEffectRequest::checked(
                    NativeOutputNamespace::Temporary,
                    filename_prefix,
                    "png",
                    output_index,
                    NativeOutputShape::Image {
                        width: u32::try_from(width)
                            .map_err(|_| NativeImagePreviewError::DimensionOverflow)?,
                        height: u32::try_from(height)
                            .map_err(|_| NativeImagePreviewError::DimensionOverflow)?,
                    },
                    Arc::from(encoded),
                    effects_service.maximum_output_bytes(),
                )?;
                effects_service
                    .prepare_output(request, &self.cancellation)
                    .map_err(NativeImagePreviewError::from)
            })();
            let effect = match prepared {
                Ok(effect) => effect,
                Err(error) => {
                    for effect in effects.iter().rev() {
                        effects_service.rollback_prepared(effect)?;
                    }
                    return Err(error);
                }
            };
            ui_images.push(json!({
                "transaction_id": effect.transaction_id(),
                "batch_index": batch_index,
                "type": "temp",
            }));
            effects.push(effect);
        }
        if self.cancellation.is_cancelled() {
            for effect in effects.iter().rev() {
                effects_service.rollback_prepared(effect)?;
            }
            return Err(NativeImagePreviewError::Effect(
                NativeEffectServiceError::Cancelled,
            ));
        }
        Ok(NativePreparedImagePreview {
            effects,
            ui: json!({"images": ui_images, "animated": [false]}),
        })
    }

    pub fn validate(&self) -> Result<(), NativeNodeContractError> {
        if self.prompt_id.0.is_nil()
            || self.attempt_id.0.is_nil()
            || self.handle_store.attempt_id() != self.attempt_id
        {
            return Err(NativeNodeContractError::InvalidNodeContext);
        }
        validate_identifier("native node context node ID", &self.node_id.0)?;
        self.handle_store.identity().validate()?;
        for identity in self
            .services
            .assets
            .as_deref()
            .map(NativeAssetResolver::identity)
            .into_iter()
            .chain(
                self.services
                    .effects
                    .as_deref()
                    .map(NativePreparedEffectService::identity),
            )
            .chain(
                self.services
                    .compute
                    .as_ref()
                    .map(NativeNodeComputeSession::identity),
            )
        {
            if !identity.matches(self.attempt_id, &self.node_id) {
                return Err(NativeNodeContractError::InvalidNodeServiceIdentity);
            }
        }
        if let Some(compute) = &self.services.compute {
            if compute.scratch_binding != self.scratch.binding_identity() {
                return Err(NativeNodeContractError::InvalidComputeSession);
            }
            compute
                .backend
                .validate_scratch_reservation(&self.scratch)
                .map_err(|_| NativeNodeContractError::InvalidComputeSession)?;
        }
        Ok(())
    }
}

impl NativeNodeComputeSession {
    pub fn execution_context<'a>(
        &self,
        context: &'a NativeNodeContext,
    ) -> Result<ExecutionContext<'a>, NativeNodeContractError> {
        if !self.identity.matches(context.attempt_id, &context.node_id)
            || self.scratch_binding != context.scratch.binding_identity()
        {
            return Err(NativeNodeContractError::InvalidComputeSession);
        }
        self.backend
            .validate_scratch_reservation(&context.scratch)
            .map_err(|_| NativeNodeContractError::InvalidComputeSession)?;
        Ok(ExecutionContext {
            stream: self.stream,
            scratch: context.scratch.clone(),
            rng_phase: None,
            cancellation: &context.cancellation,
        })
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeCacheDependencies {
    pub artifact_digests: BTreeMap<String, String>,
    pub plugin_digest: Option<String>,
    pub rng_phase: Option<String>,
}

impl NativeCacheDependencies {
    pub fn validate(&self) -> Result<(), NativeNodeContractError> {
        for (identifier, digest) in &self.artifact_digests {
            validate_identifier("cache artifact identifier", identifier)?;
            if !valid_sha256(digest) {
                return Err(NativeNodeContractError::InvalidDigest);
            }
        }
        if let Some(digest) = &self.plugin_digest
            && !valid_sha256(digest)
        {
            return Err(NativeNodeContractError::InvalidDigest);
        }
        if let Some(phase) = &self.rng_phase {
            validate_identifier("cache RNG phase", phase)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativePreparedEffectRequest {
    service_id: Uuid,
    transaction_id: Uuid,
    kind: NativePreparedEffectKind,
    request_digest_sha256: String,
}

impl NativePreparedEffectRequest {
    pub fn checked(
        service_id: Uuid,
        transaction_id: Uuid,
        kind: NativePreparedEffectKind,
        request_digest_sha256: impl Into<String>,
    ) -> Result<Self, NativeNodeContractError> {
        let request = Self {
            service_id,
            transaction_id,
            kind,
            request_digest_sha256: request_digest_sha256.into(),
        };
        request.validate()?;
        Ok(request)
    }

    pub const fn service_id(&self) -> Uuid {
        self.service_id
    }

    pub const fn transaction_id(&self) -> Uuid {
        self.transaction_id
    }

    pub const fn kind(&self) -> NativePreparedEffectKind {
        self.kind
    }

    pub fn request_digest_sha256(&self) -> &str {
        &self.request_digest_sha256
    }

    pub fn validate(&self) -> Result<(), NativeNodeContractError> {
        if self.service_id.is_nil()
            || self.transaction_id.is_nil()
            || !valid_sha256(&self.request_digest_sha256)
        {
            return Err(NativeNodeContractError::InvalidEffectRequest);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum NativeNodeOutcome {
    Values {
        outputs: Vec<NativeValue>,
        ui: Option<Value>,
        effects: Vec<NativePreparedEffectRequest>,
    },
    Blocked {
        reason: String,
    },
    Expansion {
        prompt: ApiPrompt,
        output_node: NodeId,
    },
}

impl NativeNodeOutcome {
    pub fn validate(&self) -> Result<(), NativeNodeContractError> {
        match self {
            Self::Values {
                outputs,
                ui,
                effects,
            } => {
                if outputs.len() > MAX_PORTS || effects.len() > MAX_PORTS {
                    return Err(NativeNodeContractError::InvalidOutcome);
                }
                for output in outputs {
                    output.validate()?;
                }
                if let Some(ui) = ui {
                    let encoded = serde_json::to_vec(ui)
                        .map_err(NativeNodeContractError::EncodePresentationValue)?;
                    if encoded.len() > MAX_UNKNOWN_BYTES {
                        return Err(NativeNodeContractError::PresentationValueTooLarge);
                    }
                }
                for effect in effects {
                    effect.validate()?;
                }
                Ok(())
            }
            Self::Blocked { reason } => {
                validate_text("blocked reason", reason, MAX_TEXT_BYTES, false)
            }
            Self::Expansion {
                prompt,
                output_node,
            } => {
                validate_identifier("expansion output node", &output_node.0)?;
                if prompt.0.is_empty() || !prompt.0.contains_key(output_node) {
                    return Err(NativeNodeContractError::InvalidExpansion);
                }
                Ok(())
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeNodeFailureKind {
    Failure,
    Interrupted,
}

#[derive(Clone, Debug, Error, Eq, PartialEq, Serialize, Deserialize)]
#[error("{code}: {message}")]
#[serde(deny_unknown_fields)]
pub struct NativeNodeFailure {
    pub code: String,
    pub message: String,
    pub kind: NativeNodeFailureKind,
    pub retryable: bool,
}

impl NativeNodeFailure {
    pub fn validate(&self) -> Result<(), NativeNodeContractError> {
        validate_identifier("node failure code", &self.code)?;
        validate_text("node failure message", &self.message, MAX_TEXT_BYTES, false)
    }
}

pub trait NativeNode: Send + Sync {
    fn class_type(&self) -> &str;
    fn implementation_version(&self) -> &str;

    fn implementation_namespace(&self) -> &str {
        "sim.native_rust"
    }

    fn demanded_lazy_inputs(
        &self,
        _context: &NativeNodeContext,
        _available_inputs: &BTreeMap<String, NativeValue>,
    ) -> Result<BTreeSet<String>, NativeNodeFailure> {
        Ok(BTreeSet::new())
    }

    fn cache_change_token(
        &self,
        _inputs: &BTreeMap<String, NativeValue>,
    ) -> Result<String, NativeNodeFailure> {
        Ok("stable".to_owned())
    }

    fn cache_dependencies(
        &self,
        _context: &NativeNodeContext,
        _inputs: &BTreeMap<String, NativeValue>,
    ) -> Result<NativeCacheDependencies, NativeNodeFailure> {
        Ok(NativeCacheDependencies::default())
    }

    fn execute<'a>(
        &'a self,
        context: NativeNodeContext,
        inputs: BTreeMap<String, NativeValue>,
    ) -> BoxFuture<'a, Result<NativeNodeOutcome, NativeNodeFailure>>;
}

pub type NativeNodeBindingsFactory =
    fn() -> Result<Vec<NativeNodeBinding>, NativeNodeContractError>;

#[derive(Clone)]
pub enum NativeNodeBinding {
    Executable {
        feature_id: String,
        descriptor: NativeNodeDescriptor,
        presentation: NativeNodePresentation,
        node: Arc<dyn NativeNode>,
    },
    ProviderRequired {
        feature_id: String,
        descriptor: NativeNodeDescriptor,
        presentation: NativeNodePresentation,
        provider: String,
        reason: String,
    },
    Unavailable {
        feature_id: String,
        descriptor: NativeNodeDescriptor,
        presentation: NativeNodePresentation,
        reason: String,
    },
}

impl fmt::Debug for NativeNodeBinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeNodeBinding")
            .field("disposition", &self.disposition())
            .field("feature_id", &self.feature_id())
            .field("class_type", &self.descriptor().class_type)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeNodeBindingDisposition {
    Executable,
    ProviderRequired,
    Unavailable,
}

impl NativeNodeBinding {
    pub fn feature_id(&self) -> &str {
        match self {
            Self::Executable { feature_id, .. }
            | Self::ProviderRequired { feature_id, .. }
            | Self::Unavailable { feature_id, .. } => feature_id,
        }
    }

    pub fn descriptor(&self) -> &NativeNodeDescriptor {
        match self {
            Self::Executable { descriptor, .. }
            | Self::ProviderRequired { descriptor, .. }
            | Self::Unavailable { descriptor, .. } => descriptor,
        }
    }

    pub fn presentation(&self) -> &NativeNodePresentation {
        match self {
            Self::Executable { presentation, .. }
            | Self::ProviderRequired { presentation, .. }
            | Self::Unavailable { presentation, .. } => presentation,
        }
    }

    pub const fn disposition(&self) -> NativeNodeBindingDisposition {
        match self {
            Self::Executable { .. } => NativeNodeBindingDisposition::Executable,
            Self::ProviderRequired { .. } => NativeNodeBindingDisposition::ProviderRequired,
            Self::Unavailable { .. } => NativeNodeBindingDisposition::Unavailable,
        }
    }

    pub fn validate(&self) -> Result<(), NativeNodeContractError> {
        validate_feature_id(self.feature_id())?;
        self.descriptor().validate()?;
        self.presentation().validate()?;
        if self
            .descriptor()
            .outputs
            .iter()
            .map(|output| output.name.as_str())
            .ne(self.presentation().output_names.iter().map(String::as_str))
        {
            return Err(NativeNodeContractError::InvalidPresentationOutputs);
        }
        match self {
            Self::Executable {
                descriptor, node, ..
            } => {
                if descriptor.class_type != node.class_type()
                    || descriptor.implementation_version != node.implementation_version()
                    || node.implementation_namespace().trim().is_empty()
                {
                    return Err(NativeNodeContractError::BindingImplementationMismatch);
                }
                Ok(())
            }
            Self::ProviderRequired {
                provider, reason, ..
            } => {
                validate_identifier("native provider", provider)?;
                validate_text("provider reason", reason, MAX_TEXT_BYTES, false)
            }
            Self::Unavailable { reason, .. } => {
                validate_text("unavailable reason", reason, MAX_TEXT_BYTES, false)
            }
        }
    }
}

pub fn validate_generated_family_bindings(
    bindings: &[NativeNodeBinding],
    descriptor_ids: &[&str],
) -> Result<(), NativeNodeContractError> {
    let expected = descriptor_ids.iter().copied().collect::<BTreeSet<_>>();
    if expected.len() != descriptor_ids.len() {
        return Err(NativeNodeContractError::DuplicateGeneratedDescriptor);
    }
    let mut actual = BTreeSet::new();
    for binding in bindings {
        binding.validate()?;
        if !actual.insert(binding.descriptor().class_type.as_str()) {
            return Err(NativeNodeContractError::DuplicateGeneratedDescriptor);
        }
    }
    if actual != expected {
        return Err(NativeNodeContractError::GeneratedBindingMismatch);
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum NativeNodeContractError {
    #[error("native node contract schema {0} is unsupported")]
    UnsupportedContractSchema(u16),
    #[error("native opaque handle schema {0} is unsupported")]
    UnsupportedHandleSchema(u16),
    #[error("native node contract {field} is invalid")]
    InvalidText { field: &'static str },
    #[error("native node feature ID is invalid")]
    InvalidFeatureId,
    #[error("native opaque handle generation must be nonzero")]
    InvalidHandleGeneration,
    #[error("native opaque handle type identity is invalid")]
    InvalidHandleType,
    #[error("native opaque handle store identity is nil")]
    InvalidHandleStoreIdentity,
    #[error("native node SHA-256 digest is invalid")]
    InvalidDigest,
    #[error("native node number must be finite")]
    NonFiniteNumber,
    #[error("native node value nesting exceeds its limit")]
    ValueNestingTooDeep,
    #[error("native node list exceeds its value limit")]
    TooManyListValues,
    #[error("native preserved unknown value exceeds its byte limit")]
    PreservedUnknownTooLarge,
    #[error("native structured value is invalid")]
    InvalidStructuredValue,
    #[error("native preserved unknown value could not be encoded: {0}")]
    EncodePreservedUnknown(serde_json::Error),
    #[error("native UI presentation value could not be encoded: {0}")]
    EncodePresentationValue(serde_json::Error),
    #[error("native UI presentation value exceeds its byte limit")]
    PresentationValueTooLarge,
    #[error("native type union is empty, duplicated, unsorted, or ambiguous")]
    InvalidTypeUnion,
    #[error("native dynamic input descriptor is invalid")]
    InvalidDynamicInput,
    #[error("native node descriptor has an invalid port count")]
    InvalidPortCount,
    #[error("native node source schema is invalid: {0}")]
    InvalidSourceSchema(String),
    #[error("native node descriptor repeats port `{0}`")]
    DuplicatePort(String),
    #[error("native node presentation repeats search alias `{0}`")]
    DuplicateSearchAlias(String),
    #[error("native node presentation output names do not match its descriptor")]
    InvalidPresentationOutputs,
    #[error("native prepared effect request is invalid")]
    InvalidEffectRequest,
    #[error("native node outcome exceeds limits or is malformed")]
    InvalidOutcome,
    #[error("native node expansion is empty or lacks its output node")]
    InvalidExpansion,
    #[error("native node binding does not match its executable implementation")]
    BindingImplementationMismatch,
    #[error("native node context does not match its attempt-local handle store")]
    InvalidNodeContext,
    #[error("native node service identity does not match its attempt and node")]
    InvalidNodeServiceIdentity,
    #[error("native node compute session does not match its backend and scratch reservation")]
    InvalidComputeSession,
    #[error("native stored object metadata does not match its payload contract")]
    InvalidStoredObject,
    #[error("generated native node descriptors contain a duplicate")]
    DuplicateGeneratedDescriptor,
    #[error("generated native node bindings do not exactly match descriptor IDs")]
    GeneratedBindingMismatch,
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), NativeNodeContractError> {
    validate_text(field, value, MAX_IDENTIFIER_BYTES, false)
}

fn validate_text(
    field: &'static str,
    value: &str,
    maximum_bytes: usize,
    allow_empty: bool,
) -> Result<(), NativeNodeContractError> {
    if (!allow_empty && value.is_empty())
        || value.len() > maximum_bytes
        || value.contains('\0')
        || value.chars().any(char::is_control)
    {
        return Err(NativeNodeContractError::InvalidText { field });
    }
    Ok(())
}

fn validate_workflow_text(field: &'static str, value: &str) -> Result<(), NativeNodeContractError> {
    if value.len() > MAX_TEXT_BYTES
        || value.contains('\0')
        || value
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(NativeNodeContractError::InvalidText { field });
    }
    Ok(())
}

fn validate_feature_id(value: &str) -> Result<(), NativeNodeContractError> {
    let suffix = value
        .strip_prefix("COMFY-NODE-")
        .ok_or(NativeNodeContractError::InvalidFeatureId)?;
    let digits = suffix.strip_prefix("INACTIVE-").unwrap_or(suffix);
    if digits.len() != 4 || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(NativeNodeContractError::InvalidFeatureId);
    }
    Ok(())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn output_request_digest(
    namespace: NativeOutputNamespace,
    filename_prefix: &str,
    extension: &str,
    batch_index: u32,
    shape: NativeOutputShape,
    content: &[u8],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"sim.comfy.native-output-effect.v1");
    hasher.update([match namespace {
        NativeOutputNamespace::Output => 0,
        NativeOutputNamespace::Temporary => 1,
    }]);
    hasher.update((filename_prefix.len() as u64).to_le_bytes());
    hasher.update(filename_prefix.as_bytes());
    hasher.update((extension.len() as u64).to_le_bytes());
    hasher.update(extension.as_bytes());
    hasher.update(batch_index.to_le_bytes());
    match shape {
        NativeOutputShape::File => hasher.update([0]),
        NativeOutputShape::Image { width, height } => {
            hasher.update([1]);
            hasher.update(width.to_le_bytes());
            hasher.update(height.to_le_bytes());
        }
    }
    hasher.update((content.len() as u64).to_le_bytes());
    hasher.update(content);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use comfy_sampler::{NativeNoisePayload, NativeSamplerPayloadError};
    use comfy_tensor::CpuWorkspaceAuthority;
    use std::sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    };

    #[derive(Debug)]
    struct TestResolvedPayloadRetention;

    impl NativeResolvedPayloadRetention for TestResolvedPayloadRetention {}

    struct TestHandleStore {
        identity: NativeHandleStoreIdentity,
        attempt_id: AttemptId,
        next_identifier: AtomicU64,
        values: Mutex<BTreeMap<String, Arc<NativeStoredPayload>>>,
    }

    struct TestPreviewEffectService {
        identity: NativeNodeServiceIdentity,
        fail_on_ordinal: Option<u64>,
        next_ordinal: AtomicU64,
        prepared: Mutex<Vec<NativePreparedEffectRequest>>,
        rolled_back: Mutex<Vec<Uuid>>,
    }

    #[derive(Debug)]
    struct TestShaderExecutor;

    impl NativeShaderExecutor for TestShaderExecutor {
        fn configuration_identity(&self) -> String {
            "test-shader-v1".to_owned()
        }

        fn execute(
            &self,
            request: &NativeShaderRequest,
            backend: &CpuBackend,
            context: &ExecutionContext<'_>,
        ) -> Result<NativeShaderResult, NativeShaderError> {
            context
                .cancellation
                .check()
                .map_err(|_| NativeShaderError::Cancelled)?;
            let batch = request
                .images
                .first()
                .ok_or_else(|| NativeShaderError::Bounds("missing test image".to_owned()))?
                .dimensions()?
                .0;
            let pixel_count = usize::try_from(
                batch
                    .checked_mul(u64::from(request.width))
                    .and_then(|value| value.checked_mul(u64::from(request.height)))
                    .ok_or_else(|| {
                        NativeShaderError::Bounds("test output overflowed".to_owned())
                    })?,
            )
            .map_err(|_| NativeShaderError::Bounds("test output is too large".to_owned()))?;
            let output_values = pixel_count
                .checked_mul(4)
                .ok_or_else(|| NativeShaderError::Bounds("test output overflowed".to_owned()))?;
            let mut values = Vec::new();
            values.try_reserve_exact(output_values).map_err(|error| {
                NativeShaderError::Bounds(format!("test output allocation failed: {error}"))
            })?;
            for _ in 0..pixel_count {
                values.extend_from_slice(&[0.25, 0.5, 0.75, 1.0]);
            }
            let output = ImageTensor::from_f32(
                backend,
                context,
                batch,
                u64::from(request.height),
                u64::from(request.width),
                4,
                &values,
            )?;
            Ok(NativeShaderResult {
                outputs: vec![output.clone(), output.clone(), output.clone(), output],
                pass_count: 1,
            })
        }
    }

    impl fmt::Debug for TestPreviewEffectService {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter
                .debug_struct("TestPreviewEffectService")
                .field("identity", &self.identity)
                .finish_non_exhaustive()
        }
    }

    impl TestPreviewEffectService {
        fn new(identity: NativeNodeServiceIdentity, fail_on_ordinal: Option<u64>) -> Self {
            Self {
                identity,
                fail_on_ordinal,
                next_ordinal: AtomicU64::new(0),
                prepared: Mutex::new(Vec::new()),
                rolled_back: Mutex::new(Vec::new()),
            }
        }
    }

    impl NativePreparedEffectService for TestPreviewEffectService {
        fn identity(&self) -> &NativeNodeServiceIdentity {
            &self.identity
        }

        fn maximum_output_bytes(&self) -> u64 {
            1024 * 1024
        }

        fn prepare_output(
            &self,
            request: NativeOutputEffectRequest,
            cancellation: &CancellationToken,
        ) -> Result<NativePreparedEffectRequest, NativeEffectServiceError> {
            cancellation
                .check()
                .map_err(|_| NativeEffectServiceError::Cancelled)?;
            let ordinal = self.next_ordinal.fetch_add(1, Ordering::AcqRel);
            if self.fail_on_ordinal == Some(ordinal) {
                return Err(NativeEffectServiceError::Rejected);
            }
            let ticket = NativePreparedEffectRequest::checked(
                self.identity.service_id(),
                Uuid::from_u128(0x600 + u128::from(ordinal)),
                NativePreparedEffectKind::Output,
                request.request_digest_sha256(),
            )
            .map_err(|_| NativeEffectServiceError::Rejected)?;
            self.prepared
                .lock()
                .map_err(|_| NativeEffectServiceError::Rejected)?
                .push(ticket.clone());
            Ok(ticket)
        }

        fn rollback_prepared(
            &self,
            request: &NativePreparedEffectRequest,
        ) -> Result<(), NativeEffectServiceError> {
            let mut prepared = self
                .prepared
                .lock()
                .map_err(|_| NativeEffectServiceError::Rejected)?;
            let index = prepared
                .iter()
                .position(|ticket| ticket == request)
                .ok_or(NativeEffectServiceError::InvalidTicket)?;
            prepared.remove(index);
            self.rolled_back
                .lock()
                .map_err(|_| NativeEffectServiceError::Rejected)?
                .push(request.transaction_id());
            Ok(())
        }

        fn rollback_all_prepared(&self) -> Result<(), NativeEffectServiceError> {
            let mut prepared = self
                .prepared
                .lock()
                .map_err(|_| NativeEffectServiceError::Rejected)?;
            let mut rolled_back = self
                .rolled_back
                .lock()
                .map_err(|_| NativeEffectServiceError::Rejected)?;
            rolled_back.extend(prepared.drain(..).map(|ticket| ticket.transaction_id()));
            Ok(())
        }
    }

    impl fmt::Debug for TestHandleStore {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter
                .debug_struct("TestHandleStore")
                .field("identity", &self.identity)
                .field("attempt_id", &self.attempt_id)
                .finish_non_exhaustive()
        }
    }

    impl TestHandleStore {
        fn new(identity: NativeHandleStoreIdentity, attempt_id: AttemptId) -> Self {
            Self {
                identity,
                attempt_id,
                next_identifier: AtomicU64::new(1),
                values: Mutex::new(BTreeMap::new()),
            }
        }

        fn object_count(&self) -> Result<usize, NativeHandleStoreError> {
            self.values
                .lock()
                .map(|values| values.len())
                .map_err(|_| NativeHandleStoreError::Rejected("test store is poisoned".to_owned()))
        }

        fn check_handle(
            &self,
            handle: &NativeOpaqueHandle,
            expected_type: &NativeHandleType,
            cancellation: &CancellationToken,
        ) -> Result<(), NativeHandleStoreError> {
            cancellation
                .check()
                .map_err(|_| NativeHandleStoreError::Cancelled)?;
            handle.validate()?;
            if handle.store_identity.store_id != self.identity.store_id {
                return Err(NativeHandleStoreError::WrongStore);
            }
            if handle.store_identity.generation_id != self.identity.generation_id {
                return Err(NativeHandleStoreError::WrongGeneration);
            }
            if handle.handle_type() != expected_type {
                return Err(NativeHandleStoreError::WrongType {
                    expected: expected_type.type_id.clone(),
                    actual: handle.handle_type().type_id.clone(),
                });
            }
            Ok(())
        }
    }

    impl NativeHandleStore for TestHandleStore {
        fn identity(&self) -> NativeHandleStoreIdentity {
            self.identity
        }

        fn attempt_id(&self) -> AttemptId {
            self.attempt_id
        }

        fn resolve(
            &self,
            handle: &NativeOpaqueHandle,
            expected_type: &NativeHandleType,
            cancellation: &CancellationToken,
        ) -> Result<NativeResolvedPayload, NativeHandleStoreError> {
            self.check_handle(handle, expected_type, cancellation)?;
            let values = self.values.lock().map_err(|_| {
                NativeHandleStoreError::Rejected("test store is poisoned".to_owned())
            })?;
            let value = values
                .get(handle.identifier())
                .ok_or_else(|| NativeHandleStoreError::Missing(handle.identifier().to_owned()))?;
            if value.digest_sha256() != handle.digest_sha256().unwrap_or_default() {
                return Err(NativeHandleStoreError::DigestMismatch);
            }
            Ok(NativeResolvedPayload::checked(
                value.clone(),
                Arc::new(TestResolvedPayloadRetention),
            )?)
        }

        fn publish(
            &self,
            payload: NativeStoredPayload,
            cancellation: &CancellationToken,
        ) -> Result<NativeOpaqueHandle, NativeHandleStoreError> {
            cancellation
                .check()
                .map_err(|_| NativeHandleStoreError::Cancelled)?;
            payload.validate()?;
            let handle_type = payload.handle_type()?;
            let digest_sha256 = payload.digest_sha256();
            let generation = self.next_identifier.fetch_add(1, Ordering::AcqRel);
            let identifier = format!("handle-{generation}");
            let handle = NativeOpaqueHandle::new(
                handle_type,
                self.identity,
                identifier.clone(),
                generation,
                Some(digest_sha256),
            )?;
            self.values
                .lock()
                .map_err(|_| NativeHandleStoreError::Rejected("test store is poisoned".to_owned()))?
                .insert(identifier, Arc::new(payload));
            Ok(handle)
        }

        fn revoke(
            &self,
            handle: &NativeOpaqueHandle,
            cancellation: &CancellationToken,
        ) -> Result<(), NativeHandleStoreError> {
            self.check_handle(handle, handle.handle_type(), cancellation)?;
            let removed = self
                .values
                .lock()
                .map_err(|_| NativeHandleStoreError::Rejected("test store is poisoned".to_owned()))?
                .remove(handle.identifier());
            if removed.is_none() {
                return Err(NativeHandleStoreError::Missing(
                    handle.identifier().to_owned(),
                ));
            }
            Ok(())
        }
    }

    fn model_type() -> Result<NativeHandleType, NativeNodeContractError> {
        NativeHandleType::new(NativeHandleKind::StructuredCompute, "NOISE")
    }

    fn test_payload(value: u64) -> Result<NativeStoredPayload, NativeSamplerPayloadError> {
        Ok(NativeStoredPayload::Noise(Arc::new(
            NativeNoisePayload::random(value)?,
        )))
    }

    fn store_identity(
        store_id: u128,
        generation_id: u128,
    ) -> Result<NativeHandleStoreIdentity, NativeNodeContractError> {
        NativeHandleStoreIdentity::new(Uuid::from_u128(store_id), Uuid::from_u128(generation_id))
    }

    struct IdentityNode;

    impl NativeNode for IdentityNode {
        fn class_type(&self) -> &str {
            "IdentityModel"
        }

        fn implementation_version(&self) -> &str {
            "1"
        }

        fn execute<'a>(
            &'a self,
            context: NativeNodeContext,
            mut inputs: BTreeMap<String, NativeValue>,
        ) -> BoxFuture<'a, Result<NativeNodeOutcome, NativeNodeFailure>> {
            Box::pin(async move {
                context
                    .cancellation
                    .check()
                    .map_err(|_| NativeNodeFailure {
                        code: "execution_interrupted".to_owned(),
                        message: "native node execution was interrupted".to_owned(),
                        kind: NativeNodeFailureKind::Interrupted,
                        retryable: true,
                    })?;
                let output = inputs.remove("model").ok_or_else(|| NativeNodeFailure {
                    code: "missing_input".to_owned(),
                    message: "required model input is missing".to_owned(),
                    kind: NativeNodeFailureKind::Failure,
                    retryable: false,
                })?;
                Ok(NativeNodeOutcome::Values {
                    outputs: vec![output],
                    ui: None,
                    effects: Vec::new(),
                })
            })
        }
    }

    fn identity_descriptor() -> Result<NativeNodeDescriptor, NativeNodeContractError> {
        Ok(NativeNodeDescriptor {
            schema_version: NATIVE_NODE_CONTRACT_SCHEMA_VERSION,
            class_type: "IdentityModel".to_owned(),
            implementation_version: "1".to_owned(),
            source_schema: Some(NativeDescriptorSchemaMetadata::synthetic(
                ["model".to_owned()],
                std::iter::empty(),
                ["model".to_owned()],
            )),
            inputs: vec![NativeInputDescriptor {
                name: "model".to_owned(),
                accepted_types: NativeTypeUnion::new([NativeValueType::Handle(model_type()?)])?,
                required: true,
                hidden: false,
                lazy: false,
                cardinality: NativePortCardinality::Scalar,
                allows_literal: false,
            }],
            dynamic_inputs: Vec::new(),
            outputs: vec![NativeOutputDescriptor {
                name: "model".to_owned(),
                produced_type: NativeValueType::Handle(model_type()?),
                is_list: false,
            }],
            output_node: false,
            effect: NativeEffectClass::Pure,
            cache: NativeCachePolicy::InputIdentity,
        })
    }

    fn identity_binding() -> Result<NativeNodeBinding, NativeNodeContractError> {
        Ok(NativeNodeBinding::Executable {
            feature_id: "COMFY-NODE-0001".to_owned(),
            descriptor: identity_descriptor()?,
            presentation: NativeNodePresentation {
                display_name: "Identity Model".to_owned(),
                category: String::new(),
                description: "Passes one opaque model handle through unchanged.".to_owned(),
                output_names: vec!["model".to_owned()],
                search_aliases: vec!["model identity".to_owned()],
                is_deprecated: true,
                is_experimental: false,
            },
            node: Arc::new(IdentityNode),
        })
    }

    #[test]
    fn typed_values_cover_handles_lists_and_preserved_unknowns()
    -> Result<(), Box<dyn std::error::Error>> {
        let model_type = model_type()?;
        let model = NativeValue::Handle {
            value: NativeOpaqueHandle::new(
                model_type.clone(),
                store_identity(1, 2)?,
                "model-1",
                1,
                Some("a".repeat(64)),
            )?,
        };
        let value = NativeValue::List {
            values: vec![
                model.clone(),
                NativeValue::Primitive {
                    value: NativePrimitive::Integer(7),
                },
                NativeValue::PreservedUnknown {
                    type_name: "future.socket@2".to_owned(),
                    value: serde_json::json!({"future": true}),
                },
            ],
        };
        value.validate()?;
        assert!(NativeTypeUnion::new([NativeValueType::Handle(model_type)])?.accepts(&model));
        let media_union = NativeTypeUnion::new(
            ["FILE_3D", "KSPLAT", "PLY", "SPLAT", "SPZ"]
                .into_iter()
                .map(|type_id| {
                    NativeHandleType::new(NativeHandleKind::ThreeD, type_id)
                        .map(NativeValueType::Handle)
                })
                .collect::<Result<Vec<_>, _>>()?,
        )?;
        assert_eq!(media_union.members().len(), 5);
        assert_eq!(
            serde_json::from_slice::<NativeValue>(&serde_json::to_vec(&value)?)?,
            value
        );
        Ok(())
    }

    #[test]
    fn structured_values_keep_handles_typed_and_plain_fields_json_compatible()
    -> Result<(), Box<dyn std::error::Error>> {
        let model_type = model_type()?;
        let handle = NativeValue::Handle {
            value: NativeOpaqueHandle::new(
                model_type,
                store_identity(7, 8)?,
                "structured-model",
                1,
                Some("b".repeat(64)),
            )?,
        };
        let structured = NativeStructuredValue::checked(
            "COMFY_DYNAMICCOMBO_V3",
            BTreeMap::from([
                (
                    "choice".to_owned(),
                    NativeValue::Primitive {
                        value: NativePrimitive::String("model".to_owned()),
                    },
                ),
                ("model".to_owned(), handle.clone()),
            ]),
        )?;
        let encoded = structured.into_runtime_value()?;
        let decoded = NativeStructuredValue::from_native_value(&encoded)?
            .ok_or("typed structured value was not recognized")?;
        assert_eq!(decoded.type_name(), "COMFY_DYNAMICCOMBO_V3");
        assert_eq!(decoded.get("model"), Some(&handle));
        assert!(
            NativeTypeUnion::new([NativeValueType::NamedPreservedUnknown(
                "COMFY_DYNAMICCOMBO_V3".to_owned(),
            )])?
            .accepts(&encoded)
        );

        let plain = NativeStructuredValue::checked(
            "COMFY_DYNAMICCOMBO_V3",
            BTreeMap::from([(
                "choice".to_owned(),
                NativeValue::Primitive {
                    value: NativePrimitive::String("plain".to_owned()),
                },
            )]),
        )?
        .into_runtime_value()?;
        assert_eq!(
            plain,
            NativeValue::PreservedUnknown {
                type_name: "COMFY_DYNAMICCOMBO_V3".to_owned(),
                value: serde_json::json!({"choice": "plain"}),
            }
        );
        Ok(())
    }

    #[test]
    fn integer_primitives_preserve_full_signed_and_unsigned_ranges()
    -> Result<(), Box<dyn std::error::Error>> {
        for value in [
            NativeValue::Primitive {
                value: NativePrimitive::Integer(i64::MIN),
            },
            NativeValue::Primitive {
                value: NativePrimitive::UnsignedInteger(u64::MAX),
            },
        ] {
            assert_eq!(
                serde_json::from_slice::<NativeValue>(&serde_json::to_vec(&value)?)?,
                value
            );
            assert!(matches!(
                &value,
                NativeValue::Primitive { value }
                    if value.primitive_type() == NativePrimitiveType::Integer
            ));
        }
        Ok(())
    }

    #[test]
    fn primitive_strings_preserve_bounded_multiline_text_without_weakening_identifiers()
    -> Result<(), Box<dyn std::error::Error>> {
        let value = NativeValue::Primitive {
            value: NativePrimitive::String("first\tcolumn\r\nsecond line\u{2028}third".to_owned()),
        };
        value.validate()?;
        assert_eq!(
            serde_json::from_slice::<NativeValue>(&serde_json::to_vec(&value)?)?,
            value
        );
        for invalid in ["nul\0text", "bell\u{0007}text"] {
            assert!(matches!(
                NativeValue::Primitive {
                    value: NativePrimitive::String(invalid.to_owned()),
                }
                .validate(),
                Err(NativeNodeContractError::InvalidText {
                    field: "primitive string"
                })
            ));
        }
        assert!(matches!(
            NativeValue::Primitive {
                value: NativePrimitive::String("a".repeat(MAX_TEXT_BYTES + 1)),
            }
            .validate(),
            Err(NativeNodeContractError::InvalidText {
                field: "primitive string"
            })
        ));
        assert!(matches!(
            validate_identifier("test identifier", "line\nbreak"),
            Err(NativeNodeContractError::InvalidText {
                field: "test identifier"
            })
        ));
        Ok(())
    }

    #[test]
    fn descriptors_and_bindings_reject_ambiguous_or_mismatched_contracts()
    -> Result<(), Box<dyn std::error::Error>> {
        identity_binding()?.validate()?;
        assert!(
            NativeTypeUnion::new([NativeValueType::Handle(model_type()?), NativeValueType::Any,])
                .is_err()
        );
        let mut descriptor = identity_descriptor()?;
        descriptor.implementation_version = "2".to_owned();
        let binding = NativeNodeBinding::Executable {
            feature_id: "COMFY-NODE-0001".to_owned(),
            descriptor,
            presentation: identity_binding()?.presentation().clone(),
            node: Arc::new(IdentityNode),
        };
        assert!(matches!(
            binding.validate(),
            Err(NativeNodeContractError::BindingImplementationMismatch)
        ));
        let mut binding = identity_binding()?;
        if let NativeNodeBinding::Executable { presentation, .. } = &mut binding {
            presentation.output_names = vec!["wrong".to_owned()];
        }
        assert!(matches!(
            binding.validate(),
            Err(NativeNodeContractError::InvalidPresentationOutputs)
        ));
        Ok(())
    }

    #[test]
    fn native_descriptor_v1_decode_preserves_absent_schema_without_inference()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut legacy = identity_descriptor()?;
        legacy.schema_version = LEGACY_NATIVE_NODE_CONTRACT_SCHEMA_VERSION;
        legacy.source_schema = None;
        let encoded = serde_json::to_vec(&legacy)?;
        let decoded: NativeNodeDescriptor = serde_json::from_slice(&encoded)?;
        assert_eq!(decoded, legacy);
        decoded.validate()?;
        assert!(decoded.validate_exact_schema_v2().is_err());
        assert!(!serde_json::to_string(&decoded)?.contains("source_schema"));
        Ok(())
    }

    #[test]
    fn native_descriptor_v2_round_trips_exact_ordered_schema()
    -> Result<(), Box<dyn std::error::Error>> {
        let descriptor = identity_descriptor()?;
        descriptor.validate_exact_schema_v2()?;
        let encoded = serde_json::to_vec(&descriptor)?;
        let decoded: NativeNodeDescriptor = serde_json::from_slice(&encoded)?;
        assert_eq!(decoded, descriptor);

        let mut missing = descriptor.clone();
        missing.source_schema = None;
        assert!(matches!(
            missing.validate(),
            Err(NativeNodeContractError::InvalidSourceSchema(_))
        ));
        let mut mismatched = descriptor;
        mismatched
            .source_schema
            .as_mut()
            .ok_or("v2 source schema is missing")?
            .inputs[0]
            .name = "other".to_owned();
        assert!(matches!(
            mismatched.validate(),
            Err(NativeNodeContractError::InvalidSourceSchema(_))
        ));
        Ok(())
    }

    #[test]
    fn portable_execution_checks_cancellation_and_preserves_handle_identity()
    -> Result<(), Box<dyn std::error::Error>> {
        let (backend, authority) = CpuWorkspaceAuthority::create_backend(1024 * 1024)?;
        let scratch = authority.authorize_workspace(1024)?;
        let prompt_id = PromptId(Uuid::from_u128(10));
        let attempt_id = AttemptId(Uuid::from_u128(11));
        let handle_store = Arc::new(TestHandleStore::new(store_identity(12, 13)?, attempt_id));
        let mismatched_store = Arc::new(TestHandleStore::new(
            store_identity(12, 13)?,
            AttemptId(Uuid::from_u128(14)),
        ));
        assert!(matches!(
            NativeNodeContext::new(
                prompt_id,
                attempt_id,
                NodeId::from("identity"),
                CancellationToken::default(),
                authority.authorize_workspace(1024)?,
                mismatched_store,
            ),
            Err(NativeNodeContractError::InvalidNodeContext)
        ));
        let handle = handle_store.publish(test_payload(6)?, &CancellationToken::default())?;
        let model = NativeValue::Handle { value: handle };
        let context = NativeNodeContext::new(
            prompt_id,
            attempt_id,
            NodeId::from("identity"),
            CancellationToken::default(),
            scratch,
            handle_store.clone(),
        )?;
        let outcome = futures::executor::block_on(IdentityNode.execute(
            context,
            BTreeMap::from([("model".to_owned(), model.clone())]),
        ))?;
        assert_eq!(
            outcome,
            NativeNodeOutcome::Values {
                outputs: vec![model.clone()],
                ui: None,
                effects: Vec::new(),
            }
        );
        outcome.validate()?;

        let cancellation = CancellationToken::default();
        assert!(cancellation.cancel());
        let interrupted = futures::executor::block_on(IdentityNode.execute(
            NativeNodeContext::new(
                prompt_id,
                attempt_id,
                NodeId::from("identity"),
                cancellation,
                authority.authorize_workspace(1024)?,
                handle_store.clone(),
            )?,
            BTreeMap::from([("model".to_owned(), model)]),
        ))
        .expect_err("pre-cancelled execution must not publish an output");
        assert_eq!(interrupted.kind, NativeNodeFailureKind::Interrupted);
        assert_eq!(handle_store.object_count()?, 1);
        drop(backend);
        Ok(())
    }

    #[test]
    fn compute_session_requires_the_contexts_exact_backend_and_scratch_binding()
    -> Result<(), Box<dyn std::error::Error>> {
        let (backend, authority) = CpuWorkspaceAuthority::create_backend(1024 * 1024)?;
        let backend = Arc::new(backend);
        let (foreign_backend, foreign_authority) =
            CpuWorkspaceAuthority::create_backend(1024 * 1024)?;
        let foreign_backend = Arc::new(foreign_backend);
        let scratch = authority.authorize_workspace(1024)?;
        let prompt_id = PromptId(Uuid::from_u128(0x501));
        let attempt_id = AttemptId(Uuid::from_u128(0x502));
        let node_id = NodeId::from("compute");
        let identity = NativeNodeServiceIdentity::checked(
            Uuid::from_u128(0x503),
            attempt_id,
            node_id.clone(),
        )?;
        let compute = NativeNodeComputeSession::checked(
            identity.clone(),
            backend,
            StreamId::DEFAULT,
            &scratch,
        )?;
        assert!(matches!(
            NativeNodeComputeSession::checked(
                identity,
                foreign_backend,
                StreamId::DEFAULT,
                &scratch,
            ),
            Err(NativeNodeContractError::InvalidComputeSession)
        ));
        let store = Arc::new(TestHandleStore::new(
            store_identity(0x504, 0x505)?,
            attempt_id,
        ));
        let context = NativeNodeContext::new_with_services(
            prompt_id,
            attempt_id,
            node_id,
            CancellationToken::default(),
            scratch,
            store,
            NativeNodeServices::checked(None, None, Some(compute.clone()))?,
        )?;
        assert_eq!(
            compute.execution_context(&context)?.stream,
            StreamId::DEFAULT
        );
        assert!(matches!(
            NativeNodeContext::new_with_services(
                prompt_id,
                attempt_id,
                NodeId::from("compute"),
                CancellationToken::default(),
                foreign_authority.authorize_workspace(1024)?,
                Arc::new(TestHandleStore::new(
                    store_identity(0x506, 0x507)?,
                    attempt_id,
                )),
                NativeNodeServices::checked(None, None, Some(compute))?,
            ),
            Err(NativeNodeContractError::InvalidComputeSession)
        ));
        Ok(())
    }

    #[test]
    fn shader_service_uses_the_attempts_exact_compute_session()
    -> Result<(), Box<dyn std::error::Error>> {
        let (backend, authority) = CpuWorkspaceAuthority::create_backend(1024 * 1024)?;
        let backend = Arc::new(backend);
        let scratch = authority.authorize_workspace(1024 * 1024)?;
        let prompt_id = PromptId(Uuid::from_u128(0x551));
        let attempt_id = AttemptId(Uuid::from_u128(0x552));
        let node_id = NodeId::from("shader");
        let identity = NativeNodeServiceIdentity::checked(
            Uuid::from_u128(0x553),
            attempt_id,
            node_id.clone(),
        )?;
        let compute = NativeNodeComputeSession::checked(
            identity.clone(),
            backend.clone(),
            StreamId::DEFAULT,
            &scratch,
        )?;
        let effects = Arc::new(TestPreviewEffectService::new(identity, None));
        let services = NativeNodeServices::checked(None, Some(effects), Some(compute))?
            .with_shader(Arc::new(TestShaderExecutor));
        let context = NativeNodeContext::new_with_services(
            prompt_id,
            attempt_id,
            node_id,
            CancellationToken::default(),
            scratch,
            Arc::new(TestHandleStore::new(
                store_identity(0x554, 0x555)?,
                attempt_id,
            )),
            services,
        )?;
        let request = NativeShaderRequest {
            fragment_source: "#version 300 es\nvoid main() {}".to_owned(),
            images: vec![ImageTensor::from_f32(
                &backend,
                &context.compute_session()?.execution_context(&context)?,
                1,
                2,
                2,
                3,
                &[0.0; 12],
            )?],
            floats: Vec::new(),
            ints: Vec::new(),
            bools: Vec::new(),
            curves: Vec::new(),
            width: 2,
            height: 2,
        };
        let result = context.execute_shader(&request)?;
        assert!(
            result.outputs[0]
                .as_f32_slice()?
                .chunks_exact(4)
                .all(|pixel| pixel == [0.25, 0.5, 0.75, 1.0])
        );
        let prepared = context.execute_shader_with_previews(&request)?;
        assert_eq!(prepared.effects().len(), 2);
        assert_eq!(
            prepared.ui()["input_images"].as_array().map(Vec::len),
            Some(1)
        );
        assert_eq!(prepared.ui()["images"].as_array().map(Vec::len), Some(1));
        assert_eq!(prepared.shader().outputs.len(), MAX_SHADER_OUTPUTS);

        let cancellation = CancellationToken::default();
        assert!(cancellation.cancel());
        let scratch = authority.authorize_workspace(1024)?;
        let identity = NativeNodeServiceIdentity::checked(
            Uuid::from_u128(0x556),
            attempt_id,
            NodeId::from("shader"),
        )?;
        let compute = NativeNodeComputeSession::checked(
            identity,
            backend.clone(),
            StreamId::DEFAULT,
            &scratch,
        )?;
        let cancelled = NativeNodeContext::new_with_services(
            prompt_id,
            attempt_id,
            NodeId::from("shader"),
            cancellation,
            scratch,
            Arc::new(TestHandleStore::new(
                store_identity(0x557, 0x558)?,
                attempt_id,
            )),
            NativeNodeServices::checked(None, None, Some(compute))?
                .with_shader(Arc::new(TestShaderExecutor)),
        )?;
        assert!(matches!(
            cancelled.execute_shader(&request),
            Err(NativeShaderServiceError::Shader(
                NativeShaderError::Cancelled
            ))
        ));

        let scratch = authority.authorize_workspace(1024 * 1024)?;
        let node_id = NodeId::from("shader-preview-failure");
        let identity = NativeNodeServiceIdentity::checked(
            Uuid::from_u128(0x559),
            attempt_id,
            node_id.clone(),
        )?;
        let compute = NativeNodeComputeSession::checked(
            identity.clone(),
            backend,
            StreamId::DEFAULT,
            &scratch,
        )?;
        let failing_effects = Arc::new(TestPreviewEffectService::new(identity, Some(1)));
        let failed = NativeNodeContext::new_with_services(
            prompt_id,
            attempt_id,
            node_id,
            CancellationToken::default(),
            scratch,
            Arc::new(TestHandleStore::new(
                store_identity(0x55a, 0x55b)?,
                attempt_id,
            )),
            NativeNodeServices::checked(None, Some(failing_effects.clone()), Some(compute))?
                .with_shader(Arc::new(TestShaderExecutor)),
        )?;
        assert!(matches!(
            failed.execute_shader_with_previews(&request),
            Err(NativeShaderPreviewError::Preview(
                NativeImagePreviewError::Effect(NativeEffectServiceError::Rejected)
            ))
        ));
        assert!(
            failing_effects
                .prepared
                .lock()
                .map_err(|_| "shader preview prepared state is poisoned")?
                .is_empty()
        );
        assert_eq!(
            failing_effects
                .rolled_back
                .lock()
                .map_err(|_| "shader preview rollback state is poisoned")?
                .len(),
            1
        );
        Ok(())
    }

    #[test]
    fn image_preview_prepares_batched_pngs_and_rolls_back_partial_failure()
    -> Result<(), Box<dyn std::error::Error>> {
        let prompt_id = PromptId(Uuid::from_u128(0x581));
        let attempt_id = AttemptId(Uuid::from_u128(0x582));
        let node_id = NodeId::from("preview");
        let (backend, authority) = CpuWorkspaceAuthority::create_backend(1024 * 1024)?;
        let backend = Arc::new(backend);
        let scratch = authority.authorize_workspace(1024 * 1024)?;
        let identity = NativeNodeServiceIdentity::checked(
            Uuid::from_u128(0x583),
            attempt_id,
            node_id.clone(),
        )?;
        let compute = NativeNodeComputeSession::checked(
            identity.clone(),
            backend.clone(),
            StreamId::DEFAULT,
            &scratch,
        )?;
        let effects = Arc::new(TestPreviewEffectService::new(identity, None));
        let context = NativeNodeContext::new_with_services(
            prompt_id,
            attempt_id,
            node_id.clone(),
            CancellationToken::default(),
            scratch,
            Arc::new(TestHandleStore::new(
                store_identity(0x584, 0x585)?,
                attempt_id,
            )),
            NativeNodeServices::checked(None, Some(effects), Some(compute))?,
        )?;
        let execution_context = context.compute_session()?.execution_context(&context)?;
        let image = ImageTensor::from_f32(
            &backend,
            &execution_context,
            2,
            1,
            1,
            3,
            &[0.0, 0.5, 1.0, 1.0, 0.5, 0.0],
        )?;
        let preview = context.prepare_image_preview(&image, "preview")?;
        assert_eq!(preview.effects().len(), 2);
        assert_eq!(preview.ui()["images"].as_array().map(Vec::len), Some(2));
        assert_eq!(preview.ui()["animated"], json!([false]));

        let scratch = authority.authorize_workspace(1024 * 1024)?;
        let identity = NativeNodeServiceIdentity::checked(
            Uuid::from_u128(0x586),
            attempt_id,
            node_id.clone(),
        )?;
        let compute = NativeNodeComputeSession::checked(
            identity.clone(),
            backend,
            StreamId::DEFAULT,
            &scratch,
        )?;
        let failing_effects = Arc::new(TestPreviewEffectService::new(identity, Some(1)));
        let context = NativeNodeContext::new_with_services(
            prompt_id,
            attempt_id,
            node_id,
            CancellationToken::default(),
            scratch,
            Arc::new(TestHandleStore::new(
                store_identity(0x587, 0x588)?,
                attempt_id,
            )),
            NativeNodeServices::checked(None, Some(failing_effects.clone()), Some(compute))?,
        )?;
        assert!(matches!(
            context.prepare_image_preview(&image, "preview"),
            Err(NativeImagePreviewError::Effect(
                NativeEffectServiceError::Rejected
            ))
        ));
        assert!(
            failing_effects
                .prepared
                .lock()
                .map_err(|_| "preview prepared state is poisoned")?
                .is_empty()
        );
        assert_eq!(
            failing_effects
                .rolled_back
                .lock()
                .map_err(|_| "preview rollback state is poisoned")?
                .len(),
            1
        );
        Ok(())
    }

    #[test]
    fn attempt_local_handle_store_rejects_foreign_identity_type_and_cancellation()
    -> Result<(), Box<dyn std::error::Error>> {
        let attempt_id = AttemptId(Uuid::from_u128(20));
        assert!(matches!(
            NativeHandleStoreIdentity::new(Uuid::nil(), Uuid::from_u128(22)),
            Err(NativeNodeContractError::InvalidHandleStoreIdentity)
        ));
        let identity = store_identity(21, 22)?;
        let store = TestHandleStore::new(identity, attempt_id);
        let model_type = model_type()?;
        let handle = store.publish(test_payload(7)?, &CancellationToken::default())?;
        let resolved = store.resolve(&handle, &model_type, &CancellationToken::default())?;
        resolved.validate()?;

        let wrong_store = NativeOpaqueHandle::new(
            model_type.clone(),
            store_identity(23, 22)?,
            handle.identifier(),
            handle.generation(),
            None,
        )?;
        assert!(matches!(
            store.resolve(&wrong_store, &model_type, &CancellationToken::default()),
            Err(NativeHandleStoreError::WrongStore)
        ));
        let wrong_generation = NativeOpaqueHandle::new(
            model_type.clone(),
            store_identity(21, 24)?,
            handle.identifier(),
            handle.generation(),
            None,
        )?;
        assert!(matches!(
            store.resolve(
                &wrong_generation,
                &model_type,
                &CancellationToken::default()
            ),
            Err(NativeHandleStoreError::WrongGeneration)
        ));
        let forged_digest = NativeOpaqueHandle::new(
            model_type.clone(),
            identity,
            handle.identifier(),
            handle.generation(),
            Some("c".repeat(64)),
        )?;
        assert!(matches!(
            store.resolve(&forged_digest, &model_type, &CancellationToken::default()),
            Err(NativeHandleStoreError::DigestMismatch)
        ));
        let image_type = NativeHandleType::new(NativeHandleKind::Image, "IMAGE")?;
        assert!(matches!(
            store.resolve(&handle, &image_type, &CancellationToken::default()),
            Err(NativeHandleStoreError::WrongType { .. })
        ));

        let cancellation = CancellationToken::default();
        assert!(cancellation.cancel());
        let before = store.object_count()?;
        assert!(matches!(
            store.publish(test_payload(8)?, &cancellation),
            Err(NativeHandleStoreError::Cancelled)
        ));
        assert_eq!(store.object_count()?, before);
        Ok(())
    }

    #[test]
    fn generated_binding_validation_is_exact_and_collision_free()
    -> Result<(), Box<dyn std::error::Error>> {
        let binding = identity_binding()?;
        validate_generated_family_bindings(std::slice::from_ref(&binding), &["IdentityModel"])?;
        assert!(matches!(
            validate_generated_family_bindings(&[binding.clone(), binding], &["IdentityModel"]),
            Err(NativeNodeContractError::DuplicateGeneratedDescriptor)
        ));
        assert!(matches!(
            validate_generated_family_bindings(&[], &["IdentityModel"]),
            Err(NativeNodeContractError::GeneratedBindingMismatch)
        ));
        Ok(())
    }

    #[test]
    fn stored_payload_derives_exact_metadata_without_an_untyped_escape()
    -> Result<(), Box<dyn std::error::Error>> {
        let payload = test_payload(9)?;
        payload.validate()?;
        assert_eq!(payload.handle_type()?, model_type()?);
        assert_eq!(payload.digest_sha256().len(), 64);
        assert!(payload.resident_bytes()? > 0);
        let different = test_payload(10)?;
        different.validate()?;
        assert_ne!(payload.digest_sha256(), different.digest_sha256());
        let provider = crate::NativeProviderPayload::checked(
            NativeHandleType::new(NativeHandleKind::ProviderTask, "MODEL_TASK_ID")?,
            "signed.provider",
            "a".repeat(64),
            Vec::new(),
        )?;
        assert_eq!(provider.signed_namespace(), "signed.provider");
        assert_ne!(provider.identity_digest_sha256(), "a".repeat(64));
        assert!(matches!(
            crate::NativeProviderPayload::checked(
                NativeHandleType::new(NativeHandleKind::Image, "IMAGE")?,
                "signed.provider",
                "b".repeat(64),
                Vec::new(),
            ),
            Err(NativeStoredPayloadError::InvalidProviderPayload)
        ));
        Ok(())
    }
}
