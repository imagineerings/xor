use comfy_model::{
    MappedModelWeights, PatchGraph, PatchPayload, PatchTensor, PatchValueTransform,
    SemanticPatchOperation,
    conditioning::{
        ConditioningConstant, ConditioningControlReference, ConditioningEntry,
        ConditioningEntryOptions, ConditioningHookReference, ConditioningIdentity,
        ConditioningMask, ConditioningReferences, ConditioningRegion, ConditioningSet,
        ConditioningValue, ConditioningWindow,
    },
    controlnet::{
        ControlBase, ControlChain, ControlHintPreprocess, ControlModelBinding,
        ControlModelExecutor, ControlModelInput, ControlNet, ControlNetError, ControlNode,
        ControlPercentWindow, ControlResult, ControlTensorBinding, StrengthType,
    },
    generated_native_diffusion::{sd15_latent_format_identity, sd15_model_family_identity},
};
use comfy_runtime::{
    AttemptEventKind, AttemptState, CanonicalConditioningCacheIdentities,
    CanonicalNativeDiffusionCacheIdentities, NativeDiffusionBundle, NativeDiffusionProvider,
    NativeImageExecutor, NativeImageRuntimeError, PreboundControlExecution,
    compile_native_diffusion_workflow,
};
use comfy_sampler::{
    DiscreteSamplingProfile, GuidanceBranch, GuidanceDenoiser, GuidanceError, GuidanceEvaluation,
    GuidanceHook, GuidanceHookContext, GuidanceOptions, GuidancePredictions,
    PredictionInterpretation, SamplingPlan, SamplingProfile, SamplingProfileIdentity,
    execute_guidance,
};
use comfy_tensor::{
    CancellationToken, CpuBackend, CpuWorkspaceAuthority, DType, DeviceId, ExecutionContext,
    ResizeMode, StreamId, Tensor, TensorError,
    generated_native_diffusion::{tensor_from_f32, tensor_to_f32},
};
use comfy_test_support::NativeDiffusionFixture;
use comfy_types::{AttemptId, ProfileId};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fs,
    path::Path,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
use uuid::Uuid;

const WORKFLOW: &[u8] = include_bytes!("../fixtures/native_diffusion/workflow.json");
const MEMORY_LIMIT: u64 = 2 * 1024 * 1024 * 1024;
const CONDITIONING_TASK: &str = "comfy-parity-conditioning-value-foundation";
const GUIDANCE_TASK: &str = "comfy-parity-conditioning-guidance-adapter";
const FIXTURE_CONTROL_EXECUTOR_DIGEST: &str =
    "749e2d208a8bab88e36f9b096bb23faca0a27fb66d8cccdba8f11e17feee5e75";
const CONTRACT_CASES: [(&str, &str, &str); 17] = [
    (
        "conditioning-conditioning-value-conds-condregular-505e5b9e",
        CONDITIONING_TASK,
        "conditioning-conditioning-value-conds-condregular-505e5b9e:regular-repeat-concat-size",
    ),
    (
        "conditioning-conditioning-value-conds-condnoiseshape-7f11dbb1",
        CONDITIONING_TASK,
        "conditioning-conditioning-value-conds-condnoiseshape-7f11dbb1:noise-shape-region-repeat",
    ),
    (
        "conditioning-conditioning-value-conds-condcrossattn-4d921d69",
        CONDITIONING_TASK,
        "conditioning-conditioning-value-conds-condcrossattn-4d921d69:cross-attention-lcm-concat",
    ),
    (
        "conditioning-conditioning-value-conds-condconstant-0e559aad",
        CONDITIONING_TASK,
        "conditioning-conditioning-value-conds-condconstant-0e559aad:constant-equality-identity",
    ),
    (
        "conditioning-conditioning-value-conds-condlist-21ce2116",
        CONDITIONING_TASK,
        "conditioning-conditioning-value-conds-condlist-21ce2116:list-itemwise-process-concat-size",
    ),
    (
        "conditioning-guidance-samplers-get-area-and-mult-14d8dec2",
        GUIDANCE_TASK,
        "conditioning-guidance-samplers-get-area-and-mult-14d8dec2:resolved-area-mask-window-weight",
    ),
    (
        "conditioning-guidance-samplers-calc-cond-batch-23aa4a02",
        GUIDANCE_TASK,
        "conditioning-guidance-samplers-calc-cond-batch-23aa4a02:compatible-batch-regional-accumulation",
    ),
    (
        "conditioning-guidance-samplers-sampling-function-ef25ad1d",
        GUIDANCE_TASK,
        "conditioning-guidance-samplers-sampling-function-ef25ad1d:cfg-skip-and-hook-pipeline",
    ),
    (
        "conditioning-guidance-hook-sampler-helpers-prepare-mask-048488c7",
        GUIDANCE_TASK,
        "conditioning-guidance-hook-sampler-helpers-prepare-mask-048488c7:mask-normalize-broadcast",
    ),
    (
        "conditioning-guidance-hook-sampler-helpers-get-models-from-cond-1be91d68",
        GUIDANCE_TASK,
        "conditioning-guidance-hook-sampler-helpers-get-models-from-cond-1be91d68:typed-control-hook-reference-projection",
    ),
    (
        "conditioning-guidance-hook-sampler-helpers-convert-cond-e8752d85",
        GUIDANCE_TASK,
        "conditioning-guidance-hook-sampler-helpers-convert-cond-e8752d85:typed-entry-set-conversion",
    ),
    (
        "conditioning-guidance-hook-sampler-helpers-get-additional-models-7ba596bf",
        GUIDANCE_TASK,
        "conditioning-guidance-hook-sampler-helpers-get-additional-models-7ba596bf:prebound-additional-model-identity",
    ),
    (
        "conditioning-guidance-hook-sampler-helpers-prepare-sampling-b141c606",
        GUIDANCE_TASK,
        "conditioning-guidance-hook-sampler-helpers-prepare-sampling-b141c606:prebound-bundle-load-and-execute",
    ),
    (
        "conditioning-guidance-hook-sampler-helpers-cleanup-models-6f147c97",
        GUIDANCE_TASK,
        "conditioning-guidance-hook-sampler-helpers-cleanup-models-6f147c97:scope-drop-workspace-convergence",
    ),
    (
        "conditioning-guidance-hook-hooks-hook-536ff505",
        GUIDANCE_TASK,
        "conditioning-guidance-hook-hooks-hook-536ff505:ordered-guidance-hook-phases",
    ),
    (
        "conditioning-guidance-hook-hooks-weighthook-03327446",
        GUIDANCE_TASK,
        "conditioning-guidance-hook-hooks-weighthook-03327446:weight-hook-patchgraph-delegation",
    ),
    (
        "conditioning-guidance-hook-patcher-extension-patcherinjection-116374da",
        GUIDANCE_TASK,
        "conditioning-guidance-hook-patcher-extension-patcherinjection-116374da:injection-hook-lifecycle-cancellation",
    ),
];

