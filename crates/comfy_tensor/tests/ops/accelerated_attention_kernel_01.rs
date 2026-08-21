use comfy_tensor::{
    CancellationToken, CpuWorkspaceAuthority, DeviceId,
    GENERATED_RESOLVED_OPERATION_CONTRACT_SLICES, OperationContractId, StreamId,
    generated_accelerated_attention_kernel_01::{
        AttentionKernelError, AttentionKernelKind, AttentionKernelRequest, AttentionLayout,
        AttentionMask, AttentionMaskShape, AttentionShape, CheckedAttentionInvocation,
        FLASH_ATTENTION_OPERATION_ID, SAGE_ATTENTION_3_OPERATION_ID, SAGE_ATTENTION_OPERATION_ID,
        XFORMERS_ATTENTION_OPERATION_ID, flash_attn_func_with_context_exact_native,
        memory_efficient_attention_with_context_exact_native, sageattn_with_context_exact_native,
        sageattn3_blackwell_with_context_exact_native,
    },
};
use comfy_types::DeviceKind;

const SHAPE: AttentionShape = AttentionShape {
    batch: 1,
    query_tokens: 2,
    key_tokens: 2,
    heads: 1,
    head_dimension: 2,
    value_dimension: 2,
};

fn request(kind: AttentionKernelKind, layout: AttentionLayout) -> AttentionKernelRequest {
    AttentionKernelRequest {
        kind,
        device: DeviceId::CPU,
        layout,
        shape: SHAPE,
        scale: Some(1.0),
        causal: false,
        dropout_probability: 0.0,
    }
}

fn attention_cases<'a>(
    boolean_mask: &'a [bool; 4],
    additive_mask: &'a [f32; 4],
) -> [(
    AttentionKernelKind,
    AttentionLayout,
    Option<AttentionMask<'a>>,
); 4] {
    [
        (
            AttentionKernelKind::FlashAttention,
            AttentionLayout::Nhd,
            None,
        ),
        (
            AttentionKernelKind::SageAttention,
            AttentionLayout::Nhd,
            Some(AttentionMask::Boolean {
                values: boolean_mask,
                shape: AttentionMaskShape::QueryByKey,
            }),
        ),
        (
            AttentionKernelKind::SageAttention3Blackwell,
            AttentionLayout::Hnd,
            None,
        ),
        (
            AttentionKernelKind::XformersMemoryEfficient,
            AttentionLayout::Nhd,
            Some(AttentionMask::Additive {
                values: additive_mask,
                shape: AttentionMaskShape::BatchHeadQueryByKey,
            }),
        ),
    ]
}

fn assert_close(actual: &[f32], expected: &[f32], tolerance: f32) {
    assert_eq!(actual.len(), expected.len());
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert!(
            (actual - expected).abs() <= tolerance,
            "value {index}: expected {expected}, got {actual}"
        );
    }
}

#[test]
fn all_four_external_contracts_are_build_sealed_once() {
    let slice = GENERATED_RESOLVED_OPERATION_CONTRACT_SLICES
        .iter()
        .find(|slice| slice.module_name == "accelerated_attention_kernel_01")
        .unwrap_or_else(|| panic!("accelerated attention resolution slice is missing"));
    assert_eq!(slice.len(), 4);
    for operation_id in [
        FLASH_ATTENTION_OPERATION_ID,
        SAGE_ATTENTION_OPERATION_ID,
        SAGE_ATTENTION_3_OPERATION_ID,
        XFORMERS_ATTENTION_OPERATION_ID,
    ] {
        assert_eq!(
            slice
                .contracts
                .iter()
                .filter(|contract| contract.operation_id == operation_id)
                .count(),
            1
        );
        assert!(OperationContractId::new(operation_id).is_ok());
    }
}

