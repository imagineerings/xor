use comfy_media::NativePoseKeypoint;
use comfy_model::{
    SDPOSE_HEATMAP_CHANNELS, SDPOSE_HEATMAP_HEIGHT, SDPOSE_HEATMAP_WIDTH, SdPoseProjectionError,
    SdPoseRawKeypoint, decode_sdpose_heatmaps, project_sdpose_openpose_person,
};
use comfy_tensor::{CancellationToken, CpuWorkspaceAuthority, ExecutionContext, StreamId};
use std::error::Error;

const WORKSPACE_BYTES: u64 = 4 * 1024 * 1024;

fn raw_points(score: f32) -> Result<Vec<SdPoseRawKeypoint>, SdPoseProjectionError> {
    (0..SDPOSE_HEATMAP_CHANNELS)
        .map(|index| {
            let index = f32::from(
                u16::try_from(index).map_err(|_| SdPoseProjectionError::InvalidHeatmapShape)?,
            );
            SdPoseRawKeypoint::checked(index, index + 0.5, score)
        })
        .collect()
}

#[test]
fn sdpose_projection_preserves_raw_scores_and_openpose_cardinalities() -> Result<(), Box<dyn Error>>
{
    let mut raw = raw_points(1.25)?;
    raw[5] = SdPoseRawKeypoint::checked(10.0, 20.0, 0.5)?;
    raw[6] = SdPoseRawKeypoint::checked(14.0, 24.0, 0.4)?;
    let person = project_sdpose_openpose_person(&raw)?;

    assert_eq!(person.pose().len(), 18);
    assert_eq!(person.foot().len(), 6);
    assert_eq!(person.face().len(), 70);
    assert_eq!(person.hand_right().len(), 21);
    assert_eq!(person.hand_left().len(), 21);
    assert_eq!(person.pose()[1].x(), 12.0);
    assert_eq!(person.pose()[1].y(), 22.0);
    assert!((person.pose()[1].score() - 0.4).abs() < 1.0e-6);
    assert_eq!(person.face()[68], person.pose()[14]);
    assert_eq!(person.face()[69], person.pose()[15]);
    assert!(NativePoseKeypoint::checked(0.0, 0.0, -2.0).is_ok());
    assert!(NativePoseKeypoint::checked(0.0, 0.0, 3.0).is_ok());
    Ok(())
}

#[test]
fn sdpose_projection_uses_strict_neck_threshold() -> Result<(), Box<dyn Error>> {
    let mut raw = raw_points(0.5)?;
    raw[5] = SdPoseRawKeypoint::checked(10.0, 20.0, 0.3)?;
    raw[6] = SdPoseRawKeypoint::checked(14.0, 24.0, 0.9)?;
    let person = project_sdpose_openpose_person(&raw)?;
    assert_eq!(person.pose()[1].score(), 0.0);
    Ok(())
}

#[test]
fn sdpose_heatmap_decode_is_bounded_cancellable_and_first_index_stable()
-> Result<(), Box<dyn Error>> {
    let plane = SDPOSE_HEATMAP_HEIGHT * SDPOSE_HEATMAP_WIDTH;
    let mut heatmaps = vec![0.0; SDPOSE_HEATMAP_CHANNELS * plane];
    for channel in 0..SDPOSE_HEATMAP_CHANNELS - 2 {
        let offset = channel * plane;
        heatmaps[offset + 10 * SDPOSE_HEATMAP_WIDTH + 20] = 2.0;
        heatmaps[offset + 10 * SDPOSE_HEATMAP_WIDTH + 21] = 2.0;
    }
    heatmaps[0] = 3.0;
    let negative_offset = (SDPOSE_HEATMAP_CHANNELS - 1) * plane;
    heatmaps[negative_offset..].fill(-1.0);
    let (backend, authority) = CpuWorkspaceAuthority::create_backend(WORKSPACE_BYTES)?;
    let cancellation = CancellationToken::default();
    let context = ExecutionContext {
        stream: StreamId::DEFAULT,
        scratch: authority.authorize_workspace(WORKSPACE_BYTES)?,
        rng_phase: None,
        cancellation: &cancellation,
    };
    let points = decode_sdpose_heatmaps(&heatmaps, 1, &backend, &context)?;
    assert_eq!(points.len(), 1);
    assert_eq!(points[0].len(), SDPOSE_HEATMAP_CHANNELS);
    assert_eq!(points[0][0].score(), 3.0);
    assert!(points[0][0].x().is_finite());
    assert!(points[0][0].y().is_finite());
    assert!(points[0][1].x() < 21.0 * 767.0 / 191.0);
    assert_eq!(points[0][SDPOSE_HEATMAP_CHANNELS - 2].x(), -1.0);
    assert_eq!(points[0][SDPOSE_HEATMAP_CHANNELS - 2].score(), 0.0);
    assert_eq!(points[0][SDPOSE_HEATMAP_CHANNELS - 1].x(), -1.0);
    assert_eq!(points[0][SDPOSE_HEATMAP_CHANNELS - 1].score(), -1.0);
    assert_eq!(context.scratch.in_use_bytes(), 0);

    cancellation.cancel();
    assert!(matches!(
        decode_sdpose_heatmaps(&heatmaps, 1, &backend, &context),
        Err(SdPoseProjectionError::Tensor(_))
    ));
    assert_eq!(context.scratch.in_use_bytes(), 0);
    Ok(())
}

#[test]
fn sdpose_heatmap_decode_rejects_shape_nonfinite_and_workspace_exhaustion()
-> Result<(), Box<dyn Error>> {
    let (backend, authority) = CpuWorkspaceAuthority::create_backend(WORKSPACE_BYTES)?;
    let cancellation = CancellationToken::default();
    let context = ExecutionContext {
        stream: StreamId::DEFAULT,
        scratch: authority.authorize_workspace(WORKSPACE_BYTES)?,
        rng_phase: None,
        cancellation: &cancellation,
    };
    assert!(matches!(
        decode_sdpose_heatmaps(&[], 0, &backend, &context),
        Err(SdPoseProjectionError::InvalidHeatmapShape)
    ));

    let plane = SDPOSE_HEATMAP_HEIGHT * SDPOSE_HEATMAP_WIDTH;
    let mut heatmaps = vec![1.0; SDPOSE_HEATMAP_CHANNELS * plane];
    heatmaps[0] = f32::NAN;
    assert!(matches!(
        decode_sdpose_heatmaps(&heatmaps, 1, &backend, &context),
        Err(SdPoseProjectionError::NonFiniteInput)
    ));

    heatmaps[0] = 1.0;
    let (tiny_backend, tiny_authority) = CpuWorkspaceAuthority::create_backend(64)?;
    let tiny_context = ExecutionContext {
        stream: StreamId::DEFAULT,
        scratch: tiny_authority.authorize_workspace(64)?,
        rng_phase: None,
        cancellation: &cancellation,
    };
    assert!(matches!(
        decode_sdpose_heatmaps(&heatmaps, 1, &tiny_backend, &tiny_context),
        Err(SdPoseProjectionError::Tensor(_))
    ));
    assert_eq!(tiny_context.scratch.in_use_bytes(), 0);
    Ok(())
}
