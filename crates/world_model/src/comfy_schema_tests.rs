use serde_json::json;

use crate::{
    ComfyInputDeclaration, ComfyInputSchemaDeclaration, ComfyInputSection, ComfyNodeRegistry,
    ComfySchemaAdapter, DataType, declarations_by_section,
};

#[test]
fn adapter_normalizes_required_optional_hidden_lazy_list_and_combo_inputs() {
    let registry = ComfyNodeRegistry::with_core_nodes();
    let node = registry.get("KSampler").expect("KSampler node");
    let schema = ComfySchemaAdapter::new()
        .adapt(
            node,
            vec![
                declaration(
                    "seed",
                    ComfyInputSection::Required,
                    ComfyInputDeclaration::Primitive {
                        data_type: "INT".to_string(),
                        default: Some(json!(42)),
                    },
                ),
                declaration(
                    "sampler_name",
                    ComfyInputSection::Required,
                    ComfyInputDeclaration::Combo {
                        values: vec!["euler".to_string(), "dpmpp_2m".to_string()],
                        default: Some("euler".to_string()),
                    },
                ),
                declaration(
                    "preview",
                    ComfyInputSection::Optional,
                    ComfyInputDeclaration::Primitive {
                        data_type: "BOOLEAN".to_string(),
                        default: Some(json!(true)),
                    },
                ),
                declaration(
                    "prompt",
                    ComfyInputSection::Optional,
                    ComfyInputDeclaration::List {
                        item_type: "STRING".to_string(),
                    },
                ),
                declaration(
                    "node_id",
                    ComfyInputSection::Hidden,
                    ComfyInputDeclaration::Lazy {
                        data_type: "STRING".to_string(),
                    },
                ),
            ],
        )
        .expect("schema adapts");

    let seed = schema
        .inputs
        .iter()
        .find(|input| input.name == "seed")
        .expect("seed");
    assert_eq!(seed.data_type, DataType::Int);
    assert!(seed.required);
    assert!(!seed.hidden);

    let sampler = schema
        .inputs
        .iter()
        .find(|input| input.name == "sampler_name")
        .expect("sampler");
    assert_eq!(sampler.data_type, DataType::String);
    assert_eq!(sampler.combo_values, vec!["euler", "dpmpp_2m"]);

    let prompt = schema
        .inputs
        .iter()
        .find(|input| input.name == "prompt")
        .expect("prompt");
    assert!(prompt.list);
    assert!(!prompt.required);

    let hidden = schema
        .inputs
        .iter()
        .find(|input| input.name == "node_id")
        .expect("hidden");
    assert!(hidden.hidden);
    assert!(hidden.lazy);
}

#[test]
fn adapter_preserves_outputs_from_registered_node_definition() {
    let registry = ComfyNodeRegistry::with_core_nodes();
    let node = registry.get("CLIPTextEncode").expect("CLIPTextEncode node");
    let schema = ComfySchemaAdapter::new().from_node_definition(node);

    assert_eq!(schema.node_id, "CLIPTextEncode");
    assert!(
        schema
            .inputs
            .iter()
            .any(|input| input.name == "text" && input.data_type == DataType::String)
    );
    assert!(
        schema
            .outputs
            .iter()
            .any(|output| output.name == "CONDITIONING"
                && output.data_type == DataType::Conditioning)
    );
}

#[test]
fn adapter_reports_empty_combo_and_unknown_type_diagnostics() {
    let registry = ComfyNodeRegistry::with_core_nodes();
    let node = registry.get("KSampler").expect("KSampler node");
    let diagnostics = ComfySchemaAdapter::new()
        .adapt(
            node,
            vec![
                declaration(
                    "sampler_name",
                    ComfyInputSection::Required,
                    ComfyInputDeclaration::Combo {
                        values: Vec::new(),
                        default: None,
                    },
                ),
                declaration(
                    "mystery",
                    ComfyInputSection::Optional,
                    ComfyInputDeclaration::Primitive {
                        data_type: "MYSTERY".to_string(),
                        default: None,
                    },
                ),
            ],
        )
        .expect_err("invalid declarations rejected");

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == crate::comfy_schema::SCHEMA_EMPTY_COMBO_CODE)
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == crate::comfy_schema::SCHEMA_UNKNOWN_TYPE_CODE)
    );
}

#[test]
fn declarations_can_be_grouped_by_section_for_object_info_rendering() {
    let grouped = declarations_by_section([
        declaration(
            "model",
            ComfyInputSection::Required,
            ComfyInputDeclaration::Primitive {
                data_type: "MODEL".to_string(),
                default: None,
            },
        ),
        declaration(
            "extra_pnginfo",
            ComfyInputSection::Hidden,
            ComfyInputDeclaration::Primitive {
                data_type: "STRING".to_string(),
                default: None,
            },
        ),
    ]);

    assert_eq!(
        grouped
            .get(&ComfyInputSection::Required)
            .expect("required")
            .len(),
        1
    );
    assert_eq!(
        grouped
            .get(&ComfyInputSection::Hidden)
            .expect("hidden")
            .len(),
        1
    );
}

fn declaration(
    name: &str,
    section: ComfyInputSection,
    declaration: ComfyInputDeclaration,
) -> ComfyInputSchemaDeclaration {
    ComfyInputSchemaDeclaration {
        name: name.to_string(),
        section,
        declaration,
        tooltip: Some(format!("{name} tooltip")),
    }
}