#[test]
fn wrappers_preserve_exact_layout_mask_causal_and_fresh_output_semantics() {
    let query = [1.0, 0.0, 0.0, 1.0];
    let key = query;
    let value = [2.0, 0.0, 0.0, 4.0];
    let cancellation = CancellationToken::default();
    let (backend, workspace_authority) = CpuWorkspaceAuthority::create_backend(1_024)
        .unwrap_or_else(|error| panic!("backend construction failed: {error}"));
    let context = backend.execution_context(
        StreamId::DEFAULT,
        workspace_authority
            .authorize_workspace(256)
            .unwrap_or_else(|error| panic!("workspace authorization failed: {error}")),
        &cancellation,
    );
    let flash = flash_attn_func_with_context_exact_native(
        &backend,
        SHAPE,
        &query,
        &key,
        &value,
        0.0,
        false,
        DeviceId::CPU,
        &context,
    )
    .unwrap_or_else(|error| panic!("flash exact fallback failed: {error}"));
    assert_close(
        &flash,
        &[1.339_523_1, 1.320_953_7, 0.660_476_9, 2.679_046_2],
        0.000_001,
    );
    assert_eq!(query, [1.0, 0.0, 0.0, 1.0]);

    let boolean = [true, false, true, true];
    let sage = sageattn_with_context_exact_native(
        &backend,
        SHAPE,
        &query,
        &key,
        &value,
        Some(AttentionMask::Boolean {
            values: &boolean,
            shape: AttentionMaskShape::QueryByKey,
        }),
        false,
        AttentionLayout::Nhd,
        DeviceId::CPU,
        &context,
    )
    .unwrap_or_else(|error| panic!("sage exact fallback failed: {error}"));
    assert_close(&sage, &[2.0, 0.0, 0.660_476_9, 2.679_046_2], 0.000_001);

    let bias = [0.0, f32::NEG_INFINITY, 0.0, 0.0];
    let xformers = memory_efficient_attention_with_context_exact_native(
        &backend,
        SHAPE,
        &query,
        &key,
        &value,
        Some(&bias),
        DeviceId::CPU,
        &context,
    )
    .unwrap_or_else(|error| panic!("xformers exact fallback failed: {error}"));
    assert_eq!(xformers, sage);

    let causal = flash_attn_func_with_context_exact_native(
        &backend,
        SHAPE,
        &query,
        &key,
        &value,
        0.0,
        true,
        DeviceId::CPU,
        &context,
    )
    .unwrap_or_else(|error| panic!("causal flash exact fallback failed: {error}"));
    assert_close(&causal, &[2.0, 0.0, 0.660_476_9, 2.679_046_2], 0.000_001);
}

#[test]
fn hnd_sage_contracts_match_nhd_projection_with_multiple_heads() {
    let shape = AttentionShape {
        heads: 2,
        head_dimension: 1,
        value_dimension: 1,
        ..SHAPE
    };
    let query_nhd = [1.0, 2.0, 3.0, 4.0];
    let key_nhd = [1.0, 2.0, 3.0, 4.0];
    let value_nhd = [5.0, 6.0, 7.0, 8.0];
    let query_hnd = [1.0, 3.0, 2.0, 4.0];
    let key_hnd = [1.0, 3.0, 2.0, 4.0];
    let value_hnd = [5.0, 7.0, 6.0, 8.0];
    let cancellation = CancellationToken::default();
    let (backend, workspace_authority) = CpuWorkspaceAuthority::create_backend(1_024)
        .unwrap_or_else(|error| panic!("backend construction failed: {error}"));
    let context = backend.execution_context(
        StreamId::DEFAULT,
        workspace_authority
            .authorize_workspace(256)
            .unwrap_or_else(|error| panic!("workspace authorization failed: {error}")),
        &cancellation,
    );
    let nhd = sageattn_with_context_exact_native(
        &backend,
        shape,
        &query_nhd,
        &key_nhd,
        &value_nhd,
        None,
        false,
        AttentionLayout::Nhd,
        DeviceId::CPU,
        &context,
    )
    .unwrap_or_else(|error| panic!("NHD sage failed: {error}"));
    let hnd = sageattn3_blackwell_with_context_exact_native(
        &backend,
        shape,
        &query_hnd,
        &key_hnd,
        &value_hnd,
        false,
        DeviceId::CPU,
        &context,
    )
    .unwrap_or_else(|error| panic!("HND sage3 fallback failed: {error}"));
    assert_close(&hnd, &[nhd[0], nhd[2], nhd[1], nhd[3]], 0.000_001);
}

