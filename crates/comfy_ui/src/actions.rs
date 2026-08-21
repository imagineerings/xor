use crate::generated_command_catalog::GENERATED_COMMAND_CATALOG;
use comfy_runtime::CatalogGraphAction;
use gpui::{MenuItem, actions};
use serde::Serialize;

actions!(
    comfy_shell,
    [
        QueuePrompt,
        QueuePromptFront,
        QueueSelectedOutputNodes,
        Interrupt,
        ClearPendingTasks,
        ToggleQueueOverlay,
        ToggleQpov2,
        ToggleExecutionPanel,
        ToggleGraphPropertiesPanel,
        ClearExecutionHistory,
        CancelSelectedExecution,
        RetrySelectedExecution,
        RemoveSelectedExecution,
        CopySelectedExecutionId,
        CopySelectedExecutionError,
        ToggleDockedExecutionHistory,
        ToggleExecutionProgress,
        ExecutionRunManual,
        ExecutionRunOnChange,
        ExecutionRunInstantIdle,
        RestoreExecutionNavigation,
        RefreshNodeDefinitions,
        ToggleWorkflowsSidebar,
        ToggleNodeLibrarySidebar,
        ToggleModelLibrarySidebar,
        ToggleAssetsSidebar,
        ToggleLinear,
        SaveWorkflow,
        OpenWorkflow,
        GroupSelectedNodes,
        ShowSettings,
        ShowKeybindings,
        ToggleSelectedItemsPin,
        ToggleSelectedCollapse,
        ToggleSelectedBypass,
        ToggleSelectedMute,
        ToggleLogsPanel,
        ConvertToSubgraph,
        ToggleMinimap,
        UnlockCanvas,
        LockCanvas,
        ExitSubgraph,
        PasteWithConnect,
        MoveSelectedDown,
        MoveSelectedLeft,
        MoveSelectedRight,
        MoveSelectedUp,
        ResetView,
        ResizeSelectedNodes,
        ToggleLinkVisibility,
        ToggleCanvasLock,
        ToggleSelectedNodesPin,
        FitGroupToContents,
        UnpackSubgraph,
        PublishSubgraph,
    ]
);

macro_rules! native_actions {
    ($($variant:ident => ($name:literal, $action:path)),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
        #[serde(rename_all = "kebab-case")]
        pub enum NativeAction {
            $($variant),+
        }

        impl NativeAction {
            pub fn name(self) -> &'static str {
                match self {
                    $(Self::$variant => $name),+
                }
            }

            pub fn menu_item(self, label: impl Into<gpui::SharedString>) -> MenuItem {
                match self {
                    $(Self::$variant => MenuItem::action(label, $action)),+
                }
            }
        }
    };
}

