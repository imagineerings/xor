use comfy_media::{PngLimits, encode_png_frame};
use comfy_model::{
    NativeFrameInterpolationModel, NativeModelPayload, NativeSdPoseHeatmapHead, NativeSdPoseModel,
    NativeSdPoseSd2Denoiser, SdPoseHeatmapHeadConfiguration, SdPoseSd2Configuration,
    sdpose_heatmap_head_weight_manifest, sdpose_sd2_weight_manifest,
};
use comfy_nodes::{
    NativePreparedEffectKind, NativeStoredModelPayload, NativeStructuredValue,
    built_in_source_schema,
};
use comfy_runtime::{
    AttemptState, NATIVE_IMAGE_REGISTRY_VERSION, NativeHandleKind, NativeHandleStoreError,
    NativeHandleStoreGeneration, NativeHandleType, NativeImageWorkerEvent, NativeImageWorkerPlan,
    NativeInputDescriptor, NativeNodeDescriptor, NativeNodeFailure, NativeNodeFailureKind,
    NativeNodeOutcome, NativeOpaqueHandle, NativePortCardinality, NativePreparedEffectRequest,
    NativePrimitive, NativePrimitiveType, NativeStoredPayload, NativeTypeUnion, NativeValue,
    NativeValueType, RuntimeSupervisor, SupervisorPolicy, WorkerHealth, WorkerLaunchConfig,
    compile_generated_native_prompt, generated_native_frontend_descriptors,
    generated_native_node_registry_projection, graph_to_prompt, native_image_registry_projection,
};
use comfy_tensor::{
    CancellationToken, CpuBackend, CpuWorkspaceAuthority, DType, ExecutionContext, ImageTensor,
    NativeTensorPayload, NativeTensorRole, StreamId, Tensor, TensorBackend,
    generated_comfy_operator_indirection_01::tensor_from_f32_with_context_exact_native,
};
use comfy_types::{AttemptId, NodeId, ProfileId, PromptId, WorkerId, WorkerMessage};
use serde_json::json;
use std::{
    collections::BTreeMap,
    error::Error,
    fs,
    sync::Arc,
    time::{Duration, Instant},
};
use uuid::Uuid;

const PROFILE_ID: ProfileId = ProfileId(Uuid::from_u128(0x3670));
const WORKFLOW_FIXTURE: &[u8] = include_bytes!("../fixtures/native_image/workflow.json");
const SDPOSE_FIXTURE_ARTIFACT: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn sdpose_tensor(
    backend: &CpuBackend,
    shape: &[u64],
    context: &ExecutionContext<'_>,
) -> Result<Tensor, Box<dyn Error>> {
    let elements = shape.iter().try_fold(1usize, |count, dimension| {
        count.checked_mul(usize::try_from(*dimension).ok()?)
    });
    let elements = elements.ok_or("SDPose fixture tensor shape overflowed")?;
    let mut values = Vec::new();
    values.try_reserve_exact(elements)?;
    values.resize(elements, 0.0);
    Ok(tensor_from_f32_with_context_exact_native(
        backend,
        shape,
        &values,
        DType::F32,
        backend.device(),
        context,
    )?)
}

