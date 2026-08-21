use std::{path::Path, sync::Arc};

use anyhow::Result;
use async_trait::async_trait;
use fs::FakeFs;
use gpui::{AppContext as _, TestAppContext};
use project::{
    Project, ProjectPath,
    cargo_workspace::{
        CargoConfigurationCompleteness, CargoConfigurationDiagnosticCategory, CargoDependencyKind,
        CargoDependencySourceKind, CargoFeatureEnabled, CargoHostCompilerModel,
        CargoHostCompilerStatus, CargoProfileModel, CargoProfileOrigin, CargoTargetKind,
        CargoToolchainFormat, CargoWorkspaceConfiguration, parse_cargo_profiles, parse_metadata,
        parse_rust_toolchain, parse_rustc_verbose_version, workspace_from_metadata,
    },
    cargo_workspace_store::{
        CargoConfigurationProbe, CargoConfigurationProbeRequest, CargoMetadataRequest,
        CargoMetadataRunner, CargoWorkspaceStore,
    },
    trusted_worktrees::{self, DbTrustedPaths, TrustedWorktrees},
};
use serde_json::json;
use settings::WorktreeId;
use util::{paths::PathStyle, rel_path::RelPath};

fn resolve(root: &Path, path: &Path) -> Option<ProjectPath> {
    let relative = path.strip_prefix(root).ok()?;
    Some(ProjectPath {
        worktree_id: WorktreeId::from_usize(1),
        path: Arc::from(RelPath::new(relative, PathStyle::Posix).ok()?.as_ref()),
    })
}

fn fixture_path(path: &str) -> ProjectPath {
    ProjectPath {
        worktree_id: WorktreeId::from_usize(1),
        path: Arc::from(RelPath::from_unix_str(path).expect("fixture path should be valid")),
    }
}

#[test]
fn cargo_workspace_fixture_covers_model_kinds_and_stable_order() {
    let metadata = parse_metadata(include_bytes!(
        "../../test_data/cargo_workspace/workspace-v1.json"
    ))
    .expect("workspace fixture should parse");
    let model = workspace_from_metadata(&metadata, |path| resolve(Path::new("/workspace"), path))
        .expect("workspace fixture should convert");

    assert!(model.is_virtual);
    assert_eq!(model.members.len(), 2);
    let member = model
        .members
        .iter()
        .find(|member| member.name == "member-one")
        .expect("member-one should be present");
    assert!(!member.id.contains("/workspace"));
    assert!(member.id.contains("member-one/Cargo.toml"));
    assert!(member.is_default_member);
    assert_eq!(member.targets.len(), 7);
    assert!(
        member
            .targets
            .iter()
            .any(|target| target.kind == CargoTargetKind::Library)
    );
    assert!(
        member
            .targets
            .iter()
            .any(|target| target.kind == CargoTargetKind::Binary)
    );
    assert!(
        member
            .targets
            .iter()
            .any(|target| target.kind == CargoTargetKind::Example)
    );
    assert!(
        member
            .targets
            .iter()
            .any(|target| target.kind == CargoTargetKind::Test)
    );
    assert!(
        member
            .targets
            .iter()
            .any(|target| target.kind == CargoTargetKind::Bench)
    );
    assert!(
        member
            .targets
            .iter()
            .any(|target| target.kind == CargoTargetKind::BuildScript)
    );
    assert!(
        member
            .targets
            .iter()
            .any(|target| matches!(target.kind, CargoTargetKind::Other(_)))
    );

    assert_eq!(
        member
            .features
            .iter()
            .find(|feature| feature.name == "extra")
            .map(|feature| feature.enabled),
        Some(CargoFeatureEnabled::Enabled)
    );
    assert_eq!(
        member
            .features
            .iter()
            .find(|feature| feature.name == "implicit")
            .map(|feature| feature.defined),
        Some(false)
    );
    assert!(member.dependencies.iter().any(|dependency| {
        dependency.kind == CargoDependencyKind::Normal
            && dependency.rename.as_deref() == Some("renamed_member")
            && dependency.source_kind == CargoDependencySourceKind::Path
            && dependency.resolved_version.as_deref() == Some("0.2.0")
            && dependency.resolved_workspace_member.is_some()
    }));
    assert!(
        member
            .dependencies
            .iter()
            .any(|dependency| dependency.kind == CargoDependencyKind::Development)
    );
    assert!(
        member
            .dependencies
            .iter()
            .any(|dependency| dependency.kind == CargoDependencyKind::Build)
    );
    assert!(
        member
            .dependencies
            .iter()
            .any(|dependency| dependency.source_kind == CargoDependencySourceKind::Git)
    );
    assert!(
        member
            .dependencies
            .iter()
            .any(|dependency| dependency.source_kind == CargoDependencySourceKind::Registry)
    );

    let second = workspace_from_metadata(&metadata, |path| resolve(Path::new("/workspace"), path))
        .expect("a second conversion should succeed");
    assert_eq!(model, second);
}

