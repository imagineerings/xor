use crate::{
    ComfyExecutionRegistry, LatentArtifact, LatentCompressionKind, LatentCompressionMetadata,
    LatentFormat, LatentMediaKind, ModelFamilyKind, SIM_AUDIO_DEPENDENCY_REVIEW_REQUIRED_CODE,
    SIM_AUDIO_INVALID_CHANNEL_CODE, SIM_AUDIO_INVALID_RANGE_CODE, SIM_AUDIO_LATENT_MISMATCH_CODE,
    SIM_AUDIO_SHAPE_MISMATCH_CODE, SIM_AUDIO_UNSUPPORTED_CODEC_CODE, SimAudioCodecStatus,
    SimAudioEqualizationBand, SimAudioMetadata, SimAudioNodeAdapter, SimAudioOperation,
    SimAudioSampleRange,
};

#[test]
fn audio_adapter_load_preview_record_empty_and_save_preserve_metadata() {
    let adapter = SimAudioNodeAdapter::new();
    let loaded = adapter.load(
        "input://voice.wav",
        SimAudioMetadata::new(48_000, 2, 96_000).with_field("speaker", "narrator"),
    );

    assert_eq!(loaded.metadata.sample_rate, 48_000);
    assert_eq!(loaded.metadata.channels, 2);
    assert_eq!(loaded.metadata.duration_samples, 96_000);
    assert_eq!(
        loaded.metadata.fields.get("speaker").map(String::as_str),
        Some("narrator")
    );
    assert_eq!(
        loaded
            .metadata
            .fields
            .get("sim.operation")
            .map(String::as_str),
        Some("load")
    );

    let previewed = adapter.preview(&loaded);
    assert_eq!(
        previewed
            .metadata
            .fields
            .get("sim.operation")
            .map(String::as_str),
        Some("preview")
    );

    let recorded = adapter.record("recorded://take.wav", 44_100, 1, 44_100);
    assert_eq!(recorded.metadata.channels, 1);

    let empty = adapter.empty("generated://silence.wav", 44_100, 2, 22_050);
    assert_eq!(empty.metadata.duration_samples, 22_050);

    let saved = adapter.save_as(&loaded, "output://voice.opus", "audio/opus");
    assert_eq!(saved.reference, "output://voice.opus");
    assert_eq!(saved.metadata.mime_type, "audio/opus");
}

#[test]
fn audio_adapter_trims_splits_joins_concatenates_and_mixes_shapes() {
    let adapter = SimAudioNodeAdapter::new();
    let source = adapter.empty("audio://source.wav", 48_000, 2, 96_000);

    let trimmed = adapter
        .trim(
            &source,
            SimAudioSampleRange {
                start: 12_000,
                end_exclusive: 36_000,
            },
        )
        .expect("trim");
    assert_eq!(trimmed.metadata.duration_samples, 24_000);
    assert_eq!(
        trimmed
            .metadata
            .fields
            .get("sim.sample_range")
            .map(String::as_str),
        Some("12000..36000")
    );

    let left = adapter.split_channel(&source, 0).expect("left channel");
    let right = adapter.split_channel(&source, 1).expect("right channel");
    assert_eq!(left.metadata.channels, 1);

    let joined = adapter
        .join_channels("audio://joined.wav", &[left, right])
        .expect("join channels");
    assert_eq!(joined.metadata.channels, 2);
    assert_eq!(
        joined.metadata.duration_samples,
        source.metadata.duration_samples
    );

    let concat = adapter
        .concatenate("audio://concat.wav", &[trimmed.clone(), trimmed.clone()])
        .expect("concat");
    assert_eq!(concat.metadata.duration_samples, 48_000);

    let mixed = adapter
        .mix("audio://mix.wav", &[trimmed, source])
        .expect("mix");
    assert_eq!(mixed.metadata.duration_samples, 96_000);
}

