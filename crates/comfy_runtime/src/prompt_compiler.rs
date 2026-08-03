use crate::executor::NativeNodeRegistry;
use comfy_types::{ApiPrompt, NodeId, PromptId, PromptNode, PromptSubmission};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;
use uuid::Uuid;

pub const MAX_PROMPT_NODES: usize = 100_000;
pub const MAX_PROMPT_INPUTS: usize = 1_000_000;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueType {
    Any,
    Boolean,
    Integer,
    Number,
    String,
    Image,
    Mask,
    Latent,
    Model,
    Conditioning,
    Tensor,
    Artifact,
    Custom(String),
}

impl ValueType {
    fn accepts(&self, output: &Self) -> bool {
        self == &Self::Any
            || output == &Self::Any
            || self == output
            || (self == &Self::Number && output == &Self::Integer)
    }

    fn accepts_literal(&self, value: &Value) -> bool {
        match self {
            Self::Any => true,
            Self::Boolean => value.is_boolean(),
            Self::Integer => value.as_i64().is_some() || value.as_u64().is_some(),
            Self::Number => value.is_number(),
            Self::String => value.is_string(),
            Self::Image
            | Self::Mask
            | Self::Latent
            | Self::Model
            | Self::Conditioning
            | Self::Tensor
            | Self::Artifact
            | Self::Custom(_) => false,
        }
    }

