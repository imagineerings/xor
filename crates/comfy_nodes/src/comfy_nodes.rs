pub mod descriptor;
pub mod execution;
pub mod object_info;
pub mod registry_generator;
pub mod slice_registry;

pub use descriptor::{
    CatalogNodeDescriptor, CatalogNodeSource, CatalogNodeStatus, NODE_DESCRIPTOR_SCHEMA_VERSION,
    NodeDescriptor, PortDescriptor,
};
pub use execution::{
    NATIVE_NODE_CONTRACT_SCHEMA_VERSION, NATIVE_OPAQUE_HANDLE_SCHEMA_VERSION,
    NativeCacheDependencies, NativeCachePolicy, NativeDynamicInputDescriptor, NativeEffectClass,
    NativeHandleKind, NativeHandleStore, NativeHandleStoreError, NativeHandleStoreIdentity,
    NativeHandleType, NativeInputDescriptor, NativeNode, NativeNodeBinding,
    NativeNodeBindingDisposition, NativeNodeBindingsFactory, NativeNodeContext,
    NativeNodeContractError, NativeNodeDescriptor, NativeNodeFailure, NativeNodeFailureKind,
    NativeNodeOutcome, NativeNodePresentation, NativeOpaqueHandle, NativeOutputDescriptor,
    NativePortCardinality, NativePreparedEffectRequest, NativePrimitive, NativePrimitiveType,
    NativeStoredArtifactObject, NativeStoredModelObject, NativeStoredObject,
    NativeStoredTensorObject, NativeTypeUnion, NativeValue, NativeValueType,
    validate_generated_family_bindings,
};
pub use object_info::{
    OBJECT_INFO_SCHEMA_VERSION, ObjectInfoInputSchema, ObjectInfoNode, ObjectInfoOutputSchema,
    ObjectInfoRegistry,
};
pub use registry_generator::{
    INACTIVE_NODE_CATALOG, NodeRegistry, NodeRegistryError, NodeRegistryGenerator,
    REGISTERED_NODE_CATALOG,
};
pub use slice_registry::{
    DIFFUSION_SLICE_NODE_IDS, EarlySliceRegistry, IMAGE_SLICE_NODE_IDS, SliceRegistryError,
};

include!(concat!(env!("OUT_DIR"), "/generated_modules.rs"));

pub use generated_native_diffusion::{
    NATIVE_DIFFUSION_DESCRIPTOR_SCHEMA_VERSION, native_diffusion_descriptors,
};
pub use generated_native_image::{
    NATIVE_IMAGE_DESCRIPTOR_SCHEMA_VERSION, NativeImageDescriptor, NativeImageDescriptorError,
    NativeImageEffect, NativeImagePort, native_image_descriptors,
};

#[cfg(test)]
mod generated_manifest_tests {
    use super::*;
    use std::{collections::BTreeSet, error::Error};

    #[test]
    fn generated_manifests_are_sorted_unique_and_catalog_backed() -> Result<(), Box<dyn Error>> {
        assert!(GENERATED_MODULES.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(
            GENERATED_DESCRIPTOR_IDS
                .windows(2)
                .all(|pair| pair[0] < pair[1])
        );
        assert_eq!(
            GENERATED_DESCRIPTOR_IDS.len(),
            GENERATED_DESCRIPTOR_IDS
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
        );
        assert!(
            GENERATED_FAMILY_MODULES
                .windows(2)
                .all(|pair| pair[0] < pair[1])
        );
        assert!(
            GENERATED_FAMILY_DESCRIPTOR_IDS
                .windows(2)
                .all(|pair| pair[0] < pair[1])
        );
        assert!(
            GENERATED_FAMILY_MODULES
                .iter()
                .all(|module| GENERATED_MODULES.contains(module))
        );
        assert!(
            GENERATED_FAMILY_DESCRIPTOR_IDS
                .iter()
                .all(|identifier| GENERATED_DESCRIPTOR_IDS.contains(identifier))
        );
        let family_bindings = generated_family_node_bindings()?;
        assert_eq!(family_bindings.len(), GENERATED_FAMILY_DESCRIPTOR_IDS.len());
        let registry = NodeRegistry::built_in()?;
        assert!(
            GENERATED_DESCRIPTOR_IDS
                .iter()
                .all(|identifier| registry.descriptor(identifier).is_some())
        );
        Ok(())
    }
}
