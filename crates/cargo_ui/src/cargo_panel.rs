use std::{collections::HashMap, time::Duration};

use db::kvp::KeyValueStore;
use gpui::{
    Action, Anchor, App, AsyncWindowContext, Context, DismissEvent, Entity, EventEmitter,
    FocusHandle, Focusable, IntoElement, Pixels, Point, Render, ScrollStrategy, Subscription,
    TaskExt as _, WeakEntity, Window, actions, anchored, deferred, div,
};
use language_tools::language_tool_tree::{
    self, LanguageToolNode, LanguageToolNodeId, LanguageToolProviderStatus, LanguageToolSnapshot,
    LanguageToolTreeHost, LanguageToolTreeStatus, language_tool_tree, status_message,
};
use project::{
    Project, ProjectPath,
    cargo_workspace::{
        CargoConfigurationDiagnosticCategory, CargoDependencyKind, CargoDependencySourceKind,
        CargoFeatureEnabled, CargoHostCompilerStatus, CargoPackageModel, CargoProfileOrigin,
        CargoSnapshotCompleteness, CargoTargetConfiguration, CargoTargetKind, CargoToolchainFormat,
        CargoWorkspaceConfiguration, CargoWorkspaceSnapshot,
    },
    cargo_workspace_store::{CargoWorkspaceRemoteError, CargoWorkspaceRemoteErrorKind},
};
use settings::{DockSide, Settings, SettingsStore};
use ui::{ContextMenu, IconName, Tooltip, prelude::*};
use workspace::{
    Panel, Workspace,
    dock::{DockPosition, PanelEvent},
    workspace_scoped_state_key,
};

use crate::{
    BenchSelected, BuildSelected, CargoAction, CargoActionNodeKind, CargoActionRuntime,
    CargoActionSelection, CargoActionTargetKind, CargoPanelSettings, CargoPresetScope,
    CargoPresetSettings, CargoPresetWorkspaceState, CargoSafeSelectionState, CheckSelected,
    DebugSelected, RunSelected, TestSelected, WorkspaceCargoActionDispatcher,
    cargo_action_availability, dispatch_cargo_action, plan_cargo_action, recover_workspace_state,
};

actions!(cargo_panel, [ToggleCargoPanel]);

const CARGO_PANEL_KEY: &str = "CargoPanel";
const CARGO_PRESET_STATE_KEY: &str = "cargo-preset-state-v1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoActiveConfiguration {
    pub scope: String,
    pub profile: Option<String>,
    pub selected_features: Vec<String>,
    pub default_features: Option<bool>,
    pub explicit_target: Option<String>,
    pub target_selector: Option<String>,
    pub environment_keys: Vec<String>,
    pub notice: Option<String>,
}

impl Default for CargoActiveConfiguration {
    fn default() -> Self {
        Self {
            scope: "selected workspace or package".to_string(),
            profile: None,
            selected_features: Vec::new(),
            default_features: None,
            explicit_target: None,
            target_selector: None,
            environment_keys: Vec::new(),
            notice: None,
        }
    }
}

pub struct CargoTreeProvider {
    navigation: HashMap<LanguageToolNodeId, ProjectPath>,
    action_selections: HashMap<LanguageToolNodeId, CargoActionSelection>,
}

impl CargoTreeProvider {
    pub fn project(snapshot: &CargoWorkspaceSnapshot) -> (LanguageToolSnapshot, Self) {
        Self::project_with_configuration(snapshot, &CargoActiveConfiguration::default())
    }

    pub fn project_with_configuration(
        snapshot: &CargoWorkspaceSnapshot,
        active_configuration: &CargoActiveConfiguration,
    ) -> (LanguageToolSnapshot, Self) {
        let mut navigation = HashMap::new();
        let mut action_selections = HashMap::new();
        let mut roots = Vec::new();
        for workspace in &snapshot.workspaces {
            let workspace_id = id(&format!(
                "workspace:{}:{}",
                workspace.key.root.worktree_id.to_proto(),
                workspace.key.root.path.as_unix_str()
            ));
            if let Some(path) = workspace.root_manifest.clone() {
                navigation.insert(workspace_id.clone(), path);
            }
            if let Some(workspace_manifest) = workspace.root_manifest.clone() {
                action_selections.insert(
                    workspace_id.clone(),
                    CargoActionSelection {
                        node_kind: CargoActionNodeKind::Workspace,
                        worktree_id: workspace_manifest.worktree_id,
                        workspace_name: workspace.display_name.clone(),
                        workspace_manifest,
                        package_name: None,
                        package_manifest: None,
                        target: None,
                        has_bench_targets: workspace.members.iter().any(|package| {
                            package
                                .targets
                                .iter()
                                .any(|target| target.kind == CargoTargetKind::Bench)
                        }),
                    },
                );
            }
            let mut members = vec![configuration_node(
                &workspace_id,
                &workspace.configuration,
                workspace.root_manifest.as_ref(),
                active_configuration,
                &mut navigation,
            )];
            for package in &workspace.members {
                if let Some(workspace_manifest) = workspace.root_manifest.as_ref() {
                    members.push(package_node(
                        &workspace_id,
                        &workspace.display_name,
                        workspace_manifest,
                        package,
                        &mut navigation,
                        &mut action_selections,
                    ));
                } else {
                    members.push(package_node(
                        &workspace_id,
                        &workspace.display_name,
                        &package.manifest_path,
                        package,
                        &mut navigation,
                        &mut action_selections,
                    ));
                }
            }
            roots.push(LanguageToolNode {
                id: workspace_id,
                label: workspace.display_name.clone(),
                secondary_label: Some(workspace.key.root.path.as_unix_str().to_string()),
                icon: Some(IconName::FileRust),
                accessibility_label: format!("Cargo workspace {}", workspace.display_name),
                children: members,
                enabled: workspace.root_manifest.is_some(),
                activation_label: workspace
                    .root_manifest
                    .as_ref()
                    .map(|_| "Open Cargo.toml".to_string()),
            });
        }
        for failure in &snapshot.failures {
            let failure_label = match failure.category {
                project::cargo_workspace::CargoWorkspaceErrorCategory::Restricted => {
                    "Cargo access restricted"
                }
                project::cargo_workspace::CargoWorkspaceErrorCategory::CargoNotFound => {
                    "Cargo not found"
                }
                project::cargo_workspace::CargoWorkspaceErrorCategory::UnsupportedMetadata => {
                    "Unsupported Cargo metadata"
                }
                project::cargo_workspace::CargoWorkspaceErrorCategory::Disconnected => {
                    "Cargo host disconnected"
                }
                project::cargo_workspace::CargoWorkspaceErrorCategory::Cancelled => {
                    "Cargo refresh cancelled"
                }
                project::cargo_workspace::CargoWorkspaceErrorCategory::CargoFailed
                | project::cargo_workspace::CargoWorkspaceErrorCategory::InvalidMetadata
                | project::cargo_workspace::CargoWorkspaceErrorCategory::Internal => {
                    "Cargo metadata error"
                }
            };
            roots.push(LanguageToolNode {
                id: id(&format!(
                    "failure:{}:{}",
                    failure.manifest_path.worktree_id.to_proto(),
                    failure.manifest_path.path.as_unix_str()
                )),
                label: failure_label.to_string(),
                secondary_label: Some(format!(
                    "{} · {}",
                    failure.manifest_path.path.as_unix_str(),
                    failure.message
                )),
                icon: Some(IconName::CircleHelp),
                accessibility_label: format!("Cargo metadata error: {}", failure.message),
                children: Vec::new(),
                enabled: false,
                activation_label: None,
            });
        }
        let status = if roots.is_empty() && snapshot.failures.is_empty() {
            LanguageToolProviderStatus::Empty(
                "No Cargo.toml files were found in visible worktrees.".to_string(),
            )
        } else if snapshot.workspaces.is_empty()
            && !snapshot.failures.is_empty()
            && snapshot.failures.iter().all(|failure| {
                failure.category
                    == project::cargo_workspace::CargoWorkspaceErrorCategory::Restricted
            })
        {
            LanguageToolProviderStatus::Restricted(
                "Cargo metadata is disabled for restricted worktrees. Trust the worktree and refresh."
                    .to_string(),
            )
        } else if snapshot.completeness == CargoSnapshotCompleteness::Partial {
            let message = if snapshot.failures.is_empty()
                && snapshot.workspaces.iter().any(|workspace| {
                    workspace.configuration.completeness
                        == project::cargo_workspace::CargoConfigurationCompleteness::Partial
                }) {
                "Cargo metadata loaded, but some configuration facts are unavailable. Refresh to retry."
            } else {
                "Some Cargo workspaces could not be loaded. Refresh to retry."
            };
            LanguageToolProviderStatus::Partial(message.to_string())
        } else {
            LanguageToolProviderStatus::Current
        };
        (
            LanguageToolSnapshot { roots, status },
            Self {
                navigation,
                action_selections,
            },
        )
    }

