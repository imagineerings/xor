use crate::{
    ComponentHost, ComponentHostError, InstalledVerifiedPlugin, InvocationInputs,
    capabilities::artifact_value_identity,
};
use comfy_nodes::{
    NATIVE_NODE_CONTRACT_SCHEMA_VERSION, NativeHandleKind, NativeHandleType, NativeInputDescriptor,
    NativeNode, NativeNodeContext, NativeNodeFailure, NativeNodeFailureKind, NativeNodeOutcome,
    NativeNodePresentation, NativeOutputDescriptor, NativePortCardinality,
    NativePreparedEffectRequest, NativePrimitive, NativePrimitiveType, NativeStoredArtifactObject,
    NativeStoredModelObject, NativeStoredObject, NativeStoredTensorObject, NativeTypeUnion,
    NativeValue, NativeValueType,
};
use comfy_plugin_sdk::{
    ArtifactValue, CachePolicy, EffectPolicy, ModelValue, PluginNode, PluginPort, PluginValue,
    PluginValueRepresentation, PortCardinality, PortDirection, PortPresence, ScalarValue,
    TensorValue, TypeRegistry, ValueFamily,
};
use comfy_runtime::{NativeNodeRegistry, NativeNodeRegistryError};
use futures::future::BoxFuture;
use serde_json::{Map, Number, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, sync::Arc};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum PluginRegistryAdapterError {
    #[error(transparent)]
    ComponentHost(#[from] ComponentHostError),
    #[error(transparent)]
    Registry(#[from] NativeNodeRegistryError),
    #[error("plugin node `{node}` has an invalid port contract: {message}")]
    InvalidPort { node: String, message: String },
}

pub fn registry_with_installed_plugins(
    base: &NativeNodeRegistry,
    component_host: &ComponentHost,
) -> Result<NativeNodeRegistry, PluginRegistryAdapterError> {
    registry_with_plugins(base, component_host, component_host.installed_plugins()?)
}

pub(crate) fn registry_with_plugins(
    base: &NativeNodeRegistry,
    component_host: &ComponentHost,
    plugins: Vec<InstalledVerifiedPlugin>,
) -> Result<NativeNodeRegistry, PluginRegistryAdapterError> {
    let type_registry =
        TypeRegistry::built_in().map_err(|error| PluginRegistryAdapterError::InvalidPort {
            node: "type-registry".to_owned(),
            message: error.to_string(),
        })?;
    let mut bindings = Vec::new();
    for plugin in plugins {
        for node in &plugin.manifest().nodes {
            let descriptor = native_descriptor(node, &type_registry)?;
            let implementation_version = descriptor.implementation_version.clone();
            let implementation: Arc<dyn NativeNode> = Arc::new(PluginNativeNode {
                component_host: component_host.clone(),
                plugin: plugin.clone(),
                node: node.clone(),
                type_registry: type_registry.clone(),
                implementation_version,
            });
            bindings.push((descriptor, implementation, native_presentation(node)));
        }
    }
    let mut registry = base.clone();
    registry.register_bound_batch_with_presentations(bindings)?;
    Ok(registry)
}

fn native_presentation(node: &PluginNode) -> NativeNodePresentation {
    NativeNodePresentation {
        display_name: node.display_name.clone(),
        category: node.category.clone(),
        description: String::new(),
        output_names: node
            .ports
            .iter()
            .filter(|port| port.direction == PortDirection::Output)
            .map(|port| port.name.clone())
            .collect(),
        search_aliases: Vec::new(),
        is_deprecated: false,
        is_experimental: false,
    }
}

struct PluginNativeNode {
    component_host: ComponentHost,
    plugin: InstalledVerifiedPlugin,
    node: PluginNode,
    type_registry: TypeRegistry,
    implementation_version: String,
}

impl NativeNode for PluginNativeNode {
    fn class_type(&self) -> &str {
        &self.node.id
    }

    fn implementation_version(&self) -> &str {
        &self.implementation_version
    }

    fn implementation_namespace(&self) -> &str {
        self.plugin.binding().signed_plugin_identifier()
    }

    fn execute<'a>(
        &'a self,
        context: NativeNodeContext,
        inputs: BTreeMap<String, NativeValue>,
    ) -> BoxFuture<'a, Result<NativeNodeOutcome, NativeNodeFailure>> {
        Box::pin(async move {
            context.validate().map_err(plugin_failure)?;
            let profile_id = self.plugin.authorization().capabilities().profile_id();
            let invocation_inputs = invocation_inputs(
                &self.node,
                inputs,
                &self.type_registry,
                profile_id,
                &context,
            )?;
            let result = self
                .component_host
                .execute_plugin(
                    &self.plugin,
                    &self.node.id,
                    invocation_inputs,
                    context.clone(),
                )
                .await
                .map_err(plugin_failure)?;
            let outputs = invocation_outputs(
                &self.node,
                &result.outputs,
                &result.output_presence,
                &self.type_registry,
                profile_id,
                &context,
            )?;
            let ui = (!result.effects.ui_state.is_empty() || !result.effects.logs.is_empty()).then(
                || {
                    serde_json::json!({
                        "plugin_ui": &result.effects.ui_state,
                        "plugin_logs": &result.effects.logs,
                    })
                },
            );
            let effects = if result.effects.outputs.is_empty() && result.effects.routes.is_empty() {
                Vec::new()
            } else {
                let metadata = serde_json::to_vec(&result.effects).map_err(plugin_failure)?;
                vec![NativePreparedEffectRequest {
                    transaction_id: effect_transaction_id(&context, &metadata),
                    metadata,
                }]
            };
            Ok(NativeNodeOutcome::Values {
                outputs,
                ui,
                effects,
            })
        })
    }
}

