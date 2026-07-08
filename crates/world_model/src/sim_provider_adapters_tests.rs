use crate::{
    SIM_PROVIDER_ADAPTER_UNSUPPORTED_OPERATION_CODE,
    SIM_PROVIDER_CONNECTOR_CANCEL_UNSUPPORTED_CODE, SIM_PROVIDER_CONNECTOR_START_FAILED_CODE,
    SimProviderAdapterCatalog, SimProviderAdapterFamily, SimProviderCapability,
    SimProviderConnector, SimProviderId, SimProviderPolicyRequest, SimProviderRemoteTaskStatus,
};

#[test]
fn provider_adapter_catalog_exposes_native_skeletons_for_required_families() {
    let catalog = SimProviderAdapterCatalog::default();
    let families = catalog
        .definitions()
        .iter()
        .map(|definition| definition.family)
        .collect::<std::collections::BTreeSet<_>>();

    for family in [
        SimProviderAdapterFamily::OpenAi,
        SimProviderAdapterFamily::Gemini,
        SimProviderAdapterFamily::AnthropicOpenRouter,
        SimProviderAdapterFamily::ImageVideo,
        SimProviderAdapterFamily::Audio,
        SimProviderAdapterFamily::ThreeD,
    ] {
        assert!(
            families.contains(&family),
            "missing adapter family {family:?}"
        );
    }

    for definition in catalog.definitions() {
        assert!(
            definition
                .native_handler_prefix
                .starts_with("sim.provider."),
            "adapter skeletons must expose native Sim handlers"
        );
        assert!(
            !definition.comfy_node_ids.is_empty(),
            "adapter skeleton must declare Comfy-compatible node ids"
        );
        assert!(
            definition
                .credential_keys
                .iter()
                .all(|credential_key| !credential_key.is_empty()),
            "adapter credentials must resolve through named Sim secret keys"
        );
    }
}

#[test]
fn provider_adapter_skeleton_starts_supported_native_task() {
    let catalog = SimProviderAdapterCatalog::default();
    let provider_id = SimProviderId::new("openai");
    let mut connector = catalog
        .connector(&provider_id)
        .expect("openai adapter skeleton");
    let request = SimProviderPolicyRequest::new(
        provider_id,
        SimProviderCapability::TextToImage,
        "OpenAIImageGenerate",
        "sim.provider.openai.OpenAIImageGenerate",
    );

    let handle = connector.start(request).expect("native task handle");

    assert_eq!(handle.provider_id.as_str(), "openai");
    assert_eq!(handle.comfy_node_id, "OpenAIImageGenerate");
    assert_eq!(
        handle.native_handler,
        "sim.provider.openai.OpenAIImageGenerate"
    );
    assert!(
        handle
            .remote_task_id
            .as_str()
            .starts_with("sim-provider-adapter-openai-")
    );

    assert!(matches!(
        connector.poll(&handle).expect("poll skeleton"),
        SimProviderRemoteTaskStatus::Running { .. }
    ));
}

#[test]
fn provider_adapter_skeleton_gates_unsupported_operations_with_diagnostics() {
    let catalog = SimProviderAdapterCatalog::default();
    let provider_id = SimProviderId::new("runway");
    let mut connector = catalog
        .connector(&provider_id)
        .expect("runway adapter skeleton");
    let request = SimProviderPolicyRequest::new(
        provider_id,
        SimProviderCapability::ImageToVideo,
        "LumaImageToVideo",
        "sim.provider.runway.LumaImageToVideo",
    );

    let error = connector
        .start(request)
        .expect_err("unsupported operation should be gated");

    assert_eq!(error.code, SIM_PROVIDER_ADAPTER_UNSUPPORTED_OPERATION_CODE);
    assert!(
        catalog
            .unsupported_diagnostics()
            .iter()
            .any(
                |diagnostic| diagnostic.code == SIM_PROVIDER_ADAPTER_UNSUPPORTED_OPERATION_CODE
                    && diagnostic.provider_id.as_str() == "runway"
                    && diagnostic.comfy_node_id == "LumaImageToVideo"
            )
    );
}

#[test]
fn provider_adapter_skeleton_rejects_wrong_provider_and_unsupported_cancel() {
    let catalog = SimProviderAdapterCatalog::default();
    let provider_id = SimProviderId::new("gemini");
    let mut connector = catalog
        .connector(&provider_id)
        .expect("gemini adapter skeleton");
    let request = SimProviderPolicyRequest::new(
        SimProviderId::new("openai"),
        SimProviderCapability::PromptEnhancement,
        "GeminiPromptEnhance",
        "sim.provider.gemini.GeminiPromptEnhance",
    );

    let error = connector
        .start(request)
        .expect_err("wrong provider should fail");

    assert_eq!(error.code, SIM_PROVIDER_CONNECTOR_START_FAILED_CODE);

    let request = SimProviderPolicyRequest::new(
        provider_id,
        SimProviderCapability::PromptEnhancement,
        "GeminiPromptEnhance",
        "sim.provider.gemini.GeminiPromptEnhance",
    );
    let handle = connector.start(request).expect("native task handle");
    let error = connector
        .cancel(&handle)
        .expect_err("adapter skeleton cancellation is unsupported");

    assert_eq!(error.code, SIM_PROVIDER_CONNECTOR_CANCEL_UNSUPPORTED_CODE);
}
