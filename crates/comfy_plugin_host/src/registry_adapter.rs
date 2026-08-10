use crate::{
    ComponentHost, ComponentHostError, InstalledVerifiedPlugin, InvocationInputs,
    capabilities::artifact_value_identity,
};
use comfy_nodes::{
    NATIVE_NODE_CONTRACT_SCHEMA_VERSION, NativeHandleKind, NativeHandleStoreIdentity,
    NativeHandleType, NativeInputDescriptor, NativeNode, NativeNodeContext, NativeNodeFailure,
    NativeNodeFailureKind, NativeNodeOutcome, NativeNodePresentation, NativeOpaqueHandle,
    NativeOutputDescriptor, NativeOutputEffectRequest, NativeOutputNamespace, NativeOutputShape,
    NativePortCardinality, NativePrimitive, NativeStoredPayload, NativeTypeUnion, NativeValue,
    NativeValueType, native_plugin_source_type_projection,
};
use comfy_plugin_sdk::{
    CachePolicy, DeterminismPolicy, EffectPolicy, ModelValue, PluginNode, PluginPort, PluginValue,
    PluginValueRepresentation, PortCardinality, PortDirection, PortPresence, ProviderBindingClaim,
    ScalarValue, TensorValue, TypeRegistry, ValueFamily,
};
use comfy_runtime::{
    NativeNodeRegistry, NativeNodeRegistryError, NativeProviderBindingActivation,
    NativeProviderBindingActivationSet,
};
use futures::future::BoxFuture;
use serde_json::{Map, Number, Value};
use std::{collections::BTreeMap, sync::Arc};
use thiserror::Error;
#[cfg(test)]
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
    let generation = component_host.verified_generation()?;
    let mut ordinary_bindings = Vec::new();
    let mut provider_activation_sets = Vec::new();
    for plugin in plugins {
        let provider_binding_set = plugin.manifest().provider_binding.as_ref();
        let claims = provider_binding_set
            .into_iter()
            .flat_map(|set| &set.bindings)
            .map(|claim| (claim.node_id.as_str(), claim))
            .collect::<BTreeMap<_, _>>();
        let mut used_claims = BTreeMap::new();
        let mut provider_bindings = Vec::new();
        for node in &plugin.manifest().nodes {
            let descriptor = native_descriptor(node, &type_registry)?;
            let declared_disposition = base.binding_declared_disposition(&node.id);
            let implementation_version = if declared_disposition
                == Some(comfy_nodes::NativeNodeBindingDisposition::ProviderRequired)
            {
                base.descriptor(&node.id)
                    .ok_or_else(|| provider_binding_error(&node.id, "descriptor is missing"))?
                    .implementation_version
                    .clone()
            } else {
                descriptor.implementation_version.clone()
            };
            let implementation: Arc<dyn NativeNode> = Arc::new(PluginNativeNode {
                component_host: component_host.clone(),
                plugin: plugin.clone(),
                node: node.clone(),
                type_registry: type_registry.clone(),
                implementation_version,
            });
            match declared_disposition {
                Some(comfy_nodes::NativeNodeBindingDisposition::ProviderRequired) => {
                    let claim = claims.get(node.id.as_str()).copied().ok_or_else(|| {
                        provider_binding_error(&node.id, "signed provider claim is missing")
                    })?;
                    validate_provider_node_projection(base, node, claim)?;
                    if used_claims.insert(node.id.as_str(), claim).is_some() {
                        return Err(provider_binding_error(
                            &node.id,
                            "signed provider claim is duplicated",
                        ));
                    }
                    provider_bindings.push(NativeProviderBindingActivation::new(
                        claim.clone(),
                        implementation,
                    ));
                }
                Some(_) => {
                    return Err(provider_binding_error(
                        &node.id,
                        "plugin node collides with a non-provider native binding",
                    ));
                }
                None => {
                    if claims.contains_key(node.id.as_str()) {
                        return Err(provider_binding_error(
                            &node.id,
                            "provider claim targets a non-provider plugin node",
                        ));
                    }
                    ordinary_bindings.push((descriptor, implementation, native_presentation(node)));
                }
            }
        }
        if used_claims.len() != claims.len() {
            return Err(provider_binding_error(
                plugin.binding().signed_plugin_identifier(),
                "signed provider claim set contains an undeclared component node",
            ));
        }
        if let Some(binding_set) = provider_binding_set {
            let deployment = generation
                .components()
                .iter()
                .find(|deployment| deployment.extension_id() == plugin.extension_id())
                .ok_or_else(|| {
                    provider_binding_error(
                        plugin.extension_id(),
                        "verified component deployment is missing",
                    )
                })?;
            provider_activation_sets.push(NativeProviderBindingActivationSet::checked(
                generation.profile_id(),
                generation.generation(),
                generation.snapshot_sha256(),
                deployment.component_sha256(),
                deployment.authorization_generation(),
                binding_set.clone(),
                provider_bindings,
            )?);
        } else if !provider_bindings.is_empty() {
            return Err(provider_binding_error(
                plugin.binding().signed_plugin_identifier(),
                "provider bindings require a signed claim set",
            ));
        }
    }
    let mut registry = base.clone();
    registry.register_bound_batch_with_presentations(ordinary_bindings)?;
    for activation in provider_activation_sets {
        registry.activate_provider_binding_set(activation)?;
    }
    Ok(registry)
}