#[derive(Clone, Copy)]
enum FixtureControlMode {
    Success,
    Cancelled,
    AllocationFailed,
}

struct FixtureControlExecutor {
    calls: Arc<AtomicUsize>,
    mode: FixtureControlMode,
}

impl ControlModelExecutor for FixtureControlExecutor {
    fn execution_digest(&self) -> &str {
        FIXTURE_CONTROL_EXECUTOR_DIGEST
    }

    fn execute_controlnet(
        &self,
        _binding: &ControlModelBinding,
        input: &ControlModelInput,
        context: &ExecutionContext<'_>,
    ) -> Result<ControlResult, ControlNetError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        match self.mode {
            FixtureControlMode::Success => {
                context
                    .cancellation
                    .check()
                    .map_err(|_| ControlNetError::Cancelled)?;
                ControlResult::checked(vec![Some(input.hint.clone())], Vec::new(), Vec::new())
            }
            FixtureControlMode::Cancelled => Err(ControlNetError::Cancelled),
            FixtureControlMode::AllocationFailed => Err(ControlNetError::CanonicalTensor(
                Box::new(TensorError::AllocationFailed {
                    requested: 4096,
                    reason: "injected ControlNet allocation failure".to_owned(),
                }),
            )),
        }
    }

    fn execute_t2i_adapter(
        &self,
        _binding: &ControlModelBinding,
        _hint: &Tensor,
        _context: &ExecutionContext<'_>,
    ) -> Result<ControlResult, ControlNetError> {
        Err(ControlNetError::Invalid(
            "the SD15 integration fixture binds a ControlNet executor".to_owned(),
        ))
    }
}