fn native_descriptor(
    node: &PluginNode,
    registry: &TypeRegistry,
) -> Result<comfy_nodes::NativeNodeDescriptor, PluginRegistryAdapterError> {
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    for port in &node.ports {
        let family = registry.family(&port.type_id).map_err(|error| {
            PluginRegistryAdapterError::InvalidPort {
                node: node.id.clone(),
                message: error.to_string(),
            }
        })?;
        let value_type = native_value_type(port, family).map_err(|error| {
            PluginRegistryAdapterError::InvalidPort {
                node: node.id.clone(),
                message: error.to_string(),
            }
        })?;
        match port.direction {
            PortDirection::Input => inputs.push(NativeInputDescriptor {
                name: port.id.clone(),
                accepted_types: NativeTypeUnion::new([value_type]).map_err(|error| {
                    PluginRegistryAdapterError::InvalidPort {
                        node: node.id.clone(),
                        message: error.to_string(),
                    }
                })?,
                required: port.presence == PortPresence::Required,
                hidden: port.hidden,
                lazy: port.lazy,
                cardinality: if port.cardinality == PortCardinality::List {
                    NativePortCardinality::List
                } else {
                    NativePortCardinality::Scalar
                },
                allows_literal: family == ValueFamily::Scalar,
            }),
            PortDirection::Output => outputs.push(NativeOutputDescriptor {
                name: port.name.clone(),
                produced_type: if port.presence == PortPresence::Optional {
                    NativeValueType::Any
                } else {
                    value_type
                },
                is_list: port.cardinality == PortCardinality::List,
            }),
        }
    }
    let descriptor = comfy_nodes::NativeNodeDescriptor {
        schema_version: NATIVE_NODE_CONTRACT_SCHEMA_VERSION,
        class_type: node.id.clone(),
        implementation_version: node.version.to_string(),
        inputs,
        dynamic_inputs: Vec::new(),
        outputs,
        output_node: node.effects != EffectPolicy::Pure,
        effect: match node.effects {
            EffectPolicy::Pure => comfy_nodes::NativeEffectClass::Pure,
            EffectPolicy::Transactional => comfy_nodes::NativeEffectClass::WritesArtifact,
            EffectPolicy::Provider => comfy_nodes::NativeEffectClass::Provider,
        },
        cache: if node.cache == CachePolicy::Never {
            comfy_nodes::NativeCachePolicy::Never
        } else {
            comfy_nodes::NativeCachePolicy::InputIdentity
        },
    };
    descriptor
        .validate()
        .map_err(|error| PluginRegistryAdapterError::InvalidPort {
            node: node.id.clone(),
            message: error.to_string(),
        })?;
    Ok(descriptor)
}

