use comfy_media::{PngLimits, encode_png_frame};
use comfy_runtime::{
    AttemptState, NATIVE_IMAGE_REGISTRY_VERSION, NativeHandleKind, NativeHandleStoreError,
    NativeHandleStoreGeneration, NativeHandleType, NativeImageWorkerEvent, NativeImageWorkerPlan,
    NativeInputDescriptor, NativeNodeDescriptor, NativeNodeFailure, NativeNodeFailureKind,
    NativeNodeOutcome, NativePortCardinality, NativePreparedEffectRequest, NativePrimitive,
    NativePrimitiveType, NativeTypeUnion, NativeValue, NativeValueType, RuntimeSupervisor,
    SupervisorPolicy, WorkerHealth, WorkerLaunchConfig, compile_generated_native_prompt,
    generated_native_frontend_descriptors, generated_native_node_registry_projection,
    graph_to_prompt, native_image_registry_projection,
};
use comfy_tensor::CancellationToken;
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
        schema_version: comfy_runtime::NATIVE_NODE_CONTRACT_SCHEMA_VERSION,
        class_type: "Task367PortableProbe".to_owned(),
        implementation_version: "1".to_owned(),
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
    let handle = first_store.publish(
        image_type.clone(),
        Arc::new(vec![1_u8, 2, 3]),
        None,
        3,
        &CancellationToken::default(),
    )?;
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
        first_store.publish(image_type, Arc::new(0_u8), None, 1, &cancellation),
        Err(NativeHandleStoreError::Cancelled)
    ));

    let effect = NativePreparedEffectRequest {
        transaction_id: Uuid::from_u128(0x3675),
        metadata: b"prepared-only".to_vec(),
    };
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
