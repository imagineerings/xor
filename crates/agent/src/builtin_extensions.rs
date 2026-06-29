#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BuiltinExtension {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub kind: BuiltinExtensionKind,
    pub default_enabled: bool,
    pub tool_names: &'static [&'static str],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BuiltinExtensionKind {
    Platform,
    Utility,
    Integration,
}

pub const CODE_EXECUTION_EXTENSION_ID: &str = "code_execution";
pub const APPS_EXTENSION_ID: &str = "apps";
pub const ORCHESTRATOR_EXTENSION_ID: &str = "orchestrator";
pub const CHATRECALL_EXTENSION_ID: &str = "chatrecall";
pub const SUMMARIZE_EXTENSION_ID: &str = "summarize";
pub const SUMMON_EXTENSION_ID: &str = "summon";
pub const TODO_EXTENSION_ID: &str = "todo";
pub const TOM_EXTENSION_ID: &str = "tom";
pub const ANALYZE_EXTENSION_ID: &str = "analyze";
pub const DEVELOPER_EXTENSION_ID: &str = "developer";

pub const BUILTIN_EXTENSIONS: &[BuiltinExtension] = &[
    BuiltinExtension {
        id: CODE_EXECUTION_EXTENSION_ID,
        name: "Code Execution",
        description: "Run sandboxed code from agent workflows.",
        kind: BuiltinExtensionKind::Platform,
        default_enabled: false,
        tool_names: &["code_execution"],
    },
    BuiltinExtension {
        id: APPS_EXTENSION_ID,
        name: "Apps",
        description: "Expose app connector capabilities to the agent.",
        kind: BuiltinExtensionKind::Integration,
        default_enabled: true,
        tool_names: &["apps"],
    },
    BuiltinExtension {
        id: ORCHESTRATOR_EXTENSION_ID,
        name: "Orchestrator",
        description: "Coordinate multi-step agent workflows.",
        kind: BuiltinExtensionKind::Platform,
        default_enabled: true,
        tool_names: &["orchestrator"],
    },
    BuiltinExtension {
        id: CHATRECALL_EXTENSION_ID,
        name: "Chatrecall",
        description: "Retrieve relevant prior conversation context.",
        kind: BuiltinExtensionKind::Integration,
        default_enabled: true,
        tool_names: &["chatrecall"],
    },
    BuiltinExtension {
        id: SUMMARIZE_EXTENSION_ID,
        name: "Summarize",
        description: "Summarize long content for agent context.",
        kind: BuiltinExtensionKind::Utility,
        default_enabled: true,
        tool_names: &["summarize"],
    },
    BuiltinExtension {
        id: SUMMON_EXTENSION_ID,
        name: "Summon",
        description: "Invite another agent or workflow into the current task.",
        kind: BuiltinExtensionKind::Integration,
        default_enabled: false,
        tool_names: &["summon"],
    },
    BuiltinExtension {
        id: TODO_EXTENSION_ID,
        name: "Todo",
        description: "Track task state inside an agent session.",
        kind: BuiltinExtensionKind::Utility,
        default_enabled: true,
        tool_names: &["todo"],
    },
    BuiltinExtension {
        id: TOM_EXTENSION_ID,
        name: "Tom",
        description: "Provide TOM workflow integration to the agent.",
        kind: BuiltinExtensionKind::Integration,
        default_enabled: false,
        tool_names: &["tom"],
    },
    BuiltinExtension {
        id: ANALYZE_EXTENSION_ID,
        name: "Analyze",
        description: "Analyze project data and produce structured findings.",
        kind: BuiltinExtensionKind::Utility,
        default_enabled: true,
        tool_names: &["analyze"],
    },
    BuiltinExtension {
        id: DEVELOPER_EXTENSION_ID,
        name: "Developer",
        description: "Expose developer-focused agent capabilities.",
        kind: BuiltinExtensionKind::Platform,
        default_enabled: true,
        tool_names: &["developer"],
    },
];

pub fn builtin_extensions() -> &'static [BuiltinExtension] {
    BUILTIN_EXTENSIONS
}

pub fn default_builtin_extensions() -> impl Iterator<Item = &'static BuiltinExtension> {
    BUILTIN_EXTENSIONS
        .iter()
        .filter(|extension| extension.default_enabled)
}

pub fn builtin_extension(id: &str) -> Option<&'static BuiltinExtension> {
    BUILTIN_EXTENSIONS
        .iter()
        .find(|extension| extension.id == id)
}

pub fn builtin_extension_for_tool(tool_name: &str) -> Option<&'static BuiltinExtension> {
    BUILTIN_EXTENSIONS
        .iter()
        .find(|extension| extension.tool_names.contains(&tool_name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use collections::HashSet;

    #[test]
    fn registry_ids_are_unique() {
        let mut ids = HashSet::default();

        for extension in builtin_extensions() {
            assert!(
                ids.insert(extension.id),
                "duplicate built-in extension id: {}",
                extension.id
            );
        }
    }

    #[test]
    fn lookup_finds_extensions_by_id_and_tool_name() {
        assert_eq!(
            builtin_extension(TODO_EXTENSION_ID).map(|extension| extension.name),
            Some("Todo")
        );
        assert_eq!(
            builtin_extension_for_tool("summarize").map(|extension| extension.id),
            Some(SUMMARIZE_EXTENSION_ID)
        );
        assert!(builtin_extension("missing").is_none());
        assert!(builtin_extension_for_tool("missing").is_none());
    }

    #[test]
    fn default_extensions_are_subset_of_registry() {
        let defaults = default_builtin_extensions().collect::<Vec<_>>();

        assert!(!defaults.is_empty());
        assert!(defaults.iter().all(|extension| extension.default_enabled));
        assert!(
            defaults
                .iter()
                .all(|extension| builtin_extension(extension.id).is_some())
        );
    }
}
