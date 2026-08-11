mod capabilities;
mod component_host;
mod legacy_mapping;
mod private_worker;
mod registry_adapter;

pub use capabilities::{
    AssetPluginCapabilityServices, BrokerPluginCapabilityServices, CancellationToken,
    CapabilityEffects, CapabilityLimits, CapabilityServiceContext, CapabilityState,
    PluginCapabilityServices, PluginOutputProposal, PluginOutputPublicationAdapter,
    PluginOutputPublicationError, RouteEffect, UnavailablePluginCapabilityServices,
    check_plugin_cancellation,
};
pub use component_host::{
    ComponentExecutionBoundary, ComponentHost, ComponentHostError, ComponentHostRouter,
    InstalledComponentBinding, InstalledVerifiedPlugin, PluginInvocationExecutor,
    PreparedPluginInvocation, VerifiedComponentDeployment, VerifiedComponentGeneration,
    WorkerPluginInvocation,
};
pub use legacy_mapping::{
    AcceptedRewrite, InstalledMappingProjection, LegacyCompatibilityProjection,
    LegacyInputPortTranslation, LegacyInputSourceProjection, LegacyMappingError,
    LegacyMappingResolver, LegacyNodeReference, LegacyOutputPortTranslation, LegacyPortTranslation,
    LegacyProviderProjection, LegacyProviderScope, LegacyResolution, MAX_LEGACY_REFERENCE_BYTES,
    MappingCandidate, MappingProvenance, MappingSource, MappingTarget,
};
pub use private_worker::PrivateWorkerPluginExecutor;
pub use registry_adapter::{PluginRegistryAdapterError, registry_with_installed_plugins};

use comfy_plugin_sdk::{
    COMPONENT_API_VERSION, CancelReason, CanonicalTypeId, CapabilityCall, CapabilityResponse,
    ComponentManifestProjection, InputState, InvocationError, NegotiatedApi,
    PROVIDER_COMPONENT_WORLD, PluginContractError, PluginInvocation, PluginManifest, PluginNode,
    PluginValue, PluginValueRepresentation, PortCardinality, PortDirection, PortPresence,
    PortSerialization, ProviderBindingClaim, ProviderBindingSet, ProviderResultReceiptSet,
    RustComfyPlugin, TypeRegistry, ValueFamily, ValueHandle,
};
use comfy_runtime::{AssetIdentity, AssetNamespace, PluginAuthorization, TrustError};
use extension_host::ComponentRuntime;
use sha2::{Digest, Sha256};
use std::{
    cell::Cell,
    collections::{BTreeMap, BTreeSet},
    mem,
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use thiserror::Error;
use wasmtime::{
    Store, StoreLimits, StoreLimitsBuilder,
    component::{Component, Linker},
};

mod wit_contract {
    wasmtime::component::bindgen!({
        path: "../comfy_plugin_sdk/wit",
        world: "comfy-plugin",
    });
}

mod provider_wit_contract {
    wasmtime::component::bindgen!({
        path: "../comfy_plugin_sdk/wit",
        world: "comfy-provider-plugin",
        with: {
            "sim:comfy-plugin/types@1.0.0": super::wit_contract::sim::comfy_plugin::types,
            "sim:comfy-plugin/host@1.0.0": super::wit_contract::sim::comfy_plugin::host,
        },
    });
}

type WitInvocationError = wit_contract::sim::comfy_plugin::types::InvocationError;

pub const DEFAULT_API_FEATURES: &[&str] = &[
    "capabilities.transactional",
    "handles.revocation",
    "legacy.non-destructive",
    "ports.list",
    "provider.bindings.v1",
];

static NEXT_INVOCATION_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentLimits {
    pub maximum_component_bytes: usize,
    pub maximum_memory_bytes: usize,
    pub maximum_table_elements: usize,
    pub maximum_instances: usize,
    pub maximum_tables: usize,
    pub maximum_memories: usize,
    pub maximum_fuel: u64,
    pub epoch_deadline_ticks: u64,
    pub maximum_values_per_port: usize,
    pub maximum_value_bytes: u64,
    pub maximum_value_handles: usize,
    pub maximum_invocation_value_bytes: u64,
    pub maximum_port_operations: u64,
    pub maximum_port_response_bytes: u64,
    pub capability_limits: CapabilityLimits,
}

impl Default for ComponentLimits {
    fn default() -> Self {
        Self {
            maximum_component_bytes: 64 * 1024 * 1024,
            maximum_memory_bytes: 256 * 1024 * 1024,
            maximum_table_elements: 100_000,
            maximum_instances: 32,
            maximum_tables: 32,
            maximum_memories: 16,
            maximum_fuel: 10_000_000,
            epoch_deadline_ticks: 1,
            maximum_values_per_port: 16_384,
            maximum_value_bytes: 256 * 1024 * 1024,
            maximum_value_handles: 65_536,
            maximum_invocation_value_bytes: 512 * 1024 * 1024,
            maximum_port_operations: 1_000_000,
            maximum_port_response_bytes: 512 * 1024 * 1024,
            capability_limits: CapabilityLimits::default(),
        }
    }
}

impl ComponentLimits {
    pub(crate) fn validate(&self) -> Result<(), PluginError> {
        if self.maximum_component_bytes == 0
            || self.maximum_memory_bytes == 0
            || self.maximum_table_elements == 0
            || self.maximum_instances == 0
            || self.maximum_tables == 0
            || self.maximum_memories == 0
            || self.maximum_fuel == 0
            || self.epoch_deadline_ticks == 0
            || self.maximum_values_per_port == 0
            || self.maximum_value_bytes == 0
            || self.maximum_value_handles == 0
            || self.maximum_invocation_value_bytes == 0
            || self.maximum_port_operations == 0
            || self.maximum_port_response_bytes == 0
        {
            return Err(PluginError::InvalidHostLimits);
        }
        self.capability_limits
            .validate()
            .map_err(|_| PluginError::InvalidHostLimits)?;
        Ok(())
    }
}

pub struct VerifiedManifest<'a> {
    pub manifest: &'a PluginManifest,
    pub authorization: &'a PluginAuthorization,
    pub negotiated_api: NegotiatedApi,
}