fn reduced_sdpose_stored_payload() -> Result<NativeStoredPayload, Box<dyn Error>> {
    let workspace_bytes = 64 * 1024 * 1024;
    let (backend, authority) = CpuWorkspaceAuthority::create_backend(workspace_bytes)?;
    let cancellation = CancellationToken::default();
    let context = backend.execution_context(
        StreamId::DEFAULT,
        authority.authorize_workspace(workspace_bytes)?,
        &cancellation,
    );
    let denoiser_configuration = SdPoseSd2Configuration::reduced_fixture(4, 3, 1, 1, 8, 8)?;
    let mut denoiser_weights = BTreeMap::new();
    for specification in sdpose_sd2_weight_manifest(&denoiser_configuration)? {
        denoiser_weights.insert(
            specification.key().to_owned(),
            sdpose_tensor(&backend, specification.shape(), &context)?,
        );
    }
    let denoiser = NativeSdPoseSd2Denoiser::from_reduced_fixture(
        denoiser_configuration,
        denoiser_weights,
        &cancellation,
    )?;
    let head_configuration = SdPoseHeatmapHeadConfiguration::reduced_fixture(8, 8, 3)?;
    let mut head_weights = BTreeMap::new();
    for specification in sdpose_heatmap_head_weight_manifest(&head_configuration)? {
        head_weights.insert(
            specification.key().to_owned(),
            sdpose_tensor(&backend, specification.shape(), &context)?,
        );
    }
    let head = NativeSdPoseHeatmapHead::from_reduced_fixture(
        head_configuration,
        head_weights,
        &cancellation,
    )?;
    let resource = Arc::new(NativeSdPoseModel::from_reduced_fixture(
        SDPOSE_FIXTURE_ARTIFACT.to_owned(),
        denoiser,
        head,
        &cancellation,
    )?);
    let model = Arc::new(NativeModelPayload::sdpose_model_test_fixture(resource)?);
    Ok(NativeStoredPayload::Model(Arc::new(
        NativeStoredModelPayload::model_resource(model)?,
    )))
}

fn reduced_frame_interpolation_stored_payload() -> Result<NativeStoredPayload, Box<dyn Error>> {
    let workspace_bytes = 16 * 1024 * 1024;
    let (backend, authority) = CpuWorkspaceAuthority::create_backend(workspace_bytes)?;
    let cancellation = CancellationToken::default();
    let context = backend.execution_context(
        StreamId::DEFAULT,
        authority.authorize_workspace(workspace_bytes)?,
        &cancellation,
    );
    let resource = Arc::new(NativeFrameInterpolationModel::reduced_rife_test_fixture(
        &backend, &context,
    )?);
    let model = Arc::new(NativeModelPayload::frame_interpolation(resource)?);
    Ok(NativeStoredPayload::Model(Arc::new(
        NativeStoredModelPayload::model_resource(model)?,
    )))
}

#[test]
fn generated_registry_frontend_compiler_and_worker_dispatch_share_one_path()
-> Result<(), Box<dyn Error>> {
    let registry = generated_native_node_registry_projection(None)?;
    registry.validate_comprehensive_bindings()?;
    let early = native_image_registry_projection()?;
    assert!(
        early
            .descriptors()
            .all(|(class_type, _)| registry.descriptor(class_type).is_some())
    );

    let frontend = generated_native_frontend_descriptors(None)?;
    assert_eq!(frontend.len(), registry.descriptor_len());
    assert!(
        registry
            .descriptors()
            .all(|(class_type, _)| frontend.contains_key(class_type))
    );

    let workflow = comfy_runtime::WorkflowFormatDocument::parse(WORKFLOW_FIXTURE)?;
    let submission = graph_to_prompt(&workflow, &frontend, "task367-generated-native")?;
    let mut plan = compile_generated_native_prompt(submission, None)?;
    plan.prompt_id = PromptId(Uuid::from_u128(0x3671));
    assert_eq!(plan.nodes.len(), 5);
    assert_eq!(
        plan.output_nodes,
        vec![NodeId("4".to_owned()), NodeId("5".to_owned())]
    );

    let input_bytes = encode_png_frame(
        &[0.25, 0.5, 0.75],
        1,
        1,
        1,
        3,
        0,
        &BTreeMap::new(),
        PngLimits::default(),
    )?;
    let worker_plan = NativeImageWorkerPlan::new(
        plan.clone(),
        BTreeMap::from([("fixture.png".to_owned(), input_bytes)]),
        true,
        0,
    )?;
    let directory = tempfile::tempdir()?;
    let worker_directory = directory.path().join("worker");
    fs::create_dir(&worker_directory)?;
    let mut launch = WorkerLaunchConfig::new(
        env!("CARGO_BIN_EXE_comfy_native_image_worker_fixture"),
        PROFILE_ID,
        WorkerId(Uuid::from_u128(0x3672)),
        NATIVE_IMAGE_REGISTRY_VERSION,
        1024 * 1024 * 1024,
    );
    launch.working_directory = Some(worker_directory);
    launch.environment = vec![("PATH".to_owned(), String::new())];
    launch.policy = SupervisorPolicy {
        heartbeat_interval: Duration::from_secs(30),
        missed_heartbeat_limit: 3,
        shutdown_timeout: Duration::from_secs(3),
        ready_timeout: Duration::from_secs(10),
        maximum_automatic_restarts: 0,
        restart_backoff: Duration::from_millis(1),
    };
    let mut supervisor = smol::block_on(RuntimeSupervisor::start(launch))?;
    assert_eq!(supervisor.snapshot().health, WorkerHealth::BackendReady);
    smol::block_on(supervisor.execute(
        plan.prompt_id,
        AttemptId(Uuid::from_u128(0x3673)),
        serde_json::to_vec(&worker_plan)?,
    ))?;
    let terminal = smol::block_on(await_terminal_worker_event(&supervisor))?;
    let NativeImageWorkerEvent::Completed { result } = terminal else {
        return Err(format!("generated plan did not complete in the worker: {terminal:?}").into());
    };
    assert_eq!(result.report.state, AttemptState::Succeeded);
    assert_eq!(result.executed_node_count, 5);
    smol::block_on(supervisor.shutdown())?;
    Ok(())
}

