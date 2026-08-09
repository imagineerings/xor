use crate::{
    CatalogNodeDescriptor, CatalogNodeSource, CatalogNodeStatus, NODE_DESCRIPTOR_SCHEMA_VERSION,
    NativeNodeBinding, NativeNodeBindingDisposition,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

pub const REGISTERED_NODE_CATALOG: &str =
    include_str!("../../../.agents/specs/comfy-parity/catalogs/backend-nodes.csv");
pub const INACTIVE_NODE_CATALOG: &str =
    include_str!("../../../.agents/specs/comfy-parity/catalogs/backend-inactive-nodes.csv");

const MAX_CATALOG_BYTES: usize = 16 * 1024 * 1024;
const MAX_CATALOG_ROWS: usize = 100_000;
const MAX_COLUMNS: usize = 128;
const MAX_FIELD_BYTES: usize = 2 * 1024 * 1024;
const REGISTERED_HEADER: &[&str] = &[
    "node_identifier",
    "class_name",
    "display_name",
    "category",
    "product",
    "classification",
    "availability",
    "evidence_level",
    "confidence",
    "schema_api",
    "schema_source",
    "inputs",
    "outputs",
    "input_is_list",
    "output_is_list",
    "lazy_inputs",
    "output_node",
    "execution_function",
    "validation",
    "caching",
    "change_detection",
    "execution_blocking",
    "error_behavior",
    "source_file",
    "source_symbol",
    "source_line",
    "test_evidence",
    "registration_evidence",
    "feature_id",
];

const INACTIVE_HEADER: &[&str] = &[
    "node_identifier",
    "class_name",
    "display_name",
    "category",
    "classification",
    "availability",
    "evidence_level",
    "confidence",
    "reason",
    "inputs",
    "outputs",
    "source_file",
    "source_symbol",
    "source_line",
    "test_evidence",
    "sim_status",
    "parity_gap",
    "feature_id",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NodeRegistryError {
    CatalogTooLarge,
    TooManyRows,
    TooManyColumns {
        row: usize,
    },
    FieldTooLarge {
        row: usize,
        column: usize,
    },
    MalformedCsv {
        position: usize,
        reason: String,
    },
    HeaderMismatch,
    ColumnCount {
        row: usize,
        expected: usize,
        actual: usize,
    },
    InvalidBoolean {
        row: usize,
        field: &'static str,
    },
    InvalidNumber {
        row: usize,
        field: &'static str,
    },
    InvalidDescriptor {
        row: usize,
        field: &'static str,
    },
    MissingSourceProjection(String),
    DuplicateNode(String),
    DuplicateFeature(String),
    InvalidNativeBinding {
        identifier: String,
        reason: String,
    },
}

impl fmt::Display for NodeRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CatalogTooLarge => formatter.write_str("node catalog exceeds its byte limit"),
            Self::TooManyRows => formatter.write_str("node catalog exceeds its row limit"),
            Self::TooManyColumns { row } => {
                write!(formatter, "node catalog row {row} exceeds its column limit")
            }
            Self::FieldTooLarge { row, column } => write!(
                formatter,
                "node catalog field at row {row}, column {column} exceeds its byte limit"
            ),
            Self::MalformedCsv { position, reason } => {
                write!(
                    formatter,
                    "malformed node catalog CSV at {position}: {reason}"
                )
            }
            Self::HeaderMismatch => formatter.write_str("node catalog header does not match"),
            Self::ColumnCount {
                row,
                expected,
                actual,
            } => write!(
                formatter,
                "node catalog row {row} has {actual} columns, expected {expected}"
            ),
            Self::InvalidBoolean { row, field } => {
                write!(
                    formatter,
                    "node catalog row {row} has invalid boolean `{field}`"
                )
            }
            Self::InvalidNumber { row, field } => {
                write!(
                    formatter,
                    "node catalog row {row} has invalid number `{field}`"
                )
            }
            Self::InvalidDescriptor { row, field } => {
                write!(formatter, "node catalog row {row} has invalid `{field}`")
            }
            Self::MissingSourceProjection(identifier) => write!(
                formatter,
                "node catalog descriptor `{identifier}` has no checked source module projection"
            ),
            Self::DuplicateNode(identifier) => {
                write!(formatter, "duplicate node identifier `{identifier}`")
            }
            Self::DuplicateFeature(identifier) => {
                write!(formatter, "duplicate node feature ID `{identifier}`")
            }
            Self::InvalidNativeBinding { identifier, reason } => {
                write!(
                    formatter,
                    "native node binding for `{identifier}` is invalid: {reason}"
                )
            }
        }
    }
}

impl Error for NodeRegistryError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeRegistry {
    registered: BTreeMap<String, CatalogNodeDescriptor>,
    inactive: BTreeMap<String, CatalogNodeDescriptor>,
}

impl NodeRegistry {
    pub fn built_in() -> Result<Self, NodeRegistryError> {
        let registry =
            NodeRegistryGenerator::from_catalogs(REGISTERED_NODE_CATALOG, INACTIVE_NODE_CATALOG)?
                .finish();
        for identifier in crate::GENERATED_DESCRIPTOR_IDS {
            if !registry.registered.contains_key(*identifier) {
                return Err(NodeRegistryError::InvalidDescriptor {
                    row: 0,
                    field: "generated_descriptor_id",
                });
            }
        }
        Ok(registry)
    }

    pub fn registered(&self) -> &BTreeMap<String, CatalogNodeDescriptor> {
        &self.registered
    }

    pub fn inactive(&self) -> &BTreeMap<String, CatalogNodeDescriptor> {
        &self.inactive
    }

    pub fn descriptor(&self, identifier: &str) -> Option<&CatalogNodeDescriptor> {
        self.registered
            .get(identifier)
            .or_else(|| self.inactive.get(identifier))
    }

    pub fn source_python_module(&self, identifier: &str) -> Option<String> {
        self.descriptor(identifier)
            .and_then(|descriptor| source_python_module(&descriptor.source_file))
    }