#[derive(Debug, Error)]
pub enum PluginError {
    #[error(transparent)]
    Contract(#[from] PluginContractError),
    #[error(transparent)]
    Trust(#[from] TrustError),
    #[error("invalid plugin host resource limits")]
    InvalidHostLimits,
    #[error("plugin component is larger than the configured byte limit")]
    ComponentTooLarge,
    #[error("plugin component compilation failed: {0}")]
    ComponentCompilation(String),
    #[error("plugin invocation failed: {0}")]
    Invocation(#[from] InvocationError),
    #[error("plugin component trapped: {0}")]
    WasmTrap(String),
    #[error("plugin node `{0}` was not declared by its signed manifest")]
    UndeclaredNode(String),
    #[error("plugin manifest and Rust implementation disagree")]
    ManifestProjectionMismatch,
    #[error("compiled plugin and invocation authorization disagree")]
    InvocationBindingMismatch,
    #[error("plugin component world disagrees with its signed manifest")]
    ComponentWorldMismatch,
    #[error("provider component binding set disagrees with its signed manifest")]
    ProviderBindingMismatch,
    #[error("provider invocation is unavailable for this component")]
    ProviderInvocationUnavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ComponentWorld {
    Legacy,
    Provider,
}

pub struct CompiledPlugin {
    component: Component,
    identifier: String,
    digest_sha256: String,
    manifest_projection: ComponentManifestProjection,
    provider_binding: Option<ProviderBindingSet>,
    world: ComponentWorld,
}

impl CompiledPlugin {
    pub fn component(&self) -> &Component {
        &self.component
    }

    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    pub fn digest_sha256(&self) -> &str {
        &self.digest_sha256
    }
}

pub struct WasmStoreState {
    limits: StoreLimits,
    invocation: Option<InvocationHost>,
}

pub struct WasmPluginInstance {
    store: Store<WasmStoreState>,
    bindings: WasmBindings,
    expected_manifest_projection: ComponentManifestProjection,
    provider_binding: Option<ProviderBindingSet>,
    terminal: bool,
}

enum WasmBindings {
    Legacy(wit_contract::ComfyPlugin),
    Provider(provider_wit_contract::ComfyProviderPlugin),
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderInvocationResult {
    pub outputs: BTreeMap<String, Vec<PluginValue>>,
    pub output_presence: BTreeMap<String, bool>,
    pub effects: CapabilityEffects,
    receipts: Vec<Vec<u8>>,
}

impl ProviderInvocationResult {
    pub fn receipts(&self) -> &[Vec<u8>] {
        &self.receipts
    }
}

impl WasmPluginInstance {
    pub fn manifest_bytes(&mut self) -> Result<ComponentManifestProjection, PluginError> {
        self.check_active()?;
        let result = match &self.bindings {
            WasmBindings::Legacy(bindings) => bindings
                .sim_comfy_plugin_plugin()
                .call_manifest(&mut self.store),
            WasmBindings::Provider(bindings) => bindings
                .sim_comfy_plugin_plugin()
                .call_manifest(&mut self.store),
        };
        match result {
            Ok(projection) => {
                let projection = match sdk_manifest_projection(projection) {
                    Ok(projection) => projection,
                    Err(_) => {
                        self.abort();
                        return Err(PluginError::ManifestProjectionMismatch);
                    }
                };
                if let Err(error) =
                    validate_component_projection(&self.expected_manifest_projection, &projection)
                {
                    self.abort();
                    return Err(error);
                }
                Ok(projection)
            }
            Err(error) => Err(self.wasm_call_error(error)),
        }
    }

    pub fn create_node(&mut self, node_id: &str) -> Result<u64, PluginError> {
        self.check_active()?;
        let declared_for_invocation = self
            .store
            .data()
            .invocation
            .as_ref()
            .is_some_and(|invocation| invocation.node.id == node_id);
        if !declared_for_invocation {
            self.abort();
            return Err(PluginError::UndeclaredNode(node_id.to_owned()));
        }
        let result = match &self.bindings {
            WasmBindings::Legacy(bindings) => bindings
                .sim_comfy_plugin_plugin()
                .call_create_node(&mut self.store, node_id),
            WasmBindings::Provider(bindings) => bindings
                .sim_comfy_plugin_plugin()
                .call_create_node(&mut self.store, node_id),
        };
        match result {
            Ok(Ok(instance)) => Ok(instance),
            Ok(Err(error)) => {
                self.abort();
                Err(PluginError::Invocation(sdk_error(error)))
            }
            Err(error) => Err(self.wasm_call_error(error)),
        }
    }

    pub fn provider_binding_set(&mut self) -> Result<ProviderBindingSet, PluginError> {
        self.check_active()?;
        let expected = self
            .provider_binding
            .clone()
            .ok_or(PluginError::ProviderInvocationUnavailable)?;
        let WasmBindings::Provider(bindings) = &self.bindings else {
            return Err(PluginError::ProviderInvocationUnavailable);
        };
        let binding_set = bindings
            .sim_comfy_plugin_provider_binding()
            .call_binding_set(&mut self.store)
            .map_err(|error| self.wasm_call_error(error))?;
        let actual = match sdk_provider_binding_set(binding_set) {
            Ok(actual) => actual,
            Err(error) => {
                self.abort();
                return Err(error);
            }
        };
        if actual != expected {
            self.abort();
            return Err(PluginError::ProviderBindingMismatch);
        }
        Ok(actual)
    }

    pub fn invoke(&mut self, instance: u64) -> Result<(), PluginError> {
        self.check_active()?;
        let result = match &self.bindings {
            WasmBindings::Legacy(bindings) => bindings
                .sim_comfy_plugin_plugin()
                .call_invoke(&mut self.store, instance),
            WasmBindings::Provider(bindings) => bindings
                .sim_comfy_plugin_plugin()
                .call_invoke(&mut self.store, instance),
        };
        match result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => {
                self.abort();
                Err(PluginError::Invocation(sdk_error(error)))
            }
            Err(error) => Err(self.wasm_call_error(error)),
        }
    }

    pub fn cancel(&mut self, instance: u64, reason: CancelReason) -> Result<(), PluginError> {
        self.check_active()?;
        let reason = wit_cancel_reason(reason);
        let result = match &self.bindings {
            WasmBindings::Legacy(bindings) => {
                bindings
                    .sim_comfy_plugin_plugin()
                    .call_cancel(&mut self.store, instance, reason)
            }
            WasmBindings::Provider(bindings) => {
                bindings
                    .sim_comfy_plugin_plugin()
                    .call_cancel(&mut self.store, instance, reason)
            }
        };
        match result {
            Ok(Ok(())) => {
                self.abort();
                Ok(())
            }
            Ok(Err(error)) => {
                self.abort();
                Err(PluginError::Invocation(sdk_error(error)))
            }
            Err(error) => Err(self.wasm_call_error(error)),
        }
    }

    pub fn drop_node(&mut self, instance: u64) -> Result<(), PluginError> {
        self.check_active()?;
        let result = match &self.bindings {
            WasmBindings::Legacy(bindings) => bindings
                .sim_comfy_plugin_plugin()
                .call_drop_node(&mut self.store, instance),
            WasmBindings::Provider(bindings) => bindings
                .sim_comfy_plugin_plugin()
                .call_drop_node(&mut self.store, instance),
        };
        match result {
            Ok(()) => Ok(()),
            Err(error) => {
                self.abort();
                Err(PluginError::WasmTrap(sanitize_diagnostic(
                    &error.to_string(),
                )))
            }
        }
    }

    pub fn finish(mut self) -> Result<InvocationResult, PluginError> {
        let invocation = self
            .store
            .data_mut()
            .invocation
            .take()
            .ok_or_else(|| PluginError::Invocation(InvocationError::RevokedHandle))?;
        let result = invocation.finish().map_err(PluginError::from);
        self.terminal = true;
        result
    }

    pub fn invoke_provider(
        mut self,
        class_type: &str,
        request: &[u8],
    ) -> Result<ProviderInvocationResult, PluginError> {
        self.check_active()?;
        let provider_binding = self
            .provider_binding
            .as_ref()
            .ok_or(PluginError::ProviderInvocationUnavailable)?;
        let invocation = self
            .store
            .data()
            .invocation
            .as_ref()
            .ok_or_else(|| PluginError::Invocation(InvocationError::RevokedHandle))?;
        if invocation.node.id != class_type
            || !provider_binding
                .bindings
                .iter()
                .any(|binding| binding.node_id == class_type)
        {
            self.abort();
            return Err(PluginError::UndeclaredNode(class_type.to_owned()));
        }
        if request.is_empty()
            || request.len()
                > usize::try_from(invocation.limits.maximum_value_bytes).unwrap_or(usize::MAX)
        {
            self.abort();
            return Err(PluginError::Invocation(value_quota_error()));
        }
        invocation.check_cancellation()?;
        let WasmBindings::Provider(bindings) = &self.bindings else {
            self.abort();
            return Err(PluginError::ProviderInvocationUnavailable);
        };
        let response = match bindings
            .sim_comfy_plugin_provider_binding()
            .call_invoke_provider(&mut self.store, class_type, request)
        {
            Ok(Ok(response)) => response,
            Ok(Err(error)) => {
                self.abort();
                return Err(PluginError::Invocation(sdk_error(error)));
            }
            Err(error) => return Err(self.wasm_call_error(error)),
        };
        let invocation = self
            .store
            .data_mut()
            .invocation
            .take()
            .ok_or_else(|| PluginError::Invocation(InvocationError::RevokedHandle))?;
        let result = invocation
            .finish_provider_response(response)
            .map_err(PluginError::from);
        self.terminal = true;
        result
    }

    pub fn abort(&mut self) {
        if let Some(invocation) = self.store.data_mut().invocation.as_mut() {
            invocation.abort();
        }
        self.terminal = true;
    }

    fn check_active(&self) -> Result<(), PluginError> {
        if self.terminal {
            Err(PluginError::Invocation(InvocationError::RevokedHandle))
        } else {
            Ok(())
        }
    }

    fn wasm_call_error(&mut self, error: impl std::fmt::Display) -> PluginError {
        let cancelled = self
            .store
            .data()
            .invocation
            .as_ref()
            .is_some_and(InvocationHost::is_cancelled);
        self.abort();
        if cancelled {
            PluginError::Invocation(InvocationError::Cancelled)
        } else {
            PluginError::WasmTrap(sanitize_diagnostic(&format!("{error:#}")))
        }
    }
}

impl Drop for WasmPluginInstance {
    fn drop(&mut self) {
        if !self.terminal {
            self.abort();
        }
    }
}

pub struct PluginHost {
    runtime: ComponentRuntime,
    registry: TypeRegistry,
    api_features: BTreeSet<String>,
    limits: ComponentLimits,
}

impl PluginHost {
    pub fn new() -> Result<Self, PluginError> {
        Self::with_configuration(
            ComponentLimits::default(),
            DEFAULT_API_FEATURES
                .iter()
                .map(|feature| (*feature).to_owned()),
        )
    }

    pub fn with_configuration(
        limits: ComponentLimits,
        api_features: impl IntoIterator<Item = String>,
    ) -> Result<Self, PluginError> {
        let runtime = ComponentRuntime::no_wasi()
            .map_err(|error| PluginError::ComponentCompilation(error.to_string()))?;
        Self::with_component_runtime(runtime, limits, api_features)
    }

    pub fn with_component_runtime(
        runtime: ComponentRuntime,
        limits: ComponentLimits,
        api_features: impl IntoIterator<Item = String>,
    ) -> Result<Self, PluginError> {
        limits.validate()?;
        Ok(Self {
            runtime,
            registry: TypeRegistry::built_in().map_err(PluginContractError::from)?,
            api_features: api_features.into_iter().collect(),
            limits,
        })
    }

    pub fn registry(&self) -> &TypeRegistry {
        &self.registry
    }

    pub fn limits(&self) -> &ComponentLimits {
        &self.limits
    }

    pub fn validate<'a>(
        &self,
        manifest: &'a PluginManifest,
        authorization: &'a PluginAuthorization,
    ) -> Result<VerifiedManifest<'a>, PluginError> {
        manifest.validate(&self.registry)?;
        for request in &manifest.capabilities {
            self.limits
                .capability_limits
                .validate_quota(request.quota)
                .map_err(PluginError::Invocation)?;
        }
        authorization.require_manifest(manifest)?;
        let negotiated_api = manifest
            .api
            .negotiate(COMPONENT_API_VERSION, &self.api_features)?;
        Ok(VerifiedManifest {
            manifest,
            authorization,
            negotiated_api,
        })
    }

    pub fn compile_component(
        &self,
        bytes: &[u8],
        manifest: &PluginManifest,
        authorization: &PluginAuthorization,
    ) -> Result<CompiledPlugin, PluginError> {
        self.validate(manifest, authorization)?;
        if bytes.len() > self.limits.maximum_component_bytes {
            return Err(PluginError::ComponentTooLarge);
        }
        let digest = encode_hex(&Sha256::digest(bytes));
        if !constant_time_equal(digest.as_bytes(), manifest.digest_sha256.as_bytes()) {
            return Err(PluginError::ManifestProjectionMismatch);
        }
        let component = self.runtime.compile_component(bytes).map_err(|error| {
            PluginError::ComponentCompilation(sanitize_diagnostic(&error.to_string()))
        })?;
        let manifest_projection = manifest.component_projection();
        let world = if manifest.provider_binding.is_some() {
            if manifest_projection.component_world != PROVIDER_COMPONENT_WORLD {
                return Err(PluginError::ComponentWorldMismatch);
            }
            ComponentWorld::Provider
        } else {
            ComponentWorld::Legacy
        };
        let compiled = CompiledPlugin {
            component,
            identifier: manifest.identifier.clone(),
            digest_sha256: digest,
            manifest_projection,
            provider_binding: manifest.provider_binding.clone(),
            world,
        };
        self.preflight_component(&compiled)?;
        Ok(compiled)
    }

    fn new_wasm_store(&self) -> Result<Store<WasmStoreState>, PluginError> {
        self.make_wasm_store(None)
    }

    fn new_wasm_invocation_store(
        &self,
        invocation: InvocationHost,
    ) -> Result<Store<WasmStoreState>, PluginError> {
        self.make_wasm_store(Some(invocation))
    }

    pub fn instantiate_component(
        &self,
        plugin: &CompiledPlugin,
        invocation: InvocationHost,
    ) -> Result<WasmPluginInstance, PluginError> {
        if invocation.plugin_identifier != plugin.identifier
            || invocation.plugin_digest_sha256 != plugin.digest_sha256
            || !plugin.manifest_projection.nodes.iter().any(|node| {
                node.id == invocation.node.id && node.version == invocation.node.version
            })
        {
            return Err(PluginError::InvocationBindingMismatch);
        }
        let store = self.new_wasm_invocation_store(invocation)?;
        let (store, bindings) = self.instantiate_bindings(plugin, store)?;
        let mut instance = WasmPluginInstance {
            store,
            bindings,
            expected_manifest_projection: plugin.manifest_projection.clone(),
            provider_binding: plugin.provider_binding.clone(),
            terminal: false,
        };
        instance.manifest_bytes()?;
        if plugin.world == ComponentWorld::Provider {
            instance.provider_binding_set()?;
        }
        Ok(instance)
    }

    fn preflight_component(&self, plugin: &CompiledPlugin) -> Result<(), PluginError> {
        let store = self.new_wasm_store()?;
        let (store, bindings) = self
            .instantiate_bindings(plugin, store)
            .map_err(preflight_component_error)?;
        let mut instance = WasmPluginInstance {
            store,
            bindings,
            expected_manifest_projection: plugin.manifest_projection.clone(),
            provider_binding: plugin.provider_binding.clone(),
            terminal: false,
        };
        instance.manifest_bytes()?;
        if plugin.world == ComponentWorld::Provider {
            instance.provider_binding_set()?;
        }
        instance.terminal = true;
        Ok(())
    }

    fn instantiate_bindings(
        &self,
        plugin: &CompiledPlugin,
        mut store: Store<WasmStoreState>,
    ) -> Result<(Store<WasmStoreState>, WasmBindings), PluginError> {
        let mut linker = Linker::<WasmStoreState>::new(self.runtime.engine());
        let bindings = match plugin.world {
            ComponentWorld::Legacy => {
                wit_contract::ComfyPlugin::add_to_linker::<
                    WasmStoreState,
                    wasmtime::component::HasSelf<WasmStoreState>,
                >(&mut linker, |state| state)
                .map_err(component_compilation_error)?;
                WasmBindings::Legacy(
                    wit_contract::ComfyPlugin::instantiate(&mut store, plugin.component(), &linker)
                        .map_err(component_instantiation_error)?,
                )
            }
            ComponentWorld::Provider => {
                provider_wit_contract::ComfyProviderPlugin::add_to_linker::<
                    WasmStoreState,
                    wasmtime::component::HasSelf<WasmStoreState>,
                >(&mut linker, |state| state)
                .map_err(component_compilation_error)?;
                WasmBindings::Provider(
                    provider_wit_contract::ComfyProviderPlugin::instantiate(
                        &mut store,
                        plugin.component(),
                        &linker,
                    )
                    .map_err(component_instantiation_error)?,
                )
            }
        };
        Ok((store, bindings))
    }

    fn make_wasm_store(
        &self,
        invocation: Option<InvocationHost>,
    ) -> Result<Store<WasmStoreState>, PluginError> {
        let limits = StoreLimitsBuilder::new()
            .memory_size(self.limits.maximum_memory_bytes)
            .table_elements(self.limits.maximum_table_elements)
            .instances(self.limits.maximum_instances)
            .tables(self.limits.maximum_tables)
            .memories(self.limits.maximum_memories)
            .trap_on_grow_failure(true)
            .build();
        let mut store = Store::new(self.runtime.engine(), WasmStoreState { limits, invocation });
        store.limiter(|state| &mut state.limits);
        store
            .set_fuel(self.limits.maximum_fuel)
            .map_err(|error| PluginError::ComponentCompilation(error.to_string()))?;
        store.set_epoch_deadline(self.limits.epoch_deadline_ticks);
        Ok(store)
    }

    pub fn interrupt_wasm(&self) {
        self.runtime.increment_epoch();
    }

    pub fn begin_invocation(
        &self,
        manifest: &PluginManifest,
        authorization: &PluginAuthorization,
        node_id: &str,
        inputs: InvocationInputs,
        services: Arc<dyn PluginCapabilityServices>,
        cancellation: CancellationToken,
    ) -> Result<InvocationHost, PluginError> {
        self.validate(manifest, authorization)?;
        let node = manifest
            .nodes
            .iter()
            .find(|node| node.id == node_id)
            .ok_or_else(|| PluginError::UndeclaredNode(node_id.to_owned()))?;
        let ui_contributions = manifest
            .ui
            .iter()
            .map(|contribution| contribution.id.clone())
            .collect();
        let route_response_limits = manifest
            .routes
            .iter()
            .map(|route| (route.id.clone(), route.maximum_response_bytes))
            .collect();
        InvocationHost::new(
            node,
            inputs,
            authorization,
            manifest,
            services,
            cancellation,
            self.registry.clone(),
            self.limits.clone(),
            ui_contributions,
            route_response_limits,
        )
        .map_err(PluginError::from)
    }

    pub fn invoke_rust(
        &self,
        plugin: &dyn RustComfyPlugin,
        authorization: &PluginAuthorization,
        node_id: &str,
        inputs: InvocationInputs,
        services: Arc<dyn PluginCapabilityServices>,
        cancellation: CancellationToken,
    ) -> Result<InvocationResult, PluginError> {
        let manifest = plugin.manifest();
        let mut invocation = self.begin_invocation(
            manifest,
            authorization,
            node_id,
            inputs,
            services,
            cancellation,
        )?;
        let mut node = plugin.create_node(node_id)?;
        if let Err(error) = node.invoke(&mut invocation) {
            match &error {
                InvocationError::Cancelled => node.cancel(CancelReason::User),
                InvocationError::TimedOut => node.cancel(CancelReason::Timeout),
                _ => {}
            }
            invocation.abort();
            return Err(PluginError::Invocation(error));
        }
        invocation.finish().map_err(PluginError::from)
    }
}

#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct InvocationInputs {
    values: BTreeMap<String, Option<Vec<PluginValue>>>,
}

impl InvocationInputs {
    pub fn set_absent(&mut self, port_id: impl Into<String>) {
        self.values.insert(port_id.into(), None);
    }