fn tensor_digest(
    backend: &CpuBackend,
    tensor: &Tensor,
    context: &ExecutionContext<'_>,
) -> Result<String, Box<dyn Error>> {
    let mut hasher = Sha256::new();
    hasher.update(serde_json::to_vec(tensor.descriptor())?);
    for value in tensor_to_f32(backend, tensor, context)?.iter() {
        hasher.update(value.to_bits().to_le_bytes());
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn assert_f32_tensor(
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
    tensor: &Tensor,
    expected_shape: &[u64],
    expected_values: &[f32],
) -> Result<(), Box<dyn Error>> {
    assert_eq!(tensor.descriptor().shape(), expected_shape);
    let actual_values = tensor_to_f32(backend, tensor, context)?;
    assert_eq!(&*actual_values, expected_values);
    Ok(())
}

fn record_contract_case(
    executed_case_ids: &mut BTreeSet<&'static str>,
    contract_id: &str,
    expected_task: &str,
) -> Result<(), Box<dyn Error>> {
    let Some((_, task_id, case_id)) = CONTRACT_CASES
        .iter()
        .find(|(mapped_contract_id, _, _)| *mapped_contract_id == contract_id)
    else {
        return Err(format!("missing contract mapping for {contract_id}").into());
    };
    if *task_id != expected_task {
        return Err(format!("contract {contract_id} is associated with task {task_id}").into());
    }
    if !executed_case_ids.insert(*case_id) {
        return Err(format!("contract case {case_id} executed more than once").into());
    }
    Ok(())
}

fn assert_conditioning_value_contract_cases(
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
) -> Result<BTreeSet<&'static str>, Box<dyn Error>> {
    let mut executed_case_ids = BTreeSet::new();

    let regular =
        ConditioningValue::regular(tensor_from_f32(backend, &[2, 1], &[1.0, 2.0], context)?)?;
    assert_eq!(regular.size()?, vec![2_u64, 1]);
    let processed_regular = regular.process(5, None, backend, context)?;
    let ConditioningValue::Regular(processed_regular_tensor) = &processed_regular else {
        return Err("processed regular conditioning changed kind".into());
    };
    assert_f32_tensor(
        backend,
        context,
        processed_regular_tensor,
        &[5, 1],
        &[1.0, 2.0, 1.0, 2.0, 1.0],
    )?;
    let other_regular =
        ConditioningValue::regular(tensor_from_f32(backend, &[2, 1], &[3.0, 4.0], context)?)?;
    assert!(regular.can_concat(&other_regular));
    let concatenated_regular = regular.concat(&[other_regular], backend, context)?;
    let ConditioningValue::Regular(concatenated_regular_tensor) = &concatenated_regular else {
        return Err("concatenated regular conditioning changed kind".into());
    };
    assert_f32_tensor(
        backend,
        context,
        concatenated_regular_tensor,
        &[4, 1],
        &[1.0, 2.0, 3.0, 4.0],
    )?;
    record_contract_case(
        &mut executed_case_ids,
        "conditioning-conditioning-value-conds-condregular-505e5b9e",
        CONDITIONING_TASK,
    )?;

    let noise_values = (0..20).map(|value| value as f32).collect::<Vec<_>>();
    let noise = ConditioningValue::noise_shape(tensor_from_f32(
        backend,
        &[1, 1, 4, 5],
        &noise_values,
        context,
    )?)?;
    let region = ConditioningRegion::absolute(vec![2, 3], vec![1, 1])?.resolve(&[4, 5])?;
    let processed_noise = noise.process(2, Some(&region), backend, context)?;
    let ConditioningValue::NoiseShape(processed_noise_tensor) = &processed_noise else {
        return Err("processed noise-shape conditioning changed kind".into());
    };
    assert_f32_tensor(
        backend,
        context,
        processed_noise_tensor,
        &[2, 1, 2, 3],
        &[
            6.0, 7.0, 8.0, 11.0, 12.0, 13.0, 6.0, 7.0, 8.0, 11.0, 12.0, 13.0,
        ],
    )?;
    record_contract_case(
        &mut executed_case_ids,
        "conditioning-conditioning-value-conds-condnoiseshape-7f11dbb1",
        CONDITIONING_TASK,
    )?;

    let left_cross_attention = ConditioningValue::cross_attention(tensor_from_f32(
        backend,
        &[1, 2, 1],
        &[1.0, 2.0],
        context,
    )?)?;
    let right_cross_attention = ConditioningValue::cross_attention(tensor_from_f32(
        backend,
        &[1, 3, 1],
        &[3.0, 4.0, 5.0],
        context,
    )?)?;
    assert!(left_cross_attention.can_concat(&right_cross_attention));
    let concatenated_cross_attention =
        left_cross_attention.concat(&[right_cross_attention], backend, context)?;
    let ConditioningValue::CrossAttention(cross_attention_tensor) = &concatenated_cross_attention
    else {
        return Err("concatenated cross-attention conditioning changed kind".into());
    };
    assert_f32_tensor(
        backend,
        context,
        cross_attention_tensor,
        &[2, 6, 1],
        &[1.0, 2.0, 1.0, 2.0, 1.0, 2.0, 3.0, 4.0, 5.0, 3.0, 4.0, 5.0],
    )?;
    record_contract_case(
        &mut executed_case_ids,
        "conditioning-conditioning-value-conds-condcrossattn-4d921d69",
        CONDITIONING_TASK,
    )?;

    let constant = ConditioningValue::constant(ConditioningConstant::finite_f64(0.5)?)?;
    let same_constant = ConditioningValue::constant(ConditioningConstant::finite_f64(0.5)?)?;
    let different_constant = ConditioningValue::constant(ConditioningConstant::finite_f64(0.25)?)?;
    assert_eq!(constant.size()?, vec![1_u64]);
    assert!(constant.can_concat(&same_constant));
    assert!(!constant.can_concat(&different_constant));
    let concatenated_constant = constant.concat(&[same_constant], backend, context)?;
    let ConditioningValue::Constant(concatenated_constant) = concatenated_constant else {
        return Err("concatenated constant conditioning changed kind".into());
    };
    assert_eq!(
        concatenated_constant,
        ConditioningConstant::finite_f64(0.5)?
    );
    assert!(
        constant
            .concat(&[different_constant], backend, context)
            .is_err()
    );
    record_contract_case(
        &mut executed_case_ids,
        "conditioning-conditioning-value-conds-condconstant-0e559aad",
        CONDITIONING_TASK,
    )?;

    let list = ConditioningValue::list(vec![
        tensor_from_f32(backend, &[1, 2], &[1.0, 2.0], context)?,
        tensor_from_f32(backend, &[1, 1], &[3.0], context)?,
    ])?;
    let other_list = ConditioningValue::list(vec![
        tensor_from_f32(backend, &[1, 2], &[4.0, 5.0], context)?,
        tensor_from_f32(backend, &[1, 1], &[6.0], context)?,
    ])?;
    assert_eq!(list.size()?, vec![1_u64, 1, 3]);
    assert!(list.can_concat(&other_list));
    let processed_list = list.process(2, None, backend, context)?;
    let ConditioningValue::List(processed_items) = &processed_list else {
        return Err("processed list conditioning changed kind".into());
    };
    let [processed_first, processed_second] = processed_items.as_slice() else {
        return Err("processed list conditioning did not contain two items".into());
    };
    assert_f32_tensor(
        backend,
        context,
        processed_first,
        &[2, 2],
        &[1.0, 2.0, 1.0, 2.0],
    )?;
    assert_f32_tensor(backend, context, processed_second, &[2, 1], &[3.0, 3.0])?;
    let concatenated_list = list.concat(&[other_list], backend, context)?;
    let ConditioningValue::List(concatenated_items) = &concatenated_list else {
        return Err("concatenated list conditioning changed kind".into());
    };
    let [concatenated_first, concatenated_second] = concatenated_items.as_slice() else {
        return Err("concatenated list conditioning did not contain two items".into());
    };
    assert_f32_tensor(
        backend,
        context,
        concatenated_first,
        &[2, 2],
        &[1.0, 2.0, 4.0, 5.0],
    )?;
    assert_f32_tensor(backend, context, concatenated_second, &[2, 1], &[3.0, 6.0])?;
    record_contract_case(
        &mut executed_case_ids,
        "conditioning-conditioning-value-conds-condlist-21ce2116",
        CONDITIONING_TASK,
    )?;

    Ok(executed_case_ids)
}

fn guidance_contract_identity(namespace: &str) -> Result<ConditioningIdentity, Box<dyn Error>> {
    Ok(ConditioningIdentity::new(
        namespace,
        sd15_model_family_identity()?,
        sd15_latent_format_identity()?,
    )?)
}

fn assert_guidance_entry_contract_cases(
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
    executed_case_ids: &mut BTreeSet<&'static str>,
) -> Result<(), Box<dyn Error>> {
    let target = tensor_from_f32(backend, &[2, 2, 4], &[0.0; 16], context)?;
    let mask = tensor_from_f32(backend, &[4], &[0.25, 0.5, 0.75, 1.0], context)?;
    let references = ConditioningReferences::checked(
        Some(ConditioningControlReference::checked("control.primary")?),
        vec![
            ConditioningHookReference::checked("hook.first")?,
            ConditioningHookReference::checked("hook.second")?,
        ],
    )?;
    let entry = ConditioningEntry::checked_with_references(
        "source.guidance.entry",
        ConditioningValue::regular(tensor_from_f32(backend, &[1, 1], &[1.0], context)?)?,
        ConditioningEntryOptions {
            strength: 2.0,
            region: Some(ConditioningRegion::absolute(vec![2], vec![1])?),
            mask: Some(ConditioningMask::new(mask, 0.5, vec![0], false)?),
            window: ConditioningWindow::new(0.25, 0.75)?,
            default_region: false,
        },
        references,
    )?;
    let conditioning = ConditioningSet::checked(
        guidance_contract_identity("native.conditioning.guidance.entry")?,
        vec![entry],
        context.cancellation,
    )?;
    let resolved = conditioning.resolve(target.descriptor(), backend, context)?;
    let [resolved_entry] = resolved.as_slice() else {
        return Err("guidance entry fixture did not resolve exactly one entry".into());
    };
    let ConditioningValue::Regular(resolved_value) = resolved_entry.value() else {
        return Err("resolved guidance entry changed conditioning kind".into());
    };
    assert_f32_tensor(backend, context, resolved_value, &[2, 1], &[1.0, 1.0])?;
    record_contract_case(
        executed_case_ids,
        "conditioning-guidance-hook-sampler-helpers-convert-cond-e8752d85",
        GUIDANCE_TASK,
    )?;

    assert_eq!(resolved_entry.region().sizes(), &[2]);
    assert_eq!(resolved_entry.region().offsets(), &[1]);
    assert_eq!(resolved_entry.strength(), 2.0);
    assert!(resolved_entry.window().contains(0.5));
    assert!(!resolved_entry.window().contains(0.8));
    let resolved_mask = resolved_entry
        .mask()
        .ok_or("guidance entry mask did not resolve")?;
    assert_eq!(resolved_mask.strength(), 0.5);
    assert_eq!(resolved_entry.contribution_weight(&[0], Some(0.5))?, 0.5);
    assert_eq!(resolved_entry.contribution_weight(&[1], Some(0.75))?, 0.75);
    record_contract_case(
        executed_case_ids,
        "conditioning-guidance-samplers-get-area-and-mult-14d8dec2",
        GUIDANCE_TASK,
    )?;

    assert_f32_tensor(
        backend,
        context,
        resolved_mask.tensor(),
        &[2, 2, 2],
        &[0.5, 0.75, 0.5, 0.75, 0.5, 0.75, 0.5, 0.75],
    )?;
    record_contract_case(
        executed_case_ids,
        "conditioning-guidance-hook-sampler-helpers-prepare-mask-048488c7",
        GUIDANCE_TASK,
    )?;

    assert_eq!(
        resolved_entry
            .references()
            .control()
            .map(|reference| reference.identifier()),
        Some("control.primary")
    );
    let hook_identifiers = resolved_entry
        .references()
        .hooks()
        .iter()
        .map(ConditioningHookReference::identifier)
        .collect::<Vec<_>>();
    assert_eq!(hook_identifiers, vec!["hook.first", "hook.second"]);
    record_contract_case(
        executed_case_ids,
        "conditioning-guidance-hook-sampler-helpers-get-models-from-cond-1be91d68",
        GUIDANCE_TASK,
    )?;
    Ok(())
}

struct GuidanceContractDenoiser<'a> {
    backend: &'a CpuBackend,
    batches: Vec<Vec<GuidanceBranch>>,
}