    pub(crate) fn accepts_runtime_output(&self, value: &Value) -> bool {
        match self {
            Self::Image
            | Self::Mask
            | Self::Latent
            | Self::Model
            | Self::Conditioning
            | Self::Tensor
            | Self::Artifact
            | Self::Custom(_) => !value.is_null(),
            _ => self.accepts_literal(value),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputMode {
    Scalar,
    List,
    Mapped,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeInputDescriptor {
    pub value_type: ValueType,
    pub required: bool,
    pub hidden: bool,
    pub lazy: bool,
    pub mode: InputMode,
    pub allows_literal: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeOutputDescriptor {
    pub value_type: ValueType,
    pub is_list: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeAvailability {
    Native,
    NativeProvider { provider: String },
    Unavailable { reason: String },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectClass {
    Pure,
    ReadsArtifact,
    WritesArtifact,
    Provider,
    ExclusiveDevice,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeCachePolicy {
    InputIdentity,
    Never,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeNodeDescriptor {
    pub class_type: String,
    pub implementation_version: String,
    pub inputs: BTreeMap<String, RuntimeInputDescriptor>,
    pub outputs: Vec<RuntimeOutputDescriptor>,
    pub output_node: bool,
    pub availability: RuntimeAvailability,
    pub effect: EffectClass,
    pub cache: RuntimeCachePolicy,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeNodePresentation {
    pub display_name: String,
    pub category: String,
    pub output_names: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InputBinding {
    Literal {
        value: Value,
    },
    Link {
        source: NodeId,
        output_index: usize,
        lazy: bool,
        mode: InputMode,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompiledNode {
    pub id: NodeId,
    pub class_type: String,
    pub descriptor: RuntimeNodeDescriptor,
    pub inputs: BTreeMap<String, InputBinding>,
    pub unknown: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompiledPlan {
    pub prompt_id: PromptId,
    pub client_id: Option<String>,
    pub prompt_number: Option<f64>,
    pub extra_data: BTreeMap<String, Value>,
    pub unknown: BTreeMap<String, Value>,
    pub nodes: BTreeMap<NodeId, CompiledNode>,
    pub topological_order: Vec<NodeId>,
    pub static_required_nodes: BTreeSet<NodeId>,
    pub output_nodes: Vec<NodeId>,
    #[serde(default, flatten)]
    pub persistence_unknown_fields: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum PromptCompileError {
    #[error("prompt contains {actual} nodes, exceeding the {maximum} node limit")]
    TooManyNodes { actual: usize, maximum: usize },
    #[error("prompt contains more than {maximum} inputs")]
    TooManyInputs { maximum: usize },
    #[error("runtime descriptor `{0}` is invalid")]
    InvalidRuntimeDescriptor(String),
    #[error("runtime descriptor `{0}` is already registered")]
    DuplicateRuntimeDescriptor(String),
    #[error("node {node:?} references unknown class type `{class_type}`")]
    UnknownNodeType { node: NodeId, class_type: String },
    #[error("node {node:?} class type `{class_type}` is unavailable: {reason}")]
    UnavailableNode {
        node: NodeId,
        class_type: String,
        reason: String,
    },
    #[error("node {node:?} is missing required input `{input}`")]
    MissingInput { node: NodeId, input: String },
    #[error("node {node:?} supplied unknown input `{input}`")]
    UnknownInput { node: NodeId, input: String },
    #[error("node {node:?} input `{input}` has an invalid literal")]
    InvalidLiteral { node: NodeId, input: String },
    #[error("node {node:?} requests unsupported hidden input `{input}`")]
    UnsupportedHiddenInput { node: NodeId, input: String },
    #[error("node {node:?} input `{input}` links to unknown node {source_node:?}")]
    UnknownLink {
        node: NodeId,
        input: String,
        source_node: NodeId,
    },
    #[error(
        "node {node:?} input `{input}` links to missing output {output_index} on {source_node:?}"
    )]
    UnknownOutput {
        node: NodeId,
        input: String,
        source_node: NodeId,
        output_index: usize,
    },
    #[error(
        "node {node:?} input `{input}` is incompatible with {source_node:?} output {output_index}"
    )]
    IncompatibleLink {
        node: NodeId,
        input: String,
        source_node: NodeId,
        output_index: usize,
    },
    #[error("prompt has no reachable output node")]
    NoOutputNode,
    #[error("prompt graph contains a cycle involving {0:?}")]
    Cycle(Vec<NodeId>),
}

pub struct PromptCompiler<'a> {
    registry: &'a NativeNodeRegistry,
}

impl<'a> PromptCompiler<'a> {
    pub fn new(registry: &'a NativeNodeRegistry) -> Self {
        Self { registry }
    }

    pub fn compile(
        &self,
        submission: PromptSubmission,
    ) -> Result<CompiledPlan, PromptCompileError> {
        if submission.prompt.0.len() > MAX_PROMPT_NODES {
            return Err(PromptCompileError::TooManyNodes {
                actual: submission.prompt.0.len(),
                maximum: MAX_PROMPT_NODES,
            });
        }
        submission
            .prompt
            .0
            .values()
            .try_fold(0_usize, |count, node| count.checked_add(node.inputs.len()))
            .filter(|count| *count <= MAX_PROMPT_INPUTS)
            .ok_or(PromptCompileError::TooManyInputs {
                maximum: MAX_PROMPT_INPUTS,
            })?;
        let descriptors = self.resolve_descriptors(&submission.prompt)?;
        let mut nodes = BTreeMap::new();
        for (node_id, prompt_node) in &submission.prompt.0 {
            let descriptor =
                descriptors
                    .get(node_id)
                    .ok_or_else(|| PromptCompileError::UnknownNodeType {
                        node: node_id.clone(),
                        class_type: prompt_node.class_type.clone(),
                    })?;
            let mut inputs = compile_inputs(
                node_id,
                prompt_node,
                descriptor,
                &submission.prompt,
                &descriptors,
            )?;
            inject_hidden_inputs(
                node_id,
                descriptor,
                &submission.prompt,
                &submission.extra_data,
                &mut inputs,
            )?;
            nodes.insert(
                node_id.clone(),
                CompiledNode {
                    id: node_id.clone(),
                    class_type: prompt_node.class_type.clone(),
                    descriptor: (*descriptor).clone(),
                    inputs,
                    unknown: prompt_node.unknown.clone(),
                },
            );
        }
        let output_nodes = nodes
            .values()
            .filter(|node| node.descriptor.output_node)
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        if output_nodes.is_empty() {
            return Err(PromptCompileError::NoOutputNode);
        }
        let topological_order = topological_order(&nodes)?;
        let static_required_nodes = static_required_nodes(&nodes, &output_nodes);
        Ok(CompiledPlan {
            prompt_id: submission
                .prompt_id
                .unwrap_or_else(|| PromptId(Uuid::new_v4())),
            client_id: submission.client_id,
            prompt_number: submission.number,
            extra_data: submission.extra_data,
            unknown: submission.unknown,
            nodes,
            topological_order,
            static_required_nodes,
            output_nodes,
            persistence_unknown_fields: BTreeMap::new(),
        })
    }

    fn resolve_descriptors<'b>(
        &'b self,
        prompt: &ApiPrompt,
    ) -> Result<BTreeMap<NodeId, &'a RuntimeNodeDescriptor>, PromptCompileError> {
        let mut descriptors = BTreeMap::new();
        for (node_id, node) in &prompt.0 {
            let descriptor = self.registry.descriptor(&node.class_type).ok_or_else(|| {
                PromptCompileError::UnknownNodeType {
                    node: node_id.clone(),
                    class_type: node.class_type.clone(),
                }
            })?;
            if let RuntimeAvailability::Unavailable { reason } = &descriptor.availability {
                return Err(PromptCompileError::UnavailableNode {
                    node: node_id.clone(),
                    class_type: node.class_type.clone(),
                    reason: reason.clone(),
                });
            }
            descriptors.insert(node_id.clone(), descriptor);
        }
        Ok(descriptors)
    }
}

fn compile_inputs(
    node_id: &NodeId,
    prompt_node: &PromptNode,
    descriptor: &RuntimeNodeDescriptor,
    prompt: &ApiPrompt,
    descriptors: &BTreeMap<NodeId, &RuntimeNodeDescriptor>,
) -> Result<BTreeMap<String, InputBinding>, PromptCompileError> {
    for (name, input) in &descriptor.inputs {
        if input.required && !input.hidden && !prompt_node.inputs.contains_key(name) {
            return Err(PromptCompileError::MissingInput {
                node: node_id.clone(),
                input: name.clone(),
            });
        }
    }
    let mut compiled = BTreeMap::new();
    for (name, value) in &prompt_node.inputs {
        let input =
            descriptor
                .inputs
                .get(name)
                .ok_or_else(|| PromptCompileError::UnknownInput {
                    node: node_id.clone(),
                    input: name.clone(),
                })?;
        if input.hidden {
            continue;
        }
        let binding = if let Some((source, output_index)) = decode_link(value) {
            if !prompt.0.contains_key(&source) {
                return Err(PromptCompileError::UnknownLink {
                    node: node_id.clone(),
                    input: name.clone(),
                    source_node: source,
                });
            }
            let source_descriptor = descriptors.get(&source).copied().ok_or_else(|| {
                PromptCompileError::UnknownLink {
                    node: node_id.clone(),
                    input: name.clone(),
                    source_node: source.clone(),
                }
            })?;
            let output = source_descriptor.outputs.get(output_index).ok_or_else(|| {
                PromptCompileError::UnknownOutput {
                    node: node_id.clone(),
                    input: name.clone(),
                    source_node: source.clone(),
                    output_index,
                }
            })?;
            let list_compatible = !output.is_list || input.mode != InputMode::Scalar;
            if !input.value_type.accepts(&output.value_type) || !list_compatible {
                return Err(PromptCompileError::IncompatibleLink {
                    node: node_id.clone(),
                    input: name.clone(),
                    source_node: source,
                    output_index,
                });
            }
            InputBinding::Link {
                source,
                output_index,
                lazy: input.lazy,
                mode: input.mode,
            }
        } else {
            let shape_valid = match input.mode {
                InputMode::List => value.as_array().is_some_and(|values| {
                    values
                        .iter()
                        .all(|value| input.value_type.accepts_literal(value))
                }),
                InputMode::Mapped => value.as_array().map_or_else(
                    || input.value_type.accepts_literal(value),
                    |values| {
                        values
                            .iter()
                            .all(|value| input.value_type.accepts_literal(value))
                    },
                ),
                InputMode::Scalar => input.value_type.accepts_literal(value),
            };
            if !input.allows_literal || !shape_valid {
                return Err(PromptCompileError::InvalidLiteral {
                    node: node_id.clone(),
                    input: name.clone(),
                });
            }
            InputBinding::Literal {
                value: value.clone(),
            }
        };
        compiled.insert(name.clone(), binding);
    }
    Ok(compiled)
}

fn inject_hidden_inputs(
    node_id: &NodeId,
    descriptor: &RuntimeNodeDescriptor,
    prompt: &ApiPrompt,
    extra_data: &BTreeMap<String, Value>,
    inputs: &mut BTreeMap<String, InputBinding>,
) -> Result<(), PromptCompileError> {
    for (name, input) in &descriptor.inputs {
        if !input.hidden {
            continue;
        }
        let value = match name.as_str() {
            "prompt" => serde_json::to_value(prompt),
            "extra_pnginfo" => Ok(extra_data
                .get("extra_pnginfo")
                .cloned()
                .unwrap_or(Value::Null)),
            "unique_id" => Ok(Value::String(node_id.0.clone())),
            _ if !input.required => continue,
            _ => {
                return Err(PromptCompileError::UnsupportedHiddenInput {
                    node: node_id.clone(),
                    input: name.clone(),
                });
            }
        }
        .map_err(|_| PromptCompileError::InvalidLiteral {
            node: node_id.clone(),
            input: name.clone(),
        })?;
        inputs.insert(name.clone(), InputBinding::Literal { value });
    }
    Ok(())
}

fn decode_link(value: &Value) -> Option<(NodeId, usize)> {
    let values = value.as_array()?;
    if values.len() != 2 {
        return None;
    }
    let source = values.first()?.as_str()?;
    let output_index = usize::try_from(values.get(1)?.as_u64()?).ok()?;
    Some((NodeId(source.to_owned()), output_index))
}

fn topological_order(
    nodes: &BTreeMap<NodeId, CompiledNode>,
) -> Result<Vec<NodeId>, PromptCompileError> {
    let mut incoming = nodes
        .keys()
        .map(|identifier| (identifier.clone(), 0_usize))
        .collect::<BTreeMap<_, _>>();
    let mut outgoing = BTreeMap::<NodeId, BTreeSet<NodeId>>::new();
    for node in nodes.values() {
        let dependencies = node
            .inputs
            .values()
            .filter_map(|input| match input {
                InputBinding::Link { source, .. } => Some(source),
                InputBinding::Literal { .. } => None,
            })
            .collect::<BTreeSet<_>>();
        if let Some(count) = incoming.get_mut(&node.id) {
            *count = dependencies.len();
        }
        for dependency in dependencies {
            outgoing
                .entry(dependency.clone())
                .or_default()
                .insert(node.id.clone());
        }
    }
    let mut ready = incoming
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(identifier, _)| identifier.clone())
        .collect::<BTreeSet<_>>();
    let mut ordered = Vec::with_capacity(nodes.len());
    while let Some(identifier) = ready.pop_first() {
        ordered.push(identifier.clone());
        if let Some(dependents) = outgoing.get(&identifier) {
            for dependent in dependents {
                if let Some(count) = incoming.get_mut(dependent) {
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        ready.insert(dependent.clone());
                    }
                }
            }
        }
    }
    if ordered.len() != nodes.len() {
        return Err(PromptCompileError::Cycle(
            incoming
                .into_iter()
                .filter(|(_, count)| *count > 0)
                .map(|(identifier, _)| identifier)
                .collect(),
        ));
    }
    Ok(ordered)
}

fn static_required_nodes(
    nodes: &BTreeMap<NodeId, CompiledNode>,
    output_nodes: &[NodeId],
) -> BTreeSet<NodeId> {
    let mut required = BTreeSet::new();
    let mut pending = output_nodes.to_vec();
    while let Some(identifier) = pending.pop() {
        if !required.insert(identifier.clone()) {
            continue;
        }
        if let Some(node) = nodes.get(&identifier) {
            pending.extend(node.inputs.values().filter_map(|input| match input {
                InputBinding::Link {
                    source,
                    lazy: false,
                    ..
                } => Some(source.clone()),
                InputBinding::Literal { .. } | InputBinding::Link { lazy: true, .. } => None,
            }));
        }
    }
    required
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::json;

    fn descriptor(class_type: &str, output_node: bool) -> RuntimeNodeDescriptor {
        RuntimeNodeDescriptor {
            class_type: class_type.to_owned(),
            implementation_version: "1".to_owned(),
            inputs: BTreeMap::new(),
            outputs: vec![RuntimeOutputDescriptor {
                value_type: ValueType::Number,
                is_list: false,
            }],
            output_node,
            availability: RuntimeAvailability::Native,
            effect: EffectClass::Pure,
            cache: RuntimeCachePolicy::InputIdentity,
        }
    }

    fn submission(nodes: BTreeMap<NodeId, PromptNode>) -> PromptSubmission {
        PromptSubmission {
            prompt: ApiPrompt(nodes),
            prompt_id: Some(PromptId(Uuid::nil())),
            client_id: Some("client".to_owned()),
            number: Some(7.0),
            extra_data: BTreeMap::from([("workflow".to_owned(), json!({"version": 1}))]),
            unknown: BTreeMap::from([("future".to_owned(), json!(true))]),
        }
    }

    #[test]
    fn val_domain_004_prompt_graph_and_lazy_dependencies_compile_deterministically()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut source = descriptor("Source", false);
        source.outputs[0].is_list = true;
        let mut choose = descriptor("Choose", true);
        choose.inputs.insert(
            "condition".to_owned(),
            RuntimeInputDescriptor {
                value_type: ValueType::Boolean,
                required: true,
                hidden: false,
                lazy: false,
                mode: InputMode::Scalar,
                allows_literal: true,
            },
        );
        choose.inputs.insert(
            "value".to_owned(),
            RuntimeInputDescriptor {
                value_type: ValueType::Number,
                required: true,
                hidden: false,
                lazy: true,
                mode: InputMode::Mapped,
                allows_literal: false,
            },
        );
        let mut registry = NativeNodeRegistry::default();
        registry.register_descriptor(source)?;
        registry.register_descriptor(choose)?;
        let plan = PromptCompiler::new(&registry).compile(submission(BTreeMap::from([
            (
                NodeId::from("001"),
                PromptNode {
                    class_type: "Source".to_owned(),
                    inputs: BTreeMap::new(),
                    unknown: BTreeMap::new(),
                },
            ),
            (
                NodeId::from("output"),
                PromptNode {
                    class_type: "Choose".to_owned(),
                    inputs: BTreeMap::from([
                        ("condition".to_owned(), json!(true)),
                        ("value".to_owned(), json!(["001", 0])),
                    ]),
                    unknown: BTreeMap::from([("future_node_field".to_owned(), json!(9))]),
                },
            ),
        ])))?;
        assert_eq!(
            plan.topological_order,
            [NodeId::from("001"), NodeId::from("output")]
        );
        assert_eq!(
            plan.static_required_nodes,
            BTreeSet::from([NodeId::from("output")])
        );
        assert_eq!(plan.unknown["future"], true);
        assert_eq!(
            plan.nodes[&NodeId::from("output")].unknown["future_node_field"],
            9
        );
        Ok(())
    }

    #[test]
    fn val_domain_004_invalid_graphs_fail_with_node_addressable_errors()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut output = descriptor("Output", true);
        output.inputs.insert(
            "value".to_owned(),
            RuntimeInputDescriptor {
                value_type: ValueType::Number,
                required: true,
                hidden: false,
                lazy: false,
                mode: InputMode::Scalar,
                allows_literal: false,
            },
        );
        let mut registry = NativeNodeRegistry::default();
        registry.register_descriptor(output)?;
        let error = PromptCompiler::new(&registry)
            .compile(submission(BTreeMap::from([(
                NodeId::from("out"),
                PromptNode {
                    class_type: "Output".to_owned(),
                    inputs: BTreeMap::from([("value".to_owned(), json!(["missing", 0]))]),
                    unknown: BTreeMap::new(),
                },
            )])))
            .expect_err("unknown link must fail");
        assert!(matches!(error, PromptCompileError::UnknownLink { .. }));
        Ok(())
    }

    #[test]
    fn val_domain_004_hidden_inputs_are_host_injected() -> Result<(), Box<dyn std::error::Error>> {
        let mut output = descriptor("Output", true);
        for (name, value_type) in [
            ("prompt", ValueType::Custom("PROMPT".to_owned())),
            (
                "extra_pnginfo",
                ValueType::Custom("EXTRA_PNGINFO".to_owned()),
            ),
            ("unique_id", ValueType::String),
        ] {
            output.inputs.insert(
                name.to_owned(),
                RuntimeInputDescriptor {
                    value_type,
                    required: true,
                    hidden: true,
                    lazy: false,
                    mode: InputMode::Scalar,
                    allows_literal: false,
                },
            );
        }
        let mut registry = NativeNodeRegistry::default();
        registry.register_descriptor(output)?;
        let mut host_submission = submission(BTreeMap::from([(
            NodeId::from("out"),
            PromptNode {
                class_type: "Output".to_owned(),
                inputs: BTreeMap::from([
                    ("prompt".to_owned(), json!("untrusted")),
                    ("extra_pnginfo".to_owned(), json!("untrusted")),
                    ("unique_id".to_owned(), json!("untrusted")),
                ]),
                unknown: BTreeMap::new(),
            },
        )]));
        host_submission.extra_data.insert(
            "extra_pnginfo".to_owned(),
            json!({"workflow": {"id": "fixture"}}),
        );
        let plan = PromptCompiler::new(&registry).compile(host_submission)?;
        let inputs = &plan.nodes[&NodeId::from("out")].inputs;
        assert_eq!(
            inputs["unique_id"],
            InputBinding::Literal {
                value: json!("out")
            }
        );
        assert_eq!(
            inputs["extra_pnginfo"],
            InputBinding::Literal {
                value: json!({"workflow": {"id": "fixture"}})
            }
        );
        let InputBinding::Literal { value: prompt } = &inputs["prompt"] else {
            return Err("prompt hidden input was not injected".into());
        };
        assert!(prompt.get("out").is_some());

        let mut provider = descriptor("Provider", true);
        provider.inputs.insert(
            "auth_token_comfy_org".to_owned(),
            RuntimeInputDescriptor {
                value_type: ValueType::String,
                required: true,
                hidden: true,
                lazy: false,
                mode: InputMode::Scalar,
                allows_literal: false,
            },
        );
        let mut provider_registry = NativeNodeRegistry::default();
        provider_registry.register_descriptor(provider)?;
        let provider_error = PromptCompiler::new(&provider_registry)
            .compile(submission(BTreeMap::from([(
                NodeId::from("provider"),
                PromptNode {
                    class_type: "Provider".to_owned(),
                    inputs: BTreeMap::from([(
                        "auth_token_comfy_org".to_owned(),
                        json!("untrusted-secret"),
                    )]),
                    unknown: BTreeMap::new(),
                },
            )])))
            .expect_err("raw prompt data must not inject an authorization secret");
        assert!(matches!(
            provider_error,
            PromptCompileError::UnsupportedHiddenInput { .. }
        ));
        Ok(())
    }

    pub(crate) fn val_domain_004_prompt_case_results()
    -> Result<Vec<(&'static str, bool)>, Box<dyn std::error::Error>> {
        val_domain_004_prompt_graph_and_lazy_dependencies_compile_deterministically()?;
        val_domain_004_invalid_graphs_fail_with_node_addressable_errors()?;
        val_domain_004_hidden_inputs_are_host_injected()?;
        Ok(vec![
            ("prompt_graph_lazy_demand_closure", true),
            ("prompt_node_addressable_validation", true),
            ("prompt_host_hidden_input_injection", true),
        ])
    }
}
