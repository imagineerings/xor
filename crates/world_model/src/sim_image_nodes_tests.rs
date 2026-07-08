use crate::{
    SIM_IMAGE_INVALID_REGION_CODE, SIM_IMAGE_SHAPE_MISMATCH_CODE, SIM_MASK_SHAPE_MISMATCH_CODE,
    SimGlslDependency, SimImageArtifact, SimImageFlipAxis, SimImageFormat, SimImageNodeAdapter,
    SimImageRegion, SimImageShape, SimMaskArtifact, SimMaskNodeAdapter, SimMaskShape,
};

#[test]
fn image_adapter_preserves_batch_channels_and_metadata_for_transforms() {
    let adapter = SimImageNodeAdapter::new();
    let image = SimImageArtifact::new(SimImageShape::new(640, 480, 4).with_batch(2))
        .with_metadata("source", "fixture")
        .with_glsl_dependency("sim.image.blur", true);

    let resized = adapter.resize(&image, 320, 240).expect("resize");
    assert_eq!(resized.shape, SimImageShape::new(320, 240, 4).with_batch(2));
    assert_eq!(
        resized.metadata.get("source").map(String::as_str),
        Some("fixture")
    );
    assert_eq!(
        resized.glsl_dependencies,
        vec![SimGlslDependency {
            shader_id: "sim.image.blur".to_string(),
            reviewed: true,
        }]
    );

    let rotated = adapter.rotate_right(&resized);
    assert_eq!(rotated.shape, SimImageShape::new(240, 320, 4).with_batch(2));
}

#[test]
fn image_adapter_validates_crop_and_blend_shapes() {
    let adapter = SimImageNodeAdapter::new();
    let image = SimImageArtifact::new(SimImageShape::new(100, 80, 3));

    let cropped = adapter
        .crop(
            &image,
            SimImageRegion {
                x: 10,
                y: 10,
                width: 40,
                height: 30,
            },
        )
        .expect("crop");
    assert_eq!(cropped.shape, SimImageShape::new(40, 30, 3));

    let invalid = adapter
        .crop(
            &image,
            SimImageRegion {
                x: 90,
                y: 0,
                width: 20,
                height: 10,
            },
        )
        .expect_err("invalid region");
    assert_eq!(invalid.code, SIM_IMAGE_INVALID_REGION_CODE);

    let mismatch = adapter
        .blend(&image, &cropped, 500)
        .expect_err("shape mismatch");
    assert_eq!(mismatch.code, SIM_IMAGE_SHAPE_MISMATCH_CODE);
}

#[test]
fn image_adapter_records_flip_noise_and_save_format() {
    let adapter = SimImageNodeAdapter::new();
    let image = SimImageArtifact::new(SimImageShape::new(64, 64, 4));

    let flipped = adapter.flip(&image, SimImageFlipAxis::Horizontal);
    assert_eq!(
        flipped.metadata.get("sim.flip_axis").map(String::as_str),
        Some("Horizontal")
    );

    let noisy = adapter.add_noise_metadata(&flipped, 42, 1200);
    assert_eq!(
        noisy.metadata.get("sim.noise_seed").map(String::as_str),
        Some("42")
    );

    let saved = adapter.save_as(&noisy, SimImageFormat::Svg);
    assert_eq!(saved.format, Some(SimImageFormat::Svg));
}

#[test]
fn mask_adapter_converts_and_preserves_comfy_compatible_shapes() {
    let adapter = SimMaskNodeAdapter::new();
    let image = SimImageArtifact::new(SimImageShape::new(128, 96, 4).with_batch(3));

    let mask = adapter.image_to_mask(&image);
    assert_eq!(mask.shape, SimMaskShape::new(128, 96).with_batch(3));

    let image = adapter.mask_to_image(&mask);
    assert_eq!(image.shape, SimImageShape::new(128, 96, 4).with_batch(3));
}

#[test]
fn mask_adapter_tracks_deterministic_mask_operations_and_composite_validation() {
    let adapter = SimMaskNodeAdapter::new();
    let image = SimImageArtifact::new(SimImageShape::new(128, 96, 4));
    let mask = SimMaskArtifact::new(SimMaskShape::new(128, 96));

    let mask = adapter.threshold(&adapter.feather(&adapter.invert(&mask), 8), 1500);
    assert!(mask.inverted);
    assert_eq!(mask.feather_radius, 8);
    assert_eq!(mask.threshold_milli, Some(1000));

    let composited = adapter
        .composite_over_image(&image, &mask)
        .expect("matching composite");
    assert_eq!(
        composited
            .metadata
            .get("sim.mask_composited")
            .map(String::as_str),
        Some("true")
    );

    let mismatch = adapter
        .composite_over_image(&image, &SimMaskArtifact::new(SimMaskShape::new(32, 32)))
        .expect_err("mismatched mask");
    assert_eq!(mismatch.code, SIM_MASK_SHAPE_MISMATCH_CODE);
}
