use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    SIM_EXTENSION_DISABLED_PACK_CODE, SIM_EXTENSION_NOT_WHITELISTED_CODE, SimExtensionDiscovery,
    SimExtensionDiscoveryConfig, SimExtensionId, SimExtensionSourceKind,
};

#[test]
fn extension_discovery_finds_python_files_and_directories_in_deterministic_order() {
    let root = temp_root("deterministic");
    create_dir(root.join("z_pack"));
    create_file(root.join("alpha.py"));
    create_file(root.join("ignored.txt"));
    create_dir(root.join("Beta Pack"));

    let report = SimExtensionDiscovery::default().discover_roots(std::slice::from_ref(&root));

    assert!(report.diagnostics.is_empty());
    let ids = report
        .extensions
        .iter()
        .map(|extension| extension.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["alpha", "beta-pack", "z-pack"]);
    assert_eq!(report.extensions[0].load_order, 0);
    assert_eq!(report.extensions[1].load_order, 1);
    assert_eq!(report.extensions[2].load_order, 2);
    assert_eq!(
        report.extensions[0].source_kind,
        SimExtensionSourceKind::PythonFile
    );
    assert_eq!(
        report.extensions[1].source_kind,
        SimExtensionSourceKind::Directory
    );
}

#[test]
fn extension_discovery_skips_disabled_packs_with_diagnostics() {
    let root = temp_root("disabled");
    create_dir(root.join("enabled_pack"));
    create_dir(root.join("legacy_pack.disabled"));

    let report = SimExtensionDiscovery::default().discover_roots(std::slice::from_ref(&root));

    assert_eq!(report.extensions.len(), 1);
    assert_eq!(report.extensions[0].id, SimExtensionId::new("enabled_pack"));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == SIM_EXTENSION_DISABLED_PACK_CODE
            && diagnostic
                .extension_id
                .as_ref()
                .is_some_and(|id| id.as_str() == "legacy-pack")
    }));
}

#[test]
fn extension_discovery_respects_global_disable_and_whitelist() {
    let root = temp_root("whitelist");
    create_dir(root.join("allowed_pack"));
    create_file(root.join("blocked_pack.py"));

    let discovery = SimExtensionDiscovery::new(
        SimExtensionDiscoveryConfig::default()
            .with_custom_nodes_enabled(false)
            .with_whitelisted_pack("allowed_pack"),
    );
    let report = discovery.discover_roots(std::slice::from_ref(&root));

    assert_eq!(report.extensions.len(), 1);
    assert_eq!(report.extensions[0].id.as_str(), "allowed-pack");
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == SIM_EXTENSION_NOT_WHITELISTED_CODE
            && diagnostic
                .extension_id
                .as_ref()
                .is_some_and(|id| id.as_str() == "blocked-pack")
    }));
}

#[test]
fn extension_discovery_treats_nonempty_whitelist_as_filter_when_enabled() {
    let root = temp_root("filter");
    create_dir(root.join("allowed_pack"));
    create_dir(root.join("other_pack"));

    let discovery = SimExtensionDiscovery::new(
        SimExtensionDiscoveryConfig::default().with_whitelisted_pack("allowed_pack"),
    );
    let report = discovery.discover_roots(std::slice::from_ref(&root));

    assert_eq!(report.extensions.len(), 1);
    assert_eq!(report.extensions[0].id.as_str(), "allowed-pack");
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == SIM_EXTENSION_NOT_WHITELISTED_CODE
            && diagnostic
                .extension_id
                .as_ref()
                .is_some_and(|id| id.as_str() == "other-pack")
    }));
}

fn temp_root(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("sim-extension-{label}-{unique}"));
    create_dir(root.clone());
    root
}

fn create_dir(path: impl AsRef<Path>) {
    fs::create_dir_all(path).expect("test directory should be created");
}

fn create_file(path: impl AsRef<Path>) {
    fs::write(path, b"# native Sim extension discovery fixture")
        .expect("test file should be created");
}
