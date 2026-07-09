use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result, bail};
use clap::Args;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Args)]
pub struct ComfyInventoryArgs {
    /// External Comfy checkout root to scan.
    #[arg(long)]
    comfy_root: PathBuf,
    /// Inventory fixture path to compare and optionally update.
    #[arg(
        long,
        default_value = "crates/world_model/fixtures/comfy/source_inventory.json"
    )]
    output: PathBuf,
    /// Capture date to write into the fixture.
    #[arg(long, default_value = "2026-07-09")]
    captured_at: String,
    /// Print the diff summary without writing the output fixture.
    #[arg(long)]
    check: bool,
}

pub fn run(args: ComfyInventoryArgs) -> Result<()> {
    let source_root = args.comfy_root.canonicalize().with_context(|| {
        format!(
            "failed to resolve external Comfy checkout {}",
            args.comfy_root.display()
        )
    })?;
    if !source_root.is_dir() {
        bail!("{} is not a directory", source_root.display());
    }

    let previous = read_inventory(&args.output)?;
    let inventory = build_inventory(&source_root, &args.captured_at)?;
    print_summary(previous.as_ref(), &inventory);

    if args.check {
        return Ok(());
    }

    let json = serde_json::to_string_pretty(&inventory)?;
    fs::write(&args.output, format!("{json}\n"))
        .with_context(|| format!("failed to write {}", args.output.display()))?;
    Ok(())
}