#[test]
fn portable_values_dynamic_ports_and_attempt_handles_fail_closed() -> Result<(), Box<dyn Error>> {
    let image_type = NativeHandleType::new(NativeHandleKind::Image, "IMAGE")?;
    let union = NativeTypeUnion::new([
        NativeValueType::Primitive(NativePrimitiveType::Integer),
        NativeValueType::Primitive(NativePrimitiveType::String),
        NativeValueType::Handle(image_type.clone()),
    ])?;
    let descriptor = NativeNodeDescriptor {
        schema_version: comfy_nodes::NATIVE_NODE_CONTRACT_SCHEMA_VERSION,
        class_type: "Task367PortableProbe".to_owned(),
        implementation_version: "1".to_owned(),
        source_schema: Some(comfy_nodes::NativeDescriptorSchemaMetadata::synthetic(
            ["value".to_owned()],
            [comfy_nodes::NativeDynamicSchemaMetadata::compatibility(
                "value_{index}",
                1,
                1,
                8,
                comfy_nodes::NativeInputSchemaMetadata::compatibility("value", "ANY"),
            )],
            std::iter::empty(),
        )),
        inputs: vec![NativeInputDescriptor {
            name: "value".to_owned(),
            accepted_types: union.clone(),
            required: true,
            hidden: false,
            lazy: true,
            cardinality: NativePortCardinality::Mapped,
            allows_literal: true,
        }],
        dynamic_inputs: vec![comfy_runtime::NativeDynamicInputDescriptor {
            name_template: "value_{index}".to_owned(),
            start_index: 1,
            minimum_count: 1,
            maximum_count: 8,
            input: NativeInputDescriptor {
                name: "value".to_owned(),
                accepted_types: union.clone(),
                required: false,
                hidden: false,
                lazy: false,
                cardinality: NativePortCardinality::List,
                allows_literal: true,
            },
        }],
        outputs: Vec::new(),
        output_node: true,
        effect: comfy_runtime::NativeEffectClass::WritesArtifact,
        cache: comfy_runtime::NativeCachePolicy::Never,
    };
    descriptor.validate()?;

    let scalar = NativeValue::Primitive {
        value: NativePrimitive::Integer(7),
    };
    let list = NativeValue::List {
        values: vec![
            scalar.clone(),
            NativeValue::Primitive {
                value: NativePrimitive::String("seven".to_owned()),
            },
        ],
    };
    assert!(union.accepts(&scalar));
    list.validate()?;
    let restored: NativeValue = serde_json::from_slice(&serde_json::to_vec(&list)?)?;
    assert_eq!(restored, list);

    let first_generation = NativeHandleStoreGeneration::with_capacities(4, 1024)?;
    let attempt_id = AttemptId(Uuid::from_u128(0x3674));
    let first_store = first_generation.handle_store_for_attempt(attempt_id);
    let (backend, workspace_authority) = CpuWorkspaceAuthority::create_backend(1024 * 1024)?;
    let image_cancellation = CancellationToken::default();
    let image_context = backend.execution_context(
        StreamId::DEFAULT,
        workspace_authority.authorize_workspace(1024 * 1024)?,
        &image_cancellation,
    );
    let image = ImageTensor::from_f32(&backend, &image_context, 1, 1, 1, 3, &[0.25, 0.5, 0.75])?;
    let payload = NativeStoredPayload::Tensor(Arc::new(NativeTensorPayload::from_image(
        NativeTensorRole::Image,
        image,
    )?));
    let handle = first_store.publish(payload.clone(), &CancellationToken::default())?;
    let handle_value = NativeValue::Handle {
        value: handle.clone(),
    };
    assert!(union.accepts(&handle_value));
    assert!(
        first_store
            .resolve(&handle, &image_type, &CancellationToken::default())
            .is_ok()
    );

    let recovered_generation = NativeHandleStoreGeneration::with_capacities(4, 1024)?;
    let recovered_store = recovered_generation.handle_store_for_attempt(attempt_id);
    assert!(matches!(
        recovered_store.resolve(&handle, &image_type, &CancellationToken::default()),
        Err(NativeHandleStoreError::WrongStore | NativeHandleStoreError::WrongGeneration)
    ));
    let cancellation = CancellationToken::default();
    cancellation.cancel();
    assert!(matches!(
        first_store.publish(payload, &cancellation),
        Err(NativeHandleStoreError::Cancelled)
    ));

    let effect = NativePreparedEffectRequest::checked(
        Uuid::from_u128(0x3674),
        Uuid::from_u128(0x3675),
        NativePreparedEffectKind::Output,
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    )?;
    effect.validate()?;
    NativeNodeOutcome::Values {
        outputs: vec![list],
        ui: Some(json!({"task": 367})),
        effects: vec![effect],
    }
    .validate()?;
    NativeNodeOutcome::Blocked {
        reason: "provider activation required".to_owned(),
    }
    .validate()?;
    assert!(
        NativeNodeOutcome::Expansion {
            prompt: comfy_types::ApiPrompt::default(),
            output_node: NodeId("missing".to_owned()),
        }
        .validate()
        .is_err()
    );
    let failure = NativeNodeFailure {
        code: "task367_failure".to_owned(),
        message: "deterministic failure".to_owned(),
        kind: NativeNodeFailureKind::Failure,
        retryable: false,
    };
    failure.validate()?;
    assert_eq!(
        serde_json::from_slice::<NativeNodeFailure>(&serde_json::to_vec(&failure)?)?,
        failure
    );
    Ok(())
}

