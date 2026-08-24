use crate::{
    AggregateVersion, AuthenticatedPrincipalKind, AuthorizationAction, AuthorizationDecision,
    AuthorizationDenial, AuthorizationRequest, CommunityId, ModerationRestriction, PrincipalId,
    authorize,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModerationSnapshot<T> {
    Absent,
    Current(T),
    Unavailable,
    Ambiguous,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityArchivePolicyState {
    Visible,
    Archived,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IdentityArchiveSnapshot {
    pub community_id: CommunityId,
    pub principal_id: PrincipalId,
    pub state: IdentityArchivePolicyState,
    pub version: AggregateVersion,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommunityArchivePolicyState {
    Active,
    Archived,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommunityArchiveSnapshot {
    pub community_id: CommunityId,
    pub state: CommunityArchivePolicyState,
    pub version: AggregateVersion,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HistoricalAttributionSnapshot {
    pub community_id: CommunityId,
    pub principal_id: PrincipalId,
    pub identity_archive: ModerationSnapshot<IdentityArchiveSnapshot>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModerationResourceContext {
    Current,
    HistoricalAttribution(HistoricalAttributionSnapshot),
}

pub struct ModerationAuthorizationRequest<'request, 'authorization> {
    pub authorization: &'request AuthorizationRequest<'authorization>,
    pub restriction: ModerationSnapshot<&'request ModerationRestriction>,
    pub principal_archive: ModerationSnapshot<IdentityArchiveSnapshot>,
    pub community_archive: ModerationSnapshot<CommunityArchiveSnapshot>,
    pub resource_context: ModerationResourceContext,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModerationAuthorization {
    community_id: CommunityId,
    principal_id: PrincipalId,
    action: AuthorizationAction,
    historical_attribution_principal_id: Option<PrincipalId>,
}

impl ModerationAuthorization {
    pub const fn community_id(self) -> CommunityId {
        self.community_id
    }

    pub const fn principal_id(self) -> PrincipalId {
        self.principal_id
    }

    pub const fn action(self) -> AuthorizationAction {
        self.action
    }

    pub const fn historical_attribution_principal_id(self) -> Option<PrincipalId> {
        self.historical_attribution_principal_id
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModerationAuthorizationDecision {
    Allowed(ModerationAuthorization),
    Denied(ModerationAuthorizationDenial),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModerationAuthorizationDenial {
    Authorization(AuthorizationDenial),
    TenantMismatch,
    InvalidPolicyInput,
    PolicyUnavailable,
    AmbiguousPolicyState,
    Banned,
    TimedOut,
    IdentityArchived,
    CommunityArchived,
}

impl ModerationAuthorizationDenial {
    pub const fn diagnostic_code(self) -> &'static str {
        match self {
            Self::Authorization(_) => "base_authorization_denied",
            Self::TenantMismatch => "moderation_tenant_mismatch",
            Self::InvalidPolicyInput => "moderation_policy_input_invalid",
            Self::PolicyUnavailable => "moderation_policy_unavailable",
            Self::AmbiguousPolicyState => "moderation_policy_ambiguous",
            Self::Banned => "moderation_ban_active",
            Self::TimedOut => "moderation_timeout_active",
            Self::IdentityArchived => "moderation_identity_archived",
            Self::CommunityArchived => "moderation_community_archived",
        }
    }
}

pub fn authorize_with_moderation(
    request: &ModerationAuthorizationRequest<'_, '_>,
) -> ModerationAuthorizationDecision {
    if let AuthorizationDecision::Denied(denial) = authorize(request.authorization) {
        return denied(ModerationAuthorizationDenial::Authorization(denial));
    }

    let community_id = request.authorization.tenant.community_id();
    let principal_id = authorization_subject(request.authorization);
    let restriction = match resolve_snapshot(request.restriction) {
        Ok(restriction) => restriction,
        Err(denial) => return denied(denial),
    };
    let principal_archive = match resolve_snapshot(request.principal_archive) {
        Ok(principal_archive) => principal_archive,
        Err(denial) => return denied(denial),
    };
    let community_archive = match resolve_snapshot(request.community_archive) {
        Ok(community_archive) => community_archive,
        Err(denial) => return denied(denial),
    };

    if let Some(restriction) = restriction {
        let fields = restriction.fields();
        if fields.community_id != community_id || fields.target_principal_id != principal_id {
            return denied(ModerationAuthorizationDenial::TenantMismatch);
        }
        if fields.ban.is_active_at(request.authorization.now_millis) {
            return denied(ModerationAuthorizationDenial::Banned);
        }
        if request.authorization.action != AuthorizationAction::Read
            && fields
                .timeout
                .is_active_at(request.authorization.now_millis)
        {
            return denied(ModerationAuthorizationDenial::TimedOut);
        }
    }

    if let Some(archive) = principal_archive {
        if archive.community_id != community_id || archive.principal_id != principal_id {
            return denied(ModerationAuthorizationDenial::TenantMismatch);
        }
        if archive.state == IdentityArchivePolicyState::Archived {
            return denied(ModerationAuthorizationDenial::IdentityArchived);
        }
    }

    if let Some(archive) = community_archive {
        if archive.community_id != community_id {
            return denied(ModerationAuthorizationDenial::TenantMismatch);
        }
        if archive.state == CommunityArchivePolicyState::Archived
            && request.authorization.action != AuthorizationAction::Read
        {
            return denied(ModerationAuthorizationDenial::CommunityArchived);
        }
    }

    let historical_attribution_principal_id = match request.resource_context {
        ModerationResourceContext::Current => None,
        ModerationResourceContext::HistoricalAttribution(attribution) => {
            if request.authorization.action != AuthorizationAction::Read
                || attribution.community_id != community_id
                || attribution.principal_id.as_uuid().is_nil()
            {
                return denied(ModerationAuthorizationDenial::InvalidPolicyInput);
            }
            let attribution_archive = match resolve_snapshot(attribution.identity_archive) {
                Ok(attribution_archive) => attribution_archive,
                Err(denial) => return denied(denial),
            };
            if let Some(archive) = attribution_archive
                && (archive.community_id != community_id
                    || archive.principal_id != attribution.principal_id)
            {
                return denied(ModerationAuthorizationDenial::TenantMismatch);
            }
            Some(attribution.principal_id)
        }
    };

    ModerationAuthorizationDecision::Allowed(ModerationAuthorization {
        community_id,
        principal_id,
        action: request.authorization.action,
        historical_attribution_principal_id,
    })
}

fn resolve_snapshot<T>(
    snapshot: ModerationSnapshot<T>,
) -> Result<Option<T>, ModerationAuthorizationDenial> {
    match snapshot {
        ModerationSnapshot::Absent => Ok(None),
        ModerationSnapshot::Current(value) => Ok(Some(value)),
        ModerationSnapshot::Unavailable => Err(ModerationAuthorizationDenial::PolicyUnavailable),
        ModerationSnapshot::Ambiguous => Err(ModerationAuthorizationDenial::AmbiguousPolicyState),
    }
}

fn authorization_subject(request: &AuthorizationRequest<'_>) -> PrincipalId {
    match request.principal.kind() {
        AuthenticatedPrincipalKind::ScopedToken {
            subject_principal_id,
            ..
        } => *subject_principal_id,
        _ => request.principal.principal_id(),
    }
}

const fn denied(denial: ModerationAuthorizationDenial) -> ModerationAuthorizationDecision {
    ModerationAuthorizationDecision::Denied(denial)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AggregateId, AuthenticatedPrincipal, AuthorizationResource, AuthorizationResourceKind,
        AuthorizationScope, BanState, CommunityMembership, MembershipRole, MembershipStatus,
        ModerationCommandSource, ModerationRestrictionRecordFields, OperationId, PrincipalScopes,
        RestrictionTransition, RestrictionTransitionKind, ServiceAccountId, TimeoutState,
        TrustedTenantRoute,
    };
    use uuid::Uuid;

    const NOW_MILLIS: u64 = 1_000;

    fn community(value: u128) -> CommunityId {
        CommunityId::from_uuid(Uuid::from_u128(value))
    }

    fn principal(value: u128) -> PrincipalId {
        PrincipalId::from_uuid(Uuid::from_u128(value))
    }

    fn aggregate(value: u128) -> AggregateId {
        AggregateId::from_uuid(Uuid::from_u128(value))
    }

    fn source(value: u128, occurred_at_millis: u64) -> ModerationCommandSource {
        ModerationCommandSource {
            operation_id: OperationId::from_uuid(Uuid::from_u128(value)),
            occurred_at_millis,
        }
    }

    fn moderation_restriction(
        community_id: CommunityId,
        target_principal_id: PrincipalId,
        ban: BanState,
        timeout: TimeoutState,
        kind: RestrictionTransitionKind,
        transition_source: ModerationCommandSource,
    ) -> ModerationRestriction {
        let resulting_version = AggregateVersion::FIRST.next().expect("second version");
        ModerationRestriction::from_record(ModerationRestrictionRecordFields {
            community_id,
            target_principal_id,
            ban,
            timeout,
            transitions: vec![RestrictionTransition {
                kind,
                actor_principal_id: principal(90),
                source: transition_source,
                resulting_version,
            }],
            version: resulting_version,
        })
        .expect("restriction")
    }

    struct Fixture {
        tenant: crate::TenantContext,
        principal: AuthenticatedPrincipal,
        scope: AuthorizationScope,
        membership: CommunityMembership,
    }

    impl Fixture {
        fn new() -> Self {
            let community_id = community(1);
            let principal_id = principal(2);
            let scope = AuthorizationScope::new("messages:write").expect("scope");
            let principal = AuthenticatedPrincipal::zed_account(
                principal_id,
                community_id,
                ServiceAccountId::new(1),
                PrincipalScopes::new([scope.clone()]).expect("scopes"),
            );
            let tenant = crate::TenantContext::establish(
                Some(
                    TrustedTenantRoute::from_listener(community_id, "moderation-policy-test")
                        .expect("route"),
                ),
                &[],
            )
            .expect("tenant");
            Self {
                tenant,
                principal,
                scope,
                membership: CommunityMembership {
                    community_id,
                    principal_id,
                    role: MembershipRole::Member,
                    status: MembershipStatus::Active,
                    version: AggregateVersion::FIRST,
                },
            }
        }

        fn authorization(&self, action: AuthorizationAction) -> AuthorizationRequest<'_> {
            AuthorizationRequest {
                tenant: &self.tenant,
                principal: &self.principal,
                required_scope: &self.scope,
                action,
                resource: AuthorizationResource {
                    community_id: self.membership.community_id,
                    kind: AuthorizationResourceKind::Community,
                    resource_id: aggregate(10),
                    owner_principal_id: None,
                    channel_id: None,
                },
                current_membership_version: AggregateVersion::FIRST,
                community_membership: Some(self.membership),
                current_channel_membership_version: None,
                channel_membership: None,
                delegation: None,
                now_millis: NOW_MILLIS,
            }
        }
    }

    fn evaluate(
        authorization: &AuthorizationRequest<'_>,
        restriction: ModerationSnapshot<&ModerationRestriction>,
        principal_archive: ModerationSnapshot<IdentityArchiveSnapshot>,
        community_archive: ModerationSnapshot<CommunityArchiveSnapshot>,
        resource_context: ModerationResourceContext,
    ) -> ModerationAuthorizationDecision {
        authorize_with_moderation(&ModerationAuthorizationRequest {
            authorization,
            restriction,
            principal_archive,
            community_archive,
            resource_context,
        })
    }

    #[test]
    fn active_ban_denies_reads_and_writes() {
        let fixture = Fixture::new();
        let community_id = fixture.membership.community_id;
        let target_principal_id = fixture.membership.principal_id;
        let transition_source = source(20, 500);
        let ban = BanState::Active {
            expires_at_millis: None,
            actor_principal_id: principal(90),
            source: transition_source,
        };
        let restriction = moderation_restriction(
            community_id,
            target_principal_id,
            ban,
            TimeoutState::None,
            RestrictionTransitionKind::ApplyBan {
                expires_at_millis: None,
            },
            transition_source,
        );

        for action in [AuthorizationAction::Read, AuthorizationAction::Write] {
            let authorization = fixture.authorization(action);
            assert_eq!(
                evaluate(
                    &authorization,
                    ModerationSnapshot::Current(&restriction),
                    ModerationSnapshot::Absent,
                    ModerationSnapshot::Absent,
                    ModerationResourceContext::Current,
                ),
                denied(ModerationAuthorizationDenial::Banned)
            );
        }
    }

    #[test]
    fn timeout_blocks_mutation_only_until_its_expiry() {
        let fixture = Fixture::new();
        let community_id = fixture.membership.community_id;
        let target_principal_id = fixture.membership.principal_id;
        let transition_source = source(21, 500);
        let timeout = TimeoutState::Active {
            expires_at_millis: NOW_MILLIS + 1,
            actor_principal_id: principal(90),
            source: transition_source,
        };
        let restriction = moderation_restriction(
            community_id,
            target_principal_id,
            BanState::None,
            timeout,
            RestrictionTransitionKind::ApplyTimeout {
                expires_at_millis: NOW_MILLIS + 1,
            },
            transition_source,
        );
        let read = fixture.authorization(AuthorizationAction::Read);
        let write = fixture.authorization(AuthorizationAction::Write);

        assert!(matches!(
            evaluate(
                &read,
                ModerationSnapshot::Current(&restriction),
                ModerationSnapshot::Absent,
                ModerationSnapshot::Absent,
                ModerationResourceContext::Current,
            ),
            ModerationAuthorizationDecision::Allowed(_)
        ));
        assert_eq!(
            evaluate(
                &write,
                ModerationSnapshot::Current(&restriction),
                ModerationSnapshot::Absent,
                ModerationSnapshot::Absent,
                ModerationResourceContext::Current,
            ),
            denied(ModerationAuthorizationDenial::TimedOut)
        );

        let expired_source = source(22, 500);
        let expired = moderation_restriction(
            community_id,
            target_principal_id,
            BanState::None,
            TimeoutState::Active {
                expires_at_millis: NOW_MILLIS,
                actor_principal_id: principal(90),
                source: expired_source,
            },
            RestrictionTransitionKind::ApplyTimeout {
                expires_at_millis: NOW_MILLIS,
            },
            expired_source,
        );
        assert!(matches!(
            evaluate(
                &write,
                ModerationSnapshot::Current(&expired),
                ModerationSnapshot::Absent,
                ModerationSnapshot::Absent,
                ModerationResourceContext::Current,
            ),
            ModerationAuthorizationDecision::Allowed(_)
        ));
    }

    #[test]
    fn identity_and_community_archives_remove_only_active_authority() {
        let fixture = Fixture::new();
        let community_id = fixture.membership.community_id;
        let principal_id = fixture.membership.principal_id;
        let identity_archive = IdentityArchiveSnapshot {
            community_id,
            principal_id,
            state: IdentityArchivePolicyState::Archived,
            version: AggregateVersion::FIRST,
        };
        let community_archive = CommunityArchiveSnapshot {
            community_id,
            state: CommunityArchivePolicyState::Archived,
            version: AggregateVersion::FIRST,
        };
        let read = fixture.authorization(AuthorizationAction::Read);
        let write = fixture.authorization(AuthorizationAction::Write);

        assert_eq!(
            evaluate(
                &read,
                ModerationSnapshot::Absent,
                ModerationSnapshot::Current(identity_archive),
                ModerationSnapshot::Absent,
                ModerationResourceContext::Current,
            ),
            denied(ModerationAuthorizationDenial::IdentityArchived)
        );
        assert!(matches!(
            evaluate(
                &read,
                ModerationSnapshot::Absent,
                ModerationSnapshot::Absent,
                ModerationSnapshot::Current(community_archive),
                ModerationResourceContext::Current,
            ),
            ModerationAuthorizationDecision::Allowed(_)
        ));
        assert_eq!(
            evaluate(
                &write,
                ModerationSnapshot::Absent,
                ModerationSnapshot::Absent,
                ModerationSnapshot::Current(community_archive),
                ModerationResourceContext::Current,
            ),
            denied(ModerationAuthorizationDenial::CommunityArchived)
        );
    }

    #[test]
    fn historical_read_retains_an_archived_authors_attribution() {
        let fixture = Fixture::new();
        let community_id = fixture.membership.community_id;
        let author_principal_id = principal(30);
        let authorization = fixture.authorization(AuthorizationAction::Read);
        let decision = evaluate(
            &authorization,
            ModerationSnapshot::Absent,
            ModerationSnapshot::Absent,
            ModerationSnapshot::Absent,
            ModerationResourceContext::HistoricalAttribution(HistoricalAttributionSnapshot {
                community_id,
                principal_id: author_principal_id,
                identity_archive: ModerationSnapshot::Current(IdentityArchiveSnapshot {
                    community_id,
                    principal_id: author_principal_id,
                    state: IdentityArchivePolicyState::Archived,
                    version: AggregateVersion::FIRST,
                }),
            }),
        );

        let ModerationAuthorizationDecision::Allowed(authorization) = decision else {
            panic!("historical read should remain authorized");
        };
        assert_eq!(
            authorization.historical_attribution_principal_id(),
            Some(author_principal_id)
        );
        assert_eq!(
            authorization.principal_id(),
            fixture.membership.principal_id
        );
    }

    #[test]
    fn unavailable_ambiguous_and_foreign_policy_inputs_fail_closed() {
        let fixture = Fixture::new();
        let authorization = fixture.authorization(AuthorizationAction::Read);
        assert_eq!(
            evaluate(
                &authorization,
                ModerationSnapshot::Unavailable,
                ModerationSnapshot::Absent,
                ModerationSnapshot::Absent,
                ModerationResourceContext::Current,
            ),
            denied(ModerationAuthorizationDenial::PolicyUnavailable)
        );

        let ambiguous_identity = evaluate(
            &authorization,
            ModerationSnapshot::Absent,
            ModerationSnapshot::Ambiguous,
            ModerationSnapshot::Absent,
            ModerationResourceContext::Current,
        );
        let ambiguous_community = evaluate(
            &authorization,
            ModerationSnapshot::Absent,
            ModerationSnapshot::Absent,
            ModerationSnapshot::Ambiguous,
            ModerationResourceContext::Current,
        );
        let ambiguous_attribution = evaluate(
            &authorization,
            ModerationSnapshot::Absent,
            ModerationSnapshot::Absent,
            ModerationSnapshot::Absent,
            ModerationResourceContext::HistoricalAttribution(HistoricalAttributionSnapshot {
                community_id: fixture.membership.community_id,
                principal_id: principal(30),
                identity_archive: ModerationSnapshot::Ambiguous,
            }),
        );
        for decision in [
            ambiguous_identity,
            ambiguous_community,
            ambiguous_attribution,
        ] {
            assert_eq!(
                decision,
                denied(ModerationAuthorizationDenial::AmbiguousPolicyState)
            );
        }

        assert_eq!(
            evaluate(
                &authorization,
                ModerationSnapshot::Absent,
                ModerationSnapshot::Current(IdentityArchiveSnapshot {
                    community_id: community(99),
                    principal_id: fixture.membership.principal_id,
                    state: IdentityArchivePolicyState::Visible,
                    version: AggregateVersion::FIRST,
                }),
                ModerationSnapshot::Absent,
                ModerationResourceContext::Current,
            ),
            denied(ModerationAuthorizationDenial::TenantMismatch)
        );
        assert_eq!(
            ModerationAuthorizationDenial::AmbiguousPolicyState.diagnostic_code(),
            "moderation_policy_ambiguous"
        );
    }
}