    pub fn navigation(&self, id: &LanguageToolNodeId) -> Option<&ProjectPath> {
        self.navigation.get(id)
    }

    pub fn action_selection(&self, id: &LanguageToolNodeId) -> Option<&CargoActionSelection> {
        self.action_selections.get(id)
    }
}

fn configuration_node(
    workspace_id: &LanguageToolNodeId,
    configuration: &CargoWorkspaceConfiguration,
    root_manifest: Option<&ProjectPath>,
    active: &CargoActiveConfiguration,
    navigation: &mut HashMap<LanguageToolNodeId, ProjectPath>,
) -> LanguageToolNode {
    let configuration_id = id(&format!("{}:section:configuration", workspace_id.0));
    let active_id = id(&format!("{}:active", configuration_id.0));
    let mut environment_keys = active.environment_keys.clone();
    environment_keys.sort();
    environment_keys.dedup();
    let selected_features = if active.selected_features.is_empty() {
        "Cargo default".to_string()
    } else {
        active.selected_features.join(", ")
    };
    let environment_key_summary = if environment_keys.is_empty() {
        "none".to_string()
    } else {
        environment_keys.join(", ")
    };
    let active_children = vec![
        configuration_leaf(
            &active_id,
            "scope",
            "Scope",
            &active.scope,
            None,
            navigation,
        ),
        configuration_leaf(
            &active_id,
            "profile",
            "Profile",
            active.profile.as_deref().unwrap_or("Cargo default"),
            None,
            navigation,
        ),
        configuration_leaf(
            &active_id,
            "features",
            "Features",
            &selected_features,
            None,
            navigation,
        ),
        configuration_leaf(
            &active_id,
            "default-features",
            "Default features",
            match active.default_features {
                Some(true) => "enabled",
                Some(false) => "disabled",
                None => "Cargo default",
            },
            None,
            navigation,
        ),
        configuration_leaf(
            &active_id,
            "explicit-target",
            "Explicit target",
            active.explicit_target.as_deref().unwrap_or("none"),
            None,
            navigation,
        ),
        configuration_leaf(
            &active_id,
            "target-selector",
            "Target selector",
            active.target_selector.as_deref().unwrap_or("Cargo default"),
            None,
            navigation,
        ),
        configuration_leaf(
            &active_id,
            "environment-keys",
            "Environment keys",
            &environment_key_summary,
            None,
            navigation,
        ),
    ];
    let mut active_children = active_children;
    if let Some(notice) = active.notice.as_deref() {
        active_children.push(configuration_leaf(
            &active_id,
            "preset-notice",
            "Preset notice",
            notice,
            None,
            navigation,
        ));
    }
    let active_summary = [
        active
            .profile
            .as_ref()
            .map(|profile| format!("profile {profile}"))
            .unwrap_or_else(|| "Cargo default profile".to_string()),
        active
            .explicit_target
            .as_ref()
            .map(|target| format!("target {target}"))
            .unwrap_or_else(|| "no explicit target".to_string()),
    ]
    .join(" · ");
    let mut children = vec![LanguageToolNode {
        id: active_id,
        label: "Active configuration".to_string(),
        secondary_label: Some(active_summary.clone()),
        icon: Some(IconName::Settings),
        accessibility_label: format!("Cargo active configuration, {active_summary}"),
        children: active_children,
        enabled: false,
        activation_label: None,
    }];

    let profiles_id = id(&format!("{}:profiles", configuration_id.0));
    let profile_children = configuration
        .profiles
        .iter()
        .map(|profile| {
            configuration_leaf(
                &profiles_id,
                &profile.name,
                &profile.name,
                match profile.origin {
                    CargoProfileOrigin::Implicit => "implicit Cargo profile",
                    CargoProfileOrigin::Declared => "declared in Cargo.toml",
                },
                root_manifest.cloned(),
                navigation,
            )
        })
        .collect::<Vec<_>>();
    children.push(LanguageToolNode {
        id: profiles_id,
        label: "Profiles".to_string(),
        secondary_label: Some(configuration.profiles.len().to_string()),
        icon: None,
        accessibility_label: format!("Cargo profiles, {} entries", configuration.profiles.len()),
        children: profile_children,
        enabled: false,
        activation_label: None,
    });

    let toolchain_id = id(&format!("{}:toolchain", configuration_id.0));
    if let Some(toolchain) = &configuration.declared_toolchain {
        let toolchain_format = match toolchain.format {
            CargoToolchainFormat::Toml => "rust-toolchain.toml",
            CargoToolchainFormat::Legacy => "legacy rust-toolchain",
        };
        let components = if toolchain.components.is_empty() {
            "none declared".to_string()
        } else {
            toolchain.components.join(", ")
        };
        let targets = if toolchain.targets.is_empty() {
            "none declared".to_string()
        } else {
            toolchain.targets.join(", ")
        };
        let mut toolchain_children = vec![configuration_leaf(
            &toolchain_id,
            "channel",
            "Channel",
            toolchain.channel.as_deref().unwrap_or("not declared"),
            Some(toolchain.source_path.clone()),
            navigation,
        )];
        toolchain_children.push(configuration_leaf(
            &toolchain_id,
            "components",
            "Components",
            &components,
            Some(toolchain.source_path.clone()),
            navigation,
        ));
        toolchain_children.push(configuration_leaf(
            &toolchain_id,
            "targets",
            "Declared targets",
            &targets,
            Some(toolchain.source_path.clone()),
            navigation,
        ));
        children.push(LanguageToolNode {
            id: toolchain_id,
            label: "Declared toolchain".to_string(),
            secondary_label: Some(
                toolchain
                    .channel
                    .clone()
                    .unwrap_or_else(|| toolchain_format.to_string()),
            ),
            icon: Some(IconName::Settings),
            accessibility_label: format!("Declared Cargo toolchain, {toolchain_format}"),
            children: toolchain_children,
            enabled: true,
            activation_label: Some("Open toolchain declaration".to_string()),
        });
        navigation.insert(
            id(&format!("{}:toolchain", configuration_id.0)),
            toolchain.source_path.clone(),
        );
    } else {
        children.push(configuration_leaf(
            &configuration_id,
            "toolchain",
            "Declared toolchain",
            "none",
            None,
            navigation,
        ));
    }

    let host_status = match configuration.host_compiler.status {
        CargoHostCompilerStatus::Available => match (
            configuration.host_compiler.release.as_deref(),
            configuration.host_compiler.host_target.as_deref(),
        ) {
            (Some(release), Some(target)) => format!("rustc {release} · {target}"),
            (Some(release), None) => format!("rustc {release} · host target unknown"),
            (None, Some(target)) => format!("release unknown · {target}"),
            (None, None) => "available · details unknown".to_string(),
        },
        CargoHostCompilerStatus::Restricted => "restricted".to_string(),
        CargoHostCompilerStatus::Missing => "rustc not found".to_string(),
        CargoHostCompilerStatus::Failed => "probe failed".to_string(),
        CargoHostCompilerStatus::Unknown => "unknown".to_string(),
    };
    let host_status = if configuration.host_compiler.stale {
        format!("{host_status} · stale")
    } else {
        host_status
    };
    children.push(configuration_leaf(
        &configuration_id,
        "host-compiler",
        "Host compiler",
        &host_status,
        None,
        navigation,
    ));
    let cargo_target = match configuration.cargo_target {
        CargoTargetConfiguration::UnresolvedCargoDefault => "unresolved Cargo default",
    };
    children.push(configuration_leaf(
        &configuration_id,
        "cargo-target",
        "Cargo target resolution",
        cargo_target,
        None,
        navigation,
    ));

    if !configuration.diagnostics.is_empty() {
        let diagnostics_id = id(&format!("{}:diagnostics", configuration_id.0));
        let diagnostics = configuration
            .diagnostics
            .iter()
            .enumerate()
            .map(|(index, diagnostic)| {
                configuration_leaf(
                    &diagnostics_id,
                    &format!(
                        "{}-{index}",
                        configuration_diagnostic_label(diagnostic.category)
                    ),
                    configuration_diagnostic_label(diagnostic.category),
                    &diagnostic.message,
                    diagnostic.source_path.clone(),
                    navigation,
                )
            })
            .collect();
        children.push(LanguageToolNode {
            id: diagnostics_id,
            label: "Configuration diagnostics".to_string(),
            secondary_label: Some(configuration.diagnostics.len().to_string()),
            icon: Some(IconName::CircleHelp),
            accessibility_label: format!(
                "Cargo configuration diagnostics, {} entries",
                configuration.diagnostics.len()
            ),
            children: diagnostics,
            enabled: false,
            activation_label: None,
        });
    }

    LanguageToolNode {
        id: configuration_id,
        label: "Configuration".to_string(),
        secondary_label: Some(active_summary),
        icon: Some(IconName::Settings),
        accessibility_label: "Cargo workspace configuration".to_string(),
        children,
        enabled: false,
        activation_label: None,
    }
}