    pub fn set_present(&mut self, port_id: impl Into<String>, values: Vec<PluginValue>) {
        self.values.insert(port_id.into(), Some(values));
    }
}

struct InputPortState {
    present: bool,
    values: Vec<Option<PluginValue>>,
}

struct OutputPortState {
    values: Vec<PluginValue>,
    value_bytes: u64,
    present: Option<bool>,
    finished: bool,
}

struct HandleEntry {
    generation: u32,
    value: PluginValue,
}

#[derive(Clone, Copy, Default)]
struct PortCallUsage {
    operations: u64,
    response_bytes: u64,
}

pub struct InvocationHost {
    invocation_id: u64,
    plugin_identifier: String,
    plugin_digest_sha256: String,
    node: PluginNode,
    registry: TypeRegistry,
    limits: ComponentLimits,
    inputs: BTreeMap<String, InputPortState>,
    outputs: BTreeMap<String, OutputPortState>,
    handles: BTreeMap<u32, HandleEntry>,
    next_handle: u32,
    invocation_value_bytes: u64,
    port_call_usage: Cell<PortCallUsage>,
    capabilities: Option<CapabilityState>,
    terminal: bool,
}

impl InvocationHost {
    fn new(
        node: &PluginNode,
        supplied_inputs: InvocationInputs,
        authorization: &PluginAuthorization,
        manifest: &PluginManifest,
        services: Arc<dyn PluginCapabilityServices>,
        cancellation: CancellationToken,
        registry: TypeRegistry,
        limits: ComponentLimits,
        ui_contributions: BTreeSet<String>,
        route_response_limits: BTreeMap<String, u64>,
    ) -> Result<Self, InvocationError> {
        let invocation_id = next_invocation_id()?;
        let mut supplied = supplied_inputs.values;
        let mut inputs = BTreeMap::new();
        let mut outputs = BTreeMap::new();
        let mut total_value_bytes = 0_u64;
        for port in &node.ports {
            match port.direction {
                PortDirection::Input => {
                    let supplied_values = supplied.remove(&port.id).unwrap_or(None);
                    let present = supplied_values.is_some();
                    let values = supplied_values.unwrap_or_default();
                    let port_value_bytes =
                        validate_input_port(port, present, &values, &registry, &limits)?;
                    total_value_bytes = total_value_bytes
                        .checked_add(port_value_bytes)
                        .ok_or_else(invocation_value_quota_error)?;
                    if total_value_bytes > limits.maximum_invocation_value_bytes {
                        return Err(invocation_value_quota_error());
                    }
                    inputs.insert(
                        port.id.clone(),
                        InputPortState {
                            present,
                            values: values.into_iter().map(Some).collect(),
                        },
                    );
                }
                PortDirection::Output => {
                    outputs.insert(
                        port.id.clone(),
                        OutputPortState {
                            values: Vec::new(),
                            value_bytes: 0,
                            present: None,
                            finished: false,
                        },
                    );
                }
            }
        }
        if let Some((unknown, _)) = supplied.into_iter().next() {
            return Err(InvocationError::UnknownPort(unknown));
        }
        let capabilities = CapabilityState::with_declarations(
            authorization,
            manifest,
            services,
            cancellation,
            limits.capability_limits,
            ui_contributions,
            route_response_limits,
        )?;
        Ok(Self {
            invocation_id,
            plugin_identifier: manifest.identifier.clone(),
            plugin_digest_sha256: manifest.digest_sha256.clone(),
            node: node.clone(),
            registry,
            limits,
            inputs,
            outputs,
            handles: BTreeMap::new(),
            next_handle: 1,
            invocation_value_bytes: total_value_bytes,
            port_call_usage: Cell::new(PortCallUsage::default()),
            capabilities: Some(capabilities),
            terminal: false,
        })
    }

    pub fn abort(&mut self) {
        if let Some(capabilities) = self.capabilities.as_mut() {
            capabilities.rollback();
        }
        self.inputs.clear();
        self.outputs.clear();
        self.handles.clear();
        self.terminal = true;
    }

    pub fn finish(mut self) -> Result<InvocationResult, InvocationError> {
        self.check_active()?;
        self.check_cancellation()?;
        let unfinished_output = self
            .node
            .ports
            .iter()
            .filter(|port| port.direction == PortDirection::Output)
            .find_map(|port| {
                self.outputs
                    .get(&port.id)
                    .filter(|state| !state.finished)
                    .map(|_| port.id.clone())
            });
        if let Some(port_id) = unfinished_output {
            self.abort();
            return Err(InvocationError::UnfinishedOutput(port_id));
        }
        let mut capabilities = self.capabilities.take().ok_or_else(|| {
            InvocationError::HostFailure("invocation capability state is missing".to_owned())
        })?;
        if capabilities.has_open_output_buffers() {
            capabilities.rollback();
            self.abort();
            return Err(InvocationError::HostFailure(
                "plugin invocation left an output transaction open".to_owned(),
            ));
        }
        let effects = capabilities.finish()?;
        let mut outputs = BTreeMap::new();
        let mut output_presence = BTreeMap::new();
        for (port, state) in mem::take(&mut self.outputs) {
            let present = state.present.ok_or_else(|| {
                InvocationError::HostFailure(format!(
                    "finished plugin output `{port}` has no presence state"
                ))
            })?;
            output_presence.insert(port.clone(), present);
            outputs.insert(port, state.values);
        }
        self.inputs.clear();
        self.handles.clear();
        self.terminal = true;
        Ok(InvocationResult {
            outputs,
            output_presence,
            effects,
        })
    }

    fn finish_provider_response(
        mut self,
        response: wit_contract::sim::comfy_plugin::types::ProviderInvocationResponse,
    ) -> Result<ProviderInvocationResult, InvocationError> {
        self.check_active()?;
        self.check_cancellation()?;
        if response.receipt.is_empty()
            || u64::try_from(response.receipt.len()).unwrap_or(u64::MAX)
                > self.limits.maximum_port_response_bytes
        {
            return Err(InvocationError::InvocationQuotaExceeded {
                limit: "provider-receipt-byte".to_owned(),
            });
        }
        let receipts = ProviderResultReceiptSet::from_bytes(&response.receipt)
            .map_err(|_| {
                InvocationError::HostFailure(
                    "provider component returned an invalid result receipt set".to_owned(),
                )
            })?
            .into_receipts();
        if !self.handles.is_empty()
            || self
                .outputs
                .values()
                .any(|state| !state.values.is_empty() || state.present.is_some() || state.finished)
        {
            return Err(InvocationError::HostFailure(
                "provider component mixed handle-port output with its typed response".to_owned(),
            ));
        }

        let mut response_bytes =
            u64::try_from(response.receipt.len()).map_err(|_| invocation_value_quota_error())?;
        for output in response.outputs {
            let port = self
                .node
                .ports
                .iter()
                .find(|port| port.id == output.port_id && port.direction == PortDirection::Output)
                .ok_or_else(|| InvocationError::UnknownPort(output.port_id.clone()))?;
            let value = PluginValue::from_abi_bytes(&output.value.abi_bytes, &self.registry)
                .map_err(|error| {
                    InvocationError::HostFailure(format!("invalid provider output value: {error}"))
                })?;
            if value.type_id().to_string() != output.value.type_id
                || value.family() != sdk_value_family(output.value.family)
            {
                return Err(InvocationError::HostFailure(
                    "encoded provider output metadata disagrees with its canonical ABI value"
                        .to_owned(),
                ));
            }
            require_exact_port_type(port, &value)?;
            let expected_family = self.registry.family(&port.type_id).map_err(|error| {
                InvocationError::HostFailure(format!("plugin type registry failed: {error}"))
            })?;
            if value.family() != expected_family {
                return Err(InvocationError::WrongValueFamily {
                    port: port.id.clone(),
                    expected: expected_family,
                    actual: value.family(),
                });
            }
            let value_bytes = value_size(&value)?;
            if value_bytes > self.limits.maximum_value_bytes {
                return Err(value_quota_error());
            }
            response_bytes = response_bytes
                .checked_add(u64::try_from(output.port_id.len()).map_err(|_| {
                    InvocationError::HostFailure("provider response size overflow".to_owned())
                })?)
                .and_then(|bytes| {
                    bytes.checked_add(u64::try_from(output.value.type_id.len()).ok()?)
                })
                .and_then(|bytes| {
                    bytes.checked_add(u64::try_from(output.value.abi_bytes.len()).ok()?)
                })
                .ok_or_else(|| {
                    InvocationError::HostFailure("provider response size overflow".to_owned())
                })?;
            if response_bytes > self.limits.maximum_port_response_bytes {
                return Err(InvocationError::InvocationQuotaExceeded {
                    limit: "provider-response-byte".to_owned(),
                });
            }
            self.invocation_value_bytes = self
                .invocation_value_bytes
                .checked_add(value_bytes)
                .ok_or_else(invocation_value_quota_error)?;
            if self.invocation_value_bytes > self.limits.maximum_invocation_value_bytes {
                return Err(invocation_value_quota_error());
            }
            let state = self.outputs.get_mut(&port.id).ok_or_else(|| {
                InvocationError::HostFailure("provider output state is missing".to_owned())
            })?;
            if port.cardinality == PortCardinality::Singular && !state.values.is_empty() {
                return Err(InvocationError::InvalidCardinality(port.id.clone()));
            }
            if state.values.len() >= self.limits.maximum_values_per_port {
                return Err(InvocationError::InvalidCardinality(port.id.clone()));
            }
            state.value_bytes = state
                .value_bytes
                .checked_add(value_bytes)
                .ok_or_else(value_quota_error)?;
            if state.value_bytes > self.limits.maximum_value_bytes {
                return Err(value_quota_error());
            }
            state.values.push(value);
        }

        let mut output_presence = BTreeMap::new();
        for port in self
            .node
            .ports
            .iter()
            .filter(|port| port.direction == PortDirection::Output)
        {
            let state = self.outputs.get_mut(&port.id).ok_or_else(|| {
                InvocationError::HostFailure("provider output state is missing".to_owned())
            })?;
            let present = !state.values.is_empty();
            validate_finished_output(port, present, state.values.len())?;
            state.present = Some(present);
            state.finished = true;
            output_presence.insert(port.id.clone(), present);
        }
        self.check_cancellation()?;
        let mut capabilities = self.capabilities.take().ok_or_else(|| {
            InvocationError::HostFailure("invocation capability state is missing".to_owned())
        })?;
        if capabilities.has_open_output_buffers() {
            capabilities.rollback();
            return Err(InvocationError::HostFailure(
                "provider invocation left an output transaction open".to_owned(),
            ));
        }
        let effects = capabilities.finish()?;
        let outputs = mem::take(&mut self.outputs)
            .into_iter()
            .map(|(port, state)| (port, state.values))
            .collect();
        self.inputs.clear();
        self.terminal = true;
        Ok(ProviderInvocationResult {
            outputs,
            output_presence,
            effects,
            receipts,
        })
    }

    fn check_active(&self) -> Result<(), InvocationError> {
        if self.terminal {
            Err(InvocationError::RevokedHandle)
        } else {
            Ok(())
        }
    }

    fn check_cancellation(&self) -> Result<(), InvocationError> {
        self.capabilities
            .as_ref()
            .ok_or_else(|| InvocationError::HostFailure("capability state is missing".to_owned()))?
            .check_cancelled()
    }

    fn is_cancelled(&self) -> bool {
        self.capabilities
            .as_ref()
            .is_some_and(CapabilityState::is_cancelled)
    }

    fn begin_port_call(&self) -> Result<(), InvocationError> {
        self.check_active()?;
        self.check_cancellation()?;
        let usage = self.port_call_usage.get();
        let operations = usage.operations.checked_add(1).ok_or_else(|| {
            InvocationError::InvocationQuotaExceeded {
                limit: "port-operation".to_owned(),
            }
        })?;
        if operations > self.limits.maximum_port_operations {
            return Err(InvocationError::InvocationQuotaExceeded {
                limit: "port-operation".to_owned(),
            });
        }
        self.port_call_usage.set(PortCallUsage {
            operations,
            response_bytes: usage.response_bytes,
        });
        Ok(())
    }

    fn charge_port_response(&self, response_bytes: u64) -> Result<(), InvocationError> {
        self.check_active()?;
        self.check_cancellation()?;
        let usage = self.port_call_usage.get();
        let response_bytes = usage
            .response_bytes
            .checked_add(response_bytes)
            .ok_or_else(|| InvocationError::InvocationQuotaExceeded {
                limit: "port-response-byte".to_owned(),
            })?;
        if response_bytes > self.limits.maximum_port_response_bytes {
            return Err(InvocationError::InvocationQuotaExceeded {
                limit: "port-response-byte".to_owned(),
            });
        }
        self.port_call_usage.set(PortCallUsage {
            operations: usage.operations,
            response_bytes,
        });
        Ok(())
    }