#[test]
fn invalid_boundaries_and_uncertified_devices_fail_typed_before_output() {
    let query = [1.0, 0.0, 0.0, 1.0];
    let key = query;
    let value = [2.0, 0.0, 0.0, 4.0];
    let boolean_mask = [true, false, true, true];
    let additive_mask = [0.0, -0.75, 0.25, 0.0];
    let cancellation = CancellationToken::default();
    let (backend, workspace_authority) = CpuWorkspaceAuthority::create_backend(1_024)
        .unwrap_or_else(|error| panic!("backend construction failed: {error}"));
    let context = backend.execution_context(
        StreamId::DEFAULT,
        workspace_authority
            .authorize_workspace(256)
            .unwrap_or_else(|error| panic!("workspace authorization failed: {error}")),
        &cancellation,
    );
    assert!(matches!(
        flash_attn_func_with_context_exact_native(
            &backend,
            SHAPE,
            &query,
            &key,
            &value,
            0.1,
            false,
            DeviceId::CPU,
            &context,
        ),
        Err(AttentionKernelError::UnsupportedDropout)
    ));
    assert!(matches!(
        flash_attn_func_with_context_exact_native(
            &backend,
            SHAPE,
            &query,
            &key[..3],
            &value,
            0.0,
            false,
            DeviceId::CPU,
            &context,
        ),
        Err(AttentionKernelError::ValueCount { name: "key", .. })
    ));
    assert!(matches!(
        sageattn_with_context_exact_native(
            &backend,
            SHAPE,
            &query,
            &key,
            &value,
            Some(AttentionMask::Boolean {
                values: &[true],
                shape: AttentionMaskShape::QueryByKey,
            }),
            false,
            AttentionLayout::Nhd,
            DeviceId::CPU,
            &context,
        ),
        Err(AttentionKernelError::MaskValueCount { .. })
    ));
    for (kind, layout, mask) in attention_cases(&boolean_mask, &additive_mask) {
        let mut unsupported = request(kind, layout);
        unsupported.device = DeviceId::new(DeviceKind::Cuda, 0);
        assert!(
            matches!(
                CheckedAttentionInvocation::new(unsupported, &query, &key, &value, mask),
                Err(AttentionKernelError::UnsupportedDevice { .. })
            ),
            "{kind:?} accepted an uncertified device"
        );
    }
    assert!(matches!(
        CheckedAttentionInvocation::new(
            request(AttentionKernelKind::FlashAttention, AttentionLayout::Hnd),
            &query,
            &key,
            &value,
            None,
        ),
        Err(AttentionKernelError::UnsupportedLayout { .. })
    ));
}

