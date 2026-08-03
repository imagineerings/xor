use crate::{
    ComponentHost, ComponentHostError, InstalledVerifiedPlugin, InvocationInputs,
    capabilities::artifact_value_identity,
};
use comfy_plugin_sdk::{
    CachePolicy, EffectPolicy, PluginNode, PluginPort, PluginValue, PluginValueRepresentation,
    PortCardinality, PortDirection, PortPresence, ScalarValue, TypeRegistry, ValueFamily,
};
use comfy_runtime::{
    EffectClass, InputMode, NativeNode, NativeNodeRegistry, NativeNodeRegistryError, NodeContext,
    NodeFailure, NodeFailureKind, NodeOutcome, PreparedEffectRequest, RuntimeAvailability,
    RuntimeCachePolicy, RuntimeInputDescriptor, RuntimeNodeDescriptor, RuntimeNodePresentation,
    RuntimeOutputDescriptor, ValueType,
};
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
            let descriptor = runtime_descriptor(node, &type_registry)?;
            let implementation_version = descriptor.implementation_version.clone();
            let implementation: Arc<dyn NativeNode> = Arc::new(PluginNativeNode {
                component_host: component_host.clone(),
                plugin: plugin.clone(),
                node: node.clone(),
                type_registry: type_registry.clone(),
                implementation_version,
            });
            let presentation = runtime_presentation(node);
            bindings.push((descriptor, implementation, presentation));
        }
    }
    let mut registry = base.clone();
    registry.register_bound_batch_with_presentations(bindings)?;
    Ok(registry)
}

fn runtime_presentation(node: &PluginNode) -> RuntimeNodePresentation {
    RuntimeNodePresentation {
        display_name: node.display_name.clone(),
        category: node.category.clone(),
        output_names: node
            .ports
            .iter()
            .filter(|port| port.direction == PortDirection::Output)
            .map(|port| port.name.clone())
            .collect(),
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
        context: NodeContext,
        inputs: BTreeMap<String, Value>,
    ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
        Box::pin(async move {
            let profile_id = self.plugin.authorization().capabilities().profile_id();
            let invocation_inputs =
                invocation_inputs(&self.node, inputs, &self.type_registry, profile_id)?;
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
                profile_id,
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
                vec![PreparedEffectRequest {
                    transaction_id: effect_transaction_id(&context, &metadata),
                    metadata,
                }]
            };
            Ok(NodeOutcome::Values {
                outputs,
                ui,
                effects,
            })
        })
    }
}

fn runtime_descriptor(
    node: &PluginNode,
    registry: &TypeRegistry,
) -> Result<RuntimeNodeDescriptor, PluginRegistryAdapterError> {
    let mut inputs = BTreeMap::new();
    let mut outputs = Vec::new();
    for port in &node.ports {
        let family = registry.family(&port.type_id).map_err(|error| {
            PluginRegistryAdapterError::InvalidPort {
                node: node.id.clone(),
                message: error.to_string(),
            }
        })?;
        let value_type = value_type(port, family);
        match port.direction {
            PortDirection::Input => {
                inputs.insert(
                    port.id.clone(),
                    RuntimeInputDescriptor {
                        value_type,
                        required: port.presence == PortPresence::Required,
                        hidden: port.hidden,
                        lazy: port.lazy,
                        mode: if port.cardinality == PortCardinality::List {
                            InputMode::List
                        } else {
                            InputMode::Scalar
                        },
                        allows_literal: family == ValueFamily::Scalar,
                    },
                );
            }
            PortDirection::Output => outputs.push(RuntimeOutputDescriptor {
                value_type: if port.presence == PortPresence::Optional {
                    ValueType::Any
                } else {
                    value_type
                },
                is_list: port.cardinality == PortCardinality::List,
            }),
        }
    }
    Ok(RuntimeNodeDescriptor {
        class_type: node.id.clone(),
        implementation_version: node.version.to_string(),
        inputs,
        outputs,
        output_node: node.effects != EffectPolicy::Pure,
        availability: RuntimeAvailability::Native,
        effect: match node.effects {
            EffectPolicy::Pure => EffectClass::Pure,
            EffectPolicy::Transactional => EffectClass::WritesArtifact,
            EffectPolicy::Provider => EffectClass::Provider,
        },
        cache: if node.cache == CachePolicy::Never {
            RuntimeCachePolicy::Never
        } else {
            RuntimeCachePolicy::InputIdentity
        },
    })
}

fn value_type(port: &PluginPort, family: ValueFamily) -> ValueType {
    match family {
        ValueFamily::Scalar => match port.type_id.name() {
            "boolean" => ValueType::Boolean,
            "integer" => ValueType::Integer,
            "float" | "number" => ValueType::Number,
            "string" => ValueType::String,
            _ => ValueType::Custom(port.type_id.to_string()),
        },
        ValueFamily::Tensor => ValueType::Tensor,
        ValueFamily::Artifact => ValueType::Artifact,
        ValueFamily::Model => ValueType::Model,
    }
}