fn build_inventory(source_root: &Path, captured_at: &str) -> Result<SimSourceInventory> {
    let route_regex =
        Regex::new(r#"(?m)@routes?\.(get|post|put|delete|patch)\(\s*["']([^"']+)["']"#)?;
    let route_add_regex = Regex::new(
        r#"(?m)(?:add_routes|add_get|add_post|add_put|add_delete|add_patch)\(\s*["']([^"']+)["']"#,
    )?;
    let class_regex = Regex::new(r#"(?m)^class\s+([A-Za-z_][A-Za-z0-9_]*)"#)?;
    let cli_flag_regex = Regex::new(r#""(--[A-Za-z0-9][A-Za-z0-9_-]*)""#)?;
    let folder_key_regex =
        Regex::new(r#"folder_names_and_paths\[\s*["']([A-Za-z0-9_-]+)["']\s*\]"#)?;
    let provider_regex = Regex::new(r#"(?m)^class\s+([A-Za-z_][A-Za-z0-9_]*(?:Node|Extension))"#)?;

    let mut items = Vec::new();
    let mut diagnostics = Vec::new();
    for path in source_files(source_root)? {
        let relative_path = path
            .strip_prefix(source_root)
            .expect("source path came from root walk");
        let source_path = format!("projects/comfy/{}", normalize_path(relative_path));
        match path.extension().and_then(|extension| extension.to_str()) {
            Some("py") => {
                let text = fs::read_to_string(&path)
                    .with_context(|| format!("failed to read {}", path.display()))?;
                let mut classified = false;

                for capture in route_regex.captures_iter(&text) {
                    let method = capture[1].to_ascii_uppercase();
                    items.push(SimSourceItem::classified(
                        SimSourceKind::Route,
                        source_path.clone(),
                        format!("{method} {}", &capture[2]),
                    ));
                    classified = true;
                }
                for capture in route_add_regex.captures_iter(&text) {
                    items.push(SimSourceItem::classified(
                        SimSourceKind::Route,
                        source_path.clone(),
                        capture[1].to_string(),
                    ));
                    classified = true;
                }
                for capture in cli_flag_regex.captures_iter(&text) {
                    items.push(SimSourceItem::classified(
                        SimSourceKind::CliFlag,
                        source_path.clone(),
                        capture[1].to_string(),
                    ));
                    classified = true;
                }

                if source_path.ends_with("server.py") && text.contains("WebSocketResponse") {
                    items.push(SimSourceItem::classified(
                        SimSourceKind::WebSocketProtocol,
                        source_path.clone(),
                        "GET /ws",
                    ));
                    classified = true;
                }

                let path_text = source_path.as_str();
                if path_text.ends_with("/nodes.py") {
                    for capture in class_regex.captures_iter(&text) {
                        items.push(SimSourceItem::classified(
                            SimSourceKind::CoreNode,
                            source_path.clone(),
                            capture[1].to_string(),
                        ));
                        classified = true;
                    }
                } else if path_text.contains("/nodes_") {
                    for capture in class_regex.captures_iter(&text) {
                        let kind = if path_text.contains("comfy_api_nodes") {
                            SimSourceKind::ApiProviderNode
                        } else {
                            SimSourceKind::ExtraNode
                        };
                        items.push(SimSourceItem::classified(
                            kind,
                            source_path.clone(),
                            capture[1].to_string(),
                        ));
                        classified = true;
                    }
                } else if path_text.contains("comfy_api_nodes") {
                    for capture in provider_regex.captures_iter(&text) {
                        items.push(SimSourceItem::classified(
                            SimSourceKind::ApiProviderNode,
                            source_path.clone(),
                            capture[1].to_string(),
                        ));
                        classified = true;
                    }
                }

                if path_text.contains("supported_models") {
                    for capture in class_regex.captures_iter(&text) {
                        items.push(SimSourceItem::classified(
                            SimSourceKind::ModelFamily,
                            source_path.clone(),
                            capture[1].to_string(),
                        ));
                        classified = true;
                    }
                }

                if path_text.ends_with("folder_paths.py") {
                    for capture in folder_key_regex.captures_iter(&text) {
                        items.push(SimSourceItem::classified(
                            SimSourceKind::ModelFolder,
                            source_path.clone(),
                            capture[1].to_string(),
                        ));
                        classified = true;
                    }
                }

                if path_text.contains("app/assets/api/") {
                    items.push(SimSourceItem::classified(
                        SimSourceKind::AssetApi,
                        source_path.clone(),
                        path.file_stem()
                            .and_then(|stem| stem.to_str())
                            .unwrap_or("asset-api"),
                    ));
                    classified = true;
                }

                if path_text.contains("comfy_extras/")
                    || path_text.contains("comfy_api_nodes/")
                    || path_text.contains("app/assets/")
                {
                    items.push(SimSourceItem::classified(
                        SimSourceKind::ExtensionHook,
                        source_path.clone(),
                        path.file_stem()
                            .and_then(|stem| stem.to_str())
                            .unwrap_or("extension"),
                    ));
                    classified = true;
                }

                if path_text.contains("/tests/") || path_text.contains("test") {
                    items.push(SimSourceItem::classified(
                        SimSourceKind::TestSurface,
                        source_path.clone(),
                        path.file_stem()
                            .and_then(|stem| stem.to_str())
                            .unwrap_or("test"),
                    ));
                    classified = true;
                }

                if !classified {
                    diagnostics.push(SimSourceDiagnostic::warning(
                        source_path.clone(),
                        "No configured feature extractor classified this Python source file",
                    ));
                    items.push(SimSourceItem::unclassified(source_path, "python"));
                }
            }
            Some("json") if source_path.contains("/blueprints/") => {
                let text = fs::read_to_string(&path)
                    .with_context(|| format!("failed to read {}", path.display()))?;
                let value = serde_json::from_str::<Value>(&text).unwrap_or(Value::Null);
                let symbol = value
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .or_else(|| {
                        path.file_stem()
                            .and_then(|stem| stem.to_str())
                            .map(str::to_string)
                    })
                    .unwrap_or_else(|| "blueprint".to_string());
                items.push(
                    SimSourceItem::classified(SimSourceKind::Blueprint, source_path, symbol)
                        .with_metadata(json!({ "format": "json" })),
                );
            }
            Some("yaml") | Some("yml") if source_path.ends_with("openapi.yaml") => {
                let text = fs::read_to_string(&path)
                    .with_context(|| format!("failed to read {}", path.display()))?;
                let value = serde_yaml::from_str::<serde_yaml::Value>(&text)
                    .unwrap_or(serde_yaml::Value::Null);
                collect_openapi_operations(&mut items, source_path, value);
            }
            Some("yaml") | Some("yml") if source_path.contains("/.github/") => {
                items.push(SimSourceItem::classified(
                    SimSourceKind::TestSurface,
                    source_path.clone(),
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("workflow"),
                ));
                items.push(SimSourceItem::classified(
                    SimSourceKind::PackagingSurface,
                    source_path.clone(),
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("workflow"),
                ));
            }
            Some("toml") | Some("lock") => {
                items.push(SimSourceItem::classified(
                    SimSourceKind::PackagingSurface,
                    source_path.clone(),
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("package"),
                ));
            }
            _ => {}
        }
    }

    items.sort_by(|left, right| left.source_id.cmp(&right.source_id));
    items.dedup_by(|left, right| left.source_id == right.source_id);
    diagnostics.sort_by(|left, right| left.source_path.cmp(&right.source_path));
    Ok(
        SimSourceInventory::new(1, "projects/comfy", captured_at, items)
            .with_diagnostics(diagnostics),
    )
}

fn collect_openapi_operations(
    items: &mut Vec<SimSourceItem>,
    source_path: String,
    value: serde_yaml::Value,
) {
    let Some(paths) = value.get("paths").and_then(serde_yaml::Value::as_mapping) else {
        return;
    };
    for (path, methods) in paths {
        let Some(path) = path.as_str() else {
            continue;
        };
        let Some(methods) = methods.as_mapping() else {
            continue;
        };
        for (method, operation) in methods {
            let Some(method) = method.as_str() else {
                continue;
            };
            let operation_id = operation
                .get("operationId")
                .and_then(serde_yaml::Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| format!("{} {}", method.to_ascii_uppercase(), path));
            items.push(SimSourceItem::classified(
                SimSourceKind::OpenApiOperation,
                source_path.clone(),
                operation_id,
            ));
        }
    }
}

fn source_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(path) = pending.pop() {
        for entry in
            fs::read_dir(&path).with_context(|| format!("failed to read {}", path.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            let file_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            if file_name == ".git"
                || file_name == "__pycache__"
                || file_name == ".venv"
                || file_name == "venv"
                || file_name == "node_modules"
            {
                continue;
            }
            if path.is_dir() {
                pending.push(path);
            } else {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

fn print_summary(previous: Option<&SimSourceInventory>, inventory: &SimSourceInventory) {
    println!("Comfy inventory refresh summary");
    println!("  total: {}", inventory.summary.total_items);
    for (kind, count) in &inventory.summary.counts_by_kind {
        println!("  {kind:?}: {count}");
    }

    let Some(previous) = previous else {
        println!("  previous fixture: missing");
        return;
    };

    let previous_ids = previous
        .items
        .iter()
        .map(|item| item.source_id.as_str())
        .collect::<BTreeSet<_>>();
    let next_ids = inventory
        .items
        .iter()
        .map(|item| item.source_id.as_str())
        .collect::<BTreeSet<_>>();
    let added = next_ids.difference(&previous_ids).count();
    let removed = previous_ids.difference(&next_ids).count();
    let previous_by_id = previous
        .items
        .iter()
        .map(|item| (item.source_id.as_str(), item))
        .collect::<BTreeMap<_, _>>();
    let changed = inventory
        .items
        .iter()
        .filter(|item| {
            previous_by_id
                .get(item.source_id.as_str())
                .is_some_and(|previous| *previous != *item)
        })
        .count();
    println!("  added: {added}");
    println!("  removed: {removed}");
    println!("  changed: {changed}");
}

fn read_inventory(path: &Path) -> Result<Option<SimSourceInventory>> {
    if !path.exists() {
        return Ok(None);
    }
    let text =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(Some(serde_json::from_str(&text).with_context(|| {
        format!("failed to parse {}", path.display())
    })?))
}

fn normalize_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct SimSourceInventory {
    schema_version: u32,
    source_root: String,
    captured_at: String,
    summary: SimSourceInventorySummary,
    items: Vec<SimSourceItem>,
    diagnostics: Vec<SimSourceDiagnostic>,
}

impl SimSourceInventory {
    fn new(
        schema_version: u32,
        source_root: impl Into<String>,
        captured_at: impl Into<String>,
        items: Vec<SimSourceItem>,
    ) -> Self {
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

    fn with_diagnostics(mut self, diagnostics: Vec<SimSourceDiagnostic>) -> Self {
        self.diagnostics = diagnostics;
        self
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
struct SimSourceInventorySummary {
    total_items: u64,
    counts_by_kind: BTreeMap<SimSourceKind, u64>,
    counts_by_status: BTreeMap<SimSourceExtractionStatus, u64>,
}

impl SimSourceInventorySummary {
    fn from_items(items: &[SimSourceItem]) -> Self {
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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
enum SimSourceKind {
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
enum SimSourceExtractionStatus {
    Classified,
    Unclassified,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct SimSourceItem {
    source_id: String,
    source_kind: SimSourceKind,
    source_path: String,
    symbol: String,
    category: Option<String>,
    metadata: Value,
    extraction_status: SimSourceExtractionStatus,
}

impl SimSourceItem {
    fn classified(
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
            metadata: Value::Object(Default::default()),
            extraction_status: SimSourceExtractionStatus::Classified,
        }
    }

    fn unclassified(source_path: impl Into<String>, symbol: impl Into<String>) -> Self {
        let source_path = source_path.into();
        let symbol = symbol.into();
        Self {
            source_id: stable_source_id(SimSourceKind::Unknown, &source_path, &symbol),
            source_kind: SimSourceKind::Unknown,
            source_path,
            symbol,
            category: None,
            metadata: Value::Object(Default::default()),
            extraction_status: SimSourceExtractionStatus::Unclassified,
        }
    }

    fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = metadata;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct SimSourceDiagnostic {
    code: String,
    source_path: String,
    message: String,
    severity: SimSourceDiagnosticSeverity,
}

impl SimSourceDiagnostic {
    fn warning(source_path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: "world_model.sim_inventory.unclassified".to_string(),
            source_path: source_path.into(),
            message: message.into(),
            severity: SimSourceDiagnosticSeverity::Warning,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
enum SimSourceDiagnosticSeverity {
    Warning,
}

fn stable_source_id(source_kind: SimSourceKind, source_path: &str, symbol: &str) -> String {
    let kind = format!("{source_kind:?}").to_ascii_lowercase();
    let path = source_path.replace(['/', '.', ':'], "_");
    let symbol = symbol.replace(['/', '.', ':', ' '], "_");
    format!("{kind}:{path}:{symbol}")
}