    pub fn validate_native_binding(
        &self,
        binding: &NativeNodeBinding,
    ) -> Result<(), NodeRegistryError> {
        let identifier = binding.descriptor().class_type.clone();
        binding
            .validate()
            .map_err(|error| NodeRegistryError::InvalidNativeBinding {
                identifier: identifier.clone(),
                reason: error.to_string(),
            })?;
        let catalog = self.descriptor(&identifier).ok_or_else(|| {
            NodeRegistryError::InvalidNativeBinding {
                identifier: identifier.clone(),
                reason: "catalog descriptor is absent".to_owned(),
            }
        })?;
        let expected_category = match catalog.category.as_str() {
            "(empty root category declared by source)" => "",
            category => category,
        };
        let expected_disposition = match (catalog.source, catalog.catalog_status) {
            (CatalogNodeSource::Inactive, _) => NativeNodeBindingDisposition::Unavailable,
            (CatalogNodeSource::Registered, CatalogNodeStatus::ProviderRequired) => {
                NativeNodeBindingDisposition::ProviderRequired
            }
            (
                CatalogNodeSource::Registered,
                CatalogNodeStatus::DescriptorOnly | CatalogNodeStatus::Inactive,
            ) => NativeNodeBindingDisposition::Executable,
        };
        let mismatch = if binding.feature_id() != catalog.feature_id {
            Some("feature_id")
        } else if binding.presentation().display_name != catalog.display_name {
            Some("display_name")
        } else if binding.presentation().category != expected_category {
            Some("category")
        } else if binding.descriptor().output_node != catalog.output_node {
            Some("output_node")
        } else if binding.disposition() != expected_disposition {
            Some("disposition")
        } else {
            None
        };
        if let Some(field) = mismatch {
            return Err(NodeRegistryError::InvalidNativeBinding {
                identifier,
                reason: format!("catalog field `{field}` does not match"),
            });
        }
        Ok(())
    }
}

pub struct NodeRegistryGenerator {
    registered: BTreeMap<String, CatalogNodeDescriptor>,
    inactive: BTreeMap<String, CatalogNodeDescriptor>,
    feature_ids: BTreeSet<String>,
}

impl NodeRegistryGenerator {
    pub fn from_catalogs(
        registered_catalog: &str,
        inactive_catalog: &str,
    ) -> Result<Self, NodeRegistryError> {
        let registered_rows = parse_csv(registered_catalog)?;
        validate_header(&registered_rows, REGISTERED_HEADER)?;
        let inactive_rows = parse_csv(inactive_catalog)?;
        validate_header(&inactive_rows, INACTIVE_HEADER)?;
        let mut generator = Self {
            registered: BTreeMap::new(),
            inactive: BTreeMap::new(),
            feature_ids: BTreeSet::new(),
        };
        for (row_index, row) in registered_rows.iter().enumerate().skip(1) {
            generator.insert_catalog_descriptor(parse_registered_row(row_index + 1, row)?)?;
        }
        for (row_index, row) in inactive_rows.iter().enumerate().skip(1) {
            generator.insert_catalog_descriptor(parse_inactive_row(row_index + 1, row)?)?;
        }
        Ok(generator)
    }

    pub fn finish(self) -> NodeRegistry {
        NodeRegistry {
            registered: self.registered,
            inactive: self.inactive,
        }
    }

    fn insert_catalog_descriptor(
        &mut self,
        descriptor: CatalogNodeDescriptor,
    ) -> Result<(), NodeRegistryError> {
        if self.registered.contains_key(&descriptor.node_identifier)
            || self.inactive.contains_key(&descriptor.node_identifier)
        {
            return Err(NodeRegistryError::DuplicateNode(descriptor.node_identifier));
        }
        if !self.feature_ids.insert(descriptor.feature_id.clone()) {
            return Err(NodeRegistryError::DuplicateFeature(descriptor.feature_id));
        }
        match descriptor.source {
            CatalogNodeSource::Registered => {
                self.registered
                    .insert(descriptor.node_identifier.clone(), descriptor);
            }
            CatalogNodeSource::Inactive => {
                self.inactive
                    .insert(descriptor.node_identifier.clone(), descriptor);
            }
        }
        Ok(())
    }
}

fn parse_registered_row(
    row_number: usize,
    row: &[String],
) -> Result<CatalogNodeDescriptor, NodeRegistryError> {
    validate_column_count(row_number, row, REGISTERED_HEADER.len())?;
    validate_common(row_number, row, 28, "COMFY-NODE-")?;
    validate_source_file(row_number, &row[23])?;
    Ok(CatalogNodeDescriptor {
        schema_version: NODE_DESCRIPTOR_SCHEMA_VERSION,
        source: CatalogNodeSource::Registered,
        node_identifier: row[0].clone(),
        class_name: row[1].clone(),
        display_name: row[2].clone(),
        category: row[3].clone(),
        product: row[4].clone(),
        classification: row[5].clone(),
        availability: row[6].clone(),
        evidence_level: row[7].clone(),
        confidence: row[8].clone(),
        schema_api: Some(row[9].clone()),
        schema_source: row[10].clone(),
        inputs: row[11].clone(),
        outputs: row[12].clone(),
        input_is_list: row[13].clone(),
        output_is_list: row[14].clone(),
        lazy_inputs: row[15].clone(),
        output_node: parse_bool(row_number, "output_node", &row[16])?,
        execution_function: row[17].clone(),
        validation: row[18].clone(),
        caching: row[19].clone(),
        change_detection: row[20].clone(),
        execution_blocking: row[21].clone(),
        error_behavior: row[22].clone(),
        source_file: row[23].clone(),
        source_symbol: row[24].clone(),
        source_line: parse_optional_u32(row_number, "source_line", &row[25])?,
        test_evidence: row[26].clone(),
        registration_evidence: row[27].clone(),
        inactive_reason: None,
        sim_status: None,
        parity_gap: None,
        feature_id: row[28].clone(),
        catalog_status: status_for_availability(&row[6], false),
    })
}

