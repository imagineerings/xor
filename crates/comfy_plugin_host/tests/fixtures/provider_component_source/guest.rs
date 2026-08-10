wit_bindgen::generate!({
    path: "../../../../comfy_plugin_sdk/wit",
    world: "comfy-provider-plugin",
});

use exports::sim::comfy_plugin::{plugin, provider_binding};
use sim::comfy_plugin::types;

struct ProviderComponent;

impl plugin::Guest for ProviderComponent {
    fn manifest() -> types::ManifestProjection {
        types::ManifestProjection {
            component_world: "sim:comfy-provider-plugin@1.0.0".to_owned(),
            schema_version: 1,
            identifier: "test.provider-plugin".to_owned(),
            plugin_version: version(1, 0, 0),
            api: types::ApiRequirement {
                major: 1,
                minimum_minor: 0,
                maximum_minor: 0,
                required_features: vec!["provider.bindings.v1".to_owned()],
            },
            nodes: vec![types::Node {
                id: "provider.echo".to_owned(),
                version: version(1, 0, 0),
                display_name: "Provider Echo".to_owned(),
                category: "test/provider".to_owned(),
                ports: vec![types::Port {
                    id: "result".to_owned(),
                    name: "Result".to_owned(),
                    direction: types::PortDirection::Output,
                    type_id: "comfy:string@1".to_owned(),
                    cardinality: types::PortCardinality::Singular,
                    presence: types::PortPresence::Required,
                    hidden: false,
                    lazy: false,
                    default: None,
                    serialization: types::PortSerialization::Inline,
                    accepted_legacy_names: Vec::new(),
                }],
                determinism: types::DeterminismPolicy::External,
                cache: types::CachePolicy::Never,
                effects: types::EffectPolicy::Provider,
            }],
            capabilities: Vec::new(),
            ui: Vec::new(),
            routes: Vec::new(),
            legacy_mappings: Vec::new(),
        }
    }

    fn create_node(node_id: String) -> Result<u64, types::InvocationError> {
        if node_id == "provider.echo" {
            Ok(1)
        } else {
            Err(types::InvocationError::PluginFailure(
                "unknown provider fixture node".to_owned(),
            ))
        }
    }

    fn invoke(_instance: u64) -> Result<(), types::InvocationError> {
        Err(types::InvocationError::PluginFailure(
            "provider fixture requires invoke-provider".to_owned(),
        ))
    }

    fn cancel(_instance: u64, _reason: types::CancelReason) -> Result<(), types::InvocationError> {
        Ok(())
    }

    fn drop_node(_instance: u64) {}
}

impl provider_binding::Guest for ProviderComponent {
    fn binding_set() -> types::ProviderBindingSet {
        types::ProviderBindingSet {
            schema_version: 1,
            implementation_namespace: "test.provider-plugin".to_owned(),
            bindings_sha256: "f802af8cb7dd2f4f526136adbbcd1919fefbbd8c4599a5b5eacdc00f5dca309c"
                .to_owned(),
            bindings: vec![types::ProviderBindingClaim {
                feature_id: "COMFY-NODE-TEST-PROVIDER".to_owned(),
                node_id: "provider.echo".to_owned(),
                contract_sha256: "3".repeat(64),
                transport_schema: "sim:comfy-provider-transport@1".to_owned(),
                materializer_schema: "sim:comfy-provider-materializer@1".to_owned(),
            }],
        }
    }

    fn invoke_provider(
        class_type: String,
        request: Vec<u8>,
    ) -> Result<types::ProviderInvocationResponse, types::InvocationError> {
        if class_type != "provider.echo" || request.is_empty() {
            return Err(types::InvocationError::PluginFailure(
                "invalid provider request".to_owned(),
            ));
        }
        Ok(types::ProviderInvocationResponse {
            outputs: vec![types::ProviderMaterializedOutput {
                port_id: "result".to_owned(),
                value: types::EncodedValue {
                    type_id: "comfy:string@1".to_owned(),
                    family: types::ValueFamily::Scalar,
                    abi_bytes: request,
                },
            }],
            receipt: b"provider-fixture-receipt".to_vec(),
        })
    }
}

fn version(major: u16, minor: u16, patch: u16) -> types::ApiVersion {
    types::ApiVersion {
        major,
        minor,
        patch,
    }
}

export!(ProviderComponent);
