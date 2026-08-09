use crate::executor::NativeNodeRegistry;
use comfy_nodes::{
    NativeNodeDescriptor, NativePortCardinality, NativePrimitive, NativePrimitiveType,
    NativeTypeUnion, NativeValue, NativeValueType,
};
use comfy_types::{ApiPrompt, NodeId, PromptId, PromptNode, PromptSubmission};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;
use uuid::Uuid;

pub const MAX_PROMPT_NODES: usize = 100_000;
pub const MAX_PROMPT_INPUTS: usize = 1_000_000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InputBinding {
    Literal {
        value: NativeValue,
    },
    Link {
        source: NodeId,
        output_index: usize,
        lazy: bool,
        mode: NativePortCardinality,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompiledNode {
    pub id: NodeId,
    pub class_type: String,
    pub descriptor: NativeNodeDescriptor,
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
    ) -> Result<BTreeMap<NodeId, &'a NativeNodeDescriptor>, PromptCompileError> {
        let mut descriptors = BTreeMap::new();
        for (node_id, node) in &prompt.0 {
            let descriptor = self.registry.descriptor(&node.class_type).ok_or_else(|| {
                PromptCompileError::UnknownNodeType {
                    node: node_id.clone(),
                    class_type: node.class_type.clone(),
                }
            })?;
            if let Some(reason) = self.registry.unavailable_reason(&node.class_type) {
                return Err(PromptCompileError::UnavailableNode {
                    node: node_id.clone(),
                    class_type: node.class_type.clone(),
                    reason: reason.to_owned(),
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
    descriptor: &NativeNodeDescriptor,
    prompt: &ApiPrompt,
    descriptors: &BTreeMap<NodeId, &NativeNodeDescriptor>,
) -> Result<BTreeMap<String, InputBinding>, PromptCompileError> {
    for input in &descriptor.inputs {
        if input.required && !input.hidden && !prompt_node.inputs.contains_key(&input.name) {
            return Err(PromptCompileError::MissingInput {
                node: node_id.clone(),
                input: input.name.clone(),
            });
        }
    }
    for (dynamic_index, dynamic) in descriptor.dynamic_inputs.iter().enumerate() {
        for name in required_dynamic_input_names(descriptor, dynamic_index, dynamic)? {
            if dynamic.input.required
                && !dynamic.input.hidden
                && !prompt_node.inputs.contains_key(&name)
            {
                return Err(PromptCompileError::MissingInput {
                    node: node_id.clone(),
                    input: name,
                });
            }
        }
    }
    let mut compiled = BTreeMap::new();
    for (name, value) in &prompt_node.inputs {
        let input = resolve_input_descriptor(descriptor, name).ok_or_else(|| {
            PromptCompileError::UnknownInput {
                node: node_id.clone(),
                input: name.clone(),
            }
        })?;
        if input.hidden {
            continue;
        }
        let source_schema = resolve_input_schema(descriptor, name);
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
            let list_compatible =
                !output.is_list || input.cardinality != NativePortCardinality::Scalar;
            if !type_union_accepts_output(&input.accepted_types, &output.produced_type)
                || !list_compatible
            {
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
                mode: input.cardinality,
            }
        } else {
            let native_value = native_literal(value, &input.accepted_types, input.cardinality);
            if !input.allows_literal || native_value.is_none() {
                return Err(PromptCompileError::InvalidLiteral {
                    node: node_id.clone(),
                    input: name.clone(),
                });
            }
            let value = native_value.ok_or_else(|| PromptCompileError::InvalidLiteral {
                node: node_id.clone(),
                input: name.clone(),
            })?;
            if source_schema.as_ref().is_some_and(|schema| {
                !comfy_nodes::native_value_matches_input_schema(&value, schema)
            }) {
                return Err(PromptCompileError::InvalidLiteral {
                    node: node_id.clone(),
                    input: name.clone(),
                });
            }
            InputBinding::Literal { value }
        };
        compiled.insert(name.clone(), binding);
    }
    Ok(compiled)
}

fn inject_hidden_inputs(
    node_id: &NodeId,
    descriptor: &NativeNodeDescriptor,
    prompt: &ApiPrompt,
    extra_data: &BTreeMap<String, Value>,
    inputs: &mut BTreeMap<String, InputBinding>,
) -> Result<(), PromptCompileError> {
    for input in &descriptor.inputs {
        if !input.hidden {
            continue;
        }
        let value = match input.name.as_str() {
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
                    input: input.name.clone(),
                });
            }
        }
        .map_err(|_| PromptCompileError::InvalidLiteral {
            node: node_id.clone(),
            input: input.name.clone(),
        })?;
        let value = native_literal(&value, &input.accepted_types, NativePortCardinality::Scalar)
            .ok_or_else(|| PromptCompileError::InvalidLiteral {
                node: node_id.clone(),
                input: input.name.clone(),
            })?;
        if resolve_input_schema(descriptor, &input.name)
            .as_ref()
            .is_some_and(|schema| !comfy_nodes::native_value_matches_input_schema(&value, schema))
        {
            return Err(PromptCompileError::InvalidLiteral {
                node: node_id.clone(),
                input: input.name.clone(),
            });
        }
        inputs.insert(input.name.clone(), InputBinding::Literal { value });
    }
    Ok(())
}

pub(crate) fn resolve_input_descriptor(
    descriptor: &NativeNodeDescriptor,
    name: &str,
) -> Option<comfy_nodes::NativeInputDescriptor> {
    if let Some(input) = descriptor.inputs.iter().find(|input| input.name == name) {
        return Some(input.clone());
    }
    for (dynamic_index, dynamic) in descriptor.dynamic_inputs.iter().enumerate() {
        if dynamic.name_template == "{name}"
            && descriptor
                .source_schema
                .as_ref()
                .and_then(|schema| schema.dynamic_inputs.get(dynamic_index))
                .is_some_and(|schema| schema.names.iter().any(|candidate| candidate == name))
        {
            let mut input = dynamic.input.clone();
            input.name = name.to_owned();
            return Some(input);
        }
        let Some((prefix, suffix)) = dynamic.name_template.split_once("{index}") else {
            continue;
        };
        let Some(index) = name
            .strip_prefix(prefix)
            .and_then(|value| value.strip_suffix(suffix))
            .and_then(|value| value.parse::<u32>().ok())
        else {
            continue;
        };
        let Some(end) = dynamic.start_index.checked_add(dynamic.maximum_count) else {
            continue;
        };
        if index >= dynamic.start_index && index < end {
            let mut input = dynamic.input.clone();
            input.name = name.to_owned();
            return Some(input);
        }
    }
    None
}

fn resolve_input_schema(
    descriptor: &NativeNodeDescriptor,
    name: &str,
) -> Option<comfy_nodes::NativeInputSchemaMetadata> {
    let source_schema = descriptor.source_schema.as_ref()?;
    if let Some((index, _)) = descriptor
        .inputs
        .iter()
        .enumerate()
        .find(|(_, input)| input.name == name)
    {
        return source_schema.inputs.get(index).cloned();
    }
    for (index, dynamic) in descriptor.dynamic_inputs.iter().enumerate() {
        if dynamic.name_template == "{name}" {
            let dynamic_schema = source_schema.dynamic_inputs.get(index)?;
            if dynamic_schema
                .names
                .iter()
                .any(|candidate| candidate == name)
            {
                let mut schema = (*dynamic_schema.input).clone();
                schema.name = name.to_owned();
                return Some(schema);
            }
            continue;
        }
        let Some((prefix, suffix)) = dynamic.name_template.split_once("{index}") else {
            continue;
        };
        let Some(value) = name
            .strip_prefix(prefix)
            .and_then(|value| value.strip_suffix(suffix))
            .and_then(|value| value.parse::<u32>().ok())
        else {
            continue;
        };
        let end = dynamic.start_index.checked_add(dynamic.maximum_count)?;
        if value >= dynamic.start_index && value < end {
            let mut schema = (*source_schema.dynamic_inputs.get(index)?.input).clone();
            schema.name = name.to_owned();
            return Some(schema);
        }
    }
    None
}

fn required_dynamic_input_names(
    descriptor: &NativeNodeDescriptor,
    dynamic_index: usize,
    dynamic: &comfy_nodes::NativeDynamicInputDescriptor,
) -> Result<Vec<String>, PromptCompileError> {
    if dynamic.name_template == "{name}" {
        let names = descriptor
            .source_schema
            .as_ref()
            .and_then(|schema| schema.dynamic_inputs.get(dynamic_index))
            .map(|schema| schema.names.clone())
            .ok_or_else(|| {
                PromptCompileError::InvalidRuntimeDescriptor(descriptor.class_type.clone())
            })?;
        return Ok(names
            .into_iter()
            .take(dynamic.minimum_count as usize)
            .collect());
    }
    (0..dynamic.minimum_count)
        .map(|offset| {
            dynamic
                .start_index
                .checked_add(offset)
                .map(|index| dynamic.name_template.replace("{index}", &index.to_string()))
                .ok_or_else(|| {
                    PromptCompileError::InvalidRuntimeDescriptor(descriptor.class_type.clone())
                })
        })
        .collect()
}

fn type_union_accepts_output(union: &NativeTypeUnion, output: &NativeValueType) -> bool {
    union.members().iter().any(|expected| {
        expected == &NativeValueType::Any
            || expected == output
            || matches!(
                (expected, output),
                (
                    NativeValueType::Primitive(NativePrimitiveType::Number),
                    NativeValueType::Primitive(NativePrimitiveType::Integer)
                )
            )
    })
}

fn native_literal(
    value: &Value,
    accepted_types: &NativeTypeUnion,
    cardinality: NativePortCardinality,
) -> Option<NativeValue> {
    match cardinality {
        NativePortCardinality::List => value.as_array().and_then(|values| {
            values
                .iter()
                .map(|value| native_scalar_literal(value, accepted_types))
                .collect::<Option<Vec<_>>>()
                .map(|values| NativeValue::List { values })
        }),
        NativePortCardinality::Mapped => value.as_array().map_or_else(
            || native_scalar_literal(value, accepted_types),
            |values| {
                values
                    .iter()
                    .map(|value| native_scalar_literal(value, accepted_types))
                    .collect::<Option<Vec<_>>>()
                    .map(|values| NativeValue::List { values })
            },
        ),
        NativePortCardinality::Scalar => native_scalar_literal(value, accepted_types),
    }
}

fn native_scalar_literal(value: &Value, accepted_types: &NativeTypeUnion) -> Option<NativeValue> {
    let primitive = match value {
        Value::Null => Some(NativePrimitive::Null),
        Value::Bool(value) => Some(NativePrimitive::Boolean(*value)),
        Value::Number(value) => value
            .as_i64()
            .map(NativePrimitive::Integer)
            .or_else(|| value.as_u64().map(NativePrimitive::UnsignedInteger))
            .or_else(|| {
                value
                    .as_f64()
                    .filter(|value| value.is_finite())
                    .map(NativePrimitive::Number)
            }),
        Value::String(value) => Some(NativePrimitive::String(value.clone())),
        Value::Array(_) | Value::Object(_) => None,
    };
    if let Some(primitive) = primitive {
        let native = NativeValue::Primitive { value: primitive };
        if accepted_types.accepts(&native)
            || accepted_types.members() == [NativeValueType::Any]
            || matches!(
                &native,
                NativeValue::Primitive {
                    value: NativePrimitive::Integer(_) | NativePrimitive::UnsignedInteger(_)
                }
            ) && accepted_types
                .members()
                .contains(&NativeValueType::Primitive(NativePrimitiveType::Number))
        {
            return Some(native);
        }
    }
    if accepted_types.members() == [NativeValueType::Any]
        || accepted_types
            .members()
            .contains(&NativeValueType::PreservedUnknown)
    {
        Some(NativeValue::PreservedUnknown {
            type_name: "sim.json@1".to_owned(),
            value: value.clone(),
        })
    } else {
        None
    }
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
    use comfy_nodes::{
        LEGACY_NATIVE_NODE_CONTRACT_SCHEMA_VERSION, NativeCachePolicy,
        NativeDynamicInputDescriptor, NativeEffectClass, NativeInputDescriptor,
        NativeNodeContractError, NativeOutputDescriptor, NativePrimitiveType,
    };
    use serde_json::json;

    fn descriptor(
        class_type: &str,
        output_node: bool,
    ) -> Result<NativeNodeDescriptor, NativeNodeContractError> {
        Ok(NativeNodeDescriptor {
            schema_version: LEGACY_NATIVE_NODE_CONTRACT_SCHEMA_VERSION,
            class_type: class_type.to_owned(),
            implementation_version: "1".to_owned(),
            source_schema: None,
            inputs: Vec::new(),
            dynamic_inputs: Vec::new(),
            outputs: vec![NativeOutputDescriptor {
                name: "value".to_owned(),
                produced_type: NativeValueType::Primitive(NativePrimitiveType::Number),
                is_list: false,
            }],
            output_node,
            effect: NativeEffectClass::Pure,
            cache: NativeCachePolicy::InputIdentity,
        })
    }

    fn input(
        name: &str,
        value_type: NativeValueType,
        lazy: bool,
        cardinality: NativePortCardinality,
        allows_literal: bool,
    ) -> Result<NativeInputDescriptor, NativeNodeContractError> {
        Ok(NativeInputDescriptor {
            name: name.to_owned(),
            accepted_types: NativeTypeUnion::new([value_type])?,
            required: true,
            hidden: false,
            lazy,
            cardinality,
            allows_literal,
        })
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
        let mut source = descriptor("Source", false)?;
        source.outputs[0].is_list = true;
        let mut choose = descriptor("Choose", true)?;
        choose.inputs.push(input(
            "condition",
            NativeValueType::Primitive(NativePrimitiveType::Boolean),
            false,
            NativePortCardinality::Scalar,
            true,
        )?);
        choose.inputs.push(input(
            "value",
            NativeValueType::Primitive(NativePrimitiveType::Number),
            true,
            NativePortCardinality::Mapped,
            false,
        )?);
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
        let mut output = descriptor("Output", true)?;
        output.inputs.push(input(
            "value",
            NativeValueType::Primitive(NativePrimitiveType::Number),
            false,
            NativePortCardinality::Scalar,
            false,
        )?);
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
        let mut output = descriptor("Output", true)?;
        for (name, value_type) in [
            ("prompt", NativeValueType::Any),
            ("extra_pnginfo", NativeValueType::Any),
            (
                "unique_id",
                NativeValueType::Primitive(NativePrimitiveType::String),
            ),
        ] {
            let mut descriptor = input(
                name,
                value_type,
                false,
                NativePortCardinality::Scalar,
                false,
            )?;
            descriptor.hidden = true;
            output.inputs.push(descriptor);
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
                value: NativeValue::Primitive {
                    value: NativePrimitive::String("out".to_owned())
                }
            }
        );
        assert!(matches!(
            &inputs["extra_pnginfo"],
            InputBinding::Literal {
                value: NativeValue::PreservedUnknown { value, .. }
            } if value == &json!({"workflow": {"id": "fixture"}})
        ));
        let InputBinding::Literal { value: prompt } = &inputs["prompt"] else {
            return Err("prompt hidden input was not injected".into());
        };
        assert!(matches!(
            prompt,
            NativeValue::PreservedUnknown { value, .. }
                if value.get("out").is_some()
        ));

        let mut provider = descriptor("Provider", true)?;
        let mut secret = input(
            "auth_token_comfy_org",
            NativeValueType::Primitive(NativePrimitiveType::String),
            false,
            NativePortCardinality::Scalar,
            false,
        )?;
        secret.hidden = true;
        provider.inputs.push(secret);
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

    #[test]
    fn integer_literals_preserve_u64_and_signed_values_exactly()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut output = descriptor("Output", true)?;
        output.inputs.push(input(
            "seed",
            NativeValueType::Primitive(NativePrimitiveType::Integer),
            false,
            NativePortCardinality::Scalar,
            true,
        )?);
        let mut registry = NativeNodeRegistry::default();
        registry.register_descriptor(output)?;
        for (literal, expected) in [
            (json!(u64::MAX), NativePrimitive::UnsignedInteger(u64::MAX)),
            (json!(-7), NativePrimitive::Integer(-7)),
        ] {
            let plan = PromptCompiler::new(&registry).compile(submission(BTreeMap::from([(
                NodeId::from("out"),
                PromptNode {
                    class_type: "Output".to_owned(),
                    inputs: BTreeMap::from([("seed".to_owned(), literal)]),
                    unknown: BTreeMap::new(),
                },
            )])))?;
            assert_eq!(
                plan.nodes[&NodeId::from("out")].inputs["seed"],
                InputBinding::Literal {
                    value: NativeValue::Primitive { value: expected }
                }
            );
        }
        Ok(())
    }

    #[test]
    fn prompt_schema_constraints_fail_before_execution() -> Result<(), Box<dyn std::error::Error>> {
        let mut bounded = descriptor("Bounded", true)?;
        bounded.schema_version = comfy_nodes::NATIVE_NODE_CONTRACT_SCHEMA_VERSION;
        bounded.inputs = vec![
            input(
                "seed",
                NativeValueType::Primitive(NativePrimitiveType::Integer),
                false,
                NativePortCardinality::Scalar,
                true,
            )?,
            input(
                "weights",
                NativeValueType::Primitive(NativePrimitiveType::Number),
                false,
                NativePortCardinality::List,
                true,
            )?,
        ];
        let mut source_schema = comfy_nodes::NativeDescriptorSchemaMetadata::compatibility(
            comfy_nodes::NativeSchemaProvenance::SourceV3,
            [
                ("seed".to_owned(), "INT".to_owned()),
                ("weights".to_owned(), "FLOAT".to_owned()),
            ],
            std::iter::empty(),
            [("value".to_owned(), "FLOAT".to_owned())],
        );
        source_schema.inputs[0].default =
            Some(comfy_nodes::NativeSchemaValue::UnsignedInteger { value: 3 });
        source_schema.inputs[0].minimum =
            Some(comfy_nodes::NativeSchemaValue::UnsignedInteger { value: 1 });
        source_schema.inputs[0].maximum =
            Some(comfy_nodes::NativeSchemaValue::UnsignedInteger { value: 5 });
        source_schema.inputs[0].choices = [1, 3, 5]
            .into_iter()
            .map(|value| comfy_nodes::NativeSchemaValue::UnsignedInteger { value })
            .collect();
        source_schema.inputs[1].minimum = Some(comfy_nodes::NativeSchemaValue::FiniteDecimal {
            value: "0.0".to_owned(),
        });
        source_schema.inputs[1].maximum = Some(comfy_nodes::NativeSchemaValue::FiniteDecimal {
            value: "1.0".to_owned(),
        });
        bounded.source_schema = Some(source_schema);
        let mut registry = NativeNodeRegistry::default();
        registry.register_descriptor(bounded)?;

        let compile = |seed, weights| {
            PromptCompiler::new(&registry).compile(submission(BTreeMap::from([(
                NodeId::from("out"),
                PromptNode {
                    class_type: "Bounded".to_owned(),
                    inputs: BTreeMap::from([
                        ("seed".to_owned(), seed),
                        ("weights".to_owned(), weights),
                    ]),
                    unknown: BTreeMap::new(),
                },
            )])))
        };
        compile(json!(3), json!([0.0, 0.5, 1.0]))?;
        for (seed, weights) in [
            (json!(0), json!([0.5])),
            (json!(4), json!([0.5])),
            (json!(6), json!([0.5])),
            (json!(3), json!([0.5, 1.1])),
        ] {
            assert!(matches!(
                compile(seed, weights),
                Err(PromptCompileError::InvalidLiteral { .. })
            ));
        }
        Ok(())
    }

    #[test]
    fn dynamic_input_resolution_checks_every_template() -> Result<(), Box<dyn std::error::Error>> {
        let mut dynamic = descriptor("Dynamic", true)?;
        let dynamic_input = input(
            "value",
            NativeValueType::Primitive(NativePrimitiveType::Number),
            false,
            NativePortCardinality::Scalar,
            true,
        )?;
        dynamic.dynamic_inputs = vec![
            NativeDynamicInputDescriptor {
                name_template: "first_{index}".to_owned(),
                start_index: 0,
                minimum_count: 0,
                maximum_count: 2,
                input: dynamic_input.clone(),
            },
            NativeDynamicInputDescriptor {
                name_template: "second_{index}".to_owned(),
                start_index: 4,
                minimum_count: 0,
                maximum_count: 2,
                input: dynamic_input,
            },
            NativeDynamicInputDescriptor {
                name_template: "{name}".to_owned(),
                start_index: 0,
                minimum_count: 1,
                maximum_count: 2,
                input: input(
                    "named_value",
                    NativeValueType::Primitive(NativePrimitiveType::Number),
                    false,
                    NativePortCardinality::Scalar,
                    true,
                )?,
            },
        ];
        let mut named_schema = comfy_nodes::NativeDynamicSchemaMetadata::compatibility(
            "{name}",
            0,
            1,
            2,
            comfy_nodes::NativeInputSchemaMetadata::compatibility("named_value", "FLOAT"),
        );
        named_schema.names = vec!["left".to_owned(), "right".to_owned()];
        dynamic.schema_version = comfy_nodes::NATIVE_NODE_CONTRACT_SCHEMA_VERSION;
        dynamic.source_schema = Some(comfy_nodes::NativeDescriptorSchemaMetadata::compatibility(
            comfy_nodes::NativeSchemaProvenance::SourceV3,
            std::iter::empty(),
            [
                comfy_nodes::NativeDynamicSchemaMetadata::compatibility(
                    "first_{index}",
                    0,
                    0,
                    2,
                    comfy_nodes::NativeInputSchemaMetadata::compatibility("value", "FLOAT"),
                ),
                comfy_nodes::NativeDynamicSchemaMetadata::compatibility(
                    "second_{index}",
                    4,
                    0,
                    2,
                    comfy_nodes::NativeInputSchemaMetadata::compatibility("value", "FLOAT"),
                ),
                named_schema,
            ],
            [("value".to_owned(), "FLOAT".to_owned())],
        ));
        dynamic.validate()?;
        assert_eq!(
            resolve_input_descriptor(&dynamic, "second_5").map(|input| input.name),
            Some("second_5".to_owned())
        );
        assert_eq!(
            resolve_input_descriptor(&dynamic, "right").map(|input| input.name),
            Some("right".to_owned())
        );
        assert!(resolve_input_descriptor(&dynamic, "unknown").is_none());
        Ok(())
    }

    pub(crate) fn val_domain_004_prompt_case_results()
    -> Result<Vec<(&'static str, bool)>, Box<dyn std::error::Error>> {
        val_domain_004_prompt_graph_and_lazy_dependencies_compile_deterministically()?;
        val_domain_004_invalid_graphs_fail_with_node_addressable_errors()?;
        val_domain_004_hidden_inputs_are_host_injected()?;
        integer_literals_preserve_u64_and_signed_values_exactly()?;
        dynamic_input_resolution_checks_every_template()?;
        Ok(vec![
            ("prompt_graph_lazy_demand_closure", true),
            ("prompt_node_addressable_validation", true),
            ("prompt_host_hidden_input_injection", true),
        ])
    }
}