    fn available_value_handle(&self) -> Result<(ValueHandle, u32), InvocationError> {
        if self.handles.len() >= self.limits.maximum_value_handles {
            return Err(InvocationError::InvocationQuotaExceeded {
                limit: "invocation-handle".to_owned(),
            });
        }
        let next_handle = self.next_handle.checked_add(1).ok_or_else(|| {
            InvocationError::InvocationQuotaExceeded {
                limit: "invocation-handle".to_owned(),
            }
        })?;
        Ok((
            ValueHandle {
                invocation: self.invocation_id,
                slot: self.next_handle,
                generation: 1,
            },
            next_handle,
        ))
    }

    fn create_output_value_after_begin(
        &mut self,
        value: PluginValue,
    ) -> Result<ValueHandle, InvocationError> {
        let expected_family = self.registry.family(value.type_id()).map_err(|error| {
            InvocationError::HostFailure(format!("plugin type registry failed: {error}"))
        })?;
        if expected_family != value.family() {
            return Err(InvocationError::HostFailure(
                "plugin output value type and representation disagree".to_owned(),
            ));
        }
        let value_bytes = value_size(&value)?;
        if value_bytes > self.limits.maximum_value_bytes {
            return Err(value_quota_error());
        }
        let invocation_value_bytes = self
            .invocation_value_bytes
            .checked_add(value_bytes)
            .ok_or_else(invocation_value_quota_error)?;
        if invocation_value_bytes > self.limits.maximum_invocation_value_bytes {
            return Err(invocation_value_quota_error());
        }
        let (handle, next_handle) = self.available_value_handle()?;
        self.charge_port_response(16)?;
        self.next_handle = next_handle;
        self.invocation_value_bytes = invocation_value_bytes;
        self.handles.insert(
            handle.slot,
            HandleEntry {
                generation: handle.generation,
                value,
            },
        );
        Ok(handle)
    }

    fn port(
        &self,
        port_id: &str,
        direction: PortDirection,
    ) -> Result<&comfy_plugin_sdk::PluginPort, InvocationError> {
        let port = self
            .node
            .ports
            .iter()
            .find(|port| port.id == port_id)
            .ok_or_else(|| InvocationError::UnknownPort(port_id.to_owned()))?;
        if port.direction != direction {
            return Err(InvocationError::WrongDirection(port_id.to_owned()));
        }
        Ok(port)
    }
}

impl PluginInvocation for InvocationHost {
    fn input_state(&self, port_id: &str) -> Result<InputState, InvocationError> {
        self.begin_port_call()?;
        let port = self.port(port_id, PortDirection::Input)?;
        let state = self
            .inputs
            .get(port_id)
            .ok_or_else(|| InvocationError::UnknownPort(port_id.to_owned()))?;
        let family = self.registry.family(&port.type_id).map_err(|error| {
            InvocationError::HostFailure(format!("plugin type registry failed: {error}"))
        })?;
        let result = InputState {
            present: state.present,
            length: u32::try_from(state.values.len()).map_err(|_| {
                InvocationError::HostFailure("input length exceeds the ABI limit".to_owned())
            })?,
            type_id: port.type_id.clone(),
            family,
            cardinality: port.cardinality,
            presence: port.presence,
            serialization: port.serialization,
            lazy: port.lazy,
        };
        let type_id_bytes = u64::try_from(port.type_id.to_string().len()).map_err(|_| {
            InvocationError::HostFailure("input state type identifier is too large".to_owned())
        })?;
        self.charge_port_response(type_id_bytes.saturating_add(12))?;
        Ok(result)
    }

    fn read_scalar_input(&self, port_id: &str, index: u32) -> Result<PluginValue, InvocationError> {
        self.begin_port_call()?;
        let port = self.port(port_id, PortDirection::Input)?;
        let family = self.registry.family(&port.type_id).map_err(|error| {
            InvocationError::HostFailure(format!("plugin type registry failed: {error}"))
        })?;
        if family != ValueFamily::Scalar || port.serialization != PortSerialization::Inline {
            return Err(InvocationError::HostFailure(format!(
                "input port `{port_id}` does not use inline scalar ownership"
            )));
        }
        let state = self
            .inputs
            .get(port_id)
            .ok_or_else(|| InvocationError::UnknownPort(port_id.to_owned()))?;
        let index = usize::try_from(index).map_err(|_| InvocationError::IndexOutOfBounds {
            port: port_id.to_owned(),
            index,
        })?;
        let value = state
            .values
            .get(index)
            .ok_or_else(|| InvocationError::IndexOutOfBounds {
                port: port_id.to_owned(),
                index: index as u32,
            })?
            .as_ref()
            .ok_or_else(|| InvocationError::AlreadyTaken {
                port: port_id.to_owned(),
                index: index as u32,
            })?;
        require_exact_port_type(port, value)?;
        self.charge_port_response(encoded_value_response_size(value)?)?;
        Ok(value.clone())
    }

    fn take_input(&mut self, port_id: &str, index: u32) -> Result<ValueHandle, InvocationError> {
        self.begin_port_call()?;
        let port = self.port(port_id, PortDirection::Input)?;
        let family = self.registry.family(&port.type_id).map_err(|error| {
            InvocationError::HostFailure(format!("plugin type registry failed: {error}"))
        })?;
        if family == ValueFamily::Scalar || port.serialization == PortSerialization::Inline {
            return Err(InvocationError::HostFailure(format!(
                "input port `{port_id}` uses inline scalar ownership and cannot be taken"
            )));
        }
        let state = self
            .inputs
            .get(port_id)
            .ok_or_else(|| InvocationError::UnknownPort(port_id.to_owned()))?;
        let index = usize::try_from(index).map_err(|_| InvocationError::IndexOutOfBounds {
            port: port_id.to_owned(),
            index,
        })?;
        let value = state
            .values
            .get(index)
            .ok_or_else(|| InvocationError::IndexOutOfBounds {
                port: port_id.to_owned(),
                index: index as u32,
            })?
            .as_ref()
            .ok_or_else(|| InvocationError::AlreadyTaken {
                port: port_id.to_owned(),
                index: index as u32,
            })?;
        require_exact_port_type(port, value)?;
        let (handle, next_handle) = self.available_value_handle()?;
        self.charge_port_response(16)?;
        let value = self
            .inputs
            .get_mut(port_id)
            .and_then(|state| state.values.get_mut(index))
            .and_then(Option::take)
            .ok_or_else(|| {
                InvocationError::HostFailure(
                    "validated plugin input disappeared before handle creation".to_owned(),
                )
            })?;
        self.next_handle = next_handle;
        self.handles.insert(
            handle.slot,
            HandleEntry {
                generation: handle.generation,
                value,
            },
        );
        Ok(handle)
    }

    fn read_handle(&self, handle: ValueHandle) -> Result<&PluginValue, InvocationError> {
        self.begin_port_call()?;
        if handle.invocation != self.invocation_id {
            return Err(InvocationError::InvalidHandle);
        }
        let entry = self
            .handles
            .get(&handle.slot)
            .ok_or(InvocationError::RevokedHandle)?;
        if entry.generation != handle.generation {
            return Err(InvocationError::RevokedHandle);
        }
        self.charge_port_response(encoded_value_response_size(&entry.value)?)?;
        Ok(&entry.value)
    }

    fn create_output_value(&mut self, value: PluginValue) -> Result<ValueHandle, InvocationError> {
        self.begin_port_call()?;
        self.create_output_value_after_begin(value)
    }

    fn push_output(&mut self, port_id: &str, handle: ValueHandle) -> Result<(), InvocationError> {
        self.begin_port_call()?;
        let port = self.port(port_id, PortDirection::Output)?.clone();
        if handle.invocation != self.invocation_id {
            return Err(InvocationError::InvalidHandle);
        }
        let entry = self
            .handles
            .get(&handle.slot)
            .ok_or(InvocationError::RevokedHandle)?;
        if entry.generation != handle.generation {
            return Err(InvocationError::RevokedHandle);
        }
        require_exact_port_type(&port, &entry.value)?;
        let value_bytes = value_size(&entry.value)?;
        let state = self
            .outputs
            .get(port_id)
            .ok_or_else(|| InvocationError::UnknownPort(port_id.to_owned()))?;
        if state.finished {
            return Err(InvocationError::OutputAlreadyFinished(port_id.to_owned()));
        }
        if port.cardinality == PortCardinality::Singular && !state.values.is_empty() {
            return Err(InvocationError::InvalidCardinality(port_id.to_owned()));
        }
        if state.values.len() >= self.limits.maximum_values_per_port {
            return Err(InvocationError::InvalidCardinality(port_id.to_owned()));
        }
        let output_value_bytes = state
            .value_bytes
            .checked_add(value_bytes)
            .ok_or_else(value_quota_error)?;
        if output_value_bytes > self.limits.maximum_value_bytes {
            return Err(value_quota_error());
        }
        let entry = self
            .handles
            .remove(&handle.slot)
            .ok_or(InvocationError::RevokedHandle)?;
        let state = self.outputs.get_mut(port_id).ok_or_else(|| {
            InvocationError::HostFailure("validated output disappeared".to_owned())
        })?;
        state.value_bytes = output_value_bytes;
        state.values.push(entry.value);
        Ok(())
    }

    fn finish_output(&mut self, port_id: &str, present: bool) -> Result<(), InvocationError> {
        self.begin_port_call()?;
        let port = self.port(port_id, PortDirection::Output)?.clone();
        let state = self
            .outputs
            .get_mut(port_id)
            .ok_or_else(|| InvocationError::UnknownPort(port_id.to_owned()))?;
        if state.finished {
            return Err(InvocationError::OutputAlreadyFinished(port_id.to_owned()));
        }
        validate_finished_output(&port, present, state.values.len())?;
        let state = self.outputs.get_mut(port_id).ok_or_else(|| {
            InvocationError::HostFailure("validated output disappeared".to_owned())
        })?;
        state.present = Some(present);
        state.finished = true;
        Ok(())
    }

    fn call(&mut self, call: CapabilityCall) -> Result<CapabilityResponse, InvocationError> {
        self.check_active()?;
        self.capabilities
            .as_mut()
            .ok_or_else(|| InvocationError::HostFailure("capability state is missing".to_owned()))?
            .execute(call)
    }

    fn check_cancelled(&self) -> Result<(), InvocationError> {
        self.begin_port_call()
    }
}

impl wit_contract::sim::comfy_plugin::types::Host for WasmStoreState {}

impl wit_contract::sim::comfy_plugin::host::Host for WasmStoreState {
    fn get_input_state(
        &mut self,
        port_id: String,
    ) -> Result<wit_contract::sim::comfy_plugin::host::InputState, WitInvocationError> {
        let state = self
            .invocation_mut()?
            .input_state(&port_id)
            .map_err(wit_error)?;
        Ok(wit_contract::sim::comfy_plugin::host::InputState {
            present: state.present,
            length: state.length,
            type_id: state.type_id.to_string(),
            family: wit_value_family(state.family),
            cardinality: wit_port_cardinality(state.cardinality),
            presence: wit_port_presence(state.presence),
            serialization: wit_port_serialization(state.serialization),
            lazy: state.lazy,
        })
    }

    fn read_scalar_input(
        &mut self,
        port_id: String,
        index: u32,
    ) -> Result<wit_contract::sim::comfy_plugin::host::EncodedValue, WitInvocationError> {
        let value = self
            .invocation_mut()?
            .read_scalar_input(&port_id, index)
            .map_err(wit_error)?;
        let abi_bytes = value.abi_bytes().map_err(|error| {
            wit_host_failure(&format!("plugin value ABI encoding failed: {error}"))
        })?;
        Ok(wit_contract::sim::comfy_plugin::host::EncodedValue {
            type_id: value.type_id().to_string(),
            family: wit_value_family(value.family()),
            abi_bytes,
        })
    }

    fn take_input(
        &mut self,
        port_id: String,
        index: u32,
    ) -> Result<wit_contract::sim::comfy_plugin::host::ValueHandle, WitInvocationError> {
        self.invocation_mut()?
            .take_input(&port_id, index)
            .map(wit_value_handle)
            .map_err(wit_error)
    }

    fn read_handle(
        &mut self,
        handle: wit_contract::sim::comfy_plugin::host::ValueHandle,
    ) -> Result<wit_contract::sim::comfy_plugin::host::EncodedValue, WitInvocationError> {
        let handle = sdk_value_handle(handle);
        let value = self
            .invocation_mut()?
            .read_handle(handle)
            .map_err(wit_error)?;
        let abi_bytes = value.abi_bytes().map_err(|error| {
            wit_host_failure(&format!("plugin value ABI encoding failed: {error}"))
        })?;
        Ok(wit_contract::sim::comfy_plugin::host::EncodedValue {
            type_id: value.type_id().to_string(),
            family: wit_value_family(value.family()),
            abi_bytes,
        })
    }

