use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use collab::tenant_admission::{AuthorizedRpcRequest, RpcAdmissionError, bind_rpc_tenant};
use collaboration_domain::{
    AggregateId, AggregateVersion, AuthenticatedPrincipal, AuthorizationAction,
    AuthorizationRequest, AuthorizationResource, AuthorizationResourceKind, AuthorizationScope,
    CommunityId, CommunityMembership, MembershipRole, MembershipStatus, PrincipalId,
    PrincipalScopes, ServiceAccountId, TrustedTenantRoute, UntrustedTenantClaim,
    UntrustedTenantClaimSource,
};
use uuid::Uuid;

fn community(value: u128) -> CommunityId {
    CommunityId::from_uuid(Uuid::from_u128(value))
}

fn principal(value: u128) -> PrincipalId {
    PrincipalId::from_uuid(Uuid::from_u128(value))
}

fn request_fixture<'a>(
    tenant: &'a collaboration_domain::TenantContext,
    authenticated: &'a AuthenticatedPrincipal,
    required_scope: &'a AuthorizationScope,
    membership: Option<CommunityMembership>,
) -> AuthorizationRequest<'a> {
    AuthorizationRequest {
        tenant,
        principal: authenticated,
        required_scope,
        action: AuthorizationAction::Read,
        resource: AuthorizationResource {
            community_id: tenant.community_id(),
            kind: AuthorizationResourceKind::Project,
            resource_id: AggregateId::from_uuid(Uuid::from_u128(20)),
            owner_principal_id: None,
            channel_id: None,
        },
        current_membership_version: AggregateVersion::FIRST,
        community_membership: membership,
        current_channel_membership_version: None,
        channel_membership: None,
        delegation: None,
        now_millis: 100,
    }
}

#[tokio::test]
async fn tenant_admission_rpc_authorizes_before_database_queries() {
    let community_id = community(1);
    let principal_id = principal(2);
    let tenant = bind_rpc_tenant(
        Some(TrustedTenantRoute::from_listener(community_id, "sim-rpc").expect("trusted route")),
        &[],
    )
    .expect("tenant");
    let required_scope = AuthorizationScope::new("projects:read").expect("scope");
    let scopes = PrincipalScopes::new([required_scope.clone()]).expect("scopes");
    let authenticated = AuthenticatedPrincipal::sim_account(
        principal_id,
        community_id,
        ServiceAccountId::new(3),
        scopes,
    );
    let queries = Arc::new(AtomicUsize::new(0));

    let denied_request = request_fixture(&tenant, &authenticated, &required_scope, None);
    assert!(matches!(
        AuthorizedRpcRequest::authorize(&denied_request),
        Err(RpcAdmissionError::Denied)
    ));
    assert_eq!(queries.load(Ordering::SeqCst), 0);

    let membership = CommunityMembership {
        community_id,
        principal_id,
        role: MembershipRole::Member,
        status: MembershipStatus::Active,
        version: AggregateVersion::FIRST,
    };
    let allowed_request =
        request_fixture(&tenant, &authenticated, &required_scope, Some(membership));
    let authorized = AuthorizedRpcRequest::authorize(&allowed_request).expect("authorized");
    let result = authorized
        .run({
            let queries = queries.clone();
            move |query_tenant, query_principal| async move {
                queries.fetch_add(1, Ordering::SeqCst);
                assert_eq!(query_tenant.community_id(), community_id);
                assert_eq!(query_principal.principal_id(), principal_id);
                Ok::<_, ()>("queried")
            }
        })
        .await;
    assert_eq!(result, Ok("queried"));
    assert_eq!(queries.load(Ordering::SeqCst), 1);
}

#[test]
fn tenant_admission_rpc_rejects_payload_selected_and_conflicting_tenants() {
    let event_claim = UntrustedTenantClaim::new(community(1), UntrustedTenantClaimSource::EventTag);
    assert_eq!(
        bind_rpc_tenant(None, &[event_claim]),
        Err(RpcAdmissionError::Denied)
    );

    let route = TrustedTenantRoute::from_listener(community(1), "sim-rpc").expect("trusted route");
    let body_claim = UntrustedTenantClaim::new(community(2), UntrustedTenantClaimSource::BodyField);
    assert_eq!(
        bind_rpc_tenant(Some(route), &[body_claim]),
        Err(RpcAdmissionError::Denied)
    );
}