#[test]
fn source_structured_values_keep_resolved_handles_typed_across_recovery()
-> Result<(), Box<dyn Error>> {
    let schema = built_in_source_schema("ResizeImageMaskNode")?;
    let resize_type = schema
        .inputs
        .iter()
        .find(|input| input.schema.name == "resize_type")
        .ok_or("ResizeImageMaskNode has no resize_type input")?;
    let match_size = resize_type
        .schema
        .structured_options()?
        .into_iter()
        .find(|option| option.selector == "match size")
        .ok_or("ResizeImageMaskNode has no match size option")?;
    assert!(match_size.fields.iter().any(|field| {
        field.path.as_slice() == ["match"]
            && field.schema.source_type_names.as_slice() == ["IMAGE", "MASK"]
    }));

    let generation = NativeHandleStoreGeneration::with_capacities(4, 1024)?;
    let attempt_id = AttemptId(Uuid::from_u128(0x3760));
    let store = generation.handle_store_for_attempt(attempt_id);
    let (backend, workspace_authority) = CpuWorkspaceAuthority::create_backend(1024 * 1024)?;
    let cancellation = CancellationToken::default();
    let context = backend.execution_context(
        StreamId::DEFAULT,
        workspace_authority.authorize_workspace(1024 * 1024)?,
        &cancellation,
    );
    let image = ImageTensor::from_f32(&backend, &context, 1, 1, 1, 3, &[0.1, 0.2, 0.3])?;
    let payload = NativeStoredPayload::Tensor(Arc::new(NativeTensorPayload::from_image(
        NativeTensorRole::Image,
        image,
    )?));
    let handle = store.publish(payload, &cancellation)?;
    let structured = NativeStructuredValue::checked(
        "COMFY_DYNAMICCOMBO_V3",
        BTreeMap::from([
            (
                "resize_type".to_owned(),
                NativeValue::Primitive {
                    value: NativePrimitive::String("match size".to_owned()),
                },
            ),
            (
                "crop".to_owned(),
                NativeValue::Primitive {
                    value: NativePrimitive::String("center".to_owned()),
                },
            ),
            (
                "match".to_owned(),
                NativeValue::Handle {
                    value: handle.clone(),
                },
            ),
        ]),
    )?;
    let value = structured.into_native_value();
    let expected = NativeTypeUnion::new([NativeValueType::NamedPreservedUnknown(
        "COMFY_DYNAMICCOMBO_V3".to_owned(),
    )])?;
    assert!(expected.accepts(&value));
    let restored: NativeValue = serde_json::from_slice(&serde_json::to_vec(&value)?)?;
    let restored = NativeStructuredValue::from_native_value(&restored)?
        .ok_or("structured value lost its typed representation")?;
    assert_eq!(
        restored.get("match"),
        Some(&NativeValue::Handle {
            value: handle.clone(),
        })
    );
    let image_type = NativeHandleType::new(NativeHandleKind::Image, "IMAGE")?;
    store.resolve(&handle, &image_type, &cancellation)?;

    let recovered =
        NativeHandleStoreGeneration::with_capacities(4, 1024)?.handle_store_for_attempt(attempt_id);
    assert!(matches!(
        recovered.resolve(&handle, &image_type, &cancellation),
        Err(NativeHandleStoreError::WrongStore | NativeHandleStoreError::WrongGeneration)
    ));
    let cancelled = CancellationToken::default();
    cancelled.cancel();
    assert!(matches!(
        store.resolve(&handle, &image_type, &cancelled),
        Err(NativeHandleStoreError::Cancelled)
    ));
    Ok(())
}