fn invocation_inputs(
    node: &PluginNode,
    mut values: BTreeMap<String, Value>,
    registry: &TypeRegistry,
    profile_id: &str,
) -> Result<InvocationInputs, NodeFailure> {
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
            value
                .as_array()
                .cloned()
                .ok_or_else(|| invalid_value(&port.id))?
        } else {
            vec![value]
        };
        let values = values
            .into_iter()
            .map(|value| plugin_value(port, value, registry, profile_id))
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
    value: Value,
    registry: &TypeRegistry,
    profile_id: &str,
) -> Result<PluginValue, NodeFailure> {
    match registry.family(&port.type_id).map_err(plugin_failure)? {
        ValueFamily::Scalar => {
            PluginValue::scalar(port.type_id.clone(), scalar_from_json(value)?, registry)
                .map_err(plugin_failure)
        }
        _ => {
            let value: PluginValue = serde_json::from_value(value).map_err(plugin_failure)?;
            if value.type_id() != &port.type_id {
                return Err(invalid_value(&port.id));
            }
            if let PluginValueRepresentation::Artifact(artifact) = value.representation() {
                artifact_value_identity(profile_id, artifact).map_err(plugin_failure)?;
            }
            Ok(value)
        }
    }
}

fn scalar_from_json(value: Value) -> Result<ScalarValue, NodeFailure> {
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
                .collect::<Result<Vec<_>, NodeFailure>>()
                .map(ScalarValue::Record)
        }
    }
}

fn invocation_outputs(
    node: &PluginNode,
    outputs: &BTreeMap<String, Vec<PluginValue>>,
    presence: &BTreeMap<String, bool>,
    profile_id: &str,
) -> Result<Vec<Value>, NodeFailure> {
    node.ports
        .iter()
        .filter(|port| port.direction == PortDirection::Output)
        .map(|port| {
            if !presence.get(&port.id).copied().unwrap_or(false) {
                return Ok(Value::Null);
            }
            let values = outputs.get(&port.id).cloned().unwrap_or_default();
            if port.cardinality == PortCardinality::List {
                values
                    .into_iter()
                    .map(|value| runtime_value(value, profile_id))
                    .collect::<Result<Vec<_>, _>>()
                    .map(Value::Array)
            } else {
                let mut values = values.into_iter();
                let value = values.next().ok_or_else(|| invalid_value(&port.id))?;
                if values.next().is_some() {
                    return Err(invalid_value(&port.id));
                }
                runtime_value(value, profile_id)
            }
        })
        .collect()
}

fn runtime_value(value: PluginValue, profile_id: &str) -> Result<Value, NodeFailure> {
    match value.representation() {
        PluginValueRepresentation::Scalar(value) => scalar_to_json(value.clone()),
        PluginValueRepresentation::Artifact(artifact) => {
            artifact_value_identity(profile_id, artifact).map_err(plugin_failure)?;
            serde_json::to_value(value).map_err(plugin_failure)
        }
        _ => serde_json::to_value(value).map_err(plugin_failure),
    }
}

fn scalar_to_json(value: ScalarValue) -> Result<Value, NodeFailure> {
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
            .collect::<Result<Map<_, _>, NodeFailure>>()
            .map(Value::Object),
    }
}

fn effect_transaction_id(context: &NodeContext, metadata: &[u8]) -> Uuid {
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

fn invalid_value(port: &str) -> NodeFailure {
    NodeFailure {
        code: "invalid_plugin_value".to_owned(),
        message: format!("plugin port `{port}` has an invalid runtime value"),
        kind: NodeFailureKind::Failure,
        retryable: false,
    }
}

fn plugin_failure(error: impl std::fmt::Display) -> NodeFailure {
    NodeFailure {
        code: "plugin_invocation_failed".to_owned(),
        message: error.to_string(),
        kind: NodeFailureKind::Failure,
        retryable: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use comfy_plugin_sdk::ArtifactValue;
    use std::error::Error;

    fn artifact_value(identifier: &str) -> Result<PluginValue, Box<dyn Error>> {
        let registry = TypeRegistry::built_in()?;
        Ok(PluginValue::artifact(
            registry.resolve("SVG")?.clone(),
            ArtifactValue::new("input", identifier, 3, "2".repeat(64))?,
            &registry,
        )?)
    }

    #[test]
    fn registry_adapter_rejects_noncanonical_artifact_abi_paths() -> Result<(), Box<dyn Error>> {
        let valid = runtime_value(artifact_value("nested/fixture.svg")?, "profile-a")?;
        let projected: PluginValue = serde_json::from_value(valid)?;
        assert!(matches!(
            projected.representation(),
            PluginValueRepresentation::Artifact(value)
                if value.identifier() == "nested/fixture.svg"
        ));

        let error = runtime_value(artifact_value("../escape.svg")?, "profile-a")
            .expect_err("canonical asset owner must reject traversal");
        assert_eq!(error.code, "plugin_invocation_failed");
        assert!(!error.retryable);
        Ok(())
    }
}
