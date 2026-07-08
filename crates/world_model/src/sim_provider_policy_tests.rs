use crate::{
    SIM_PROVIDER_POLICY_API_DISABLED_CODE, SIM_PROVIDER_POLICY_CAPABILITY_UNAVAILABLE_CODE,
    SIM_PROVIDER_POLICY_COST_CODE, SIM_PROVIDER_POLICY_EXTERNAL_DATA_CODE,
    SIM_PROVIDER_POLICY_MODEL_UNAVAILABLE_CODE, SIM_PROVIDER_POLICY_OFFLINE_CODE,
    SIM_PROVIDER_POLICY_QUOTA_EXCEEDED_CODE, SimProviderCapability, SimProviderCapabilityPolicy,
    SimProviderId, SimProviderNodeRegistry, SimProviderPolicyContext, SimProviderPolicyGate,
    SimProviderPolicyRequest,
};

#[test]
fn provider_policy_blocks_disabled_api_nodes_and_offline_mode() {
    let registry = SimProviderNodeRegistry::default();
    let node = registry.node("OpenAIImageGenerate").expect("provider node");
    let request = SimProviderPolicyRequest::for_node(node);
    let context = SimProviderPolicyContext::default()
        .with_api_nodes_enabled(false)
        .with_offline_mode(true)
        .with_cost_approved(true);

    let decision = SimProviderPolicyGate::new().evaluate(&request, &context);

    assert!(!decision.allowed);
    assert!(
        decision
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == SIM_PROVIDER_POLICY_API_DISABLED_CODE)
    );
    assert!(
        decision
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == SIM_PROVIDER_POLICY_OFFLINE_CODE)
    );
}

#[test]
fn provider_policy_requires_external_data_and_cost_approval() {
    let request = SimProviderPolicyRequest::new(
        SimProviderId::new("runway"),
        SimProviderCapability::TextToVideo,
        "RunwayTextToVideo",
        "sim.provider.runway.RunwayTextToVideo",
    )
    .with_external_data(true)
    .with_cost(true);

    let decision =
        SimProviderPolicyGate::new().evaluate(&request, &SimProviderPolicyContext::default());

    assert!(!decision.allowed);
    assert!(
        decision
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == SIM_PROVIDER_POLICY_EXTERNAL_DATA_CODE)
    );
    assert!(
        decision
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == SIM_PROVIDER_POLICY_COST_CODE)
    );
}

#[test]
fn provider_policy_validates_capability_model_and_quota() {
    let request = SimProviderPolicyRequest::new(
        SimProviderId::new("openai"),
        SimProviderCapability::TextToImage,
        "OpenAIImageGenerate",
        "sim.provider.openai.OpenAIImageGenerate",
    )
    .with_model_id("gpt-image-unavailable")
    .with_estimated_quota_units(8);
    let gate = SimProviderPolicyGate::new().with_capability_policy(
        SimProviderCapabilityPolicy::new(
            SimProviderId::new("openai"),
            SimProviderCapability::TextToImage,
        )
        .with_allowed_model("gpt-image-1")
        .with_remaining_quota_units(4),
    );
    let context = SimProviderPolicyContext::default()
        .with_external_data_approved(true)
        .with_cost_approved(true);

    let decision = gate.evaluate(&request, &context);

    assert!(!decision.allowed);
    assert!(
        decision
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == SIM_PROVIDER_POLICY_MODEL_UNAVAILABLE_CODE)
    );
    assert!(
        decision
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == SIM_PROVIDER_POLICY_QUOTA_EXCEEDED_CODE)
    );
}

#[test]
fn provider_policy_allows_approved_available_native_provider_request() {
    let request = SimProviderPolicyRequest::new(
        SimProviderId::new("elevenlabs"),
        SimProviderCapability::TextToSpeech,
        "ElevenLabsTextToSpeech",
        "sim.provider.elevenlabs.ElevenLabsTextToSpeech",
    )
    .with_external_data(true)
    .with_cost(true)
    .with_model_id("eleven_multilingual_v2")
    .with_estimated_quota_units(2);
    let gate = SimProviderPolicyGate::new().with_capability_policy(
        SimProviderCapabilityPolicy::new(
            SimProviderId::new("elevenlabs"),
            SimProviderCapability::TextToSpeech,
        )
        .with_allowed_model("eleven_multilingual_v2")
        .with_remaining_quota_units(20),
    );
    let context = SimProviderPolicyContext::default()
        .with_external_data_approved(true)
        .with_cost_approved(true);

    let decision = gate.evaluate(&request, &context);

    assert!(decision.allowed);
    assert!(decision.diagnostics.is_empty());
}

#[test]
fn provider_policy_reports_unavailable_capability() {
    let request = SimProviderPolicyRequest::new(
        SimProviderId::new("sam3"),
        SimProviderCapability::ImageEdit,
        "SAM3Segment",
        "sim.provider.sam3.SAM3Segment",
    );
    let gate = SimProviderPolicyGate::new().with_capability_policy(
        SimProviderCapabilityPolicy::new(
            SimProviderId::new("sam3"),
            SimProviderCapability::ImageEdit,
        )
        .unavailable("SAM3 is not available in native Sim provider policy"),
    );
    let context = SimProviderPolicyContext::default();

    let decision = gate.evaluate(&request, &context);

    assert!(!decision.allowed);
    assert!(
        decision
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == SIM_PROVIDER_POLICY_CAPABILITY_UNAVAILABLE_CODE)
    );
}