fn configuration_leaf(
    parent: &LanguageToolNodeId,
    discriminator: &str,
    label: &str,
    value: &str,
    navigation_path: Option<ProjectPath>,
    navigation: &mut HashMap<LanguageToolNodeId, ProjectPath>,
) -> LanguageToolNode {
    let node_id = id(&format!("{}:{discriminator}", parent.0));
    if let Some(path) = navigation_path.clone() {
        navigation.insert(node_id.clone(), path);
    }
    LanguageToolNode {
        id: node_id,
        label: label.to_string(),
        secondary_label: Some(value.to_string()),
        icon: None,
        accessibility_label: format!("Cargo configuration {label}, {value}"),
        children: Vec::new(),
        enabled: navigation_path.is_some(),
        activation_label: navigation_path.map(|_| "Open configuration source".to_string()),
    }
}

fn configuration_diagnostic_label(category: CargoConfigurationDiagnosticCategory) -> &'static str {
    match category {
        CargoConfigurationDiagnosticCategory::Manifest => "Manifest",
        CargoConfigurationDiagnosticCategory::Toolchain => "Toolchain",
        CargoConfigurationDiagnosticCategory::CompilerProbe => "Compiler probe",
    }
}

fn package_node(
    workspace_id: &LanguageToolNodeId,
    workspace_name: &str,
    workspace_manifest: &ProjectPath,
    package: &CargoPackageModel,
    navigation: &mut HashMap<LanguageToolNodeId, ProjectPath>,
    action_selections: &mut HashMap<LanguageToolNodeId, CargoActionSelection>,
) -> LanguageToolNode {
    let package_id = id(&format!(
        "{}:member:{}",
        workspace_id.0,
        package.manifest_path.path.as_unix_str()
    ));
    navigation.insert(package_id.clone(), package.manifest_path.clone());
    let has_bench_targets = package
        .targets
        .iter()
        .any(|target| target.kind == CargoTargetKind::Bench);
    action_selections.insert(
        package_id.clone(),
        CargoActionSelection {
            node_kind: CargoActionNodeKind::Package,
            worktree_id: package.manifest_path.worktree_id,
            workspace_name: workspace_name.to_string(),
            workspace_manifest: workspace_manifest.clone(),
            package_name: Some(package.name.clone()),
            package_manifest: Some(package.manifest_path.clone()),
            target: None,
            has_bench_targets,
        },
    );
    let mut sections = Vec::new();
    if !package.targets.is_empty() {
        let children = package
            .targets
            .iter()
            .map(|target| {
                let target_id = id(&format!(
                    "{}:target:{:?}:{}:{}",
                    package_id.0,
                    target.kind,
                    target.name,
                    target.source_display_path.as_deref().unwrap_or("")
                ));
                if let Some(path) = target.source_path.clone() {
                    navigation.insert(target_id.clone(), path);
                }
                action_selections.insert(
                    target_id.clone(),
                    CargoActionSelection {
                        node_kind: CargoActionNodeKind::Target(action_target_kind(&target.kind)),
                        worktree_id: package.manifest_path.worktree_id,
                        workspace_name: workspace_name.to_string(),
                        workspace_manifest: workspace_manifest.clone(),
                        package_name: Some(package.name.clone()),
                        package_manifest: Some(package.manifest_path.clone()),
                        target: target_selector(&target.kind, &target.name),
                        has_bench_targets: target.kind == CargoTargetKind::Bench,
                    },
                );
                LanguageToolNode {
                    id: target_id,
                    label: format!("{} {}", target_kind_label(&target.kind), target.name),
                    secondary_label: Some(
                        [
                            target.source_display_path.clone(),
                            (!target.crate_types.is_empty()).then(|| target.crate_types.join(", ")),
                            (!target.required_features.is_empty()).then(|| {
                                format!("requires {}", target.required_features.join(", "))
                            }),
                            Some(format!("edition {}", target.edition)),
                        ]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>()
                        .join(" · "),
                    ),
                    icon: Some(IconName::FileRust),
                    accessibility_label: format!(
                        "Cargo target {} {}",
                        target_kind_label(&target.kind),
                        target.name
                    ),
                    children: Vec::new(),
                    enabled: target.source_path.is_some(),
                    activation_label: target
                        .source_path
                        .as_ref()
                        .map(|_| "Open target source".to_string()),
                }
            })
            .collect();
        sections.push(section(&package_id, "targets", "Targets", children));
    }
    if !package.features.is_empty() {
        let children = package
            .features
            .iter()
            .map(|feature| {
                let feature_id = id(&format!("{}:feature:{}", package_id.0, feature.name));
                navigation.insert(feature_id.clone(), package.manifest_path.clone());
                let enabled = match feature.enabled {
                    CargoFeatureEnabled::Enabled if feature.defined => "enabled",
                    CargoFeatureEnabled::Enabled => "enabled implicit",
                    CargoFeatureEnabled::Disabled => "disabled",
                    CargoFeatureEnabled::Unknown => "unknown",
                };
                LanguageToolNode {
                    id: feature_id,
                    label: feature.name.clone(),
                    secondary_label: Some(enabled.to_string()),
                    icon: None,
                    accessibility_label: format!("Cargo feature {}, {enabled}", feature.name),
                    children: Vec::new(),
                    enabled: true,
                    activation_label: Some("Open Cargo.toml".to_string()),
                }
            })
            .collect();
        sections.push(section(
            &package_id,
            "features",
            "Features — Cargo default resolution",
            children,
        ));
    }
    if !package.dependencies.is_empty() {
        let mut groups = Vec::new();
        for (kind, label) in [
            (CargoDependencyKind::Normal, "Normal"),
            (CargoDependencyKind::Development, "Development"),
            (CargoDependencyKind::Build, "Build"),
            (CargoDependencyKind::Unknown, "Other"),
        ] {
            let children = package
                .dependencies
                .iter()
                .enumerate()
                .filter(|(_, dependency)| dependency.kind == kind)
                .map(|(index, dependency)| {
                    let display_name = dependency
                        .rename
                        .as_deref()
                        .unwrap_or(&dependency.declaration_name);
                    let dependency_id = id(&format!(
                        "{}:dependency:{kind:?}:{display_name}:{}:{}:{index}",
                        package_id.0,
                        dependency.target.as_deref().unwrap_or(""),
                        dependency.version_requirement
                    ));
                    let navigation_path = dependency
                        .resolved_workspace_member
                        .clone()
                        .or_else(|| dependency.local_manifest.clone())
                        .unwrap_or_else(|| package.manifest_path.clone());
                    navigation.insert(dependency_id.clone(), navigation_path);
                    let mut annotations = vec![dependency.version_requirement.clone()];
                    annotations.push(
                        match dependency.source_kind {
                            CargoDependencySourceKind::Path => "path",
                            CargoDependencySourceKind::Registry => "registry",
                            CargoDependencySourceKind::Git => "git",
                            CargoDependencySourceKind::Other => "other",
                        }
                        .to_string(),
                    );
                    if dependency.optional {
                        annotations.push("optional".to_string());
                    }
                    if !dependency.uses_default_features {
                        annotations.push("default features off".to_string());
                    }
                    if !dependency.requested_features.is_empty() {
                        annotations.push(format!(
                            "features {}",
                            dependency.requested_features.join(", ")
                        ));
                    }
                    if let Some(target) = &dependency.target {
                        annotations.push(target.clone());
                    }
                    if let Some(version) = &dependency.resolved_version {
                        annotations.push(format!("resolved {version}"));
                    }
                    LanguageToolNode {
                        id: dependency_id,
                        label: display_name.to_string(),
                        secondary_label: Some(annotations.join(" · ")),
                        icon: None,
                        accessibility_label: format!("Cargo dependency {display_name}"),
                        children: Vec::new(),
                        enabled: true,
                        activation_label: Some("Open relevant Cargo.toml".to_string()),
                    }
                })
                .collect::<Vec<_>>();
            if !children.is_empty() {
                groups.push(section(
                    &package_id,
                    &format!("dependency-{label}"),
                    label,
                    children,
                ));
            }
        }
        sections.push(section(&package_id, "dependencies", "Dependencies", groups));
    }
    LanguageToolNode {
        id: package_id,
        label: format!("{} {}", package.name, package.version),
        secondary_label: Some(package.manifest_path.path.as_unix_str().to_string()),
        icon: Some(IconName::Box),
        accessibility_label: format!("Cargo package {} {}", package.name, package.version),
        children: sections,
        enabled: true,
        activation_label: Some("Open Cargo.toml".to_string()),
    }
}