    fn create_output_value(
        &mut self,
        value: wit_contract::sim::comfy_plugin::host::EncodedValue,
    ) -> Result<wit_contract::sim::comfy_plugin::host::ValueHandle, WitInvocationError> {
        let invocation = self.invocation_mut()?;
        invocation.begin_port_call().map_err(wit_error)?;
        let decoded = PluginValue::from_abi_bytes(&value.abi_bytes, &invocation.registry)
            .map_err(|error| wit_host_failure(&format!("invalid plugin output value: {error}")))?;
        if decoded.type_id().to_string() != value.type_id
            || decoded.family() != sdk_value_family(value.family)
        {
            return Err(wit_host_failure(
                "encoded plugin output metadata disagrees with its canonical ABI value",
            ));
        }
        invocation
            .create_output_value_after_begin(decoded)
            .map(wit_value_handle)
            .map_err(wit_error)
    }

    fn push_output(
        &mut self,
        port_id: String,
        handle: wit_contract::sim::comfy_plugin::host::ValueHandle,
    ) -> Result<(), WitInvocationError> {
        self.invocation_mut()?
            .push_output(&port_id, sdk_value_handle(handle))
            .map_err(wit_error)
    }

    fn finish_output(&mut self, port_id: String, present: bool) -> Result<(), WitInvocationError> {
        self.invocation_mut()?
            .finish_output(&port_id, present)
            .map_err(wit_error)
    }

    fn check_cancelled(&mut self) -> Result<(), WitInvocationError> {
        self.invocation_mut()?.check_cancelled().map_err(wit_error)
    }

    fn filesystem_read(
        &mut self,
        root: String,
        relative_path: String,
    ) -> Result<Vec<u8>, WitInvocationError> {
        match self.call(CapabilityCall::FilesystemRead {
            root,
            relative_path,
        })? {
            CapabilityResponse::Bytes(bytes) => Ok(bytes),
            _ => Err(wit_host_failure(
                "filesystem host returned the wrong response type",
            )),
        }
    }

    fn provider_request(
        &mut self,
        provider: String,
        endpoint: String,
        body: Vec<u8>,
        secret_id: Option<String>,
    ) -> Result<Vec<u8>, WitInvocationError> {
        match self.call(CapabilityCall::NetworkProvider {
            provider,
            endpoint,
            body,
            secret_id,
        })? {
            CapabilityResponse::Bytes(bytes) => Ok(bytes),
            _ => Err(wit_host_failure(
                "provider host returned the wrong response type",
            )),
        }
    }

    fn secret_exists(&mut self, identifier: String) -> Result<bool, WitInvocationError> {
        match self.call(CapabilityCall::SecretExists { identifier })? {
            CapabilityResponse::Boolean(exists) => Ok(exists),
            _ => Err(wit_host_failure(
                "secret host returned the wrong response type",
            )),
        }
    }

    fn clock_now(&mut self, clock: String) -> Result<u64, WitInvocationError> {
        match self.call(CapabilityCall::ClockNow { clock })? {
            CapabilityResponse::TimestampMilliseconds(milliseconds) => Ok(milliseconds),
            _ => Err(wit_host_failure(
                "clock host returned the wrong response type",
            )),
        }
    }

    fn random_bytes(&mut self, stream: String, length: u32) -> Result<Vec<u8>, WitInvocationError> {
        match self.call(CapabilityCall::RandomBytes { stream, length })? {
            CapabilityResponse::Bytes(bytes) => Ok(bytes),
            _ => Err(wit_host_failure(
                "random host returned the wrong response type",
            )),
        }
    }

    fn model_open(&mut self, identifier: String) -> Result<u64, WitInvocationError> {
        match self.call(CapabilityCall::ModelOpen { identifier })? {
            CapabilityResponse::Handle(handle) => Ok(handle),
            _ => Err(wit_host_failure(
                "model host returned the wrong response type",
            )),
        }
    }

    fn output_begin(&mut self, namespace: String, name: String) -> Result<u64, WitInvocationError> {
        match self.call(CapabilityCall::OutputBegin { namespace, name })? {
            CapabilityResponse::Handle(handle) => Ok(handle),
            _ => Err(wit_host_failure(
                "output host returned the wrong response type",
            )),
        }
    }

    fn output_write(&mut self, transaction: u64, bytes: Vec<u8>) -> Result<(), WitInvocationError> {
        match self.call(CapabilityCall::OutputWrite { transaction, bytes })? {
            CapabilityResponse::Unit => Ok(()),
            _ => Err(wit_host_failure(
                "output host returned the wrong response type",
            )),
        }
    }

    fn output_commit(&mut self, transaction: u64) -> Result<String, WitInvocationError> {
        match self.call(CapabilityCall::OutputCommit { transaction })? {
            CapabilityResponse::CommittedArtifact(identifier) => Ok(identifier),
            _ => Err(wit_host_failure(
                "output host returned the wrong response type",
            )),
        }
    }

    fn log(&mut self, level: String, message: String) -> Result<(), WitInvocationError> {
        match self.call(CapabilityCall::Log { level, message })? {
            CapabilityResponse::Unit => Ok(()),
            _ => Err(wit_host_failure(
                "log host returned the wrong response type",
            )),
        }
    }

    fn ui_set(&mut self, contribution: String, state: Vec<u8>) -> Result<(), WitInvocationError> {
        match self.call(CapabilityCall::UiSet {
            contribution,
            state,
        })? {
            CapabilityResponse::Unit => Ok(()),
            _ => Err(wit_host_failure("UI host returned the wrong response type")),
        }
    }

    fn route_respond(
        &mut self,
        route: String,
        status: u16,
        body: Vec<u8>,
    ) -> Result<(), WitInvocationError> {
        match self.call(CapabilityCall::RouteRespond {
            route,
            status,
            body,
        })? {
            CapabilityResponse::Unit => Ok(()),
            _ => Err(wit_host_failure(
                "route host returned the wrong response type",
            )),
        }
    }
}

impl WasmStoreState {
    fn invocation_mut(&mut self) -> Result<&mut InvocationHost, WitInvocationError> {
        self.invocation
            .as_mut()
            .ok_or_else(|| wit_host_failure("WASM store has no active plugin invocation"))
    }