impl GuidanceDenoiser for GuidanceContractDenoiser<'_> {
    fn evaluate_batch(
        &mut self,
        evaluations: &[GuidanceEvaluation],
        context: &ExecutionContext<'_>,
    ) -> Result<Vec<Tensor>, GuidanceError> {
        self.batches
            .push(evaluations.iter().map(GuidanceEvaluation::branch).collect());
        let mut outputs = Vec::new();
        outputs
            .try_reserve_exact(evaluations.len())
            .map_err(|_| GuidanceError::ShapeOverflow("contract denoiser outputs"))?;
        for evaluation in evaluations {
            let value = match evaluation.entry().identifier() {
                "left" => 2.0,
                "strong" => 6.0,
                "negative" => 1.0,
                identifier => {
                    return Err(GuidanceError::Invalid(format!(
                        "unexpected contract denoiser entry {identifier}"
                    )));
                }
            };
            let count = usize::try_from(evaluation.latent().descriptor().element_count()?)
                .map_err(|_| GuidanceError::ShapeOverflow("contract denoiser output"))?;
            outputs.push(tensor_from_f32(
                self.backend,
                evaluation.latent().descriptor().shape(),
                &vec![value; count],
                context,
            )?);
        }
        Ok(outputs)
    }
}

struct OrderedGuidanceContractHook {
    name: &'static str,
    events: Rc<RefCell<Vec<String>>>,
    override_with_conditional: bool,
}

impl GuidanceHook for OrderedGuidanceContractHook {
    fn pre_cfg(
        &mut self,
        _hook: &GuidanceHookContext<'_>,
        _predictions: &mut GuidancePredictions,
        _context: &ExecutionContext<'_>,
    ) -> Result<(), GuidanceError> {
        self.events.borrow_mut().push(format!("pre:{}", self.name));
        Ok(())
    }

    fn override_cfg(
        &mut self,
        _hook: &GuidanceHookContext<'_>,
        predictions: &GuidancePredictions,
        _current: &Tensor,
        _context: &ExecutionContext<'_>,
    ) -> Result<Option<Tensor>, GuidanceError> {
        self.events
            .borrow_mut()
            .push(format!("override:{}", self.name));
        Ok(self
            .override_with_conditional
            .then(|| predictions.conditional().clone()))
    }

    fn post_cfg(
        &mut self,
        _hook: &GuidanceHookContext<'_>,
        _predictions: &GuidancePredictions,
        current: &Tensor,
        _context: &ExecutionContext<'_>,
    ) -> Result<Option<Tensor>, GuidanceError> {
        self.events.borrow_mut().push(format!("post:{}", self.name));
        Ok(Some(current.clone()))
    }
}

struct InjectionLifecycleHook {
    events: Rc<RefCell<Vec<&'static str>>>,
    active: bool,
}

impl GuidanceHook for InjectionLifecycleHook {
    fn pre_cfg(
        &mut self,
        _hook: &GuidanceHookContext<'_>,
        _predictions: &mut GuidancePredictions,
        _context: &ExecutionContext<'_>,
    ) -> Result<(), GuidanceError> {
        if self.active {
            return Err(GuidanceError::Invalid(
                "injection hook was activated twice".to_owned(),
            ));
        }
        self.active = true;
        self.events.borrow_mut().push("inject");
        Ok(())
    }

    fn override_cfg(
        &mut self,
        _hook: &GuidanceHookContext<'_>,
        _predictions: &GuidancePredictions,
        _current: &Tensor,
        context: &ExecutionContext<'_>,
    ) -> Result<Option<Tensor>, GuidanceError> {
        self.events.borrow_mut().push("override");
        context.cancellation.cancel();
        Ok(None)
    }
}

impl Drop for InjectionLifecycleHook {
    fn drop(&mut self) {
        if self.active {
            self.active = false;
            self.events.borrow_mut().push("eject");
        }
    }
}

