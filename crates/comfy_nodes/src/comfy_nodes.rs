pub mod descriptor;
pub mod object_info;
pub mod registry_generator;
pub mod slice_registry;

pub use descriptor::{
    CatalogNodeDescriptor, CatalogNodeSource, CatalogNodeStatus, NODE_DESCRIPTOR_SCHEMA_VERSION,
    NodeDescriptor, PortDescriptor,
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
        let registry = NodeRegistry::built_in()?;
        assert!(
            GENERATED_DESCRIPTOR_IDS
                .iter()
                .all(|identifier| registry.descriptor(identifier).is_some())
        );
        Ok(())
    }
}