    fn call(&mut self, call: CapabilityCall) -> Result<CapabilityResponse, WitInvocationError> {
        self.invocation_mut()?.call(call).map_err(wit_error)
    }
}

fn wit_value_handle(handle: ValueHandle) -> wit_contract::sim::comfy_plugin::host::ValueHandle {
    wit_contract::sim::comfy_plugin::host::ValueHandle {
        invocation: handle.invocation,
        slot: handle.slot,
        generation: handle.generation,
    }
}

fn sdk_value_handle(handle: wit_contract::sim::comfy_plugin::host::ValueHandle) -> ValueHandle {
    ValueHandle {
        invocation: handle.invocation,
        slot: handle.slot,
        generation: handle.generation,
    }
}

fn sdk_manifest_projection(
    projection: wit_contract::sim::comfy_plugin::types::ManifestProjection,
) -> Result<ComponentManifestProjection, PluginError> {
    Ok(ComponentManifestProjection {
        component_world: projection.component_world,
        schema_version: projection.schema_version,
        identifier: projection.identifier,
        plugin_version: sdk_api_version(projection.plugin_version),
        api: comfy_plugin_sdk::ApiRequirement {
            major: projection.api.major,
            minimum_minor: projection.api.minimum_minor,
            maximum_minor: projection.api.maximum_minor,
            required_features: projection.api.required_features,
        },
        nodes: projection
            .nodes
            .into_iter()
            .map(sdk_projected_node)
            .collect::<Result<Vec<_>, PluginError>>()?,
        capabilities: projection
            .capabilities
            .into_iter()
            .map(|request| comfy_plugin_sdk::CapabilityRequest {
                kind: sdk_capability_kind(request.kind),
                scope: request.scope,
                quota: comfy_plugin_sdk::CapabilityQuota {
                    maximum_operations: request.quota.maximum_operations,
                    maximum_request_bytes: request.quota.maximum_request_bytes,
                    maximum_response_bytes: request.quota.maximum_response_bytes,
                    maximum_total_bytes: request.quota.maximum_total_bytes,
                    maximum_handles: request.quota.maximum_handles,
                    timeout_milliseconds: request.quota.timeout_milliseconds,
                },
            })
            .collect(),
        ui: projection
            .ui
            .into_iter()
            .map(|contribution| comfy_plugin_sdk::UiContribution {
                id: contribution.id,
                surface: contribution.surface,
                state_schema: contribution.state_schema,
            })
            .collect(),
        routes: projection
            .routes
            .into_iter()
            .map(|route| comfy_plugin_sdk::RouteDeclaration {
                id: route.id,
                method: route.method,
                path: route.path,
                maximum_request_bytes: route.maximum_request_bytes,
                maximum_response_bytes: route.maximum_response_bytes,
            })
            .collect(),
        legacy_mappings: projection
            .legacy_mappings
            .into_iter()
            .map(|mapping| comfy_plugin_sdk::ComponentLegacyMapping {
                legacy_identifier: mapping.legacy_identifier,
                node_id: mapping.node_id,
                node_version: sdk_api_version(mapping.node_version),
            })
            .collect(),
    })
}

fn sdk_provider_binding_set(
    binding_set: wit_contract::sim::comfy_plugin::types::ProviderBindingSet,
) -> Result<ProviderBindingSet, PluginError> {
    Ok(ProviderBindingSet {
        schema_version: binding_set.schema_version,
        implementation_namespace: binding_set.implementation_namespace,
        bindings_sha256: binding_set.bindings_sha256,
        bindings: binding_set
            .bindings
            .into_iter()
            .map(|binding| {
                Ok(ProviderBindingClaim {
                    feature_id: binding.feature_id,
                    node_id: binding.node_id,
                    contract_sha256: binding.contract_sha256,
                    transport_schema: CanonicalTypeId::from_str(&binding.transport_schema)
                        .map_err(|_| PluginError::ProviderBindingMismatch)?,
                    materializer_schema: CanonicalTypeId::from_str(&binding.materializer_schema)
                        .map_err(|_| PluginError::ProviderBindingMismatch)?,
                })
            })
            .collect::<Result<Vec<_>, PluginError>>()?,
    })
}

fn sdk_projected_node(
    node: wit_contract::sim::comfy_plugin::types::Node,
) -> Result<PluginNode, PluginError> {
    Ok(PluginNode {
        id: node.id,
        version: sdk_api_version(node.version),
        display_name: node.display_name,
        category: node.category,
        ports: node
            .ports
            .into_iter()
            .map(sdk_projected_port)
            .collect::<Result<Vec<_>, PluginError>>()?,
        determinism: match node.determinism {
            wit_contract::sim::comfy_plugin::types::DeterminismPolicy::Deterministic => {
                comfy_plugin_sdk::DeterminismPolicy::Deterministic
            }
            wit_contract::sim::comfy_plugin::types::DeterminismPolicy::Seeded => {
                comfy_plugin_sdk::DeterminismPolicy::Seeded
            }
            wit_contract::sim::comfy_plugin::types::DeterminismPolicy::External => {
                comfy_plugin_sdk::DeterminismPolicy::External
            }
        },
        cache: match node.cache {
            wit_contract::sim::comfy_plugin::types::CachePolicy::InputIdentity => {
                comfy_plugin_sdk::CachePolicy::InputIdentity
            }
            wit_contract::sim::comfy_plugin::types::CachePolicy::Never => {
                comfy_plugin_sdk::CachePolicy::Never
            }
            wit_contract::sim::comfy_plugin::types::CachePolicy::PluginKey => {
                comfy_plugin_sdk::CachePolicy::PluginKey
            }
        },
        effects: match node.effects {
            wit_contract::sim::comfy_plugin::types::EffectPolicy::Pure => {
                comfy_plugin_sdk::EffectPolicy::Pure
            }
            wit_contract::sim::comfy_plugin::types::EffectPolicy::Transactional => {
                comfy_plugin_sdk::EffectPolicy::Transactional
            }
            wit_contract::sim::comfy_plugin::types::EffectPolicy::Provider => {
                comfy_plugin_sdk::EffectPolicy::Provider
            }
        },
    })
}

fn sdk_projected_port(
    port: wit_contract::sim::comfy_plugin::types::Port,
) -> Result<comfy_plugin_sdk::PluginPort, PluginError> {
    let type_id = port.type_id.parse().map_err(PluginContractError::from)?;
    let default = port.default.map(sdk_projected_scalar).transpose()?;
    Ok(comfy_plugin_sdk::PluginPort {
        id: port.id,
        name: port.name,
        direction: match port.direction {
            wit_contract::sim::comfy_plugin::types::PortDirection::Input => PortDirection::Input,
            wit_contract::sim::comfy_plugin::types::PortDirection::Output => PortDirection::Output,
        },
        type_id,
        cardinality: match port.cardinality {
            wit_contract::sim::comfy_plugin::types::PortCardinality::Singular => {
                PortCardinality::Singular
            }
            wit_contract::sim::comfy_plugin::types::PortCardinality::List => PortCardinality::List,
        },
        presence: match port.presence {
            wit_contract::sim::comfy_plugin::types::PortPresence::Required => {
                PortPresence::Required
            }
            wit_contract::sim::comfy_plugin::types::PortPresence::Optional => {
                PortPresence::Optional
            }
            wit_contract::sim::comfy_plugin::types::PortPresence::Hidden => PortPresence::Hidden,
        },
        hidden: port.hidden,
        lazy: port.lazy,
        default,
        serialization: match port.serialization {
            wit_contract::sim::comfy_plugin::types::PortSerialization::Inline => {
                PortSerialization::Inline
            }
            wit_contract::sim::comfy_plugin::types::PortSerialization::Handle => {
                PortSerialization::Handle
            }
            wit_contract::sim::comfy_plugin::types::PortSerialization::ArtifactReference => {
                PortSerialization::ArtifactReference
            }
            wit_contract::sim::comfy_plugin::types::PortSerialization::OpaquePreserved => {
                PortSerialization::OpaquePreserved
            }
        },
        accepted_legacy_names: port.accepted_legacy_names,
    })
}

fn sdk_projected_scalar(
    scalar: wit_contract::sim::comfy_plugin::types::ScalarValue,
) -> Result<comfy_plugin_sdk::ScalarValue, PluginError> {
    const MAX_SCALAR_NODES: usize = 65_536;
    const MAX_SCALAR_DEPTH: usize = 64;
    const MAX_SCALAR_BYTES: usize = 8 * 1024 * 1024;

    let root =
        usize::try_from(scalar.root_node).map_err(|_| PluginError::ManifestProjectionMismatch)?;
    if scalar.nodes.is_empty()
        || scalar.nodes.len() > MAX_SCALAR_NODES
        || root >= scalar.nodes.len()
    {
        return Err(PluginError::ManifestProjectionMismatch);
    }

    let mut parent_counts = vec![0_u8; scalar.nodes.len()];
    let mut encoded_bytes = 0_usize;
    for node in &scalar.nodes {
        use wit_contract::sim::comfy_plugin::types::ScalarNode;
        let children: &[u32] = match node {
            ScalarNode::ListValue(children) => children,
            ScalarNode::RecordValue(entries) => {
                for entry in entries {
                    encoded_bytes = encoded_bytes
                        .checked_add(entry.key.len())
                        .ok_or(PluginError::ManifestProjectionMismatch)?;
                    let child = usize::try_from(entry.value_node)
                        .map_err(|_| PluginError::ManifestProjectionMismatch)?;
                    let parent_count = parent_counts
                        .get_mut(child)
                        .ok_or(PluginError::ManifestProjectionMismatch)?;
                    *parent_count = parent_count
                        .checked_add(1)
                        .ok_or(PluginError::ManifestProjectionMismatch)?;
                    if *parent_count > 1 {
                        return Err(PluginError::ManifestProjectionMismatch);
                    }
                }
                &[]
            }
            ScalarNode::TextValue(value) => {
                encoded_bytes = encoded_bytes
                    .checked_add(value.len())
                    .ok_or(PluginError::ManifestProjectionMismatch)?;
                &[]
            }
            ScalarNode::BytesValue(value) => {
                encoded_bytes = encoded_bytes
                    .checked_add(value.len())
                    .ok_or(PluginError::ManifestProjectionMismatch)?;
                &[]
            }
            ScalarNode::NullValue
            | ScalarNode::BooleanValue(_)
            | ScalarNode::IntegerValue(_)
            | ScalarNode::FloatValue(_) => &[],
        };
        for child in children {
            let child =
                usize::try_from(*child).map_err(|_| PluginError::ManifestProjectionMismatch)?;
            let parent_count = parent_counts
                .get_mut(child)
                .ok_or(PluginError::ManifestProjectionMismatch)?;
            *parent_count = parent_count
                .checked_add(1)
                .ok_or(PluginError::ManifestProjectionMismatch)?;
            if *parent_count > 1 {
                return Err(PluginError::ManifestProjectionMismatch);
            }
        }
        if encoded_bytes > MAX_SCALAR_BYTES {
            return Err(PluginError::ManifestProjectionMismatch);
        }
    }
    if parent_counts[root] != 0
        || parent_counts
            .iter()
            .enumerate()
            .any(|(index, count)| index != root && *count != 1)
    {
        return Err(PluginError::ManifestProjectionMismatch);
    }

    let mut states = vec![0_u8; scalar.nodes.len()];
    let mut order = Vec::with_capacity(scalar.nodes.len());
    let mut stack = vec![(root, 0_usize, false)];
    while let Some((index, depth, exiting)) = stack.pop() {
        if depth > MAX_SCALAR_DEPTH {
            return Err(PluginError::ManifestProjectionMismatch);
        }
        if exiting {
            states[index] = 2;
            order.push(index);
            continue;
        }
        match states[index] {
            1 => return Err(PluginError::ManifestProjectionMismatch),
            2 => continue,
            _ => states[index] = 1,
        }
        stack.push((index, depth, true));
        let children = projected_scalar_children(&scalar.nodes[index]);
        for child in children.into_iter().rev() {
            let child =
                usize::try_from(child).map_err(|_| PluginError::ManifestProjectionMismatch)?;
            if child >= scalar.nodes.len() {
                return Err(PluginError::ManifestProjectionMismatch);
            }
            stack.push((child, depth + 1, false));
        }
    }
    if states.iter().any(|state| *state != 2) {
        return Err(PluginError::ManifestProjectionMismatch);
    }

    let mut values = vec![None; scalar.nodes.len()];
    for index in order {
        use wit_contract::sim::comfy_plugin::types::ScalarNode;
        let value = match &scalar.nodes[index] {
            ScalarNode::NullValue => comfy_plugin_sdk::ScalarValue::Null,
            ScalarNode::BooleanValue(value) => comfy_plugin_sdk::ScalarValue::Boolean(*value),
            ScalarNode::IntegerValue(value) => comfy_plugin_sdk::ScalarValue::Integer(*value),
            ScalarNode::FloatValue(value) => comfy_plugin_sdk::ScalarValue::Float(*value),
            ScalarNode::TextValue(value) => comfy_plugin_sdk::ScalarValue::String(value.clone()),
            ScalarNode::BytesValue(value) => comfy_plugin_sdk::ScalarValue::Bytes(value.clone()),
            ScalarNode::ListValue(children) => comfy_plugin_sdk::ScalarValue::List(
                take_projected_scalar_children(children, &mut values)?,
            ),
            ScalarNode::RecordValue(entries) => comfy_plugin_sdk::ScalarValue::Record(
                entries
                    .iter()
                    .map(|entry| {
                        let child = take_projected_scalar_child(entry.value_node, &mut values)?;
                        Ok((entry.key.clone(), child))
                    })
                    .collect::<Result<Vec<_>, PluginError>>()?,
            ),
        };
        values[index] = Some(value);
    }
    let value = values[root]
        .take()
        .ok_or(PluginError::ManifestProjectionMismatch)?;
    value
        .abi_bytes()
        .map_err(|_| PluginError::ManifestProjectionMismatch)?;
    Ok(value)
}

fn projected_scalar_children(
    node: &wit_contract::sim::comfy_plugin::types::ScalarNode,
) -> Vec<u32> {
    use wit_contract::sim::comfy_plugin::types::ScalarNode;
    match node {
        ScalarNode::ListValue(children) => children.clone(),
        ScalarNode::RecordValue(entries) => entries.iter().map(|entry| entry.value_node).collect(),
        ScalarNode::NullValue
        | ScalarNode::BooleanValue(_)
        | ScalarNode::IntegerValue(_)
        | ScalarNode::FloatValue(_)
        | ScalarNode::TextValue(_)
        | ScalarNode::BytesValue(_) => Vec::new(),
    }
}

fn take_projected_scalar_children(
    children: &[u32],
    values: &mut [Option<comfy_plugin_sdk::ScalarValue>],
) -> Result<Vec<comfy_plugin_sdk::ScalarValue>, PluginError> {
    children
        .iter()
        .map(|child| take_projected_scalar_child(*child, values))
        .collect()
}

fn take_projected_scalar_child(
    child: u32,
    values: &mut [Option<comfy_plugin_sdk::ScalarValue>],
) -> Result<comfy_plugin_sdk::ScalarValue, PluginError> {
    let child = usize::try_from(child).map_err(|_| PluginError::ManifestProjectionMismatch)?;
    values
        .get_mut(child)
        .and_then(Option::take)
        .ok_or(PluginError::ManifestProjectionMismatch)
}

fn sdk_api_version(
    version: wit_contract::sim::comfy_plugin::types::ApiVersion,
) -> comfy_plugin_sdk::ApiVersion {
    comfy_plugin_sdk::ApiVersion::new(version.major, version.minor, version.patch)
}

fn wit_error(error: InvocationError) -> WitInvocationError {
    use wit_contract::sim::comfy_plugin::types::{
        CapabilityError, InvocationError as WitError, PortIndexError, QuotaError, ValueFamilyError,
    };
    match error {
        InvocationError::Cancelled => WitError::Cancelled,
        InvocationError::TimedOut => WitError::TimedOut,
        InvocationError::UnknownPort(port) => WitError::UnknownPort(sanitize_diagnostic(&port)),
        InvocationError::WrongDirection(port) => {
            WitError::WrongDirection(sanitize_diagnostic(&port))
        }
        InvocationError::MissingRequiredPort(port) => {
            WitError::MissingRequiredPort(sanitize_diagnostic(&port))
        }
        InvocationError::InvalidCardinality(port) => {
            WitError::InvalidCardinality(sanitize_diagnostic(&port))
        }
        InvocationError::IndexOutOfBounds { port, index } => {
            WitError::IndexOutOfBounds(PortIndexError {
                port: sanitize_diagnostic(&port),
                index,
            })
        }
        InvocationError::AlreadyTaken { port, index } => WitError::AlreadyTaken(PortIndexError {
            port: sanitize_diagnostic(&port),
            index,
        }),
        InvocationError::InvalidHandle => WitError::InvalidHandle,
        InvocationError::RevokedHandle => WitError::RevokedHandle,
        InvocationError::WrongValueFamily {
            port,
            expected,
            actual,
        } => WitError::WrongValueFamily(ValueFamilyError {
            port: sanitize_diagnostic(&port),
            expected: wit_value_family(expected),
            actual: wit_value_family(actual),
        }),
        InvocationError::OutputAlreadyFinished(port) => {
            WitError::OutputAlreadyFinished(sanitize_diagnostic(&port))
        }
        InvocationError::UnfinishedOutput(port) => {
            WitError::UnfinishedOutput(sanitize_diagnostic(&port))
        }
        InvocationError::CapabilityDenied { kind, scope } => {
            WitError::CapabilityDenied(CapabilityError {
                kind: wit_capability_kind(kind),
                scope: sanitize_diagnostic(&scope),
            })
        }
        InvocationError::QuotaExceeded { kind, limit } => WitError::QuotaExceeded(QuotaError {
            kind: wit_capability_kind(kind),
            limit: sanitize_diagnostic(&limit),
        }),
        InvocationError::InvocationQuotaExceeded { limit } => {
            WitError::InvocationQuotaExceeded(sanitize_diagnostic(&limit))
        }
        InvocationError::InvalidCapabilityRequest(message) => {
            WitError::InvalidCapabilityRequest(sanitize_diagnostic(&message))
        }
        InvocationError::HostFailure(message) => {
            WitError::HostFailure(sanitize_diagnostic(&message))
        }
        InvocationError::PluginFailure(message) => {
            WitError::PluginFailure(sanitize_diagnostic(&message))
        }
    }
}

fn sdk_error(error: WitInvocationError) -> InvocationError {
    use wit_contract::sim::comfy_plugin::types::InvocationError as WitError;
    match error {
        WitError::Cancelled => InvocationError::Cancelled,
        WitError::TimedOut => InvocationError::TimedOut,
        WitError::UnknownPort(port) => InvocationError::UnknownPort(sanitize_diagnostic(&port)),
        WitError::WrongDirection(port) => {
            InvocationError::WrongDirection(sanitize_diagnostic(&port))
        }
        WitError::MissingRequiredPort(port) => {
            InvocationError::MissingRequiredPort(sanitize_diagnostic(&port))
        }
        WitError::InvalidCardinality(port) => {
            InvocationError::InvalidCardinality(sanitize_diagnostic(&port))
        }
        WitError::IndexOutOfBounds(error) => InvocationError::IndexOutOfBounds {
            port: sanitize_diagnostic(&error.port),
            index: error.index,
        },
        WitError::AlreadyTaken(error) => InvocationError::AlreadyTaken {
            port: sanitize_diagnostic(&error.port),
            index: error.index,
        },
        WitError::InvalidHandle => InvocationError::InvalidHandle,
        WitError::RevokedHandle => InvocationError::RevokedHandle,
        WitError::WrongValueFamily(error) => InvocationError::WrongValueFamily {
            port: sanitize_diagnostic(&error.port),
            expected: sdk_value_family(error.expected),
            actual: sdk_value_family(error.actual),
        },
        WitError::OutputAlreadyFinished(port) => {
            InvocationError::OutputAlreadyFinished(sanitize_diagnostic(&port))
        }
        WitError::UnfinishedOutput(port) => {
            InvocationError::UnfinishedOutput(sanitize_diagnostic(&port))
        }
        WitError::CapabilityDenied(error) => InvocationError::CapabilityDenied {
            kind: sdk_capability_kind(error.kind),
            scope: sanitize_diagnostic(&error.scope),
        },
        WitError::QuotaExceeded(error) => InvocationError::QuotaExceeded {
            kind: sdk_capability_kind(error.kind),
            limit: sanitize_diagnostic(&error.limit),
        },
        WitError::InvocationQuotaExceeded(limit) => InvocationError::InvocationQuotaExceeded {
            limit: sanitize_diagnostic(&limit),
        },
        WitError::InvalidCapabilityRequest(message) => {
            InvocationError::InvalidCapabilityRequest(sanitize_diagnostic(&message))
        }
        WitError::HostFailure(message) => {
            InvocationError::HostFailure(sanitize_diagnostic(&message))
        }
        WitError::PluginFailure(message) => {
            InvocationError::PluginFailure(sanitize_diagnostic(&message))
        }
    }
}

fn wit_host_failure(message: &str) -> WitInvocationError {
    WitInvocationError::HostFailure(sanitize_diagnostic(message))
}

fn wit_value_family(
    family: comfy_plugin_sdk::ValueFamily,
) -> wit_contract::sim::comfy_plugin::types::ValueFamily {
    use comfy_plugin_sdk::ValueFamily;
    use wit_contract::sim::comfy_plugin::types::ValueFamily as WitFamily;
    match family {
        ValueFamily::Scalar => WitFamily::Scalar,
        ValueFamily::Tensor => WitFamily::Tensor,
        ValueFamily::Artifact => WitFamily::Artifact,
        ValueFamily::Model => WitFamily::Model,
    }
}

fn sdk_value_family(
    family: wit_contract::sim::comfy_plugin::types::ValueFamily,
) -> comfy_plugin_sdk::ValueFamily {
    use comfy_plugin_sdk::ValueFamily;
    use wit_contract::sim::comfy_plugin::types::ValueFamily as WitFamily;
    match family {
        WitFamily::Scalar => ValueFamily::Scalar,
        WitFamily::Tensor => ValueFamily::Tensor,
        WitFamily::Artifact => ValueFamily::Artifact,
        WitFamily::Model => ValueFamily::Model,
    }
}

fn wit_port_cardinality(
    cardinality: PortCardinality,
) -> wit_contract::sim::comfy_plugin::types::PortCardinality {
    use wit_contract::sim::comfy_plugin::types::PortCardinality as WitCardinality;
    match cardinality {
        PortCardinality::Singular => WitCardinality::Singular,
        PortCardinality::List => WitCardinality::List,
    }
}

fn wit_port_presence(
    presence: PortPresence,
) -> wit_contract::sim::comfy_plugin::types::PortPresence {
    use wit_contract::sim::comfy_plugin::types::PortPresence as WitPresence;
    match presence {
        PortPresence::Required => WitPresence::Required,
        PortPresence::Optional => WitPresence::Optional,
        PortPresence::Hidden => WitPresence::Hidden,
    }
}

fn wit_port_serialization(
    serialization: PortSerialization,
) -> wit_contract::sim::comfy_plugin::types::PortSerialization {
    use wit_contract::sim::comfy_plugin::types::PortSerialization as WitSerialization;
    match serialization {
        PortSerialization::Inline => WitSerialization::Inline,
        PortSerialization::Handle => WitSerialization::Handle,
        PortSerialization::ArtifactReference => WitSerialization::ArtifactReference,
        PortSerialization::OpaquePreserved => WitSerialization::OpaquePreserved,
    }
}

fn wit_capability_kind(
    kind: comfy_plugin_sdk::CapabilityKind,
) -> wit_contract::sim::comfy_plugin::types::CapabilityKind {
    use comfy_plugin_sdk::CapabilityKind;
    use wit_contract::sim::comfy_plugin::types::CapabilityKind as WitKind;
    match kind {
        CapabilityKind::Filesystem => WitKind::Filesystem,
        CapabilityKind::NetworkProvider => WitKind::NetworkProvider,
        CapabilityKind::Secret => WitKind::Secret,
        CapabilityKind::Clock => WitKind::Clock,
        CapabilityKind::Randomness => WitKind::Randomness,
        CapabilityKind::Model => WitKind::Model,
        CapabilityKind::TransactionalOutput => WitKind::TransactionalOutput,
        CapabilityKind::SanitizedLog => WitKind::SanitizedLog,
        CapabilityKind::DeclarativeUi => WitKind::DeclarativeUi,
        CapabilityKind::Route => WitKind::Route,
    }
}

fn sdk_capability_kind(
    kind: wit_contract::sim::comfy_plugin::types::CapabilityKind,
) -> comfy_plugin_sdk::CapabilityKind {
    use comfy_plugin_sdk::CapabilityKind;
    use wit_contract::sim::comfy_plugin::types::CapabilityKind as WitKind;
    match kind {
        WitKind::Filesystem => CapabilityKind::Filesystem,
        WitKind::NetworkProvider => CapabilityKind::NetworkProvider,
        WitKind::Secret => CapabilityKind::Secret,
        WitKind::Clock => CapabilityKind::Clock,
        WitKind::Randomness => CapabilityKind::Randomness,
        WitKind::Model => CapabilityKind::Model,
        WitKind::TransactionalOutput => CapabilityKind::TransactionalOutput,
        WitKind::SanitizedLog => CapabilityKind::SanitizedLog,
        WitKind::DeclarativeUi => CapabilityKind::DeclarativeUi,
        WitKind::Route => CapabilityKind::Route,
    }
}

fn wit_cancel_reason(reason: CancelReason) -> wit_contract::sim::comfy_plugin::types::CancelReason {
    use wit_contract::sim::comfy_plugin::types::CancelReason as WitReason;
    match reason {
        CancelReason::User => WitReason::User,
        CancelReason::Timeout => WitReason::Timeout,
        CancelReason::HostShutdown => WitReason::HostShutdown,
        CancelReason::CapabilityRevoked => WitReason::CapabilityRevoked,
    }
}

impl Drop for InvocationHost {
    fn drop(&mut self) {
        if !self.terminal {
            self.abort();
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct InvocationResult {
    pub outputs: BTreeMap<String, Vec<PluginValue>>,
    pub output_presence: BTreeMap<String, bool>,
    pub effects: CapabilityEffects,
}

fn validate_input_port(
    port: &comfy_plugin_sdk::PluginPort,
    present: bool,
    values: &[PluginValue],
    registry: &TypeRegistry,
    limits: &ComponentLimits,
) -> Result<u64, InvocationError> {
    if port.presence == PortPresence::Required && !present {
        return Err(InvocationError::MissingRequiredPort(port.id.clone()));
    }
    if !present && !values.is_empty() {
        return Err(InvocationError::InvalidCardinality(port.id.clone()));
    }
    match port.cardinality {
        PortCardinality::Singular if present && values.len() != 1 => {
            return Err(InvocationError::InvalidCardinality(port.id.clone()));
        }
        PortCardinality::List if values.len() > limits.maximum_values_per_port => {
            return Err(InvocationError::InvalidCardinality(port.id.clone()));
        }
        _ => {}
    }
    let expected = registry.family(&port.type_id).map_err(|error| {
        InvocationError::HostFailure(format!("plugin type registry failed: {error}"))
    })?;
    let mut total_bytes = 0_u64;
    for value in values {
        if value.family() != expected {
            return Err(InvocationError::WrongValueFamily {
                port: port.id.clone(),
                expected,
                actual: value.family(),
            });
        }
        require_exact_port_type(port, value)?;
        total_bytes = total_bytes
            .checked_add(value_size(value)?)
            .ok_or_else(|| value_quota_error())?;
        if total_bytes > limits.maximum_value_bytes {
            return Err(value_quota_error());
        }
    }
    Ok(total_bytes)
}

fn validate_finished_output(
    port: &comfy_plugin_sdk::PluginPort,
    present: bool,
    length: usize,
) -> Result<(), InvocationError> {
    if !present {
        if port.presence != PortPresence::Optional {
            return Err(InvocationError::MissingRequiredPort(port.id.clone()));
        }
        if length != 0 {
            return Err(InvocationError::InvalidCardinality(port.id.clone()));
        }
        return Ok(());
    }
    if port.cardinality == PortCardinality::Singular && length != 1 {
        return Err(if length == 0 {
            InvocationError::MissingRequiredPort(port.id.clone())
        } else {
            InvocationError::InvalidCardinality(port.id.clone())
        });
    }
    Ok(())
}

fn value_size(value: &PluginValue) -> Result<u64, InvocationError> {
    let size = match value.representation() {
        PluginValueRepresentation::Scalar(value) => scalar_size(value)?,
        PluginValueRepresentation::Tensor(value) => value.byte_length(),
        PluginValueRepresentation::Artifact(value) => {
            if canonical_artifact_identity(value.namespace(), value.identifier()).is_err()
                || !valid_sha256(value.digest())
            {
                return Err(invalid_value("artifact metadata"));
            }
            value.byte_length()
        }
        PluginValueRepresentation::Model(value) => {
            if !valid_value_text(value.identifier(), 4_096)
                || !valid_value_text(value.format(), 128)
                || !valid_sha256(value.digest())
            {
                return Err(invalid_value("model metadata"));
            }
            value
                .identifier()
                .len()
                .checked_add(value.format().len())
                .and_then(|length| length.checked_add(value.digest().len()))
                .and_then(|length| u64::try_from(length).ok())
                .ok_or_else(|| {
                    InvocationError::HostFailure("model value size overflow".to_owned())
                })?
        }
    };
    Ok(size)
}

fn encoded_value_response_size(value: &PluginValue) -> Result<u64, InvocationError> {
    let abi_bytes = value.abi_bytes().map_err(|error| {
        InvocationError::HostFailure(format!("plugin value ABI encoding failed: {error}"))
    })?;
    let type_id_bytes = value.type_id().to_string().len();
    type_id_bytes
        .checked_add(abi_bytes.len())
        .and_then(|bytes| bytes.checked_add(1))
        .and_then(|bytes| u64::try_from(bytes).ok())
        .ok_or_else(|| InvocationError::HostFailure("plugin value ABI size overflow".to_owned()))
}

fn require_exact_port_type(
    port: &comfy_plugin_sdk::PluginPort,
    value: &PluginValue,
) -> Result<(), InvocationError> {
    if value.type_id() != &port.type_id {
        return Err(InvocationError::HostFailure(format!(
            "port `{}` expects canonical type `{}`, received `{}`",
            port.id,
            port.type_id,
            value.type_id()
        )));
    }
    Ok(())
}

fn scalar_size(value: &comfy_plugin_sdk::ScalarValue) -> Result<u64, InvocationError> {
    use comfy_plugin_sdk::ScalarValue;
    let mut stack = vec![(value, 0_usize)];
    let mut total = 0_u64;
    let mut value_count = 0_usize;
    while let Some((value, depth)) = stack.pop() {
        if depth > 64 || value_count >= 65_536 {
            return Err(InvocationError::HostFailure(
                "scalar nesting or value count exceeds the host limit".to_owned(),
            ));
        }
        value_count += 1;
        let bytes = match value {
            ScalarValue::Null => 0,
            ScalarValue::Boolean(_) => 1,
            ScalarValue::Integer(_) => 8,
            ScalarValue::Float(value) => {
                if !value.is_finite() {
                    return Err(invalid_value("non-finite scalar float"));
                }
                8
            }
            ScalarValue::String(value) => u64::try_from(value.len())
                .map_err(|_| InvocationError::HostFailure("scalar size overflow".to_owned()))?,
            ScalarValue::Bytes(value) => u64::try_from(value.len())
                .map_err(|_| InvocationError::HostFailure("scalar size overflow".to_owned()))?,
            ScalarValue::List(values) => {
                stack.extend(values.iter().map(|value| (value, depth + 1)));
                0
            }
            ScalarValue::Record(values) => {
                let mut keys = BTreeSet::new();
                for (key, value) in values {
                    if key.is_empty() || key.len() > 1_024 || !keys.insert(key) {
                        return Err(invalid_value("scalar record key"));
                    }
                    total = total
                        .checked_add(u64::try_from(key.len()).map_err(|_| {
                            InvocationError::HostFailure("scalar size overflow".to_owned())
                        })?)
                        .ok_or_else(|| {
                            InvocationError::HostFailure("scalar size overflow".to_owned())
                        })?;
                    stack.push((value, depth + 1));
                }
                0
            }
        };
        total = total
            .checked_add(bytes)
            .ok_or_else(|| InvocationError::HostFailure("scalar size overflow".to_owned()))?;
    }
    Ok(total)
}

fn next_invocation_id() -> Result<u64, InvocationError> {
    NEXT_INVOCATION_ID
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |identifier| {
            identifier.checked_add(1)
        })
        .map_err(|_| {
            InvocationError::HostFailure("invocation identifier space exhausted".to_owned())
        })
}

fn invocation_value_quota_error() -> InvocationError {
    InvocationError::InvocationQuotaExceeded {
        limit: "invocation-value-byte".to_owned(),
    }
}

fn value_quota_error() -> InvocationError {
    InvocationError::InvocationQuotaExceeded {
        limit: "value-byte".to_owned(),
    }
}

fn invalid_value(subject: &str) -> InvocationError {
    InvocationError::HostFailure(format!("invalid plugin {subject}"))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_value_text(value: &str, maximum_bytes: usize) -> bool {
    !value.is_empty() && value.len() <= maximum_bytes && !value.chars().any(char::is_control)
}

fn canonical_artifact_identity(namespace: &str, identifier: &str) -> Result<AssetIdentity, ()> {
    let namespace = AssetNamespace::from_locator_type(namespace).map_err(|_| ())?;
    AssetIdentity::new("plugin-wire", namespace, identifier).map_err(|_| ())
}

fn validate_component_projection(
    expected: &ComponentManifestProjection,
    actual: &ComponentManifestProjection,
) -> Result<(), PluginError> {
    if actual.canonical_bytes().is_err() || expected != actual {
        return Err(PluginError::ManifestProjectionMismatch);
    }
    Ok(())
}

fn component_compilation_error(error: wasmtime::Error) -> PluginError {
    PluginError::ComponentCompilation(sanitize_diagnostic(&error.to_string()))
}

fn component_instantiation_error(error: wasmtime::Error) -> PluginError {
    PluginError::WasmTrap(sanitize_diagnostic(&error.to_string()))
}

fn preflight_component_error(error: PluginError) -> PluginError {
    match error {
        PluginError::WasmTrap(message) => PluginError::ComponentCompilation(message),
        error => error,
    }
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        result.push(char::from(HEX[usize::from(byte >> 4)]));
        result.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    result
}

fn sanitize_diagnostic(message: &str) -> String {
    message
        .chars()
        .filter(|character| !character.is_control() || matches!(character, '\n' | '\t'))
        .take(4_096)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn component_projection_requires_an_exact_bounded_match() {
        let signed = ComponentManifestProjection {
            component_world: comfy_plugin_sdk::COMPONENT_WORLD.to_owned(),
            schema_version: comfy_plugin_sdk::MANIFEST_SCHEMA_VERSION,
            identifier: "test.plugin".to_owned(),
            plugin_version: comfy_plugin_sdk::ApiVersion::new(1, 0, 0),
            api: comfy_plugin_sdk::ApiRequirement {
                major: 1,
                minimum_minor: 0,
                maximum_minor: 0,
                required_features: Vec::new(),
            },
            nodes: Vec::new(),
            capabilities: Vec::new(),
            ui: Vec::new(),
            routes: Vec::new(),
            legacy_mappings: Vec::new(),
        };
        assert!(validate_component_projection(&signed, &signed).is_ok());
        let mut changed = signed.clone();
        changed.identifier = "test.changed".to_owned();
        assert!(matches!(
            validate_component_projection(&signed, &changed),
            Err(PluginError::ManifestProjectionMismatch)
        ));
        let mut oversized = signed;
        oversized.identifier = "x".repeat(comfy_plugin_sdk::MAX_MANIFEST_BYTES + 1);
        assert!(matches!(
            validate_component_projection(&oversized, &oversized),
            Err(PluginError::ManifestProjectionMismatch)
        ));
    }

    #[test]
    fn typed_scalar_projection_rejects_hostile_graphs() -> Result<(), PluginError> {
        use wit_contract::sim::comfy_plugin::types::{
            ScalarNode as WitScalarNode, ScalarRecordEntry, ScalarValue as WitScalarValue,
        };

        let valid = sdk_projected_scalar(WitScalarValue {
            root_node: 0,
            nodes: vec![
                WitScalarNode::RecordValue(vec![ScalarRecordEntry {
                    key: "items".to_owned(),
                    value_node: 1,
                }]),
                WitScalarNode::ListValue(vec![2]),
                WitScalarNode::TextValue("value".to_owned()),
            ],
        })?;
        assert_eq!(
            valid,
            comfy_plugin_sdk::ScalarValue::Record(vec![(
                "items".to_owned(),
                comfy_plugin_sdk::ScalarValue::List(vec![comfy_plugin_sdk::ScalarValue::String(
                    "value".to_owned()
                ),]),
            )])
        );

        let hostile = [
            WitScalarValue {
                root_node: 1,
                nodes: vec![WitScalarNode::NullValue],
            },
            WitScalarValue {
                root_node: 0,
                nodes: vec![WitScalarNode::ListValue(vec![0])],
            },
            WitScalarValue {
                root_node: 0,
                nodes: vec![WitScalarNode::NullValue, WitScalarNode::NullValue],
            },
            WitScalarValue {
                root_node: 0,
                nodes: vec![WitScalarNode::ListValue(vec![3])],
            },
            WitScalarValue {
                root_node: 0,
                nodes: vec![
                    WitScalarNode::RecordValue(vec![
                        ScalarRecordEntry {
                            key: "duplicate".to_owned(),
                            value_node: 1,
                        },
                        ScalarRecordEntry {
                            key: "duplicate".to_owned(),
                            value_node: 2,
                        },
                    ]),
                    WitScalarNode::NullValue,
                    WitScalarNode::NullValue,
                ],
            },
        ];
        for scalar in hostile {
            assert!(matches!(
                sdk_projected_scalar(scalar),
                Err(PluginError::ManifestProjectionMismatch)
            ));
        }

        let mut deep_nodes = (0..=64)
            .map(|index| WitScalarNode::ListValue(vec![index + 1]))
            .collect::<Vec<_>>();
        deep_nodes.push(WitScalarNode::NullValue);
        assert!(matches!(
            sdk_projected_scalar(WitScalarValue {
                root_node: 0,
                nodes: deep_nodes,
            }),
            Err(PluginError::ManifestProjectionMismatch)
        ));
        Ok(())
    }

    #[test]
    fn typed_wit_errors_round_trip_every_sdk_variant() {
        use comfy_plugin_sdk::{CapabilityKind, ValueFamily};
        let errors = vec![
            InvocationError::Cancelled,
            InvocationError::TimedOut,
            InvocationError::UnknownPort("input".to_owned()),
            InvocationError::WrongDirection("output".to_owned()),
            InvocationError::MissingRequiredPort("required".to_owned()),
            InvocationError::InvalidCardinality("list".to_owned()),
            InvocationError::IndexOutOfBounds {
                port: "items".to_owned(),
                index: 7,
            },
            InvocationError::AlreadyTaken {
                port: "items".to_owned(),
                index: 3,
            },
            InvocationError::InvalidHandle,
            InvocationError::RevokedHandle,
            InvocationError::WrongValueFamily {
                port: "image".to_owned(),
                expected: ValueFamily::Tensor,
                actual: ValueFamily::Scalar,
            },
            InvocationError::OutputAlreadyFinished("output".to_owned()),
            InvocationError::UnfinishedOutput("output".to_owned()),
            InvocationError::CapabilityDenied {
                kind: CapabilityKind::NetworkProvider,
                scope: "provider".to_owned(),
            },
            InvocationError::QuotaExceeded {
                kind: CapabilityKind::TransactionalOutput,
                limit: "total-byte".to_owned(),
            },
            InvocationError::InvocationQuotaExceeded {
                limit: "port-operation".to_owned(),
            },
            InvocationError::InvalidCapabilityRequest("invalid".to_owned()),
            InvocationError::HostFailure("host".to_owned()),
            InvocationError::PluginFailure("plugin".to_owned()),
        ];
        for error in errors {
            assert_eq!(sdk_error(wit_error(error.clone())), error);
        }
    }

    #[test]
    fn wasm_store_has_fuel_and_resource_limits() -> Result<(), PluginError> {
        let host = PluginHost::new()?;
        let store = host.new_wasm_store()?;
        assert_eq!(store.get_fuel().ok(), Some(host.limits.maximum_fuel));
        Ok(())
    }

    #[test]
    fn wasm_traps_and_hangs_are_contained() -> Result<(), Box<dyn Error>> {
        const TRAP_MODULE: &[u8] = &[
            0, 97, 115, 109, 1, 0, 0, 0, 1, 4, 1, 96, 0, 0, 3, 2, 1, 0, 7, 8, 1, 4, 116, 114, 97,
            112, 0, 0, 10, 5, 1, 3, 0, 0, 11,
        ];
        const HANG_MODULE: &[u8] = &[
            0, 97, 115, 109, 1, 0, 0, 0, 1, 4, 1, 96, 0, 0, 3, 2, 1, 0, 7, 8, 1, 4, 104, 97, 110,
            103, 0, 0, 10, 9, 1, 7, 0, 3, 64, 12, 0, 11, 11,
        ];

        let host = PluginHost::new()?;
        let trap_module = wasmtime::Module::new(host.runtime.engine(), TRAP_MODULE)?;
        let mut trap_store = host.new_wasm_store()?;
        let trap_instance = wasmtime::Instance::new(&mut trap_store, &trap_module, &[])?;
        let trap = trap_instance.get_typed_func::<(), ()>(&mut trap_store, "trap")?;
        assert!(trap.call(&mut trap_store, ()).is_err());

        let hang_module = wasmtime::Module::new(host.runtime.engine(), HANG_MODULE)?;
        let mut hang_store = host.new_wasm_store()?;
        let hang_instance = wasmtime::Instance::new(&mut hang_store, &hang_module, &[])?;
        let hang = hang_instance.get_typed_func::<(), ()>(&mut hang_store, "hang")?;
        assert!(hang.call(&mut hang_store, ()).is_err());
        assert!(
            hang_store
                .get_fuel()
                .is_ok_and(|remaining| remaining < host.limits.maximum_fuel)
        );

        let mut interrupted_store = host.new_wasm_store()?;
        let interrupted_instance =
            wasmtime::Instance::new(&mut interrupted_store, &hang_module, &[])?;
        let interrupted =
            interrupted_instance.get_typed_func::<(), ()>(&mut interrupted_store, "hang")?;
        host.interrupt_wasm();
        assert!(interrupted.call(&mut interrupted_store, ()).is_err());
        assert_eq!(
            interrupted_store.get_fuel().ok(),
            Some(host.limits.maximum_fuel)
        );
        Ok(())
    }

    #[test]
    fn wasm_memory_and_instance_growth_are_enforced() -> Result<(), Box<dyn Error>> {
        const MEMORY_GROW_MODULE: &[u8] = &[
            0, 97, 115, 109, 1, 0, 0, 0, 1, 6, 1, 96, 1, 127, 1, 127, 3, 2, 1, 0, 5, 3, 1, 0, 1, 7,
            8, 1, 4, 103, 114, 111, 119, 0, 0, 10, 8, 1, 6, 0, 32, 0, 64, 0, 11,
        ];
        const TABLE_GROW_MODULE: &[u8] = &[
            0, 97, 115, 109, 1, 0, 0, 0, 1, 6, 1, 96, 1, 127, 1, 127, 3, 2, 1, 0, 4, 4, 1, 112, 0,
            1, 7, 8, 1, 4, 103, 114, 111, 119, 0, 0, 10, 11, 1, 9, 0, 208, 112, 32, 0, 252, 15, 0,
            11,
        ];
        const EMPTY_MODULE: &[u8] = &[0, 97, 115, 109, 1, 0, 0, 0];
        let host = PluginHost::new()?;
        let memory_module = wasmtime::Module::new(host.runtime.engine(), MEMORY_GROW_MODULE)?;
        let mut memory_store = host.new_wasm_store()?;
        let memory_instance = wasmtime::Instance::new(&mut memory_store, &memory_module, &[])?;
        let grow = memory_instance.get_typed_func::<i32, i32>(&mut memory_store, "grow")?;
        assert!(grow.call(&mut memory_store, i32::MAX).is_err());

        let table_module = wasmtime::Module::new(host.runtime.engine(), TABLE_GROW_MODULE)?;
        let mut table_store = host.new_wasm_store()?;
        let table_instance = wasmtime::Instance::new(&mut table_store, &table_module, &[])?;
        let grow = table_instance.get_typed_func::<i32, i32>(&mut table_store, "grow")?;
        assert!(grow.call(&mut table_store, i32::MAX).is_err());

        let instance_module = wasmtime::Module::new(host.runtime.engine(), EMPTY_MODULE)?;
        let mut instance_store = host.new_wasm_store()?;
        let mut instances = Vec::new();
        for _ in 0..host.limits.maximum_instances {
            instances.push(wasmtime::Instance::new(
                &mut instance_store,
                &instance_module,
                &[],
            )?);
        }
        assert!(wasmtime::Instance::new(&mut instance_store, &instance_module, &[]).is_err());
        Ok(())
    }
}
