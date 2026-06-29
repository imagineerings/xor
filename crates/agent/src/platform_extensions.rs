#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlatformExtension {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub default_enabled: bool,
}

pub const PLATFORM_EXTENSIONS: &[PlatformExtension] = &[
    PlatformExtension {
        id: "apps",
        name: "Apps",
        description: "Expose app connector capabilities to the agent.",
        default_enabled: true,
    },
    PlatformExtension {
        id: "chatrecall",
        name: "Chatrecall",
        description: "Retrieve relevant prior conversation context.",
        default_enabled: true,
    },
    PlatformExtension {
        id: "summon",
        name: "Summon",
        description: "Invite another agent or workflow into the current task.",
        default_enabled: false,
    },
    PlatformExtension {
        id: "tom",
        name: "Tom",
        description: "Provide TOM workflow integration to the agent.",
        default_enabled: false,
    },
    PlatformExtension {
        id: "analyze",
        name: "Analyze",
        description: "Analyze project data and produce structured findings.",
        default_enabled: true,
    },
    PlatformExtension {
        id: "developer",
        name: "Developer",
        description: "Expose developer-focused agent capabilities.",
        default_enabled: true,
    },
];

pub fn platform_extensions() -> &'static [PlatformExtension] {
    PLATFORM_EXTENSIONS
}

pub fn platform_extension(id: &str) -> Option<&'static PlatformExtension> {
    PLATFORM_EXTENSIONS
        .iter()
        .find(|extension| extension.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use collections::HashSet;

    #[test]
    fn platform_extension_ids_are_unique() {
        let mut ids = HashSet::default();

        for extension in platform_extensions() {
            assert!(ids.insert(extension.id));
        }
    }

    #[test]
    fn platform_extension_lookup_finds_metadata() {
        assert_eq!(
            platform_extension("developer").map(|extension| extension.name),
            Some("Developer")
        );
        assert!(platform_extension("missing").is_none());
    }
}
