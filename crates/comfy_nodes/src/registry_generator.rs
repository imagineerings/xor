use crate::{
    CatalogNodeDescriptor, CatalogNodeSource, CatalogNodeStatus, NODE_DESCRIPTOR_SCHEMA_VERSION,
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
    DuplicateNode(String),
    DuplicateFeature(String),
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
            Self::DuplicateNode(identifier) => {
                write!(formatter, "duplicate node identifier `{identifier}`")
            }
            Self::DuplicateFeature(identifier) => {
                write!(formatter, "duplicate node feature ID `{identifier}`")
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
        output.push_str("\r\n");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CatalogNodeStatus, DIFFUSION_SLICE_NODE_IDS, EarlySliceRegistry, IMAGE_SLICE_NODE_IDS,
        ObjectInfoRegistry,
    };
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

        let generated_descriptor_membership_is_exact = crate::GENERATED_MODULES
            == ["slices/native_diffusion", "slices/native_image"]
            && crate::GENERATED_DESCRIPTOR_IDS
                == [
                    "CLIPTextEncode",
                    "CheckpointLoaderSimple",
                    "EmptyLatentImage",
                    "ImageInvert",
                    "ImageScale",
                    "KSampler",
                    "LoadImage",
                    "PreviewImage",
                    "SaveImage",
                    "VAEDecode",
                ]
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

        let object_info = ObjectInfoRegistry::from_node_registry(&registry);
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
                                && projected.schema_source == descriptor.schema_source
                                && projected.input.raw == descriptor.inputs
                                && projected.input.input_is_list == descriptor.input_is_list
                                && projected.input.lazy_inputs == descriptor.lazy_inputs
                                && projected.output.raw == descriptor.outputs
                                && projected.output.output_is_list == descriptor.output_is_list
                                && projected.output.output_node == descriptor.output_node
                                && projected.availability == descriptor.availability
                                && projected.catalog_status == descriptor.catalog_status
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