#[test]
fn sdpose_model_resource_handle_is_sealed_alias_aware_and_restart_safe()
-> Result<(), Box<dyn Error>> {
    let payload = reduced_sdpose_stored_payload()?;
    payload.validate()?;
    let handle_type = payload.handle_type()?;
    assert_eq!(handle_type.kind, NativeHandleKind::Model);
    assert_eq!(handle_type.type_id, "MODEL");
    let byte_capacity = payload.resident_bytes()?;
    let generation = NativeHandleStoreGeneration::with_capacities(3, byte_capacity)?;
    let attempt_id = AttemptId(Uuid::from_u128(0x4030));
    let store = generation.handle_store_for_attempt(attempt_id);
    let cancellation = CancellationToken::default();
    let first = store.publish(payload.clone(), &cancellation)?;
    let first_bytes = generation.resident_bytes();
    assert_eq!(first_bytes, byte_capacity);
    let second = store.publish(payload, &cancellation)?;
    assert_eq!(generation.len(), 2);
    assert_eq!(generation.resident_bytes(), first_bytes);

    let resolved = store.resolve(&first, &handle_type, &cancellation)?;
    let NativeStoredPayload::Model(model) = resolved.as_ref() else {
        return Err("SDPose MODEL handle resolved to another stored payload kind".into());
    };
    assert!(model.model_payload().sdpose_model_resource().is_some());
    assert_eq!(Some(model.digest_sha256()), first.digest_sha256());

    let distinct = reduced_sdpose_stored_payload()?;
    assert_eq!(distinct.digest_sha256(), model.digest_sha256());
    assert!(matches!(
        store.publish(distinct, &cancellation),
        Err(NativeHandleStoreError::Rejected(message)) if message.contains("capacity is exhausted")
    ));
    assert_eq!(generation.len(), 2);
    assert_eq!(generation.resident_bytes(), first_bytes);

    let forged = NativeOpaqueHandle::new(
        handle_type.clone(),
        first.store_identity(),
        first.identifier(),
        first.generation(),
        Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_owned()),
    )?;
    assert!(matches!(
        store.resolve(&forged, &handle_type, &cancellation),
        Err(NativeHandleStoreError::DigestMismatch)
    ));

    let restarted = NativeHandleStoreGeneration::with_capacities(2, byte_capacity)?
        .handle_store_for_attempt(attempt_id);
    assert!(matches!(
        restarted.resolve(&first, &handle_type, &cancellation),
        Err(NativeHandleStoreError::WrongStore | NativeHandleStoreError::WrongGeneration)
    ));

    let cancelled = CancellationToken::default();
    cancelled.cancel();
    let before_len = generation.len();
    let before_bytes = generation.resident_bytes();
    assert!(matches!(
        store.publish(reduced_sdpose_stored_payload()?, &cancelled),
        Err(NativeHandleStoreError::Cancelled)
    ));
    assert_eq!(generation.len(), before_len);
    assert_eq!(generation.resident_bytes(), before_bytes);
    assert_ne!(first.identifier(), second.identifier());
    Ok(())
}

