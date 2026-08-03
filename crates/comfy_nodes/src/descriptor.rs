use serde::{Deserialize, Serialize};

pub const NODE_DESCRIPTOR_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortDescriptor {
    pub name: String,
    pub type_name: String,
    pub required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeDescriptor {
    pub type_name: String,
    pub display_name: String,
    pub inputs: Vec<PortDescriptor>,
    pub outputs: Vec<PortDescriptor>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CatalogNodeSource {
    Registered,
    Inactive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CatalogNodeStatus {
    DescriptorOnly,
    ProviderRequired,
    Inactive,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogNodeDescriptor {
    pub schema_version: u16,
    pub source: CatalogNodeSource,
    pub node_identifier: String,
    pub class_name: String,
    pub display_name: String,
    pub category: String,
    pub product: String,
    pub classification: String,
    pub availability: String,
    pub evidence_level: String,
    pub confidence: String,
    pub schema_api: Option<String>,
    pub schema_source: String,
    pub inputs: String,
    pub outputs: String,
    pub input_is_list: String,
    pub output_is_list: String,
    pub lazy_inputs: String,
    pub output_node: bool,
    pub execution_function: String,
    pub validation: String,
    pub caching: String,
    pub change_detection: String,
    pub execution_blocking: String,
    pub error_behavior: String,
    pub source_file: String,
    pub source_symbol: String,
    pub source_line: Option<u32>,
    pub test_evidence: String,
    pub registration_evidence: String,
    pub inactive_reason: Option<String>,
    pub sim_status: Option<String>,
    pub parity_gap: Option<String>,
    pub feature_id: String,
    pub catalog_status: CatalogNodeStatus,
}

impl CatalogNodeDescriptor {
    pub fn is_registered(&self) -> bool {
        self.source == CatalogNodeSource::Registered
    }
}