#[test]
fn every_external_contract_has_adjoint_analytical_gradients_matching_finite_difference() {
    let query = [0.2, -0.4, 0.7, 0.1];
    let key = [0.3, 0.6, -0.2, 0.8];
    let value = [1.1, -0.5, 0.2, 0.9];
    let query_tangent = [0.1, -0.3, 0.2, 0.4];
    let key_tangent = [-0.2, 0.5, 0.3, -0.1];
    let value_tangent = [0.6, -0.4, 0.2, 0.1];
    let output_gradient = [0.7, -0.8, 0.3, 0.5];
    let boolean_mask = [true, false, true, true];
    let additive_mask = [0.0, -0.75, 0.25, 0.0];
    let cancellation = CancellationToken::default();
    let (backend, workspace_authority) = CpuWorkspaceAuthority::create_backend(1_024)
        .unwrap_or_else(|error| panic!("backend construction failed: {error}"));
    let context = backend.execution_context(
        StreamId::DEFAULT,
        workspace_authority
            .authorize_workspace(256)
            .unwrap_or_else(|error| panic!("workspace authorization failed: {error}")),
        &cancellation,
    );
    let epsilon = 0.000_5;
    let plus_query = add_scaled(&query, &query_tangent, epsilon);
    let plus_key = add_scaled(&key, &key_tangent, epsilon);
    let plus_value = add_scaled(&value, &value_tangent, epsilon);
    let minus_query = add_scaled(&query, &query_tangent, -epsilon);
    let minus_key = add_scaled(&key, &key_tangent, -epsilon);
    let minus_value = add_scaled(&value, &value_tangent, -epsilon);
    for (kind, layout, mask) in attention_cases(&boolean_mask, &additive_mask) {
        let invocation =
            CheckedAttentionInvocation::new(request(kind, layout), &query, &key, &value, mask)
                .unwrap_or_else(|error| panic!("{kind:?} checked invocation failed: {error}"));
        let jvp = invocation
            .jvp_with_context(
                &backend,
                &query_tangent,
                &key_tangent,
                &value_tangent,
                &context,
            )
            .unwrap_or_else(|error| panic!("{kind:?} JVP failed: {error}"));
        let vjp = invocation
            .vjp_with_context(&backend, &output_gradient, &context)
            .unwrap_or_else(|error| panic!("{kind:?} VJP failed: {error}"));
        let left = dot(&jvp, &output_gradient);
        let right = dot(&query_tangent, &vjp.query)
            + dot(&key_tangent, &vjp.key)
            + dot(&value_tangent, &vjp.value);
        assert!(
            (left - right).abs() <= 0.000_01,
            "{kind:?} adjoint mismatch: {left} vs {right}"
        );

        let plus = CheckedAttentionInvocation::new(
            request(kind, layout),
            &plus_query,
            &plus_key,
            &plus_value,
            mask,
        )
        .and_then(|invocation| invocation.execute_with_context(&backend, 1, &context))
        .unwrap_or_else(|error| panic!("{kind:?} positive finite difference failed: {error}"));
        let minus = CheckedAttentionInvocation::new(
            request(kind, layout),
            &minus_query,
            &minus_key,
            &minus_value,
            mask,
        )
        .and_then(|invocation| invocation.execute_with_context(&backend, 1, &context))
        .unwrap_or_else(|error| panic!("{kind:?} negative finite difference failed: {error}"));
        let finite_difference = plus
            .iter()
            .zip(&minus)
            .map(|(plus, minus)| (plus - minus) / (2.0 * epsilon))
            .collect::<Vec<_>>();
        assert_close(&jvp, &finite_difference, 0.000_3);
    }
}

#[test]
fn every_external_contract_checks_cancellation_before_forward_and_gradient_publication() {
    let query = [1.0, 0.0, 0.0, 1.0];
    let key = query;
    let value = [2.0, 0.0, 0.0, 4.0];
    let boolean_mask = [true, false, true, true];
    let additive_mask = [0.0, -0.75, 0.25, 0.0];
    let cancellation = CancellationToken::default();
    cancellation.cancel();
    let (backend, workspace_authority) = CpuWorkspaceAuthority::create_backend(1_024)
        .unwrap_or_else(|error| panic!("backend construction failed: {error}"));
    let context = backend.execution_context(
        StreamId::DEFAULT,
        workspace_authority
            .authorize_workspace(256)
            .unwrap_or_else(|error| panic!("workspace authorization failed: {error}")),
        &cancellation,
    );
    for (kind, layout, mask) in attention_cases(&boolean_mask, &additive_mask) {
        let invocation =
            CheckedAttentionInvocation::new(request(kind, layout), &query, &key, &value, mask)
                .unwrap_or_else(|error| panic!("{kind:?} checked invocation failed: {error}"));
        assert!(matches!(
            invocation.execute_with_context(&backend, 1, &context),
            Err(AttentionKernelError::Cancelled)
        ));
        assert!(matches!(
            invocation.vjp_with_context(&backend, &[1.0; 4], &context),
            Err(AttentionKernelError::Cancelled)
        ));
        assert!(matches!(
            invocation.jvp_with_context(&backend, &[0.0; 4], &[0.0; 4], &[0.0; 4], &context),
            Err(AttentionKernelError::Cancelled)
        ));
    }
}