fn assert_guidance_execution_contract_cases(
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
    executed_case_ids: &mut BTreeSet<&'static str>,
) -> Result<(), Box<dyn Error>> {
    let conditioning_value = tensor_from_f32(backend, &[1, 1], &[1.0], context)?;
    let regional_options = |strength, offset| -> Result<_, Box<dyn Error>> {
        Ok(ConditioningEntryOptions {
            strength,
            region: Some(ConditioningRegion::absolute(vec![3], vec![offset])?),
            mask: None,
            window: ConditioningWindow::full(),
            default_region: false,
        })
    };
    let conditional = ConditioningSet::checked(
        guidance_contract_identity("native.conditioning.guidance.positive")?,
        vec![
            ConditioningEntry::checked(
                "left",
                ConditioningValue::regular(conditioning_value.clone())?,
                regional_options(1.0, 0)?,
            )?,
            ConditioningEntry::checked(
                "strong",
                ConditioningValue::regular(conditioning_value.clone())?,
                regional_options(3.0, 1)?,
            )?,
        ],
        context.cancellation,
    )?;
    let unconditional = ConditioningSet::checked(
        guidance_contract_identity("native.conditioning.guidance.negative")?,
        vec![ConditioningEntry::checked(
            "negative",
            ConditioningValue::regular(conditioning_value)?,
            ConditioningEntryOptions::default(),
        )?],
        context.cancellation,
    )?;
    let latent = tensor_from_f32(backend, &[1, 1, 4], &[0.0; 4], context)?;
    let profile = DiscreteSamplingProfile::new(
        SamplingProfileIdentity::new("conditioning-contract-profile")?,
        PredictionInterpretation::Epsilon,
        Arc::from([1.0_f32, 2.0_f32]),
    )?;
    let plan = SamplingPlan::new(
        "euler",
        "normal",
        profile.identity().clone(),
        7,
        1,
        1.0,
        1.0,
    )?;

    let hook_events = Rc::new(RefCell::new(Vec::new()));
    let mut first_hook = OrderedGuidanceContractHook {
        name: "a",
        events: hook_events.clone(),
        override_with_conditional: true,
    };
    let mut second_hook = OrderedGuidanceContractHook {
        name: "b",
        events: hook_events.clone(),
        override_with_conditional: false,
    };
    let mut denoiser = GuidanceContractDenoiser {
        backend,
        batches: Vec::new(),
    };
    let result = {
        let mut hooks: [&mut dyn GuidanceHook; 2] = [&mut first_hook, &mut second_hook];
        execute_guidance(
            backend,
            &latent,
            2.0,
            &profile,
            &plan,
            &conditional,
            &unconditional,
            GuidanceOptions::default(),
            &mut denoiser,
            &mut hooks,
            context,
        )?
    };
    assert_f32_tensor(
        backend,
        context,
        result.guided(),
        &[1, 1, 4],
        &[2.0, 5.0, 5.0, 6.0],
    )?;
    assert_eq!(result.denoiser_evaluations(), 2);
    assert_eq!(result.denoiser_batches(), 1);
    assert_eq!(
        denoiser.batches,
        vec![vec![
            GuidanceBranch::Conditional,
            GuidanceBranch::Conditional,
        ]]
    );
    record_contract_case(
        executed_case_ids,
        "conditioning-guidance-samplers-calc-cond-batch-23aa4a02",
        GUIDANCE_TASK,
    )?;
    assert!(result.unconditional_skipped());
    assert_eq!(
        &*hook_events.borrow(),
        &[
            "pre:a",
            "pre:b",
            "override:a",
            "override:b",
            "post:a",
            "post:b",
        ]
    );
    record_contract_case(
        executed_case_ids,
        "conditioning-guidance-samplers-sampling-function-ef25ad1d",
        GUIDANCE_TASK,
    )?;
    record_contract_case(
        executed_case_ids,
        "conditioning-guidance-hook-hooks-hook-536ff505",
        GUIDANCE_TASK,
    )?;

    denoiser.batches.clear();
    let injection_events = Rc::new(RefCell::new(Vec::new()));
    let injection_cancellation = CancellationToken::default();
    let injection_context = backend.execution_context(
        context.stream,
        context.scratch.clone(),
        &injection_cancellation,
    );
    {
        let mut injection_hook = InjectionLifecycleHook {
            events: injection_events.clone(),
            active: false,
        };
        let mut injection_hooks: [&mut dyn GuidanceHook; 1] = [&mut injection_hook];
        assert!(matches!(
            execute_guidance(
                backend,
                &latent,
                2.0,
                &profile,
                &plan,
                &conditional,
                &unconditional,
                GuidanceOptions::default(),
                &mut denoiser,
                &mut injection_hooks,
                &injection_context,
            ),
            Err(GuidanceError::Cancelled)
        ));
        assert!(injection_cancellation.is_cancelled());
        assert_eq!(&*injection_events.borrow(), &["inject", "override"]);
    }
    assert_eq!(
        &*injection_events.borrow(),
        &["inject", "override", "eject"]
    );
    assert_eq!(injection_context.scratch.in_use_bytes(), 0);
    record_contract_case(
        executed_case_ids,
        "conditioning-guidance-hook-patcher-extension-patcherinjection-116374da",
        GUIDANCE_TASK,
    )?;
    Ok(())
}

fn assert_guidance_contract_cases(
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
) -> Result<BTreeSet<&'static str>, Box<dyn Error>> {
    let mut executed_case_ids = BTreeSet::new();
    assert_guidance_entry_contract_cases(backend, context, &mut executed_case_ids)?;
    assert_guidance_execution_contract_cases(backend, context, &mut executed_case_ids)?;
    Ok(executed_case_ids)
}

fn prebound_fixture(
    backend: &Arc<CpuBackend>,
    context: &ExecutionContext<'_>,
    mode: FixtureControlMode,
) -> Result<(NativeDiffusionFixture, Arc<AtomicUsize>), Box<dyn Error>> {
    let fixture = NativeDiffusionFixture::checked_in();
    let cache_identities = fixture.cache_identities(context.cancellation)?;
    let model_digest = cache_identities.model_digest().to_owned();
    let patch_graph = Arc::new(PatchGraph::checked_semantic(
        &model_digest,
        vec![SemanticPatchOperation {
            identifier: "native-conditioning-output-bias".to_owned(),
            target_key: "model.diffusion_model.out.2.bias".to_owned(),
            expected_shape: vec![4],
            strength: 1.0,
            strength_model: 1.0,
            slices: Vec::new(),
            transform: PatchValueTransform::default(),
            payload: PatchPayload::Set {
                tensor: PatchTensor::checked(vec![4], vec![0.015, -0.01, 0.005, -0.02])?,
            },
        }],
    )?);
    let source_bias = tensor_from_f32(backend, &[4], &[0.0; 4], context)?;
    let mapped = MappedModelWeights::from_test_parts(
        model_digest.clone(),
        BTreeMap::from([("model.diffusion_model.out.2.bias".to_owned(), source_bias)]),
        Vec::new(),
    )?;
    let patched = patch_graph.apply(backend.as_ref(), &mapped, context)?;
    let patched_bias = patched
        .tensors()
        .get("model.diffusion_model.out.2.bias")
        .ok_or("patched fixture output bias is missing")?;
    assert_f32_tensor(
        backend.as_ref(),
        context,
        patched_bias,
        &[4],
        &[0.015, -0.01, 0.005, -0.02],
    )?;
    assert_ne!(patched.cache_identity(), mapped.cache_identity());

    let hint = tensor_from_f32(backend, &[1, 32, 4, 4], &[0.0025; 512], context)?;
    let hint = ControlTensorBinding::checked(
        hint.clone(),
        tensor_digest(backend.as_ref(), &hint, context)?,
    )?;
    let model = ControlModelBinding::checked(
        sd15_model_family_identity()?,
        patch_graph.identity(),
        model_digest,
        FIXTURE_CONTROL_EXECUTOR_DIGEST,
        DType::F32,
        DeviceId::CPU,
    )?;
    let control = ControlNet::checked(
        ControlBase::checked(
            0.125,
            StrengthType::Constant,
            ControlPercentWindow::checked(0.0, 1.0)?,
            false,
            Some(DType::F32),
        )?,
        model,
        hint,
        1,
        ResizeMode::NearestExact,
        ControlHintPreprocess::Identity,
        None,
        Vec::new(),
        false,
        Vec::new(),
    )?;
    let chain = Arc::new(ControlChain::checked(vec![ControlNode::ControlNet(
        control,
    )])?);
    let calls = Arc::new(AtomicUsize::new(0));
    let executor = Arc::new(FixtureControlExecutor {
        calls: calls.clone(),
        mode,
    });
    let control_execution = PreboundControlExecution::checked(chain, executor)?;
    Ok((
        fixture.with_prebound_conditioning(
            patch_graph,
            Some(control_execution),
            patched.cache_identity().to_owned(),
        )?,
        calls,
    ))
}

