use crate::{CatalogNodeDescriptor, CatalogNodeStatus, NodeRegistry};
use serde::Serialize;
use std::collections::BTreeMap;

pub const OBJECT_INFO_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ObjectInfoInputSchema {
    pub raw: String,
    pub input_is_list: String,
    pub lazy_inputs: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ObjectInfoOutputSchema {
    pub raw: String,
    pub output_is_list: String,
    pub output_node: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ObjectInfoNode {
    pub schema_version: u16,
    pub node_identifier: String,
    pub display_name: String,
    pub category: String,
    pub schema_source: String,
    pub input: ObjectInfoInputSchema,
    pub output: ObjectInfoOutputSchema,
    pub availability: String,
    pub catalog_status: CatalogNodeStatus,
    pub feature_id: String,
}

impl From<&CatalogNodeDescriptor> for ObjectInfoNode {
    fn from(descriptor: &CatalogNodeDescriptor) -> Self {
        Self {
            schema_version: OBJECT_INFO_SCHEMA_VERSION,
            node_identifier: descriptor.node_identifier.clone(),
            display_name: descriptor.display_name.clone(),
            category: descriptor.category.clone(),
            schema_source: descriptor.schema_source.clone(),
            input: ObjectInfoInputSchema {
                raw: descriptor.inputs.clone(),
                input_is_list: descriptor.input_is_list.clone(),
                lazy_inputs: descriptor.lazy_inputs.clone(),
            },
            output: ObjectInfoOutputSchema {
                raw: descriptor.outputs.clone(),
                output_is_list: descriptor.output_is_list.clone(),
                output_node: descriptor.output_node,
            },
            availability: descriptor.availability.clone(),
            catalog_status: descriptor.catalog_status,
            feature_id: descriptor.feature_id.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ObjectInfoRegistry {
    nodes: BTreeMap<String, ObjectInfoNode>,
}

impl ObjectInfoRegistry {
    pub fn from_node_registry(registry: &NodeRegistry) -> Self {
        let nodes = registry
            .registered()
            .iter()
            .chain(registry.inactive())
            .map(|(identifier, descriptor)| (identifier.clone(), descriptor.into()))
            .collect();
        Self { nodes }
    }

    pub fn nodes(&self) -> &BTreeMap<String, ObjectInfoNode> {
        &self.nodes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn object_info_is_a_read_only_schema_projection() -> Result<(), Box<dyn Error>> {
        let registry = NodeRegistry::built_in()?;
        let object_info = ObjectInfoRegistry::from_node_registry(&registry);
        assert_eq!(object_info.nodes().len(), 801);
        let load_image = &object_info.nodes()["LoadImage"];
        assert_eq!(load_image.display_name, "Load Image");
        assert_eq!(load_image.category, "image");
        assert!(load_image.input.raw.contains("image_upload"));
        assert!(load_image.output.raw.contains("IMAGE"));
        assert_eq!(load_image.catalog_status, CatalogNodeStatus::DescriptorOnly);
        assert!(load_image.schema_source.contains("LoadImage.INPUT_TYPES"));
        let inactive = &object_info.nodes()["AutogrowNamesTestNode"];
        assert_eq!(inactive.catalog_status, CatalogNodeStatus::Inactive);
        Ok(())
    }
}