fn provider_binding_error(
    node: impl Into<String>,
    message: impl Into<String>,
) -> PluginRegistryAdapterError {
    PluginRegistryAdapterError::InvalidPort {
        node: node.into(),
        message: message.into(),
    }
}

fn validate_provider_node_projection(
    base: &NativeNodeRegistry,
    node: &PluginNode,
    claim: &ProviderBindingClaim,
) -> Result<(), PluginRegistryAdapterError> {
    let presentation = base
        .presentation(&node.id)
        .ok_or_else(|| provider_binding_error(&node.id, "presentation is missing"))?;
    let output_names = node
        .ports
        .iter()
        .filter(|port| port.direction == PortDirection::Output)
        .map(|port| port.name.as_str())
        .collect::<Vec<_>>();
    let expected_digest = base
        .provider_binding_contract_sha256(
            &node.id,
            &claim.transport_schema.to_string(),
            &claim.materializer_schema.to_string(),
        )?
        .ok_or_else(|| provider_binding_error(&node.id, "provider contract is missing"))?;
    if node.determinism != DeterminismPolicy::External
        || node.cache != CachePolicy::Never
        || node.effects != EffectPolicy::Provider
        || node.display_name != presentation.display_name
        || node.category != presentation.category
        || output_names
            != presentation
                .output_names
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
        || claim.contract_sha256 != expected_digest
    {
        return Err(provider_binding_error(
            &node.id,
            "signed provider node does not match the canonical native contract",
        ));
    }
    Ok(())
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

#[derive(Clone, Debug, Eq, PartialEq)]
enum ImportedHandle {
    Exact(NativeOpaqueHandle),
    Ambiguous,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ImportedHandles {
    store_identity: NativeHandleStoreIdentity,
    handles: BTreeMap<(String, String), ImportedHandle>,
}

impl ImportedHandles {
    fn new(store_identity: NativeHandleStoreIdentity) -> Self {
        Self {
            store_identity,
            handles: BTreeMap::new(),
        }
    }
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
            let (invocation_inputs, imported_handles) = invocation_inputs(
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
                &imported_handles,
            )?;
            let ui = (!result.effects.ui_state.is_empty()
                || !result.effects.logs.is_empty()
                || !result.effects.routes.is_empty())
            .then(|| {
                serde_json::json!({
                    "plugin_ui": &result.effects.ui_state,
                    "plugin_logs": &result.effects.logs,
                    "plugin_routes": &result.effects.routes,
                })
            });
            let effect_service = context.prepared_effects().map_err(plugin_failure)?;
            let requests = result
                .effects
                .outputs
                .iter()
                .enumerate()
                .map(|(index, proposal)| {
                    let namespace = match proposal.namespace.as_str() {
                        "output" | "outputs" => NativeOutputNamespace::Output,
                        "temp" | "temporary" => NativeOutputNamespace::Temporary,
                        _ => return Err(unmaterialized_plugin_effect("output namespace")),
                    };
                    let (filename_prefix, extension) = proposal
                        .name
                        .rsplit_once('.')
                        .ok_or_else(|| unmaterialized_plugin_effect("output filename"))?;
                    NativeOutputEffectRequest::checked(
                        namespace,
                        filename_prefix,
                        extension,
                        u32::try_from(index).map_err(plugin_failure)?,
                        NativeOutputShape::File,
                        Arc::from(proposal.bytes.clone()),
                        effect_service.maximum_output_bytes(),
                    )
                    .map_err(plugin_failure)
                })
                .collect::<Result<Vec<_>, _>>()?;
            let mut effects = Vec::with_capacity(requests.len());
            for request in requests {
                match effect_service.prepare_output(request, &context.cancellation) {
                    Ok(effect) => effects.push(effect),
                    Err(error) => {
                        for effect in effects.iter().rev() {
                            effect_service
                                .rollback_prepared(effect)
                                .map_err(plugin_failure)?;
                        }
                        return Err(plugin_failure(error));
                    }
                }
            }
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
                accepted_types: NativeTypeUnion::new([value_type.clone()]).map_err(|error| {
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
                allows_literal: !matches!(value_type, NativeValueType::Handle(_)),
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
    let source_schema = comfy_nodes::NativeDescriptorSchemaMetadata::compatibility(
        comfy_nodes::NativeSchemaProvenance::Plugin,
        node.ports
            .iter()
            .filter(|port| port.direction == PortDirection::Input)
            .map(|port| (port.id.clone(), port.type_id.to_string())),
        std::iter::empty(),
        node.ports
            .iter()
            .filter(|port| port.direction == PortDirection::Output)
            .map(|port| (port.name.clone(), port.type_id.to_string())),
    );
    let descriptor = comfy_nodes::NativeNodeDescriptor {
        schema_version: NATIVE_NODE_CONTRACT_SCHEMA_VERSION,
        class_type: node.id.clone(),
        implementation_version: node.version.to_string(),
        source_schema: Some(source_schema),
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
    let value_type = native_plugin_source_type_projection(port.type_id.name())
        .map_err(|error| {
            comfy_nodes::NativeNodeContractError::InvalidSourceSchema(error.to_string())
        })?
        .value_type()
        .map_err(|error| {
            comfy_nodes::NativeNodeContractError::InvalidSourceSchema(error.to_string())
        })?;
    let representation_matches = match family {
        ValueFamily::Scalar => !matches!(value_type, NativeValueType::Handle(_)),
        ValueFamily::Tensor | ValueFamily::Artifact | ValueFamily::Model => {
            matches!(value_type, NativeValueType::Handle(_))
        }
    };
    if !representation_matches {
        return Err(comfy_nodes::NativeNodeContractError::InvalidSourceSchema(
            format!(
                "plugin type `{}` disagrees with canonical source value class",
                port.type_id
            ),
        ));
    }
    Ok(value_type)
}

fn native_handle_type(
    port: &PluginPort,
) -> Result<NativeHandleType, comfy_nodes::NativeNodeContractError> {
    native_plugin_source_type_projection(port.type_id.name())
        .map_err(|error| {
            comfy_nodes::NativeNodeContractError::InvalidSourceSchema(error.to_string())
        })?
        .handle_type()
        .map_err(|error| {
            comfy_nodes::NativeNodeContractError::InvalidSourceSchema(error.to_string())
        })?
        .ok_or_else(|| {
            comfy_nodes::NativeNodeContractError::InvalidSourceSchema(format!(
                "plugin type `{}` is not handle-backed",
                port.type_id
            ))
        })
}

fn invocation_inputs(
    node: &PluginNode,
    mut values: BTreeMap<String, NativeValue>,
    registry: &TypeRegistry,
    profile_id: &str,
    context: &NativeNodeContext,
) -> Result<(InvocationInputs, ImportedHandles), NativeNodeFailure> {
    let mut inputs = InvocationInputs::default();
    let mut imported_handles = ImportedHandles::new(context.handle_store().identity());
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
        let mut plugin_values = Vec::with_capacity(values.len());
        for value in values {
            let (plugin_value, imported_handle) =
                plugin_value(port, value, registry, profile_id, context)?;
            if let Some((key, handle)) = imported_handle {
                remember_imported_handle(&mut imported_handles, key, handle);
            }
            plugin_values.push(plugin_value);
        }
        inputs.set_present(&port.id, plugin_values);
    }
    if let Some((unknown, _)) = values.into_iter().next() {
        return Err(invalid_value(&unknown));
    }
    Ok((inputs, imported_handles))
}

fn remember_imported_handle(
    imported_handles: &mut ImportedHandles,
    key: (String, String),
    handle: NativeOpaqueHandle,
) {
    imported_handles
        .handles
        .entry(key)
        .and_modify(|imported| {
            if matches!(imported, ImportedHandle::Exact(existing) if existing != &handle) {
                *imported = ImportedHandle::Ambiguous;
            }
        })
        .or_insert(ImportedHandle::Exact(handle));
}

fn plugin_value(
    port: &PluginPort,
    value: NativeValue,
    registry: &TypeRegistry,
    profile_id: &str,
    context: &NativeNodeContext,
) -> Result<(PluginValue, Option<((String, String), NativeOpaqueHandle)>), NativeNodeFailure> {
    let family = registry.family(&port.type_id).map_err(plugin_failure)?;
    match family {
        ValueFamily::Scalar => Ok((
            PluginValue::scalar(
                port.type_id.clone(),
                scalar_from_native(value, &port.type_id.to_string())?,
                registry,
            )
            .map_err(plugin_failure)?,
            None,
        )),
        ValueFamily::Tensor | ValueFamily::Artifact | ValueFamily::Model => {
            let NativeValue::Handle { value: handle } = value else {
                return Err(invalid_value(&port.id));
            };
            let expected_type = native_handle_type(port).map_err(plugin_failure)?;
            let stored = context
                .handle_store()
                .resolve(&handle, &expected_type, &context.cancellation)
                .map_err(plugin_failure)?;
            let provider_semantic_digest = match stored.as_ref() {
                NativeStoredPayload::Provider(payload) => {
                    Some(payload.semantic_digest_sha256().to_owned())
                }
                _ => None,
            };
            let stored = plugin_value_from_stored(port, family, stored.as_ref(), registry)?;
            if stored.type_id() != &port.type_id || stored.family() != family {
                return Err(invalid_value(&port.id));
            }
            if let Some(expected_digest) = provider_semantic_digest {
                if value_digest(&stored) != Some(expected_digest.as_str()) {
                    return Err(invalid_value(&port.id));
                }
            } else {
                validate_value_digest(&handle, &stored, &port.id)?;
            }
            if let PluginValueRepresentation::Artifact(artifact) = stored.representation() {
                artifact_value_identity(profile_id, artifact).map_err(plugin_failure)?;
            }
            let digest = value_digest(&stored)
                .ok_or_else(|| invalid_value(&port.id))?
                .to_owned();
            Ok((stored, Some(((port.type_id.to_string(), digest), handle))))
        }
    }
}

fn plugin_value_from_stored(
    port: &PluginPort,
    family: ValueFamily,
    stored: &NativeStoredPayload,
    registry: &TypeRegistry,
) -> Result<PluginValue, NativeNodeFailure> {
    match stored {
        NativeStoredPayload::Provider(stored) => {
            let expected_type = native_handle_type(port).map_err(plugin_failure)?;
            if expected_type.kind != NativeHandleKind::ProviderTask
                || stored.handle_type() != &expected_type
            {
                return Err(invalid_value(&port.id));
            }
            let value = PluginValue::from_abi_bytes(stored.abi_bytes(), registry)
                .map_err(plugin_failure)?;
            if value.type_id() != &port.type_id || value.family() != family {
                return Err(invalid_value(&port.id));
            }
            Ok(value)
        }
        NativeStoredPayload::Tensor(stored) => {
            if family != ValueFamily::Tensor {
                return Err(invalid_value(&port.id));
            }
            let projection = stored.projection();
            let value = TensorValue::new(
                projection.descriptor().clone(),
                projection.resident_bytes(),
                projection.content_digest(),
            )
            .map_err(plugin_failure)?;
            PluginValue::tensor(port.type_id.clone(), value, registry).map_err(plugin_failure)
        }
        NativeStoredPayload::Model(stored) => {
            if family != ValueFamily::Model {
                return Err(invalid_value(&port.id));
            }
            let identity = stored.model_payload().identity();
            let value = ModelValue::new(
                identity.identifier(),
                identity.format(),
                stored.digest_sha256(),
            )
            .map_err(plugin_failure)?;
            PluginValue::model(port.type_id.clone(), value, registry).map_err(plugin_failure)
        }
        NativeStoredPayload::Control(stored) => {
            if family != ValueFamily::Model {
                return Err(invalid_value(&port.id));
            }
            let value = ModelValue::new(
                stored.digest_sha256(),
                "sim-native-control-v1",
                stored.digest_sha256(),
            )
            .map_err(plugin_failure)?;
            PluginValue::model(port.type_id.clone(), value, registry).map_err(plugin_failure)
        }
        NativeStoredPayload::Guider(stored) => {
            if family != ValueFamily::Model {
                return Err(invalid_value(&port.id));
            }
            let identity = stored.model().identity();
            let value = ModelValue::new(
                identity.identifier(),
                "sim-native-guider-v1",
                stored.semantic_digest_sha256(),
            )
            .map_err(plugin_failure)?;
            PluginValue::model(port.type_id.clone(), value, registry).map_err(plugin_failure)
        }
        NativeStoredPayload::Sampler(stored) => {
            if family != ValueFamily::Model {
                return Err(invalid_value(&port.id));
            }
            let value = ModelValue::new(
                stored.identity().as_str(),
                "sim-native-sampler-v1",
                stored.semantic_digest_sha256(),
            )
            .map_err(plugin_failure)?;
            PluginValue::model(port.type_id.clone(), value, registry).map_err(plugin_failure)
        }
        NativeStoredPayload::Conditioning(_)
        | NativeStoredPayload::Noise(_)
        | NativeStoredPayload::BoundingBox(_)
        | NativeStoredPayload::FaceLandmarks(_)
        | NativeStoredPayload::PoseKeypoint(_)
        | NativeStoredPayload::Sam3TrackData(_)
        | NativeStoredPayload::Tracks(_)
        | NativeStoredPayload::AudioEncoderOutput(_)
        | NativeStoredPayload::ClipVisionOutput(_)
        | NativeStoredPayload::IcLoraParameters(_)
        | NativeStoredPayload::LossMap(_)
        | NativeStoredPayload::Audio(_)
        | NativeStoredPayload::Video(_)
        | NativeStoredPayload::Artifact(_)
        | NativeStoredPayload::File3D(_)
        | NativeStoredPayload::Camera(_)
        | NativeStoredPayload::Splat(_)
        | NativeStoredPayload::Mesh(_)
        | NativeStoredPayload::Voxel(_) => Err(unmaterialized_plugin_input(&port.id)),
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
    imported_handles: &ImportedHandles,
) -> Result<Vec<NativeValue>, NativeNodeFailure> {
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
                    .map(|value| runtime_value(value, port, registry, profile_id, imported_handles))
                    .collect::<Result<Vec<_>, _>>()
                    .map(|values| NativeValue::List { values })
            } else {
                let mut values = values.into_iter();
                let value = values.next().ok_or_else(|| invalid_value(&port.id))?;
                if values.next().is_some() {
                    return Err(invalid_value(&port.id));
                }
                runtime_value(value, port, registry, profile_id, imported_handles)
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    if context.cancellation.is_cancelled() {
        return Err(plugin_failure("plugin output projection was cancelled"));
    }
    Ok(result)
}

fn runtime_value(
    value: PluginValue,
    port: &PluginPort,
    registry: &TypeRegistry,
    profile_id: &str,
    imported_handles: &ImportedHandles,
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
            imported_runtime_value(value, port, registry, imported_handles)
        }
        PluginValueRepresentation::Tensor(_) | PluginValueRepresentation::Model(_) => {
            imported_runtime_value(value, port, registry, imported_handles)
        }
    }
}

fn imported_runtime_value(
    value: PluginValue,
    port: &PluginPort,
    registry: &TypeRegistry,
    imported_handles: &ImportedHandles,
) -> Result<NativeValue, NativeNodeFailure> {
    let family = registry.family(&port.type_id).map_err(plugin_failure)?;
    if family == ValueFamily::Scalar || value.family() != family {
        return Err(invalid_value(&port.id));
    }
    let digest = value_digest(&value).ok_or_else(|| invalid_value(&port.id))?;
    let key = (port.type_id.to_string(), digest.to_owned());
    let ImportedHandle::Exact(handle) = imported_handles
        .handles
        .get(&key)
        .ok_or_else(|| unmaterialized_plugin_output(&port.id))?
    else {
        return Err(unmaterialized_plugin_output(&port.id));
    };
    let expected = native_handle_type(port).map_err(plugin_failure)?;
    if handle.handle_type() != &expected
        || handle.store_identity() != imported_handles.store_identity
        || (expected.kind != NativeHandleKind::ProviderTask
            && handle.digest_sha256() != Some(digest))
    {
        return Err(invalid_value(&port.id));
    }
    Ok(NativeValue::Handle {
        value: handle.clone(),
    })
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

fn invalid_value(port: &str) -> NativeNodeFailure {
    NativeNodeFailure {
        code: "invalid_plugin_value".to_owned(),
        message: format!("plugin port `{port}` has an invalid native value"),
        kind: NativeNodeFailureKind::Failure,
        retryable: false,
    }
}

fn unmaterialized_plugin_effect(kind: &str) -> NativeNodeFailure {
    NativeNodeFailure {
        code: "unmaterialized_plugin_effect".to_owned(),
        message: format!("plugin {kind} effects are not materializable at the native boundary"),
        kind: NativeNodeFailureKind::Failure,
        retryable: false,
    }
}

fn unmaterialized_plugin_output(port: &str) -> NativeNodeFailure {
    NativeNodeFailure {
        code: "unmaterialized_plugin_payload".to_owned(),
        message: format!(
            "plugin port `{port}` returned non-scalar metadata without a host-materialized payload"
        ),
        kind: NativeNodeFailureKind::Failure,
        retryable: false,
    }
}

fn unmaterialized_plugin_input(port: &str) -> NativeNodeFailure {
    NativeNodeFailure {
        code: "unmaterialized_plugin_payload".to_owned(),
        message: format!(
            "plugin port `{port}` has no lossless representation in the plugin SDK value families"
        ),
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
    use comfy_model::ClipVisionOutput;
    use comfy_nodes::NativePrimitiveType;
    use comfy_plugin_sdk::{
        ArtifactValue, DType, DeviceId, PortSerialization, StreamId, TensorDescriptor,
    };
    use comfy_runtime::{AttemptId, NativeHandleStoreGeneration, PromptId};
    use comfy_tensor::{CancellationToken, CpuBackend, CpuWorkspaceAuthority, Tensor};
    use comfy_types::NodeId;
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

    fn opaque_handle(
        handle_type: NativeHandleType,
        identifier: &str,
        generation: u64,
        digest: &str,
    ) -> Result<NativeOpaqueHandle, Box<dyn Error>> {
        opaque_handle_with_identity(
            handle_type,
            test_store_identity()?,
            identifier,
            generation,
            digest,
        )
    }

    fn opaque_handle_with_identity(
        handle_type: NativeHandleType,
        store_identity: NativeHandleStoreIdentity,
        identifier: &str,
        generation: u64,
        digest: &str,
    ) -> Result<NativeOpaqueHandle, Box<dyn Error>> {
        Ok(NativeOpaqueHandle::new(
            handle_type,
            store_identity,
            identifier,
            generation,
            Some(digest.to_owned()),
        )?)
    }

    fn test_store_identity() -> Result<NativeHandleStoreIdentity, Box<dyn Error>> {
        Ok(NativeHandleStoreIdentity::new(
            Uuid::from_u128(0x100),
            Uuid::from_u128(0x200),
        )?)
    }

    fn tensor_value(digest: &str) -> Result<PluginValue, Box<dyn Error>> {
        let registry = TypeRegistry::built_in()?;
        Ok(PluginValue::tensor(
            registry.resolve("IMAGE")?.clone(),
            TensorValue::new(
                TensorDescriptor::contiguous(
                    vec![1],
                    DType::F32,
                    DeviceId::CPU,
                    StreamId::DEFAULT,
                )?,
                4,
                digest,
            )?,
            &registry,
        )?)
    }

    fn model_value(digest: &str) -> Result<PluginValue, Box<dyn Error>> {
        model_value_for("MODEL", "model", "safetensors", digest)
    }

    fn model_value_for(
        type_name: &str,
        identifier: &str,
        format: &str,
        digest: &str,
    ) -> Result<PluginValue, Box<dyn Error>> {
        let registry = TypeRegistry::built_in()?;
        Ok(PluginValue::model(
            registry.resolve(type_name)?.clone(),
            ModelValue::new(identifier, format, digest)?,
            &registry,
        )?)
    }

    fn clip_vision_output_tensor(
        backend: &CpuBackend,
        authority: &CpuWorkspaceAuthority,
        cancellation: &CancellationToken,
        shape: Vec<u64>,
        value: f32,
    ) -> Result<Tensor, Box<dyn Error>> {
        let elements = shape
            .iter()
            .try_fold(1_u64, |total, dimension| total.checked_mul(*dimension))
            .ok_or("clip vision output tensor element count overflowed")?;
        let descriptor =
            TensorDescriptor::contiguous(shape, DType::F32, DeviceId::CPU, StreamId::DEFAULT)?;
        let context = backend.execution_context(
            StreamId::DEFAULT,
            authority.authorize_workspace(
                elements
                    .checked_mul(4)
                    .ok_or("clip vision output tensor byte count overflowed")?,
            )?,
            cancellation,
        );
        Ok(backend
            .upload_f32(
                descriptor,
                &vec![value; usize::try_from(elements)?],
                &context,
            )?
            .0)
    }

    #[test]
    fn signed_plugin_families_delegate_to_canonical_source_projections()
    -> Result<(), Box<dyn Error>> {
        let registry = TypeRegistry::built_in()?;
        let cases = [
            (
                "String",
                NativeValueType::Primitive(NativePrimitiveType::String),
            ),
            (
                "Image",
                NativeValueType::Handle(NativeHandleType::new(NativeHandleKind::Image, "IMAGE")?),
            ),
            (
                "SVG",
                NativeValueType::Handle(NativeHandleType::new(NativeHandleKind::Artifact, "SVG")?),
            ),
            (
                "Model",
                NativeValueType::Handle(NativeHandleType::new(NativeHandleKind::Model, "MODEL")?),
            ),
            (
                "AudioEncoderOutput",
                NativeValueType::Handle(NativeHandleType::new(
                    NativeHandleKind::StructuredCompute,
                    "AUDIO_ENCODER_OUTPUT",
                )?),
            ),
            (
                "File3DGLB",
                NativeValueType::Handle(NativeHandleType::new(
                    NativeHandleKind::ThreeD,
                    "FILE_3D_GLB",
                )?),
            ),
        ];
        for (source_type, expected) in cases {
            let port = input_port(&registry, source_type, PortSerialization::Handle)?;
            assert_eq!(
                native_value_type(&port, registry.family(&port.type_id)?)?,
                expected
            );
        }
        let curve = input_port(&registry, "Curve", PortSerialization::Handle)?;
        assert!(native_value_type(&curve, registry.family(&curve.type_id)?).is_err());
        Ok(())
    }

    #[test]
    fn imported_tensor_artifact_and_model_handles_round_trip_exactly() -> Result<(), Box<dyn Error>>
    {
        let registry = TypeRegistry::built_in()?;
        let digest = "a".repeat(64);
        let cases = [
            (
                input_port(&registry, "IMAGE", PortSerialization::Handle)?,
                tensor_value(&digest)?,
            ),
            (
                input_port(&registry, "SVG", PortSerialization::ArtifactReference)?,
                PluginValue::artifact(
                    registry.resolve("SVG")?.clone(),
                    ArtifactValue::new("input", "fixture.svg", 4, &digest)?,
                    &registry,
                )?,
            ),
            (
                input_port(&registry, "MODEL", PortSerialization::Handle)?,
                model_value(&digest)?,
            ),
        ];
        for (index, (port, value)) in cases.into_iter().enumerate() {
            let handle = opaque_handle(
                native_handle_type(&port)?,
                &format!("imported-{index}"),
                u64::try_from(index)? + 1,
                &digest,
            )?;
            let key = (port.type_id.to_string(), digest.clone());
            let mut imported = ImportedHandles::new(test_store_identity()?);
            remember_imported_handle(&mut imported, key, handle.clone());
            assert_eq!(
                runtime_value(value, &port, &registry, "profile-a", &imported)?,
                NativeValue::Handle { value: handle }
            );
        }
        Ok(())
    }

    #[test]
    fn explicit_model_family_handles_round_trip_without_mutating_imports()
    -> Result<(), Box<dyn Error>> {
        let registry = TypeRegistry::built_in()?;
        for (index, type_name) in [
            "Model",
            "Clip",
            "Vae",
            "ControlNet",
            "Guider",
            "Sampler",
            "ClipVision",
        ]
        .into_iter()
        .enumerate()
        {
            let digest = format!("{index:x}").repeat(64);
            let port = input_port(&registry, type_name, PortSerialization::Handle)?;
            let value = model_value_for(
                type_name,
                &format!("{type_name}-identity"),
                "sim-native-test-v1",
                &digest,
            )?;
            let handle = opaque_handle(
                native_handle_type(&port)?,
                &format!("imported-{type_name}"),
                u64::try_from(index)? + 1,
                &digest,
            )?;
            let mut imported = ImportedHandles::new(test_store_identity()?);
            remember_imported_handle(
                &mut imported,
                (port.type_id.to_string(), digest),
                handle.clone(),
            );
            let before = imported.clone();
            assert_eq!(
                runtime_value(value, &port, &registry, "profile-a", &imported)?,
                NativeValue::Handle { value: handle }
            );
            assert_eq!(imported, before);
        }
        Ok(())
    }

    #[test]
    fn clip_vision_output_input_rejection_preserves_the_canonical_store_entry()
    -> Result<(), Box<dyn Error>> {
        let registry = TypeRegistry::built_in()?;
        let port = input_port(&registry, "ClipVisionOutput", PortSerialization::Handle)?;
        let cancellation = CancellationToken::default();
        let (backend, authority) = CpuWorkspaceAuthority::create_backend(1024 * 1024)?;
        let hidden =
            clip_vision_output_tensor(&backend, &authority, &cancellation, vec![1, 2, 2], 1.0)?;
        let embeds =
            clip_vision_output_tensor(&backend, &authority, &cancellation, vec![1, 2], 2.0)?;
        let output = Arc::new(ClipVisionOutput::checked(
            hidden,
            None,
            embeds,
            None,
            vec![[3, 32, 32]],
        )?);
        let payload = NativeStoredPayload::ClipVisionOutput(output.clone());
        let payload_bytes = payload.resident_bytes()?;
        let generation = NativeHandleStoreGeneration::with_capacities(1, payload_bytes)?;
        let attempt_id = AttemptId(Uuid::from_u128(0x300));
        let store = generation.handle_store_for_attempt(attempt_id);
        let handle = store.publish(payload, &cancellation)?;
        let expected_type = NativeHandleType::new(
            NativeHandleKind::StructuredCompute,
            ClipVisionOutput::SOURCE_TYPE_ID,
        )?;
        assert_eq!(handle.handle_type(), &expected_type);
        let before_len = generation.len();
        let before_resident_bytes = generation.resident_bytes();
        let context = NativeNodeContext::new(
            PromptId(Uuid::from_u128(0x400)),
            attempt_id,
            NodeId::from("clip-vision-output-plugin-input"),
            cancellation.clone(),
            authority.authorize_workspace(0)?,
            store.clone(),
        )?;

        let failure = plugin_value(
            &port,
            NativeValue::Handle {
                value: handle.clone(),
            },
            &registry,
            "profile-a",
            &context,
        )
        .expect_err("CLIP_VISION_OUTPUT has no lossless plugin SDK representation");
        assert_eq!(failure.code, "unmaterialized_plugin_payload");
        assert_eq!(generation.len(), before_len);
        assert_eq!(generation.resident_bytes(), before_resident_bytes);

        let resolved = store.resolve(&handle, &expected_type, &cancellation)?;
        let NativeStoredPayload::ClipVisionOutput(resolved_output) = resolved.as_ref() else {
            return Err("resolved CLIP_VISION_OUTPUT changed stored payload variant".into());
        };
        assert!(Arc::ptr_eq(resolved_output, &output));
        assert_eq!(
            resolved_output.semantic_digest_sha256(),
            output.semantic_digest_sha256()
        );
        Ok(())
    }

    #[test]
    fn explicit_stored_variants_are_exhaustively_projected_or_rejected()
    -> Result<(), Box<dyn Error>> {
        let source = include_str!("registry_adapter.rs");
        let projection = source
            .split_once("fn plugin_value_from_stored(")
            .and_then(|(_, source)| source.split_once("fn scalar_from_native("))
            .map(|(projection, _)| projection)
            .ok_or("stored payload projection function is missing")?;
        for projected in [
            "NativeStoredPayload::Provider(stored)",
            "NativeStoredPayload::Tensor(stored)",
            "NativeStoredPayload::Model(stored)",
            "NativeStoredPayload::Control(stored)",
            "NativeStoredPayload::Guider(stored)",
            "NativeStoredPayload::Sampler(stored)",
        ] {
            assert!(
                projection.contains(projected),
                "missing projection {projected}"
            );
        }
        for rejected in [
            "NativeStoredPayload::Conditioning(_)",
            "NativeStoredPayload::Noise(_)",
            "NativeStoredPayload::BoundingBox(_)",
            "NativeStoredPayload::FaceLandmarks(_)",
            "NativeStoredPayload::PoseKeypoint(_)",
            "NativeStoredPayload::Sam3TrackData(_)",
            "NativeStoredPayload::Tracks(_)",
            "NativeStoredPayload::AudioEncoderOutput(_)",
            "NativeStoredPayload::ClipVisionOutput(_)",
            "NativeStoredPayload::IcLoraParameters(_)",
            "NativeStoredPayload::LossMap(_)",
        ] {
            assert!(
                projection.contains(rejected),
                "missing rejection {rejected}"
            );
        }
        assert!(projection.contains("Err(unmaterialized_plugin_input(&port.id))"));
        assert!(!projection.contains("StructuredCompute"));
        assert!(!projection.contains("serde_json"));
        assert!(!projection.contains("remember_imported_handle"));
        assert!(!projection.contains(".publish("));
        assert!(!projection.contains(".revoke("));
        assert!(projection.contains("stored.model_payload().identity()"));
        assert!(!projection.contains("stored.diffusion()"));

        let failure = unmaterialized_plugin_input("tracks");
        assert_eq!(failure.code, "unmaterialized_plugin_payload");
        assert!(failure.message.contains("no lossless representation"));
        let failure = unmaterialized_plugin_input("conditioning");
        assert_eq!(failure.code, "unmaterialized_plugin_payload");
        assert!(failure.message.contains("no lossless representation"));
        Ok(())
    }

    #[test]
    fn forged_or_unmaterialized_plugin_outputs_fail_without_changing_imports()
    -> Result<(), Box<dyn Error>> {
        let registry = TypeRegistry::built_in()?;
        let digest = "b".repeat(64);
        let port = input_port(&registry, "IMAGE", PortSerialization::Handle)?;
        let value = tensor_value(&digest)?;
        let mut imported = ImportedHandles::new(test_store_identity()?);
        let wrong_type = opaque_handle(
            NativeHandleType::new(NativeHandleKind::Model, "MODEL")?,
            "wrong-type",
            1,
            &digest,
        )?;
        remember_imported_handle(
            &mut imported,
            (port.type_id.to_string(), digest.clone()),
            wrong_type,
        );
        let before = imported.clone();
        let failure = runtime_value(value.clone(), &port, &registry, "profile-a", &imported)
            .expect_err("a wrong-type imported handle must fail closed");
        assert_eq!(failure.code, "invalid_plugin_value");
        assert_eq!(imported, before);

        let unmaterialized = ImportedHandles::new(test_store_identity()?);
        let failure = runtime_value(
            value.clone(),
            &port,
            &registry,
            "profile-a",
            &unmaterialized,
        )
        .expect_err("metadata-only output must not allocate a native handle");
        assert_eq!(failure.code, "unmaterialized_plugin_payload");
        assert!(unmaterialized.handles.is_empty());

        let first = opaque_handle(native_handle_type(&port)?, "first", 1, &digest)?;
        let second = opaque_handle(native_handle_type(&port)?, "second", 2, &digest)?;
        let mut ambiguous = ImportedHandles::new(test_store_identity()?);
        let key = (port.type_id.to_string(), digest);
        remember_imported_handle(&mut ambiguous, key.clone(), first);
        remember_imported_handle(&mut ambiguous, key, second);
        let failure = runtime_value(value, &port, &registry, "profile-a", &ambiguous)
            .expect_err("equal metadata from distinct handles must not alias");
        assert_eq!(failure.code, "unmaterialized_plugin_payload");

        for (label, handle) in [
            (
                "store",
                opaque_handle_with_identity(
                    native_handle_type(&port)?,
                    NativeHandleStoreIdentity::new(Uuid::from_u128(0x101), Uuid::from_u128(0x200))?,
                    "wrong-store",
                    1,
                    &"b".repeat(64),
                )?,
            ),
            (
                "generation",
                opaque_handle_with_identity(
                    native_handle_type(&port)?,
                    NativeHandleStoreIdentity::new(Uuid::from_u128(0x100), Uuid::from_u128(0x201))?,
                    "wrong-generation",
                    1,
                    &"b".repeat(64),
                )?,
            ),
            (
                "digest",
                opaque_handle(
                    native_handle_type(&port)?,
                    "wrong-digest",
                    1,
                    &"c".repeat(64),
                )?,
            ),
        ] {
            let mut forged = ImportedHandles::new(test_store_identity()?);
            remember_imported_handle(
                &mut forged,
                (port.type_id.to_string(), "b".repeat(64)),
                handle,
            );
            let failure = runtime_value(
                tensor_value(&"b".repeat(64))?,
                &port,
                &registry,
                "profile-a",
                &forged,
            )
            .expect_err("forged imported handle provenance must fail closed");
            assert_eq!(failure.code, "invalid_plugin_value", "{label}");
        }
        Ok(())
    }

    #[test]
    fn unmaterialized_plugin_payloads_cannot_forge_native_store_objects()
    -> Result<(), Box<dyn Error>> {
        let registry = TypeRegistry::built_in()?;
        let port = input_port(&registry, "IMAGE", PortSerialization::Handle)?;
        let payload =
            NativeStoredPayload::Provider(Arc::new(comfy_nodes::NativeProviderPayload::checked(
                NativeHandleType::new(NativeHandleKind::ProviderTask, "TEST_TASK")?,
                "test.signed",
                "1".repeat(64),
                vec![1],
            )?));
        let failure = plugin_value_from_stored(&port, ValueFamily::Tensor, &payload, &registry)
            .expect_err("an unrelated sealed payload must not satisfy an IMAGE input");
        assert_eq!(failure.code, "invalid_plugin_value");

        let failure = unmaterialized_plugin_output("image");
        assert_eq!(failure.code, "unmaterialized_plugin_payload");
        assert!(failure.message.contains("host-materialized payload"));

        let artifact_port = input_port(&registry, "SVG", PortSerialization::ArtifactReference)?;
        let artifact = artifact_value("provider.svg")?;
        let semantic_digest = value_digest(&artifact).ok_or("artifact digest is absent")?;
        let provider = comfy_nodes::NativeProviderPayload::checked(
            NativeHandleType::new(NativeHandleKind::ProviderTask, "SVG")?,
            "test.provider",
            semantic_digest,
            artifact.abi_bytes()?,
        )?;
        let failure = plugin_value_from_stored(
            &artifact_port,
            ValueFamily::Artifact,
            &NativeStoredPayload::Provider(Arc::new(provider)),
            &registry,
        )
        .expect_err("artifact provider payloads are not provider-task identities");
        assert_eq!(failure.code, "invalid_plugin_value");
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
        assert_eq!(native_handle_type(&port)?.type_id, "SVG");
        Ok(())
    }
}