fn section(
    parent: &LanguageToolNodeId,
    discriminator: &str,
    label: &str,
    children: Vec<LanguageToolNode>,
) -> LanguageToolNode {
    LanguageToolNode {
        id: id(&format!("{}:section:{discriminator}", parent.0)),
        label: label.to_string(),
        secondary_label: None,
        icon: None,
        accessibility_label: label.to_string(),
        children,
        enabled: false,
        activation_label: None,
    }
}

fn target_kind_label(kind: &CargoTargetKind) -> &str {
    match kind {
        CargoTargetKind::Library => "lib",
        CargoTargetKind::Binary => "bin",
        CargoTargetKind::Example => "example",
        CargoTargetKind::Test => "test",
        CargoTargetKind::Bench => "bench",
        CargoTargetKind::BuildScript => "build script",
        CargoTargetKind::Other(value) => value,
    }
}

fn action_target_kind(kind: &CargoTargetKind) -> CargoActionTargetKind {
    match kind {
        CargoTargetKind::Library => CargoActionTargetKind::Library,
        CargoTargetKind::Binary => CargoActionTargetKind::Binary,
        CargoTargetKind::Example => CargoActionTargetKind::Example,
        CargoTargetKind::Test => CargoActionTargetKind::Test,
        CargoTargetKind::Bench => CargoActionTargetKind::Bench,
        CargoTargetKind::BuildScript => CargoActionTargetKind::BuildScript,
        CargoTargetKind::Other(_) => CargoActionTargetKind::Other,
    }
}

fn target_selector(kind: &CargoTargetKind, name: &str) -> Option<crate::CargoTargetSelector> {
    match kind {
        CargoTargetKind::Library => Some(crate::CargoTargetSelector::Library),
        CargoTargetKind::Binary => Some(crate::CargoTargetSelector::Binary(name.to_string())),
        CargoTargetKind::Example => Some(crate::CargoTargetSelector::Example(name.to_string())),
        CargoTargetKind::Test => Some(crate::CargoTargetSelector::Test(name.to_string())),
        CargoTargetKind::Bench => Some(crate::CargoTargetSelector::Bench(name.to_string())),
        CargoTargetKind::BuildScript | CargoTargetKind::Other(_) => None,
    }
}

fn id(value: &str) -> LanguageToolNodeId {
    LanguageToolNodeId(value.to_string())
}

fn retain_stale_workspaces(
    mut current: CargoWorkspaceSnapshot,
    previous: Option<&CargoWorkspaceSnapshot>,
) -> CargoWorkspaceSnapshot {
    let Some(previous) = previous else {
        return current;
    };
    for workspace in &mut current.workspaces {
        if let Some(previous_workspace) = previous
            .workspaces
            .iter()
            .find(|previous| previous.key == workspace.key)
        {
            workspace
                .configuration
                .retain_stale_safe_facts(&previous_workspace.configuration);
        }
    }
    for failure in &mut current.failures {
        let stale_workspace = previous.workspaces.iter().find(|workspace| {
            workspace.root_manifest.as_ref() == Some(&failure.manifest_path)
                || workspace
                    .members
                    .iter()
                    .any(|member| member.manifest_path == failure.manifest_path)
        });
        let Some(stale_workspace) = stale_workspace else {
            continue;
        };
        failure.has_stale_model = true;
        if !current
            .workspaces
            .iter()
            .any(|workspace| workspace.key == stale_workspace.key)
        {
            current.workspaces.push(stale_workspace.clone());
        }
    }
    current
        .workspaces
        .sort_by(|left, right| left.key.cmp(&right.key));
    current
}

fn active_configuration_from_preset_state(
    state: &CargoPresetWorkspaceState,
    settings: &CargoPresetSettings,
    notice: Option<String>,
) -> CargoActiveConfiguration {
    let Some(preset) = state
        .active_preset_id
        .as_ref()
        .and_then(|identifier| settings.presets.get(identifier))
    else {
        return CargoActiveConfiguration {
            notice,
            ..CargoActiveConfiguration::default()
        };
    };
    let scope = state.selection.scope.unwrap_or(preset.scope);
    let package = state.selection.package.as_ref().or(preset.package.as_ref());
    let target = state.selection.target.as_ref().or(preset.target.as_ref());
    CargoActiveConfiguration {
        scope: match (scope, package) {
            (CargoPresetScope::Workspace, _) => "workspace".to_string(),
            (CargoPresetScope::Package, Some(package)) => format!("package {package}"),
            (CargoPresetScope::Package, None) => "selected package".to_string(),
        },
        profile: preset.profile.clone(),
        selected_features: preset.features.clone(),
        default_features: preset.default_features,
        explicit_target: preset.target_triple.clone(),
        target_selector: target.map(|target| target.summary()),
        environment_keys: preset.environment_keys(),
        notice,
    }
}

fn persist_preset_state(key: String, state: CargoPresetWorkspaceState, cx: &App) -> gpui::Task<()> {
    let key_value_store = KeyValueStore::global(cx);
    cx.background_spawn(async move {
        let result = async {
            let serialized = serde_json::to_string(&state)?;
            key_value_store.write_kvp(key, serialized).await?;
            anyhow::Ok(())
        }
        .await;
        if let Err(error) = result {
            log::warn!("failed to persist Cargo preset state: {error}");
        }
    })
}

pub struct CargoPanel {
    project: Entity<Project>,
    workspace: WeakEntity<Workspace>,
    focus_handle: FocusHandle,
    active: bool,
    host: LanguageToolTreeHost,
    provider: CargoTreeProvider,
    active_configuration: CargoActiveConfiguration,
    preset_state: CargoPresetWorkspaceState,
    preset_serialization_key: Option<String>,
    preset_notice: Option<String>,
    pending_preset_serialization: Option<gpui::Task<()>>,
    last_snapshot: Option<CargoWorkspaceSnapshot>,
    execution_host_available: bool,
    context_menu: Option<(Entity<ContextMenu>, Point<Pixels>, Subscription)>,
    _subscriptions: Vec<Subscription>,
}

