use std::{collections::BTreeMap, path::PathBuf};

use crate::{
    SIM_EXTENSION_LOADER_HOOK_RESTORED_CODE, SIM_EXTENSION_LOADER_IMPORT_FAILED_CODE,
    SIM_EXTENSION_LOADER_MISSING_DEPENDENCY_CODE, SIM_EXTENSION_POLICY_SCRIPT_DENIED_CODE,
    SimExtensionId, SimExtensionLoadMetadata, SimExtensionLoader, SimExtensionPolicy,
    SimExtensionRecord, SimExtensionSourceKind,
};

#[test]
fn extension_loader_runs_permitted_prestartup_scripts() {
    let extension = record("script_pack");
    let mut metadata = BTreeMap::new();
    metadata.insert(
        extension.id.clone(),
        SimExtensionLoadMetadata::default().with_prestartup_script("prestartup.py"),
    );
    let loader =
        SimExtensionLoader::new(SimExtensionPolicy::default().with_script_allowed("script_pack"));

    let report = loader.load(std::slice::from_ref(&extension), &metadata);

    assert_eq!(report.loaded.len(), 1);
    assert!(report.loaded[0].prestartup_script_ran);
    assert!(report.skipped.is_empty());
    assert!(report.diagnostics.is_empty());
}

#[test]
fn extension_loader_skips_unpermitted_prestartup_scripts_with_policy_diagnostic() {
    let extension = record("script_pack");
    let mut metadata = BTreeMap::new();
    metadata.insert(
        extension.id.clone(),
        SimExtensionLoadMetadata::default().with_prestartup_script("prestartup.py"),
    );

    let report = SimExtensionLoader::new(SimExtensionPolicy::default())
        .load(std::slice::from_ref(&extension), &metadata);

    assert!(report.loaded.is_empty());
    assert_eq!(report.skipped.len(), 1);
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == SIM_EXTENSION_POLICY_SCRIPT_DENIED_CODE
            && diagnostic.extension_id.as_str() == "script-pack"
    }));
}

#[test]
fn extension_loader_isolates_import_failures_and_loads_other_extensions() {
    let broken = record("broken_pack");
    let healthy = record("healthy_pack");
    let mut metadata = BTreeMap::new();
    metadata.insert(
        broken.id.clone(),
        SimExtensionLoadMetadata::default().with_import_error("missing NODE_CLASS_MAPPINGS"),
    );

    let report = SimExtensionLoader::new(SimExtensionPolicy::default())
        .load(&[broken.clone(), healthy.clone()], &metadata);

    assert_eq!(report.loaded.len(), 1);
    assert_eq!(report.loaded[0].extension_id, healthy.id);
    assert_eq!(report.skipped.len(), 1);
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == SIM_EXTENSION_LOADER_IMPORT_FAILED_CODE
            && diagnostic.extension_id == broken.id
    }));
}

#[test]
fn extension_loader_reports_missing_dependencies_without_installing() {
    let extension = record("dependency_pack");
    let mut metadata = BTreeMap::new();
    metadata.insert(
        extension.id.clone(),
        SimExtensionLoadMetadata::default().with_missing_dependency("torchvision"),
    );

    let report = SimExtensionLoader::new(
        SimExtensionPolicy::default()
            .with_install_allowed("dependency_pack")
            .with_dependency_reviewed_install("dependency_pack"),
    )
    .load(std::slice::from_ref(&extension), &metadata);

    assert!(report.loaded.is_empty());
    assert_eq!(report.skipped.len(), 1);
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == SIM_EXTENSION_LOADER_MISSING_DEPENDENCY_CODE
            && diagnostic.message.contains("torchvision")
    }));
}

#[test]
fn extension_loader_restores_protected_hooks_after_loading() {
    let extension = record("hook_pack");
    let mut metadata = BTreeMap::new();
    metadata.insert(
        extension.id.clone(),
        SimExtensionLoadMetadata::default().with_protected_hook_change("sys.path"),
    );

    let report = SimExtensionLoader::new(SimExtensionPolicy::default())
        .load(std::slice::from_ref(&extension), &metadata);

    assert_eq!(report.loaded.len(), 1);
    assert_eq!(report.loaded[0].restored_hooks, vec!["sys.path"]);
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == SIM_EXTENSION_LOADER_HOOK_RESTORED_CODE
            && diagnostic.extension_id.as_str() == "hook-pack"
    }));
}

fn record(name: &str) -> SimExtensionRecord {
    SimExtensionRecord {
        id: SimExtensionId::new(name),
        display_name: name.to_string(),
        source_path: PathBuf::from(format!("/custom_nodes/{name}")),
        source_kind: SimExtensionSourceKind::Directory,
        root_index: 0,
        load_order: 0,
    }
}