native_actions!(
    GraphUndo => ("comfy_graph::GraphUndo", crate::GraphUndo),
    GraphRedo => ("comfy_graph::GraphRedo", crate::GraphRedo),
    GraphCopy => ("comfy_graph::GraphCopy", crate::GraphCopy),
    GraphDelete => ("comfy_graph::GraphDelete", crate::GraphDelete),
    GraphFitView => ("comfy_graph::GraphFitView", crate::GraphFitView),
    LockCanvas => ("comfy_shell::LockCanvas", crate::LockCanvas),
    MoveSelectedDown => ("comfy_shell::MoveSelectedDown", crate::MoveSelectedDown),
    MoveSelectedLeft => ("comfy_shell::MoveSelectedLeft", crate::MoveSelectedLeft),
    MoveSelectedRight => ("comfy_shell::MoveSelectedRight", crate::MoveSelectedRight),
    MoveSelectedUp => ("comfy_shell::MoveSelectedUp", crate::MoveSelectedUp),
    GraphPaste => ("comfy_graph::GraphPaste", crate::GraphPaste),
    PasteWithConnect => ("comfy_shell::PasteWithConnect", crate::PasteWithConnect),
    ResetView => ("comfy_shell::ResetView", crate::ResetView),
    ResizeSelectedNodes => ("comfy_shell::ResizeSelectedNodes", crate::ResizeSelectedNodes),
    GraphSelectAll => ("comfy_graph::GraphSelectAll", crate::GraphSelectAll),
    ToggleLinkVisibility => ("comfy_shell::ToggleLinkVisibility", crate::ToggleLinkVisibility),
    ToggleCanvasLock => ("comfy_shell::ToggleCanvasLock", crate::ToggleCanvasLock),
    ToggleMinimap => ("comfy_shell::ToggleMinimap", crate::ToggleMinimap),
    ToggleSelectedBypass => ("comfy_shell::ToggleSelectedBypass", crate::ToggleSelectedBypass),
    ToggleSelectedCollapse => ("comfy_shell::ToggleSelectedCollapse", crate::ToggleSelectedCollapse),
    ToggleSelectedMute => ("comfy_shell::ToggleSelectedMute", crate::ToggleSelectedMute),
    ToggleSelectedNodesPin => ("comfy_shell::ToggleSelectedNodesPin", crate::ToggleSelectedNodesPin),
    ToggleSelectedItemsPin => ("comfy_shell::ToggleSelectedItemsPin", crate::ToggleSelectedItemsPin),
    UnlockCanvas => ("comfy_shell::UnlockCanvas", crate::UnlockCanvas),
    GraphZoomIn => ("comfy_graph::GraphZoomIn", crate::GraphZoomIn),
    GraphZoomOut => ("comfy_graph::GraphZoomOut", crate::GraphZoomOut),
    ClearPendingTasks => ("comfy_shell::ClearPendingTasks", crate::ClearPendingTasks),
    Interrupt => ("comfy_shell::Interrupt", crate::Interrupt),
    QueuePrompt => ("comfy_shell::QueuePrompt", crate::QueuePrompt),
    QueuePromptFront => ("comfy_shell::QueuePromptFront", crate::QueuePromptFront),
    QueueSelectedOutputNodes => ("comfy_shell::QueueSelectedOutputNodes", crate::QueueSelectedOutputNodes),
    ToggleQueueOverlay => ("comfy_shell::ToggleQueueOverlay", crate::ToggleQueueOverlay),
    ToggleQpov2 => ("comfy_shell::ToggleQpov2", crate::ToggleQpov2),
    ExitSubgraph => ("comfy_shell::ExitSubgraph", crate::ExitSubgraph),
    FitGroupToContents => ("comfy_shell::FitGroupToContents", crate::FitGroupToContents),
    GroupSelectedNodes => ("comfy_shell::GroupSelectedNodes", crate::GroupSelectedNodes),
    UnpackSubgraph => ("comfy_shell::UnpackSubgraph", crate::UnpackSubgraph),
    PublishSubgraph => ("comfy_shell::PublishSubgraph", crate::PublishSubgraph),
    RefreshNodeDefinitions => ("comfy_shell::RefreshNodeDefinitions", crate::RefreshNodeDefinitions),
    ConvertToSubgraph => ("comfy_shell::ConvertToSubgraph", crate::ConvertToSubgraph),
    ToggleWorkflowsSidebar => ("comfy_shell::ToggleWorkflowsSidebar", crate::ToggleWorkflowsSidebar),
    ToggleNodeLibrarySidebar => ("comfy_shell::ToggleNodeLibrarySidebar", crate::ToggleNodeLibrarySidebar),
    ToggleModelLibrarySidebar => ("comfy_shell::ToggleModelLibrarySidebar", crate::ToggleModelLibrarySidebar),
    ToggleAssetsSidebar => ("comfy_shell::ToggleAssetsSidebar", crate::ToggleAssetsSidebar),
    ToggleLinear => ("comfy_shell::ToggleLinear", crate::ToggleLinear),
    SaveWorkflow => ("comfy_shell::SaveWorkflow", crate::SaveWorkflow),
    OpenWorkflow => ("comfy_shell::OpenWorkflow", crate::OpenWorkflow),
    ShowSettings => ("comfy_shell::ShowSettings", crate::ShowSettings),
    ShowKeybindings => ("comfy_shell::ShowKeybindings", crate::ShowKeybindings),
    ToggleLogsPanel => ("comfy_shell::ToggleLogsPanel", crate::ToggleLogsPanel),
    CopySelectedExecutionId => ("comfy_shell::CopySelectedExecutionId", crate::CopySelectedExecutionId),
    CopySelectedExecutionError => ("comfy_shell::CopySelectedExecutionError", crate::CopySelectedExecutionError),
    RemoveSelectedExecution => ("comfy_shell::RemoveSelectedExecution", crate::RemoveSelectedExecution),
    CancelSelectedExecution => ("comfy_shell::CancelSelectedExecution", crate::CancelSelectedExecution),
    ToggleDockedExecutionHistory => ("comfy_shell::ToggleDockedExecutionHistory", crate::ToggleDockedExecutionHistory),
    ToggleExecutionProgress => ("comfy_shell::ToggleExecutionProgress", crate::ToggleExecutionProgress),
    ClearExecutionHistory => ("comfy_shell::ClearExecutionHistory", crate::ClearExecutionHistory),
    ExecutionRunManual => ("comfy_shell::ExecutionRunManual", crate::ExecutionRunManual),
    ExecutionRunOnChange => ("comfy_shell::ExecutionRunOnChange", crate::ExecutionRunOnChange),
    ExecutionRunInstantIdle => ("comfy_shell::ExecutionRunInstantIdle", crate::ExecutionRunInstantIdle),
);

