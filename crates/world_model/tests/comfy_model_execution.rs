use std::collections::BTreeSet;

use serde_json::Value;
use world_model::SimDiffusionWorldModelBacklogCatalog;

const DIFFUSION_WORLD_MODEL_BACKLOG: &str =
    include_str!("../fixtures/comfy/diffusion_world_model_backlog.json");
const MANIFEST: &str = include_str!("../fixtures/comfy/model_execution_manifest.json");

#[test]
fn model_execution_manifest_covers_required_comfy_workflows_with_mock_runners() {
    let manifest: Value = serde_json::from_str(MANIFEST).expect("manifest parses");

    assert_eq!(manifest["schema_version"], 1);
    assert_eq!(manifest["runner"], "mock");
    assert_eq!(manifest["requires_production_weights"], false);
    assert_eq!(manifest["requires_downloads"], false);
    assert_eq!(manifest["native_sim_records"], true);

    let workflows = manifest["workflows"].as_array().expect("workflows array");
    let categories = workflows
        .iter()
        .map(|workflow| workflow["category"].as_str().expect("category"))
        .collect::<BTreeSet<_>>();

    for required in [
        "text-to-image",
        "image-to-image",
        "inpaint",
        "ControlNet",
        "LoRA",
        "VAE",
        "sampler/scheduler",
        "video/world-model",
    ] {
        assert!(
            categories.contains(required),
            "missing fixture category {required}"
        );
    }
}

#[test]
fn model_execution_manifest_records_native_validation_surfaces() {
    let manifest: Value = serde_json::from_str(MANIFEST).expect("manifest parses");
    let workflows = manifest["workflows"].as_array().expect("workflows array");
    let validations = workflows
        .iter()
        .flat_map(|workflow| {
            workflow["validates"]
                .as_array()
                .expect("validates array")
                .iter()
                .map(|validation| validation.as_str().expect("validation string"))
        })
        .collect::<BTreeSet<_>>();

    for required in [
        "sampler",
        "scheduler",
        "conditioning",
        "latent",
        "vae",
        "component_set",
        "patch_pipeline",
        "runner_profile",
        "worker",
        "provenance",
    ] {
        assert!(
            validations.contains(required),
            "missing validation surface {required}"
        );
    }

    let divergences = manifest["divergences"]
        .as_array()
        .expect("divergences array");
    assert!(divergences.iter().any(|divergence| {
        divergence["behavior"] == "production_weights"
            && divergence["reason"] == "DependencyReview"
            && divergence["sim_behavior"]
                .as_str()
                .expect("sim behavior")
                .contains("mock runner")
    }));
}

#[test]
fn diffusion_world_model_backlog_maps_remaining_nodes_to_native_sim_surfaces() {
    let backlog: SimDiffusionWorldModelBacklogCatalog =
        serde_json::from_str(DIFFUSION_WORLD_MODEL_BACKLOG).expect("backlog fixture parses");
    backlog
        .validate()
        .expect("backlog fixture should be internally valid");

    assert_eq!(backlog.records.len(), 200);

    let surfaces = backlog.records_by_surface();
    for required in [
        "sampler",
        "scheduler",
        "conditioning",
        "latent",
        "vae",
        "component_set",
        "patch_pipeline",
        "runner_profile",
        "worker",
    ] {
        assert!(
            surfaces.contains_key(required),
            "missing native Sim surface {required}"
        );
    }

    for record in backlog.records {
        assert!(record.metadata_only);
        assert!(!record.requires_dependency_review);
        assert!(
            record
                .evidence_module
                .starts_with("crates/world_model/src/")
        );
        assert!(
            record.evidence_module.contains("comfy_"),
            "compatibility module should carry source attribution"
        );
    }
}

#[test]
fn diffusion_world_model_backlog_surfaces_are_exercised_by_model_execution_manifest() {
    let backlog: SimDiffusionWorldModelBacklogCatalog =
        serde_json::from_str(DIFFUSION_WORLD_MODEL_BACKLOG).expect("backlog fixture parses");
    let manifest: Value = serde_json::from_str(MANIFEST).expect("manifest parses");
    let validations = manifest["workflows"]
        .as_array()
        .expect("workflows array")
        .iter()
        .flat_map(|workflow| {
            workflow["validates"]
                .as_array()
                .expect("validates array")
                .iter()
                .map(|validation| validation.as_str().expect("validation string").to_string())
        })
        .collect::<BTreeSet<_>>();

    let covered_surfaces = backlog
        .records_by_surface()
        .into_keys()
        .collect::<BTreeSet<_>>();

    for surface in covered_surfaces {
        let manifest_surface = match surface.as_str() {
            "patch_pipeline" => "patch_pipeline",
            "runner_profile" => "runner_profile",
            "worker" => "worker",
            other => other,
        };
        assert!(
            validations.contains(manifest_surface),
            "model execution manifest must exercise surface {surface}"
        );
    }
}