fn parse_inactive_row(
    row_number: usize,
    row: &[String],
) -> Result<CatalogNodeDescriptor, NodeRegistryError> {
    validate_column_count(row_number, row, INACTIVE_HEADER.len())?;
    validate_common(row_number, row, 17, "COMFY-NODE-INACTIVE-")?;
    validate_source_file(row_number, &row[11])?;
    Ok(CatalogNodeDescriptor {
        schema_version: NODE_DESCRIPTOR_SCHEMA_VERSION,
        source: CatalogNodeSource::Inactive,
        node_identifier: row[0].clone(),
        class_name: row[1].clone(),
        display_name: row[2].clone(),
        category: row[3].clone(),
        product: "ComfyUI".to_owned(),
        classification: row[4].clone(),
        availability: row[5].clone(),
        evidence_level: row[6].clone(),
        confidence: row[7].clone(),
        schema_api: None,
        schema_source: String::new(),
        inputs: row[9].clone(),
        outputs: row[10].clone(),
        input_is_list: String::new(),
        output_is_list: String::new(),
        lazy_inputs: String::new(),
        output_node: false,
        execution_function: String::new(),
        validation: String::new(),
        caching: String::new(),
        change_detection: String::new(),
        execution_blocking: String::new(),
        error_behavior: String::new(),
        source_file: row[11].clone(),
        source_symbol: row[12].clone(),
        source_line: parse_optional_u32(row_number, "source_line", &row[13])?,
        test_evidence: row[14].clone(),
        registration_evidence: String::new(),
        inactive_reason: Some(row[8].clone()),
        sim_status: Some(row[15].clone()),
        parity_gap: Some(row[16].clone()),
        feature_id: row[17].clone(),
        catalog_status: status_for_availability(&row[5], true),
    })
}

fn validate_common(
    row_number: usize,
    row: &[String],
    feature_index: usize,
    feature_prefix: &str,
) -> Result<(), NodeRegistryError> {
    for (index, field) in [
        (0, "node_identifier"),
        (1, "class_name"),
        (2, "display_name"),
        (3, "category"),
    ] {
        if row[index].is_empty() {
            return Err(NodeRegistryError::InvalidDescriptor {
                row: row_number,
                field,
            });
        }
    }
    if !valid_catalog_identifier(&row[0]) {
        return Err(NodeRegistryError::InvalidDescriptor {
            row: row_number,
            field: "node_identifier",
        });
    }
    if !row[feature_index].starts_with(feature_prefix) {
        return Err(NodeRegistryError::InvalidDescriptor {
            row: row_number,
            field: "feature_id",
        });
    }
    Ok(())
}

fn status_for_availability(availability: &str, inactive: bool) -> CatalogNodeStatus {
    if inactive || availability == "deprecated/dead" {
        CatalogNodeStatus::Inactive
    } else if availability == "cloud/paid" {
        CatalogNodeStatus::ProviderRequired
    } else {
        CatalogNodeStatus::DescriptorOnly
    }
}

fn parse_bool(row: usize, field: &'static str, value: &str) -> Result<bool, NodeRegistryError> {
    match value {
        "True" => Ok(true),
        "False" => Ok(false),
        _ => Err(NodeRegistryError::InvalidBoolean { row, field }),
    }
}

fn parse_optional_u32(
    row: usize,
    field: &'static str,
    value: &str,
) -> Result<Option<u32>, NodeRegistryError> {
    if value.is_empty() {
        Ok(None)
    } else {
        value
            .parse()
            .map(Some)
            .map_err(|_| NodeRegistryError::InvalidNumber { row, field })
    }
}

fn valid_catalog_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 4_096
        && value.chars().all(|character| !character.is_control())
}

fn validate_source_file(row: usize, value: &str) -> Result<(), NodeRegistryError> {
    if source_python_module(value).is_some() {
        Ok(())
    } else {
        Err(NodeRegistryError::InvalidDescriptor {
            row,
            field: "source_file",
        })
    }
}

fn source_python_module(source_file: &str) -> Option<String> {
    if source_file == "nodes.py" {
        return Some("nodes".to_owned());
    }
    let (parent, filename) = source_file.split_once('/')?;
    if !matches!(parent, "comfy_extras" | "comfy_api_nodes")
        || filename.contains('/')
        || !filename.ends_with(".py")
    {
        return None;
    }
    let stem = filename.strip_suffix(".py")?;
    if stem.is_empty()
        || stem.len() > 4_096
        || !stem
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return None;
    }
    Some(format!("{parent}.{stem}"))
}

fn validate_header(rows: &[Vec<String>], expected: &[&str]) -> Result<(), NodeRegistryError> {
    let Some(header) = rows.first() else {
        return Err(NodeRegistryError::HeaderMismatch);
    };
    if header.len() != expected.len()
        || header
            .iter()
            .zip(expected)
            .any(|(actual, expected)| actual != expected)
    {
        return Err(NodeRegistryError::HeaderMismatch);
    }
    Ok(())
}

fn validate_column_count(
    row_number: usize,
    row: &[String],
    expected: usize,
) -> Result<(), NodeRegistryError> {
    if row.len() == expected {
        Ok(())
    } else {
        Err(NodeRegistryError::ColumnCount {
            row: row_number,
            expected,
            actual: row.len(),
        })
    }
}

