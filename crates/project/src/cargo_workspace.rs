use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use anyhow::{Context as _, Result, bail};
use cargo_metadata::{
    Dependency, DependencyKind, Metadata, Node, Package, PackageId, Target, TargetKind,
};

use crate::ProjectPath;

pub const CARGO_METADATA_FORMAT_VERSION: u64 = 1;
pub const MAX_CARGO_CONFIGURATION_DIAGNOSTICS: usize = 32;
pub const MAX_CARGO_CONFIGURATION_ITEMS: usize = 128;
pub const MAX_CARGO_CONFIGURATION_FIELD_BYTES: usize = 512;
pub const MAX_RUSTC_VERBOSE_VERSION_BYTES: usize = 64 * 1024;
pub const MAX_CARGO_DEPENDENCY_INSTANCES: usize = 32;
pub const MAX_CARGO_DEPENDENCY_FEATURES: usize = 256;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoWorkspaceSnapshot {
    pub revision: u64,
    pub input_fingerprint: u64,
    pub workspaces: Vec<CargoWorkspaceModel>,
    pub failures: Vec<CargoCandidateFailure>,
    pub completeness: CargoSnapshotCompleteness,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CargoSnapshotCompleteness {
    Complete,
    Partial,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct CargoWorkspaceKey {
    pub root: ProjectPath,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoWorkspaceModel {
    pub key: CargoWorkspaceKey,
    pub root_manifest: Option<ProjectPath>,
    pub display_name: String,
    pub is_virtual: bool,
    pub members: Vec<CargoPackageModel>,
    pub configuration: CargoWorkspaceConfiguration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoWorkspaceConfiguration {
    pub profiles: Vec<CargoProfileModel>,
    pub declared_toolchain: Option<CargoToolchainModel>,
    pub host_compiler: CargoHostCompilerModel,
    pub cargo_target: CargoTargetConfiguration,
    pub diagnostics: Vec<CargoConfigurationDiagnostic>,
    pub completeness: CargoConfigurationCompleteness,
}

impl CargoWorkspaceConfiguration {
    pub fn unresolved() -> Self {
        Self {
            profiles: implicit_cargo_profiles(),
            declared_toolchain: None,
            host_compiler: CargoHostCompilerModel::unknown(),
            cargo_target: CargoTargetConfiguration::UnresolvedCargoDefault,
            diagnostics: Vec::new(),
            completeness: CargoConfigurationCompleteness::Complete,
        }
    }

    pub fn retain_stale_safe_facts(&mut self, previous: &Self) {
        if self.host_compiler.status != CargoHostCompilerStatus::Available
            && previous.host_compiler.status == CargoHostCompilerStatus::Available
        {
            self.host_compiler.release = previous.host_compiler.release.clone();
            self.host_compiler.host_target = previous.host_compiler.host_target.clone();
            self.host_compiler.stale = true;
        }
        if self.declared_toolchain.is_none()
            && self.completeness == CargoConfigurationCompleteness::Partial
        {
            self.declared_toolchain = previous.declared_toolchain.clone();
        }
    }

    pub fn add_diagnostic(
        &mut self,
        source_path: Option<ProjectPath>,
        category: CargoConfigurationDiagnosticCategory,
        message: impl AsRef<str>,
    ) {
        self.completeness = CargoConfigurationCompleteness::Partial;
        if self.diagnostics.len() >= MAX_CARGO_CONFIGURATION_DIAGNOSTICS {
            return;
        }
        self.diagnostics.push(CargoConfigurationDiagnostic {
            source_path,
            category,
            message: bounded_configuration_field(message.as_ref()),
        });
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CargoConfigurationCompleteness {
    Complete,
    Partial,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoProfileModel {
    pub name: String,
    pub origin: CargoProfileOrigin,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CargoProfileOrigin {
    Implicit,
    Declared,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoToolchainModel {
    pub source_path: ProjectPath,
    pub format: CargoToolchainFormat,
    pub channel: Option<String>,
    pub components: Vec<String>,
    pub targets: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CargoToolchainFormat {
    Toml,
    Legacy,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoHostCompilerModel {
    pub status: CargoHostCompilerStatus,
    pub release: Option<String>,
    pub host_target: Option<String>,
    pub stale: bool,
}

impl CargoHostCompilerModel {
    pub fn unknown() -> Self {
        Self {
            status: CargoHostCompilerStatus::Unknown,
            release: None,
            host_target: None,
            stale: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CargoHostCompilerStatus {
    Unknown,
    Available,
    Restricted,
    Missing,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CargoTargetConfiguration {
    UnresolvedCargoDefault,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoConfigurationDiagnostic {
    pub source_path: Option<ProjectPath>,
    pub category: CargoConfigurationDiagnosticCategory,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CargoConfigurationDiagnosticCategory {
    Manifest,
    Toolchain,
    CompilerProbe,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoPackageModel {
    pub id: String,
    pub name: String,
    pub version: String,
    pub manifest_path: ProjectPath,
    pub is_default_member: bool,
    pub targets: Vec<CargoTargetModel>,
    pub features: Vec<CargoFeatureModel>,
    pub dependencies: Vec<CargoDependencyModel>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoTargetModel {
    pub name: String,
    pub kind: CargoTargetKind,
    pub crate_types: Vec<String>,
    pub source_path: Option<ProjectPath>,
    pub source_display_path: Option<String>,
    pub required_features: Vec<String>,
    pub edition: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum CargoTargetKind {
    Library,
    Binary,
    Example,
    Test,
    Bench,
    BuildScript,
    Other(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoFeatureModel {
    pub name: String,
    pub defined: bool,
    pub enabled: CargoFeatureEnabled,
    pub expands: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CargoFeatureEnabled {
    Enabled,
    Disabled,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoDependencyModel {
    pub declaration_name: String,
    pub rename: Option<String>,
    pub kind: CargoDependencyKind,
    pub version_requirement: String,
    pub optional: bool,
    pub uses_default_features: bool,
    pub requested_features: Vec<String>,
    pub target: Option<String>,
    pub source_kind: CargoDependencySourceKind,
    pub resolved_name: Option<String>,
    pub resolved_version: Option<String>,
    pub resolved_workspace_member: Option<ProjectPath>,
    pub local_manifest: Option<ProjectPath>,
    pub declaration_manifest: Option<ProjectPath>,
    pub declaration_origin: CargoDependencyDeclarationOrigin,
    pub resolved_instances: Vec<CargoResolvedDependencyModel>,
    pub resolution_truncated: bool,
    pub feature_causality: CargoDependencyFeatureCausality,
    pub cycle_detected: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CargoDependencyDeclarationOrigin {
    Direct,
    WorkspaceInherited,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CargoDependencyFeatureCausality {
    Validated,
    Ambiguous,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoResolvedDependencyModel {
    pub name: String,
    pub version: String,
    pub source_kind: CargoDependencySourceKind,
    pub enabled_features: Vec<String>,
    pub lock_status: CargoDependencyLockStatus,
    pub workspace_member: Option<ProjectPath>,
    pub local_manifest: Option<ProjectPath>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CargoDependencyLockStatus {
    Locked,
    NotLocked,
    MissingLockfile,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum CargoDependencyKind {
    Normal,
    Development,
    Build,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CargoDependencySourceKind {
    Path,
    Registry,
    Git,
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoCandidateFailure {
    pub manifest_path: ProjectPath,
    pub category: CargoWorkspaceErrorCategory,
    pub message: String,
    pub has_stale_model: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CargoWorkspaceErrorCategory {
    Restricted,
    CargoNotFound,
    CargoFailed,
    InvalidMetadata,
    UnsupportedMetadata,
    Disconnected,
    Cancelled,
    Internal,
}

pub fn implicit_cargo_profiles() -> Vec<CargoProfileModel> {
    ["dev", "release"]
        .into_iter()
        .map(|name| CargoProfileModel {
            name: name.to_string(),
            origin: CargoProfileOrigin::Implicit,
        })
        .collect()
}

pub fn parse_cargo_profiles(manifest: &str) -> Result<Vec<CargoProfileModel>> {
    let value = toml::from_str::<toml::Value>(manifest).context("invalid Cargo manifest TOML")?;
    let Some(profile_value) = value.get("profile") else {
        return Ok(implicit_cargo_profiles());
    };
    let profiles = profile_value
        .as_table()
        .context("Cargo manifest profile must be a table")?;
    let mut names = BTreeSet::new();
    for (name, descriptor) in profiles {
        if name.is_empty() || name.len() > MAX_CARGO_CONFIGURATION_FIELD_BYTES {
            bail!("Cargo profile name is empty or exceeds the supported length");
        }
        if !descriptor.is_table() {
            bail!("Cargo profile {name} must be a table");
        }
        names.insert(name.clone());
    }
    if names.len() > MAX_CARGO_CONFIGURATION_ITEMS {
        bail!("Cargo manifest declares more than {MAX_CARGO_CONFIGURATION_ITEMS} profiles");
    }

    let mut result = implicit_cargo_profiles();
    for name in names {
        if let Some(implicit) = result.iter_mut().find(|profile| profile.name == name) {
            implicit.origin = CargoProfileOrigin::Declared;
        } else {
            result.push(CargoProfileModel {
                name,
                origin: CargoProfileOrigin::Declared,
            });
        }
    }
    Ok(result)
}

pub fn parse_rust_toolchain(
    source_path: ProjectPath,
    contents: &str,
) -> Result<CargoToolchainModel> {
    let first_content_line = contents
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'));
    if !contents.trim_start().starts_with('[') {
        let channel = first_content_line.context("legacy rust-toolchain is empty")?;
        if channel.len() > MAX_CARGO_CONFIGURATION_FIELD_BYTES {
            bail!("legacy rust-toolchain channel exceeds the supported length");
        }
        return Ok(CargoToolchainModel {
            source_path,
            format: CargoToolchainFormat::Legacy,
            channel: Some(channel.to_string()),
            components: Vec::new(),
            targets: Vec::new(),
        });
    }

    let value =
        toml::from_str::<toml::Value>(contents).context("invalid rust-toolchain.toml contents")?;
    let toolchain = value
        .get("toolchain")
        .and_then(toml::Value::as_table)
        .context("rust-toolchain.toml is missing a [toolchain] table")?;
    let channel = optional_toolchain_string(toolchain, "channel")?;
    let components = optional_toolchain_string_array(toolchain, "components")?;
    let targets = optional_toolchain_string_array(toolchain, "targets")?;
    if channel.is_none() && components.is_empty() && targets.is_empty() {
        bail!("rust-toolchain.toml does not declare a supported toolchain field");
    }
    Ok(CargoToolchainModel {
        source_path,
        format: CargoToolchainFormat::Toml,
        channel,
        components,
        targets,
    })
}

pub fn parse_rustc_verbose_version(output: &[u8]) -> Result<CargoHostCompilerModel> {
    if output.len() > MAX_RUSTC_VERBOSE_VERSION_BYTES {
        bail!("rustc -vV output exceeds the supported limit");
    }
    let output = std::str::from_utf8(output).context("rustc -vV output is not valid UTF-8")?;
    let release = rustc_field(output, "release:").context("rustc -vV output is missing release")?;
    let host_target = rustc_field(output, "host:").context("rustc -vV output is missing host")?;
    Ok(CargoHostCompilerModel {
        status: CargoHostCompilerStatus::Available,
        release: Some(release),
        host_target: Some(host_target),
        stale: false,
    })
}

fn optional_toolchain_string(
    table: &toml::map::Map<String, toml::Value>,
    key: &str,
) -> Result<Option<String>> {
    let Some(value) = table.get(key) else {
        return Ok(None);
    };
    let value = value
        .as_str()
        .with_context(|| format!("rust-toolchain {key} must be a string"))?;
    if value.len() > MAX_CARGO_CONFIGURATION_FIELD_BYTES {
        bail!("rust-toolchain {key} exceeds the supported length");
    }
    Ok(Some(value.to_string()))
}

fn optional_toolchain_string_array(
    table: &toml::map::Map<String, toml::Value>,
    key: &str,
) -> Result<Vec<String>> {
    let Some(value) = table.get(key) else {
        return Ok(Vec::new());
    };
    let values = value
        .as_array()
        .with_context(|| format!("rust-toolchain {key} must be an array"))?;
    if values.len() > MAX_CARGO_CONFIGURATION_ITEMS {
        bail!("rust-toolchain {key} exceeds the supported item limit");
    }
    values
        .iter()
        .map(|value| {
            let value = value
                .as_str()
                .with_context(|| format!("rust-toolchain {key} entries must be strings"))?;
            if value.len() > MAX_CARGO_CONFIGURATION_FIELD_BYTES {
                bail!("rust-toolchain {key} entry exceeds the supported length");
            }
            Ok(value.to_string())
        })
        .collect()
}

fn rustc_field(output: &str, prefix: &str) -> Option<String> {
    output
        .lines()
        .find_map(|line| line.strip_prefix(prefix).map(str::trim))
        .filter(|value| !value.is_empty())
        .map(bounded_configuration_field)
}

fn bounded_configuration_field(value: &str) -> String {
    let mut boundary = value.len().min(MAX_CARGO_CONFIGURATION_FIELD_BYTES);
    while !value.is_char_boundary(boundary) {
        boundary = boundary.saturating_sub(1);
    }
    value[..boundary].to_string()
}

pub fn parse_metadata(output: &[u8]) -> Result<Metadata> {
    let value: serde_json::Value =
        serde_json::from_slice(output).context("invalid Cargo metadata JSON")?;
    let version = value
        .get("version")
        .and_then(serde_json::Value::as_u64)
        .context("Cargo metadata is missing a numeric format version")?;
    if version != CARGO_METADATA_FORMAT_VERSION {
        bail!("unsupported Cargo metadata format version {version}");
    }
    serde_json::from_value(value).context("invalid Cargo metadata structure")
}

pub fn workspace_from_metadata(
    metadata: &Metadata,
    mut resolve_path: impl FnMut(&Path) -> Option<ProjectPath>,
) -> Result<CargoWorkspaceModel> {
    let root = resolve_path(metadata.workspace_root.as_std_path())
        .context("Cargo workspace root is outside visible worktrees")?;
    let root_manifest = resolve_path(metadata.workspace_root.join("Cargo.toml").as_std_path());
    let workspace_ids: BTreeSet<&PackageId> = metadata.workspace_members.iter().collect();
    let default_ids: BTreeSet<&PackageId> = if metadata.workspace_default_members.is_available() {
        metadata.workspace_default_members.iter().collect()
    } else {
        BTreeSet::new()
    };
    let packages_by_id: HashMap<&PackageId, &Package> = metadata
        .packages
        .iter()
        .map(|package| (&package.id, package))
        .collect();
    let nodes_by_id: HashMap<&PackageId, &Node> = metadata
        .resolve
        .iter()
        .flat_map(|resolve| &resolve.nodes)
        .map(|node| (&node.id, node))
        .collect();

    let mut members = Vec::new();
    for package in metadata
        .packages
        .iter()
        .filter(|package| workspace_ids.contains(&package.id))
    {
        let Some(manifest_path) = resolve_path(package.manifest_path.as_std_path()) else {
            continue;
        };
        members.push(convert_package(
            package,
            manifest_path,
            default_ids.contains(&package.id),
            nodes_by_id.get(&package.id).copied(),
            &nodes_by_id,
            &packages_by_id,
            &workspace_ids,
            &mut resolve_path,
        ));
    }

    if !metadata.workspace_members.is_empty() && members.is_empty() {
        bail!("Cargo workspace has no members in visible worktrees");
    }

    members.sort_by(|left, right| {
        right
            .is_default_member
            .cmp(&left.is_default_member)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.version.cmp(&right.version))
            .then_with(|| left.manifest_path.cmp(&right.manifest_path))
    });

    let display_name = root
        .path
        .file_name()
        .map(|name| name.to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Cargo workspace".to_string());

    Ok(CargoWorkspaceModel {
        key: CargoWorkspaceKey { root },
        root_manifest,
        display_name,
        is_virtual: metadata.root_package().is_none(),
        members,
        configuration: CargoWorkspaceConfiguration::unresolved(),
    })
}

fn convert_package(
    package: &Package,
    manifest_path: ProjectPath,
    is_default_member: bool,
    node: Option<&Node>,
    nodes_by_id: &HashMap<&PackageId, &Node>,
    packages_by_id: &HashMap<&PackageId, &Package>,
    workspace_ids: &BTreeSet<&PackageId>,
    resolve_path: &mut impl FnMut(&Path) -> Option<ProjectPath>,
) -> CargoPackageModel {
    let mut targets = package
        .targets
        .iter()
        .map(|target| convert_target(target, resolve_path))
        .collect::<Vec<_>>();
    targets.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.source_display_path.cmp(&right.source_display_path))
    });

    let mut feature_names: BTreeSet<String> = package.features.keys().cloned().collect();
    if let Some(node) = node {
        feature_names.extend(node.features.iter().cloned());
    }
    let features = feature_names
        .into_iter()
        .map(|name| {
            let expands = package.features.get(&name).cloned().unwrap_or_default();
            CargoFeatureModel {
                defined: package.features.contains_key(&name),
                enabled: match node {
                    Some(node) if node.features.contains(&name) => CargoFeatureEnabled::Enabled,
                    Some(_) => CargoFeatureEnabled::Disabled,
                    None => CargoFeatureEnabled::Unknown,
                },
                name,
                expands,
            }
        })
        .collect();

    let mut dependencies = package
        .dependencies
        .iter()
        .map(|dependency| {
            convert_dependency(
                dependency,
                node,
                nodes_by_id,
                packages_by_id,
                workspace_ids,
                resolve_path,
            )
        })
        .collect::<Vec<_>>();
    dependencies.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| dependency_label(left).cmp(dependency_label(right)))
            .then_with(|| left.target.cmp(&right.target))
            .then_with(|| left.version_requirement.cmp(&right.version_requirement))
    });

    CargoPackageModel {
        id: stable_package_id(&manifest_path, &package.name, &package.version.to_string()),
        name: package.name.clone(),
        version: package.version.to_string(),
        manifest_path,
        is_default_member,
        targets,
        features,
        dependencies,
    }
}

pub(crate) fn stable_package_id(manifest_path: &ProjectPath, name: &str, version: &str) -> String {
    format!(
        "{}:{}:{name}@{version}",
        manifest_path.worktree_id.to_proto(),
        manifest_path.path.as_unix_str()
    )
}

fn convert_target(
    target: &Target,
    resolve_path: &mut impl FnMut(&Path) -> Option<ProjectPath>,
) -> CargoTargetModel {
    let source_path = resolve_path(target.src_path.as_std_path());
    let source_display_path = source_path
        .as_ref()
        .map(|path| path.path.as_unix_str().to_string());
    CargoTargetModel {
        name: target.name.clone(),
        kind: normalize_target_kind(&target.kind),
        crate_types: target.crate_types.iter().map(ToString::to_string).collect(),
        source_path,
        source_display_path,
        required_features: target.required_features.clone(),
        edition: target.edition.to_string(),
    }
}

fn normalize_target_kind(kinds: &[TargetKind]) -> CargoTargetKind {
    let Some(kind) = kinds.first() else {
        return CargoTargetKind::Other("unknown".to_string());
    };
    match kind {
        TargetKind::Lib
        | TargetKind::RLib
        | TargetKind::DyLib
        | TargetKind::CDyLib
        | TargetKind::StaticLib
        | TargetKind::ProcMacro => CargoTargetKind::Library,
        TargetKind::Bin => CargoTargetKind::Binary,
        TargetKind::Example => CargoTargetKind::Example,
        TargetKind::Test => CargoTargetKind::Test,
        TargetKind::Bench => CargoTargetKind::Bench,
        TargetKind::CustomBuild => CargoTargetKind::BuildScript,
        TargetKind::Unknown(value) => CargoTargetKind::Other(value.clone()),
        _ => CargoTargetKind::Other(kind.to_string()),
    }
}

fn convert_dependency(
    dependency: &Dependency,
    node: Option<&Node>,
    nodes_by_id: &HashMap<&PackageId, &Node>,
    packages_by_id: &HashMap<&PackageId, &Package>,
    workspace_ids: &BTreeSet<&PackageId>,
    resolve_path: &mut impl FnMut(&Path) -> Option<ProjectPath>,
) -> CargoDependencyModel {
    let alias = dependency.rename.as_deref().unwrap_or(&dependency.name);
    let matching_packages = node
        .into_iter()
        .flat_map(|node| &node.deps)
        .filter(|node_dependency| node_dependency.name == alias)
        .filter_map(|node_dependency| {
            packages_by_id
                .get(&node_dependency.pkg)
                .copied()
                .map(|package| (node_dependency, package))
        })
        .collect::<Vec<_>>();
    let resolved_package = matching_packages.first().map(|(_, package)| *package);
    let local_manifest = dependency
        .path
        .as_ref()
        .and_then(|path| resolve_path(path.join("Cargo.toml").as_std_path()))
        .or_else(|| {
            resolved_package.and_then(|package| resolve_path(package.manifest_path.as_std_path()))
        });
    let resolved_workspace_member = resolved_package
        .filter(|package| workspace_ids.contains(&package.id))
        .and_then(|package| resolve_path(package.manifest_path.as_std_path()));
    let resolution_truncated = matching_packages.len() > MAX_CARGO_DEPENDENCY_INSTANCES;
    let resolved_instances = matching_packages
        .into_iter()
        .take(MAX_CARGO_DEPENDENCY_INSTANCES)
        .map(|(node_dependency, package)| {
            let workspace_member = workspace_ids
                .contains(&package.id)
                .then(|| resolve_path(package.manifest_path.as_std_path()))
                .flatten();
            let local_manifest = dependency
                .path
                .as_ref()
                .and_then(|path| resolve_path(path.join("Cargo.toml").as_std_path()))
                .or_else(|| resolve_path(package.manifest_path.as_std_path()));
            let mut enabled_features = nodes_by_id
                .get(&node_dependency.pkg)
                .map(|node| node.features.clone())
                .unwrap_or_default();
            enabled_features.sort();
            enabled_features.dedup();
            enabled_features.truncate(MAX_CARGO_DEPENDENCY_FEATURES);
            CargoResolvedDependencyModel {
                name: package.name.clone(),
                version: package.version.to_string(),
                source_kind: dependency_source_kind(dependency),
                enabled_features,
                lock_status: CargoDependencyLockStatus::Unknown,
                workspace_member,
                local_manifest,
            }
        })
        .collect::<Vec<_>>();

    CargoDependencyModel {
        declaration_name: dependency.name.clone(),
        rename: dependency.rename.clone(),
        kind: match dependency.kind {
            DependencyKind::Normal => CargoDependencyKind::Normal,
            DependencyKind::Development => CargoDependencyKind::Development,
            DependencyKind::Build => CargoDependencyKind::Build,
            DependencyKind::Unknown => CargoDependencyKind::Unknown,
        },
        version_requirement: dependency.req.to_string(),
        optional: dependency.optional,
        uses_default_features: dependency.uses_default_features,
        requested_features: dependency.features.clone(),
        target: dependency.target.as_ref().map(ToString::to_string),
        source_kind: dependency_source_kind(dependency),
        resolved_name: resolved_package.map(|package| package.name.clone()),
        resolved_version: resolved_package.map(|package| package.version.to_string()),
        resolved_workspace_member,
        local_manifest,
        declaration_manifest: None,
        declaration_origin: CargoDependencyDeclarationOrigin::Unknown,
        feature_causality: if resolved_instances.len() == 1 {
            CargoDependencyFeatureCausality::Validated
        } else if resolved_instances.len() > 1 {
            CargoDependencyFeatureCausality::Ambiguous
        } else {
            CargoDependencyFeatureCausality::Unknown
        },
        resolved_instances,
        resolution_truncated,
        cycle_detected: false,
    }
}

fn dependency_source_kind(dependency: &Dependency) -> CargoDependencySourceKind {
    if dependency.path.is_some() {
        CargoDependencySourceKind::Path
    } else if dependency
        .source
        .as_deref()
        .is_some_and(|source| source.starts_with("git+"))
    {
        CargoDependencySourceKind::Git
    } else if dependency.registry.is_some()
        || dependency
            .source
            .as_deref()
            .is_some_and(|source| source.starts_with("registry+"))
    {
        CargoDependencySourceKind::Registry
    } else {
        CargoDependencySourceKind::Other
    }
}

pub fn enrich_dependency_provenance(
    workspace: &mut CargoWorkspaceModel,
    manifests: &BTreeMap<ProjectPath, String>,
    lock_contents: Option<&str>,
) {
    let lock_packages = lock_contents.and_then(parse_lock_packages);
    let lock_state = match (lock_contents, lock_packages.as_ref()) {
        (None, _) => CargoDependencyLockStatus::MissingLockfile,
        (Some(_), None) => CargoDependencyLockStatus::Unknown,
        (Some(_), Some(_)) => CargoDependencyLockStatus::NotLocked,
    };
    let adjacency = workspace
        .members
        .iter()
        .map(|package| {
            (
                package.manifest_path.clone(),
                package
                    .dependencies
                    .iter()
                    .flat_map(|dependency| &dependency.resolved_instances)
                    .filter_map(|resolved| resolved.workspace_member.clone())
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    for package in &mut workspace.members {
        let manifest = manifests
            .get(&package.manifest_path)
            .and_then(|contents| toml::from_str::<toml::Value>(contents).ok());
        for dependency in &mut package.dependencies {
            dependency.declaration_manifest = manifests
                .contains_key(&package.manifest_path)
                .then(|| package.manifest_path.clone());
            dependency.declaration_origin = manifest
                .as_ref()
                .and_then(|manifest| manifest_dependency_value(manifest, dependency))
                .map(|value| {
                    if value
                        .as_table()
                        .and_then(|table| table.get("workspace"))
                        .and_then(toml::Value::as_bool)
                        == Some(true)
                    {
                        CargoDependencyDeclarationOrigin::WorkspaceInherited
                    } else {
                        CargoDependencyDeclarationOrigin::Direct
                    }
                })
                .unwrap_or(CargoDependencyDeclarationOrigin::Unknown);

            for resolved in &mut dependency.resolved_instances {
                resolved.lock_status = lock_packages
                    .as_ref()
                    .map(|packages| {
                        if packages.iter().any(|package| package.matches(resolved)) {
                            CargoDependencyLockStatus::Locked
                        } else {
                            CargoDependencyLockStatus::NotLocked
                        }
                    })
                    .unwrap_or(lock_state);
                dependency.cycle_detected |=
                    resolved.workspace_member.as_ref().is_some_and(|target| {
                        dependency_path_reaches(target, &package.manifest_path, &adjacency)
                    });
            }
        }
    }
}

fn manifest_dependency_value<'a>(
    manifest: &'a toml::Value,
    dependency: &CargoDependencyModel,
) -> Option<&'a toml::Value> {
    let section = match dependency.kind {
        CargoDependencyKind::Normal => "dependencies",
        CargoDependencyKind::Development => "dev-dependencies",
        CargoDependencyKind::Build => "build-dependencies",
        CargoDependencyKind::Unknown => return None,
    };
    let key = dependency
        .rename
        .as_deref()
        .unwrap_or(&dependency.declaration_name);
    let root_match = manifest
        .get(section)
        .and_then(toml::Value::as_table)
        .and_then(|table| table.get(key));
    if dependency.target.is_none() && root_match.is_some() {
        return root_match;
    }
    manifest
        .get("target")
        .and_then(toml::Value::as_table)
        .into_iter()
        .flat_map(|targets| targets.values())
        .filter_map(toml::Value::as_table)
        .filter_map(|target| target.get(section))
        .filter_map(toml::Value::as_table)
        .find_map(|dependencies| dependencies.get(key))
        .or(root_match)
}

#[derive(Clone, Debug)]
struct LockPackage {
    name: String,
    version: String,
    source_kind: CargoDependencySourceKind,
}

impl LockPackage {
    fn matches(&self, resolved: &CargoResolvedDependencyModel) -> bool {
        self.name == resolved.name
            && self.version == resolved.version
            && (self.source_kind == resolved.source_kind
                || matches!(resolved.source_kind, CargoDependencySourceKind::Path))
    }
}

fn parse_lock_packages(contents: &str) -> Option<Vec<LockPackage>> {
    let value = toml::from_str::<toml::Value>(contents).ok()?;
    let packages = value.get("package")?.as_array()?;
    let mut result = Vec::with_capacity(packages.len());
    for package in packages {
        let package = package.as_table()?;
        let name = package.get("name")?.as_str()?;
        let version = package.get("version")?.as_str()?;
        let source_kind = package
            .get("source")
            .and_then(toml::Value::as_str)
            .map(|source| {
                if source.starts_with("registry+") {
                    CargoDependencySourceKind::Registry
                } else if source.starts_with("git+") {
                    CargoDependencySourceKind::Git
                } else {
                    CargoDependencySourceKind::Other
                }
            })
            .unwrap_or(CargoDependencySourceKind::Path);
        result.push(LockPackage {
            name: name.to_string(),
            version: version.to_string(),
            source_kind,
        });
    }
    Some(result)
}

fn dependency_path_reaches(
    start: &ProjectPath,
    destination: &ProjectPath,
    adjacency: &BTreeMap<ProjectPath, Vec<ProjectPath>>,
) -> bool {
    let mut pending = vec![start.clone()];
    let mut visited = BTreeSet::new();
    while let Some(path) = pending.pop() {
        if &path == destination {
            return true;
        }
        if !visited.insert(path.clone()) || visited.len() > 10_000 {
            continue;
        }
        if let Some(next) = adjacency.get(&path) {
            pending.extend(next.iter().cloned());
        }
    }
    false
}

fn dependency_label(dependency: &CargoDependencyModel) -> &str {
    dependency
        .rename
        .as_deref()
        .unwrap_or(&dependency.declaration_name)
}

pub fn deduplicate_workspaces(
    workspaces: impl IntoIterator<Item = CargoWorkspaceModel>,
) -> Vec<CargoWorkspaceModel> {
    let mut by_key = BTreeMap::new();
    for workspace in workspaces {
        by_key.entry(workspace.key.clone()).or_insert(workspace);
    }
    by_key.into_values().collect()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use settings::WorktreeId;
    use util::rel_path::RelPath;

    use super::*;

    fn project_path(path: &str) -> ProjectPath {
        ProjectPath {
            worktree_id: WorktreeId::from_usize(1),
            path: Arc::from(RelPath::from_unix_str(path).expect("fixture path should be valid")),
        }
    }

    #[test]
    fn cargo_workspace_profiles_include_implicit_and_declared_profiles() {
        let profiles = parse_cargo_profiles(include_str!(
            "../test_data/cargo_workspace/profiles-custom.toml"
        ))
        .expect("custom profile fixture should parse");
        assert_eq!(
            profiles,
            vec![
                CargoProfileModel {
                    name: "dev".to_string(),
                    origin: CargoProfileOrigin::Declared,
                },
                CargoProfileModel {
                    name: "release".to_string(),
                    origin: CargoProfileOrigin::Implicit,
                },
                CargoProfileModel {
                    name: "ci".to_string(),
                    origin: CargoProfileOrigin::Declared,
                },
                CargoProfileModel {
                    name: "ship".to_string(),
                    origin: CargoProfileOrigin::Declared,
                },
            ]
        );
        assert!(
            parse_cargo_profiles(include_str!(
                "../test_data/cargo_workspace/profiles-malformed.toml"
            ))
            .is_err()
        );
    }

    #[test]
    fn cargo_workspace_toolchain_formats_are_bounded_declarations() {
        let toml = parse_rust_toolchain(
            project_path("rust-toolchain.toml"),
            include_str!("../test_data/cargo_workspace/rust-toolchain.toml"),
        )
        .expect("TOML toolchain fixture should parse");
        assert_eq!(toml.format, CargoToolchainFormat::Toml);
        assert_eq!(toml.channel.as_deref(), Some("stable"));
        assert_eq!(toml.components, ["rustfmt", "clippy"]);
        assert_eq!(toml.targets, ["wasm32-unknown-unknown"]);

        let legacy = parse_rust_toolchain(
            project_path("rust-toolchain"),
            include_str!("../test_data/cargo_workspace/rust-toolchain-legacy"),
        )
        .expect("legacy toolchain fixture should parse");
        assert_eq!(legacy.format, CargoToolchainFormat::Legacy);
        assert_eq!(legacy.channel.as_deref(), Some("1.90.0"));
    }

    #[test]
    fn cargo_workspace_rustc_probe_parses_only_release_and_host() {
        let compiler = parse_rustc_verbose_version(include_bytes!(
            "../test_data/cargo_workspace/rustc-vv.txt"
        ))
        .expect("rustc fixture should parse");
        assert_eq!(compiler.status, CargoHostCompilerStatus::Available);
        assert_eq!(compiler.release.as_deref(), Some("1.90.0"));
        assert_eq!(
            compiler.host_target.as_deref(),
            Some("x86_64-unknown-linux-gnu")
        );
        assert!(parse_rustc_verbose_version(b"rustc 1.90.0\n").is_err());
    }

    #[test]
    fn cargo_workspace_configuration_can_retain_safe_stale_facts() {
        let mut previous = CargoWorkspaceConfiguration::unresolved();
        previous.host_compiler = CargoHostCompilerModel {
            status: CargoHostCompilerStatus::Available,
            release: Some("1.90.0".to_string()),
            host_target: Some("x86_64-unknown-linux-gnu".to_string()),
            stale: false,
        };
        previous.declared_toolchain = Some(
            parse_rust_toolchain(
                project_path("rust-toolchain.toml"),
                include_str!("../test_data/cargo_workspace/rust-toolchain.toml"),
            )
            .expect("toolchain fixture should parse"),
        );
        let mut current = CargoWorkspaceConfiguration::unresolved();
        current.host_compiler.status = CargoHostCompilerStatus::Failed;
        current.add_diagnostic(
            None,
            CargoConfigurationDiagnosticCategory::CompilerProbe,
            "probe failed",
        );
        current.retain_stale_safe_facts(&previous);
        assert!(current.host_compiler.stale);
        assert_eq!(
            current.host_compiler.release,
            previous.host_compiler.release
        );
        assert_eq!(current.declared_toolchain, previous.declared_toolchain);
    }

    #[test]
    fn cargo_dependency_provenance_combines_manifest_lock_resolution_and_cycles() {
        let package_a_manifest = project_path("a/Cargo.toml");
        let package_b_manifest = project_path("b/Cargo.toml");
        let dependency = |name: &str, workspace_member: ProjectPath| CargoDependencyModel {
            declaration_name: name.to_string(),
            rename: Some(format!("{name}_alias")),
            kind: CargoDependencyKind::Normal,
            version_requirement: "^1".to_string(),
            optional: true,
            uses_default_features: false,
            requested_features: vec!["derive".to_string()],
            target: None,
            source_kind: CargoDependencySourceKind::Path,
            resolved_name: Some(name.to_string()),
            resolved_version: Some("1.0.0".to_string()),
            resolved_workspace_member: Some(workspace_member.clone()),
            local_manifest: Some(workspace_member.clone()),
            declaration_manifest: None,
            declaration_origin: CargoDependencyDeclarationOrigin::Unknown,
            resolved_instances: vec![CargoResolvedDependencyModel {
                name: name.to_string(),
                version: "1.0.0".to_string(),
                source_kind: CargoDependencySourceKind::Path,
                enabled_features: vec!["derive".to_string()],
                lock_status: CargoDependencyLockStatus::Unknown,
                workspace_member: Some(workspace_member.clone()),
                local_manifest: Some(workspace_member),
            }],
            resolution_truncated: false,
            feature_causality: CargoDependencyFeatureCausality::Validated,
            cycle_detected: false,
        };
        let package = |name: &str, manifest_path: ProjectPath, dependency: CargoDependencyModel| {
            CargoPackageModel {
                id: name.to_string(),
                name: name.to_string(),
                version: "1.0.0".to_string(),
                manifest_path,
                is_default_member: false,
                targets: Vec::new(),
                features: Vec::new(),
                dependencies: vec![dependency],
            }
        };
        let mut workspace = CargoWorkspaceModel {
            key: CargoWorkspaceKey {
                root: project_path(""),
            },
            root_manifest: Some(project_path("Cargo.toml")),
            display_name: "fixture".to_string(),
            is_virtual: true,
            members: vec![
                package(
                    "a",
                    package_a_manifest.clone(),
                    dependency("b", package_b_manifest.clone()),
                ),
                package(
                    "b",
                    package_b_manifest.clone(),
                    dependency("a", package_a_manifest.clone()),
                ),
            ],
            configuration: CargoWorkspaceConfiguration::unresolved(),
        };
        let manifests = BTreeMap::from([
            (
                package_a_manifest.clone(),
                "[dependencies]\nb_alias = { workspace = true, features = [\"derive\"] }\n"
                    .to_string(),
            ),
            (
                package_b_manifest,
                "[dependencies]\na_alias = { path = \"../a\" }\n".to_string(),
            ),
        ]);
        let lock = "version = 4\n[[package]]\nname = \"a\"\nversion = \"1.0.0\"\n[[package]]\nname = \"b\"\nversion = \"1.0.0\"\n";
        enrich_dependency_provenance(&mut workspace, &manifests, Some(lock));

        let dependency_a = &workspace.members[0].dependencies[0];
        assert_eq!(
            dependency_a.declaration_origin,
            CargoDependencyDeclarationOrigin::WorkspaceInherited
        );
        assert_eq!(
            dependency_a.resolved_instances[0].lock_status,
            CargoDependencyLockStatus::Locked
        );
        assert!(dependency_a.cycle_detected);
        assert_eq!(
            dependency_a.declaration_manifest.as_ref(),
            Some(&package_a_manifest)
        );

        enrich_dependency_provenance(&mut workspace, &manifests, Some("malformed = ["));
        assert_eq!(
            workspace.members[0].dependencies[0].resolved_instances[0].lock_status,
            CargoDependencyLockStatus::Unknown
        );
        assert_eq!(
            workspace.members[1].dependencies[0].declaration_origin,
            CargoDependencyDeclarationOrigin::Direct
        );
    }
}
