#!/usr/bin/env python3

import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import regenerate_native_planning as planning


class ValidationGenerationTests(unittest.TestCase):
    def test_schema_foundation_reopens_until_source_identity_evidence_is_fresh(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "tasks.md").write_text(
                "- [x] 368. Preserve exact native node schema and source metadata\n"
                "  - _id: comfy-parity-native-node-schema-metadata-foundation\n"
                "  - _validation_evidence: prior catalog evidence\n",
                encoding="utf-8",
            )
            with patch.object(planning, "ROOT", root):
                stale = planning.existing_task_annotations()[
                    "comfy-parity-native-node-schema-metadata-foundation"
                ]
            self.assertFalse(stale["complete"])
            self.assertIn("STALE AFTER NODE SOURCE IDENTITY REVALIDATION", stale["evidence"])

            (root / "tasks.md").write_text(
                "- [x] 368. Preserve exact native node schema and source metadata\n"
                "  - _id: comfy-parity-native-node-schema-metadata-foundation\n"
                "  - _validation_evidence: POST-NODE-SOURCE-IDENTITY-REVALIDATION fresh evidence\n",
                encoding="utf-8",
            )
            with patch.object(planning, "ROOT", root):
                fresh = planning.existing_task_annotations()[
                    "comfy-parity-native-node-schema-metadata-foundation"
                ]
            self.assertTrue(fresh["complete"])

    def test_native_node_runtime_foundation_orders_disjoint_leaves_and_registry(self) -> None:
        tasks, mapping = planning.all_tasks()
        tasks_by_id = {str(item["id"]): item for item in tasks}
        foundation_id = "comfy-parity-native-node-runtime-foundation"
        schema_id = "comfy-parity-native-node-schema-metadata-foundation"
        value_id = "comfy-parity-native-node-compute-value-foundation"
        asset_id = "comfy-parity-native-node-asset-effect-foundation"
        provider_id = "comfy-parity-native-node-provider-invocation-foundation"
        compute_id = "comfy-parity-native-compute-breadth-integration"
        registry_id = "comfy-parity-native-registry-integration"
        scalar_generation_id = "comfy-parity-native-decoder-text-generation-foundation"
        prepared_generation_id = "comfy-parity-native-prepared-decoder-generation-foundation"
        qwen_preparation_id = "comfy-parity-native-qwen-image-preparation-foundation"
        deepstack_id = "comfy-parity-native-prepared-decoder-deepstack-foundation"
        qwen_tokenizer_id = "comfy-parity-native-qwen2-tokenizer-foundation"
        qwen3_decoder_id = "comfy-parity-native-qwen3-decoder-exactness-foundation"
        qwen35_decoder_id = "comfy-parity-native-qwen35-decoder-exactness-foundation"
        qwen_vision_id = "comfy-parity-native-qwen-vision-projection-foundation"
        qwen_resource_id = "comfy-parity-native-qwen-multimodal-resource-foundation"
        qwen_generation_id = "comfy-parity-native-qwen-multimodal-generation-foundation"
        gemma_image_video_id = (
            "comfy-parity-native-gemma-image-video-preparation-foundation"
        )
        gemma_audio_preparation_id = (
            "comfy-parity-native-gemma-audio-preparation-foundation"
        )
        gemma_tokenizer_id = "comfy-parity-native-gemma-tokenizer-foundation"
        gemma3_decoder_id = "comfy-parity-native-gemma3-decoder-exactness-foundation"
        gemma4_decoder_id = "comfy-parity-native-gemma4-decoder-exactness-foundation"
        gemma3_vision_id = "comfy-parity-native-gemma3-vision-projection-foundation"
        gemma4_vision_id = "comfy-parity-native-gemma4-vision-projection-foundation"
        gemma4_audio_id = "comfy-parity-native-gemma4-audio-execution-foundation"
        gemma_resource_id = "comfy-parity-native-gemma-multimodal-resource-foundation"
        gemma_generation_id = "comfy-parity-native-gemma-multimodal-generation-foundation"
        multimodal_generation_id = "comfy-parity-native-text-generation-foundation"
        node_ids = sorted(
            identifier
            for identifier in tasks_by_id
            if identifier.startswith("comfy-parity-native-nodes-")
        )

        self.assertEqual(len(tasks), 600)
        self.assertEqual(len(node_ids), 102)
        self.assertEqual(tasks_by_id[foundation_id]["dependencies"], [compute_id])
        for identifier in (schema_id, value_id, asset_id, provider_id):
            self.assertTrue(tasks_by_id[identifier]["feature_scoped"])
        self.assertEqual(tasks_by_id[schema_id]["dependencies"], [foundation_id])
        self.assertEqual(
            tasks_by_id[value_id]["dependencies"],
            [
                schema_id,
                compute_id,
                "comfy-parity-model-detection-any-of-key-selector-consolidation",
            ],
        )
        self.assertEqual(
            tasks_by_id[asset_id]["dependencies"],
            [
                value_id,
                "comfy-parity-artifact-owner-consolidation",
                "comfy-parity-execution-output-owner-consolidation",
            ],
        )
        self.assertEqual(
            tasks_by_id[provider_id]["dependencies"],
            [
                asset_id,
                "comfy-parity-extension-host-plugin-adapter",
                "comfy-parity-opt-in-product-build-boundary",
                "comfy-parity-native-shader-execution-foundation",
            ],
        )
        self.assertEqual(
            tasks_by_id[prepared_generation_id]["dependencies"],
            [scalar_generation_id],
        )
        self.assertEqual(
            tasks_by_id[qwen_preparation_id]["dependencies"],
            [prepared_generation_id],
        )
        self.assertEqual(
            tasks_by_id[deepstack_id]["dependencies"],
            [qwen_preparation_id],
        )
        self.assertEqual(tasks_by_id[qwen_tokenizer_id]["dependencies"], [deepstack_id])
        self.assertEqual(tasks_by_id[qwen3_decoder_id]["dependencies"], [qwen_tokenizer_id])
        self.assertEqual(tasks_by_id[qwen35_decoder_id]["dependencies"], [qwen3_decoder_id])
        self.assertEqual(tasks_by_id[qwen_vision_id]["dependencies"], [qwen35_decoder_id])
        self.assertEqual(tasks_by_id[qwen_resource_id]["dependencies"], [qwen_vision_id])
        self.assertEqual(tasks_by_id[qwen_generation_id]["dependencies"], [qwen_resource_id])
        self.assertEqual(
            tasks_by_id[gemma_image_video_id]["dependencies"],
            [qwen_generation_id],
        )
        self.assertEqual(
            tasks_by_id[gemma_audio_preparation_id]["dependencies"],
            [gemma_image_video_id],
        )
        self.assertIn(
            "crates/comfy_tensor/src/ops/spectral_transform_01.rs",
            tasks_by_id[gemma_audio_preparation_id]["reads"],
        )
        self.assertIn(
            "crates/comfy_test_support/fixtures/text_generation/gemma_multimodal/audio_preparation",
            tasks_by_id[gemma_audio_preparation_id]["writes"],
        )
        self.assertEqual(
            tasks_by_id[gemma_tokenizer_id]["dependencies"],
            [gemma_audio_preparation_id],
        )
        self.assertIn(
            "crates/comfy_model/src/clip_tokenizer.rs",
            tasks_by_id[gemma_tokenizer_id]["writes"],
        )
        self.assertIn(
            "crates/comfy_test_support/fixtures/text_generation/gemma_multimodal/tokenizer",
            tasks_by_id[gemma_tokenizer_id]["writes"],
        )
        self.assertEqual(
            tasks_by_id[gemma3_decoder_id]["dependencies"],
            [gemma_tokenizer_id],
        )
        self.assertIn(
            "crates/comfy_model/src/clip_text_encoder_decoder.rs",
            tasks_by_id[gemma3_decoder_id]["writes"],
        )
        self.assertIn(
            "crates/comfy_model/src/clip_text_encoder_multimodal.rs",
            tasks_by_id[gemma3_decoder_id]["writes"],
        )
        self.assertIn(
            "crates/comfy_model/tests/clip_text_encoder_multimodal.rs",
            tasks_by_id[gemma3_decoder_id]["writes"],
        )
        self.assertIn(
            "crates/comfy_test_support/fixtures/text_generation/gemma_multimodal/gemma3_decoder",
            tasks_by_id[gemma3_decoder_id]["writes"],
        )
        self.assertEqual(
            tasks_by_id[gemma4_decoder_id]["dependencies"],
            [gemma3_decoder_id],
        )
        self.assertIn(
            "projects/comfy/ComfyUI/comfy/text_encoders/gemma4.py",
            tasks_by_id[gemma4_decoder_id]["reads"],
        )
        self.assertIn(
            "crates/comfy_model/src/clip_text_encoder_multimodal.rs",
            tasks_by_id[gemma4_decoder_id]["writes"],
        )
        self.assertIn(
            "crates/comfy_model/tests/clip_text_encoder_multimodal.rs",
            tasks_by_id[gemma4_decoder_id]["writes"],
        )
        self.assertIn(
            "crates/comfy_test_support/fixtures/text_generation/gemma_multimodal/gemma4_decoder",
            tasks_by_id[gemma4_decoder_id]["writes"],
        )
        self.assertEqual(
            tasks_by_id[gemma4_decoder_id]["validations"],
            [
                "VAL-CLIP-001",
                "VAL-RNG-001",
                "VAL-CANCEL-001",
                "VAL-MEMORY-001",
                "VAL-OWNERSHIP-001",
            ],
        )
        self.assertEqual(
            tasks_by_id[gemma3_vision_id]["dependencies"],
            [gemma4_decoder_id],
        )
        self.assertEqual(
            tasks_by_id[gemma4_vision_id]["dependencies"],
            [gemma3_vision_id],
        )
        self.assertEqual(
            tasks_by_id[gemma4_vision_id]["validations"],
            [
                "VAL-CLIP-001",
                "VAL-TENSOR-001",
                "VAL-MODEL-FAMILY-001",
                "VAL-CANCEL-001",
                "VAL-MEMORY-001",
                "VAL-OWNERSHIP-001",
            ],
        )
        self.assertIn(
            "projects/comfy/ComfyUI/comfy/text_encoders/llama.py",
            tasks_by_id[gemma4_vision_id]["reads"],
        )
        self.assertIn(
            "crates/comfy_model/src/attention.rs",
            tasks_by_id[gemma4_vision_id]["reads"],
        )
        self.assertIn(
            "crates/comfy_test_support/fixtures/text_generation/gemma_multimodal/gemma4_vision",
            tasks_by_id[gemma4_vision_id]["writes"],
        )
        self.assertEqual(
            tasks_by_id[gemma4_audio_id]["dependencies"],
            [gemma4_vision_id],
        )
        self.assertIn(
            "crates/comfy_test_support/fixtures/text_generation/gemma_multimodal/audio_preparation/manifest.json",
            tasks_by_id[gemma4_audio_id]["reads"],
        )
        self.assertIn(
            "crates/comfy_test_support/fixtures/text_generation/gemma_multimodal/gemma4_audio",
            tasks_by_id[gemma4_audio_id]["writes"],
        )
        self.assertEqual(
            tasks_by_id[gemma4_audio_id]["validations"],
            [
                "VAL-CLIP-001",
                "VAL-TENSOR-001",
                "VAL-MODEL-FAMILY-001",
                "VAL-CANCEL-001",
                "VAL-MEMORY-001",
                "VAL-OWNERSHIP-001",
            ],
        )
        self.assertEqual(
            tasks_by_id[gemma_resource_id]["dependencies"],
            [gemma4_audio_id],
        )
        self.assertIn(
            "projects/comfy/ComfyUI/comfy/ldm/lumina/model/lumina2.py",
            tasks_by_id[gemma_resource_id]["reads"],
        )
        self.assertIn(
            "projects/comfy/ComfyUI/comfy/sd1_clip.py",
            tasks_by_id[gemma_resource_id]["reads"],
        )
        self.assertIn(
            "projects/comfy/ComfyUI/comfy/text_encoders/lt.py",
            tasks_by_id[gemma_resource_id]["reads"],
        )
        self.assertIn(
            "crates/comfy_test_support/fixtures/text_generation/gemma_multimodal/gemma4_audio/manifest.json",
            tasks_by_id[gemma_resource_id]["reads"],
        )
        self.assertIn(
            "projects/comfy/ComfyUI/comfy/text_encoders/spiece_tokenizer.py",
            tasks_by_id[gemma_resource_id]["reads"],
        )
        self.assertIn(
            "crates/comfy_nodes/src/stored_payload.rs",
            tasks_by_id[gemma_resource_id]["writes"],
        )
        self.assertEqual(
            tasks_by_id[gemma_resource_id]["validations"],
            [
                "VAL-CLIP-001",
                "VAL-MODEL-FAMILY-001",
                "VAL-CANCEL-001",
                "VAL-MEMORY-001",
                "VAL-OWNERSHIP-001",
            ],
        )
        self.assertEqual(
            tasks_by_id[gemma_generation_id]["dependencies"],
            [gemma_resource_id],
        )
        self.assertIn(
            "crates/comfy_model/src/clip_text_encoder_decoder.rs",
            tasks_by_id[gemma_generation_id]["writes"],
        )
        self.assertIn(
            "crates/comfy_test_support/fixtures/text_generation/gemma_multimodal/generation",
            tasks_by_id[gemma_generation_id]["writes"],
        )
        self.assertEqual(
            tasks_by_id[gemma_generation_id]["validations"],
            [
                "VAL-MODEL-FAMILY-001",
                "VAL-RNG-001",
                "VAL-CANCEL-001",
                "VAL-MEMORY-001",
                "VAL-OWNERSHIP-001",
            ],
        )
        self.assertEqual(
            tasks_by_id[multimodal_generation_id]["dependencies"],
            [gemma_generation_id],
        )
        self.assertIn(
            "crates/comfy_runtime/src/native_execution_controller.rs",
            tasks_by_id[multimodal_generation_id]["writes"],
        )
        self.assertIn(
            "crates/comfy_test_support/fixtures/text_generation/multimodal",
            tasks_by_id[multimodal_generation_id]["writes"],
        )
        self.assertIn(
            "crates/comfy_model/tests/clip_tokenizer.rs",
            tasks_by_id[multimodal_generation_id]["writes"],
        )
        self.assertEqual(
            tasks_by_id[multimodal_generation_id]["validations"],
            [
                "VAL-MODEL-FAMILY-001",
                "VAL-RNG-001",
                "VAL-NATIVE-E2E-001",
                "VAL-CANCEL-001",
                "VAL-MEMORY-001",
                "VAL-OWNERSHIP-001",
            ],
        )
        dependencies = {
            identifier: set(tasks_by_id[identifier]["dependencies"])
            for identifier in node_ids
        }
        self.assertEqual(sum(schema_id in value for value in dependencies.values()), 102)
        self.assertEqual(sum(value_id in value for value in dependencies.values()), 84)
        self.assertEqual(sum(asset_id in value for value in dependencies.values()), 84)
        self.assertEqual(sum(provider_id in value for value in dependencies.values()), 26)
        self.assertEqual(
            sum(
                value_id in value and provider_id in value
                for value in dependencies.values()
            ),
            8,
        )
        self.assertEqual(
            sum(
                value_id in value and provider_id not in value
                for value in dependencies.values()
            ),
            76,
        )
        self.assertEqual(
            sum(
                provider_id in value and value_id not in value
                for value in dependencies.values()
            ),
            18,
        )
        mapped_values = {
            identifier: sum(
                identifier in task_ids for task_ids in mapping.values()
            )
            for identifier in (schema_id, value_id, asset_id, provider_id)
        }
        self.assertEqual(
            mapped_values,
            {schema_id: 789, value_id: 575, asset_id: 189, provider_id: 214},
        )
        self.assertEqual(sorted(tasks_by_id[registry_id]["dependencies"]), node_ids)

        three_d_id = "comfy-parity-native-nodes-three-d-comfy-node-0115"
        splat_id = "comfy-parity-native-nodes-three-d-splat-comfy-node-0172"
        modifier_id = "comfy-parity-native-nodes-advanced-debug-comfy-node-0140"
        guidance_id = "comfy-parity-native-nodes-advanced-guidance-comfy-node-0049"
        hooks_id = "comfy-parity-native-nodes-advanced-hooks-comfy-node-0079"
        hook_consumers_id = "comfy-parity-native-nodes-advanced-hooks-comfy-node-0119"
        text_regex_id = "comfy-parity-native-nodes-text-comfy-node-0002"
        text_regex_foundation_id = "comfy-parity-native-text-value-regex-foundation"
        text_transform_foundation_id = "comfy-parity-native-text-transform-foundation"
        decoder_text_generation_foundation_id = (
            "comfy-parity-native-decoder-text-generation-foundation"
        )
        text_generation_foundation_id = "comfy-parity-native-text-generation-foundation"
        media_text_foundation_id = "comfy-parity-native-media-text-rendering-foundation"
        sdpose_foundation_id = "comfy-parity-native-sdpose-execution-foundation"
        sdpose_projection_id = (
            "comfy-parity-native-sdpose-heatmap-projection-foundation"
        )
        immutable_dense_attention_id = (
            "comfy-parity-native-immutable-dense-inference-attention-foundation"
        )
        sdpose_capture_id = "comfy-parity-native-sdpose-sd2-capture-foundation"
        sdpose_resource_id = "comfy-parity-native-sdpose-model-resource-foundation"
        bounded_dense_spatial_id = (
            "comfy-parity-native-bounded-dense-spatial-inference-foundation"
        )
        lotusd_sampling_id = "comfy-parity-native-lotusd-sampling-foundation"
        sdpose_head_projection_id = (
            "comfy-parity-native-sdpose-head-projection-foundation"
        )
        video_foundation_id = "comfy-parity-native-video-execution-foundation"
        video_codec_plan_foundation_id = (
            "comfy-parity-native-video-codec-plan-foundation"
        )
        video_codec_ffi_certification_foundation_id = (
            "comfy-parity-native-video-codec-ffi-certification-foundation"
        )
        video_codec_package_capture_foundation_id = (
            "comfy-parity-native-video-codec-package-capture-foundation"
        )
        video_codec_elf_inspection_foundation_id = (
            "comfy-parity-native-video-codec-elf-inspection-foundation"
        )
        video_codec_inspected_certification_foundation_id = (
            "comfy-parity-native-video-codec-inspected-certification-foundation"
        )
        video_codec_dependency_contract_foundation_id = (
            "comfy-parity-native-video-codec-dependency-contract-foundation"
        )
        video_codec_dependency_closure_foundation_id = (
            "comfy-parity-native-video-codec-dependency-closure-certification-foundation"
        )
        video_codec_retained_loader_foundation_id = (
            "comfy-parity-native-video-codec-retained-loader-foundation"
        )
        video_codec_reviewed_abi_foundation_id = (
            "comfy-parity-native-video-codec-reviewed-abi-foundation"
        )
        video_codec_callable_symbol_certification_foundation_id = (
            "comfy-parity-native-video-codec-callable-symbol-certification-foundation"
        )
        video_codec_symbol_binding_foundation_id = (
            "comfy-parity-native-video-codec-symbol-binding-foundation"
        )
        video_codec_data_plane_abi_foundation_id = (
            "comfy-parity-native-video-codec-data-plane-abi-foundation"
        )
        video_codec_bounded_memory_avio_foundation_id = (
            "comfy-parity-native-video-codec-bounded-memory-avio-foundation"
        )
        video_codec_ltxv_h264_admission_foundation_id = (
            "comfy-parity-native-video-codec-ltxv-h264-admission-foundation"
        )
        video_codec_ltxv_h264_mp4_encode_foundation_id = (
            "comfy-parity-native-video-codec-ltxv-h264-mp4-encode-foundation"
        )
        video_codec_ltxv_h264_mp4_demux_foundation_id = (
            "comfy-parity-native-video-codec-ltxv-h264-mp4-demux-foundation"
        )
        video_output_prefix_foundation_id = "comfy-parity-native-video-output-prefix-foundation"
        video_component_foundation_id = "comfy-parity-native-video-component-foundation"
        video_output_media_foundation_id = "comfy-parity-native-video-output-media-foundation"
        video_output_projection_foundation_id = "comfy-parity-native-video-output-projection-foundation"
        frame_interpolation_model_foundation_id = "comfy-parity-native-frame-interpolation-model-foundation"
        frame_interpolation_resource_foundation_id = "comfy-parity-native-frame-interpolation-resource-foundation"
        frame_interpolation_invocation_foundation_id = "comfy-parity-native-frame-interpolation-invocation-foundation"
        tensor_grid_sample_foundation_id = "comfy-parity-native-tensor-grid-sample-foundation"
        tensor_interpolate_foundation_id = "comfy-parity-native-tensor-interpolate-foundation"
        rife_tensor_arithmetic_foundation_id = (
            "comfy-parity-native-rife-tensor-arithmetic-foundation"
        )
        rife_execution_foundation_id = "comfy-parity-native-rife-execution-foundation"
        rife_sequence_execution_foundation_id = (
            "comfy-parity-native-rife-sequence-execution-foundation"
        )
        film_tensor_average_pool_foundation_id = (
            "comfy-parity-native-film-tensor-average-pool-foundation"
        )
        film_warp_foundation_id = "comfy-parity-native-film-warp-foundation"
        film_padded_convolution_foundation_id = (
            "comfy-parity-native-film-padded-convolution-foundation"
        )
        film_image_pyramid_foundation_id = (
            "comfy-parity-native-film-image-pyramid-foundation"
        )
        film_subtree_foundation_id = "comfy-parity-native-film-subtree-foundation"
        film_feature_pyramid_foundation_id = (
            "comfy-parity-native-film-feature-pyramid-foundation"
        )
        film_flow_pyramid_synthesis_foundation_id = (
            "comfy-parity-native-film-flow-pyramid-synthesis-foundation"
        )
        film_flow_estimator_foundation_id = (
            "comfy-parity-native-film-flow-estimator-foundation"
        )
        film_pyramid_algebra_foundation_id = (
            "comfy-parity-native-film-pyramid-algebra-foundation"
        )
        film_fusion_foundation_id = "comfy-parity-native-film-fusion-foundation"
        film_multi_timestep_foundation_id = (
            "comfy-parity-native-film-multi-timestep-synthesis-foundation"
        )
        film_sequence_foundation_id = (
            "comfy-parity-native-film-sequence-execution-foundation"
        )
        frame_interpolation_resource_exhaustion_foundation_id = (
            "comfy-parity-native-frame-interpolation-resource-exhaustion-foundation"
        )
        image_source_foundation_id = "comfy-parity-native-image-source-compatibility-foundation"
        structured_link_foundation_id = "comfy-parity-native-structured-input-link-foundation"
        shader_foundation_id = "comfy-parity-native-shader-execution-foundation"
        detection_foundation_id = "comfy-parity-native-detection-execution-foundation"
        sdpose_id = "comfy-parity-native-nodes-image-detection-comfy-node-0607"
        detection_id = "comfy-parity-native-nodes-image-detection-comfy-node-0136"
        image_filter_id = "comfy-parity-native-nodes-image-filters-comfy-node-0045"
        image_transform_id = "comfy-parity-native-nodes-image-transform-comfy-node-0047"
        structured_transform_id = "comfy-parity-native-nodes-image-transform-comfy-node-0541"
        shader_id = "comfy-parity-native-nodes-image-shader-comfy-node-0211"
        text_transform_id = "comfy-parity-native-nodes-text-comfy-node-0531"
        text_generation_id = "comfy-parity-native-nodes-text-comfy-node-0649"
        media_text_id = "comfy-parity-native-nodes-utilities-comfy-node-0077"
        primitive_id = "comfy-parity-native-nodes-utilities-primitive-comfy-node-0494"
        video_id = "comfy-parity-native-nodes-video-comfy-node-0124"
        video_preprocessor_id = "comfy-parity-native-nodes-video-preprocessors-comfy-node-0372"
        self.assertIn(provider_id, dependencies[three_d_id])
        self.assertIn(three_d_id, dependencies[splat_id])
        self.assertIn(splat_id, dependencies[modifier_id])
        self.assertIn(modifier_id, dependencies[guidance_id])
        self.assertIn(guidance_id, dependencies[hooks_id])
        self.assertIn(hooks_id, dependencies[hook_consumers_id])
        self.assertIn("crates/comfy_media/src/three_d.rs", tasks_by_id[three_d_id]["writes"])
        self.assertIn(
            "crates/comfy_media/src/gaussian_splat_compute.rs",
            tasks_by_id[splat_id]["writes"],
        )
        self.assertIn(
            "crates/comfy_sampler/src/model_execution_modifiers.rs",
            tasks_by_id[modifier_id]["writes"],
        )
        self.assertIn(
            "crates/comfy_sampler/src/guidance.rs",
            tasks_by_id[guidance_id]["writes"],
        )
        self.assertIn(
            "crates/comfy_model/src/hooks.rs",
            tasks_by_id[hooks_id]["writes"],
        )
        self.assertIn(
            "crates/comfy_runtime/src/assets.rs",
            tasks_by_id[hook_consumers_id]["writes"],
        )
        self.assertIn(
            "crates/comfy_model/src/clip.rs",
            tasks_by_id[hook_consumers_id]["writes"],
        )
        self.assertIn(
            video_output_prefix_foundation_id,
            tasks_by_id[video_component_foundation_id]["dependencies"],
        )
        self.assertIn(
            video_component_foundation_id,
            tasks_by_id[video_output_media_foundation_id]["dependencies"],
        )
        self.assertIn(
            video_output_media_foundation_id,
            tasks_by_id[video_output_projection_foundation_id]["dependencies"],
        )
        self.assertIn(
            video_output_projection_foundation_id,
            tasks_by_id[frame_interpolation_model_foundation_id]["dependencies"],
        )
        self.assertIn(
            frame_interpolation_model_foundation_id,
            tasks_by_id[frame_interpolation_resource_foundation_id]["dependencies"],
        )
        self.assertIn(
            frame_interpolation_resource_foundation_id,
            tasks_by_id[frame_interpolation_invocation_foundation_id]["dependencies"],
        )
        self.assertIn(
            frame_interpolation_invocation_foundation_id,
            tasks_by_id[tensor_grid_sample_foundation_id]["dependencies"],
        )
        self.assertIn(
            tensor_grid_sample_foundation_id,
            tasks_by_id[tensor_interpolate_foundation_id]["dependencies"],
        )
        self.assertIn(
            tensor_interpolate_foundation_id,
            tasks_by_id[rife_tensor_arithmetic_foundation_id]["dependencies"],
        )
        self.assertIn(
            rife_tensor_arithmetic_foundation_id,
            tasks_by_id[rife_execution_foundation_id]["dependencies"],
        )
        self.assertIn(
            rife_sequence_execution_foundation_id,
            tasks_by_id[film_tensor_average_pool_foundation_id]["dependencies"],
        )
        self.assertIn(
            film_tensor_average_pool_foundation_id,
            tasks_by_id[film_warp_foundation_id]["dependencies"],
        )
        self.assertIn(
            film_warp_foundation_id,
            tasks_by_id[film_padded_convolution_foundation_id]["dependencies"],
        )
        self.assertIn(
            film_padded_convolution_foundation_id,
            tasks_by_id[film_image_pyramid_foundation_id]["dependencies"],
        )
        self.assertIn(
            film_image_pyramid_foundation_id,
            tasks_by_id[film_subtree_foundation_id]["dependencies"],
        )
        self.assertIn(
            film_subtree_foundation_id,
            tasks_by_id[film_feature_pyramid_foundation_id]["dependencies"],
        )
        self.assertIn(
            film_feature_pyramid_foundation_id,
            tasks_by_id[film_flow_pyramid_synthesis_foundation_id]["dependencies"],
        )
        self.assertIn(
            film_flow_pyramid_synthesis_foundation_id,
            tasks_by_id[film_flow_estimator_foundation_id]["dependencies"],
        )
        self.assertIn(
            film_flow_estimator_foundation_id,
            tasks_by_id[film_pyramid_algebra_foundation_id]["dependencies"],
        )
        self.assertIn(
            film_pyramid_algebra_foundation_id,
            tasks_by_id[film_fusion_foundation_id]["dependencies"],
        )
        self.assertIn(
            film_fusion_foundation_id,
            tasks_by_id[film_multi_timestep_foundation_id]["dependencies"],
        )
        self.assertIn(
            film_multi_timestep_foundation_id,
            tasks_by_id[film_sequence_foundation_id]["dependencies"],
        )
        self.assertIn(
            film_sequence_foundation_id,
            tasks_by_id[frame_interpolation_resource_exhaustion_foundation_id][
                "dependencies"
            ],
        )
        self.assertIn(
            frame_interpolation_resource_exhaustion_foundation_id,
            tasks_by_id[video_codec_plan_foundation_id]["dependencies"],
        )
        self.assertIn(
            video_codec_plan_foundation_id,
            tasks_by_id[video_codec_ffi_certification_foundation_id]["dependencies"],
        )
        self.assertIn(
            video_codec_ffi_certification_foundation_id,
            tasks_by_id[video_codec_package_capture_foundation_id]["dependencies"],
        )
        self.assertIn(
            video_codec_package_capture_foundation_id,
            tasks_by_id[video_codec_elf_inspection_foundation_id]["dependencies"],
        )
        self.assertIn(
            video_codec_elf_inspection_foundation_id,
            tasks_by_id[video_codec_inspected_certification_foundation_id]["dependencies"],
        )
        self.assertIn(
            video_codec_inspected_certification_foundation_id,
            tasks_by_id[video_codec_dependency_contract_foundation_id]["dependencies"],
        )
        self.assertIn(
            video_codec_dependency_contract_foundation_id,
            tasks_by_id[video_codec_dependency_closure_foundation_id]["dependencies"],
        )
        self.assertIn(
            video_codec_dependency_closure_foundation_id,
            tasks_by_id[video_codec_retained_loader_foundation_id]["dependencies"],
        )
        self.assertIn(
            video_codec_retained_loader_foundation_id,
            tasks_by_id[video_codec_reviewed_abi_foundation_id]["dependencies"],
        )
        self.assertIn(
            video_codec_reviewed_abi_foundation_id,
            tasks_by_id[video_codec_callable_symbol_certification_foundation_id][
                "dependencies"
            ],
        )
        self.assertIn(
            video_codec_callable_symbol_certification_foundation_id,
            tasks_by_id[video_codec_symbol_binding_foundation_id]["dependencies"],
        )
        self.assertIn(
            video_codec_symbol_binding_foundation_id,
            tasks_by_id[video_codec_data_plane_abi_foundation_id]["dependencies"],
        )
        self.assertIn(
            video_codec_ltxv_h264_mp4_demux_foundation_id,
            tasks_by_id[video_foundation_id]["dependencies"],
        )
        codec_certification = tasks_by_id[video_codec_ffi_certification_foundation_id]
        self.assertIn("crates/comfy_runtime/src/trust.rs", codec_certification["writes"])
        self.assertIn("VAL-RUNTIME-TRUST-001", codec_certification["validations"])
        self.assertNotIn("Cargo.lock", codec_certification["writes"])
        codec_capture = tasks_by_id[video_codec_package_capture_foundation_id]
        self.assertEqual(
            codec_capture["writes"],
            [
                "crates/comfy_runtime/src/trust.rs",
                "crates/comfy_test_support/fixtures/video/codec-package-capture/manifest.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertNotIn("Cargo.lock", codec_capture["writes"])
        codec_elf_inspection = tasks_by_id[video_codec_elf_inspection_foundation_id]
        self.assertIn(
            "crates/comfy_runtime/src/native_ffi_elf.rs",
            codec_elf_inspection["writes"],
        )
        self.assertIn("VAL-NATIVE-BOUNDARY-001", codec_elf_inspection["validations"])
        self.assertNotIn("Cargo.lock", codec_elf_inspection["writes"])
        codec_inspected_certification = tasks_by_id[
            video_codec_inspected_certification_foundation_id
        ]
        self.assertIn(
            "crates/comfy_test_support/fixtures/video/codec-inspected-certification/manifest.json",
            codec_inspected_certification["writes"],
        )
        self.assertIn(
            "VAL-RUNTIME-TRUST-001",
            codec_inspected_certification["validations"],
        )
        self.assertNotIn("Cargo.lock", codec_inspected_certification["writes"])
        codec_dependency_contract = tasks_by_id[
            video_codec_dependency_contract_foundation_id
        ]
        self.assertEqual(
            codec_dependency_contract["writes"],
            [
                "crates/comfy_runtime/src/trust.rs",
                "crates/comfy_test_support/fixtures/video/codec-dependency-contract/manifest.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            codec_dependency_contract["validations"],
            [
                "VAL-RUNTIME-TRUST-001",
                "VAL-NATIVE-BOUNDARY-001",
                "VAL-CANCEL-001",
                "VAL-OWNERSHIP-001",
            ],
        )
        self.assertNotIn("Cargo.toml", codec_dependency_contract["writes"])
        self.assertNotIn("Cargo.lock", codec_dependency_contract["writes"])
        codec_dependency_closure = tasks_by_id[
            video_codec_dependency_closure_foundation_id
        ]
        self.assertEqual(
            codec_dependency_closure["writes"],
            [
                "crates/comfy_runtime/src/trust.rs",
                "crates/comfy_runtime/src/native_ffi_elf.rs",
                "crates/comfy_test_support/fixtures/video/codec-inspected-certification/manifest.json",
                "crates/comfy_test_support/fixtures/video/codec-dependency-closure/manifest.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            codec_dependency_closure["validations"],
            [
                "VAL-RUNTIME-TRUST-001",
                "VAL-NATIVE-BOUNDARY-001",
                "VAL-CANCEL-001",
                "VAL-OWNERSHIP-001",
            ],
        )
        self.assertNotIn("Cargo.toml", codec_dependency_closure["writes"])
        self.assertNotIn("Cargo.lock", codec_dependency_closure["writes"])
        codec_retained_loader = tasks_by_id[
            video_codec_retained_loader_foundation_id
        ]
        self.assertEqual(
            codec_retained_loader["writes"],
            [
                "crates/comfy_runtime/src/trust.rs",
                "crates/comfy_runtime/src/native_video_codec_ffi.rs",
                "crates/comfy_runtime/src/comfy_runtime.rs",
                "crates/comfy_test_support/fixtures/video/codec-retained-loader/manifest.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            codec_retained_loader["validations"],
            [
                "VAL-RUNTIME-TRUST-001",
                "VAL-NATIVE-BOUNDARY-001",
                "VAL-CANCEL-001",
                "VAL-OWNERSHIP-001",
            ],
        )
        self.assertNotIn("Cargo.toml", codec_retained_loader["writes"])
        self.assertNotIn("Cargo.lock", codec_retained_loader["writes"])
        codec_reviewed_abi = tasks_by_id[video_codec_reviewed_abi_foundation_id]
        self.assertEqual(
            codec_reviewed_abi["writes"],
            [
                "crates/comfy_runtime/src/native_video_codec_abi.rs",
                "crates/comfy_runtime/src/trust.rs",
                "crates/comfy_runtime/src/comfy_runtime.rs",
                "crates/comfy_runtime/abi/video-codec/ffmpeg-7.1-x86_64-gnu-v1.json",
                "crates/comfy_runtime/abi/video-codec/verify-bindings.c",
                "crates/comfy_test_support/fixtures/video/codec-reviewed-abi/manifest.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            codec_reviewed_abi["validations"],
            [
                "VAL-RUNTIME-TRUST-001",
                "VAL-NATIVE-BOUNDARY-001",
                "VAL-OWNERSHIP-001",
            ],
        )
        self.assertNotIn("Cargo.toml", codec_reviewed_abi["writes"])
        self.assertNotIn("Cargo.lock", codec_reviewed_abi["writes"])
        codec_callable_symbols = tasks_by_id[
            video_codec_callable_symbol_certification_foundation_id
        ]
        self.assertEqual(
            codec_callable_symbols["writes"],
            [
                "crates/comfy_runtime/src/native_ffi_elf.rs",
                "crates/comfy_runtime/src/trust.rs",
                "crates/comfy_runtime/src/native_video_codec_abi.rs",
                "crates/comfy_runtime/abi/video-codec/ffmpeg-7.1-x86_64-gnu-v1.json",
                "crates/comfy_test_support/fixtures/video/codec-callable-symbol-certification/manifest.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            codec_callable_symbols["validations"],
            [
                "VAL-RUNTIME-TRUST-001",
                "VAL-NATIVE-BOUNDARY-001",
                "VAL-CANCEL-001",
                "VAL-OWNERSHIP-001",
            ],
        )
        codec_symbol_binding = tasks_by_id[video_codec_symbol_binding_foundation_id]
        self.assertEqual(
            codec_symbol_binding["dependencies"],
            [video_codec_callable_symbol_certification_foundation_id],
        )
        self.assertEqual(
            codec_symbol_binding["writes"],
            [
                "crates/comfy_runtime/src/native_video_codec_ffi.rs",
                "crates/comfy_runtime/src/native_video_codec_abi.rs",
                "crates/comfy_runtime/src/trust.rs",
                "crates/comfy_test_support/fixtures/video/codec-symbol-binding/manifest.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            codec_symbol_binding["validations"],
            [
                "VAL-RUNTIME-TRUST-001",
                "VAL-NATIVE-BOUNDARY-001",
                "VAL-CANCEL-001",
                "VAL-OWNERSHIP-001",
            ],
        )
        self.assertEqual(
            tasks_by_id[video_codec_data_plane_abi_foundation_id]["dependencies"],
            [video_codec_symbol_binding_foundation_id],
        )
        codec_data_plane_abi = tasks_by_id[video_codec_data_plane_abi_foundation_id]
        self.assertEqual(
            codec_data_plane_abi["writes"],
            [
                "crates/comfy_runtime/src/native_video_codec_abi.rs",
                "crates/comfy_runtime/abi/video-codec/ffmpeg-7.1-x86_64-gnu-data-plane-v1.json",
                "crates/comfy_runtime/abi/video-codec/verify-data-plane-bindings.c",
                "crates/comfy_test_support/fixtures/video/codec-data-plane-abi/manifest.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            codec_data_plane_abi["validations"],
            [
                "VAL-RUNTIME-TRUST-001",
                "VAL-NATIVE-BOUNDARY-001",
                "VAL-OWNERSHIP-001",
            ],
        )
        codec_bounded_avio = tasks_by_id[video_codec_bounded_memory_avio_foundation_id]
        self.assertEqual(
            codec_bounded_avio["dependencies"],
            [video_codec_data_plane_abi_foundation_id],
        )
        self.assertEqual(
            codec_bounded_avio["writes"],
            [
                "crates/comfy_runtime/src/native_video_codec_ffi.rs",
                "crates/comfy_runtime/src/native_video_codec_abi.rs",
                "crates/comfy_runtime/abi/video-codec/ffmpeg-7.1-x86_64-gnu-data-plane-v1.json",
                "crates/comfy_runtime/abi/video-codec/verify-data-plane-bindings.c",
                "crates/comfy_test_support/fixtures/video/codec-data-plane-abi/manifest.json",
                "crates/comfy_test_support/fixtures/video/codec-bounded-memory-avio/manifest.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            tasks_by_id[video_codec_ltxv_h264_admission_foundation_id]["dependencies"],
            [video_codec_bounded_memory_avio_foundation_id],
        )
        codec_ltxv_h264_admission = tasks_by_id[
            video_codec_ltxv_h264_admission_foundation_id
        ]
        self.assertIn(
            "crates/comfy_runtime/src/native_video_codec_ffi.rs",
            codec_ltxv_h264_admission["writes"],
        )
        self.assertIn(
            "crates/comfy_runtime/src/trust.rs",
            codec_ltxv_h264_admission["writes"],
        )
        self.assertIn(
            "crates/comfy_test_support/fixtures/video/codec-ltxv-h264-admission/manifest.json",
            codec_ltxv_h264_admission["writes"],
        )
        self.assertEqual(
            tasks_by_id[video_codec_ltxv_h264_mp4_encode_foundation_id]["dependencies"],
            [video_codec_ltxv_h264_admission_foundation_id],
        )
        codec_ltxv_h264_mp4_encode = tasks_by_id[
            video_codec_ltxv_h264_mp4_encode_foundation_id
        ]
        self.assertEqual(
            codec_ltxv_h264_mp4_encode["writes"],
            [
                "crates/comfy_runtime/src/native_video_codec_ffi.rs",
                "crates/comfy_test_support/fixtures/video/codec-ltxv-h264-mp4-encode/manifest.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            tasks_by_id[video_codec_ltxv_h264_mp4_demux_foundation_id]["dependencies"],
            [video_codec_ltxv_h264_mp4_encode_foundation_id],
        )
        codec_ltxv_h264_mp4_demux = tasks_by_id[
            video_codec_ltxv_h264_mp4_demux_foundation_id
        ]
        self.assertEqual(
            codec_ltxv_h264_mp4_demux["writes"],
            [
                "crates/comfy_runtime/src/native_video_codec_ffi.rs",
                "crates/comfy_test_support/fixtures/video/codec-ltxv-h264-mp4-demux/manifest.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            tasks_by_id["comfy-parity-native-video-execution-foundation"]["dependencies"],
            [video_codec_ltxv_h264_mp4_demux_foundation_id],
        )
        self.assertEqual(
            tasks_by_id[video_foundation_id]["dependencies"],
            [video_codec_ltxv_h264_mp4_demux_foundation_id],
        )
        self.assertNotIn("Cargo.toml", codec_callable_symbols["writes"])
        self.assertNotIn("Cargo.lock", codec_callable_symbols["writes"])
        self.assertEqual(
            tasks_by_id[video_codec_plan_foundation_id]["writes"],
            [
                "crates/comfy_media/src/video.rs",
                "crates/comfy_media/src/comfy_media.rs",
                "crates/comfy_test_support/fixtures/video/codec-plan/manifest.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertNotIn("Cargo.toml", tasks_by_id[video_codec_plan_foundation_id]["writes"])
        self.assertNotIn(
            "crates/comfy_runtime/src/native_execution_controller.rs",
            tasks_by_id[video_codec_plan_foundation_id]["writes"],
        )
        self.assertEqual(
            tasks_by_id[video_codec_plan_foundation_id]["validations"],
            [
                "VAL-MEDIA-001",
                "VAL-CANCEL-001",
                "VAL-MEMORY-001",
                "VAL-OWNERSHIP-001",
            ],
        )
        self.assertTrue(tasks_by_id[video_codec_plan_foundation_id]["locked"])
        self.assertEqual(
            tasks_by_id[video_output_prefix_foundation_id]["writes"],
            [
                "crates/comfy_nodes/src/execution.rs",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
                "crates/comfy_nodes/src/comfy_nodes.rs",
            ],
        )
        self.assertTrue(tasks_by_id[video_output_prefix_foundation_id]["locked"])
        self.assertIn(
            "crates/comfy_media/src/native_node_payload.rs",
            tasks_by_id[video_component_foundation_id]["writes"],
        )
        self.assertIn(
            "crates/comfy_runtime/src/provider_materialization.rs",
            tasks_by_id[video_component_foundation_id]["writes"],
        )
        self.assertNotIn(
            "crates/comfy_model/src/frame_interpolation.rs",
            tasks_by_id[video_component_foundation_id]["writes"],
        )
        self.assertTrue(tasks_by_id[video_component_foundation_id]["locked"])
        self.assertEqual(
            tasks_by_id[video_output_media_foundation_id]["writes"],
            [
                "crates/comfy_nodes/src/execution.rs",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
                "crates/comfy_nodes/src/comfy_nodes.rs",
            ],
        )
        self.assertTrue(tasks_by_id[video_output_media_foundation_id]["locked"])
        self.assertEqual(
            tasks_by_id[video_output_projection_foundation_id]["writes"][:2],
            [
                "crates/comfy_nodes/src/execution.rs",
                "crates/comfy_runtime/src/native_execution_controller.rs",
            ],
        )
        self.assertNotIn(
            "crates/comfy_runtime/src/output_committer.rs",
            tasks_by_id[video_output_projection_foundation_id]["writes"],
        )
        self.assertTrue(tasks_by_id[video_output_projection_foundation_id]["locked"])
        self.assertIn(
            "crates/comfy_model/src/frame_interpolation.rs",
            tasks_by_id[frame_interpolation_model_foundation_id]["writes"],
        )
        self.assertNotIn(
            "crates/comfy_model/src/native_node_payload.rs",
            tasks_by_id[frame_interpolation_model_foundation_id]["writes"],
        )
        self.assertTrue(tasks_by_id[frame_interpolation_model_foundation_id]["locked"])
        self.assertIn(
            "crates/comfy_model/src/native_node_payload.rs",
            tasks_by_id[frame_interpolation_resource_foundation_id]["writes"],
        )
        self.assertIn(
            "crates/comfy_nodes/src/stored_payload.rs",
            tasks_by_id[frame_interpolation_resource_foundation_id]["writes"],
        )
        self.assertTrue(tasks_by_id[frame_interpolation_resource_foundation_id]["locked"])
        self.assertEqual(
            tasks_by_id[frame_interpolation_invocation_foundation_id]["writes"][:2],
            [
                "crates/comfy_model/src/frame_interpolation.rs",
                "crates/comfy_model/src/comfy_model.rs",
            ],
        )
        self.assertTrue(tasks_by_id[frame_interpolation_invocation_foundation_id]["locked"])
        self.assertEqual(
            tasks_by_id[tensor_grid_sample_foundation_id]["writes"][:2],
            [
                "crates/comfy_tensor/src/ops/spatial_functional_kernel_01.rs",
                "crates/comfy_tensor/tests/ops/spatial_functional_kernel_01.rs",
            ],
        )
        self.assertIn(
            "VAL-TENSOR-001",
            tasks_by_id[tensor_grid_sample_foundation_id]["validations"],
        )
        self.assertTrue(tasks_by_id[tensor_grid_sample_foundation_id]["locked"])
        self.assertEqual(
            tasks_by_id[tensor_interpolate_foundation_id]["writes"][:2],
            [
                "crates/comfy_tensor/src/ops/spatial_functional_kernel_01.rs",
                "crates/comfy_tensor/tests/ops/spatial_functional_kernel_01.rs",
            ],
        )
        self.assertIn(
            "VAL-TENSOR-001",
            tasks_by_id[tensor_interpolate_foundation_id]["validations"],
        )
        self.assertTrue(tasks_by_id[tensor_interpolate_foundation_id]["locked"])
        self.assertEqual(
            tasks_by_id[rife_tensor_arithmetic_foundation_id]["writes"][:3],
            [
                "crates/comfy_tensor/src/ops/activation_normalization_functional_01.rs",
                "crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_03.rs",
                "crates/comfy_model/src/native_ops.rs",
            ],
        )
        self.assertIn(
            "VAL-MODEL-FAMILY-001",
            tasks_by_id[rife_tensor_arithmetic_foundation_id]["validations"],
        )
        self.assertTrue(tasks_by_id[rife_tensor_arithmetic_foundation_id]["locked"])
        self.assertEqual(
            tasks_by_id[rife_execution_foundation_id]["writes"][:4],
            [
                "crates/comfy_model/src/frame_interpolation.rs",
                "crates/comfy_test_support/fixtures/models/frame-interpolation/admission/manifest.json",
                "crates/comfy_test_support/fixtures/models/frame-interpolation/resource/manifest.json",
                "crates/comfy_test_support/fixtures/models/frame-interpolation/rife-execution",
            ],
        )
        self.assertIn(
            "VAL-MODEL-FORMAT-001",
            tasks_by_id[rife_execution_foundation_id]["validations"],
        )
        self.assertTrue(tasks_by_id[rife_execution_foundation_id]["locked"])
        self.assertEqual(
            tasks_by_id[rife_sequence_execution_foundation_id]["writes"][:2],
            [
                "crates/comfy_model/src/frame_interpolation.rs",
                "crates/comfy_test_support/fixtures/models/frame-interpolation/rife-sequence",
            ],
        )
        self.assertIn(
            rife_execution_foundation_id,
            tasks_by_id[rife_sequence_execution_foundation_id]["dependencies"],
        )
        self.assertIn(
            rife_sequence_execution_foundation_id,
            tasks_by_id[film_tensor_average_pool_foundation_id]["dependencies"],
        )
        self.assertEqual(
            tasks_by_id[film_tensor_average_pool_foundation_id]["writes"][:3],
            [
                "crates/comfy_tensor/src/ops/spatial_functional_kernel_01.rs",
                "crates/comfy_tensor/tests/ops/spatial_functional_kernel_01.rs",
                "crates/comfy_test_support/fixtures/tensor_operations/spatial_functional_kernel_01/film-average-pool",
            ],
        )
        self.assertIn(
            film_tensor_average_pool_foundation_id,
            tasks_by_id[film_warp_foundation_id]["dependencies"],
        )
        self.assertEqual(
            tasks_by_id[film_warp_foundation_id]["writes"][:2],
            [
                "crates/comfy_model/src/frame_interpolation.rs",
                "crates/comfy_test_support/fixtures/models/frame-interpolation/film-warp",
            ],
        )
        self.assertIn(
            "VAL-DEVICE-001",
            tasks_by_id[film_warp_foundation_id]["validations"],
        )
        self.assertEqual(
            tasks_by_id[film_padded_convolution_foundation_id]["writes"][:2],
            [
                "crates/comfy_model/src/frame_interpolation.rs",
                "crates/comfy_test_support/fixtures/models/frame-interpolation/film-convolution",
            ],
        )
        self.assertIn(
            "VAL-DEVICE-001",
            tasks_by_id[film_padded_convolution_foundation_id]["validations"],
        )
        self.assertEqual(
            tasks_by_id[film_image_pyramid_foundation_id]["writes"][:2],
            [
                "crates/comfy_model/src/frame_interpolation.rs",
                "crates/comfy_test_support/fixtures/models/frame-interpolation/film-image-pyramid",
            ],
        )
        self.assertIn(
            "VAL-DEVICE-001",
            tasks_by_id[film_image_pyramid_foundation_id]["validations"],
        )
        self.assertEqual(
            tasks_by_id[film_subtree_foundation_id]["writes"][:2],
            [
                "crates/comfy_model/src/frame_interpolation.rs",
                "crates/comfy_test_support/fixtures/models/frame-interpolation/film-subtree",
            ],
        )
        self.assertIn(
            "VAL-MODEL-FORMAT-001",
            tasks_by_id[film_subtree_foundation_id]["validations"],
        )
        self.assertEqual(
            tasks_by_id[film_feature_pyramid_foundation_id]["writes"][:2],
            [
                "crates/comfy_model/src/frame_interpolation.rs",
                "crates/comfy_test_support/fixtures/models/frame-interpolation/film-feature-pyramid",
            ],
        )
        self.assertIn(
            "VAL-MODEL-FORMAT-001",
            tasks_by_id[film_feature_pyramid_foundation_id]["validations"],
        )
        self.assertEqual(
            tasks_by_id[film_flow_pyramid_synthesis_foundation_id]["writes"][:2],
            [
                "crates/comfy_model/src/frame_interpolation.rs",
                "crates/comfy_test_support/fixtures/models/frame-interpolation/film-flow-pyramid-synthesis",
            ],
        )
        self.assertIn(
            "VAL-DEVICE-001",
            tasks_by_id[film_flow_pyramid_synthesis_foundation_id]["validations"],
        )
        self.assertEqual(
            tasks_by_id[film_flow_estimator_foundation_id]["writes"][:2],
            [
                "crates/comfy_model/src/frame_interpolation.rs",
                "crates/comfy_test_support/fixtures/models/frame-interpolation/film-flow-estimator",
            ],
        )
        self.assertIn(
            "VAL-MODEL-FORMAT-001",
            tasks_by_id[film_flow_estimator_foundation_id]["validations"],
        )
        self.assertEqual(
            tasks_by_id[film_pyramid_algebra_foundation_id]["writes"][:2],
            [
                "crates/comfy_model/src/frame_interpolation.rs",
                "crates/comfy_test_support/fixtures/models/frame-interpolation/film-pyramid-algebra",
            ],
        )
        self.assertIn(
            "VAL-DEVICE-001",
            tasks_by_id[film_pyramid_algebra_foundation_id]["validations"],
        )
        self.assertEqual(
            tasks_by_id[film_fusion_foundation_id]["writes"][:2],
            [
                "crates/comfy_model/src/frame_interpolation.rs",
                "crates/comfy_test_support/fixtures/models/frame-interpolation/film-fusion",
            ],
        )
        self.assertIn(
            "VAL-MODEL-FORMAT-001",
            tasks_by_id[film_fusion_foundation_id]["validations"],
        )
        self.assertEqual(
            tasks_by_id[film_multi_timestep_foundation_id]["writes"][:2],
            [
                "crates/comfy_model/src/frame_interpolation.rs",
                "crates/comfy_test_support/fixtures/models/frame-interpolation/film-multi-timestep",
            ],
        )
        self.assertIn(
            "VAL-MODEL-FORMAT-001",
            tasks_by_id[film_multi_timestep_foundation_id]["validations"],
        )
        self.assertEqual(
            tasks_by_id[film_sequence_foundation_id]["writes"][:2],
            [
                "crates/comfy_model/src/frame_interpolation.rs",
                "crates/comfy_test_support/fixtures/models/frame-interpolation/film-sequence",
            ],
        )
        self.assertIn(
            "VAL-MODEL-FORMAT-001",
            tasks_by_id[film_sequence_foundation_id]["validations"],
        )
        self.assertEqual(
            tasks_by_id[frame_interpolation_resource_exhaustion_foundation_id][
                "writes"
            ][:5],
            [
                "crates/comfy_model/src/frame_interpolation.rs",
                "crates/comfy_model/src/native_ops.rs",
                "crates/comfy_tensor/src/ops/spatial_functional_kernel_01.rs",
                "crates/comfy_tensor/src/ops/shape_layout_transform_02.rs",
                "crates/comfy_tensor/src/ops/shape_layout_transform_03.rs",
            ],
        )
        self.assertIn(
            "VAL-MEMORY-001",
            tasks_by_id[frame_interpolation_resource_exhaustion_foundation_id][
                "validations"
            ],
        )
        self.assertIn(
            frame_interpolation_resource_exhaustion_foundation_id,
            tasks_by_id[video_codec_plan_foundation_id]["dependencies"],
        )
        self.assertIn(
            video_codec_dependency_contract_foundation_id,
            tasks_by_id[video_codec_dependency_closure_foundation_id]["dependencies"],
        )
        self.assertIn(
            video_codec_dependency_closure_foundation_id,
            tasks_by_id[video_codec_retained_loader_foundation_id]["dependencies"],
        )
        self.assertIn(
            video_codec_retained_loader_foundation_id,
            tasks_by_id[video_codec_reviewed_abi_foundation_id]["dependencies"],
        )
        self.assertIn(
            video_codec_reviewed_abi_foundation_id,
            tasks_by_id[video_codec_callable_symbol_certification_foundation_id][
                "dependencies"
            ],
        )
        self.assertIn(
            video_codec_callable_symbol_certification_foundation_id,
            tasks_by_id[video_codec_symbol_binding_foundation_id]["dependencies"],
        )
        self.assertIn(
            video_codec_symbol_binding_foundation_id,
            tasks_by_id[video_codec_data_plane_abi_foundation_id]["dependencies"],
        )
        self.assertIn(
            video_codec_ltxv_h264_mp4_demux_foundation_id,
            tasks_by_id[video_foundation_id]["dependencies"],
        )
        self.assertTrue(tasks_by_id[rife_sequence_execution_foundation_id]["locked"])
        self.assertTrue(tasks_by_id[film_tensor_average_pool_foundation_id]["locked"])
        self.assertTrue(tasks_by_id[film_warp_foundation_id]["locked"])
        self.assertTrue(tasks_by_id[film_padded_convolution_foundation_id]["locked"])
        self.assertTrue(tasks_by_id[film_image_pyramid_foundation_id]["locked"])
        self.assertTrue(tasks_by_id[film_subtree_foundation_id]["locked"])
        self.assertTrue(tasks_by_id[film_feature_pyramid_foundation_id]["locked"])
        self.assertTrue(tasks_by_id[film_flow_pyramid_synthesis_foundation_id]["locked"])
        self.assertTrue(tasks_by_id[film_flow_estimator_foundation_id]["locked"])
        self.assertTrue(tasks_by_id[film_pyramid_algebra_foundation_id]["locked"])
        self.assertTrue(tasks_by_id[film_fusion_foundation_id]["locked"])
        self.assertTrue(tasks_by_id[film_multi_timestep_foundation_id]["locked"])
        self.assertTrue(tasks_by_id[film_sequence_foundation_id]["locked"])
        self.assertTrue(
            tasks_by_id[frame_interpolation_resource_exhaustion_foundation_id]["locked"]
        )
        self.assertIn(text_regex_foundation_id, dependencies[text_regex_id])
        self.assertIn(
            "projects/comfy/ComfyUI/comfy_extras/nodes_string.py",
            tasks_by_id[text_regex_id]["reads"],
        )
        self.assertIn(
            "crates/comfy_nodes/Cargo.toml",
            tasks_by_id[text_regex_foundation_id]["writes"],
        )
        self.assertIn("Cargo.lock", tasks_by_id[text_regex_foundation_id]["writes"])
        self.assertIn(
            ".agents/specs/comfy-parity/catalogs/native-backend-dependencies.json",
            tasks_by_id[text_regex_foundation_id]["writes"],
        )
        self.assertIn(
            ".agents/specs/comfy-parity/validate_backend_dependencies.py",
            tasks_by_id[text_regex_foundation_id]["writes"],
        )
        self.assertIn(
            "crates/comfy_nodes/src/execution.rs",
            tasks_by_id[text_regex_foundation_id]["writes"],
        )
        self.assertIn(text_regex_foundation_id, dependencies[primitive_id])
        self.assertIn(
            "projects/comfy/ComfyUI/comfy_extras/nodes_primitive.py",
            tasks_by_id[primitive_id]["reads"],
        )
        self.assertTrue(tasks_by_id[text_regex_foundation_id]["locked"])
        self.assertIn(text_transform_foundation_id, dependencies[text_transform_id])
        self.assertIn(
            "crates/comfy_nodes/src/text_format.rs",
            tasks_by_id[text_transform_foundation_id]["writes"],
        )
        self.assertIn(
            ".agents/specs/comfy-parity/test_generate_node_contract_catalog.py",
            tasks_by_id[text_transform_foundation_id]["writes"],
        )
        self.assertIn(
            ".agents/specs/comfy-parity/validation.md",
            tasks_by_id[text_transform_foundation_id]["writes"],
        )
        self.assertTrue(tasks_by_id[text_transform_foundation_id]["locked"])
        self.assertIn(
            "octal escapes",
            tasks_by_id[text_transform_foundation_id]["done"],
        )
        self.assertIn(
            "pre-allocation bounded output",
            tasks_by_id[text_transform_foundation_id]["done"],
        )
        self.assertIn(text_generation_foundation_id, dependencies[text_generation_id])
        self.assertIn(
            gemma_generation_id,
            tasks_by_id[text_generation_foundation_id]["dependencies"],
        )
        self.assertIn(
            "crates/comfy_model/src/clip_text_encoder_decoder.rs",
            tasks_by_id[decoder_text_generation_foundation_id]["writes"],
        )
        self.assertIn(media_text_foundation_id, dependencies[media_text_id])
        self.assertIn(
            "crates/comfy_media/src/text_rendering.rs",
            tasks_by_id[media_text_foundation_id]["writes"],
        )
        self.assertIn(
            "crates/comfy_model/src/sdpose.rs",
            tasks_by_id[sdpose_projection_id]["writes"],
        )
        self.assertIn(
            "crates/comfy_runtime/src/native_execution_controller.rs",
            tasks_by_id[sdpose_foundation_id]["writes"],
        )
        self.assertEqual(
            tasks_by_id[lotusd_sampling_id]["writes"][:2],
            [
                "crates/comfy_sampler/src/sampling_profile.rs",
                "crates/comfy_sampler/src/algorithms/native_diffusion.rs",
            ],
        )
        self.assertNotIn(
            "crates/comfy_sampler/src/native_diffusion_payload.rs",
            tasks_by_id[lotusd_sampling_id]["writes"],
        )
        self.assertIn(
            bounded_dense_spatial_id,
            tasks_by_id[lotusd_sampling_id]["dependencies"],
        )
        self.assertIn(
            lotusd_sampling_id,
            tasks_by_id[sdpose_head_projection_id]["dependencies"],
        )
        self.assertIn(
            "crates/comfy_model/src/sdpose.rs",
            tasks_by_id[sdpose_head_projection_id]["writes"],
        )
        self.assertIn(
            "crates/comfy_test_support/fixtures/sdpose/head_projection",
            tasks_by_id[sdpose_head_projection_id]["writes"],
        )
        self.assertNotIn(
            "crates/comfy_model/src/comfy_model.rs",
            tasks_by_id[sdpose_head_projection_id]["writes"],
        )
        self.assertIn(
            "crates/comfy_test_support/fixtures/sdpose/projection/manifest.json",
            tasks_by_id[sdpose_head_projection_id]["reads"],
        )
        self.assertIn(
            sdpose_head_projection_id,
            tasks_by_id[sdpose_foundation_id]["dependencies"],
        )
        self.assertNotIn(
            "crates/comfy_sampler/src/native_diffusion_payload.rs",
            tasks_by_id[sdpose_foundation_id]["writes"],
        )
        self.assertNotIn(
            "crates/comfy_model/src/sdpose.rs",
            tasks_by_id[sdpose_foundation_id]["writes"],
        )
        self.assertIn(
            "crates/comfy_model/src/sdpose.rs",
            tasks_by_id[sdpose_foundation_id]["reads"],
        )
        self.assertIn(
            "crates/comfy_tensor/src/image_ops.rs",
            tasks_by_id[sdpose_foundation_id]["reads"],
        )
        self.assertIn(
            "crates/comfy_test_support/fixtures/sdpose/execution",
            tasks_by_id[sdpose_foundation_id]["writes"],
        )
        self.assertIn(
            "crates/comfy_media/src/native_node_payload.rs",
            tasks_by_id[sdpose_projection_id]["writes"],
        )
        self.assertIn(
            "crates/comfy_model/Cargo.toml",
            tasks_by_id[sdpose_projection_id]["writes"],
        )
        self.assertIn("Cargo.lock", tasks_by_id[sdpose_projection_id]["writes"])
        self.assertIn(
            "projects/comfy/ComfyUI/comfy/ldm/modules/diffusionmodules/openaimodel.py",
            tasks_by_id[sdpose_capture_id]["reads"],
        )
        self.assertIn(
            "crates/comfy_model/src/native_ops.rs",
            tasks_by_id[immutable_dense_attention_id]["writes"],
        )
        self.assertIn(
            "crates/comfy_model/tests/native_ops.rs",
            tasks_by_id[immutable_dense_attention_id]["writes"],
        )
        self.assertIn(
            "crates/comfy_model/src/attention.rs",
            tasks_by_id[immutable_dense_attention_id]["writes"],
        )
        self.assertNotIn(
            "crates/comfy_tensor/src/ops/native_diffusion.rs",
            tasks_by_id[immutable_dense_attention_id]["writes"],
        )
        self.assertNotIn(
            "crates/comfy_model/src/comfy_model.rs",
            tasks_by_id[immutable_dense_attention_id]["writes"],
        )
        self.assertIn(
            "leaving generation, prefetch, semantic digest",
            tasks_by_id[immutable_dense_attention_id]["done"],
        )
        self.assertIn(
            "Tensor SDPA fixtures",
            tasks_by_id[immutable_dense_attention_id]["done"],
        )
        self.assertEqual(
            tasks_by_id[immutable_dense_attention_id]["validation_packages"],
            ["comfy_test_support"],
        )
        self.assertEqual(
            tasks_by_id[immutable_dense_attention_id]["writes"],
            [
                "crates/comfy_model/src/native_ops.rs",
                "crates/comfy_model/tests/native_ops.rs",
                "crates/comfy_model/src/attention.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertIn(
            "crates/comfy_model/src/sd2_family.rs",
            tasks_by_id[sdpose_capture_id]["reads"],
        )
        self.assertIn(
            "crates/comfy_model/src/families/lotusd_comfy_model_0106.rs",
            tasks_by_id[sdpose_capture_id]["reads"],
        )
        self.assertIn(
            "crates/comfy_model/src/families/sd20_comfy_model_0119.rs",
            tasks_by_id[sdpose_capture_id]["reads"],
        )
        self.assertNotIn(
            "crates/comfy_model/src/attention.rs",
            tasks_by_id[sdpose_capture_id]["writes"],
        )
        self.assertNotIn(
            "crates/comfy_model/src/native_ops.rs",
            tasks_by_id[sdpose_capture_id]["writes"],
        )
        self.assertIn(
            "crates/comfy_test_support/fixtures/sdpose/sd2_capture/production_manifest",
            tasks_by_id[sdpose_capture_id]["writes"],
        )
        self.assertIn(
            "crates/comfy_test_support/fixtures/sdpose/sd2_capture/reduced_numeric",
            tasks_by_id[sdpose_capture_id]["writes"],
        )
        self.assertIn(
            "manifest-only production fixture",
            tasks_by_id[sdpose_capture_id]["done"],
        )
        self.assertIn(
            "test-support-only shape-reduced numeric oracle",
            tasks_by_id[sdpose_capture_id]["done"],
        )
        self.assertEqual(
            tasks_by_id[sdpose_capture_id]["reads"],
            [
                "projects/comfy/ComfyUI/comfy/ldm/modules/diffusionmodules/openaimodel.py",
                "projects/comfy/ComfyUI/comfy/ldm/modules/attention.py",
                "projects/comfy/ComfyUI/comfy/ops.py",
                "projects/comfy/ComfyUI/comfy/model_base.py",
                "projects/comfy/ComfyUI/comfy/supported_models.py",
                "projects/comfy/ComfyUI/comfy/supported_models_base.py",
                "crates/comfy_model/src/attention.rs",
                "crates/comfy_model/src/model_family.rs",
                "crates/comfy_model/src/native_ops.rs",
                "crates/comfy_model/src/sd2_family.rs",
                "crates/comfy_model/src/families/lotusd_comfy_model_0106.rs",
                "crates/comfy_model/src/families/sd20_comfy_model_0119.rs",
                "crates/comfy_model/src/slices/native_diffusion.rs",
                "crates/comfy_model/src/sdpose.rs",
                "crates/comfy_model/tests/families/lotusd_comfy_model_0106.rs",
                "crates/comfy_model/tests/families/sd20_comfy_model_0119.rs",
            ],
        )
        self.assertEqual(
            tasks_by_id[sdpose_capture_id]["writes"],
            [
                "crates/comfy_model/src/sdpose.rs",
                "crates/comfy_model/src/comfy_model.rs",
                "crates/comfy_model/tests/sdpose.rs",
                "crates/comfy_test_support/fixtures/sdpose/sd2_capture/production_manifest",
                "crates/comfy_test_support/fixtures/sdpose/sd2_capture/reduced_numeric",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            tasks_by_id[sdpose_resource_id]["writes"],
            [
                "crates/comfy_model/src/sdpose.rs",
                "crates/comfy_model/src/native_node_payload.rs",
                "crates/comfy_model/src/comfy_model.rs",
                "crates/comfy_model/tests/sdpose.rs",
                "crates/comfy_nodes/src/stored_payload.rs",
                "crates/comfy_test_support/fixtures/sdpose/resource",
                "crates/comfy_test_support/tests/native_node_family_e2e.rs",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            tasks_by_id[sdpose_resource_id]["validations"],
            [
                "VAL-MODEL-FAMILY-001",
                "VAL-TENSOR-001",
                "VAL-CANCEL-001",
                "VAL-MEMORY-001",
                "VAL-OWNERSHIP-001",
            ],
        )
        self.assertIn(
            sdpose_projection_id,
            tasks_by_id[immutable_dense_attention_id]["dependencies"],
        )
        self.assertIn(
            immutable_dense_attention_id,
            tasks_by_id[sdpose_capture_id]["dependencies"],
        )
        self.assertIn(
            "comfy-parity-native-model-family-lotusd-comfy-model-0106",
            tasks_by_id[sdpose_capture_id]["dependencies"],
        )
        self.assertIn(
            "comfy-parity-native-model-family-sd20-comfy-model-0119",
            tasks_by_id[sdpose_capture_id]["dependencies"],
        )
        self.assertEqual(
            tasks_by_id[sdpose_capture_id]["dependencies"],
            [
                immutable_dense_attention_id,
                "comfy-parity-native-model-family-lotusd-comfy-model-0106",
                "comfy-parity-native-model-family-sd20-comfy-model-0119",
                "comfy-parity-model-detection-any-of-key-selector-consolidation",
            ],
        )
        self.assertIn(
            sdpose_capture_id,
            tasks_by_id[sdpose_resource_id]["dependencies"],
        )
        self.assertIn(
            sdpose_resource_id,
            tasks_by_id[bounded_dense_spatial_id]["dependencies"],
        )
        self.assertEqual(
            tasks_by_id[bounded_dense_spatial_id]["writes"],
            [
                "crates/comfy_model/src/native_ops.rs",
                "crates/comfy_model/tests/native_ops.rs",
                "crates/comfy_tensor/src/ops/comfy_operator_indirection_01.rs",
                "crates/comfy_tensor/src/ops/spatial_functional_kernel_01.rs",
                "crates/comfy_tensor/src/ops/neural_network_module_02.rs",
                "crates/comfy_tensor/src/ops/activation_normalization_functional_01.rs",
                "crates/comfy_tensor/tests/ops/spatial_functional_kernel_01.rs",
                "crates/comfy_tensor/tests/ops/neural_network_module_02.rs",
                "crates/comfy_tensor/tests/ops/activation_normalization_functional_01.rs",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertIn(
            sdpose_head_projection_id,
            tasks_by_id[sdpose_foundation_id]["dependencies"],
        )
        self.assertIn(
            "projects/comfy/ComfyUI/comfy_extras/nodes_sdpose.py",
            tasks_by_id[sdpose_id]["reads"],
        )
        self.assertIn(
            sdpose_foundation_id,
            dependencies[sdpose_id],
        )
        self.assertTrue(tasks_by_id[sdpose_foundation_id]["locked"])
        self.assertIn(video_foundation_id, dependencies[video_id])
        self.assertIn(video_foundation_id, dependencies[video_preprocessor_id])
        self.assertIn(
            "crates/comfy_media/src/video.rs",
            tasks_by_id[video_foundation_id]["writes"],
        )
        self.assertIn(
            "crates/comfy_model/src/frame_interpolation.rs",
            tasks_by_id[video_foundation_id]["writes"],
        )
        self.assertTrue(tasks_by_id[video_foundation_id]["locked"])
        self.assertIn(image_source_foundation_id, dependencies[image_filter_id])
        self.assertIn(image_source_foundation_id, dependencies[image_transform_id])
        self.assertIn(
            "crates/comfy_media/src/image_quantization.rs",
            tasks_by_id[image_source_foundation_id]["writes"],
        )
        self.assertIn(structured_link_foundation_id, dependencies[structured_transform_id])
        self.assertIn(
            "crates/comfy_runtime/src/prompt_compiler.rs",
            tasks_by_id[structured_link_foundation_id]["writes"],
        )
        self.assertIn(
            ".agents/specs/comfy-parity/ownership-policy.json",
            tasks_by_id[structured_link_foundation_id]["writes"],
        )
        self.assertIn(
            "flat dotted prompt keys",
            tasks_by_id[structured_link_foundation_id]["outcome"],
        )
        self.assertIn(shader_foundation_id, dependencies[shader_id])
        self.assertIn(
            "crates/comfy_tensor/src/shader.rs",
            tasks_by_id[shader_foundation_id]["writes"],
        )
        self.assertIn(detection_foundation_id, dependencies[detection_id])
        self.assertIn(
            "crates/comfy_model/src/detection.rs",
            tasks_by_id[detection_foundation_id]["writes"],
        )
        for identifier in (
            image_source_foundation_id,
            decoder_text_generation_foundation_id,
            prepared_generation_id,
            qwen_generation_id,
            gemma_generation_id,
            text_generation_foundation_id,
            media_text_foundation_id,
            structured_link_foundation_id,
            shader_foundation_id,
            detection_foundation_id,
        ):
            self.assertTrue(tasks_by_id[identifier]["locked"])
        self.assertTrue(tasks_by_id[guidance_id]["locked"])
        self.assertTrue(tasks_by_id[hooks_id]["locked"])
        self.assertTrue(tasks_by_id[hook_consumers_id]["locked"])

        waves = planning.task_waves(tasks)
        self.assertEqual(waves[foundation_id], waves[compute_id] + 1)
        self.assertEqual(waves[schema_id], waves[foundation_id] + 1)
        self.assertEqual(waves[value_id], waves[schema_id] + 1)
        self.assertEqual(waves[asset_id], waves[value_id] + 1)
        self.assertEqual(
            waves[provider_id],
            waves[shader_foundation_id] + 1,
        )
        self.assertEqual(
            waves[text_regex_foundation_id],
            waves["comfy-parity-opt-in-product-build-boundary"] + 1,
        )
        self.assertEqual(
            waves[text_transform_foundation_id], waves[text_regex_foundation_id] + 1
        )
        self.assertEqual(
            waves[decoder_text_generation_foundation_id], waves[provider_id] + 1
        )
        self.assertEqual(
            waves[prepared_generation_id],
            waves[decoder_text_generation_foundation_id] + 1,
        )
        self.assertEqual(
            waves[qwen_preparation_id], waves[prepared_generation_id] + 1
        )
        self.assertEqual(waves[deepstack_id], waves[qwen_preparation_id] + 1)
        self.assertEqual(waves[qwen_tokenizer_id], waves[deepstack_id] + 1)
        self.assertEqual(waves[qwen3_decoder_id], waves[qwen_tokenizer_id] + 1)
        self.assertEqual(waves[qwen35_decoder_id], waves[qwen3_decoder_id] + 1)
        self.assertEqual(waves[qwen_vision_id], waves[qwen35_decoder_id] + 1)
        self.assertEqual(waves[qwen_resource_id], waves[qwen_vision_id] + 1)
        self.assertEqual(waves[qwen_generation_id], waves[qwen_resource_id] + 1)
        self.assertEqual(waves[gemma_image_video_id], waves[qwen_generation_id] + 1)
        self.assertEqual(
            waves[gemma_audio_preparation_id], waves[gemma_image_video_id] + 1
        )
        self.assertEqual(
            waves[gemma_tokenizer_id], waves[gemma_audio_preparation_id] + 1
        )
        self.assertEqual(waves[gemma3_decoder_id], waves[gemma_tokenizer_id] + 1)
        self.assertEqual(waves[gemma4_decoder_id], waves[gemma3_decoder_id] + 1)
        self.assertEqual(waves[gemma3_vision_id], waves[gemma4_decoder_id] + 1)
        self.assertEqual(waves[gemma4_vision_id], waves[gemma3_vision_id] + 1)
        self.assertEqual(waves[gemma4_audio_id], waves[gemma4_vision_id] + 1)
        self.assertEqual(waves[gemma_resource_id], waves[gemma4_audio_id] + 1)
        self.assertEqual(
            waves[gemma_generation_id], waves[gemma_resource_id] + 1
        )
        self.assertEqual(
            waves[text_generation_foundation_id], waves[gemma_generation_id] + 1
        )
        self.assertEqual(
            waves[sdpose_projection_id], waves[text_generation_foundation_id] + 1
        )
        self.assertEqual(
            waves[immutable_dense_attention_id], waves[sdpose_projection_id] + 1
        )
        self.assertEqual(
            waves[sdpose_capture_id], waves[immutable_dense_attention_id] + 1
        )
        self.assertEqual(waves[sdpose_resource_id], waves[sdpose_capture_id] + 1)
        self.assertEqual(
            waves[bounded_dense_spatial_id], waves[sdpose_resource_id] + 1
        )
        self.assertEqual(
            waves[lotusd_sampling_id], waves[bounded_dense_spatial_id] + 1
        )
        self.assertEqual(
            waves[sdpose_head_projection_id], waves[lotusd_sampling_id] + 1
        )
        self.assertEqual(
            waves[sdpose_foundation_id], waves[sdpose_head_projection_id] + 1
        )
        self.assertEqual(
            waves[video_output_prefix_foundation_id], waves[hook_consumers_id] + 1
        )
        self.assertEqual(
            waves[video_component_foundation_id],
            waves[video_output_prefix_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_output_media_foundation_id],
            waves[video_component_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_output_projection_foundation_id],
            waves[video_output_media_foundation_id] + 1,
        )
        self.assertEqual(
            waves[frame_interpolation_model_foundation_id],
            waves[video_output_projection_foundation_id] + 1,
        )
        self.assertEqual(
            waves[frame_interpolation_resource_foundation_id],
            waves[frame_interpolation_model_foundation_id] + 1,
        )
        self.assertEqual(
            waves[frame_interpolation_invocation_foundation_id],
            waves[frame_interpolation_resource_foundation_id] + 1,
        )
        self.assertEqual(
            waves[tensor_grid_sample_foundation_id],
            waves[frame_interpolation_invocation_foundation_id] + 1,
        )
        self.assertEqual(
            waves[tensor_interpolate_foundation_id],
            waves[tensor_grid_sample_foundation_id] + 1,
        )
        self.assertEqual(
            waves[rife_tensor_arithmetic_foundation_id],
            waves[tensor_interpolate_foundation_id] + 1,
        )
        self.assertEqual(
            waves[rife_execution_foundation_id],
            waves[rife_tensor_arithmetic_foundation_id] + 1,
        )
        self.assertEqual(
            waves[rife_sequence_execution_foundation_id],
            waves[rife_execution_foundation_id] + 1,
        )
        self.assertEqual(
            waves[film_pyramid_algebra_foundation_id],
            waves[film_flow_estimator_foundation_id] + 1,
        )
        self.assertEqual(
            waves[film_fusion_foundation_id],
            waves[film_pyramid_algebra_foundation_id] + 1,
        )
        self.assertEqual(
            waves[film_multi_timestep_foundation_id],
            waves[film_fusion_foundation_id] + 1,
        )
        self.assertEqual(
            waves[film_sequence_foundation_id],
            waves[film_multi_timestep_foundation_id] + 1,
        )
        self.assertEqual(
            waves[frame_interpolation_resource_exhaustion_foundation_id],
            waves[film_sequence_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_plan_foundation_id],
            waves[frame_interpolation_resource_exhaustion_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_ffi_certification_foundation_id],
            waves[video_codec_plan_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_package_capture_foundation_id],
            waves[video_codec_ffi_certification_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_elf_inspection_foundation_id],
            waves[video_codec_package_capture_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_inspected_certification_foundation_id],
            waves[video_codec_elf_inspection_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_dependency_contract_foundation_id],
            waves[video_codec_inspected_certification_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_dependency_closure_foundation_id],
            waves[video_codec_dependency_contract_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_retained_loader_foundation_id],
            waves[video_codec_dependency_closure_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_reviewed_abi_foundation_id],
            waves[video_codec_retained_loader_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_callable_symbol_certification_foundation_id],
            waves[video_codec_reviewed_abi_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_symbol_binding_foundation_id],
            waves[video_codec_callable_symbol_certification_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_data_plane_abi_foundation_id],
            waves[video_codec_symbol_binding_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_bounded_memory_avio_foundation_id],
            waves[video_codec_data_plane_abi_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_ltxv_h264_admission_foundation_id],
            waves[video_codec_bounded_memory_avio_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_ltxv_h264_mp4_encode_foundation_id],
            waves[video_codec_ltxv_h264_admission_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_ltxv_h264_mp4_demux_foundation_id],
            waves[video_codec_ltxv_h264_mp4_encode_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_foundation_id],
            waves[video_codec_ltxv_h264_mp4_demux_foundation_id] + 1,
        )
        self.assertEqual(
            waves[film_tensor_average_pool_foundation_id],
            waves[rife_sequence_execution_foundation_id] + 1,
        )
        self.assertEqual(
            waves[film_warp_foundation_id],
            waves[film_tensor_average_pool_foundation_id] + 1,
        )
        self.assertEqual(
            waves[film_padded_convolution_foundation_id],
            waves[film_warp_foundation_id] + 1,
        )
        self.assertEqual(
            waves[film_image_pyramid_foundation_id],
            waves[film_padded_convolution_foundation_id] + 1,
        )
        self.assertEqual(
            waves[film_subtree_foundation_id],
            waves[film_image_pyramid_foundation_id] + 1,
        )
        self.assertEqual(
            waves[film_feature_pyramid_foundation_id],
            waves[film_subtree_foundation_id] + 1,
        )
        self.assertEqual(
            waves[film_flow_pyramid_synthesis_foundation_id],
            waves[film_feature_pyramid_foundation_id] + 1,
        )
        self.assertEqual(
            waves[film_flow_estimator_foundation_id],
            waves[film_flow_pyramid_synthesis_foundation_id] + 1,
        )
        self.assertEqual(
            waves[image_source_foundation_id], waves[text_transform_foundation_id] + 1
        )
        self.assertEqual(
            waves[media_text_foundation_id], waves[detection_id] + 1
        )
        self.assertEqual(
            waves[structured_link_foundation_id],
            waves[image_source_foundation_id] + 1,
        )
        self.assertEqual(
            waves[shader_foundation_id], waves[structured_link_foundation_id] + 1
        )
        self.assertEqual(
            waves[detection_foundation_id],
            max(waves[video_foundation_id], waves[video_id]) + 1,
        )
        self.assertEqual(
            waves[registry_id], max(waves[identifier] for identifier in node_ids) + 1
        )

    def test_native_node_foundation_and_registry_own_runtime_reachability_paths(self) -> None:
        tasks, _ = planning.all_tasks()
        tasks_by_id = {str(item["id"]): item for item in tasks}
        foundation_writes = set(
            tasks_by_id["comfy-parity-native-node-runtime-foundation"]["writes"]
        )
        schema_writes = set(tasks_by_id["comfy-parity-native-node-schema-metadata-foundation"]["writes"])
        value_reads = set(tasks_by_id["comfy-parity-native-node-compute-value-foundation"]["reads"])
        value_writes = set(tasks_by_id["comfy-parity-native-node-compute-value-foundation"]["writes"])
        asset_reads = set(tasks_by_id["comfy-parity-native-node-asset-effect-foundation"]["reads"])
        asset_writes = set(tasks_by_id["comfy-parity-native-node-asset-effect-foundation"]["writes"])
        provider_reads = set(
            tasks_by_id["comfy-parity-native-node-provider-invocation-foundation"]["reads"]
        )
        provider_writes = set(tasks_by_id["comfy-parity-native-node-provider-invocation-foundation"]["writes"])
        registry_writes = set(
            tasks_by_id["comfy-parity-native-registry-integration"]["writes"]
        )

        self.assertIn("crates/comfy_nodes/src/execution.rs", foundation_writes)
        self.assertIn("crates/comfy_nodes/src/object_info.rs", foundation_writes)
        self.assertIn("Cargo.lock", foundation_writes)
        self.assertIn("crates/comfy_runtime/src/executor.rs", foundation_writes)
        self.assertIn("crates/comfy_runtime/src/cache.rs", foundation_writes)
        self.assertIn("crates/comfy_worker/src/comfy_worker.rs", foundation_writes)
        self.assertIn("crates/comfy_api/src/services.rs", foundation_writes)
        self.assertIn("crates/comfy_plugin_host/src/registry_adapter.rs", foundation_writes)
        self.assertIn("crates/comfy_ui/src/execution_model.rs", foundation_writes)
        self.assertIn("crates/sim/src/sim.rs", foundation_writes)
        self.assertIn(
            "crates/comfy_test_support/tests/plugin_e2e.rs", foundation_writes
        )
        self.assertIn(
            ".agents/specs/comfy-parity/ownership-policy.json", foundation_writes
        )
        self.assertIn("crates/comfy_nodes/src/execution.rs", schema_writes)
        self.assertIn(
            "crates/comfy_nodes/src/families/empty_root_category_declared_by_source_01.rs",
            schema_writes,
        )
        self.assertIn("crates/comfy_nodes/src/slices/native_image.descriptors.json", schema_writes)
        self.assertIn("crates/comfy_runtime/src/executor.rs", schema_writes)
        self.assertIn("crates/comfy_runtime/src/graph.rs", schema_writes)
        self.assertIn("crates/comfy_plugin_sdk/src/type_ids.rs", asset_reads)
        self.assertIn("crates/comfy_plugin_sdk/src/type_ids.rs", asset_writes)
        self.assertIn(
            ".agents/specs/comfy-parity/catalogs/native-backend-dependencies.json",
            asset_reads,
        )
        self.assertIn(
            ".agents/specs/comfy-parity/catalogs/native-backend-dependencies.json",
            asset_writes,
        )
        self.assertIn(
            ".agents/specs/comfy-parity/validate_backend_dependencies.py",
            asset_writes,
        )
        self.assertIn("crates/comfy_runtime/src/workflow_formats.rs", schema_writes)
        self.assertIn("crates/comfy_api/src/services.rs", schema_writes)
        self.assertIn("crates/comfy_plugin_host/src/registry_adapter.rs", schema_writes)
        self.assertIn("crates/comfy_worker/src/comfy_worker.rs", schema_writes)
        schema_task = tasks_by_id["comfy-parity-native-node-schema-metadata-foundation"]
        self.assertEqual(
            schema_task["criterion_ids"],
            [
                "4.1", "4.2", "4.3", "6.1", "6.2", "6.3", "6.5",
                "16.3", "16.4", "32.1", "32.3", "32.5", "32.8", "44.2",
            ],
        )
        self.assertNotIn("VAL-NODE-002", schema_task["validations"])
        self.assertIn("crates/comfy_model/src/native_node_payload.rs", value_writes)
        self.assertIn("crates/comfy_model/src/clip_vision.rs", value_writes)
        self.assertIn("crates/comfy_model/src/vision_models.rs", value_writes)
        for path in [
            "projects/comfy/ComfyUI/comfy_api/latest/_input/basic_types.py",
            "projects/comfy/ComfyUI/comfy_api/latest/_input/video_types.py",
            "projects/comfy/ComfyUI/comfy_api/latest/_input_impl/video_types.py",
            "projects/comfy/ComfyUI/comfy_api/latest/_util/video_types.py",
            "projects/comfy/ComfyUI/comfy_api/latest/_util/geometry_types.py",
            "projects/comfy/ComfyUI/comfy_api/latest/_io.py",
            "projects/comfy/ComfyUI/comfy_extras/nodes_hunyuan3d.py",
            "projects/comfy/ComfyUI/comfy_extras/nodes_load_3d.py",
            "projects/comfy/ComfyUI/comfy_extras/nodes_gaussian_splat.py",
        ]:
            self.assertIn(path, asset_reads)
        for path in [
            "crates/comfy_tensor/src/native_node_payload.rs",
            "crates/comfy_tensor/src/image_ops.rs",
            "crates/comfy_tensor/src/operation.rs",
            "crates/comfy_tensor/src/cpu_backend.rs",
            "crates/comfy_nodes/src/source_type.rs",
        ]:
            self.assertIn(path, asset_writes)
        self.assertIn("crates/comfy_model/tests/model_families.rs", value_writes)
        self.assertIn("crates/comfy_model/src/controlnet.rs", value_writes)
        self.assertIn("crates/comfy_model/src/conditioning.rs", value_writes)
        self.assertIn("crates/comfy_tensor/src/native_node_payload.rs", value_writes)
        self.assertIn("crates/comfy_sampler/src/native_diffusion_payload.rs", value_writes)
        self.assertIn("crates/comfy_sampler/src/native_node_payload.rs", value_writes)
        self.assertIn("crates/comfy_plugin_host/Cargo.toml", value_writes)
        self.assertIn("crates/comfy_plugin_sdk/Cargo.toml", value_writes)
        self.assertIn("crates/comfy_plugin_sdk/src/type_ids.rs", value_writes)
        self.assertIn("crates/comfy_media/Cargo.toml", value_writes)
        self.assertIn("crates/comfy_media/src/native_node_payload.rs", value_writes)
        self.assertIn("crates/comfy_runtime/src/executor.rs", value_writes)
        self.assertIn("crates/comfy_nodes/src/source_type.rs", value_writes)
        self.assertIn("crates/comfy_nodes/src/stored_payload.rs", value_writes)
        self.assertIn("crates/comfy_plugin_host/src/registry_adapter.rs", value_writes)
        self.assertIn("crates/comfy_test_support/tests/plugin_e2e.rs", value_writes)
        value_task = tasks_by_id["comfy-parity-native-node-compute-value-foundation"]
        self.assertIn(".agents/specs/comfy-parity/catalogs/backend-node-contracts.json", value_reads)
        self.assertIn(39, value_task["requirements"])
        self.assertIn(35, value_task["designs"])
        self.assertIn("39.3", value_task["criterion_ids"])
        self.assertIn("39.6", value_task["criterion_ids"])
        self.assertNotIn("VAL-NODE-002", value_task["validations"])
        for validation in [
            "VAL-PLUGIN-HOST-001",
            "VAL-E2E-003",
            "VAL-WORKER-PLUGIN-001",
        ]:
            self.assertIn(validation, value_task["validations"])
        self.assertIn("crates/comfy_media/src/native_node_payload.rs", asset_writes)
        self.assertIn("crates/comfy_media/Cargo.toml", asset_writes)
        self.assertIn("crates/comfy_media/src/gaussian_splat.rs", asset_writes)
        self.assertIn("crates/comfy_nodes/src/execution.rs", asset_writes)
        self.assertIn("crates/comfy_nodes/src/stored_payload.rs", asset_writes)
        self.assertIn("crates/comfy_runtime/src/output_committer.rs", asset_writes)
        self.assertIn("crates/comfy_runtime/src/permissions.rs", asset_reads)
        self.assertIn("crates/comfy_runtime/src/permissions.rs", asset_writes)
        self.assertIn("crates/comfy_plugin_host/src/registry_adapter.rs", asset_writes)
        self.assertIn("crates/comfy_plugin_host/tests/component_contract.rs", asset_writes)
        self.assertIn("crates/comfy_test_support/tests/plugin_e2e.rs", asset_writes)
        asset_task = tasks_by_id["comfy-parity-native-node-asset-effect-foundation"]
        self.assertEqual(
            asset_task["validations"],
            ["VAL-DOMAIN-008", "VAL-NATIVE-E2E-001", "VAL-RECOVERY-005", "VAL-OWNERSHIP-001"],
        )
        self.assertNotIn("VAL-NODE-002", asset_task["validations"])
        self.assertIn("41.6", asset_task["criterion_ids"])
        self.assertNotIn("crates/comfy_runtime/src/providers.rs", provider_reads)
        self.assertIn("crates/comfy_runtime/src/trust.rs", provider_reads)
        self.assertIn("crates/comfy_runtime/src/permissions.rs", provider_reads)
        self.assertIn("crates/comfy_runtime/src/plugin_services.rs", provider_reads)
        self.assertIn(".agents/specs/comfy-parity/catalogs/backend-node-contracts.json", provider_reads)
        self.assertIn("crates/comfy_plugin_sdk/src/type_ids.rs", provider_writes)
        self.assertIn("crates/comfy_plugin_sdk/schema/plugin-manifest-v1.schema.json", provider_writes)
        self.assertIn("crates/comfy_plugin_host/src/comfy_plugin_host.rs", provider_writes)
        self.assertIn("crates/comfy_plugin_host/src/registry_adapter.rs", provider_writes)
        self.assertIn("crates/comfy_plugin_host/src/capabilities.rs", provider_writes)
        self.assertIn("crates/comfy_plugin_host/src/private_worker.rs", provider_writes)
        self.assertIn("crates/comfy_plugin_sdk/wit/comfy-plugin.wit", provider_writes)
        self.assertIn("crates/comfy_runtime/src/plugin_services.rs", provider_writes)
        self.assertIn("crates/comfy_runtime/src/provider_materialization.rs", provider_writes)
        self.assertIn("crates/comfy_runtime/src/runtime_supervisor.rs", provider_writes)
        self.assertIn("crates/comfy_runtime/src/prompt_compiler.rs", provider_writes)
        self.assertIn("crates/comfy_runtime/src/persistence.rs", provider_writes)
        self.assertIn("crates/comfy_worker/src/plugin_runtime.rs", provider_writes)
        self.assertIn("crates/comfy_api/src/security.rs", provider_writes)
        self.assertIn("crates/comfy_api/src/headless.rs", provider_writes)
        self.assertIn("crates/comfy_ui/src/execution_model.rs", provider_writes)
        self.assertIn("crates/sim/src/comfy_plugin_services.rs", provider_writes)
        self.assertIn("crates/sim/src/sim.rs", provider_writes)
        provider_task = tasks_by_id["comfy-parity-native-node-provider-invocation-foundation"]
        self.assertEqual(
            provider_task["criterion_ids"],
            [
                "4.3", "4.4", "4.5", "6.4", "6.5", "6.6", "12.5", "12.6",
                "28.3", "28.6", "32.1", "32.2", "32.5", "32.7", "32.8",
                "34.2", "34.6", "39.2", "39.3", "39.5", "39.6", "40.1",
                "40.4", "40.6", "41.5", "44.1", "44.2", "44.3",
            ],
        )
        self.assertNotIn("VAL-NODE-002", provider_task["validations"])
        self.assertNotIn("VAL-NATIVE-E2E-002", provider_task["validations"])
        for validation in [
            "VAL-NODE-001",
            "VAL-NODE-REGISTRY-001",
            "VAL-DOMAIN-004",
            "VAL-PLUGIN-HOST-001",
            "VAL-E2E-003",
            "VAL-WORKER-PLUGIN-001",
            "VAL-RUNTIME-TRUST-001",
            "VAL-NATIVE-API-001",
            "VAL-CANCEL-001",
            "VAL-NATIVE-E2E-001",
            "VAL-OWNERSHIP-001",
        ]:
            self.assertIn(validation, provider_task["validations"])
        build_boundary_task = tasks_by_id["comfy-parity-opt-in-product-build-boundary"]
        self.assertEqual(build_boundary_task["criterion_ids"], [
            "45.1", "45.2", "45.3", "45.4", "45.5", "45.6",
        ])
        self.assertEqual(
            build_boundary_task["validations"],
            ["VAL-COMFY-BUILD-001", "VAL-NATIVE-BOUNDARY-001", "VAL-OWNERSHIP-001"],
        )
        self.assertEqual(
            build_boundary_task["dependencies"],
            ["comfy-parity-native-node-asset-effect-foundation"],
        )
        for path in [
            "crates/sim/Cargo.toml",
            "crates/sim/src/main.rs",
            "crates/sim/src/sim.rs",
            "crates/sim/src/sim/app_menus.rs",
            "crates/extension_host/Cargo.toml",
            "crates/extension_host/src/extension_host.rs",
            "script/check-comfy-feature-boundary",
            "script/bundle-mac",
            "script/bundle-linux",
            "script/bundle-windows.ps1",
            "crates/sim/resources/windows/sim.iss",
            "crates/comfy_test_support/tests/native_release_boundary.rs",
            "crates/comfy_test_support/tests/ownership_consolidation.rs",
            ".agents/specs/comfy-parity/catalogs/native-backend-dependencies.json",
            ".agents/specs/comfy-parity/validate_backend_dependencies.py",
        ]:
            self.assertIn(path, build_boundary_task["writes"])
        for command in [
            "cargo check --locked -p sim --no-default-features",
            "cargo test --locked -p sim --no-default-features",
            "cargo check --locked -p sim --features comfy",
            "PYTHONDONTWRITEBYTECODE=1 python3 .agents/specs/comfy-parity/validate_backend_dependencies.py",
            "cargo test --locked -p comfy_test_support --test native_release_boundary val_native_boundary_001_packaged_release -- --exact --nocapture",
            "cargo test --locked -p comfy_test_support --test ownership_consolidation val_ownership_001 -- --exact --nocapture",
            "./script/check-comfy-feature-boundary",
            "./script/bundle-mac --dry-run",
            "./script/bundle-mac --comfy --dry-run",
            "./script/bundle-linux --dry-run",
            "./script/bundle-linux --comfy --dry-run",
            "pwsh -File script/bundle-windows.ps1 -DryRun",
            "pwsh -File script/bundle-windows.ps1 -Comfy -DryRun",
        ]:
            self.assertIn(
                command,
                planning.task_validation_commands(build_boundary_task),
            )
        generated_mapping = json.loads(
            (planning.CATALOGS / "native-spec-mapping.json").read_text(encoding="utf-8")
        )
        self.assertEqual(
            generated_mapping["feature_criterion_overrides"]["COMFY-DESKTOP-206"],
            ["45.1", "45.2", "45.3", "45.4", "45.5", "45.6"],
        )
        self.assertEqual(
            generated_mapping["feature_validation_overrides"]["COMFY-DESKTOP-206"],
            ["VAL-COMFY-BUILD-001"],
        )
        self.assertIn(
            "comfy-parity-opt-in-product-build-boundary",
            generated_mapping["special_feature_tasks"]["COMFY-DESKTOP-206"],
        )
        self.assertIn(
            "crates/comfy_runtime/src/native_execution_controller.rs", registry_writes
        )
        self.assertIn("crates/comfy_worker/src/comfy_worker.rs", registry_writes)
        self.assertIn("crates/comfy_api/src/services.rs", registry_writes)
        self.assertIn("crates/sim/src/sim.rs", registry_writes)

        schema_commands = planning.task_validation_commands(
            tasks_by_id["comfy-parity-native-node-schema-metadata-foundation"]
        )
        for command in [
            "cargo test --locked -p comfy_nodes val_node_001 -- --nocapture",
            "cargo test --locked -p comfy_nodes val_node_registry_001 -- --nocapture",
            "cargo test --locked -p comfy_runtime val_domain_004 -- --nocapture",
            "ownership_consolidation val_ownership_001 -- --exact --nocapture",
            "PYTHONDONTWRITEBYTECODE=1 python3 .agents/specs/comfy-parity/test_generate_node_contract_catalog.py",
            "PYTHONDONTWRITEBYTECODE=1 python3 .agents/specs/comfy-parity/test_regenerate_native_planning.py",
            "python3 .agents/specs/comfy-parity/regenerate_all.py --check-twice",
            "validate_spec.py .agents/specs/comfy-parity --require-complete",
        ]:
            self.assertIn(command, schema_commands)

        value_commands = planning.task_validation_commands(
            tasks_by_id["comfy-parity-native-node-compute-value-foundation"]
        )
        for command in [
            "cargo test --locked -p comfy_runtime val_domain_004 -- --nocapture",
            "cargo test --locked -p comfy_tensor val_tensor_001 -- --nocapture",
            "cargo test --locked -p comfy_model val_model_family_001 -- --nocapture",
            "native_image_e2e val_native_e2e_001 -- --exact --nocapture",
            "cargo test --locked -p comfy_model --lib clip_vision",
            "cargo test --locked -p comfy_model --lib raft_ -- --nocapture",
            "cargo test --locked -p comfy_model --lib controlnet -- --nocapture",
            "cargo test --locked -p comfy_sampler --lib native_node_payload",
            "cargo test --locked -p comfy_media --lib native_node_payload",
            "cargo test --locked -p comfy_plugin_sdk --lib type_ids -- --nocapture",
            "cargo test --locked -p comfy_test_support --test native_conditioning_integration -- --nocapture",
            "registry_adapter::tests::explicit_stored_variants_are_exhaustively_projected_or_rejected -- --exact",
            "val_ownership_001_native_stored_payload_boundary_is_closed",
            "PYTHONDONTWRITEBYTECODE=1 python3 .agents/specs/comfy-parity/test_regenerate_native_planning.py",
            "python3 .agents/specs/comfy-parity/regenerate_all.py --check-twice",
            "validate_spec.py .agents/specs/comfy-parity --require-complete",
        ]:
            self.assertIn(command, value_commands)
        asset_commands = planning.task_validation_commands(
            tasks_by_id["comfy-parity-native-node-asset-effect-foundation"]
        )
        for command in [
            "cargo test --locked -p comfy_media --lib native_node_payload -- --nocapture",
            "cargo test --locked -p comfy_media --lib gaussian_splat -- --nocapture",
            "cargo test --locked -p comfy_nodes --lib stored_payload::tests -- --nocapture",
            "cargo test --locked -p comfy_plugin_sdk --lib type_ids -- --nocapture",
            "compute_session_requires_the_contexts_exact_backend_and_scratch_binding -- --exact --nocapture",
            "native_asset_resolver_seals_paths_and_rejects_foreign_or_cancelled_reads -- --exact --nocapture",
            "native_prepared_effects_roll_back_before_node_failure_publication -- --exact --nocapture",
            "native_worker_result_maps_ui_outputs_through_a_bounded_wire_dto -- --exact --nocapture",
            "ipc_schema_contains_no_tensor_pointer_path_or_plugin_handle -- --exact --nocapture",
            "native_prompt_literals_reject_nested_process_local_handles -- --exact --nocapture",
            "cargo test --locked -p comfy_runtime val_domain_004 -- --nocapture",
            "cargo test --locked -p comfy_plugin_host --lib registry_adapter -- --nocapture",
            "cargo test --locked -p comfy_plugin_host --test component_contract -- --nocapture",
            "plugin_e2e val_plugin_host_001 -- --exact --nocapture",
            "plugin_e2e val_e2e_003 -- --exact --nocapture",
            "plugin_e2e val_worker_plugin_001 -- --exact --nocapture",
            "native_image_e2e val_native_e2e_001 -- --exact --nocapture",
            "cargo test --locked -p comfy_runtime val_domain_008 -- --nocapture",
            "filesystem_asset_recovery val_recovery_005 -- --nocapture",
            "PYTHONDONTWRITEBYTECODE=1 python3 .agents/specs/comfy-parity/test_regenerate_native_planning.py",
            "python3 .agents/specs/comfy-parity/regenerate_all.py --check-twice",
            "validate_spec.py .agents/specs/comfy-parity --require-complete",
        ]:
            self.assertIn(command, asset_commands)
        provider_commands = planning.task_validation_commands(
            tasks_by_id["comfy-parity-native-node-provider-invocation-foundation"]
        )
        for command in [
            "cargo test --locked -p comfy_nodes val_node_001 -- --nocapture",
            "cargo test --locked -p comfy_nodes val_node_registry_001 -- --nocapture",
            "cargo test --locked -p comfy_plugin_sdk --lib type_ids -- --nocapture",
            "cargo test --locked -p comfy_runtime val_domain_004 -- --nocapture",
            "cargo test --locked -p comfy_runtime val_runtime_trust_001 -- --nocapture",
            "cargo test --locked -p comfy_runtime --lib provider_activation -- --nocapture",
            "cargo test --locked -p comfy_plugin_host --lib registry_adapter -- --nocapture",
            "cargo test --locked -p comfy_plugin_host --test component_contract -- --nocapture",
            "val_plugin_host_001 -- --exact --nocapture",
            "val_e2e_003 -- --exact --nocapture",
            "val_worker_plugin_001 -- --exact --nocapture",
            "cargo test --locked -p comfy_api val_native_api_001 -- --nocapture",
            "cargo test --locked -p comfy_test_support val_cancel_001 -- --nocapture",
            "native_image_e2e val_native_e2e_001 -- --exact --nocapture",
            "ownership_consolidation val_ownership_001 -- --exact --nocapture",
            "PYTHONDONTWRITEBYTECODE=1 python3 .agents/specs/comfy-parity/test_regenerate_native_planning.py",
            "python3 .agents/specs/comfy-parity/regenerate_all.py --check-twice",
            "validate_spec.py .agents/specs/comfy-parity --require-complete",
        ]:
            self.assertIn(command, provider_commands)

    def test_catalog_pass_signal_is_command_only_and_other_artifact_classes_remain(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with patch.object(planning, "ROOT", root):
                planning.write_validation()
            lines = (root / "validation.md").read_text(encoding="utf-8").splitlines()

        scenarios: dict[str, list[str]] = {}
        current: str | None = None
        for line in lines:
            if line.startswith("### VAL-"):
                current = line.removeprefix("### ").split(":", 1)[0]
                scenarios[current] = [line]
            elif current is not None:
                scenarios[current].append(line)

        self.assertEqual(set(scenarios), set(planning.VALIDATIONS))

        def scenario_line(identifier: str, prefix: str) -> str:
            matches = [line for line in scenarios[identifier] if line.startswith(prefix)]
            self.assertEqual(len(matches), 1, identifier)
            return matches[0]

        catalog = "\n".join(scenarios["VAL-CATALOG-001"])
        self.assertEqual(
            scenario_line("VAL-CATALOG-001", "- Command/runner: "),
            "- Command/runner: `python3 .agents/specs/comfy-parity/regenerate_all.py --check-twice`.",
        )
        self.assertEqual(
            scenario_line("VAL-CATALOG-001", "- Pass artifact: "),
            "- Pass artifact: exit status 0 from the exact runner after the "
            "source-snapshot manifest matches and both complete regeneration passes "
            "produce no changed paths. The checked-in generated outputs and command "
            "result are the evidence; this command-only gate emits no separate target "
            "JSON artifact.",
        )
        self.assertNotIn("target/comfy-parity/val-catalog-001.json", catalog)
        command_only = [
            identifier
            for identifier in planning.VALIDATIONS
            if "command-only gate emits no separate target JSON artifact"
            in scenario_line(identifier, "- Pass artifact: ")
        ]
        self.assertEqual(command_only, ["VAL-CATALOG-001"])

        generic = scenario_line("VAL-CANCEL-001", "- Pass artifact: ")
        self.assertIn("target/comfy-parity/val-cancel-001.json", generic)
        self.assertIn("fixture digests", generic)

        cumulative = scenario_line("VAL-CLIP-001", "- Pass artifact: ")
        self.assertIn("target/comfy-parity/val-clip-001.json", cumulative)
        self.assertIn("using schema version 1", cumulative)
        self.assertIn("partial artifacts claim only their exact passed rows", cumulative)

        device = scenario_line("VAL-DEVICE-001", "- Pass artifact: ")
        self.assertIn("target/comfy-parity/val-device-001.json", device)
        self.assertIn("Apple Metal baseline retains its signed artifact", device)

        model_family = scenario_line(
            "VAL-MODEL-FAMILY-ROW-001", "- Pass artifact: "
        )
        self.assertIn("target/comfy-parity/val-model-family-row-001/", model_family)
        self.assertIn("one deterministic artifact per executed fixture", model_family)


if __name__ == "__main__":
    unittest.main()
