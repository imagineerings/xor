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
        inherited_v3_presentation_id = (
            "comfy-parity-native-node-inherited-v3-presentation-catalog-correction"
        )
        v3_presentation_catalog_closure_id = (
            "comfy-parity-native-node-v3-presentation-catalog-closure"
        )
        dependency_ledger_lock_repair_id = (
            "comfy-parity-native-backend-dependency-ledger-current-lock-repair"
        )
        value_id = "comfy-parity-native-node-compute-value-foundation"
        latent_bundle_id = "comfy-parity-native-latent-bundle-foundation"
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

        self.assertEqual(len(tasks), 722)
        self.assertEqual(len(node_ids), 102)
        self.assertEqual(tasks_by_id[foundation_id]["dependencies"], [compute_id])
        for identifier in (schema_id, value_id, asset_id, provider_id):
            self.assertTrue(tasks_by_id[identifier]["feature_scoped"])
        self.assertEqual(tasks_by_id[schema_id]["dependencies"], [foundation_id])
        self.assertEqual(
            tasks_by_id[inherited_v3_presentation_id]["dependencies"], [schema_id]
        )
        self.assertTrue(tasks_by_id[inherited_v3_presentation_id]["locked"])
        self.assertTrue(tasks_by_id[inherited_v3_presentation_id]["feature_scoped"])
        self.assertIn(
            ".agents/specs/comfy-parity/catalogs/backend-nodes.csv",
            tasks_by_id[inherited_v3_presentation_id]["writes"],
        )
        self.assertIn(
            "crates/comfy_nodes/src/registry_generator.rs",
            tasks_by_id[inherited_v3_presentation_id]["writes"],
        )
        self.assertIn(
            "COMFY-NODE-0542 and COMFY-NODE-0543",
            tasks_by_id[inherited_v3_presentation_id]["done"],
        )
        self.assertEqual(
            tasks_by_id[v3_presentation_catalog_closure_id]["dependencies"],
            [inherited_v3_presentation_id],
        )
        self.assertTrue(tasks_by_id[v3_presentation_catalog_closure_id]["locked"])
        self.assertTrue(tasks_by_id[v3_presentation_catalog_closure_id]["feature_scoped"])
        self.assertIn(
            "COMFY-NODE-0047",
            tasks_by_id[v3_presentation_catalog_closure_id]["done"],
        )
        self.assertIn(
            "every non-null portable display name",
            tasks_by_id[v3_presentation_catalog_closure_id]["done"],
        )
        self.assertIn(
            "crates/comfy_nodes/src/provider_contracts.rs",
            tasks_by_id[v3_presentation_catalog_closure_id]["writes"],
        )
        center_crop_commands = planning.task_validation_commands(
            tasks_by_id[v3_presentation_catalog_closure_id]
        )
        for command in (
            "generated_registry_is_comprehensive_and_preserves_union_frontend_types",
            "test_generate_node_contract_catalog.py",
            "regenerate_all.py --check-twice",
        ):
            self.assertIn(command, center_crop_commands)
        dependency_ledger_task = tasks_by_id[dependency_ledger_lock_repair_id]
        self.assertEqual(
            dependency_ledger_task["dependencies"],
            ["comfy-parity-provider-runtime-component-activation-preflight-foundation"],
        )
        self.assertEqual(
            dependency_ledger_task["writes"],
            [
                ".agents/specs/comfy-parity/catalogs/native-backend-dependencies.json",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertNotIn("Cargo.lock", dependency_ledger_task["writes"])
        self.assertIn(
            "native_foundation",
            planning.task_validation_commands(dependency_ledger_task),
        )
        self.assertEqual(
            tasks_by_id[value_id]["dependencies"],
            [
                v3_presentation_catalog_closure_id,
                compute_id,
                "comfy-parity-model-detection-any-of-key-selector-consolidation",
            ],
        )
        model_resource_phases = [
            "comfy-parity-native-vae-resource-foundation",
            "comfy-parity-native-clip-resource-foundation",
            "comfy-parity-native-multiaxis-rope-attention-foundation",
            "comfy-parity-native-family-denoiser-invocation-foundation",
            "comfy-parity-native-family-execution-projection-binding-foundation",
            "comfy-parity-native-family-model-resource-foundation",
            "comfy-parity-native-audio-encoder-resource-foundation",
            "comfy-parity-spandrel-source-snapshots-user-authority-gate",
            "comfy-parity-native-spandrel-image-model-contract-foundation",
            "comfy-parity-native-upscale-runtime-contract-foundation",
            "comfy-parity-native-upscale-model-resource-foundation",
            "comfy-parity-native-latent-upscale-model-resource-foundation",
            "comfy-parity-native-attention-ordered-additive-mask-foundation",
            "comfy-parity-native-background-removal-resource-foundation",
            "comfy-parity-native-depth-anything-3-resource-foundation",
            "comfy-parity-native-moge-resource-foundation",
            "comfy-parity-native-conditioning-auxiliary-resource-foundation",
            "comfy-parity-native-model-resource-service-foundation",
        ]
        self.assertEqual(
            tasks_by_id[model_resource_phases[0]]["dependencies"],
            [
                value_id,
                "comfy-parity-native-asset-name-resolution-foundation",
                "comfy-parity-model-detection-any-of-key-selector-consolidation",
            ],
        )
        clip_task = tasks_by_id["comfy-parity-native-clip-resource-foundation"]
        self.assertIn("crates/comfy_model/src/clip_tokenizer.rs", clip_task["writes"])
        self.assertIn(
            "crates/comfy_model/src/clip_text_encoder_multimodal.rs",
            clip_task["writes"],
        )
        self.assertIn("crates/comfy_model/src/model_family.rs", clip_task["writes"])
        self.assertIn(
            "crates/comfy_model/src/native_node_payload.rs", clip_task["writes"]
        )
        family_task = tasks_by_id[
            "comfy-parity-native-family-model-resource-foundation"
        ]
        family_invocation_task = tasks_by_id[
            "comfy-parity-native-family-denoiser-invocation-foundation"
        ]
        family_projection_task = tasks_by_id[
            "comfy-parity-native-family-execution-projection-binding-foundation"
        ]
        self.assertEqual(
            family_invocation_task["writes"],
            [
                "crates/comfy_model/src/model_family.rs",
                "crates/comfy_model/src/families/auraflow_comfy_model_0064.rs",
                "crates/comfy_model/src/families/qwenimage_comfy_model_0113.rs",
                "crates/comfy_test_support/tests/native_family_model_invocation.rs",
                "crates/comfy_test_support/fixtures/models/native-family-denoiser-invocation-foundation/generate_oracle.py",
            ],
        )
        self.assertIn("tracked pure-standard-library oracle generator", family_invocation_task["done"])
        self.assertIn("unary forward-checkpoint fallback", family_invocation_task["done"])
        self.assertEqual(
            family_projection_task["writes"],
            [
                "crates/comfy_model/src/model_family.rs",
                "crates/comfy_model/src/families/auraflow_comfy_model_0064.rs",
                "crates/comfy_model/src/families/qwenimage_comfy_model_0113.rs",
                ".agents/specs/comfy-parity/catalogs/native-model-family-closure.json",
                "crates/comfy_test_support/tests/native_family_execution_projection.rs",
                "crates/comfy_test_support/fixtures/models/native-family-execution-projection-foundation",
            ],
        )
        self.assertIn(
            "Qwen production admission retains its 3072-wide contract",
            family_projection_task["done"],
        )
        self.assertIn(
            "execution-projection descriptors", family_projection_task["done"]
        )
        self.assertEqual(
            family_task["dependencies"],
            [
                "comfy-parity-native-family-execution-projection-binding-foundation",
                "comfy-parity-model-detection-any-of-key-selector-consolidation",
            ],
        )
        for path in (
            "crates/comfy_model/src/conditioning.rs",
            "crates/comfy_sampler/src/algorithms/native_diffusion.rs",
        ):
            self.assertIn(path, family_task["reads"])
        for path in (
            "crates/comfy_sampler/src/sampling_profile.rs",
            "crates/comfy_sampler/src/algorithms/native_diffusion.rs",
            ".agents/specs/comfy-parity/catalogs/native-compute-closure.json",
            "crates/comfy_model/tests/clip_tokenizer.rs",
            "crates/comfy_sampler/tests/ownership.rs",
            "crates/comfy_sampler/tests/sampling_foundation.rs",
            "crates/comfy_test_support/fixtures/models/native-family-model-resource-foundation",
        ):
            self.assertIn(path, family_task["writes"])
        self.assertIn("tracked pure-standard-library oracle", family_task["done"])
        self.assertIn("projection identities", family_task["done"])
        self.assertIn(
            "remain assigned to comfy-parity-native-model-resource-execution-foundation",
            family_task["done"],
        )
        self.assertNotIn("stale handles fail atomically", family_task["done"])
        latent_upscale_task = tasks_by_id[
            "comfy-parity-native-latent-upscale-model-resource-foundation"
        ]
        depth_anything_task = tasks_by_id[
            "comfy-parity-native-depth-anything-3-resource-foundation"
        ]
        for path in (
            "projects/comfy/ComfyUI/comfy/ldm/depth_anything_3/model.py",
            "projects/comfy/ComfyUI/comfy/ldm/depth_anything_3/preprocess.py",
            "projects/comfy/ComfyUI/comfy/ldm/depth_anything_3/dpt.py",
            "projects/comfy/ComfyUI/comfy/ldm/depth_anything_3/camera.py",
            "projects/comfy/ComfyUI/comfy/ldm/depth_anything_3/ray_pose.py",
            "projects/comfy/ComfyUI/comfy/ldm/depth_anything_3/reference_view_selector.py",
            "projects/comfy/ComfyUI/comfy/ldm/depth_anything_3/transform.py",
            "projects/comfy/ComfyUI/comfy/image_encoders/dino2.py",
            "projects/comfy/ComfyUI/comfy/model_detection.py",
            "crates/comfy_tensor/src/ops/linear_algebra_01.rs",
            "crates/comfy_tensor/src/ops/linear_algebra_02.rs",
            "crates/comfy_tensor/src/ops/random_number_generation_01.rs",
            "crates/comfy_tensor/src/ops/external_tensor_kernel_01.rs",
            "crates/comfy_tensor/src/ops/spatial_functional_kernel_01.rs",
        ):
            self.assertIn(path, depth_anything_task["reads"])
        self.assertIn(
            "crates/comfy_test_support/fixtures/models/depth-anything-3-resource-foundation",
            depth_anything_task["writes"],
        )
        self.assertIn(
            "pure-standard-library source-equation oracle", depth_anything_task["done"]
        )
        self.assertIn(
            "versioned RANSAC random-number phase", depth_anything_task["done"]
        )
        self.assertIn("storage-to-F32 execution projection", depth_anything_task["done"])
        self.assertIn("instead of duplicating kernels", depth_anything_task["done"])
        self.assertIn("model-detection/configuration owner", depth_anything_task["done"])
        depth_anything_validation = planning.task_validation_commands(
            depth_anything_task
        )
        for command in (
            "generate_oracle.py --check",
            "cargo test --locked -p comfy_model depth_anything_3",
            "native_node_family_e2e depth_anything_3",
        ):
            self.assertIn(command, depth_anything_validation)
        for path in (
            "projects/comfy/ComfyUI/comfy_extras/nodes_lt_upsampler.py",
            "projects/comfy/ComfyUI/comfy/ldm/hunyuan_video/upsampler.py",
            "projects/comfy/ComfyUI/comfy/ldm/lightricks/latent_upsampler.py",
            "projects/comfy/ComfyUI/comfy/ldm/lightricks/vae/causal_video_autoencoder.py",
            "crates/comfy_model/src/vae_video.rs",
            "crates/comfy_tensor/src/native_node_payload.rs",
            "crates/comfy_tensor/src/ops/spatial_functional_kernel_01.rs",
            "crates/comfy_tensor/src/ops/shape_layout_transform_01.rs",
            "crates/comfy_tensor/src/ops/shape_layout_transform_02.rs",
            "crates/comfy_tensor/src/ops/neural_network_functional_01.rs",
            "crates/comfy_tensor/src/ops/activation_normalization_functional_01.rs",
            "crates/comfy_tensor/src/ops/indexing_masking_01.rs",
            "crates/comfy_tensor/src/ops/reduction_02.rs",
            "crates/comfy_tensor/src/ops/storage_dtype_device_01.rs",
            "crates/comfy_tensor/src/ops/tensor_creation_01.rs",
            "crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_02.rs",
            "crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_03.rs",
            "crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_05.rs",
            "crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_08.rs",
            "crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_09.rs",
            "crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_13.rs",
            "crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_18.rs",
            "crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_21.rs",
            "crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_22.rs",
            ".agents/specs/comfy-parity/ownership-policy.json",
            ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
            "crates/comfy_test_support/tests/ownership_consolidation.rs",
        ):
            self.assertIn(path, latent_upscale_task["reads"])
        for path in (
            "crates/comfy_model/src/latent_upscale_model.rs",
            "crates/comfy_model/src/native_node_payload.rs",
            "crates/comfy_model/src/vae.rs",
            "crates/comfy_model/src/vae_video.rs",
            "crates/comfy_tensor/src/ops/spatial_functional_kernel_01.rs",
            "crates/comfy_test_support/src/bin/generate_latent_upscale_model_fixture.rs",
            "crates/comfy_test_support/fixtures/models/latent-upscale-model-resource-foundation",
            ".agents/specs/comfy-parity/ownership-policy.json",
            ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
            "crates/comfy_test_support/tests/ownership_consolidation.rs",
        ):
            self.assertIn(path, latent_upscale_task["writes"])
        for phrase in (
            "Hunyuan 720p",
            "Hunyuan 1080p",
            "LTX",
            "720p before 1080p before LTX",
            "past-only temporal and replicate spatial padding",
            "x + conv3(SiLU(conv2(SiLU(conv1(x)))))",
            "exactly three ordered ResnetBlocks",
            "normalize(x,dim=1)*sqrt(C)*gamma",
            "PixelShuffleND",
            "[1, 4, 6, 4, 1]",
            "Independent reduced raw-output oracles",
            "NativeLatentUpscaleModelResource alone owns",
            "conservative phase-memory OOM",
            "bislerp",
            "nearest-exact (distinct from nearest)",
            "identity aliasing of the original latent bundle",
            "fresh CPU-F32 samples-only bundle",
            "zero norm, coincident, antipodal, and rounding-sensitive",
            "noise_mask",
            "CPU F32",
        ):
            self.assertIn(phrase, latent_upscale_task["done"])
        latent_commands = planning.task_validation_commands(latent_upscale_task)
        for command in (
            "cargo run --locked -p comfy_test_support --bin generate_latent_upscale_model_fixture -- --check",
            "cargo test --locked -p comfy_model latent_upscale_model -- --nocapture",
            "cargo test --locked -p comfy_tensor --test spatial_functional_kernel_01 -- --nocapture",
            "cargo test --locked -p comfy_test_support --test native_node_family_e2e -- --nocapture",
            "cargo test --locked -p comfy_test_support --test ownership_consolidation val_ownership_001 -- --exact --nocapture",
            "python3 .agents/specs/comfy-parity/regenerate_all.py --check-twice",
            "python3 .agents/skills/feature-spec/scripts/validate_spec.py .agents/specs/comfy-parity --require-complete",
            "git diff --check",
        ):
            self.assertIn(command, latent_commands)
        background_removal_task = tasks_by_id[
            "comfy-parity-native-background-removal-resource-foundation"
        ]
        ordered_attention_task = tasks_by_id[
            "comfy-parity-native-attention-ordered-additive-mask-foundation"
        ]
        self.assertEqual(
            ordered_attention_task["dependencies"],
            ["comfy-parity-native-latent-upscale-model-resource-foundation"],
        )
        self.assertEqual(
            ordered_attention_task["writes"],
            [
                "crates/comfy_tensor/src/ops/accelerated_attention_kernel_01.rs",
                "crates/comfy_tensor/tests/accelerated_attention_kernel_01.rs",
                "crates/comfy_tensor/tests/ops/accelerated_attention_kernel_01.rs",
                "crates/comfy_model/src/attention.rs",
            ],
        )
        for phrase in (
            "append-only OrderedAdditive",
            "(score + first) + second",
            "score=1e20",
            "precombined mask would produce 0",
            "owns no attention equation",
        ):
            self.assertIn(phrase, ordered_attention_task["done"])
        self.assertEqual(
            background_removal_task["dependencies"],
            [
                "comfy-parity-native-attention-ordered-additive-mask-foundation",
                "comfy-parity-model-detection-any-of-key-selector-consolidation",
            ],
        )
        for path in (
            "projects/comfy/ComfyUI/comfy/bg_removal_model.py",
            "projects/comfy/ComfyUI/comfy/background_removal/birefnet.py",
            "projects/comfy/ComfyUI/comfy/background_removal/birefnet.json",
            "projects/comfy/ComfyUI/comfy/clip_model.py",
            "crates/comfy_tensor/src/ops/spatial_functional_kernel_01.rs",
            "crates/comfy_tensor/src/ops/external_tensor_kernel_02.rs",
        ):
            self.assertIn(path, background_removal_task["reads"])
        self.assertIn(
            "crates/comfy_test_support/fixtures/models/background-removal-resource-foundation",
            background_removal_task["writes"],
        )
        self.assertIn(
            "comfy-parity-native-model-resource-execution-foundation",
            background_removal_task["done"],
        )
        self.assertNotIn(
            "residency, cache, persistence, and restart behavior",
            background_removal_task["done"],
        )
        self.assertIn(
            "crates/comfy_sampler/src/native_node_payload.rs", family_task["writes"]
        )
        self.assertIn("crates/comfy_sampler/src/guidance.rs", family_task["writes"])
        self.assertIn(
            "crates/comfy_runtime/src/native_execution_controller.rs",
            family_task["writes"],
        )
        self.assertIn("positive and negative conditioning", family_task["done"])
        audio_id = "comfy-parity-native-audio-encoder-resource-foundation"
        latent_upscale_id = (
            "comfy-parity-native-latent-upscale-model-resource-foundation"
        )
        execution_id = "comfy-parity-native-model-resource-execution-foundation"
        stored_payload = "crates/comfy_nodes/src/stored_payload.rs"
        self.assertNotIn(stored_payload, tasks_by_id[audio_id]["writes"])
        for required_read in [
            "crates/comfy_model/src/model_family.rs",
            "crates/comfy_model/src/attention.rs",
            "crates/comfy_tensor/src/ops/reduction_01.rs",
            "crates/comfy_tensor/src/ops/shape_layout_transform_03.rs",
            "crates/comfy_tensor/src/ops/indexing_masking_01.rs",
            "crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_02.rs",
            "crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_05.rs",
            "crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_11.rs",
            "crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_18.rs",
            ".agents/specs/comfy-parity/ownership-policy.json",
            ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
            "crates/comfy_test_support/tests/ownership_consolidation.rs",
        ]:
            self.assertIn(required_read, tasks_by_id[audio_id]["reads"])
        for required_write in [
            "crates/comfy_model/tests/audio_encoder.rs",
            "crates/comfy_test_support/fixtures/models/audio-encoder-resource-foundation",
            ".agents/specs/comfy-parity/ownership-policy.json",
            ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
            "crates/comfy_test_support/tests/ownership_consolidation.rs",
        ]:
            self.assertIn(required_write, tasks_by_id[audio_id]["writes"])
        self.assertNotIn(
            "crates/comfy_test_support/tests/native_node_family_e2e.rs",
            tasks_by_id[audio_id]["writes"],
        )
        for phrase in [
            "Wav2Vec2 marker precedence",
            "post-resample audio_samples",
            "unexpected nonexecuting state",
            "zero-sample audio",
            "CPU F32",
            "comfy_model::audio_encoder::NativeAudioEncoder",
        ]:
            self.assertIn(phrase, tasks_by_id[audio_id]["done"])
        spandrel_id = "comfy-parity-native-spandrel-image-model-contract-foundation"
        spandrel_gate_id = "comfy-parity-spandrel-source-snapshots-user-authority-gate"
        spandrel_runtime_id = "comfy-parity-native-upscale-runtime-contract-foundation"
        upscale_resource_id = "comfy-parity-native-upscale-model-resource-foundation"
        spandrel_gate = tasks_by_id[spandrel_gate_id]
        self.assertEqual(spandrel_gate["dependencies"], [audio_id])
        self.assertEqual(tasks_by_id[spandrel_id]["dependencies"], [spandrel_gate_id])
        self.assertEqual(
            tasks_by_id[spandrel_runtime_id]["dependencies"],
            [spandrel_id, "comfy-parity-model-detection-any-of-key-selector-consolidation"],
        )
        self.assertEqual(
            tasks_by_id[upscale_resource_id]["dependencies"],
            [
                spandrel_runtime_id,
                "comfy-parity-model-detection-any-of-key-selector-consolidation",
            ],
        )
        self.assertTrue(spandrel_gate["locked"])
        self.assertTrue(spandrel_gate["feature_scoped"])
        self.assertEqual(spandrel_gate["writes"], [])
        self.assertIn("user-approved immutable", spandrel_gate["outcome"])
        self.assertIn("No symlink", spandrel_gate["done"])
        for required_read in [
            "projects/comfy/Spandrel",
            "projects/comfy/spandrel-extra-arches",
            ".agents/specs/comfy-parity/baseline.md",
            ".agents/specs/comfy-parity/regenerate_all.py",
            ".agents/specs/comfy-parity/regenerate_native_planning.py",
            ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
        ]:
            self.assertIn(required_read, tasks_by_id[spandrel_id]["reads"])
        for required_write in [
            ".agents/specs/comfy-parity/baseline.md",
            ".agents/specs/comfy-parity/regenerate_all.py",
            ".agents/specs/comfy-parity/regenerate_native_planning.py",
            ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            "crates/comfy_test_support/tests/spandrel_image_model_contract.rs",
        ]:
            self.assertIn(required_write, tasks_by_id[spandrel_id]["writes"])
        for phrase in [
            "user-supplied",
            "code-license and model-use dispositions",
            "without importing Python",
            "exactly three optional-extra outcomes",
            "canonical regeneration pipeline",
            "one disjoint implementation leaf per materially distinct admitted equation family",
            "comfy-parity-native-upscale-model-resource-foundation stable ID remains the final integration task",
        ]:
            self.assertIn(phrase, tasks_by_id[spandrel_id]["done"])
        self.assertEqual(
            tasks_by_id[spandrel_runtime_id]["writes"],
            [
                "crates/comfy_model/src/upscale_contract.rs",
                "crates/comfy_model/src/comfy_model.rs",
                "crates/comfy_model/tests/upscale_contract.rs",
            ],
        )
        for phrase in [
            "every generated ordinal",
            "license disposition",
            "zero-admission catalog",
            "no Python or Spandrel import",
        ]:
            self.assertIn(phrase, tasks_by_id[spandrel_runtime_id]["done"])
        generated_upscale_leaves = [
            identifier
            for identifier in tasks_by_id
            if identifier.startswith("comfy-parity-native-upscale-equation-")
        ]
        self.assertEqual(generated_upscale_leaves, [])
        self.assertIn("zero-admission", tasks_by_id[upscale_resource_id]["outcome"])
        self.assertIn(
            "crates/comfy_model/src/upscale_contract.rs",
            tasks_by_id[upscale_resource_id]["reads"],
        )
        self.assertIn(
            "missing individual-license or reference-only",
            tasks_by_id[upscale_resource_id]["done"],
        )
        self.assertIn(
            "before tensor payload, device, workspace, or resource allocation",
            tasks_by_id[upscale_resource_id]["done"],
        )
        self.assertNotIn(stored_payload, tasks_by_id[latent_upscale_id]["writes"])
        self.assertIn(stored_payload, tasks_by_id[execution_id]["reads"])
        self.assertIn(stored_payload, tasks_by_id[execution_id]["writes"])
        writers = [
            identifier
            for identifier in [*model_resource_phases, execution_id]
            if stored_payload in tasks_by_id[identifier]["writes"]
        ]
        self.assertEqual(writers, [execution_id])
        self.assertIn(execution_id, tasks_by_id[audio_id]["done"])
        self.assertIn(execution_id, tasks_by_id[latent_upscale_id]["done"])
        for required in [
            "AUDIO_ENCODER",
            "LATENT_UPSCALE_MODEL",
            "persisted",
            "restart",
            "stale",
        ]:
            self.assertIn(required, tasks_by_id[execution_id]["done"])
        video_phases = [
            "comfy-parity-native-video-codec-general-abi-foundation",
            "comfy-parity-native-video-codec-package-bootstrap-foundation",
            "comfy-parity-native-video-demux-decode-foundation",
            "comfy-parity-native-video-source-slice-materialization-foundation",
            "comfy-parity-native-video-save-remux-audio-effects-foundation",
        ]
        self.assertEqual(
            tasks_by_id[video_phases[0]]["dependencies"],
            [
                "comfy-parity-native-video-component-h264-mp4-10bit-backing-foundation",
                v3_presentation_catalog_closure_id,
                dependency_ledger_lock_repair_id,
            ],
        )
        for previous, current in zip(video_phases, video_phases[1:]):
            self.assertEqual(tasks_by_id[current]["dependencies"], [previous])
        video_package_task = tasks_by_id[
            "comfy-parity-native-video-codec-package-bootstrap-foundation"
        ]
        for path in (
            "crates/comfy_runtime/src/native_video_codec_abi.rs",
            "crates/comfy_runtime/abi/video-codec/ffmpeg-7.1-x86_64-gnu-general-video-v1.json",
            "crates/comfy_model/src/artifact_index.rs",
            "crates/comfy_media/src/video.rs",
            "crates/comfy_runtime/src/native_execution_controller.rs",
            "crates/comfy_runtime/src/executor.rs",
            "crates/comfy_nodes/src/execution.rs",
            "crates/comfy_test_support/fixtures/video/codec-general-video-abi/manifest.json",
            "crates/comfy_test_support/fixtures/video/codec-dependency-closure/manifest.json",
            "crates/comfy_test_support/fixtures/video/codec-retained-loader/manifest.json",
        ):
            self.assertIn(path, video_package_task["reads"])
        for path in (
            "crates/comfy_runtime/src/trust.rs",
            "crates/comfy_test_support/src/bin/generate_video_codec_package_bootstrap_fixture.rs",
            "crates/comfy_test_support/tests/video_codec_package_bootstrap.rs",
            "crates/comfy_test_support/fixtures/video/codec-package-bootstrap",
        ):
            self.assertIn(path, video_package_task["writes"])
        for phrase in (
            "six-library/78-symbol catalog",
            "ReviewedGeneralVideoCodecDeclarations alone remains UncertifiedFfi",
            "historical five-library/54-symbol catalog",
            "Fixture signing keys and roots are test-only",
        ):
            self.assertIn(phrase, video_package_task["done"])
        video_package_validation = planning.task_validation_commands(
            video_package_task
        )
        for command in (
            "generate_video_codec_package_bootstrap_fixture -- --check",
            "general_video_codec_package_bootstrap",
            "--test video_codec_package_bootstrap",
            "-p comfy_worker video_codec_package_bootstrap",
            "val_ownership_task558_video_codec_package_bootstrap_001",
        ):
            self.assertIn(command, video_package_validation)
        video_closure = tasks_by_id["comfy-parity-native-video-execution-foundation"]
        self.assertEqual(
            video_closure["dependencies"],
            [
                video_phases[-1],
                "comfy-parity-native-nodes-model-loaders-comfy-node-0012",
            ],
        )
        self.assertNotIn("Cargo.toml", video_closure["writes"])
        self.assertNotIn(
            "crates/comfy_runtime/src/output_committer.rs", video_closure["writes"]
        )
        provider_shared_ids = [
            "comfy-parity-provider-contract-catalog-closure",
            "comfy-parity-provider-namespace-binding-projection",
            "comfy-parity-provider-streaming-component-abi-v2",
            "comfy-parity-provider-streaming-component-abi-v2-request-authority-repair",
            "comfy-parity-provider-worker-stream-protocol",
            "comfy-parity-provider-worker-stream-protocol-clippy-correction",
            "comfy-parity-provider-runtime-stream-progress-foundation",
            "comfy-parity-provider-streaming-component-abi-v2-invocation-input-repair",
            "comfy-parity-provider-runtime-component-activation-preflight-foundation",
            "comfy-parity-provider-runtime-worker-context-preflight-repair",
            "comfy-parity-provider-component-host-stream-adapter",
            "comfy-parity-provider-worker-stream-bridge",
            "comfy-parity-provider-deployment-lifecycle",
            "comfy-parity-provider-hermetic-component-harness",
        ]
        provider_vendor_ids = [
            f"comfy-parity-provider-component-{vendor}"
            for vendor, _, _, _ in planning.PROVIDER_VENDOR_SPECS
        ]
        provider_gate_id = (
            "comfy-parity-provider-signed-deployment-registry-user-authority-gate"
        )
        provider_closure_id = (
            "comfy-parity-native-partner-provider-components-foundation"
        )
        self.assertEqual(len(provider_shared_ids), 14)
        self.assertEqual(len(provider_vendor_ids), 33)
        provider_projection = tasks_by_id[
            "comfy-parity-provider-namespace-binding-projection"
        ]
        self.assertIn(
            "crates/comfy_nodes/src/families/partner_three_d_02.rs",
            provider_projection["reads"],
        )
        self.assertIn(
            "crates/comfy_nodes/src/families/partner_three_d_03.rs",
            provider_projection["reads"],
        )
        self.assertIn(
            "crates/comfy_nodes/src/families/partner_three_d_02.rs",
            provider_projection["writes"],
        )
        provider_streaming = tasks_by_id[
            "comfy-parity-provider-streaming-component-abi-v2"
        ]
        self.assertIn(
            "crates/comfy_plugin_host/src/comfy_plugin_host.rs",
            provider_streaming["reads"],
        )
        self.assertIn(
            "crates/comfy_plugin_host/src/comfy_plugin_host.rs",
            provider_streaming["writes"],
        )
        self.assertIn(
            "crates/comfy_plugin_sdk/wit/provider-v2/comfy-provider-plugin.wit",
            provider_streaming["writes"],
        )
        self.assertIn(
            "crates/comfy_plugin_sdk/wit/provider-v2/deps/comfy-plugin/comfy-plugin.wit",
            provider_streaming["writes"],
        )
        self.assertIn("comfy_plugin_host", provider_streaming["validation_packages"])
        self.assertIn("compile-time host bindgen", provider_streaming["done"])
        provider_request_authority = tasks_by_id[
            "comfy-parity-provider-streaming-component-abi-v2-request-authority-repair"
        ]
        self.assertEqual(
            provider_request_authority["dependencies"],
            ["comfy-parity-provider-streaming-component-abi-v2"],
        )
        self.assertNotIn(
            "crates/comfy_plugin_sdk/wit/comfy-plugin.wit",
            provider_request_authority["writes"],
        )
        self.assertNotIn(
            "crates/comfy_runtime/src/trust.rs", provider_request_authority["writes"]
        )
        self.assertNotIn(
            "crates/comfy_plugin_sdk/src/type_ids.rs",
            provider_request_authority["writes"],
        )
        self.assertNotIn(
            "crates/comfy_plugin_host/src/comfy_plugin_host.rs",
            provider_request_authority["writes"],
        )
        for field in ("endpoint", "secret-id"):
            self.assertIn(field, provider_request_authority["done"])
        self.assertIn("component cannot supply a provider identity", provider_request_authority["done"])
        provider_worker_stream = tasks_by_id[
            "comfy-parity-provider-worker-stream-protocol"
        ]
        self.assertEqual(
            provider_worker_stream["dependencies"],
            [
                "comfy-parity-provider-streaming-component-abi-v2-request-authority-repair"
            ],
        )
        for path in (
            "crates/comfy_types/Cargo.toml",
            "Cargo.lock",
            "crates/comfy_worker/src/supervisor.rs",
            "crates/comfy_runtime/src/runtime_supervisor.rs",
            "crates/comfy_test_support/src/bin/comfy_test_worker_fixture.rs",
            "crates/comfy_test_support/tests/ownership_consolidation.rs",
            ".agents/specs/comfy-parity/ownership-policy.json",
            ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
        ):
            self.assertIn(path, provider_worker_stream["reads"])
            self.assertIn(path, provider_worker_stream["writes"])
        self.assertIn("Cargo.toml", provider_worker_stream["reads"])
        self.assertIn(
            "crates/comfy_plugin_sdk/src/comfy_plugin_sdk.rs",
            provider_worker_stream["reads"],
        )
        self.assertNotIn("Cargo.toml", provider_worker_stream["writes"])
        for package in (
            "comfy_types",
            "comfy_worker",
            "comfy_runtime",
            "comfy_test_support",
        ):
            self.assertIn(package, provider_worker_stream["validation_packages"])
        self.assertIn("version 7 to version 8", provider_worker_stream["done"])
        self.assertIn("exact raw discriminants", provider_worker_stream["done"])
        self.assertIn("reject the new messages", provider_worker_stream["done"])
        self.assertIn("endpoint, optional secret-id", provider_worker_stream["done"])
        self.assertIn("no component-supplied provider identity", provider_worker_stream["done"])
        self.assertIn("workspace SHA-256", provider_worker_stream["done"])
        provider_worker_clippy_correction = tasks_by_id[
            "comfy-parity-provider-worker-stream-protocol-clippy-correction"
        ]
        self.assertEqual(
            provider_worker_clippy_correction["dependencies"],
            ["comfy-parity-provider-worker-stream-protocol"],
        )
        self.assertEqual(
            provider_worker_clippy_correction["writes"],
            ["crates/comfy_types/src/worker_protocol.rs"],
        )
        self.assertIn(
            "moves the validator's already-owned streaming contract",
            provider_worker_clippy_correction["done"],
        )
        self.assertIn("comfy_types", provider_worker_clippy_correction["validation_packages"])
        provider_runtime_stream = tasks_by_id[
            "comfy-parity-provider-runtime-stream-progress-foundation"
        ]
        self.assertEqual(
            provider_runtime_stream["dependencies"],
            ["comfy-parity-provider-worker-stream-protocol-clippy-correction"],
        )
        self.assertIn(
            "crates/comfy_plugin_host/src/component_host.rs",
            provider_runtime_stream["reads"],
        )
        self.assertIn(
            "crates/comfy_plugin_host/src/capabilities.rs",
            provider_runtime_stream["reads"],
        )
        self.assertIn(
            "crates/comfy_plugin_host/src/capabilities.rs",
            provider_runtime_stream["writes"],
        )
        self.assertIn("crates/comfy_model/tests/clip_tokenizer.rs", provider_runtime_stream["reads"])
        self.assertIn("crates/comfy_model/tests/clip_tokenizer.rs", provider_runtime_stream["writes"])
        for path in (
            "crates/comfy_runtime/src/native_execution_controller.rs",
            ".agents/specs/comfy-parity/ownership-policy.json",
            ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
            "crates/comfy_test_support/tests/ownership_consolidation.rs",
        ):
            self.assertIn(path, provider_runtime_stream["reads"])
            self.assertIn(path, provider_runtime_stream["writes"])
        for read_only_path in (
            "crates/comfy_runtime/src/execution_presentation.rs",
            "crates/comfy_runtime/src/queue_history.rs",
            "crates/comfy_runtime/src/executor.rs",
        ):
            self.assertIn(read_only_path, provider_runtime_stream["reads"])
            self.assertNotIn(read_only_path, provider_runtime_stream["writes"])
        self.assertIn("sole public ProviderRuntimeStreamService", provider_runtime_stream["done"])
        self.assertIn("lock-private ProviderRuntimeStreamState", provider_runtime_stream["done"])
        self.assertIn("no network, credential, purchase, charge, or actuator", provider_runtime_stream["done"])
        self.assertIn("Begin, Call, Resolve, Finish, Abort", provider_runtime_stream["done"])
        self.assertIn(
            "only the later component-host adapter task may construct",
            provider_runtime_stream["done"],
        )
        self.assertIn(
            "no runtime API accepts a component-supplied provider identity",
            provider_runtime_stream["done"],
        )
        self.assertIn(
            "maps exhaustively to the existing fail-closed plugin-host invocation error",
            provider_runtime_stream["done"],
        )
        self.assertIn("comfy_plugin_host", provider_runtime_stream["validation_packages"])
        self.assertIn("comfy_model", provider_runtime_stream["validation_packages"])
        self.assertIn("tokenizer closure", provider_runtime_stream["done"])
        provider_input_host = tasks_by_id[
            "comfy-parity-provider-streaming-component-abi-v2-invocation-input-repair"
        ]
        self.assertEqual(
            provider_input_host["dependencies"],
            ["comfy-parity-provider-runtime-stream-progress-foundation"],
        )
        self.assertEqual(
            provider_input_host["writes"],
            [
                "crates/comfy_plugin_sdk/wit/provider-v2/comfy-provider-plugin.wit",
                "crates/comfy_plugin_sdk/src/comfy_plugin_sdk.rs",
            ],
        )
        self.assertIn(
            "crates/comfy_types/src/worker_protocol.rs",
            provider_input_host["reads"],
        )
        self.assertIn("comfy_types", provider_input_host["validation_packages"])
        self.assertIn(
            "crates/comfy_plugin_sdk/schema/plugin-manifest-v1.schema.json",
            provider_input_host["reads"],
        )
        self.assertNotIn(
            "crates/comfy_plugin_sdk/schema/plugin-manifest-v1.schema.json",
            provider_input_host["writes"],
        )
        self.assertIn(
            "crates/comfy_plugin_sdk/schema/plugin-manifest-v2.schema.json",
            provider_input_host["reads"],
        )
        self.assertNotIn(
            "crates/comfy_plugin_sdk/schema/plugin-manifest-v2.schema.json",
            provider_input_host["writes"],
        )
        self.assertIn("invocation-input-host", provider_input_host["done"])
        self.assertIn("single-consumption", provider_input_host["done"])
        self.assertIn("cannot actuate a provider", provider_input_host["done"])
        self.assertIn("no provider identity", provider_input_host["done"])
        self.assertIn("provider-request", provider_input_host["done"])
        self.assertIn("output or effect", provider_input_host["done"])
        provider_activation_preflight = tasks_by_id[
            "comfy-parity-provider-runtime-component-activation-preflight-foundation"
        ]
        self.assertEqual(
            provider_activation_preflight["dependencies"],
            ["comfy-parity-provider-streaming-component-abi-v2-invocation-input-repair"],
        )
        self.assertEqual(
            provider_activation_preflight["writes"],
            [
                "crates/comfy_runtime/src/plugin_services.rs",
                "crates/comfy_plugin_host/src/component_host.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
            ],
        )
        for preflight_read in (
            "crates/comfy_runtime/src/trust.rs",
            "crates/comfy_runtime/src/native_execution_controller.rs",
            "crates/comfy_runtime/src/comfy_runtime.rs",
            "crates/comfy_types/src/worker_protocol.rs",
            "crates/comfy_plugin_host/src/comfy_plugin_host.rs",
            "crates/comfy_plugin_sdk/wit/provider-v2/comfy-provider-plugin.wit",
        ):
            self.assertIn(preflight_read, provider_activation_preflight["reads"])
            self.assertNotIn(preflight_read, provider_activation_preflight["writes"])
        self.assertIn(
            "crates/comfy_plugin_host/src/component_host.rs",
            provider_activation_preflight["reads"],
        )
        self.assertIn(
            "crates/comfy_plugin_host/src/component_host.rs",
            provider_activation_preflight["writes"],
        )
        self.assertIn(
            "PreflightedProviderRuntimeActivationGrant",
            provider_activation_preflight["done"],
        )
        self.assertIn(
            "PreflightedProviderComponentCapsule",
            provider_activation_preflight["done"],
        )
        self.assertIn("Every failure atomically revokes", provider_activation_preflight["done"])
        self.assertIn("raw grant no longer exposes bind", provider_activation_preflight["done"])
        self.assertIn("No public primitive-field evidence constructor", provider_activation_preflight["done"])
        self.assertIn("can be swapped", provider_activation_preflight["done"])
        self.assertIn(
            "comfy-parity-provider-worker-stream-bridge owns the first executable capsule success",
            provider_activation_preflight["done"],
        )
        self.assertIn(
            "no public or feature-gated test authority factory",
            provider_activation_preflight["done"],
        )
        provider_context_preflight = tasks_by_id[
            "comfy-parity-provider-runtime-worker-context-preflight-repair"
        ]
        self.assertEqual(
            provider_context_preflight["dependencies"],
            ["comfy-parity-provider-runtime-component-activation-preflight-foundation"],
        )
        self.assertEqual(
            provider_context_preflight["writes"],
            [
                "crates/comfy_runtime/src/plugin_services.rs",
                "crates/comfy_plugin_host/src/component_host.rs",
            ],
        )
        self.assertIn(
            "crates/comfy_types/src/worker_protocol.rs",
            provider_context_preflight["reads"],
        )
        self.assertNotIn(
            "crates/comfy_types/src/worker_protocol.rs",
            provider_context_preflight["writes"],
        )
        for context_preflight_gate in (
            "NativeProviderWorkerRequest",
            "postcard bytes remain unchanged",
            "WorkerProviderInvocationContext",
            "context-free preflight overload is removed",
            "private non-cloneable capsule field only after success",
            "no second context selector",
            "atomically revokes",
            "Legacy wire hash and byte fixtures remain frozen",
            "creates no app-to-worker v2 envelope",
        ):
            self.assertIn(context_preflight_gate, provider_context_preflight["done"])
        provider_component_stream = tasks_by_id[
            "comfy-parity-provider-component-host-stream-adapter"
        ]
        self.assertEqual(
            provider_component_stream["dependencies"],
            ["comfy-parity-provider-runtime-worker-context-preflight-repair"],
        )
        self.assertIn(
            "crates/comfy_runtime/src/plugin_services.rs",
            provider_component_stream["reads"],
        )
        self.assertIn(
            "full sealed activation identity", provider_component_stream["done"]
        )
        self.assertIn(
            "cannot construct or replace its authority",
            provider_component_stream["done"],
        )
        self.assertIn(
            "Verified outer and inner provider-v2 authorization",
            provider_component_stream["done"],
        )
        self.assertIn("InvocationHost", provider_component_stream["done"])
        self.assertIn("all-or-none materializer", provider_component_stream["done"])
        self.assertIn("missing-grant denial", provider_component_stream["done"])
        self.assertIn(
            "does not expose or invoke the crate-private raw grant constructor",
            provider_component_stream["done"],
        )
        self.assertIn(
            "worker bridge owns the first production valid-grant issuance",
            provider_component_stream["done"],
        )
        for component_route_gate in (
            "capacity-one typed WorkerProviderStreamRequest",
            "nonblocking enqueue",
            "canonical WorkerProviderStreamTransportValidator",
            "no public/default transport trait",
            "Every legacy v1 preparation",
        ):
            self.assertIn(component_route_gate, provider_component_stream["done"])
        self.assertIn(
            "crates/comfy_plugin_host/src/private_worker.rs",
            provider_component_stream["reads"],
        )
        self.assertNotIn(
            "crates/comfy_plugin_host/src/private_worker.rs",
            provider_component_stream["writes"],
        )
        for frozen_fixture in (
            "crates/comfy_plugin_host/tests/fixtures/provider_component",
            "crates/comfy_plugin_host/tests/fixtures/provider_component_source",
        ):
            self.assertIn(frozen_fixture, provider_component_stream["reads"])
            self.assertNotIn(frozen_fixture, provider_component_stream["writes"])
        for streaming_fixture in (
            "crates/comfy_plugin_host/tests/fixtures/provider_streaming_component",
            "crates/comfy_plugin_host/tests/fixtures/provider_streaming_component_source",
        ):
            self.assertIn(streaming_fixture, provider_component_stream["writes"])
        provider_worker_bridge = tasks_by_id[
            "comfy-parity-provider-worker-stream-bridge"
        ]
        self.assertIn(
            "crates/comfy_runtime/src/native_execution_controller.rs",
            provider_worker_bridge["reads"],
        )
        self.assertIn(
            "crates/comfy_runtime/src/native_execution_controller.rs",
            provider_worker_bridge["writes"],
        )
        self.assertIn(
            "crates/comfy_plugin_host/src/private_worker.rs",
            provider_worker_bridge["reads"],
        )
        self.assertIn(
            "crates/comfy_plugin_host/src/private_worker.rs",
            provider_worker_bridge["writes"],
        )
        self.assertIn(
            "app-side native controller allocates the WorkerProviderInvocationContext",
            provider_worker_bridge["done"],
        )
        for worker_bridge_context_gate in (
            "passes that exact context into the consuming component-host preflight",
            "distinct app-to-worker provider-v2 invocation envelope",
            "NativeProviderWorkerRequest::Begin bytes remain unchanged",
            "canonical transport validator only from that envelope",
            "No public raw-field constructor",
        ):
            self.assertIn(worker_bridge_context_gate, provider_worker_bridge["done"])
        self.assertIn(
            "first valid-grant end-to-end success path",
            provider_worker_bridge["done"],
        )
        self.assertIn(
            "crates/comfy_nodes/src/families/partner_three_d_03.rs",
            provider_projection["writes"],
        )
        self.assertEqual(len(set(provider_vendor_ids)), 33)
        self.assertEqual(
            sum(nodes for _, nodes, _, _ in planning.PROVIDER_VENDOR_SPECS), 224
        )
        self.assertEqual(
            sum(routes for _, _, routes, _ in planning.PROVIDER_VENDOR_SPECS), 217
        )
        self.assertIn(
            ("veo2", 3, 3, ("veo",)), planning.PROVIDER_VENDOR_SPECS
        )
        self.assertEqual(
            sum(
                contract["binding_disposition"] == "provider_required"
                for contract in planning.native_node_contracts()
            ),
            224,
        )
        provider_leaves = [
            candidate
            for candidate in tasks_by_id.values()
            if candidate["id"].startswith("comfy-parity-native-nodes-")
            and provider_closure_id in candidate["dependencies"]
        ]
        self.assertGreater(len(provider_leaves), 0)
        for provider_leaf in provider_leaves:
            self.assertIn(
                "crates/comfy_nodes/src/provider_contracts.rs",
                provider_leaf["reads"],
            )
            self.assertIn(
                "no leaf owns a namespace literal or runtime rewrite",
                provider_leaf["done"],
            )
        self.assertEqual(
            tasks_by_id[provider_shared_ids[0]]["dependencies"],
            [
                provider_id,
                "comfy-parity-native-text-generation-node-bridge",
                "comfy-parity-native-sdpose-execution-foundation",
            ],
        )
        provider_catalog = tasks_by_id[provider_shared_ids[0]]
        self.assertIn(
            ".agents/specs/comfy-parity/catalogs/source-snapshot-manifest.json",
            provider_catalog["reads"],
        )
        self.assertIn(
            ".agents/specs/comfy-parity/catalogs/source-snapshot-manifest.json",
            provider_catalog["writes"],
        )
        for derived_path in [
            ".agents/specs/comfy-parity/catalogs/features.csv",
            ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
            ".agents/specs/comfy-parity/parity-matrix.md",
        ]:
            self.assertIn(derived_path, provider_catalog["writes"])
        for previous, current in zip(provider_shared_ids, provider_shared_ids[1:]):
            self.assertEqual(tasks_by_id[current]["dependencies"], [previous])
        for identifier in provider_vendor_ids:
            self.assertEqual(
                tasks_by_id[identifier]["dependencies"], [provider_shared_ids[-1]]
            )
            self.assertIn(
                "zed.comfy.provider.", tasks_by_id[identifier]["outcome"]
            )
            self.assertNotIn("comfy-node-", tasks_by_id[identifier]["outcome"])
        vendor_write_sets = [
            set(tasks_by_id[identifier]["writes"]) for identifier in provider_vendor_ids
        ]
        for index, writes in enumerate(vendor_write_sets):
            for other in vendor_write_sets[index + 1 :]:
                self.assertTrue(writes.isdisjoint(other))
        self.assertEqual(
            set(tasks_by_id[provider_gate_id]["dependencies"]), set(provider_vendor_ids)
        )
        self.assertEqual(tasks_by_id[provider_gate_id]["writes"], [])
        self.assertEqual(
            tasks_by_id[provider_closure_id]["dependencies"], [provider_gate_id]
        )
        self.assertEqual(
            [
                identifier
                for identifier, item in tasks_by_id.items()
                if provider_closure_id in item["dependencies"]
            ],
            [
                "comfy-parity-native-nodes-partner-three-d-comfy-node-0408",
                "comfy-parity-native-nodes-partner-three-d-comfy-node-0552",
                "comfy-parity-native-nodes-partner-three-d-comfy-node-0686",
                "comfy-parity-native-nodes-partner-three-d-comfy-node-0699",
                "comfy-parity-native-nodes-partner-audio-comfy-node-0040",
                "comfy-parity-native-nodes-partner-audio-comfy-node-0627",
                "comfy-parity-native-nodes-partner-image-comfy-node-0020",
                "comfy-parity-native-nodes-partner-image-comfy-node-0179",
                "comfy-parity-native-nodes-partner-image-comfy-node-0199",
                "comfy-parity-native-nodes-partner-image-comfy-node-0304",
                "comfy-parity-native-nodes-partner-image-comfy-node-0394",
                "comfy-parity-native-nodes-partner-image-comfy-node-0511",
                "comfy-parity-native-nodes-partner-image-comfy-node-0521",
                "comfy-parity-native-nodes-partner-image-comfy-node-0677",
                "comfy-parity-native-nodes-partner-text-comfy-node-0041",
                "comfy-parity-native-nodes-partner-video-comfy-node-0021",
                "comfy-parity-native-nodes-partner-video-comfy-node-0038",
                "comfy-parity-native-nodes-partner-video-comfy-node-0222",
                "comfy-parity-native-nodes-partner-video-comfy-node-0287",
                "comfy-parity-native-nodes-partner-video-comfy-node-0298",
                "comfy-parity-native-nodes-partner-video-comfy-node-0383",
                "comfy-parity-native-nodes-partner-video-comfy-node-0465",
                "comfy-parity-native-nodes-partner-video-comfy-node-0562",
                "comfy-parity-native-nodes-partner-video-comfy-node-0732",
                "comfy-parity-native-nodes-partner-video-comfy-node-0752",
            ],
        )
        for item in [
            *(tasks_by_id[identifier] for identifier in provider_shared_ids),
            *(tasks_by_id[identifier] for identifier in provider_vendor_ids),
            tasks_by_id[provider_gate_id],
            tasks_by_id[provider_closure_id],
        ]:
            paths = [str(path).casefold() for path in (*item["reads"], *item["writes"])]
            self.assertFalse(any("private-key" in path or "signing-key" in path for path in paths))
        catalog_outcome = tasks_by_id[provider_shared_ids[0]]["outcome"]
        catalog_done = tasks_by_id[provider_shared_ids[0]]["done"]
        self.assertIn("current 217 external-service rows", catalog_outcome)
        self.assertIn("61 unresolved methods", catalog_done)
        self.assertIn("zero UNKNOWN", catalog_done)
        for previous, current in zip(
            model_resource_phases, model_resource_phases[1:]
        ):
            dependencies = tasks_by_id[current]["dependencies"]
            self.assertEqual(dependencies[0], previous)
            self.assertTrue(
                set(dependencies[1:]).issubset(
                    {
                        "comfy-parity-model-detection-any-of-key-selector-consolidation",
                    }
                )
            )
        model_resource_dependencies = tasks_by_id[
            "comfy-parity-native-model-resource-execution-foundation"
        ]["dependencies"]
        self.assertEqual(model_resource_dependencies[0], model_resource_phases[-1])
        self.assertTrue(
            set(model_resource_dependencies[1:]).issubset(
                {
                    "comfy-parity-model-detection-any-of-key-selector-consolidation"
                }
            )
        )
        self.assertEqual(
            tasks_by_id[asset_id]["dependencies"],
            [
                latent_bundle_id,
                "comfy-parity-artifact-owner-consolidation",
                "comfy-parity-execution-output-owner-consolidation",
            ],
        )
        self.assertEqual(
            tasks_by_id[provider_id]["dependencies"],
            [
                "comfy-parity-native-audio-empty-segment-foundation",
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
        self.assertEqual(sum(value_id in value for value in dependencies.values()), 77)
        self.assertEqual(sum(asset_id in value for value in dependencies.values()), 77)
        self.assertEqual(sum(provider_id in value for value in dependencies.values()), 26)
        self.assertEqual(
            sum(
                value_id in value and provider_id in value
                for value in dependencies.values()
            ),
            1,
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
            25,
        )
        mapped_values = {
            identifier: sum(
                identifier in task_ids for task_ids in mapping.values()
            )
            for identifier in (schema_id, value_id, asset_id, provider_id)
        }
        self.assertEqual(
            mapped_values,
            {schema_id: 789, value_id: 565, asset_id: 189, provider_id: 224},
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
        text_generation_node_bridge_id = (
            "comfy-parity-native-text-generation-node-bridge"
        )
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
        video_codec_suite_admission_foundation_id = (
            "comfy-parity-native-video-codec-suite-admission-foundation"
        )
        video_codec_vp9_webm_encode_foundation_id = (
            "comfy-parity-native-video-codec-vp9-webm-encode-foundation"
        )
        video_codec_vp9_webm_sequence_encode_foundation_id = (
            "comfy-parity-native-video-codec-vp9-webm-sequence-encode-foundation"
        )
        video_codec_vp9_webm_thread_bridge_foundation_id = (
            "comfy-parity-native-video-codec-vp9-webm-thread-bridge-foundation"
        )
        video_codec_vp9_webm_crf_foundation_id = (
            "comfy-parity-native-video-codec-vp9-webm-crf-foundation"
        )
        video_codec_vp9_webm_container_metadata_foundation_id = (
            "comfy-parity-native-video-codec-vp9-webm-container-metadata-foundation"
        )
        video_codec_vp9_webm_alpha_foundation_id = (
            "comfy-parity-native-video-codec-vp9-webm-alpha-foundation"
        )
        video_codec_av1_webm_sequence_foundation_id = (
            "comfy-parity-native-video-codec-av1-webm-sequence-encode-foundation"
        )
        video_codec_av1_webm_thread_bridge_foundation_id = (
            "comfy-parity-native-video-codec-av1-webm-thread-bridge-foundation"
        )
        video_codec_webm_node_service_foundation_id = (
            "comfy-parity-native-video-codec-webm-node-service-foundation"
        )
        video_save_webm_node_foundation_id = (
            "comfy-parity-native-video-save-webm-node-foundation"
        )
        video_codec_h264_mp4_sequence_encode_foundation_id = (
            "comfy-parity-native-video-codec-h264-mp4-sequence-encode-foundation"
        )
        video_codec_h264_mp4_thread_bridge_foundation_id = (
            "comfy-parity-native-video-codec-h264-mp4-thread-bridge-foundation"
        )
        video_component_h264_mp4_backing_service_foundation_id = (
            "comfy-parity-native-video-component-h264-mp4-backing-service-foundation"
        )
        video_backing_representation_foundation_id = (
            "comfy-parity-native-video-backing-representation-foundation"
        )
        video_codec_h264_mp4_10bit_sequence_encode_foundation_id = (
            "comfy-parity-native-video-codec-h264-mp4-10bit-sequence-encode-foundation"
        )
        video_codec_h264_mp4_10bit_thread_bridge_foundation_id = (
            "comfy-parity-native-video-codec-h264-mp4-10bit-thread-bridge-foundation"
        )
        video_component_h264_mp4_10bit_backing_foundation_id = (
            "comfy-parity-native-video-component-h264-mp4-10bit-backing-foundation"
        )
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
        video_codec_ltxv_h264_mp4_decode_foundation_id = (
            "comfy-parity-native-video-codec-ltxv-h264-mp4-decode-foundation"
        )
        video_codec_ltxv_tensor_preprocess_foundation_id = (
            "comfy-parity-native-video-codec-ltxv-tensor-preprocess-foundation"
        )
        video_codec_ltxv_thread_service_foundation_id = (
            "comfy-parity-native-video-codec-ltxv-thread-service-foundation"
        )
        video_codec_ltxv_node_service_foundation_id = (
            "comfy-parity-native-video-codec-ltxv-node-service-foundation"
        )
        video_codec_ltxv_node_adapter_foundation_id = (
            "comfy-parity-native-video-codec-ltxv-node-adapter-foundation"
        )
        video_component_create_node_foundation_id = (
            "comfy-parity-native-video-component-create-node-foundation"
        )
        video_component_extract_node_foundation_id = (
            "comfy-parity-native-video-component-extract-node-foundation"
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
        frame_interpolation_sequence_fallback_foundation_id = (
            "comfy-parity-native-frame-interpolation-sequence-fallback-foundation"
        )
        frame_interpolate_node_foundation_id = (
            "comfy-parity-native-frame-interpolate-node-foundation"
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
        audio_core_node_id = "comfy-parity-native-nodes-audio-comfy-node-0009"
        audio_output_node_id = "comfy-parity-native-nodes-audio-comfy-node-0589"
        audio_empty_segment_id = "comfy-parity-native-audio-empty-segment-foundation"
        audio_output_codec_id = "comfy-parity-native-audio-output-codec-effect-foundation"
        model_accelerator_id = "comfy-parity-native-model-accelerator-execution-foundation"
        diffusion_retarget_id = "comfy-parity-native-diffusion-device-retarget-foundation"
        multigpu_guidance_id = "comfy-parity-native-multigpu-guidance-execution-foundation"
        multigpu_node_id = "comfy-parity-native-nodes-advanced-multigpu-comfy-node-0454"
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
            video_component_h264_mp4_10bit_backing_foundation_id,
            tasks_by_id[
                "comfy-parity-native-video-codec-general-abi-foundation"
            ]["dependencies"],
        )
        general_video_abi = tasks_by_id[
            "comfy-parity-native-video-codec-general-abi-foundation"
        ]
        self.assertIn("six-library, seventy-eight-symbol", general_video_abi["done"])
        self.assertIn("five-library, fifty-four-symbol", general_video_abi["done"])
        self.assertIn("ReviewedGeneralVideoCodecDeclarations", general_video_abi["done"])
        self.assertIn("UncertifiedFfi", general_video_abi["done"])
        general_video_commands = planning.task_validation_commands(general_video_abi)
        for command in (
            "cargo test --locked -p comfy_runtime general_video_codec_abi --lib -- --nocapture",
            "val_ownership_task555_general_video_codec_declarations_001",
            "cargo check --locked -p comfy_runtime -p comfy_test_support",
            "./script/clippy -p comfy_runtime -p comfy_test_support",
            "regenerate_all.py --check-twice",
            "validate_spec.py .agents/specs/comfy-parity --require-complete",
            "git diff --check",
        ):
            self.assertIn(command, general_video_commands)
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
            tasks_by_id[video_codec_ltxv_h264_mp4_decode_foundation_id]["dependencies"],
            [video_codec_ltxv_h264_mp4_demux_foundation_id],
        )
        codec_ltxv_h264_mp4_decode = tasks_by_id[
            video_codec_ltxv_h264_mp4_decode_foundation_id
        ]
        self.assertEqual(
            codec_ltxv_h264_mp4_decode["writes"],
            [
                "crates/comfy_runtime/src/native_video_codec_ffi.rs",
                "crates/comfy_test_support/fixtures/video/codec-ltxv-h264-mp4-decode/manifest.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            tasks_by_id[video_codec_ltxv_tensor_preprocess_foundation_id]["dependencies"],
            [video_codec_ltxv_h264_mp4_decode_foundation_id],
        )
        codec_ltxv_tensor_preprocess = tasks_by_id[
            video_codec_ltxv_tensor_preprocess_foundation_id
        ]
        self.assertEqual(
            codec_ltxv_tensor_preprocess["writes"],
            [
                "crates/comfy_runtime/src/native_video_codec_ffi.rs",
                "crates/comfy_test_support/fixtures/video/codec-ltxv-tensor-preprocess/manifest.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            tasks_by_id[video_codec_ltxv_thread_service_foundation_id]["dependencies"],
            [video_codec_ltxv_tensor_preprocess_foundation_id],
        )
        self.assertEqual(
            tasks_by_id[video_codec_ltxv_thread_service_foundation_id]["writes"],
            [
                "crates/comfy_runtime/src/native_video_codec_ffi.rs",
                "crates/comfy_runtime/src/native_video_codec_service.rs",
                "crates/comfy_runtime/src/comfy_runtime.rs",
                "crates/comfy_test_support/fixtures/video/codec-ltxv-tensor-preprocess/manifest.json",
                "crates/comfy_test_support/fixtures/video/codec-ltxv-thread-service/manifest.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            tasks_by_id[video_codec_ltxv_node_service_foundation_id]["dependencies"],
            [video_codec_ltxv_thread_service_foundation_id],
        )
        self.assertEqual(
            tasks_by_id[video_codec_ltxv_node_service_foundation_id]["writes"],
            [
                "crates/comfy_nodes/src/execution.rs",
                "crates/comfy_nodes/src/comfy_nodes.rs",
                "crates/comfy_runtime/src/native_video_codec_service.rs",
                "crates/comfy_runtime/src/executor.rs",
                "crates/comfy_runtime/src/native_execution_controller.rs",
                "crates/comfy_runtime/src/comfy_runtime.rs",
                "crates/comfy_test_support/fixtures/video/codec-ltxv-node-service/manifest.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            tasks_by_id[video_codec_ltxv_node_adapter_foundation_id]["dependencies"],
            [video_codec_ltxv_node_service_foundation_id],
        )
        self.assertEqual(
            tasks_by_id[video_component_create_node_foundation_id]["dependencies"],
            [video_codec_ltxv_node_adapter_foundation_id],
        )
        self.assertEqual(
            tasks_by_id[video_component_extract_node_foundation_id]["dependencies"],
            [video_component_create_node_foundation_id],
        )
        self.assertEqual(
            tasks_by_id[frame_interpolation_sequence_fallback_foundation_id][
                "dependencies"
            ],
            [
                video_component_extract_node_foundation_id,
                frame_interpolation_resource_exhaustion_foundation_id,
            ],
        )
        self.assertEqual(
            tasks_by_id[video_codec_suite_admission_foundation_id]["dependencies"],
            [frame_interpolate_node_foundation_id],
        )
        self.assertEqual(
            tasks_by_id[video_codec_suite_admission_foundation_id]["writes"],
            [
                "crates/comfy_runtime/src/native_video_codec_ffi.rs",
                "crates/comfy_runtime/src/native_video_codec_service.rs",
                "crates/comfy_test_support/fixtures/video/codec-suite-admission/manifest.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            tasks_by_id[video_codec_vp9_webm_encode_foundation_id]["dependencies"],
            [video_codec_suite_admission_foundation_id],
        )
        self.assertEqual(
            tasks_by_id[video_codec_vp9_webm_encode_foundation_id]["writes"],
            [
                "crates/comfy_runtime/src/native_video_codec_ffi.rs",
                "crates/comfy_test_support/fixtures/video/codec-vp9-webm-encode/manifest.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            tasks_by_id[video_codec_vp9_webm_sequence_encode_foundation_id]["dependencies"],
            [video_codec_vp9_webm_encode_foundation_id],
        )
        self.assertEqual(
            tasks_by_id[video_codec_vp9_webm_sequence_encode_foundation_id]["writes"],
            [
                "crates/comfy_runtime/src/native_video_codec_ffi.rs",
                "crates/comfy_test_support/fixtures/video/codec-vp9-webm-sequence-encode/manifest.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            tasks_by_id[video_codec_vp9_webm_thread_bridge_foundation_id]["dependencies"],
            [video_codec_vp9_webm_sequence_encode_foundation_id],
        )
        self.assertEqual(
            tasks_by_id[video_codec_vp9_webm_thread_bridge_foundation_id]["writes"],
            [
                "crates/comfy_runtime/src/native_video_codec_service.rs",
                "crates/comfy_test_support/fixtures/video/codec-vp9-webm-thread-bridge/manifest.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            tasks_by_id[video_codec_vp9_webm_crf_foundation_id]["dependencies"],
            [video_codec_vp9_webm_thread_bridge_foundation_id],
        )
        self.assertEqual(
            tasks_by_id[video_codec_vp9_webm_crf_foundation_id]["writes"],
            [
                "crates/comfy_media/src/video.rs",
                "crates/comfy_runtime/src/native_video_codec_ffi.rs",
                "crates/comfy_runtime/src/native_video_codec_service.rs",
                "crates/comfy_test_support/fixtures/video/codec-vp9-webm-crf/manifest.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            tasks_by_id[video_codec_vp9_webm_container_metadata_foundation_id][
                "dependencies"
            ],
            [video_codec_vp9_webm_crf_foundation_id],
        )
        self.assertEqual(
            tasks_by_id[video_codec_vp9_webm_container_metadata_foundation_id]["writes"],
            [
                "crates/comfy_runtime/src/native_video_codec_abi.rs",
                "crates/comfy_runtime/src/native_video_codec_ffi.rs",
                "crates/comfy_runtime/src/native_video_codec_service.rs",
                "crates/comfy_runtime/abi/video-codec/ffmpeg-7.1-x86_64-gnu-container-metadata-v1.json",
                "crates/comfy_runtime/abi/video-codec/verify-container-metadata-bindings.c",
                "crates/comfy_test_support/fixtures/video/codec-vp9-webm-container-metadata/manifest.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            tasks_by_id[video_codec_vp9_webm_alpha_foundation_id]["dependencies"],
            [video_codec_vp9_webm_container_metadata_foundation_id],
        )
        self.assertEqual(
            tasks_by_id[video_codec_vp9_webm_alpha_foundation_id]["writes"],
            [
                "crates/comfy_runtime/src/native_video_codec_abi.rs",
                "crates/comfy_runtime/src/native_video_codec_ffi.rs",
                "crates/comfy_runtime/src/native_video_codec_service.rs",
                "crates/comfy_runtime/abi/video-codec/ffmpeg-7.1-x86_64-gnu-vp9-alpha-v1.json",
                "crates/comfy_runtime/abi/video-codec/verify-vp9-alpha-bindings.c",
                "crates/comfy_test_support/fixtures/video/codec-vp9-webm-alpha/manifest.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            tasks_by_id[video_codec_av1_webm_sequence_foundation_id]["dependencies"],
            [video_codec_vp9_webm_alpha_foundation_id],
        )
        self.assertEqual(
            tasks_by_id[video_codec_av1_webm_sequence_foundation_id]["writes"],
            [
                "crates/comfy_runtime/src/native_video_codec_abi.rs",
                "crates/comfy_runtime/src/native_video_codec_ffi.rs",
                "crates/comfy_runtime/abi/video-codec/ffmpeg-7.1-x86_64-gnu-av1-pixel-format-v1.json",
                "crates/comfy_runtime/abi/video-codec/verify-av1-pixel-format-bindings.c",
                "crates/comfy_test_support/fixtures/video/codec-av1-webm-sequence-encode/manifest.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            tasks_by_id[video_codec_av1_webm_thread_bridge_foundation_id][
                "dependencies"
            ],
            [video_codec_av1_webm_sequence_foundation_id],
        )
        self.assertEqual(
            tasks_by_id[video_codec_av1_webm_thread_bridge_foundation_id]["writes"],
            [
                "crates/comfy_runtime/src/native_video_codec_service.rs",
                "crates/comfy_test_support/fixtures/video/codec-av1-webm-thread-bridge/manifest.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            tasks_by_id[video_codec_webm_node_service_foundation_id]["dependencies"],
            [video_codec_av1_webm_thread_bridge_foundation_id],
        )
        self.assertEqual(
            tasks_by_id[video_codec_webm_node_service_foundation_id]["writes"],
            [
                "crates/comfy_nodes/src/execution.rs",
                "crates/comfy_nodes/src/comfy_nodes.rs",
                "crates/comfy_runtime/src/native_video_codec_ffi.rs",
                "crates/comfy_runtime/src/native_video_codec_service.rs",
                "crates/comfy_runtime/src/executor.rs",
                "crates/comfy_runtime/src/native_execution_controller.rs",
                "crates/comfy_runtime/src/comfy_runtime.rs",
                "crates/comfy_test_support/fixtures/video/codec-webm-node-service/manifest.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            tasks_by_id[video_save_webm_node_foundation_id]["dependencies"],
            [video_codec_webm_node_service_foundation_id],
        )
        self.assertEqual(
            tasks_by_id[video_save_webm_node_foundation_id]["writes"],
            [
                "crates/comfy_media/src/video.rs",
                "crates/comfy_nodes/src/families/video_01.rs",
                "crates/comfy_test_support/fixtures/nodes/video-comfy-node-0602/fixture.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            tasks_by_id[video_codec_h264_mp4_sequence_encode_foundation_id][
                "dependencies"
            ],
            [video_save_webm_node_foundation_id],
        )
        self.assertEqual(
            tasks_by_id[video_codec_h264_mp4_sequence_encode_foundation_id]["writes"],
            [
                "crates/comfy_runtime/src/native_video_codec_ffi.rs",
                "crates/comfy_test_support/fixtures/video/codec-h264-mp4-sequence-encode/manifest.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            tasks_by_id[video_codec_h264_mp4_thread_bridge_foundation_id][
                "dependencies"
            ],
            [video_codec_h264_mp4_sequence_encode_foundation_id],
        )
        self.assertEqual(
            tasks_by_id[video_codec_h264_mp4_thread_bridge_foundation_id]["writes"],
            [
                "crates/comfy_runtime/src/native_video_codec_service.rs",
                "crates/comfy_test_support/fixtures/video/codec-h264-mp4-thread-bridge/manifest.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            tasks_by_id[video_component_h264_mp4_backing_service_foundation_id][
                "dependencies"
            ],
            [video_codec_h264_mp4_thread_bridge_foundation_id],
        )
        self.assertEqual(
            tasks_by_id[video_component_h264_mp4_backing_service_foundation_id][
                "writes"
            ],
            [
                "crates/comfy_nodes/src/execution.rs",
                "crates/comfy_nodes/src/comfy_nodes.rs",
                "crates/comfy_runtime/src/native_video_codec_ffi.rs",
                "crates/comfy_runtime/src/native_video_codec_service.rs",
                "crates/comfy_runtime/src/executor.rs",
                "crates/comfy_runtime/src/native_execution_controller.rs",
                "crates/comfy_runtime/src/comfy_runtime.rs",
                "crates/comfy_test_support/fixtures/video/component-h264-mp4-backing-service/manifest.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            tasks_by_id[video_backing_representation_foundation_id]["dependencies"],
            [video_component_h264_mp4_backing_service_foundation_id],
        )
        self.assertEqual(
            tasks_by_id[video_backing_representation_foundation_id]["writes"],
            [
                "crates/comfy_media/src/native_node_payload.rs",
                "crates/comfy_media/src/video.rs",
                "crates/comfy_nodes/src/execution.rs",
                "crates/comfy_nodes/src/comfy_nodes.rs",
                "crates/comfy_nodes/src/families/video_01.rs",
                "crates/comfy_plugin_host/src/registry_adapter.rs",
                "crates/comfy_runtime/src/native_video_codec_service.rs",
                "crates/comfy_runtime/src/native_execution_controller.rs",
                "crates/comfy_runtime/src/provider_materialization.rs",
                "crates/comfy_runtime/src/comfy_runtime.rs",
                "crates/comfy_test_support/fixtures/video/encoded-video-backing-payload/manifest.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            tasks_by_id[video_codec_h264_mp4_10bit_sequence_encode_foundation_id][
                "dependencies"
            ],
            [video_backing_representation_foundation_id],
        )
        self.assertEqual(
            tasks_by_id[video_codec_h264_mp4_10bit_thread_bridge_foundation_id][
                "dependencies"
            ],
            [video_codec_h264_mp4_10bit_sequence_encode_foundation_id],
        )
        self.assertEqual(
            tasks_by_id[video_codec_h264_mp4_10bit_thread_bridge_foundation_id][
                "writes"
            ],
            [
                "crates/comfy_runtime/src/native_video_codec_service.rs",
                "crates/comfy_test_support/fixtures/video/codec-h264-mp4-10bit-thread-bridge/manifest.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            tasks_by_id[video_component_h264_mp4_10bit_backing_foundation_id][
                "dependencies"
            ],
            [video_codec_h264_mp4_10bit_thread_bridge_foundation_id],
        )
        self.assertEqual(
            tasks_by_id[video_component_h264_mp4_10bit_backing_foundation_id]["writes"],
            [
                "crates/comfy_media/src/native_node_payload.rs",
                "crates/comfy_nodes/src/execution.rs",
                "crates/comfy_runtime/src/native_video_codec_service.rs",
                "crates/comfy_test_support/fixtures/video/component-h264-mp4-10bit-backing/manifest.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            tasks_by_id[video_foundation_id]["dependencies"],
            [
                "comfy-parity-native-video-save-remux-audio-effects-foundation",
                "comfy-parity-native-nodes-model-loaders-comfy-node-0012",
            ],
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
        self.assertEqual(
            tasks_by_id[frame_interpolation_sequence_fallback_foundation_id]["writes"],
            [
                "crates/comfy_model/src/frame_interpolation.rs",
                "crates/comfy_test_support/fixtures/models/frame-interpolation/sequence-fallback/manifest.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertIn(
            "VAL-MODEL-FORMAT-001",
            tasks_by_id[frame_interpolation_sequence_fallback_foundation_id][
                "validations"
            ],
        )
        self.assertEqual(
            tasks_by_id[frame_interpolate_node_foundation_id]["dependencies"],
            [frame_interpolation_sequence_fallback_foundation_id],
        )
        self.assertEqual(
            tasks_by_id[frame_interpolate_node_foundation_id]["writes"],
            [
                "crates/comfy_nodes/Cargo.toml",
                "crates/comfy_nodes/src/families/video_01.rs",
                "crates/comfy_test_support/fixtures/nodes/video-comfy-node-0190/fixture.json",
                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                ".agents/specs/comfy-parity/ownership-policy.json",
                ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
                ".agents/specs/comfy-parity/regenerate_native_planning.py",
                ".agents/specs/comfy-parity/test_regenerate_native_planning.py",
            ],
        )
        self.assertEqual(
            tasks_by_id[video_foundation_id]["dependencies"],
            [
                "comfy-parity-native-video-save-remux-audio-effects-foundation",
                "comfy-parity-native-nodes-model-loaders-comfy-node-0012",
            ],
        )
        self.assertIn(
            frame_interpolate_node_foundation_id,
            mapping["COMFY-NODE-0190"],
        )
        self.assertIn(
            video_save_webm_node_foundation_id,
            mapping["COMFY-NODE-0602"],
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
            video_component_h264_mp4_10bit_backing_foundation_id,
            tasks_by_id[
                "comfy-parity-native-video-codec-general-abi-foundation"
            ]["dependencies"],
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
        self.assertTrue(
            tasks_by_id[frame_interpolation_sequence_fallback_foundation_id]["locked"]
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
        self.assertIn(text_generation_node_bridge_id, dependencies[text_generation_id])
        self.assertIn(
            text_generation_foundation_id,
            tasks_by_id[text_generation_node_bridge_id]["dependencies"],
        )
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
            tasks_by_id["comfy-parity-native-video-demux-decode-foundation"][
                "writes"
            ],
        )
        self.assertIn(
            "crates/comfy_model/src/frame_interpolation.rs",
            tasks_by_id[video_foundation_id]["reads"],
        )
        self.assertNotIn(
            "crates/comfy_model/src/frame_interpolation.rs",
            tasks_by_id[video_foundation_id]["writes"],
        )
        self.assertTrue(tasks_by_id[video_foundation_id]["locked"])
        self.assertIn(image_source_foundation_id, dependencies[image_filter_id])
        self.assertIn(
            "comfy-parity-tensor-ops-random-number-generation-comfy-tensor-op-fd729b8a5363",
            dependencies[image_filter_id],
        )
        self.assertIn(
            "crates/comfy_tensor/src/ops/random_number_generation_01.rs",
            tasks_by_id[image_filter_id]["reads"],
        )
        self.assertIn(
            "crates/comfy_tensor/src/ops/random_number_generation_02.rs",
            tasks_by_id[image_filter_id]["reads"],
        )
        self.assertIn(
            "crates/comfy_tensor/src/ops/shape_layout_transform_03.rs",
            tasks_by_id[image_filter_id]["reads"],
        )
        self.assertIn(
            "crates/comfy_tensor/src/ops/spatial_functional_kernel_01.rs",
            tasks_by_id[image_filter_id]["reads"],
        )
        self.assertIn(image_source_foundation_id, dependencies[image_transform_id])
        self.assertIn(
            "crates/comfy_media/src/image_quantization.rs",
            tasks_by_id[image_source_foundation_id]["writes"],
        )
        self.assertIn(structured_link_foundation_id, dependencies[structured_transform_id])
        self.assertIn(
            inherited_v3_presentation_id, dependencies[structured_transform_id]
        )
        self.assertEqual(
            tasks_by_id[audio_output_codec_id]["dependencies"],
            [
                audio_empty_segment_id,
                video_foundation_id,
                video_id,
                detection_foundation_id,
                detection_id,
                media_text_foundation_id,
                media_text_id,
                "comfy-parity-native-nodes-image-comfy-node-0586",
            ],
        )
        self.assertEqual(
            tasks_by_id[audio_empty_segment_id]["dependencies"],
            [
                model_accelerator_id,
                "comfy-parity-native-audio-encoder-resource-foundation",
            ],
        )
        self.assertIn(
            "crates/comfy_model/src/audio_encoder.rs",
            tasks_by_id[audio_empty_segment_id]["writes"],
        )
        self.assertIn("1 through 384000", tasks_by_id[audio_empty_segment_id]["outcome"])
        self.assertIn("[1,1,0]", tasks_by_id[audio_empty_segment_id]["done"])
        self.assertIn(audio_empty_segment_id, dependencies[audio_core_node_id])
        self.assertIn(audio_output_codec_id, dependencies[audio_core_node_id])
        self.assertLess(
            tasks.index(tasks_by_id[audio_empty_segment_id]),
            tasks.index(tasks_by_id[audio_output_codec_id]),
        )
        self.assertLess(
            tasks.index(tasks_by_id[audio_output_codec_id]),
            tasks.index(tasks_by_id[audio_core_node_id]),
        )
        self.assertIn(
            "crates/comfy_media/src/audio.rs",
            tasks_by_id[audio_core_node_id]["reads"],
        )
        self.assertIn(
            "crates/comfy_runtime/src/native_audio_codec_service.rs",
            tasks_by_id[audio_core_node_id]["reads"],
        )
        self.assertIn(
            "first-stream decoding",
            tasks_by_id[audio_core_node_id]["done"],
        )
        self.assertIn(audio_empty_segment_id, dependencies[audio_output_node_id])
        self.assertIn(audio_output_codec_id, dependencies[audio_output_node_id])
        self.assertIn(
            "crates/comfy_runtime/src/native_audio_codec_service.rs",
            tasks_by_id[audio_output_codec_id]["writes"],
        )
        self.assertIn(
            "crates/comfy_runtime/src/native_video_codec_service.rs",
            tasks_by_id[audio_output_codec_id]["writes"],
        )
        self.assertIn(
            "first audio stream",
            tasks_by_id[audio_output_codec_id]["done"],
        )
        self.assertIn(
            "FLAC, libmp3lame V0/128k/320k, and Opus",
            tasks_by_id[audio_output_codec_id]["done"],
        )
        self.assertEqual(
            tasks_by_id[model_accelerator_id]["dependencies"],
            [
                "comfy-parity-native-compile-policy-bridge-foundation",
                "comfy-parity-native-compute-breadth-integration",
                "comfy-parity-native-module-backend-target-admission-consolidation",
                "comfy-parity-native-memory-planner",
                "comfy-parity-model-detection-any-of-key-selector-consolidation",
            ],
        )
        self.assertIn(
            model_accelerator_id,
            tasks_by_id[diffusion_retarget_id]["dependencies"],
        )
        self.assertIn(
            "comfy-parity-native-model-resource-execution-foundation",
            tasks_by_id[diffusion_retarget_id]["dependencies"],
        )
        self.assertIn(
            audio_output_codec_id,
            tasks_by_id[diffusion_retarget_id]["dependencies"],
        )
        self.assertIn(
            audio_core_node_id,
            tasks_by_id[diffusion_retarget_id]["dependencies"],
        )
        self.assertIn(
            diffusion_retarget_id,
            tasks_by_id[multigpu_guidance_id]["dependencies"],
        )
        self.assertIn(
            "comfy-parity-native-sampling-profile-guidance-foundation",
            tasks_by_id[multigpu_guidance_id]["dependencies"],
        )
        self.assertIn(diffusion_retarget_id, dependencies[multigpu_node_id])
        self.assertIn(multigpu_guidance_id, dependencies[multigpu_node_id])
        self.assertLess(
            tasks.index(tasks_by_id[model_accelerator_id]),
            tasks.index(tasks_by_id[diffusion_retarget_id]),
        )
        self.assertLess(
            tasks.index(tasks_by_id[diffusion_retarget_id]),
            tasks.index(tasks_by_id[multigpu_guidance_id]),
        )
        self.assertLess(
            tasks.index(tasks_by_id[multigpu_guidance_id]),
            tasks.index(tasks_by_id[multigpu_node_id]),
        )
        for source_path in [
            "projects/comfy/ComfyUI/comfy_extras/nodes_multigpu.py",
            "projects/comfy/ComfyUI/comfy/multigpu.py",
            "projects/comfy/ComfyUI/comfy/model_management.py",
            "projects/comfy/ComfyUI/comfy/model_patcher.py",
            "projects/comfy/ComfyUI/comfy/samplers.py",
            "projects/comfy/ComfyUI/comfy/sampler_helpers.py",
        ]:
            self.assertIn(source_path, tasks_by_id[multigpu_node_id]["reads"])
        self.assertEqual(
            tasks_by_id[multigpu_node_id]["writes"],
            [
                "crates/comfy_nodes/src/families/advanced_multigpu_01.rs",
                "crates/comfy_test_support/fixtures/nodes/advanced-multigpu-comfy-node-0454",
            ],
        )
        for validation in [
            "VAL-DEVICE-001",
            "VAL-SAMPLER-001",
            "VAL-MEMORY-001",
            "VAL-CANCEL-001",
            "VAL-OWNERSHIP-001",
        ]:
            self.assertIn(validation, tasks_by_id[multigpu_node_id]["validations"])
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
            text_generation_node_bridge_id,
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
        self.assertEqual(waves[inherited_v3_presentation_id], waves[schema_id] + 1)
        self.assertEqual(
            waves[v3_presentation_catalog_closure_id],
            waves[inherited_v3_presentation_id] + 1,
        )
        self.assertEqual(waves[value_id], waves[v3_presentation_catalog_closure_id] + 1)
        self.assertEqual(waves[latent_bundle_id], waves[value_id] + 1)
        self.assertEqual(waves[asset_id], waves[latent_bundle_id] + 1)
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
            waves[text_generation_node_bridge_id],
            waves[text_generation_foundation_id] + 1,
        )
        self.assertEqual(
            waves[sdpose_projection_id], waves[text_generation_node_bridge_id] + 1
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
            waves[video_codec_ltxv_h264_mp4_decode_foundation_id],
            waves[video_codec_ltxv_h264_mp4_demux_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_ltxv_tensor_preprocess_foundation_id],
            waves[video_codec_ltxv_h264_mp4_decode_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_ltxv_thread_service_foundation_id],
            waves[video_codec_ltxv_tensor_preprocess_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_ltxv_node_service_foundation_id],
            waves[video_codec_ltxv_thread_service_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_ltxv_node_adapter_foundation_id],
            waves[video_codec_ltxv_node_service_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_component_create_node_foundation_id],
            waves[video_codec_ltxv_node_adapter_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_component_extract_node_foundation_id],
            waves[video_component_create_node_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_suite_admission_foundation_id],
            waves[frame_interpolate_node_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_vp9_webm_encode_foundation_id],
            waves[video_codec_suite_admission_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_vp9_webm_sequence_encode_foundation_id],
            waves[video_codec_vp9_webm_encode_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_vp9_webm_thread_bridge_foundation_id],
            waves[video_codec_vp9_webm_sequence_encode_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_vp9_webm_crf_foundation_id],
            waves[video_codec_vp9_webm_thread_bridge_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_vp9_webm_container_metadata_foundation_id],
            waves[video_codec_vp9_webm_crf_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_vp9_webm_alpha_foundation_id],
            waves[video_codec_vp9_webm_container_metadata_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_av1_webm_sequence_foundation_id],
            waves[video_codec_vp9_webm_alpha_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_av1_webm_thread_bridge_foundation_id],
            waves[video_codec_av1_webm_sequence_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_webm_node_service_foundation_id],
            waves[video_codec_av1_webm_thread_bridge_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_save_webm_node_foundation_id],
            waves[video_codec_webm_node_service_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_h264_mp4_sequence_encode_foundation_id],
            waves[video_save_webm_node_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_h264_mp4_thread_bridge_foundation_id],
            waves[video_codec_h264_mp4_sequence_encode_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_component_h264_mp4_backing_service_foundation_id],
            waves[video_codec_h264_mp4_thread_bridge_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_backing_representation_foundation_id],
            waves[video_component_h264_mp4_backing_service_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_h264_mp4_10bit_sequence_encode_foundation_id],
            waves[video_backing_representation_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_codec_h264_mp4_10bit_thread_bridge_foundation_id],
            waves[video_codec_h264_mp4_10bit_sequence_encode_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_component_h264_mp4_10bit_backing_foundation_id],
            waves[video_codec_h264_mp4_10bit_thread_bridge_foundation_id] + 1,
        )
        self.assertEqual(
            waves[video_foundation_id],
            waves["comfy-parity-native-video-save-remux-audio-effects-foundation"]
            + 1,
        )
        self.assertEqual(
            waves[frame_interpolation_sequence_fallback_foundation_id],
            max(
                waves[video_component_extract_node_foundation_id],
                waves[frame_interpolation_resource_exhaustion_foundation_id],
            )
            + 1,
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
        self.assertIn("crates/zed/src/zed.rs", foundation_writes)
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
        self.assertIn("crates/zed/src/comfy_plugin_services.rs", provider_writes)
        self.assertIn("crates/zed/src/zed.rs", provider_writes)
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
            ["comfy-parity-native-audio-empty-segment-foundation"],
        )
        for path in [
            "crates/zed/Cargo.toml",
            "crates/zed/src/main.rs",
            "crates/zed/src/zed.rs",
            "crates/zed/src/zed/app_menus.rs",
            "crates/extension_host/Cargo.toml",
            "crates/extension_host/src/extension_host.rs",
            "script/check-comfy-feature-boundary",
            "script/bundle-mac",
            "script/bundle-linux",
            "script/bundle-windows.ps1",
            "crates/zed/resources/windows/zed.iss",
            "crates/comfy_test_support/tests/native_release_boundary.rs",
            "crates/comfy_test_support/tests/ownership_consolidation.rs",
            ".agents/specs/comfy-parity/catalogs/native-backend-dependencies.json",
            ".agents/specs/comfy-parity/validate_backend_dependencies.py",
        ]:
            self.assertIn(path, build_boundary_task["writes"])
        for command in [
            "cargo check --locked -p zed --no-default-features",
            "cargo test --locked -p zed --no-default-features",
            "cargo check --locked -p zed --features comfy",
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
        self.assertIn("crates/zed/src/zed.rs", registry_writes)

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

    def test_blocked_image_and_experimental_leaves_have_canonical_precursors(self) -> None:
        tasks, _ = planning.all_tasks()
        tasks_by_id = {str(item["id"]): item for item in tasks}
        waves = planning.task_waves(tasks)

        asset_snapshot_id = "comfy-parity-native-asset-directory-snapshot-foundation"
        progress_text_id = "comfy-parity-native-node-progress-text-foundation"
        visual_decode_id = "comfy-parity-native-visual-asset-decode-foundation"
        image_output_id = "comfy-parity-native-image-output-codec-effect-foundation"
        dataset_output_id = "comfy-parity-native-dataset-output-policy-foundation"
        image_part_one_id = "comfy-parity-native-nodes-image-comfy-node-0160"
        image_part_two_id = "comfy-parity-native-nodes-image-comfy-node-0586"
        experimental_hooks_id = "comfy-parity-native-nodes-experimental-comfy-node-0133"
        experimental_compile_id = "comfy-parity-native-nodes-experimental-comfy-node-0680"
        attention_multiply_id = (
            "comfy-parity-native-nodes-experimental-attention-experiments-comfy-node-0057"
        )
        model_transform_id = "comfy-parity-native-model-transform-foundation"
        sampler_payload_id = "comfy-parity-native-sampler-payload-algorithm-foundation"
        sampling_profile_id = "comfy-parity-native-sampling-profile-guidance-foundation"
        compile_bridge_id = "comfy-parity-native-compile-policy-bridge-foundation"
        structured_link_id = "comfy-parity-native-structured-input-link-foundation"

        self.assertEqual(
            tasks_by_id[progress_text_id]["dependencies"], [asset_snapshot_id]
        )
        self.assertIn(progress_text_id, tasks_by_id[visual_decode_id]["dependencies"])
        self.assertEqual(
            tasks_by_id[image_output_id]["dependencies"],
            [visual_decode_id, image_part_one_id],
        )
        self.assertEqual(
            tasks_by_id[dataset_output_id]["dependencies"], [image_output_id]
        )
        for identifier in (
            asset_snapshot_id,
            progress_text_id,
            visual_decode_id,
            image_output_id,
            dataset_output_id,
        ):
            self.assertTrue(tasks_by_id[identifier]["locked"])
            self.assertTrue(tasks_by_id[identifier]["feature_scoped"])
        self.assertLess(waves[asset_snapshot_id], waves[progress_text_id])
        self.assertLess(waves[progress_text_id], waves[visual_decode_id])
        self.assertLess(waves[visual_decode_id], waves[image_part_one_id])
        self.assertLess(waves[image_part_one_id], waves[image_output_id])
        self.assertLess(waves[image_output_id], waves[dataset_output_id])
        self.assertLess(waves[dataset_output_id], waves[image_part_two_id])

        for dependency in (asset_snapshot_id, progress_text_id, visual_decode_id):
            self.assertIn(dependency, tasks_by_id[image_part_one_id]["dependencies"])
        for dependency in (
            image_part_one_id,
            image_output_id,
            dataset_output_id,
            structured_link_id,
        ):
            self.assertIn(dependency, tasks_by_id[image_part_two_id]["dependencies"])
        for path in (
            "crates/comfy_nodes/src/slices/native_image.rs",
            "crates/comfy_nodes/src/slices/native_image.descriptors.json",
            "crates/comfy_runtime/src/native_execution_controller.rs",
        ):
            self.assertIn(path, tasks_by_id[image_part_one_id]["writes"])
            self.assertIn(path, tasks_by_id[image_part_two_id]["writes"])
        self.assertIn(
            "first-visual-stream", tasks_by_id[visual_decode_id]["done"]
        )
        self.assertIn("animated WebP", tasks_by_id[visual_decode_id]["done"])
        self.assertIn("PNG 8/16-bit", tasks_by_id[image_output_id]["done"])
        self.assertIn("paired PNG and TXT", tasks_by_id[dataset_output_id]["done"])
        self.assertIn("exactly one executable binding", tasks_by_id[image_part_one_id]["done"])
        self.assertIn("early SaveImage owner", tasks_by_id[image_part_two_id]["done"])

        for dependency in (model_transform_id, sampler_payload_id, sampling_profile_id):
            self.assertIn(dependency, tasks_by_id[experimental_hooks_id]["dependencies"])
        for source in (
            "projects/comfy/ComfyUI/comfy_extras/nodes_differential_diffusion.py",
            "projects/comfy/ComfyUI/comfy_extras/nodes_flux.py",
            "projects/comfy/ComfyUI/comfy_extras/nodes_fresca.py",
            "projects/comfy/ComfyUI/comfy_extras/nodes_lora_extract.py",
            "projects/comfy/ComfyUI/comfy_extras/nodes_mahiro.py",
            "projects/comfy/ComfyUI/comfy_extras/nodes_perpneg.py",
            "projects/comfy/ComfyUI/comfy_extras/nodes_advanced_samplers.py",
            "projects/comfy/ComfyUI/comfy_extras/nodes_sag.py",
        ):
            self.assertIn(source, tasks_by_id[experimental_hooks_id]["reads"])
        self.assertIn("Euler CFG++", tasks_by_id[experimental_hooks_id]["done"])
        self.assertIn(
            model_transform_id, tasks_by_id[attention_multiply_id]["dependencies"]
        )
        model_transform = tasks_by_id[model_transform_id]
        for source in (
            "projects/comfy/ComfyUI/comfy_extras/nodes_attention_multiply.py",
            "projects/comfy/ComfyUI/comfy/model_patcher.py",
            "projects/comfy/ComfyUI/comfy/lora.py",
            "crates/comfy_model/src/clip.rs",
        ):
            self.assertIn(source, model_transform["reads"])
        self.assertIn("crates/comfy_model/src/clip.rs", model_transform["writes"])
        for validation in (
            "VAL-CLIP-001",
            "VAL-PATCH-001",
            "VAL-MEMORY-001",
            "VAL-RECOVERY-005",
        ):
            self.assertIn(validation, model_transform["validations"])
        for contract in (
            "explicit scale-only operation",
            "zero patch-tensor residency",
            "every source-reachable CLIP-role payload",
            "selector, match set, order, scale",
        ):
            self.assertIn(contract, model_transform["done"])
        for source in (
            "projects/comfy/ComfyUI/comfy_extras/nodes_attention_multiply.py",
            "projects/comfy/ComfyUI/comfy/model_patcher.py",
            "projects/comfy/ComfyUI/comfy/lora.py",
            "crates/comfy_model/src/model_family.rs",
            "crates/comfy_model/src/native_node_payload.rs",
            "crates/comfy_model/src/patch_graph.rs",
            "crates/comfy_model/src/clip.rs",
        ):
            self.assertIn(source, tasks_by_id[attention_multiply_id]["reads"])
        self.assertIn(
            "explicit scale-only operation",
            tasks_by_id[attention_multiply_id]["done"],
        )
        self.assertIn(
            "Every source-reachable CLIP-role payload",
            tasks_by_id[attention_multiply_id]["done"],
        )
        self.assertIn(
            "may not narrow support to NativeClipResource",
            tasks_by_id[attention_multiply_id]["done"],
        )
        for validation in ("VAL-CLIP-001", "VAL-PATCH-001"):
            self.assertIn(
                validation, tasks_by_id[attention_multiply_id]["validations"]
            )
        self.assertIn(compile_bridge_id, tasks_by_id[experimental_compile_id]["dependencies"])
        self.assertIn(
            "projects/comfy/ComfyUI/comfy_extras/nodes_torch_compile.py",
            tasks_by_id[experimental_compile_id]["reads"],
        )
        self.assertIn(
            "projects/comfy/ComfyUI/comfy_api/torch_helpers/torch_compile.py",
            tasks_by_id[experimental_compile_id]["reads"],
        )
        self.assertIn("compile-policy bridge", tasks_by_id[experimental_compile_id]["done"])

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
