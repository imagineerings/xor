use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimSourceInventory {
    pub schema_version: u32,
    pub source_root: String,
    pub captured_at: String,
    pub summary: SimSourceInventorySummary,
    pub items: Vec<SimSourceItem>,
    pub diagnostics: Vec<SimSourceDiagnostic>,
}

impl SimSourceInventory {
    pub fn new(
        schema_version: u32,
        source_root: impl Into<String>,
        captured_at: impl Into<String>,
        items: impl IntoIterator<Item = SimSourceItem>,
    ) -> Self {
        let items = items.into_iter().collect::<Vec<_>>();
        let summary = SimSourceInventorySummary::from_items(&items);
        Self {
            schema_version,
            source_root: source_root.into(),
            captured_at: captured_at.into(),
            summary,
            items,
            diagnostics: Vec::new(),
        }
    }

    pub fn with_diagnostic(mut self, diagnostic: SimSourceDiagnostic) -> Self {
        self.diagnostics.push(diagnostic);
        self
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimSourceInventorySummary {
    pub total_items: u64,
    pub counts_by_kind: BTreeMap<SimSourceKind, u64>,
    pub counts_by_status: BTreeMap<SimSourceExtractionStatus, u64>,
}

impl SimSourceInventorySummary {
    pub fn from_items(items: &[SimSourceItem]) -> Self {
        let mut summary = Self {
            total_items: items.len() as u64,
            counts_by_kind: BTreeMap::new(),
            counts_by_status: BTreeMap::new(),
        };
        for item in items {
            *summary.counts_by_kind.entry(item.source_kind).or_insert(0) += 1;
            *summary
                .counts_by_status
                .entry(item.extraction_status)
                .or_insert(0) += 1;
        }
        summary
    }

    pub fn count_for_kind(&self, source_kind: SimSourceKind) -> u64 {
        self.counts_by_kind
            .get(&source_kind)
            .copied()
            .unwrap_or_default()
    }

    pub fn count_for_status(&self, status: SimSourceExtractionStatus) -> u64 {
        self.counts_by_status
            .get(&status)
            .copied()
            .unwrap_or_default()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SimSourceKind {
    Route,
    WebSocketProtocol,
    CoreNode,
    ExtraNode,
    ApiProviderNode,
    ModelFamily,
    ModelFolder,
    Blueprint,
    AssetApi,
    ExtensionHook,
    CliFlag,
    OpenApiOperation,
    TestSurface,
    PackagingSurface,
    FrontendSurface,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SimSourceExtractionStatus {
    Classified,
    Unclassified,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimSourceItem {
    pub source_id: String,
    pub source_kind: SimSourceKind,
    pub source_path: String,
    pub symbol: String,
    pub category: Option<String>,
    pub metadata: serde_json::Value,
    pub extraction_status: SimSourceExtractionStatus,
}

impl SimSourceItem {
    pub fn classified(
        source_kind: SimSourceKind,
        source_path: impl Into<String>,
        symbol: impl Into<String>,
    ) -> Self {
        let source_path = source_path.into();
        let symbol = symbol.into();
        Self {
            source_id: stable_source_id(source_kind, &source_path, &symbol),
            source_kind,
            source_path,
            symbol,
            category: None,
            metadata: serde_json::Value::Object(Default::default()),
            extraction_status: SimSourceExtractionStatus::Classified,
        }
    }

    pub fn unclassified(
        source_path: impl Into<String>,
        diagnostic_symbol: impl Into<String>,
    ) -> Self {
        let source_path = source_path.into();
        let symbol = diagnostic_symbol.into();
        Self {
            source_id: stable_source_id(SimSourceKind::Unknown, &source_path, &symbol),
            source_kind: SimSourceKind::Unknown,
            source_path,
            symbol,
            category: None,
            metadata: serde_json::Value::Object(Default::default()),
            extraction_status: SimSourceExtractionStatus::Unclassified,
        }
    }

    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimSourceDiagnostic {
    pub code: String,
    pub source_path: String,
    pub message: String,
    pub severity: SimSourceDiagnosticSeverity,
}

impl SimSourceDiagnostic {
    pub fn warning(
        code: impl Into<String>,
        source_path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            source_path: source_path.into(),
            message: message.into(),
            severity: SimSourceDiagnosticSeverity::Warning,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimSourceDiagnosticSeverity {
    Error,
    Warning,
    Info,
}

fn stable_source_id(source_kind: SimSourceKind, source_path: &str, symbol: &str) -> String {
    let kind = format!("{source_kind:?}").to_ascii_lowercase();
    let path = source_path.replace(['/', '.', ':'], "_");
    let symbol = symbol.replace(['/', '.', ':', ' '], "_");
    format!("{kind}:{path}:{symbol}")
}