fn native_value_type(
    port: &PluginPort,
    family: ValueFamily,
) -> Result<NativeValueType, comfy_nodes::NativeNodeContractError> {
    Ok(match family {
        ValueFamily::Scalar => match port.type_id.name() {
            "boolean" => NativeValueType::Primitive(NativePrimitiveType::Boolean),
            "integer" => NativeValueType::Primitive(NativePrimitiveType::Integer),
            "float" | "number" => NativeValueType::Primitive(NativePrimitiveType::Number),
            "string" => NativeValueType::Primitive(NativePrimitiveType::String),
            "any" => NativeValueType::Any,
            _ => NativeValueType::PreservedUnknown,
        },
        ValueFamily::Tensor | ValueFamily::Artifact | ValueFamily::Model => {
            NativeValueType::Handle(native_handle_type(port, family)?)
        }
    })
}

fn native_handle_type(
    port: &PluginPort,
    family: ValueFamily,
) -> Result<NativeHandleType, comfy_nodes::NativeNodeContractError> {
    let name = port.type_id.name();
    let kind = match family {
        ValueFamily::Tensor => match name {
            "image" => NativeHandleKind::Image,
            "mask" => NativeHandleKind::Mask,
            "audio" => NativeHandleKind::Audio,
            "video" => NativeHandleKind::Video,
            "conditioning" => NativeHandleKind::Conditioning,
            "latent" => NativeHandleKind::Latent,
            _ => NativeHandleKind::Tensor,
        },
        ValueFamily::Artifact
            if name.starts_with("file-3d") || matches!(name, "load-3d" | "mesh" | "splat") =>
        {
            NativeHandleKind::ThreeD
        }
        ValueFamily::Artifact => NativeHandleKind::Artifact,
        ValueFamily::Model => match name {
            "model" => NativeHandleKind::Model,
            "clip" | "clip-vision" => NativeHandleKind::Clip,
            "vae" => NativeHandleKind::Vae,
            "control-net" => NativeHandleKind::ControlNet,
            _ => NativeHandleKind::Model,
        },
        ValueFamily::Scalar => NativeHandleKind::Artifact,
    };
    NativeHandleType::new(kind, source_type_identity(name))
}

fn source_type_identity(name: &str) -> String {
    match name {
        "integer" => "INT".to_owned(),
        "float-list" => "FLOATS".to_owned(),
        "color-list" => "COLORS".to_owned(),
        "bounding-box-editor" => "BOUNDING_BOXES".to_owned(),
        "dictionary" => "DICT".to_owned(),
        name => name.replace('-', "_").to_ascii_uppercase(),
    }
}

fn invocation_inputs(
    node: &PluginNode,
    mut values: BTreeMap<String, NativeValue>,
    registry: &TypeRegistry,
    profile_id: &str,
    context: &NativeNodeContext,
) -> Result<InvocationInputs, NativeNodeFailure> {
    let mut inputs = InvocationInputs::default();
    for port in node
        .ports
        .iter()
        .filter(|port| port.direction == PortDirection::Input)
    {
        let Some(value) = values.remove(&port.id) else {
            inputs.set_absent(&port.id);
            continue;
        };
        let values = if port.cardinality == PortCardinality::List {
            match value {
                NativeValue::List { values } => values,
                _ => return Err(invalid_value(&port.id)),
            }
        } else {
            if matches!(value, NativeValue::List { .. }) {
                return Err(invalid_value(&port.id));
            }
            vec![value]
        };
        let values = values
            .into_iter()
            .map(|value| plugin_value(port, value, registry, profile_id, context))
            .collect::<Result<Vec<_>, _>>()?;
        inputs.set_present(&port.id, values);
    }
    if let Some((unknown, _)) = values.into_iter().next() {
        return Err(invalid_value(&unknown));
    }
    Ok(inputs)
}