struct MutableConditioningProvider {
    inner: NativeDiffusionFixture,
    cache_identities: Mutex<CanonicalNativeDiffusionCacheIdentities>,
    load_calls: AtomicUsize,
}

impl MutableConditioningProvider {
    fn checked(
        inner: NativeDiffusionFixture,
        cancellation: &CancellationToken,
    ) -> Result<Self, NativeImageRuntimeError> {
        let cache_identities = NativeDiffusionProvider::cache_identities(&inner, cancellation)?;
        Ok(Self {
            inner,
            cache_identities: Mutex::new(cache_identities),
            load_calls: AtomicUsize::new(0),
        })
    }

    fn replace_conditioning(
        &self,
        conditioning: CanonicalConditioningCacheIdentities,
    ) -> Result<(), NativeImageRuntimeError> {
        let mut identities = self.cache_identities.lock().map_err(|_| {
            NativeImageRuntimeError::Registry(
                "mutable conditioning fixture lock is poisoned".to_owned(),
            )
        })?;
        *identities = CanonicalNativeDiffusionCacheIdentities::checked(
            identities.model_digest(),
            identities.tokenizer_digest(),
            identities.clip().clone(),
            identities.vae().clone(),
            conditioning,
        )
        .map_err(|error| NativeImageRuntimeError::Registry(error.to_string()))?;
        Ok(())
    }
}