fn parse_csv(input: &str) -> Result<Vec<Vec<String>>, NodeRegistryError> {
    if input.len() > MAX_CATALOG_BYTES {
        return Err(NodeRegistryError::CatalogTooLarge);
    }
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut characters = input.char_indices().peekable();
    let mut in_quotes = false;
    let mut quote_closed = false;
    while let Some((position, character)) = characters.next() {
        if in_quotes {
            if character == '"' {
                if characters.peek().is_some_and(|(_, next)| *next == '"') {
                    characters.next();
                    field.push('"');
                } else {
                    in_quotes = false;
                    quote_closed = true;
                }
            } else {
                field.push(character);
            }
        } else {
            match character {
                '"' if field.is_empty() && !quote_closed => in_quotes = true,
                '"' => {
                    return Err(NodeRegistryError::MalformedCsv {
                        position,
                        reason: "quote appeared inside an unquoted field".to_owned(),
                    });
                }
                ',' => {
                    push_field(&mut row, &mut field)?;
                    quote_closed = false;
                }
                '\n' | '\r' => {
                    if character == '\r' && characters.peek().is_some_and(|(_, next)| *next == '\n')
                    {
                        characters.next();
                    }
                    push_field(&mut row, &mut field)?;
                    push_row(&mut rows, &mut row)?;
                    quote_closed = false;
                }
                _ if quote_closed => {
                    return Err(NodeRegistryError::MalformedCsv {
                        position,
                        reason: "characters followed a closing quote".to_owned(),
                    });
                }
                _ => field.push(character),
            }
        }
        if field.len() > MAX_FIELD_BYTES {
            return Err(NodeRegistryError::FieldTooLarge {
                row: rows.len() + 1,
                column: row.len() + 1,
            });
        }
    }
    if in_quotes {
        return Err(NodeRegistryError::MalformedCsv {
            position: input.len(),
            reason: "quoted field was not closed".to_owned(),
        });
    }
    if !field.is_empty() || !row.is_empty() || quote_closed {
        push_field(&mut row, &mut field)?;
        push_row(&mut rows, &mut row)?;
    }
    Ok(rows)
}

fn push_field(row: &mut Vec<String>, field: &mut String) -> Result<(), NodeRegistryError> {
    if row.len() >= MAX_COLUMNS {
        return Err(NodeRegistryError::TooManyColumns { row: 0 });
    }
    row.push(std::mem::take(field));
    Ok(())
}

fn push_row(rows: &mut Vec<Vec<String>>, row: &mut Vec<String>) -> Result<(), NodeRegistryError> {
    if rows.len() >= MAX_CATALOG_ROWS {
        return Err(NodeRegistryError::TooManyRows);
    }
    rows.push(std::mem::take(row));
    Ok(())
}

