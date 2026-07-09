use crate::RuntimeBoundaryDecision;

// ---------------------------------------------------------------------------
// Runtime boundary policy
// ---------------------------------------------------------------------------

/// Determines the migration boundary classification for a Godot, world-model,
/// or Comfy feature area based on duplication-avoidance rules.
///
/// Requirements 2.1, 2.2, 2.3
pub trait RuntimeBoundaryPolicy {
    /// Classify a feature area described by its name and scope into one of
    /// the four boundary decisions.
    fn classify(&self, name: &str, scope: &str) -> RuntimeBoundaryDecision;
}

/// Default boundary policy that uses keyword-based classification.
///
/// Rules:
/// - If the feature scope names a capability Sim already owns (UI,
///   rendering, platform, input, etc.), return `NativeSimFeature`.
/// - If the feature scope names a Godot runtime subsystem that duplicates
///   Sim runtime architecture, return `Excluded` or `ExternalCommand`.
/// - If the feature scope names an external tool or CLI, return `ExternalCommand`.
/// - Otherwise, return `SimAdapter` as the default fallback.
pub struct DefaultBoundaryPolicy;

impl RuntimeBoundaryPolicy for DefaultBoundaryPolicy {
    fn classify(&self, _name: &str, scope: &str) -> RuntimeBoundaryDecision {
        let lower = scope.to_lowercase();

        // Godot runtime subsystems that duplicate Sim runtime (Req 2.2).
        // Check BEFORE Sim-owned capabilities so that "Godot engine
        // rendering loop" is caught by the Godot exclusion, not by the
        // generic "rendering" native-capability keyword.
        if contains_any(&lower, GODOT_RUNTIME_DUPLICATIONS) {
            return RuntimeBoundaryDecision::Excluded {
                reason: format!(
                    "Godot runtime subsystem '{}' duplicates Sim runtime \
                     architecture (Req 2.2)",
                    scope
                ),
            };
        }

        // Sim-owned capabilities (Req 2.1).
        if contains_any(&lower, SIM_OWNED_CAPABILITIES) {
            return RuntimeBoundaryDecision::NativeSimFeature;
        }

        // External tool / CLI integrations (Req 2.2).
        if contains_any(&lower, EXTERNAL_COMMAND_KEYWORDS) {
            return RuntimeBoundaryDecision::ExternalCommand {
                command: infer_command(scope),
            };
        }

        // Default: adapter with inferred owner.
        RuntimeBoundaryDecision::SimAdapter {
            owner: infer_owner(scope),
        }
    }
}

// ---------------------------------------------------------------------------
// Keyword sets
// ---------------------------------------------------------------------------

/// Feature areas Sim already owns natively (Req 2.1).
const SIM_OWNED_CAPABILITIES: &[&str] = &[
    "editor ui",
    "platform",
    "rendering",
    "input",
    "window",
    "theme",
    "language",
    "project panel",
    "file",
    "task",
    "agent",
    "media",
    "audio",
    "notification",
    "diagnostic",
    "settings",
    "keybinding",
    "workspace",
    "pane",
    "tab",
    "title bar",
    "status bar",
    "context menu",
    "autocomplete",
    "search",
    "git",
    "terminal",
    "collaboration",
    "telemetry",
];

/// Godot runtime subsystems that duplicate Sim runtime architecture (Req
/// 2.2).
const GODOT_RUNTIME_DUPLICATIONS: &[&str] = &[
    "godot engine",
    "godot runtime",
    "godot rendering",
    "godot physics",
    "physics server",
    "godot xr",
    "openxr",
    "webxr",
    "vr runtime",
    "godot audio",
    "godot navigation",
    "navigation server",
    "godot networking",
    "godot multiplayer",
    "enet",
    "upnp",
    "packet peer",
    "godot input",
    "embedded runtime",
    "runtime engine",
];

/// Feature areas that map to external command execution (Req 2.2).
const EXTERNAL_COMMAND_KEYWORDS: &[&str] = &[
    "external",
    "cli",
    "command line",
    "export",
    "deploy",
    "build tool",
    "compiler",
    "external debug",
    "launch",
];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn contains_any(text: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|kw| text.contains(kw))
}

fn infer_command(scope: &str) -> String {
    // Try to extract a command name from the scope string.
    let lower = scope.to_lowercase();
    if let Some(cmd) = EXTERNAL_COMMAND_ALIASES
        .iter()
        .find(|(alias, _)| lower.contains(alias))
        .map(|(_, cmd)| *cmd)
    {
        return cmd.to_string();
    }
    // Fall back to a slug based on the first word.
    scope
        .split_whitespace()
        .next()
        .unwrap_or("unknown")
        .to_string()
}

/// Known external command mappings.
const EXTERNAL_COMMAND_ALIASES: &[(&str, &str)] = &[
    ("godot export", "godot --export"),
    ("godot build", "godot --headless --build"),
    ("godot debug", "godot --debug"),
    ("simscript", "sim --script"),
    ("gdunit", "gdunit"),
];

fn infer_owner(scope: &str) -> String {
    let lower = scope.to_lowercase();
    // Check more-specific terms before broader ones.
    if lower.contains("mesh") || lower.contains("3d") {
        return "world_model::mesh".to_string();
    }
    if lower.contains("comfy") || lower.contains("workflow") || lower.contains("graph node") {
        return "world_model::comfy".to_string();
    }
    if lower.contains("model") || lower.contains("serving") || lower.contains("checkpoint") {
        return "world_model::serving".to_string();
    }
    if lower.contains("world") || lower.contains("generation") || lower.contains("diffusion") {
        return "world_model".to_string();
    }
    if lower.contains("asset") || lower.contains("library") || lower.contains("fixture") {
        return "sim_game".to_string();
    }
    "sim_game".to_string()
}