impl CargoPanel {
    pub async fn load(
        workspace: WeakEntity<Workspace>,
        mut cx: AsyncWindowContext,
    ) -> anyhow::Result<Entity<Self>> {
        let workspace_handle = workspace.clone();
        workspace.update_in(&mut cx, |workspace, _window, cx| {
            let project = workspace.project().clone();
            let preset_serialization_key = workspace_scoped_state_key(
                CARGO_PRESET_STATE_KEY,
                workspace.database_id(),
                workspace.session_id().as_deref(),
            );
            let preset_settings = CargoPresetSettings::get_global(cx);
            let serialized_preset_state = preset_serialization_key.as_ref().and_then(|key| {
                KeyValueStore::global(cx)
                    .read_kvp(key)
                    .map_err(|error| {
                        log::warn!("failed to restore Cargo preset state: {error}");
                        error
                    })
                    .ok()
                    .flatten()
            });
            let recovered = recover_workspace_state(
                serialized_preset_state.as_deref(),
                &preset_settings.presets,
            );
            let active_configuration = active_configuration_from_preset_state(
                &recovered.state,
                preset_settings,
                recovered.notice.clone(),
            );
            cx.new(|cx: &mut Context<CargoPanel>| {
                let store = project.read(cx).cargo_workspace_store().clone();
                let subscription = cx.subscribe(&store, |panel, _, _, cx| {
                    panel.invalidate(cx);
                });
                let focus_handle = cx.focus_handle();
                let settings_subscription = cx.observe_global::<SettingsStore>(|panel, cx| {
                    panel.reconcile_preset_settings(cx);
                });
                let pending_preset_serialization = if recovered.rewrite {
                    preset_serialization_key
                        .as_ref()
                        .map(|key| persist_preset_state(key.clone(), recovered.state.clone(), cx))
                } else {
                    None
                };
                CargoPanel {
                    project,
                    workspace: workspace_handle,
                    focus_handle: focus_handle.clone(),
                    active: false,
                    host: LanguageToolTreeHost::with_focus_handle(focus_handle),
                    provider: CargoTreeProvider {
                        navigation: HashMap::new(),
                        action_selections: HashMap::new(),
                    },
                    active_configuration,
                    preset_state: recovered.state,
                    preset_serialization_key,
                    preset_notice: recovered.notice,
                    pending_preset_serialization,
                    last_snapshot: None,
                    execution_host_available: false,
                    context_menu: None,
                    _subscriptions: vec![subscription, settings_subscription],
                }
            })
        })
    }

    pub fn toggle_focus(
        workspace: &mut Workspace,
        _: &ToggleCargoPanel,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        workspace.toggle_panel_focus::<Self>(window, cx);
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        self.host.cancel_debounce();
        let generation = self.host.start_refresh();
        let store = self.project.read(cx).cargo_workspace_store().clone();
        let task = store.update(cx, |store, cx| store.refresh(cx));
        let refresh_task = cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |panel, cx| {
                match result {
                    Ok(snapshot) => {
                        panel.execution_host_available = true;
                        let covers_current_inputs = snapshot.input_fingerprint
                            == store.read(cx).current_input_fingerprint(cx);
                        let snapshot =
                            retain_stale_workspaces(snapshot, panel.last_snapshot.as_ref());
                        panel.last_snapshot = Some(snapshot.clone());
                        let (snapshot, provider) = CargoTreeProvider::project_with_configuration(
                            &snapshot,
                            &panel.active_configuration,
                        );
                        panel.provider = provider;
                        panel.host.apply_refresh(generation, Ok(snapshot));
                        let was_dirty = panel.host.take_dirty();
                        if was_dirty && !covers_current_inputs {
                            panel.schedule_debounced_refresh(cx);
                        }
                    }
                    Err(error) => {
                        panel.execution_host_available = false;
                        if let Some(remote_error) =
                            error.downcast_ref::<CargoWorkspaceRemoteError>()
                        {
                            let status = match remote_error.kind {
                                CargoWorkspaceRemoteErrorKind::UnsupportedHost => {
                                    LanguageToolProviderStatus::Unsupported(
                                        remote_error.to_string(),
                                    )
                                }
                                CargoWorkspaceRemoteErrorKind::Disconnected => {
                                    LanguageToolProviderStatus::Disconnected(
                                        remote_error.to_string(),
                                    )
                                }
                            };
                            panel.host.apply_provider_error(generation, status);
                        } else {
                            panel.host.apply_refresh(generation, Err(error));
                        }
                        if panel.host.take_dirty() {
                            panel.schedule_debounced_refresh(cx);
                        }
                    }
                }
                cx.notify();
            })
            .ok();
        });
        self.host.replace_refresh_task(refresh_task);
        cx.notify();
    }

    fn invalidate(&mut self, cx: &mut Context<Self>) {
        if !self.active {
            self.host.mark_dirty();
            return;
        }
        if matches!(
            self.host.status(),
            LanguageToolTreeStatus::Loading | LanguageToolTreeStatus::Refreshing
        ) {
            self.host.mark_dirty();
            return;
        }
        self.schedule_debounced_refresh(cx);
    }

    fn reconcile_preset_settings(&mut self, cx: &mut Context<Self>) {
        let preset_settings = CargoPresetSettings::get_global(cx);
        let recovered = recover_workspace_state(
            serde_json::to_string(&self.preset_state).ok().as_deref(),
            &preset_settings.presets,
        );
        let changed = recovered.state != self.preset_state;
        self.preset_state = recovered.state;
        if recovered.notice.is_some() {
            self.preset_notice = recovered.notice;
        }
        self.active_configuration = active_configuration_from_preset_state(
            &self.preset_state,
            preset_settings,
            self.preset_notice.clone(),
        );
        if changed || recovered.rewrite {
            self.persist_preset_state(cx);
        }
        self.invalidate(cx);
        cx.notify();
    }

    pub fn set_active_preset(
        &mut self,
        active_preset_id: Option<String>,
        selection: CargoSafeSelectionState,
        cx: &mut Context<Self>,
    ) {
        self.preset_state.active_preset_id = active_preset_id;
        self.preset_state.selection = selection;
        self.preset_notice = None;
        self.reconcile_preset_settings(cx);
        self.persist_preset_state(cx);
    }

    fn persist_preset_state(&mut self, cx: &mut Context<Self>) {
        let Some(key) = self.preset_serialization_key.clone() else {
            return;
        };
        self.pending_preset_serialization =
            Some(persist_preset_state(key, self.preset_state.clone(), cx));
    }

    fn schedule_debounced_refresh(&mut self, cx: &mut Context<Self>) {
        let debounce_task = cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(150))
                .await;
            this.update(cx, |panel, cx| panel.refresh(cx)).ok();
        });
        self.host.replace_debounce_task(debounce_task);
    }

    fn activate_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.host.can_activate() {
            return;
        }
        let Some(path) = self
            .host
            .selected()
            .and_then(|id| self.provider.navigation(id))
            .cloned()
        else {
            return;
        };
        if let Some(workspace) = self.workspace.upgrade() {
            workspace
                .update(cx, |workspace, cx| {
                    workspace.open_path(path, None, true, window, cx)
                })
                .detach_and_log_err(cx);
        }
    }

    fn reveal_selection(&self, strategy: ScrollStrategy) {
        self.host.reveal_selection(strategy);
    }

    fn action_runtime(&self, selection: &CargoActionSelection, cx: &App) -> CargoActionRuntime {
        let project = self.project.read(cx);
        let matching_failure = self.last_snapshot.as_ref().and_then(|snapshot| {
            snapshot
                .failures
                .iter()
                .find(|failure| failure.manifest_path == selection.workspace_manifest)
        });
        CargoActionRuntime {
            trusted: !matching_failure.is_some_and(|failure| {
                failure.category
                    == project::cargo_workspace::CargoWorkspaceErrorCategory::Restricted
            }),
            connected: !project.is_disconnected(cx),
            writable: !project.is_read_only(cx),
            cargo_available: !matching_failure.is_some_and(|failure| {
                failure.category
                    == project::cargo_workspace::CargoWorkspaceErrorCategory::CargoNotFound
            }),
            host_capable: self.execution_host_available && project.supports_terminal(cx),
        }
    }

    fn set_action_notice(&mut self, notice: impl Into<String>, cx: &mut Context<Self>) {
        let notice = notice.into();
        self.preset_notice = Some(notice.clone());
        self.active_configuration.notice = Some(notice);
        if let Some(snapshot) = self.last_snapshot.as_ref() {
            let (tree, provider) =
                CargoTreeProvider::project_with_configuration(snapshot, &self.active_configuration);
            self.provider = provider;
            self.host.replace_snapshot(tree);
        }
        cx.notify();
    }

    fn execute_selected_action(
        &mut self,
        action: CargoAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(selection) = self
            .host
            .selected()
            .and_then(|id| self.provider.action_selection(id))
            .cloned()
        else {
            self.set_action_notice("Select a Cargo workspace, package, or target first", cx);
            return;
        };
        let runtime = self.action_runtime(&selection, cx);
        let Some(availability) = cargo_action_availability(&selection, runtime)
            .into_iter()
            .find(|availability| availability.action == action)
        else {
            self.set_action_notice("The selected Cargo action is unavailable", cx);
            return;
        };
        if !availability.enabled {
            self.set_action_notice(
                availability
                    .reason
                    .unwrap_or_else(|| "The selected Cargo action is unavailable".to_string()),
                cx,
            );
            return;
        }
        let preset = self
            .preset_state
            .active_preset_id
            .as_ref()
            .and_then(|identifier| CargoPresetSettings::get_global(cx).presets.get(identifier))
            .cloned();
        let Some(workspace) = self.workspace.upgrade() else {
            self.set_action_notice("The workspace is no longer available", cx);
            return;
        };
        let task_contexts = workspace.update(cx, |workspace, cx| {
            tasks_ui::task_contexts(workspace, window, cx)
        });
        let workspace_handle = workspace.downgrade();
        cx.spawn_in(window, async move |panel, cx| {
            let task_contexts = task_contexts.await;
            let result = async {
                let base_context = task_contexts
                    .task_context_for_worktree_id(selection.worktree_id)
                    .ok_or_else(|| {
                        anyhow::anyhow!("No task context exists for the selected worktree")
                    })?;
                let plan = plan_cargo_action(action, &selection, preset.as_ref(), base_context)?;
                workspace_handle.update_in(cx, |workspace, window, cx| {
                    let mut dispatcher = WorkspaceCargoActionDispatcher::new(workspace, window, cx);
                    dispatch_cargo_action(plan, &mut dispatcher);
                })?;
                anyhow::Ok(())
            }
            .await;
            if let Err(error) = result {
                panel
                    .update(cx, |panel, cx| {
                        panel.set_action_notice(format!("Cargo action failed: {error}"), cx)
                    })
                    .ok();
            }
        })
        .detach();
    }

    fn deploy_context_menu(
        &mut self,
        position: Point<Pixels>,
        id: LanguageToolNodeId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.host.select(id);
        let can_open = self.host.can_activate()
            && self
                .host
                .selected()
                .is_some_and(|id| self.provider.navigation(id).is_some());
        let action_entries = self
            .host
            .selected()
            .and_then(|id| self.provider.action_selection(id))
            .map(|selection| {
                cargo_action_availability(selection, self.action_runtime(selection, cx))
            })
            .unwrap_or_default();
        let menu = ContextMenu::build(window, cx, |mut menu, _, _| {
            menu = menu.context(self.focus_handle.clone());
            if !action_entries.is_empty() {
                menu = menu.header("Cargo actions");
                for availability in action_entries {
                    let label = availability.accessibility_label;
                    menu = match availability.action {
                        CargoAction::Build => menu.action_disabled_when(
                            !availability.enabled,
                            label,
                            Box::new(BuildSelected),
                        ),
                        CargoAction::Check => menu.action_disabled_when(
                            !availability.enabled,
                            label,
                            Box::new(CheckSelected),
                        ),
                        CargoAction::Run => menu.action_disabled_when(
                            !availability.enabled,
                            label,
                            Box::new(RunSelected),
                        ),
                        CargoAction::Test => menu.action_disabled_when(
                            !availability.enabled,
                            label,
                            Box::new(TestSelected),
                        ),
                        CargoAction::Bench => menu.action_disabled_when(
                            !availability.enabled,
                            label,
                            Box::new(BenchSelected),
                        ),
                        CargoAction::Debug => menu.action_disabled_when(
                            !availability.enabled,
                            label,
                            Box::new(DebugSelected),
                        ),
                    };
                }
                menu = menu.separator();
            }
            menu.when(can_open, |menu| {
                menu.action("Open", Box::new(language_tool_tree::ActivateSelected))
            })
            .action("Refresh", Box::new(language_tool_tree::Refresh))
            .separator()
            .action("Expand All", Box::new(language_tool_tree::ExpandAll))
            .action("Collapse All", Box::new(language_tool_tree::CollapseAll))
        });
        window.focus(&menu.focus_handle(cx), cx);
        let subscription = cx.subscribe(&menu, |panel, _, _: &DismissEvent, cx| {
            panel.context_menu.take();
            cx.notify();
        });
        self.context_menu = Some((menu, position, subscription));
        cx.notify();
    }
}