#[cfg(test)]
fn canonical_csv(rows: &[Vec<String>]) -> String {
    let mut output = String::new();
    for row in rows {
        for (index, field) in row.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            if field
                .chars()
                .any(|character| matches!(character, ',' | '"' | '\n' | '\r'))
            {
                output.push('"');
                for character in field.chars() {
                    if character == '"' {
                        output.push('"');
                    }
                    output.push(character);
                }
                output.push('"');
            } else {
                output.push_str(field);
            }
        }
        output.push('\n');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CatalogNodeStatus, DIFFUSION_SLICE_NODE_IDS, EarlySliceRegistry, IMAGE_SLICE_NODE_IDS,
        NATIVE_NODE_CONTRACT_SCHEMA_VERSION, NativeCachePolicy, NativeEffectClass,
        NativeHandleKind, NativeHandleType, NativeInputDescriptor, NativeNode, NativeNodeContext,
        NativeNodeDescriptor, NativeNodeFailure, NativeNodeOutcome, NativeNodePresentation,
        NativeOutputDescriptor, NativePortCardinality, NativeTypeUnion, NativeValue,
        NativeValueType, ObjectInfoRegistry,
    };
    use futures::future::BoxFuture;
    use serde::Serialize;
    use std::{fs, path::Path};

    fn registered_catalog_row(descriptor: &CatalogNodeDescriptor) -> Vec<String> {
        vec![
            descriptor.node_identifier.clone(),
            descriptor.class_name.clone(),
            descriptor.display_name.clone(),
            descriptor.category.clone(),
            descriptor.product.clone(),
            descriptor.classification.clone(),
            descriptor.availability.clone(),
            descriptor.evidence_level.clone(),
            descriptor.confidence.clone(),
            descriptor.schema_api.clone().unwrap_or_default(),
            descriptor.schema_source.clone(),
            descriptor.inputs.clone(),
            descriptor.outputs.clone(),
            descriptor.input_is_list.clone(),
            descriptor.output_is_list.clone(),
            descriptor.lazy_inputs.clone(),
            if descriptor.output_node {
                "True".to_owned()
            } else {
                "False".to_owned()
            },
            descriptor.execution_function.clone(),
            descriptor.validation.clone(),
            descriptor.caching.clone(),
            descriptor.change_detection.clone(),
            descriptor.execution_blocking.clone(),
            descriptor.error_behavior.clone(),
            descriptor.source_file.clone(),
            descriptor.source_symbol.clone(),
            descriptor
                .source_line
                .map_or_else(String::new, |line| line.to_string()),
            descriptor.test_evidence.clone(),
            descriptor.registration_evidence.clone(),
            descriptor.feature_id.clone(),
        ]
    }

    fn inactive_catalog_row(descriptor: &CatalogNodeDescriptor) -> Vec<String> {
        vec![
            descriptor.node_identifier.clone(),
            descriptor.class_name.clone(),
            descriptor.display_name.clone(),
            descriptor.category.clone(),
            descriptor.classification.clone(),
            descriptor.availability.clone(),
            descriptor.evidence_level.clone(),
            descriptor.confidence.clone(),
            descriptor.inactive_reason.clone().unwrap_or_default(),
            descriptor.inputs.clone(),
            descriptor.outputs.clone(),
            descriptor.source_file.clone(),
            descriptor.source_symbol.clone(),
            descriptor
                .source_line
                .map_or_else(String::new, |line| line.to_string()),
            descriptor.test_evidence.clone(),
            descriptor.sim_status.clone().unwrap_or_default(),
            descriptor.parity_gap.clone().unwrap_or_default(),
            descriptor.feature_id.clone(),
        ]
    }

    fn reconstruct_catalogs(
        registry: &NodeRegistry,
        registered_rows: &[Vec<String>],
        inactive_rows: &[Vec<String>],
    ) -> Result<(String, String), Box<dyn Error>> {
        let mut reconstructed_registered = vec![
            REGISTERED_HEADER
                .iter()
                .map(|field| (*field).to_owned())
                .collect::<Vec<_>>(),
        ];
        for row in registered_rows.iter().skip(1) {
            let identifier = row.first().ok_or("registered row has no identifier")?;
            let descriptor = registry
                .registered()
                .get(identifier)
                .ok_or("registered descriptor is absent")?;
            let reconstructed = registered_catalog_row(descriptor);
            assert_eq!(&reconstructed, row);
            let encoded = serde_json::to_vec(descriptor)?;
            assert_eq!(
                serde_json::from_slice::<CatalogNodeDescriptor>(&encoded)?,
                *descriptor
            );
            reconstructed_registered.push(reconstructed);
        }

        let mut reconstructed_inactive = vec![
            INACTIVE_HEADER
                .iter()
                .map(|field| (*field).to_owned())
                .collect::<Vec<_>>(),
        ];
        for row in inactive_rows.iter().skip(1) {
            let identifier = row.first().ok_or("inactive row has no identifier")?;
            let descriptor = registry
                .inactive()
                .get(identifier)
                .ok_or("inactive descriptor is absent")?;
            let reconstructed = inactive_catalog_row(descriptor);
            assert_eq!(&reconstructed, row);
            let encoded = serde_json::to_vec(descriptor)?;
            assert_eq!(
                serde_json::from_slice::<CatalogNodeDescriptor>(&encoded)?,
                *descriptor
            );
            reconstructed_inactive.push(reconstructed);
        }
        Ok((
            canonical_csv(&reconstructed_registered),
            canonical_csv(&reconstructed_inactive),
        ))
    }

    const REGISTERED_CATALOG_SHA256: &str =
        "2fd562212e8f79335b619ff0fd1844d263a05fc85839366072046173129aefe1";
    const INACTIVE_CATALOG_SHA256: &str =
        "35e53171a424c0404351abffae4d36540872bc4ddfaf1a4b29e56cf5638c86aa";
    const GENERATED_DESCRIPTOR_IDS_SHA256: &str =
        "5c6a488a452a1e1e1d002cd0c1e7005f296030b857dedc7bb394cedb677ddbc4";

    #[derive(Serialize)]
    struct ValidationEnvironment<'a> {
        operating_system: &'a str,
        architecture: &'a str,
        registry_role: &'a str,
    }

    #[derive(Serialize)]
    struct ValidationCases {
        catalog_round_trip: bool,
        catalog_status_is_read_only: bool,
        generated_descriptor_membership_is_exact: bool,
        early_slice_membership_is_exact: bool,
        object_info_is_read_only_projection: bool,
        mutable_plugin_and_execution_apis_are_absent: bool,
    }

    #[derive(Serialize)]
    struct ValidationArtifact<'a> {
        validation_id: &'a str,
        registered_rows: usize,
        inactive_rows: usize,
        registered_catalog_sha256: &'a str,
        inactive_catalog_sha256: &'a str,
        generated_descriptor_ids_sha256: &'a str,
        generated_descriptor_ids: &'a [&'a str],
        image_slice: &'a [&'a str],
        diffusion_slice: &'a [&'a str],
        environment: ValidationEnvironment<'a>,
        cases: ValidationCases,
        passed: usize,
        failed: usize,
        skipped: usize,
    }

    fn write_validation_artifact(artifact: &ValidationArtifact<'_>) -> Result<(), Box<dyn Error>> {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .ok_or("workspace root is unavailable")?;
        let target_directory = match std::env::var_os("CARGO_TARGET_DIR") {
            Some(directory) => {
                let directory = std::path::PathBuf::from(directory);
                if directory.is_absolute() {
                    directory
                } else {
                    workspace_root.join(directory)
                }
            }
            None => workspace_root.join("target"),
        };
        let artifact_directory = target_directory.join("comfy-parity");
        fs::create_dir_all(&artifact_directory)?;
        let mut bytes = serde_json::to_vec_pretty(artifact)?;
        bytes.push(b'\n');
        fs::write(artifact_directory.join("val-node-registry-001.json"), bytes)?;
        Ok(())
    }

    #[test]
    fn checked_in_catalogs_round_trip_without_schema_loss() -> Result<(), Box<dyn Error>> {
        let registered = parse_csv(REGISTERED_NODE_CATALOG)?;
        let inactive = parse_csv(INACTIVE_NODE_CATALOG)?;
        assert_eq!(registered.len(), 790);
        assert_eq!(inactive.len(), 13);
        let registry = NodeRegistry::built_in()?;
        let (reconstructed_registered, reconstructed_inactive) =
            reconstruct_catalogs(&registry, &registered, &inactive)?;
        assert_eq!(reconstructed_registered, REGISTERED_NODE_CATALOG);
        assert_eq!(reconstructed_inactive, INACTIVE_NODE_CATALOG);
        Ok(())
    }

    #[test]
    fn registry_preserves_every_row_without_owning_runtime_executability()
    -> Result<(), Box<dyn Error>> {
        let registry = NodeRegistry::built_in()?;
        assert_eq!(registry.registered().len(), 789);
        assert_eq!(registry.inactive().len(), 12);
        assert_eq!(
            registry.registered()["LoadImage"].feature_id,
            "COMFY-NODE-0339"
        );
        assert_eq!(
            registry.registered()["LoadImage"].catalog_status,
            CatalogNodeStatus::DescriptorOnly
        );
        assert_eq!(registry.registered()["SaveImage"].output_node, true);
        assert!(
            registry
                .inactive()
                .values()
                .all(|descriptor| descriptor.catalog_status == CatalogNodeStatus::Inactive)
        );
        let provider_required = registry
            .registered()
            .values()
            .filter(|descriptor| descriptor.catalog_status == CatalogNodeStatus::ProviderRequired)
            .count();
        assert_eq!(provider_required, 214);
        Ok(())
    }

    #[test]
    fn malformed_or_duplicate_catalogs_are_rejected() -> Result<(), Box<dyn Error>> {
        assert!(matches!(
            parse_csv("a,b\n\"unterminated"),
            Err(NodeRegistryError::MalformedCsv { .. })
        ));
        let header = REGISTERED_NODE_CATALOG
            .lines()
            .next()
            .ok_or("registered catalog header is missing")?;
        let first = REGISTERED_NODE_CATALOG
            .lines()
            .nth(1)
            .ok_or("registered catalog row is missing")?;
        let duplicated = format!("{header}\n{first}\n{first}\n");
        assert!(matches!(
            NodeRegistryGenerator::from_catalogs(&duplicated, INACTIVE_NODE_CATALOG),
            Err(NodeRegistryError::DuplicateNode(_)) | Err(NodeRegistryError::DuplicateFeature(_))
        ));
        let mut registered = parse_csv(REGISTERED_NODE_CATALOG)?;
        for invalid_source in [
            "foreign/node.py",
            "comfy_extras/nested/node.py",
            "comfy_extras/node",
            "comfy_extras/.py",
            "comfy_extras/not-valid.py",
        ] {
            registered[1][23] = invalid_source.to_owned();
            let invalid_catalog = canonical_csv(&registered);
            assert!(matches!(
                NodeRegistryGenerator::from_catalogs(&invalid_catalog, INACTIVE_NODE_CATALOG),
                Err(NodeRegistryError::InvalidDescriptor {
                    field: "source_file",
                    ..
                })
            ));
        }
        let mut inactive = parse_csv(INACTIVE_NODE_CATALOG)?;
        inactive[1][11] = "comfy_api_nodes/nested/node.py".to_owned();
        let invalid_inactive_catalog = canonical_csv(&inactive);
        assert!(matches!(
            NodeRegistryGenerator::from_catalogs(
                REGISTERED_NODE_CATALOG,
                &invalid_inactive_catalog
            ),
            Err(NodeRegistryError::InvalidDescriptor {
                row: 2,
                field: "source_file",
            })
        ));
        Ok(())
    }

    struct CatalogTestNode(&'static str);

    impl NativeNode for CatalogTestNode {
        fn class_type(&self) -> &str {
            self.0
        }

        fn implementation_version(&self) -> &str {
            "1"
        }

        fn execute<'a>(
            &'a self,
            _context: NativeNodeContext,
            _inputs: BTreeMap<String, NativeValue>,
        ) -> BoxFuture<'a, Result<NativeNodeOutcome, NativeNodeFailure>> {
            Box::pin(async {
                Ok(NativeNodeOutcome::Blocked {
                    reason: "not executed by this metadata test".to_owned(),
                })
            })
        }
    }

    #[test]
    fn native_bindings_are_checked_against_atomic_catalog_presentation()
    -> Result<(), Box<dyn Error>> {
        let descriptor = NativeNodeDescriptor {
            schema_version: NATIVE_NODE_CONTRACT_SCHEMA_VERSION,
            class_type: "wanBlockSwap".to_owned(),
            implementation_version: "1".to_owned(),
            inputs: vec![NativeInputDescriptor {
                name: "model".to_owned(),
                accepted_types: NativeTypeUnion::new([NativeValueType::Handle(
                    NativeHandleType::new(NativeHandleKind::Model, "MODEL")?,
                )])?,
                required: true,
                hidden: false,
                lazy: false,
                cardinality: NativePortCardinality::Scalar,
                allows_literal: false,
            }],
            dynamic_inputs: Vec::new(),
            outputs: vec![NativeOutputDescriptor {
                name: "model".to_owned(),
                produced_type: NativeValueType::Handle(NativeHandleType::new(
                    NativeHandleKind::Model,
                    "MODEL",
                )?),
                is_list: false,
            }],
            output_node: false,
            effect: NativeEffectClass::Pure,
            cache: NativeCachePolicy::InputIdentity,
        };
        let presentation = NativeNodePresentation {
            display_name: "wanBlockSwap".to_owned(),
            category: String::new(),
            description: "Intercepts an unstable custom node.".to_owned(),
            output_names: vec!["model".to_owned()],
            search_aliases: Vec::new(),
            is_deprecated: true,
            is_experimental: false,
        };
        let binding = NativeNodeBinding::Executable {
            feature_id: "COMFY-NODE-0757".to_owned(),
            descriptor: descriptor.clone(),
            presentation: presentation.clone(),
            node: std::sync::Arc::new(CatalogTestNode("wanBlockSwap")),
        };
        let registry = NodeRegistry::built_in()?;
        registry.validate_native_binding(&binding)?;

        let mismatched = NativeNodeBinding::Unavailable {
            feature_id: "COMFY-NODE-0757".to_owned(),
            descriptor,
            presentation,
            reason: "unavailable for this test".to_owned(),
        };
        assert!(matches!(
            registry.validate_native_binding(&mismatched),
            Err(NodeRegistryError::InvalidNativeBinding { reason, .. })
                if reason.contains("disposition")
        ));

        let image_type = NativeHandleType::new(NativeHandleKind::Image, "IMAGE")?;
        let descriptor_only_descriptor = NativeNodeDescriptor {
            schema_version: NATIVE_NODE_CONTRACT_SCHEMA_VERSION,
            class_type: "ImageInvert".to_owned(),
            implementation_version: "1".to_owned(),
            inputs: vec![NativeInputDescriptor {
                name: "image".to_owned(),
                accepted_types: NativeTypeUnion::new([NativeValueType::Handle(
                    image_type.clone(),
                )])?,
                required: true,
                hidden: false,
                lazy: false,
                cardinality: NativePortCardinality::Scalar,
                allows_literal: false,
            }],
            dynamic_inputs: Vec::new(),
            outputs: vec![NativeOutputDescriptor {
                name: "image".to_owned(),
                produced_type: NativeValueType::Handle(image_type),
                is_list: false,
            }],
            output_node: false,
            effect: NativeEffectClass::Pure,
            cache: NativeCachePolicy::InputIdentity,
        };
        let descriptor_only_presentation = NativeNodePresentation {
            display_name: "Invert Image Colors".to_owned(),
            category: "image/color".to_owned(),
            description: String::new(),
            output_names: vec!["image".to_owned()],
            search_aliases: Vec::new(),
            is_deprecated: false,
            is_experimental: false,
        };
        registry.validate_native_binding(&NativeNodeBinding::Executable {
            feature_id: "COMFY-NODE-0254".to_owned(),
            descriptor: descriptor_only_descriptor.clone(),
            presentation: descriptor_only_presentation.clone(),
            node: std::sync::Arc::new(CatalogTestNode("ImageInvert")),
        })?;
        assert!(matches!(
            registry.validate_native_binding(&NativeNodeBinding::Unavailable {
                feature_id: "COMFY-NODE-0254".to_owned(),
                descriptor: descriptor_only_descriptor,
                presentation: descriptor_only_presentation,
                reason: "wrong disposition".to_owned(),
            }),
            Err(NodeRegistryError::InvalidNativeBinding { reason, .. })
                if reason.contains("disposition")
        ));

        let provider_descriptor = NativeNodeDescriptor {
            schema_version: NATIVE_NODE_CONTRACT_SCHEMA_VERSION,
            class_type: "BeebleSwitchXImageEdit".to_owned(),
            implementation_version: "1".to_owned(),
            inputs: Vec::new(),
            dynamic_inputs: Vec::new(),
            outputs: vec![
                NativeOutputDescriptor {
                    name: "image".to_owned(),
                    produced_type: NativeValueType::Handle(NativeHandleType::new(
                        NativeHandleKind::Image,
                        "IMAGE",
                    )?),
                    is_list: false,
                },
                NativeOutputDescriptor {
                    name: "alpha".to_owned(),
                    produced_type: NativeValueType::Handle(NativeHandleType::new(
                        NativeHandleKind::Mask,
                        "MASK",
                    )?),
                    is_list: false,
                },
            ],
            output_node: false,
            effect: NativeEffectClass::Provider,
            cache: NativeCachePolicy::Never,
        };
        let provider_presentation = NativeNodePresentation {
            display_name: "Beeble SwitchX Image Edit".to_owned(),
            category: "partner/image/Beeble".to_owned(),
            description: String::new(),
            output_names: vec!["image".to_owned(), "alpha".to_owned()],
            search_aliases: Vec::new(),
            is_deprecated: false,
            is_experimental: false,
        };
        registry.validate_native_binding(&NativeNodeBinding::ProviderRequired {
            feature_id: "COMFY-NODE-0020".to_owned(),
            descriptor: provider_descriptor.clone(),
            presentation: provider_presentation.clone(),
            provider: "comfy-api".to_owned(),
            reason: "cloud provider authorization is required".to_owned(),
        })?;
        assert!(matches!(
            registry.validate_native_binding(&NativeNodeBinding::Unavailable {
                feature_id: "COMFY-NODE-0020".to_owned(),
                descriptor: provider_descriptor,
                presentation: provider_presentation,
                reason: "wrong disposition".to_owned(),
            }),
            Err(NodeRegistryError::InvalidNativeBinding { reason, .. })
                if reason.contains("disposition")
        ));

        let inactive_descriptor = NativeNodeDescriptor {
            schema_version: NATIVE_NODE_CONTRACT_SCHEMA_VERSION,
            class_type: "AutogrowNamesTestNode".to_owned(),
            implementation_version: "1".to_owned(),
            inputs: Vec::new(),
            dynamic_inputs: Vec::new(),
            outputs: vec![NativeOutputDescriptor {
                name: "string".to_owned(),
                produced_type: NativeValueType::Primitive(crate::NativePrimitiveType::String),
                is_list: false,
            }],
            output_node: false,
            effect: NativeEffectClass::Pure,
            cache: NativeCachePolicy::Never,
        };
        let inactive_presentation = NativeNodePresentation {
            display_name: "AutogrowNamesTest".to_owned(),
            category: "utilities/logic".to_owned(),
            description: String::new(),
            output_names: vec!["string".to_owned()],
            search_aliases: Vec::new(),
            is_deprecated: true,
            is_experimental: false,
        };
        registry.validate_native_binding(&NativeNodeBinding::Unavailable {
            feature_id: "COMFY-NODE-INACTIVE-0001".to_owned(),
            descriptor: inactive_descriptor.clone(),
            presentation: inactive_presentation.clone(),
            reason: "source node is inactive".to_owned(),
        })?;
        assert!(matches!(
            registry.validate_native_binding(&NativeNodeBinding::ProviderRequired {
                feature_id: "COMFY-NODE-INACTIVE-0001".to_owned(),
                descriptor: inactive_descriptor,
                presentation: inactive_presentation,
                provider: "invalid-provider".to_owned(),
                reason: "wrong disposition".to_owned(),
            }),
            Err(NodeRegistryError::InvalidNativeBinding { reason, .. })
                if reason.contains("disposition")
        ));
        Ok(())
    }

    #[test]
    fn val_node_registry_001() -> Result<(), Box<dyn Error>> {
        let registered_rows = parse_csv(REGISTERED_NODE_CATALOG)?;
        let inactive_rows = parse_csv(INACTIVE_NODE_CATALOG)?;
        let registry = NodeRegistry::built_in()?;
        let (reconstructed_registered, reconstructed_inactive) =
            reconstruct_catalogs(&registry, &registered_rows, &inactive_rows)?;
        let catalog_round_trip = registered_rows.len() == 790
            && inactive_rows.len() == 13
            && reconstructed_registered == REGISTERED_NODE_CATALOG
            && reconstructed_inactive == INACTIVE_NODE_CATALOG;
        assert!(catalog_round_trip);

        let catalog_status_is_read_only = registry.registered().values().all(|descriptor| {
            matches!(
                descriptor.catalog_status,
                CatalogNodeStatus::DescriptorOnly
                    | CatalogNodeStatus::ProviderRequired
                    | CatalogNodeStatus::Inactive
            )
        }) && registry
            .inactive()
            .values()
            .all(|descriptor| descriptor.catalog_status == CatalogNodeStatus::Inactive);
        assert!(catalog_status_is_read_only);

        let generated_slice_modules = crate::GENERATED_MODULES
            .iter()
            .copied()
            .filter(|module| module.starts_with("slices/"))
            .collect::<Vec<_>>();
        let generated_family_modules = crate::GENERATED_MODULES
            .iter()
            .copied()
            .filter(|module| module.starts_with("families/"))
            .collect::<Vec<_>>();
        let early_descriptor_ids = IMAGE_SLICE_NODE_IDS
            .iter()
            .chain(DIFFUSION_SLICE_NODE_IDS)
            .copied()
            .collect::<BTreeSet<_>>();
        let family_descriptor_ids = crate::GENERATED_FAMILY_DESCRIPTOR_IDS
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let generated_descriptor_ids = crate::GENERATED_DESCRIPTOR_IDS
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let generated_descriptor_membership_is_exact = generated_slice_modules
            == ["slices/native_diffusion", "slices/native_image"]
            && generated_family_modules == crate::GENERATED_FAMILY_MODULES
            && crate::GENERATED_MODULES
                .windows(2)
                .all(|pair| pair[0] < pair[1])
            && crate::GENERATED_DESCRIPTOR_IDS
                .windows(2)
                .all(|pair| pair[0] < pair[1])
            && crate::GENERATED_FAMILY_DESCRIPTOR_IDS
                .windows(2)
                .all(|pair| pair[0] < pair[1])
            && early_descriptor_ids.is_disjoint(&family_descriptor_ids)
            && generated_descriptor_ids
                == early_descriptor_ids
                    .union(&family_descriptor_ids)
                    .copied()
                    .collect()
            && crate::GENERATED_DESCRIPTOR_IDS
                .iter()
                .all(|identifier| registry.registered().contains_key(*identifier));
        assert!(generated_descriptor_membership_is_exact);

        let slices = EarlySliceRegistry::from_node_registry(&registry)?;
        let early_slice_membership_is_exact = slices.image()
            == IMAGE_SLICE_NODE_IDS
                .iter()
                .map(|identifier| (*identifier).to_owned())
                .collect::<Vec<_>>()
            && slices.diffusion()
                == DIFFUSION_SLICE_NODE_IDS
                    .iter()
                    .map(|identifier| (*identifier).to_owned())
                    .collect::<Vec<_>>();
        assert!(early_slice_membership_is_exact);

        let object_info = ObjectInfoRegistry::from_node_registry(&registry)?;
        let object_info_is_read_only_projection =
            registry.registered().iter().chain(registry.inactive()).all(
                |(identifier, descriptor)| {
                    object_info
                        .nodes()
                        .get(identifier)
                        .is_some_and(|projected| {
                            projected.node_identifier == descriptor.node_identifier
                                && projected.display_name == descriptor.display_name
                                && projected.category == descriptor.category
                                && projected.source_python_module
                                    == registry
                                        .source_python_module(identifier)
                                        .unwrap_or_default()
                                && projected.schema_source == descriptor.schema_source
                                && projected.input.raw == descriptor.inputs
                                && projected.input.input_is_list == descriptor.input_is_list
                                && projected.input.lazy_inputs == descriptor.lazy_inputs
                                && projected.output.raw == descriptor.outputs
                                && projected.output.output_is_list == descriptor.output_is_list
                                && projected.output.output_node == descriptor.output_node
                                && projected.availability == descriptor.availability
                                && projected.catalog_status == descriptor.catalog_status
                                && projected.inactive_reason == descriptor.inactive_reason
                                && projected.feature_id == descriptor.feature_id
                        })
                },
            );
        assert!(object_info_is_read_only_projection);

        let registry_source = include_str!("registry_generator.rs");
        let descriptor_source = include_str!("descriptor.rs");
        let mutable_plugin_and_execution_apis_are_absent = !registry_source
            .contains(&["register", "_plugin_nodes"].concat())
            && !registry_source.contains(&["plugin", "_nodes:"].concat())
            && !registry_source.contains(&["is", "_executable"].concat())
            && !descriptor_source.contains(&["NodeExecution", "Status"].concat())
            && !descriptor_source.contains(&["Signed", "Plugin"].concat());
        assert!(mutable_plugin_and_execution_apis_are_absent);

        write_validation_artifact(&ValidationArtifact {
            validation_id: "VAL-NODE-REGISTRY-001",
            registered_rows: registry.registered().len(),
            inactive_rows: registry.inactive().len(),
            registered_catalog_sha256: REGISTERED_CATALOG_SHA256,
            inactive_catalog_sha256: INACTIVE_CATALOG_SHA256,
            generated_descriptor_ids_sha256: GENERATED_DESCRIPTOR_IDS_SHA256,
            generated_descriptor_ids: crate::GENERATED_DESCRIPTOR_IDS,
            image_slice: IMAGE_SLICE_NODE_IDS,
            diffusion_slice: DIFFUSION_SLICE_NODE_IDS,
            environment: ValidationEnvironment {
                operating_system: std::env::consts::OS,
                architecture: std::env::consts::ARCH,
                registry_role: "read-side compatibility descriptors only",
            },
            cases: ValidationCases {
                catalog_round_trip,
                catalog_status_is_read_only,
                generated_descriptor_membership_is_exact,
                early_slice_membership_is_exact,
                object_info_is_read_only_projection,
                mutable_plugin_and_execution_apis_are_absent,
            },
            passed: 6,
            failed: 0,
            skipped: 0,
        })?;
        Ok(())
    }
}