#[test]
fn frame_interpolation_resource_handle_is_concrete_alias_aware_and_restart_safe()
-> Result<(), Box<dyn Error>> {
    let payload = reduced_frame_interpolation_stored_payload()?;
    payload.validate()?;
    let handle_type = payload.handle_type()?;
    assert_eq!(handle_type.kind, NativeHandleKind::Model);
    assert_eq!(handle_type.type_id, "INTERP_MODEL");
    let byte_capacity = payload.resident_bytes()?;
    let generation = NativeHandleStoreGeneration::with_capacities(3, byte_capacity)?;
    let attempt_id = AttemptId(Uuid::from_u128(0x4080));
    let store = generation.handle_store_for_attempt(attempt_id);
    let cancellation = CancellationToken::default();
    let first = store.publish(payload.clone(), &cancellation)?;
    let first_bytes = generation.resident_bytes();
    let second = store.publish(payload, &cancellation)?;
    assert_eq!(generation.len(), 2);
    assert_eq!(generation.resident_bytes(), first_bytes);

    let resolved = store.resolve(&first, &handle_type, &cancellation)?;
    let NativeStoredPayload::Model(model) = resolved.as_ref() else {
        return Err("INTERP_MODEL handle resolved to another stored payload kind".into());
    };
    assert!(
        model
            .model_payload()
            .frame_interpolation_resource()
            .is_some()
    );
    assert_eq!(Some(model.digest_sha256()), first.digest_sha256());

    let distinct = reduced_frame_interpolation_stored_payload()?;
    assert_eq!(distinct.digest_sha256(), model.digest_sha256());
    assert!(matches!(
        store.publish(distinct, &cancellation),
        Err(NativeHandleStoreError::Rejected(message)) if message.contains("capacity is exhausted")
    ));
    assert_eq!(generation.len(), 2);
    assert_eq!(generation.resident_bytes(), first_bytes);

    let forged = NativeOpaqueHandle::new(
        handle_type.clone(),
        first.store_identity(),
        first.identifier(),
        first.generation(),
        Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_owned()),
    )?;
    assert!(matches!(
        store.resolve(&forged, &handle_type, &cancellation),
        Err(NativeHandleStoreError::DigestMismatch)
    ));
    let restarted = NativeHandleStoreGeneration::with_capacities(2, byte_capacity)?
        .handle_store_for_attempt(attempt_id);
    assert!(matches!(
        restarted.resolve(&first, &handle_type, &cancellation),
        Err(NativeHandleStoreError::WrongStore | NativeHandleStoreError::WrongGeneration)
    ));
    assert_ne!(first.identifier(), second.identifier());
    Ok(())
}

async fn await_terminal_worker_event(
    supervisor: &RuntimeSupervisor,
) -> Result<NativeImageWorkerEvent, Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("generated native worker dispatch timed out".into());
        }
        let envelope = supervisor.next_event(remaining).await?;
        if let WorkerMessage::Event { event } = envelope.message
            && let Ok(event) = postcard::from_bytes::<NativeImageWorkerEvent>(&event)
            && matches!(
                event,
                NativeImageWorkerEvent::Completed { .. }
                    | NativeImageWorkerEvent::BackendUnavailable { .. }
                    | NativeImageWorkerEvent::Failed { .. }
            )
        {
            return Ok(event);
        }
    }
}