#[test]
fn standalone_fixture_is_a_non_virtual_single_member_workspace() {
    let metadata = parse_metadata(include_bytes!(
        "../../test_data/cargo_workspace/standalone-v1.json"
    ))
    .expect("standalone fixture should parse");
    let model = workspace_from_metadata(&metadata, |path| resolve(Path::new("/standalone"), path))
        .expect("standalone fixture should convert");
    assert!(!model.is_virtual);
    assert_eq!(model.members.len(), 1);
    assert_eq!(model.members[0].name, "standalone");
}

#[test]
fn metadata_parser_rejects_unsupported_and_malformed_documents() {
    assert!(parse_metadata(br#"{"version":2}"#).is_err());
    assert!(parse_metadata(br#"not-json"#).is_err());
}

#[test]
fn cargo_workspace_configuration_parsers_are_bounded_and_fallible() {
    let profiles = parse_cargo_profiles(include_str!(
        "../../test_data/cargo_workspace/profiles-custom.toml"
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
            "../../test_data/cargo_workspace/profiles-malformed.toml"
        ))
        .is_err()
    );

    let declared = parse_rust_toolchain(
        fixture_path("rust-toolchain.toml"),
        include_str!("../../test_data/cargo_workspace/rust-toolchain.toml"),
    )
    .expect("TOML toolchain fixture should parse");
    assert_eq!(declared.format, CargoToolchainFormat::Toml);
    assert_eq!(declared.channel.as_deref(), Some("stable"));
    assert_eq!(declared.components, ["rustfmt", "clippy"]);
    assert_eq!(declared.targets, ["wasm32-unknown-unknown"]);
    let legacy = parse_rust_toolchain(
        fixture_path("rust-toolchain"),
        include_str!("../../test_data/cargo_workspace/rust-toolchain-legacy"),
    )
    .expect("legacy toolchain fixture should parse");
    assert_eq!(legacy.format, CargoToolchainFormat::Legacy);
    assert_eq!(legacy.channel.as_deref(), Some("1.90.0"));

    let compiler = parse_rustc_verbose_version(include_bytes!(
        "../../test_data/cargo_workspace/rustc-vv.txt"
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
fn cargo_workspace_configuration_retains_only_safe_stale_facts() {
    let mut previous = CargoWorkspaceConfiguration::unresolved();
    previous.host_compiler = CargoHostCompilerModel {
        status: CargoHostCompilerStatus::Available,
        release: Some("1.90.0".to_string()),
        host_target: Some("x86_64-unknown-linux-gnu".to_string()),
        stale: false,
    };
    previous.declared_toolchain = Some(
        parse_rust_toolchain(
            fixture_path("rust-toolchain.toml"),
            include_str!("../../test_data/cargo_workspace/rust-toolchain.toml"),
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

struct FixtureRunner {
    output: Vec<u8>,
    requests: parking_lot::Mutex<Vec<CargoMetadataRequest>>,
}

struct FixtureConfigurationProbe {
    output: Result<Vec<u8>, String>,
    requests: parking_lot::Mutex<Vec<CargoConfigurationProbeRequest>>,
}

#[async_trait]
impl CargoConfigurationProbe for FixtureConfigurationProbe {
    async fn run(&self, request: CargoConfigurationProbeRequest) -> Result<Vec<u8>> {
        self.requests.lock().push(request);
        self.output.clone().map_err(anyhow::Error::msg)
    }
}

#[async_trait]
impl CargoMetadataRunner for FixtureRunner {
    async fn run(&self, request: CargoMetadataRequest) -> Result<Vec<u8>> {
        self.requests.lock().push(request);
        Ok(self.output.clone())
    }
}

#[gpui::test]
async fn cargo_workspace_store_uses_an_injected_runner(cx: &mut TestAppContext) {
    crate::init_test(cx);
    let fs = FakeFs::new(cx.executor());
    fs.insert_tree(
        "/root",
        json!({
            "Cargo.toml": include_str!("../../test_data/cargo_workspace/profiles-custom.toml"),
            "rust-toolchain.toml": include_str!("../../test_data/cargo_workspace/rust-toolchain.toml"),
            ".cargo": { "config.toml": "[build]\ntarget = \"wasm32-unknown-unknown\"" }
        }),
    )
    .await;
    let project = Project::test(fs, [Path::new("/root")], cx).await;
    let output = String::from_utf8_lossy(include_bytes!(
        "../../test_data/cargo_workspace/workspace-v1.json"
    ))
    .replace("/workspace", "/root")
    .into_bytes();
    let runner = Arc::new(FixtureRunner {
        output,
        requests: parking_lot::Mutex::new(Vec::new()),
    });
    let configuration_probe = Arc::new(FixtureConfigurationProbe {
        output: Ok(include_bytes!("../../test_data/cargo_workspace/rustc-vv.txt").to_vec()),
        requests: parking_lot::Mutex::new(Vec::new()),
    });
    let (worktree_store, environment) = cx.update(|cx| {
        let project = project.read(cx);
        (project.worktree_store(), project.environment().clone())
    });
    let store = cx.new({
        let runner = runner.clone();
        let configuration_probe = configuration_probe.clone();
        move |_| {
            CargoWorkspaceStore::local_with_runners(
                worktree_store,
                environment,
                runner,
                configuration_probe,
            )
        }
    });
    let snapshot = store
        .update(cx, |store, cx| store.refresh(cx))
        .await
        .expect("fixture runner should produce a snapshot");
    assert_eq!(snapshot.workspaces.len(), 1);
    assert!(snapshot.failures.is_empty());
    let configuration = &snapshot.workspaces[0].configuration;
    assert_eq!(
        configuration.completeness,
        CargoConfigurationCompleteness::Complete
    );
    assert_eq!(configuration.profiles.len(), 4);
    assert_eq!(
        configuration
            .declared_toolchain
            .as_ref()
            .and_then(|toolchain| toolchain.channel.as_deref()),
        Some("stable")
    );
    assert_eq!(
        configuration.host_compiler.host_target.as_deref(),
        Some("x86_64-unknown-linux-gnu")
    );
    let requests = runner.requests.lock();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].manifest_path, Path::new("/root/Cargo.toml"));
    assert_eq!(requests[0].working_directory, Path::new("/root"));
    let configuration_requests = configuration_probe.requests.lock();
    assert_eq!(configuration_requests.len(), 1);
    assert_eq!(
        configuration_requests[0].working_directory,
        Path::new("/root")
    );
}

#[gpui::test]
async fn cargo_workspace_restricted_store_runs_no_injected_commands(cx: &mut TestAppContext) {
    crate::init_test(cx);
    let fs = FakeFs::new(cx.executor());
    fs.insert_tree("/restricted", json!({ "Cargo.toml": "[workspace]" }))
        .await;
    let project = Project::test(fs, [Path::new("/restricted")], cx).await;
    let runner = Arc::new(FixtureRunner {
        output: include_bytes!("../../test_data/cargo_workspace/workspace-v1.json").to_vec(),
        requests: parking_lot::Mutex::new(Vec::new()),
    });
    let configuration_probe = Arc::new(FixtureConfigurationProbe {
        output: Ok(include_bytes!("../../test_data/cargo_workspace/rustc-vv.txt").to_vec()),
        requests: parking_lot::Mutex::new(Vec::new()),
    });
    let (worktree_store, environment) = cx.update(|cx| {
        let project = project.read(cx);
        (project.worktree_store(), project.environment().clone())
    });
    cx.update(|cx| {
        if cx.try_global::<TrustedWorktrees>().is_some() {
            cx.remove_global::<TrustedWorktrees>();
        }
        trusted_worktrees::init(DbTrustedPaths::default(), cx);
        trusted_worktrees::track_worktree_trust(worktree_store.clone(), None, None, None, cx);
    });
    let store = cx.new({
        let runner = runner.clone();
        let configuration_probe = configuration_probe.clone();
        move |_| {
            CargoWorkspaceStore::local_with_runners(
                worktree_store,
                environment,
                runner,
                configuration_probe,
            )
        }
    });
    let snapshot = store
        .update(cx, |store, cx| store.refresh(cx))
        .await
        .expect("restricted refresh should return a scoped failure");
    assert!(snapshot.workspaces.is_empty());
    assert_eq!(snapshot.failures.len(), 1);
    assert!(runner.requests.lock().is_empty());
    assert!(configuration_probe.requests.lock().is_empty());
}