#[test]
fn every_external_contract_leases_exact_simultaneous_rows_and_converges() {
    let (backend, workspace_authority) = CpuWorkspaceAuthority::create_backend(1_024)
        .unwrap_or_else(|error| panic!("CPU backend construction failed: {error}"));
    let cancellation = CancellationToken::default();
    let query = [0.2, -0.4, 0.7, 0.1];
    let key = [0.3, 0.6, -0.2, 0.8];
    let value = [1.1, -0.5, 0.2, 0.9];
    let boolean_mask = [true, false, true, true];
    let additive_mask = [0.0, -0.75, 0.25, 0.0];
    for (kind, layout, mask) in attention_cases(&boolean_mask, &additive_mask) {
        let invocation =
            CheckedAttentionInvocation::new(request(kind, layout), &query, &key, &value, mask)
                .unwrap_or_else(|error| panic!("{kind:?} checked invocation failed: {error}"));
        let exact = backend.execution_context(
            StreamId::DEFAULT,
            workspace_authority
                .authorize_workspace(16)
                .unwrap_or_else(|error| panic!("workspace authorization failed: {error}")),
            &cancellation,
        );
        let gradients = invocation
            .vjp_with_context(&backend, &[0.7, -0.8, 0.3, 0.5], &exact)
            .unwrap_or_else(|error| panic!("{kind:?} canonical VJP failed: {error}"));
        assert_eq!(gradients.query.len(), query.len());
        assert_eq!(exact.scratch.in_use_bytes(), 0);
        assert_eq!(exact.scratch.peak_bytes(), 16);
        assert_eq!(backend.memory_snapshot().current_bytes, 0);

        let underauthorized = backend.execution_context(
            StreamId::DEFAULT,
            workspace_authority
                .authorize_workspace(15)
                .unwrap_or_else(|error| panic!("workspace authorization failed: {error}")),
            &cancellation,
        );
        assert!(
            invocation
                .vjp_with_context(&backend, &[0.7, -0.8, 0.3, 0.5], &underauthorized)
                .is_err(),
            "{kind:?} accepted an underauthorized gradient workspace"
        );
        assert_eq!(underauthorized.scratch.in_use_bytes(), 0);
        assert_eq!(backend.memory_snapshot().current_bytes, 0);
    }
}

#[test]
fn canonical_attention_cancellation_and_backend_oom_publish_nothing() {
    let (backend, workspace_authority) = CpuWorkspaceAuthority::create_backend(64)
        .unwrap_or_else(|error| panic!("CPU backend construction failed: {error}"));
    let cancellation = CancellationToken::default();
    let occupied_context = backend.execution_context(
        StreamId::DEFAULT,
        workspace_authority
            .authorize_workspace(64)
            .unwrap_or_else(|error| panic!("workspace authorization failed: {error}")),
        &cancellation,
    );
    let occupied = backend
        .reserve_workspace(&occupied_context, 64)
        .unwrap_or_else(|error| panic!("persistent workspace reservation failed: {error}"));
    let execution = backend.execution_context(
        StreamId::DEFAULT,
        workspace_authority
            .authorize_workspace(16)
            .unwrap_or_else(|error| panic!("workspace authorization failed: {error}")),
        &cancellation,
    );
    let query = [1.0, 0.0, 0.0, 1.0];
    let key = query;
    let value = [2.0, 0.0, 0.0, 4.0];
    let result = flash_attn_func_with_context_exact_native(
        &backend,
        SHAPE,
        &query,
        &key,
        &value,
        0.0,
        false,
        DeviceId::CPU,
        &execution,
    );
    assert!(matches!(result, Err(AttentionKernelError::Tensor(_))));
    assert_eq!(execution.scratch.in_use_bytes(), 0);
    drop(occupied);
    assert_eq!(backend.memory_snapshot().current_bytes, 0);

    cancellation.cancel();
    assert!(matches!(
        flash_attn_func_with_context_exact_native(
            &backend,
            SHAPE,
            &query,
            &key,
            &value,
            0.0,
            false,
            DeviceId::CPU,
            &execution,
        ),
        Err(AttentionKernelError::Tensor(_)) | Err(AttentionKernelError::Cancelled)
    ));
    assert_eq!(execution.scratch.in_use_bytes(), 0);
    assert_eq!(backend.memory_snapshot().current_bytes, 0);
}

fn dot(left: &[f32], right: &[f32]) -> f32 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

fn add_scaled(values: &[f32], tangent: &[f32], scale: f32) -> Vec<f32> {
    values
        .iter()
        .zip(tangent)
        .map(|(value, tangent)| value + tangent * scale)
        .collect()
}