fn plugin_value(
    port: &PluginPort,
    value: NativeValue,
    registry: &TypeRegistry,
    profile_id: &str,
    context: &NativeNodeContext,
) -> Result<PluginValue, NativeNodeFailure> {
    let family = registry.family(&port.type_id).map_err(plugin_failure)?;
    match family {
        ValueFamily::Scalar => PluginValue::scalar(
            port.type_id.clone(),
            scalar_from_native(value, &port.type_id.to_string())?,
            registry,
        )
        .map_err(plugin_failure),
        ValueFamily::Tensor | ValueFamily::Artifact | ValueFamily::Model => {
            let NativeValue::Handle { value: handle } = value else {
                return Err(invalid_value(&port.id));
            };
            let expected_type = native_handle_type(port, family).map_err(plugin_failure)?;
            let stored = context
                .handle_store()
                .resolve(&handle, &expected_type, &context.cancellation)
                .map_err(plugin_failure)?;
            let stored = plugin_value_from_stored(port, family, stored, registry)?;
            if stored.type_id() != &port.type_id || stored.family() != family {
                return Err(invalid_value(&port.id));
            }
            validate_value_digest(&handle, &stored, &port.id)?;
            if let PluginValueRepresentation::Artifact(artifact) = stored.representation() {
                artifact_value_identity(profile_id, artifact).map_err(plugin_failure)?;
            }
            Ok(stored)
        }
    }
}

fn plugin_value_from_stored(
    port: &PluginPort,
    family: ValueFamily,
    stored: NativeStoredObject,
    registry: &TypeRegistry,
) -> Result<PluginValue, NativeNodeFailure> {
    if let Ok(value) = stored.clone().downcast::<PluginValue>() {
        return Ok((*value).clone());
    }
    match family {
        ValueFamily::Tensor => {
            let stored = stored
                .downcast::<NativeStoredTensorObject>()
                .map_err(|_| invalid_value(&port.id))?;
            let value = TensorValue::new(
                stored.descriptor().clone(),
                stored.byte_length(),
                stored.digest(),
            )
            .map_err(plugin_failure)?;
            PluginValue::tensor(port.type_id.clone(), value, registry).map_err(plugin_failure)
        }
        ValueFamily::Artifact => {
            let stored = stored
                .downcast::<NativeStoredArtifactObject>()
                .map_err(|_| invalid_value(&port.id))?;
            let value = ArtifactValue::new(
                stored.namespace(),
                stored.identifier(),
                stored.byte_length(),
                stored.digest(),
            )
            .map_err(plugin_failure)?;
            PluginValue::artifact(port.type_id.clone(), value, registry).map_err(plugin_failure)
        }
        ValueFamily::Model => {
            let stored = stored
                .downcast::<NativeStoredModelObject>()
                .map_err(|_| invalid_value(&port.id))?;
            let value = ModelValue::new(stored.identifier(), stored.format(), stored.digest())
                .map_err(plugin_failure)?;
            PluginValue::model(port.type_id.clone(), value, registry).map_err(plugin_failure)
        }
        ValueFamily::Scalar => Err(invalid_value(&port.id)),
    }
}

fn scalar_from_native(
    value: NativeValue,
    expected_unknown_type: &str,
) -> Result<ScalarValue, NativeNodeFailure> {
    match value {
        NativeValue::Primitive { value } => match value {
            NativePrimitive::Null => Ok(ScalarValue::Null),
            NativePrimitive::Boolean(value) => Ok(ScalarValue::Boolean(value)),
            NativePrimitive::Integer(value) => Ok(ScalarValue::Integer(value)),
            NativePrimitive::UnsignedInteger(value) => i64::try_from(value)
                .map(ScalarValue::Integer)
                .map_err(|_| invalid_value("unsigned-integer")),
            NativePrimitive::Number(value) => Ok(ScalarValue::Float(value)),
            NativePrimitive::String(value) => Ok(ScalarValue::String(value)),
        },
        NativeValue::PreservedUnknown { type_name, value }
            if type_name == expected_unknown_type =>
        {
            scalar_from_json(value)
        }
        _ => Err(invalid_value(expected_unknown_type)),
    }
}