#[test]
fn audio_adapter_rejects_invalid_ranges_channels_and_shapes() {
    let adapter = SimAudioNodeAdapter::new();
    let source = adapter.empty("audio://source.wav", 48_000, 2, 96_000);

    let range_diagnostic = adapter
        .trim(
            &source,
            SimAudioSampleRange {
                start: 20,
                end_exclusive: 20,
            },
        )
        .expect_err("invalid range");
    assert_eq!(range_diagnostic.code, SIM_AUDIO_INVALID_RANGE_CODE);

    let channel_diagnostic = adapter
        .split_channel(&source, 2)
        .expect_err("invalid channel");
    assert_eq!(channel_diagnostic.code, SIM_AUDIO_INVALID_CHANNEL_CODE);

    let mono = adapter.split_channel(&source, 0).expect("mono");
    let mismatched = adapter.empty("audio://short.wav", 44_100, 1, 48_000);
    let shape_diagnostic = adapter
        .join_channels("audio://bad.wav", &[mono, mismatched])
        .expect_err("shape mismatch");
    assert_eq!(shape_diagnostic.code, SIM_AUDIO_SHAPE_MISMATCH_CODE);
}

#[test]
fn audio_adapter_tracks_volume_equalization_and_codec_diagnostics() {
    let adapter = SimAudioNodeAdapter::new();
    let source = adapter.empty("audio://source.wav", 48_000, 2, 96_000);

    let louder = adapter.adjust_volume(&source, 3.5);
    assert_eq!(
        louder
            .metadata
            .fields
            .get("sim.gain_db")
            .map(String::as_str),
        Some("3.5")
    );

    let equalized = adapter.equalize(
        &source,
        &[
            SimAudioEqualizationBand {
                center_hz: 120.0,
                gain_db: -2.0,
            },
            SimAudioEqualizationBand {
                center_hz: 4_000.0,
                gain_db: 1.5,
            },
        ],
    );
    assert_eq!(
        equalized
            .metadata
            .fields
            .get("sim.eq_bands")
            .map(String::as_str),
        Some("2")
    );

    let reviewed = adapter
        .codec_diagnostic(
            SimAudioOperation::Save,
            SimAudioCodecStatus::DependencyReviewRequired,
            "audio/opus",
        )
        .expect("review diagnostic");
    assert_eq!(reviewed.code, SIM_AUDIO_DEPENDENCY_REVIEW_REQUIRED_CODE);

    let unsupported = adapter
        .codec_diagnostic(
            SimAudioOperation::Save,
            SimAudioCodecStatus::Unsupported,
            "audio/x-custom",
        )
        .expect("unsupported diagnostic");
    assert_eq!(unsupported.code, SIM_AUDIO_UNSUPPORTED_CODEC_CODE);

    assert!(
        adapter
            .codec_diagnostic(
                SimAudioOperation::Save,
                SimAudioCodecStatus::Native,
                "audio/wav"
            )
            .is_none()
    );
}

#[test]
fn audio_adapter_validates_audio_latent_model_capabilities() {
    let adapter = SimAudioNodeAdapter::new();
    let registry = ComfyExecutionRegistry::new();
    let audio_family = registry
        .model_family(ModelFamilyKind::Audio)
        .expect("audio family");

    adapter
        .validate_audio_latent(&audio_latent(), audio_family)
        .expect("valid audio latent");

    let image_family = registry
        .model_family(ModelFamilyKind::StableDiffusionXl)
        .expect("image family");
    let diagnostics = adapter
        .validate_audio_latent(&audio_latent(), image_family)
        .expect_err("image family rejected");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == SIM_AUDIO_LATENT_MISMATCH_CODE)
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == crate::comfy_latents::LATENT_FORMAT_MISMATCH_CODE)
    );
}

fn audio_latent() -> LatentArtifact {
    LatentArtifact {
        id: "latent-audio".to_string(),
        format: LatentFormat::Audio,
        media: LatentMediaKind::Audio,
        width: 1,
        height: 1,
        channels: 64,
        frames: None,
        batch: 1,
        compression: LatentCompressionMetadata {
            kind: LatentCompressionKind::AudioCodec,
            scale_factor: 1.0,
            channels_last: true,
        },
        mask: None,
    }
}
