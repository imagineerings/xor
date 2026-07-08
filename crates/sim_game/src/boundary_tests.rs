use crate::{DefaultBoundaryPolicy, RuntimeBoundaryDecision, RuntimeBoundaryPolicy};

// ---------------------------------------------------------------------------
// DefaultBoundaryPolicy classification
// ---------------------------------------------------------------------------

#[test]
fn classifies_sim_owned_capability_as_native() {
    let policy = DefaultBoundaryPolicy;
    let decision = policy.classify("editor", "Editor UI and workspace management");
    assert_eq!(decision, RuntimeBoundaryDecision::NativeSimFeature);
}

#[test]
fn classifies_platform_as_native() {
    let policy = DefaultBoundaryPolicy;
    let decision = policy.classify("platform", "Platform integration and system chrome");
    assert_eq!(decision, RuntimeBoundaryDecision::NativeSimFeature);
}

#[test]
fn classifies_godot_engine_as_excluded() {
    let policy = DefaultBoundaryPolicy;
    let decision = policy.classify("godot-engine", "Godot engine runtime and rendering loop");
    match decision {
        RuntimeBoundaryDecision::Excluded { reason } => {
            assert!(
                reason.contains("Req 2.2"),
                "Expected exclusion reason mentioning Req 2.2, got: {reason}"
            );
            assert!(
                reason.contains("runtime"),
                "Expected exclusion reason mentioning runtime, got: {reason}"
            );
        }
        other => panic!("Expected Excluded, got: {other:?}"),
    }
}

#[test]
fn classifies_godot_physics_as_excluded() {
    let policy = DefaultBoundaryPolicy;
    let decision = policy.classify("godot-physics", "Godot physics and collision detection");
    assert!(
        matches!(decision, RuntimeBoundaryDecision::Excluded { .. }),
        "Expected Excluded, got: {decision:?}"
    );
}

#[test]
fn classifies_export_tool_as_external_command() {
    let policy = DefaultBoundaryPolicy;
    let decision = policy.classify("export", "Godot export and deploy pipeline");
    match decision {
        RuntimeBoundaryDecision::ExternalCommand { ref command } => {
            assert_eq!(command, "godot --export");
        }
        other => panic!("Expected ExternalCommand, got: {other:?}"),
    }
}

#[test]
fn classifies_build_tool_as_external_command() {
    let policy = DefaultBoundaryPolicy;
    let decision = policy.classify("build", "Build tool and compiler integration");
    assert!(
        matches!(decision, RuntimeBoundaryDecision::ExternalCommand { .. }),
        "Expected ExternalCommand, got: {decision:?}"
    );
}

#[test]
fn classifies_world_generation_as_adapter_with_world_model_owner() {
    let policy = DefaultBoundaryPolicy;
    let decision = policy.classify("world-gen", "World generation and diffusion inference");
    match decision {
        RuntimeBoundaryDecision::SimAdapter { ref owner } => {
            assert_eq!(owner, "world_model");
        }
        other => panic!("Expected SimAdapter, got: {other:?}"),
    }
}

#[test]
fn classifies_comfy_workflow_as_adapter_with_comfy_owner() {
    let policy = DefaultBoundaryPolicy;
    let decision = policy.classify("comfy-wf", "Comfy workflow orchestration and graph nodes");
    match decision {
        RuntimeBoundaryDecision::SimAdapter { ref owner } => {
            assert_eq!(owner, "world_model::comfy");
        }
        other => panic!("Expected SimAdapter, got: {other:?}"),
    }
}

#[test]
fn classifies_model_serving_as_adapter_with_serving_owner() {
    let policy = DefaultBoundaryPolicy;
    let decision = policy.classify("model-serving", "Model serving and checkpoint management");
    match decision {
        RuntimeBoundaryDecision::SimAdapter { ref owner } => {
            assert_eq!(owner, "world_model::serving");
        }
        other => panic!("Expected SimAdapter, got: {other:?}"),
    }
}

#[test]
fn classifies_mesh_generation_as_adapter_with_mesh_owner() {
    let policy = DefaultBoundaryPolicy;
    let decision = policy.classify("mesh-gen", "3D mesh generation pipeline");
    match decision {
        RuntimeBoundaryDecision::SimAdapter { ref owner } => {
            assert_eq!(owner, "world_model::mesh");
        }
        other => panic!("Expected SimAdapter, got: {other:?}"),
    }
}

#[test]
fn classifies_asset_library_as_adapter_with_sim_game_owner() {
    let policy = DefaultBoundaryPolicy;
    let decision = policy.classify("assets", "Asset library and fixture management");
    match decision {
        RuntimeBoundaryDecision::SimAdapter { ref owner } => {
            assert_eq!(owner, "sim_game");
        }
        other => panic!("Expected SimAdapter, got: {other:?}"),
    }
}

#[test]
fn classifies_language_scope_as_native() {
    let policy = DefaultBoundaryPolicy;
    let decision = policy.classify("simscript", "Language support for SimScript");
    assert_eq!(decision, RuntimeBoundaryDecision::NativeSimFeature);
}

#[test]
fn classifies_terminal_as_native() {
    let policy = DefaultBoundaryPolicy;
    let decision = policy.classify("terminal", "Terminal integration and shell commands");
    assert_eq!(decision, RuntimeBoundaryDecision::NativeSimFeature);
}

#[test]
fn classifies_godot_debug_as_external_command() {
    let policy = DefaultBoundaryPolicy;
    let decision = policy.classify("debug", "Godot debug and launch integration");
    match decision {
        RuntimeBoundaryDecision::ExternalCommand { ref command } => {
            assert_eq!(command, "godot --debug");
        }
        other => panic!("Expected ExternalCommand, got: {other:?}"),
    }
}

#[test]
fn classifies_godot_xr_as_excluded() {
    let policy = DefaultBoundaryPolicy;
    let decision = policy.classify("godot-xr", "Godot XR runtime and spatial interaction");
    assert!(
        matches!(decision, RuntimeBoundaryDecision::Excluded { .. }),
        "Expected Excluded for Godot XR runtime, got: {decision:?}"
    );
}

#[test]
fn policy_is_deterministic_across_calls() {
    let policy = DefaultBoundaryPolicy;
    let first = policy.classify("media", "Media pipeline and video processing");
    let second = policy.classify("media", "Media pipeline and video processing");
    assert_eq!(first, second);
}