fn scalar_from_json(value: Value) -> Result<ScalarValue, NativeNodeFailure> {
    match value {
        Value::Null => Ok(ScalarValue::Null),
        Value::Bool(value) => Ok(ScalarValue::Boolean(value)),
        Value::Number(value) => {
            if let Some(integer) = value.as_i64() {
                Ok(ScalarValue::Integer(integer))
            } else {
                value
                    .as_f64()
                    .map(ScalarValue::Float)
                    .ok_or_else(|| invalid_value("number"))
            }
        }
        Value::String(value) => Ok(ScalarValue::String(value)),
        Value::Array(values) => values
            .into_iter()
            .map(scalar_from_json)
            .collect::<Result<Vec<_>, _>>()
            .map(ScalarValue::List),
        Value::Object(values) => {
            let mut values = values.into_iter().collect::<Vec<_>>();
            values.sort_by(|left, right| left.0.cmp(&right.0));
            values
                .into_iter()
                .map(|(key, value)| Ok((key, scalar_from_json(value)?)))
                .collect::<Result<Vec<_>, NativeNodeFailure>>()
                .map(ScalarValue::Record)
        }
    }
}

fn invocation_outputs(
    node: &PluginNode,
    outputs: &BTreeMap<String, Vec<PluginValue>>,
    presence: &BTreeMap<String, bool>,
    registry: &TypeRegistry,
    profile_id: &str,
    context: &NativeNodeContext,
) -> Result<Vec<NativeValue>, NativeNodeFailure> {
    let mut published = Vec::new();
    let result = node
        .ports
        .iter()
        .filter(|port| port.direction == PortDirection::Output)
        .map(|port| {
            if !presence.get(&port.id).copied().unwrap_or(false) {
                return Ok(NativeValue::Primitive {
                    value: NativePrimitive::Null,
                });
            }
            let values = outputs.get(&port.id).cloned().unwrap_or_default();
            if port.cardinality == PortCardinality::List {
                values
                    .into_iter()
                    .map(|value| {
                        runtime_value(value, port, registry, profile_id, context, &mut published)
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .map(|values| NativeValue::List { values })
            } else {
                let mut values = values.into_iter();
                let value = values.next().ok_or_else(|| invalid_value(&port.id))?;
                if values.next().is_some() {
                    return Err(invalid_value(&port.id));
                }
                runtime_value(value, port, registry, profile_id, context, &mut published)
            }
        })
        .collect::<Result<Vec<_>, _>>();
    if let Err(mut failure) = result {
        let cleanup_errors = revoke_published(context, &published);
        if !cleanup_errors.is_empty() {
            failure.message.push_str("; output cleanup failed: ");
            failure.message.push_str(&cleanup_errors.join(", "));
        }
        return Err(failure);
    }
    if context.cancellation.is_cancelled() {
        let cleanup_errors = revoke_published(context, &published);
        let mut failure = plugin_failure("plugin output publication was cancelled");
        if !cleanup_errors.is_empty() {
            failure.message.push_str("; output cleanup failed: ");
            failure.message.push_str(&cleanup_errors.join(", "));
        }
        return Err(failure);
    }
    result
}

fn runtime_value(
    value: PluginValue,
    port: &PluginPort,
    registry: &TypeRegistry,
    profile_id: &str,
    context: &NativeNodeContext,
    published: &mut Vec<comfy_nodes::NativeOpaqueHandle>,
) -> Result<NativeValue, NativeNodeFailure> {
    if value.type_id() != &port.type_id {
        return Err(invalid_value(&port.id));
    }
    match value.representation() {
        PluginValueRepresentation::Scalar(value) => {
            scalar_to_native(value.clone(), &port.type_id.to_string())
        }
        PluginValueRepresentation::Artifact(artifact) => {
            artifact_value_identity(profile_id, artifact).map_err(plugin_failure)?;
            publish_plugin_value(value, port, registry, context, published)
        }
        PluginValueRepresentation::Tensor(_) | PluginValueRepresentation::Model(_) => {
            publish_plugin_value(value, port, registry, context, published)
        }
    }
}

fn publish_plugin_value(
    value: PluginValue,
    port: &PluginPort,
    registry: &TypeRegistry,
    context: &NativeNodeContext,
    published: &mut Vec<comfy_nodes::NativeOpaqueHandle>,
) -> Result<NativeValue, NativeNodeFailure> {
    let family = registry.family(&port.type_id).map_err(plugin_failure)?;
    if family == ValueFamily::Scalar || value.family() != family {
        return Err(invalid_value(&port.id));
    }
    let digest = value_digest(&value)
        .ok_or_else(|| invalid_value(&port.id))?
        .to_owned();
    let resident_bytes = value.abi_bytes().map_err(plugin_failure)?.len().max(1);
    let stored = stored_plugin_value(&value, &digest)?;
    let handle = context
        .handle_store()
        .publish(
            native_handle_type(port, family).map_err(plugin_failure)?,
            stored,
            Some(digest),
            resident_bytes,
            &context.cancellation,
        )
        .map_err(plugin_failure)?;
    published.push(handle.clone());
    Ok(NativeValue::Handle { value: handle })
}

fn stored_plugin_value(
    value: &PluginValue,
    digest: &str,
) -> Result<NativeStoredObject, NativeNodeFailure> {
    let payload: NativeStoredObject = Arc::new(value.clone());
    match value.representation() {
        PluginValueRepresentation::Tensor(value) => NativeStoredTensorObject::new(
            value.descriptor().clone(),
            value.byte_length(),
            digest,
            payload,
        )
        .map(|value| Arc::new(value) as NativeStoredObject)
        .map_err(plugin_failure),
        PluginValueRepresentation::Artifact(value) => NativeStoredArtifactObject::new(
            value.namespace(),
            value.identifier(),
            value.byte_length(),
            digest,
            payload,
        )
        .map(|value| Arc::new(value) as NativeStoredObject)
        .map_err(plugin_failure),
        PluginValueRepresentation::Model(value) => {
            NativeStoredModelObject::new(value.identifier(), value.format(), digest, payload)
                .map(|value| Arc::new(value) as NativeStoredObject)
                .map_err(plugin_failure)
        }
        PluginValueRepresentation::Scalar(_) => Err(invalid_value("scalar-output")),
    }
}

fn revoke_published(
    context: &NativeNodeContext,
    handles: &[comfy_nodes::NativeOpaqueHandle],
) -> Vec<String> {
    let cleanup_cancellation = comfy_types::CancellationToken::default();
    let mut errors = Vec::new();
    for handle in handles.iter().rev() {
        if let Err(error) = context.handle_store().revoke(handle, &cleanup_cancellation) {
            errors.push(error.to_string());
        }
    }
    errors
}

fn validate_value_digest(
    handle: &comfy_nodes::NativeOpaqueHandle,
    value: &PluginValue,
    port: &str,
) -> Result<(), NativeNodeFailure> {
    if value_digest(value) != handle.digest_sha256() {
        return Err(invalid_value(port));
    }
    Ok(())
}

fn value_digest(value: &PluginValue) -> Option<&str> {
    match value.representation() {
        PluginValueRepresentation::Tensor(value) => Some(value.digest()),
        PluginValueRepresentation::Artifact(value) => Some(value.digest()),
        PluginValueRepresentation::Model(value) => Some(value.digest()),
        PluginValueRepresentation::Scalar(_) => None,
    }
}

fn scalar_to_native(value: ScalarValue, type_name: &str) -> Result<NativeValue, NativeNodeFailure> {
    let value = match value {
        ScalarValue::Null => {
            return Ok(NativeValue::Primitive {
                value: NativePrimitive::Null,
            });
        }
        ScalarValue::Boolean(value) => {
            return Ok(NativeValue::Primitive {
                value: NativePrimitive::Boolean(value),
            });
        }
        ScalarValue::Integer(value) => {
            return Ok(NativeValue::Primitive {
                value: NativePrimitive::Integer(value),
            });
        }
        ScalarValue::Float(value) => {
            return Ok(NativeValue::Primitive {
                value: NativePrimitive::Number(value),
            });
        }
        ScalarValue::String(value) => {
            return Ok(NativeValue::Primitive {
                value: NativePrimitive::String(value),
            });
        }
        ScalarValue::Bytes(values) => Value::Array(
            values
                .into_iter()
                .map(|value| Value::Number(Number::from(value)))
                .collect(),
        ),
        ScalarValue::List(values) => Value::Array(
            values
                .into_iter()
                .map(scalar_to_json)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        ScalarValue::Record(values) => Value::Object(
            values
                .into_iter()
                .map(|(key, value)| Ok((key, scalar_to_json(value)?)))
                .collect::<Result<Map<_, _>, NativeNodeFailure>>()?,
        ),
    };
    Ok(NativeValue::PreservedUnknown {
        type_name: type_name.to_owned(),
        value,
    })
}

fn scalar_to_json(value: ScalarValue) -> Result<Value, NativeNodeFailure> {
    match value {
        ScalarValue::Null => Ok(Value::Null),
        ScalarValue::Boolean(value) => Ok(Value::Bool(value)),
        ScalarValue::Integer(value) => Ok(Value::Number(Number::from(value))),
        ScalarValue::Float(value) => Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| invalid_value("float")),
        ScalarValue::String(value) => Ok(Value::String(value)),
        ScalarValue::Bytes(values) => Ok(Value::Array(
            values
                .into_iter()
                .map(|value| Value::Number(Number::from(value)))
                .collect(),
        )),
        ScalarValue::List(values) => values
            .into_iter()
            .map(scalar_to_json)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        ScalarValue::Record(values) => values
            .into_iter()
            .map(|(key, value)| Ok((key, scalar_to_json(value)?)))
            .collect::<Result<Map<_, _>, NativeNodeFailure>>()
            .map(Value::Object),
    }
}

fn effect_transaction_id(context: &NativeNodeContext, metadata: &[u8]) -> Uuid {
    let mut hasher = Sha256::new();
    hasher.update(b"sim-comfy-plugin-effect-v1");
    hasher.update(context.prompt_id.0.as_bytes());
    hasher.update(context.attempt_id.0.as_bytes());
    hasher.update(context.node_id.0.as_bytes());
    hasher.update(metadata);
    let digest = hasher.finalize();
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    Uuid::from_bytes(bytes)
}

fn invalid_value(port: &str) -> NativeNodeFailure {
    NativeNodeFailure {
        code: "invalid_plugin_value".to_owned(),
        message: format!("plugin port `{port}` has an invalid native value"),
        kind: NativeNodeFailureKind::Failure,
        retryable: false,
    }
}

fn plugin_failure(error: impl std::fmt::Display) -> NativeNodeFailure {
    NativeNodeFailure {
        code: "plugin_invocation_failed".to_owned(),
        message: error.to_string(),
        kind: NativeNodeFailureKind::Failure,
        retryable: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use comfy_plugin_sdk::{
        ArtifactValue, DType, DeviceId, PortSerialization, StreamId, TensorDescriptor,
    };
    use std::error::Error;

    fn artifact_value(identifier: &str) -> Result<PluginValue, Box<dyn Error>> {
        let registry = TypeRegistry::built_in()?;
        Ok(PluginValue::artifact(
            registry.resolve("SVG")?.clone(),
            ArtifactValue::new("input", identifier, 3, "2".repeat(64))?,
            &registry,
        )?)
    }

    fn input_port(
        registry: &TypeRegistry,
        type_name: &str,
        serialization: PortSerialization,
    ) -> Result<PluginPort, Box<dyn Error>> {
        Ok(PluginPort {
            id: format!("{type_name}-input"),
            name: format!("{type_name} input"),
            direction: PortDirection::Input,
            type_id: registry.resolve(type_name)?.clone(),
            cardinality: PortCardinality::Singular,
            presence: PortPresence::Required,
            hidden: false,
            lazy: false,
            default: None,
            serialization,
            accepted_legacy_names: Vec::new(),
        })
    }

    #[test]
    fn neutral_stored_objects_bridge_native_and_plugin_values() -> Result<(), Box<dyn Error>> {
        let registry = TypeRegistry::built_in()?;
        let tensor_digest = "1".repeat(64);
        let tensor = PluginValue::tensor(
            registry.resolve("IMAGE")?.clone(),
            TensorValue::new(
                TensorDescriptor::contiguous(
                    vec![1],
                    DType::F32,
                    DeviceId::CPU,
                    StreamId::DEFAULT,
                )?,
                4,
                &tensor_digest,
            )?,
            &registry,
        )?;
        let artifact = artifact_value("bridge.svg")?;
        let model_digest = "3".repeat(64);
        let model = PluginValue::model(
            registry.resolve("MODEL")?.clone(),
            ModelValue::new("bridge-model", "safetensors", &model_digest)?,
            &registry,
        )?;
        let cases = [
            (
                input_port(&registry, "IMAGE", PortSerialization::Handle)?,
                ValueFamily::Tensor,
                tensor,
            ),
            (
                input_port(&registry, "SVG", PortSerialization::ArtifactReference)?,
                ValueFamily::Artifact,
                artifact,
            ),
            (
                input_port(&registry, "MODEL", PortSerialization::Handle)?,
                ValueFamily::Model,
                model,
            ),
        ];
        for (port, family, expected) in cases {
            let digest = value_digest(&expected).ok_or("fixture has no digest")?;
            let native_payload: NativeStoredObject = Arc::new("native-payload".to_owned());
            let native: NativeStoredObject = match expected.representation() {
                PluginValueRepresentation::Tensor(value) => {
                    Arc::new(NativeStoredTensorObject::new(
                        value.descriptor().clone(),
                        value.byte_length(),
                        digest,
                        native_payload,
                    )?)
                }
                PluginValueRepresentation::Artifact(value) => {
                    Arc::new(NativeStoredArtifactObject::new(
                        value.namespace(),
                        value.identifier(),
                        value.byte_length(),
                        digest,
                        native_payload,
                    )?)
                }
                PluginValueRepresentation::Model(value) => Arc::new(NativeStoredModelObject::new(
                    value.identifier(),
                    value.format(),
                    digest,
                    native_payload,
                )?),
                PluginValueRepresentation::Scalar(_) => {
                    return Err("unexpected scalar fixture".into());
                }
            };
            let projected = plugin_value_from_stored(&port, family, native, &registry)?;
            assert_eq!(projected, expected);
            let republished = stored_plugin_value(&projected, digest)?;
            let payload = match family {
                ValueFamily::Tensor => republished
                    .downcast::<NativeStoredTensorObject>()
                    .map_err(|_| "tensor was not wrapped")?
                    .payload()
                    .clone(),
                ValueFamily::Artifact => republished
                    .downcast::<NativeStoredArtifactObject>()
                    .map_err(|_| "artifact was not wrapped")?
                    .payload()
                    .clone(),
                ValueFamily::Model => republished
                    .downcast::<NativeStoredModelObject>()
                    .map_err(|_| "model was not wrapped")?
                    .payload()
                    .clone(),
                ValueFamily::Scalar => return Err("unexpected scalar fixture".into()),
            };
            assert_eq!(
                payload
                    .downcast::<PluginValue>()
                    .map_err(|_| "wrapper payload was not a plugin value")?
                    .as_ref(),
                &expected
            );
        }
        Ok(())
    }

    #[test]
    fn registry_adapter_rejects_noncanonical_artifact_abi_paths() -> Result<(), Box<dyn Error>> {
        let registry = TypeRegistry::built_in()?;
        let port = PluginPort {
            id: "artifact".to_owned(),
            name: "artifact".to_owned(),
            direction: PortDirection::Output,
            type_id: registry.resolve("SVG")?.clone(),
            cardinality: PortCardinality::Singular,
            presence: PortPresence::Required,
            hidden: false,
            lazy: false,
            default: None,
            serialization: comfy_plugin_sdk::PortSerialization::ArtifactReference,
            accepted_legacy_names: Vec::new(),
        };
        artifact_value_identity(
            "profile-a",
            match artifact_value("nested/fixture.svg")?.representation() {
                PluginValueRepresentation::Artifact(value) => value,
                _ => return Err("fixture is not an artifact".into()),
            },
        )?;
        let invalid = artifact_value("../escape.svg")?;
        let PluginValueRepresentation::Artifact(invalid) = invalid.representation() else {
            return Err("fixture is not an artifact".into());
        };
        assert!(artifact_value_identity("profile-a", invalid).is_err());
        assert_eq!(
            native_handle_type(&port, ValueFamily::Artifact)?.type_id,
            "SVG"
        );
        Ok(())
    }
}