impl Panel for CargoPanel {
    fn persistent_name() -> &'static str {
        "Cargo"
    }

    fn panel_key() -> &'static str {
        CARGO_PANEL_KEY
    }

    fn position(&self, _: &Window, cx: &App) -> DockPosition {
        match CargoPanelSettings::get_global(cx).dock {
            DockSide::Left => DockPosition::Left,
            DockSide::Right => DockPosition::Right,
        }
    }

    fn position_is_valid(&self, position: DockPosition) -> bool {
        matches!(position, DockPosition::Left | DockPosition::Right)
    }

    fn set_position(&mut self, position: DockPosition, _: &mut Window, cx: &mut Context<Self>) {
        settings::update_settings_file(<dyn fs::Fs>::global(cx), cx, move |settings, _| {
            settings.cargo_panel.get_or_insert_default().dock = Some(match position {
                DockPosition::Right => DockSide::Right,
                DockPosition::Left | DockPosition::Bottom => DockSide::Left,
            });
        });
    }

    fn default_size(&self, _: &Window, cx: &App) -> Pixels {
        CargoPanelSettings::get_global(cx).default_width
    }

    fn icon(&self, _: &Window, cx: &App) -> Option<IconName> {
        CargoPanelSettings::get_global(cx)
            .button
            .then_some(IconName::FileRust)
    }

    fn icon_tooltip(&self, _: &Window, _: &App) -> Option<&'static str> {
        Some("Cargo")
    }

    fn toggle_action(&self) -> Box<dyn Action> {
        Box::new(ToggleCargoPanel)
    }

    fn starts_open(&self, _: &Window, cx: &App) -> bool {
        CargoPanelSettings::get_global(cx).starts_open
    }

    fn set_active(&mut self, active: bool, _: &mut Window, cx: &mut Context<Self>) {
        let was_active = self.active;
        self.active = active;
        if active && !was_active {
            if matches!(self.host.status(), LanguageToolTreeStatus::Dormant)
                || self.host.take_dirty()
            {
                self.refresh(cx);
            }
        }
    }

    fn activation_priority(&self) -> u32 {
        7
    }

    fn hide_button_setting(&self, _: &App) -> Option<workspace::HideStatusItem> {
        Some(workspace::HideStatusItem::new(|settings| {
            settings.cargo_panel.get_or_insert_default().button = Some(false);
        }))
    }
}

impl Focusable for CargoPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<PanelEvent> for CargoPanel {}