impl NativeDiffusionProvider for MutableConditioningProvider {
    fn cache_identities(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<CanonicalNativeDiffusionCacheIdentities, NativeImageRuntimeError> {
        if cancellation.is_cancelled() {
            return Err(NativeImageRuntimeError::Cancelled);
        }
        let identities = self.cache_identities.lock().map_err(|_| {
            NativeImageRuntimeError::Registry(
                "mutable conditioning fixture lock is poisoned".to_owned(),
            )
        })?;
        if cancellation.is_cancelled() {
            return Err(NativeImageRuntimeError::Cancelled);
        }
        Ok(identities.clone())
    }

    fn load(
        &self,
        backend: Arc<CpuBackend>,
        context: &ExecutionContext<'_>,
    ) -> Result<NativeDiffusionBundle, NativeImageRuntimeError> {
        self.load_calls.fetch_add(1, Ordering::SeqCst);
        NativeDiffusionProvider::load(&self.inner, backend, context)
    }
}

#[test]
fn val_conditioning_001() -> Result<(), Box<dyn Error>> {
    let cancellation: CancellationToken = Default::default();
    let (backend, workspace_authority) = CpuWorkspaceAuthority::create_backend(MEMORY_LIMIT)?;
    let backend = Arc::new(backend);
    let setup_workspace = workspace_authority.authorize_workspace(MEMORY_LIMIT)?;
    let setup_context =
        backend.execution_context(StreamId::DEFAULT, setup_workspace.clone(), &cancellation);
    let mut executed_case_ids = assert_conditioning_value_contract_cases(&backend, &setup_context)?;
    executed_case_ids.extend(assert_guidance_contract_cases(&backend, &setup_context)?);
    assert_eq!(executed_case_ids.len(), 13);
    assert_eq!(setup_workspace.in_use_bytes(), 0);
    let (fixture, control_calls) =
        prebound_fixture(&backend, &setup_context, FixtureControlMode::Success)?;
    assert_eq!(setup_workspace.in_use_bytes(), 0);
    let base_fixture = NativeDiffusionFixture::checked_in();
    let fixture_identities = fixture.cache_identities(&cancellation)?;
    let base_identities = base_fixture.cache_identities(&cancellation)?;
    let fixture_conditioning = fixture_identities.conditioning();
    let base_conditioning = base_identities.conditioning();
    assert_ne!(fixture_conditioning, base_conditioning);
    assert_ne!(
        fixture_conditioning.model_patch(),
        base_conditioning.model_patch()
    );
    assert_ne!(fixture_conditioning.control(), base_conditioning.control());
    record_contract_case(
        &mut executed_case_ids,
        "conditioning-guidance-hook-hooks-weighthook-03327446",
        GUIDANCE_TASK,
    )?;

    let mutable_provider = Arc::new(MutableConditioningProvider::checked(
        fixture.clone(),
        &cancellation,
    )?);
    let provider: Arc<dyn NativeDiffusionProvider> = mutable_provider.clone();
    let mut plan = compile_native_diffusion_workflow(WORKFLOW, &BTreeSet::new(), provider.clone())?;
    plan.prompt_id =
        comfy_types::PromptId(Uuid::from_u128(0x5349_4d00_0000_0000_0000_0000_0000_3590));
    let executor = NativeImageExecutor::new_with_diffusion_provider(
        ProfileId(Uuid::from_u128(0x5349_4d00_0000_0000_0000_0000_0000_3591)),
        BTreeMap::new(),
        true,
        backend.clone(),
        provider.clone(),
    )?;
    let execution_workspace = workspace_authority.authorize_workspace(MEMORY_LIMIT)?;
    let result = executor.execute_blocking(
        &plan,
        AttemptId(Uuid::from_u128(0x3592)),
        Default::default(),
        0,
        execution_workspace.clone(),
    )?;
    assert_eq!(
        result.report.state,
        AttemptState::Succeeded,
        "prebound native conditioning failed: {:?}",
        result.report.error
    );
    assert_eq!(result.executed_node_count, 7);
    assert!(control_calls.load(Ordering::SeqCst) > 0);
    assert_eq!(execution_workspace.in_use_bytes(), 0);
    record_contract_case(
        &mut executed_case_ids,
        "conditioning-guidance-hook-sampler-helpers-get-additional-models-7ba596bf",
        GUIDANCE_TASK,
    )?;
    record_contract_case(
        &mut executed_case_ids,
        "conditioning-guidance-hook-sampler-helpers-prepare-sampling-b141c606",
        GUIDANCE_TASK,
    )?;

    let exact_identities = mutable_provider.cache_identities(&cancellation)?;
    let exact_conditioning = exact_identities.conditioning();
    let replacement_control = if exact_conditioning.control().starts_with('f') {
        "e".repeat(64)
    } else {
        "f".repeat(64)
    };
    let replacement_conditioning = CanonicalConditioningCacheIdentities::checked(
        exact_conditioning.conditioning(),
        exact_conditioning.guidance(),
        exact_conditioning.model_patch(),
        exact_conditioning.model_execution(),
        replacement_control,
    )?;
    assert_ne!(
        replacement_conditioning.control(),
        exact_conditioning.control()
    );
    assert_ne!(
        replacement_conditioning.execution(),
        exact_conditioning.execution()
    );
    let load_calls_before_mutation = mutable_provider.load_calls.load(Ordering::SeqCst);
    let control_calls_before_mutation = control_calls.load(Ordering::SeqCst);
    mutable_provider.replace_conditioning(replacement_conditioning.clone())?;
    let mutated_identities = mutable_provider.cache_identities(&cancellation)?;
    assert_eq!(
        mutated_identities.model_digest(),
        exact_identities.model_digest()
    );
    assert_eq!(
        mutated_identities.tokenizer_digest(),
        exact_identities.tokenizer_digest()
    );
    assert_eq!(mutated_identities.clip(), exact_identities.clip());
    assert_eq!(mutated_identities.vae(), exact_identities.vae());
    assert_eq!(mutated_identities.conditioning(), &replacement_conditioning);
    let stale_workspace = workspace_authority.authorize_workspace(MEMORY_LIMIT)?;
    let stale = executor.execute_blocking(
        &plan,
        AttemptId(Uuid::from_u128(0x3593)),
        Default::default(),
        0,
        stale_workspace.clone(),
    )?;
    assert_eq!(
        stale.report.state,
        AttemptState::Failed,
        "stale provider identity unexpectedly executed: {:?}",
        stale.report
    );
    assert!(
        stale
            .report
            .error
            .as_deref()
            .is_some_and(|error| error.contains(
                "native diffusion provider conditioning identity does not match its loaded bundle"
            ))
    );
    assert_eq!(stale.executed_node_count, 0);
    assert!(stale.report.outputs.is_empty());
    assert!(stale.output_proposals.is_empty());
    assert!(stale.report.events.iter().all(|event| !matches!(
        &event.kind,
        AttemptEventKind::OutputPrepared { .. } | AttemptEventKind::OutputAvailable { .. }
    )));
    assert_eq!(
        mutable_provider.load_calls.load(Ordering::SeqCst),
        load_calls_before_mutation
    );
    assert_eq!(
        control_calls.load(Ordering::SeqCst),
        control_calls_before_mutation
    );
    assert_eq!(stale_workspace.in_use_bytes(), 0);

    for (mode, expected_state, expected_code, attempt_suffix) in [
        (
            FixtureControlMode::Cancelled,
            AttemptState::Interrupted,
            "native_diffusion_cancelled",
            0x3594_u128,
        ),
        (
            FixtureControlMode::AllocationFailed,
            AttemptState::Failed,
            "native_diffusion_resource_exhausted",
            0x3595_u128,
        ),
    ] {
        let failure_setup_workspace = workspace_authority.authorize_workspace(MEMORY_LIMIT)?;
        let failure_setup_context = backend.execution_context(
            StreamId::DEFAULT,
            failure_setup_workspace.clone(),
            &cancellation,
        );
        let (failure_fixture, failure_calls) =
            prebound_fixture(&backend, &failure_setup_context, mode)?;
        assert_eq!(failure_setup_workspace.in_use_bytes(), 0);
        let failure_provider: Arc<dyn NativeDiffusionProvider> = Arc::new(failure_fixture);
        let mut failure_plan = compile_native_diffusion_workflow(
            WORKFLOW,
            &BTreeSet::new(),
            failure_provider.clone(),
        )?;
        failure_plan.prompt_id = comfy_types::PromptId(Uuid::from_u128(
            0x5349_4d00_0000_0000_0000_0000_0000_0000 | attempt_suffix,
        ));
        let failure_executor = NativeImageExecutor::new_with_diffusion_provider(
            ProfileId(Uuid::from_u128(
                0x5349_4d00_0000_0000_0000_0000_0001_0000 | attempt_suffix,
            )),
            BTreeMap::new(),
            true,
            backend.clone(),
            failure_provider,
        )?;
        let failure_workspace = workspace_authority.authorize_workspace(MEMORY_LIMIT)?;
        let failure = failure_executor.execute_blocking(
            &failure_plan,
            AttemptId(Uuid::from_u128(attempt_suffix)),
            Default::default(),
            0,
            failure_workspace.clone(),
        )?;
        assert_eq!(failure.report.state, expected_state);
        assert!(
            failure
                .report
                .error
                .as_deref()
                .is_some_and(|error| error.contains(expected_code)),
            "typed ControlNet failure did not retain {expected_code}: {:?}",
            failure.report.error
        );
        if matches!(mode, FixtureControlMode::AllocationFailed) {
            assert!(
                failure
                    .report
                    .error
                    .as_deref()
                    .is_some_and(|error| error.contains("injected ControlNet allocation failure"))
            );
        }
        assert_eq!(failure_calls.load(Ordering::SeqCst), 1);
        assert_eq!(failure.executed_node_count, 0);
        assert!(failure.report.outputs.is_empty());
        assert!(failure.output_proposals.is_empty());
        assert!(failure.report.events.iter().all(|event| !matches!(
            &event.kind,
            AttemptEventKind::OutputPrepared { .. } | AttemptEventKind::OutputAvailable { .. }
        )));
        assert_eq!(failure_workspace.in_use_bytes(), 0);
    }
    record_contract_case(
        &mut executed_case_ids,
        "conditioning-guidance-hook-sampler-helpers-cleanup-models-6f147c97",
        GUIDANCE_TASK,
    )?;

    let constrained_provider: Arc<dyn NativeDiffusionProvider> = Arc::new(fixture);
    let constrained_executor = NativeImageExecutor::new_with_diffusion_provider(
        ProfileId(Uuid::from_u128(0x5349_4d00_0000_0000_0000_0000_0000_3596)),
        BTreeMap::new(),
        true,
        backend,
        constrained_provider,
    )?;
    let constrained_workspace = workspace_authority.authorize_workspace(1024)?;
    let constrained = constrained_executor.execute_blocking(
        &plan,
        AttemptId(Uuid::from_u128(0x3596)),
        Default::default(),
        0,
        constrained_workspace.clone(),
    )?;
    assert_eq!(constrained.report.state, AttemptState::Failed);
    assert!(
        constrained
            .report
            .error
            .as_deref()
            .is_some_and(|error| error.contains("native_diffusion_resource_exhausted"))
    );
    assert!(constrained.report.outputs.is_empty());
    assert!(constrained.output_proposals.is_empty());
    assert!(constrained.report.events.iter().all(|event| !matches!(
        &event.kind,
        AttemptEventKind::OutputPrepared { .. } | AttemptEventKind::OutputAvailable { .. }
    )));
    assert_eq!(constrained_workspace.in_use_bytes(), 0);

    write_artifact(&executed_case_ids)?;
    Ok(())
}

fn python_symbol_sha256(source: &[u8], symbol: &str) -> Result<String, Box<dyn Error>> {
    let source = std::str::from_utf8(source)?;
    let lines = source.split_inclusive('\n').collect::<Vec<_>>();
    let signatures = [
        format!("def {symbol}("),
        format!("async def {symbol}("),
        format!("class {symbol}("),
        format!("class {symbol}:"),
    ];
    let matches = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            let trimmed = line.trim_start_matches([' ', '\t']);
            signatures
                .iter()
                .any(|signature| trimmed.starts_with(signature))
                .then_some(index)
        })
        .collect::<Vec<_>>();
    let [start] = matches.as_slice() else {
        return Err(format!(
            "expected exactly one Python definition for {symbol}, found {}",
            matches.len()
        )
        .into());
    };
    let indentation = lines[*start].len() - lines[*start].trim_start_matches([' ', '\t']).len();
    let mut header_complete = lines[*start].trim_end().ends_with(':');
    let mut body_seen = false;
    let mut end = *start + 1;
    while let Some(line) = lines.get(end) {
        let trimmed = line.trim_start_matches([' ', '\t']);
        let content = trimmed.trim_end_matches(['\r', '\n']);
        if content.is_empty() || content.starts_with('#') {
            end += 1;
            continue;
        }
        let line_indentation = line.len() - trimmed.len();
        if !header_complete {
            header_complete = line_indentation == indentation && content.ends_with(':');
            end += 1;
            continue;
        }
        if body_seen && line_indentation <= indentation {
            break;
        }
        if line_indentation > indentation {
            body_seen = true;
        }
        end += 1;
    }
    if !body_seen {
        return Err(format!("Python definition {symbol} has no body").into());
    }
    while end > *start + 1 {
        let content = lines[end - 1].trim();
        if content.is_empty() || content.starts_with('#') {
            end -= 1;
        } else {
            break;
        }
    }
    Ok(format!(
        "{:x}",
        Sha256::digest(lines[*start..end].concat().as_bytes())
    ))
}