pub const DEFAULT_COMFY_KEYMAP_PATH: &str = "keymaps/default-comfy.json";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CommandAvailability {
    Active,
    PlatformSpecific,
    Experimental,
    DeveloperOnly,
    CloudPaid,
    Deprecated,
}

impl CommandAvailability {
    pub fn catalog_name(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::PlatformSpecific => "platform-specific",
            Self::Experimental => "experimental",
            Self::DeveloperOnly => "developer-only",
            Self::CloudPaid => "cloud/paid",
            Self::Deprecated => "deprecated/dead",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NativePlacement {
    GraphWorkspace,
    ExecutionDock,
    WorkspaceShell,
    Settings,
    DesktopIntegration,
    NodeLibrary,
    AssetBrowser,
    ExtensionHost,
    MediaEditor,
    HelpCenter,
    Account,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum CommandNativeStatus {
    Executable,
    Infrastructure {
        owner: &'static str,
    },
    RequiresInput {
        owner: &'static str,
        input: &'static str,
    },
    LaterOwned {
        owner: &'static str,
    },
    Gated {
        owner: &'static str,
        gate: &'static str,
    },
    Legacy {
        owner: &'static str,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct CommandRegistration {
    pub feature_id: &'static str,
    pub command_id: &'static str,
    pub label: &'static str,
    pub availability: CommandAvailability,
    pub placement: NativePlacement,
    pub owner: &'static str,
    pub status: CommandNativeStatus,
    pub graph_action: Option<CatalogGraphAction>,
    pub gpui_action: Option<NativeAction>,
}

pub fn command_registry() -> impl ExactSizeIterator<Item = CommandRegistration> {
    GENERATED_COMMAND_CATALOG.iter().copied().map(|row| {
        let graph_action = CatalogGraphAction::ALL
            .iter()
            .copied()
            .find(|action| action.command_id() == row.command_id);
        CommandRegistration {
            feature_id: row.feature_id,
            command_id: row.command_id,
            label: row.label,
            availability: row.availability,
            placement: row.placement,
            owner: row.owner,
            status: row.status,
            graph_action,
            gpui_action: row.native_action,
        }
    })
}

pub fn command_registration(command_id: &str) -> Option<CommandRegistration> {
    command_registry().find(|registration| registration.command_id == command_id)
}