impl Render for CargoPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rows = self.host.visible_rows().to_vec();
        let selected = self.host.selected().cloned();
        let status = status_message(self.host.status());
        let can_expand_all = self.host.can_expand_all();
        let can_collapse_all = self.host.can_collapse_all();
        let can_refresh = self.host.can_refresh();
        let click_panel = cx.weak_entity();
        let toggle_panel = cx.weak_entity();
        let context_panel = cx.weak_entity();
        v_flex()
            .id("cargo-panel")
            .key_context("CargoPanel")
            .track_focus(&self.focus_handle)
            .size_full()
            .on_action(
                cx.listener(|panel, _: &language_tool_tree::Refresh, _, cx| panel.refresh(cx)),
            )
            .on_action(
                cx.listener(|panel, _: &language_tool_tree::ExpandAll, _, cx| {
                    panel.host.expand_all();
                    cx.notify();
                }),
            )
            .on_action(
                cx.listener(|panel, _: &language_tool_tree::CollapseAll, _, cx| {
                    panel.host.collapse_all();
                    cx.notify();
                }),
            )
            .on_action(
                cx.listener(|panel, _: &language_tool_tree::SelectNext, _, cx| {
                    panel.host.select_next();
                    panel.reveal_selection(ScrollStrategy::Center);
                    cx.notify();
                }),
            )
            .on_action(
                cx.listener(|panel, _: &language_tool_tree::SelectPrevious, _, cx| {
                    panel.host.select_previous();
                    panel.reveal_selection(ScrollStrategy::Center);
                    cx.notify();
                }),
            )
            .on_action(
                cx.listener(|panel, _: &language_tool_tree::SelectFirst, _, cx| {
                    panel.host.select_first();
                    panel.reveal_selection(ScrollStrategy::Top);
                    cx.notify();
                }),
            )
            .on_action(
                cx.listener(|panel, _: &language_tool_tree::SelectLast, _, cx| {
                    panel.host.select_last();
                    panel.reveal_selection(ScrollStrategy::Bottom);
                    cx.notify();
                }),
            )
            .on_action(
                cx.listener(|panel, _: &language_tool_tree::SelectParent, _, cx| {
                    panel.host.select_parent();
                    panel.reveal_selection(ScrollStrategy::Center);
                    cx.notify();
                }),
            )
            .on_action(
                cx.listener(|panel, _: &language_tool_tree::SelectFirstChild, _, cx| {
                    panel.host.select_first_child();
                    panel.reveal_selection(ScrollStrategy::Center);
                    cx.notify();
                }),
            )
            .on_action(
                cx.listener(|panel, _: &language_tool_tree::ToggleExpanded, _, cx| {
                    panel.host.toggle_selected();
                    cx.notify();
                }),
            )
            .on_action(cx.listener(
                |panel, _: &language_tool_tree::ActivateSelected, window, cx| {
                    panel.activate_selected(window, cx)
                },
            ))
            .on_action(cx.listener(|panel, _: &BuildSelected, window, cx| {
                panel.execute_selected_action(CargoAction::Build, window, cx)
            }))
            .on_action(cx.listener(|panel, _: &CheckSelected, window, cx| {
                panel.execute_selected_action(CargoAction::Check, window, cx)
            }))
            .on_action(cx.listener(|panel, _: &RunSelected, window, cx| {
                panel.execute_selected_action(CargoAction::Run, window, cx)
            }))
            .on_action(cx.listener(|panel, _: &TestSelected, window, cx| {
                panel.execute_selected_action(CargoAction::Test, window, cx)
            }))
            .on_action(cx.listener(|panel, _: &BenchSelected, window, cx| {
                panel.execute_selected_action(CargoAction::Bench, window, cx)
            }))
            .on_action(cx.listener(|panel, _: &DebugSelected, window, cx| {
                panel.execute_selected_action(CargoAction::Debug, window, cx)
            }))
            .child(
                div()
                    .h_9()
                    .px_2()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child("Cargo")
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                IconButton::new("cargo-expand-all", IconName::ExpandVertical)
                                    .aria_label("Expand All")
                                    .tooltip(Tooltip::text("Expand All"))
                                    .disabled(!can_expand_all)
                                    .on_click(cx.listener(|panel, _, _, cx| {
                                        panel.host.expand_all();
                                        cx.notify();
                                    })),
                            )
                            .child(
                                IconButton::new("cargo-collapse-all", IconName::ListCollapse)
                                    .aria_label("Collapse All")
                                    .tooltip(Tooltip::text("Collapse All"))
                                    .disabled(!can_collapse_all)
                                    .on_click(cx.listener(|panel, _, _, cx| {
                                        panel.host.collapse_all();
                                        cx.notify();
                                    })),
                            )
                            .child(
                                IconButton::new("cargo-refresh", IconName::RefreshTitle)
                                    .aria_label("Refresh")
                                    .tooltip(Tooltip::text("Refresh"))
                                    .disabled(!can_refresh)
                                    .on_click(cx.listener(|panel, _, _, cx| panel.refresh(cx))),
                            ),
                    ),
            )
            .child(language_tool_tree(
                rows,
                selected,
                status,
                self.host.scroll_handle().clone(),
                move |id, click_count, window, cx| {
                    click_panel
                        .update(cx, |panel, cx| {
                            window.focus(&panel.focus_handle, cx);
                            panel.host.select(id.clone());
                            if click_count > 1 {
                                panel.activate_selected(window, cx);
                            }
                            cx.notify();
                        })
                        .ok();
                },
                move |id, _, cx| {
                    toggle_panel
                        .update(cx, |panel, cx| {
                            panel.host.select(id.clone());
                            panel.host.toggle(&id);
                            cx.notify();
                        })
                        .ok();
                },
                move |id, position, window, cx| {
                    context_panel
                        .update(cx, |panel, cx| {
                            panel.deploy_context_menu(position, id, window, cx)
                        })
                        .ok();
                },
            ))
            .children(self.context_menu.as_ref().map(|(menu, position, _)| {
                deferred(
                    anchored()
                        .position(*position)
                        .anchor(Anchor::TopLeft)
                        .child(menu.clone()),
                )
                .with_priority(1)
            }))
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, sync::Arc};

    use project::cargo_workspace::{
        CargoCandidateFailure, CargoConfigurationCompleteness, CargoDependencyModel,
        CargoDependencySourceKind, CargoFeatureModel, CargoHostCompilerModel,
        CargoHostCompilerStatus, CargoPackageModel, CargoProfileModel, CargoProfileOrigin,
        CargoSnapshotCompleteness, CargoTargetModel, CargoToolchainFormat, CargoToolchainModel,
        CargoWorkspaceConfiguration, CargoWorkspaceErrorCategory, CargoWorkspaceKey,
        CargoWorkspaceModel,
    };
    use settings::WorktreeId;
    use util::rel_path::RelPath;

    use super::*;

    fn path(value: &str) -> ProjectPath {
        ProjectPath {
            worktree_id: WorktreeId::from_usize(1),
            path: Arc::from(RelPath::unix(&value).expect("fixture path should be relative")),
        }
    }

    fn snapshot() -> CargoWorkspaceSnapshot {
        let manifest = path("member/Cargo.toml");
        CargoWorkspaceSnapshot {
            revision: 1,
            input_fingerprint: 1,
            completeness: CargoSnapshotCompleteness::Complete,
            failures: Vec::new(),
            workspaces: vec![CargoWorkspaceModel {
                key: CargoWorkspaceKey { root: path("") },
                root_manifest: Some(path("Cargo.toml")),
                display_name: "fixture".to_string(),
                is_virtual: true,
                configuration: CargoWorkspaceConfiguration::unresolved(),
                members: vec![CargoPackageModel {
                    id: "member 0.1.0".to_string(),
                    name: "member".to_string(),
                    version: "0.1.0".to_string(),
                    manifest_path: manifest,
                    is_default_member: true,
                    targets: vec![CargoTargetModel {
                        name: "member".to_string(),
                        kind: CargoTargetKind::Binary,
                        crate_types: vec!["bin".to_string()],
                        source_path: Some(path("member/src/main.rs")),
                        source_display_path: Some("member/src/main.rs".to_string()),
                        required_features: Vec::new(),
                        edition: "2024".to_string(),
                    }],
                    features: vec![CargoFeatureModel {
                        name: "default".to_string(),
                        defined: true,
                        enabled: CargoFeatureEnabled::Enabled,
                        expands: Vec::new(),
                    }],
                    dependencies: vec![CargoDependencyModel {
                        declaration_name: "serde".to_string(),
                        rename: None,
                        kind: CargoDependencyKind::Normal,
                        version_requirement: "^1".to_string(),
                        optional: false,
                        uses_default_features: true,
                        requested_features: vec!["derive".to_string()],
                        target: None,
                        source_kind: CargoDependencySourceKind::Registry,
                        resolved_name: Some("serde".to_string()),
                        resolved_version: Some("1.0.0".to_string()),
                        resolved_workspace_member: None,
                        local_manifest: None,
                    }],
                }],
            }],
        }
    }

    #[test]
    fn projection_is_stable_finite_and_direct_only() {
        let snapshot = snapshot();
        let (first, first_provider) = CargoTreeProvider::project(&snapshot);
        let (second, _) = CargoTreeProvider::project(&snapshot);
        let first_ids = collect_ids(&first.roots);
        let second_ids = collect_ids(&second.roots);
        assert_eq!(first_ids, second_ids);
        assert!(first_ids.iter().all(|id| !id.0.contains("/Users/")));

        let dependencies = find_node(&first.roots, "Dependencies")
            .expect("Dependencies section should be projected");
        let dependency = dependencies
            .children
            .first()
            .and_then(|group| group.children.first())
            .expect("direct dependency should be projected under its kind");
        assert!(dependency.children.is_empty());
        assert_eq!(
            first_provider.navigation(&dependency.id),
            Some(&path("member/Cargo.toml"))
        );
        assert_eq!(<CargoPanel as Panel>::persistent_name(), "Cargo");
    }

    #[test]
    fn rust_workspace_projects_one_thousand_packages_with_stable_finite_ids() {
        let mut large_snapshot = snapshot();
        let package_template = large_snapshot.workspaces[0].members[0].clone();
        large_snapshot.workspaces[0].members = (0..1_000)
            .map(|index| {
                let package_name = format!("package-{index:04}");
                let package_root = format!("members/{package_name}");
                let mut package = package_template.clone();
                package.id = format!("{package_name} 0.1.0");
                package.name = package_name.clone();
                package.manifest_path = path(&format!("{package_root}/Cargo.toml"));
                package.is_default_member = index == 0;
                package.targets[0].name = package_name;
                package.targets[0].source_path = Some(path(&format!("{package_root}/src/main.rs")));
                package.targets[0].source_display_path =
                    Some(format!("{package_root}/src/main.rs"));
                package
            })
            .collect();

        let (first, _) = CargoTreeProvider::project(&large_snapshot);
        let (second, _) = CargoTreeProvider::project(&large_snapshot);
        let first_ids = collect_ids(&first.roots);
        let second_ids = collect_ids(&second.roots);
        let unique_ids = first_ids.iter().collect::<HashSet<_>>();

        assert_eq!(large_snapshot.workspaces[0].members.len(), 1_000);
        assert_eq!(first_ids, second_ids);
        assert_eq!(unique_ids.len(), first_ids.len());
        assert!(first_ids.len() >= 6_000);
        assert!(first_ids.len() < 10_000);
        assert!(first_ids.iter().all(|id| !id.0.contains("/Users/")));
    }

    #[test]
    fn configuration_projection_distinguishes_active_host_and_cargo_targets() {
        let mut snapshot = snapshot();
        snapshot.workspaces[0].configuration = CargoWorkspaceConfiguration {
            profiles: vec![
                CargoProfileModel {
                    name: "dev".to_string(),
                    origin: CargoProfileOrigin::Implicit,
                },
                CargoProfileModel {
                    name: "ship".to_string(),
                    origin: CargoProfileOrigin::Declared,
                },
            ],
            declared_toolchain: Some(CargoToolchainModel {
                source_path: path("rust-toolchain.toml"),
                format: CargoToolchainFormat::Toml,
                channel: Some("stable".to_string()),
                components: vec!["clippy".to_string()],
                targets: vec!["wasm32-unknown-unknown".to_string()],
            }),
            host_compiler: CargoHostCompilerModel {
                status: CargoHostCompilerStatus::Available,
                release: Some("1.90.0".to_string()),
                host_target: Some("aarch64-apple-darwin".to_string()),
                stale: false,
            },
            cargo_target: CargoTargetConfiguration::UnresolvedCargoDefault,
            diagnostics: Vec::new(),
            completeness: CargoConfigurationCompleteness::Complete,
        };
        let active = CargoActiveConfiguration {
            scope: "package member".to_string(),
            profile: Some("ship".to_string()),
            selected_features: vec!["serde".to_string()],
            default_features: Some(false),
            explicit_target: Some("wasm32-unknown-unknown".to_string()),
            target_selector: Some("bin member".to_string()),
            environment_keys: vec!["RUSTFLAGS".to_string(), "CARGO_HOME".to_string()],
            notice: None,
        };
        let (first, provider) = CargoTreeProvider::project_with_configuration(&snapshot, &active);
        let (second, _) = CargoTreeProvider::project_with_configuration(&snapshot, &active);
        assert_eq!(collect_ids(&first.roots), collect_ids(&second.roots));

        let explicit = find_node(&first.roots, "Explicit target")
            .expect("explicit target row should be projected");
        assert_eq!(
            explicit.secondary_label.as_deref(),
            Some("wasm32-unknown-unknown")
        );
        let host = find_node(&first.roots, "Host compiler")
            .expect("host compiler row should be projected");
        assert!(
            host.secondary_label
                .as_deref()
                .is_some_and(|label| label.contains("aarch64-apple-darwin"))
        );
        let cargo_target = find_node(&first.roots, "Cargo target resolution")
            .expect("Cargo target row should be projected");
        assert_eq!(
            cargo_target.secondary_label.as_deref(),
            Some("unresolved Cargo default")
        );
        let environment = find_node(&first.roots, "Environment keys")
            .expect("environment-key row should be projected");
        assert_eq!(
            environment.secondary_label.as_deref(),
            Some("CARGO_HOME, RUSTFLAGS")
        );
        let toolchain = find_node(&first.roots, "Declared toolchain")
            .expect("toolchain row should be projected");
        assert_eq!(
            provider.navigation(&toolchain.id),
            Some(&path("rust-toolchain.toml"))
        );
        assert!(
            collect_ids(&first.roots)
                .iter()
                .all(|id| !id.0.contains("RUSTFLAGS"))
        );
    }

    #[test]
    fn configuration_probe_failure_retains_safe_host_facts_as_stale() {
        let mut previous = snapshot();
        previous.workspaces[0].configuration.host_compiler = CargoHostCompilerModel {
            status: CargoHostCompilerStatus::Available,
            release: Some("1.90.0".to_string()),
            host_target: Some("aarch64-apple-darwin".to_string()),
            stale: false,
        };
        let mut current = snapshot();
        current.revision = 2;
        current.completeness = CargoSnapshotCompleteness::Partial;
        current.workspaces[0].configuration.host_compiler.status = CargoHostCompilerStatus::Failed;
        current.workspaces[0].configuration.completeness = CargoConfigurationCompleteness::Partial;
        current = retain_stale_workspaces(current, Some(&previous));
        assert!(current.workspaces[0].configuration.host_compiler.stale);
        assert_eq!(
            current.workspaces[0]
                .configuration
                .host_compiler
                .host_target
                .as_deref(),
            Some("aarch64-apple-darwin")
        );
        let (projected, _) = CargoTreeProvider::project(&current);
        assert!(matches!(
            projected.status,
            LanguageToolProviderStatus::Partial(message)
                if message.contains("configuration facts")
        ));
    }

    #[test]
    fn partial_refresh_retains_matching_workspace_as_stale() {
        let previous = snapshot();
        let mut current = CargoWorkspaceSnapshot {
            revision: 2,
            input_fingerprint: 2,
            completeness: CargoSnapshotCompleteness::Partial,
            workspaces: Vec::new(),
            failures: vec![CargoCandidateFailure {
                manifest_path: path("Cargo.toml"),
                category: CargoWorkspaceErrorCategory::CargoFailed,
                message: "dependency resolution failed".to_string(),
                has_stale_model: false,
            }],
        };

        current = retain_stale_workspaces(current, Some(&previous));
        assert_eq!(current.workspaces, previous.workspaces);
        assert!(current.failures[0].has_stale_model);

        let removed = retain_stale_workspaces(
            CargoWorkspaceSnapshot {
                revision: 3,
                input_fingerprint: 3,
                completeness: CargoSnapshotCompleteness::Complete,
                workspaces: Vec::new(),
                failures: Vec::new(),
            },
            Some(&previous),
        );
        assert!(removed.workspaces.is_empty());
    }

    #[test]
    fn unopened_panel_host_is_dormant_and_does_not_request_metadata() {
        let host = LanguageToolTreeHost::default();
        assert_eq!(host.status(), &LanguageToolTreeStatus::Dormant);
    }

    fn collect_ids(nodes: &[LanguageToolNode]) -> Vec<LanguageToolNodeId> {
        let mut ids = Vec::new();
        for node in nodes {
            ids.push(node.id.clone());
            ids.extend(collect_ids(&node.children));
        }
        ids
    }

    fn find_node<'a>(nodes: &'a [LanguageToolNode], label: &str) -> Option<&'a LanguageToolNode> {
        for node in nodes {
            if node.label == label {
                return Some(node);
            }
            if let Some(found) = find_node(&node.children, label) {
                return Some(found);
            }
        }
        None
    }
}