fn write_artifact(executed_case_ids: &BTreeSet<&str>) -> Result<(), Box<dyn Error>> {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let catalog = fs::read_to_string(
        repository.join(".agents/specs/comfy-parity/catalogs/backend-conditioning-contracts.csv"),
    )?;
    let mut contracts = Vec::new();
    let mut counts = BTreeMap::new();
    let expected_cases = CONTRACT_CASES
        .iter()
        .map(|(contract_id, task_id, case_id)| (*contract_id, (*task_id, *case_id)))
        .collect::<BTreeMap<_, _>>();
    if expected_cases.len() != CONTRACT_CASES.len() {
        return Err("VAL-CONDITIONING-001 contract mapping contains duplicate IDs".into());
    }
    let expected_case_ids = CONTRACT_CASES
        .iter()
        .map(|(_, _, case_id)| *case_id)
        .collect::<BTreeSet<_>>();
    if expected_case_ids.len() != CONTRACT_CASES.len() {
        return Err("VAL-CONDITIONING-001 contract mapping contains duplicate case IDs".into());
    }
    if executed_case_ids != &expected_case_ids {
        return Err(format!(
            "VAL-CONDITIONING-001 executed cases differ from the exact contract mapping: expected {expected_case_ids:?}, observed {executed_case_ids:?}"
        )
        .into());
    }
    let mut seen_contract_ids = BTreeSet::new();
    for line in catalog.lines().skip(1) {
        let columns = line.split(',').collect::<Vec<_>>();
        if columns.len() != 15 || !matches!(columns[8], CONDITIONING_TASK | GUIDANCE_TASK) {
            continue;
        }
        let Some((expected_task, case_id)) = expected_cases.get(columns[0]).copied() else {
            return Err(format!(
                "conditioning catalog contains unexpected VAL-CONDITIONING-001 contract {}",
                columns[0]
            )
            .into());
        };
        if columns[8] != expected_task || !seen_contract_ids.insert(columns[0]) {
            return Err(format!(
                "conditioning contract {} has a duplicate or mismatched task",
                columns[0]
            )
            .into());
        }
        let source = fs::read(repository.join(columns[2]))?;
        assert_eq!(format!("{:x}", Sha256::digest(&source)), columns[5]);
        assert_eq!(python_symbol_sha256(&source, columns[3])?, columns[6]);
        *counts.entry(columns[8].to_owned()).or_insert(0_usize) += 1;
        contracts.push(json!({
            "contract_id": columns[0],
            "task_id": columns[8],
            "source_sha256": columns[5],
            "symbol_sha256": columns[6],
            "status": "passed",
            "case_ids": [case_id],
        }));
    }
    assert_eq!(counts.get(CONDITIONING_TASK), Some(&5));
    assert_eq!(counts.get(GUIDANCE_TASK), Some(&12));
    assert_eq!(contracts.len(), 17);
    if seen_contract_ids != expected_cases.keys().copied().collect() {
        return Err("conditioning catalog does not exactly cover the 17 contract mappings".into());
    }
    let implementation_path = "crates/comfy_test_support/tests/native_conditioning_integration.rs";
    let implementation = fs::read(repository.join(implementation_path))?;
    let task_result = |path: &str, case_ids: &[&str], passed: usize| {
        let implementation = fs::read(repository.join(path))?;
        Ok::<_, Box<dyn Error>>(json!({
            "status": "passed",
            "passed": passed,
            "failed": 0,
            "skipped": 0,
            "case_ids": case_ids,
            "implementations": [{
                "path": path,
                "sha256": format!("{:x}", Sha256::digest(implementation)),
            }],
        }))
    };
    let task_results = BTreeMap::from([
        (
            CONDITIONING_TASK,
            task_result(
                "crates/comfy_model/src/conditioning.rs",
                &[
                    "conditioning:all-contracts",
                    "conditioning:values-regions-masks",
                    "conditioning:cancellation-oom-workspace-ownership",
                ],
                5,
            )?,
        ),
        (
            GUIDANCE_TASK,
            task_result(
                "crates/comfy_sampler/src/guidance.rs",
                &[
                    "guidance:all-contracts",
                    "guidance:cfg-hooks-batching-regions",
                    "guidance:cancellation-oom-workspace-ownership",
                ],
                12,
            )?,
        ),
    ]);
    let artifact = json!({
        "schema_version": 1,
        "validation_id": "VAL-CONDITIONING-001",
        "overall_status": "passed",
        "environment": {
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "backend": "comfy_tensor::CpuBackend",
            "device": "cpu",
            "dtype": "f32",
        },
        "summary": { "passed": 17, "failed": 0, "skipped": 0 },
        "implementation": {
            "path": implementation_path,
            "sha256": format!("{:x}", Sha256::digest(implementation)),
        },
        "task_results": task_results,
        "contracts": contracts,
    });
    let directory = repository.join("target/comfy-parity");
    fs::create_dir_all(&directory)?;
    fs::write(
        directory.join("val-conditioning-001.json"),
        serde_json::to_vec_pretty(&artifact)?,
    )?;
    Ok(())
}
