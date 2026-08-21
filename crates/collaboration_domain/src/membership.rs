use std::{error::Error, fmt};

use crate::{
    AggregateId, AggregateVersion, AuthenticatedPrincipalKind, AuthorizationAction,
    AuthorizationDecision, AuthorizationDenial, AuthorizationRequest, AuthorizationResourceKind,
    ChannelMembership, CommunityId, CommunityMembership, InviteRedemption, MembershipRole,
    MembershipStatus, PrincipalId, VirtualAgentMembershipEvidence, authorize,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MembershipScope {
    Community,
    Channel(AggregateId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MembershipRecordFields {
    pub community_id: CommunityId,
    pub scope: MembershipScope,
    pub principal_id: PrincipalId,
    pub role: MembershipRole,
    pub status: MembershipStatus,
    pub version: AggregateVersion,
    pub added_by_principal_id: Option<PrincipalId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MembershipCreateFields {
    pub community_id: CommunityId,
    pub scope: MembershipScope,
    pub principal_id: PrincipalId,
    pub role: MembershipRole,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MembershipCommandOutcome {
    Applied,
    Unchanged,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MembershipPolicyInput {
    PersistentCommunity(CommunityMembership),
    PersistentChannel(ChannelMembership),
    VirtualCommunity(CommunityMembership),
}

impl MembershipPolicyInput {
    pub const fn community_membership(self) -> Option<CommunityMembership> {
        match self {
            Self::PersistentCommunity(membership) | Self::VirtualCommunity(membership) => {
                Some(membership)
            }
            Self::PersistentChannel(_) => None,
        }
    }

    pub const fn channel_membership(self) -> Option<ChannelMembership> {
        match self {
            Self::PersistentChannel(membership) => Some(membership),
            Self::PersistentCommunity(_) | Self::VirtualCommunity(_) => None,
        }
    }

    pub const fn is_virtual(self) -> bool {
        matches!(self, Self::VirtualCommunity(_))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InviteMembershipProjection {
    membership: Membership,
    inserted: bool,
}

impl InviteMembershipProjection {
    pub const fn membership(self) -> Membership {
        self.membership
    }

    pub const fn inserted(self) -> bool {
        self.inserted
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Membership {
    fields: MembershipRecordFields,
}

impl Membership {
    pub const fn from_record(fields: MembershipRecordFields) -> Self {
        Self { fields }
    }

    pub fn add(
        fields: MembershipCreateFields,
        authorization: &AuthorizationRequest<'_>,
    ) -> Result<Self, MembershipError> {
        authorize_membership(authorization, fields.community_id, fields.scope)?;
        require_distinct_actor(authorization, fields.principal_id)?;
        require_role_grant(actor_role(authorization, fields.scope)?, fields.role)?;
        Ok(Self {
            fields: MembershipRecordFields {
                community_id: fields.community_id,
                scope: fields.scope,
                principal_id: fields.principal_id,
                role: fields.role,
                status: MembershipStatus::Active,
                version: AggregateVersion::FIRST,
                added_by_principal_id: Some(authorization_subject(authorization)),
            },
        })
    }

    pub fn from_invite_redemption(
        redemption: &InviteRedemption,
        added_by_principal_id: Option<PrincipalId>,
    ) -> InviteMembershipProjection {
        let source = redemption.membership();
        let inserted = matches!(redemption, InviteRedemption::Joined { .. });
        InviteMembershipProjection {
            membership: Self::from_record(MembershipRecordFields {
                community_id: source.community_id,
                scope: MembershipScope::Community,
                principal_id: source.principal_id,
                role: source.role,
                status: source.status,
                version: source.version,
                added_by_principal_id,
            }),
            inserted,
        }
    }

    pub const fn virtual_policy_input(
        evidence: VirtualAgentMembershipEvidence,
    ) -> MembershipPolicyInput {
        MembershipPolicyInput::VirtualCommunity(evidence.policy_membership_snapshot())
    }

    pub const fn fields(self) -> MembershipRecordFields {
        self.fields
    }

    pub const fn policy_input(self) -> MembershipPolicyInput {
        match self.fields.scope {
            MembershipScope::Community => {
                MembershipPolicyInput::PersistentCommunity(CommunityMembership {
                    community_id: self.fields.community_id,
                    principal_id: self.fields.principal_id,
                    role: self.fields.role,
                    status: self.fields.status,
                    version: self.fields.version,
                })
            }
            MembershipScope::Channel(channel_id) => {
                MembershipPolicyInput::PersistentChannel(ChannelMembership {
                    community_id: self.fields.community_id,
                    channel_id,
                    principal_id: self.fields.principal_id,
                    role: self.fields.role,
                    status: self.fields.status,
                    version: self.fields.version,
                })
            }
        }
    }

    pub fn change_role(
        &mut self,
        expected_version: AggregateVersion,
        new_role: MembershipRole,
        authorization: &AuthorizationRequest<'_>,
    ) -> Result<MembershipCommandOutcome, MembershipError> {
        self.authorize_mutation(authorization)?;
        self.require_version(expected_version)?;
        self.require_active()?;
        require_distinct_actor(authorization, self.fields.principal_id)?;
        let actor_role = actor_role(authorization, self.fields.scope)?;
        if actor_role != MembershipRole::Owner {
            return Err(MembershipError::RoleEscalation);
        }
        if new_role == MembershipRole::Owner
            || (self.fields.scope == MembershipScope::Community
                && self.fields.role == MembershipRole::Owner)
        {
            return Err(MembershipError::ProtectedOwner);
        }
        if self.fields.role == new_role {
            return Ok(MembershipCommandOutcome::Unchanged);
        }
        self.fields.role = new_role;
        self.advance_version()?;
        Ok(MembershipCommandOutcome::Applied)
    }

    pub fn revoke(
        &mut self,
        expected_version: AggregateVersion,
        authorization: &AuthorizationRequest<'_>,
    ) -> Result<MembershipCommandOutcome, MembershipError> {
        self.authorize_mutation(authorization)?;
        self.require_version(expected_version)?;
        require_distinct_actor(authorization, self.fields.principal_id)?;
        self.require_removable_by(actor_role(authorization, self.fields.scope)?)?;
        match self.fields.status {
            MembershipStatus::Active | MembershipStatus::Archived => {
                self.fields.status = MembershipStatus::Revoked;
                self.advance_version()?;
                Ok(MembershipCommandOutcome::Applied)
            }
            MembershipStatus::Revoked => Ok(MembershipCommandOutcome::Unchanged),
        }
    }

    pub fn archive(
        &mut self,
        expected_version: AggregateVersion,
        authorization: &AuthorizationRequest<'_>,
    ) -> Result<MembershipCommandOutcome, MembershipError> {
        self.authorize_mutation(authorization)?;
        self.require_version(expected_version)?;
        require_distinct_actor(authorization, self.fields.principal_id)?;
        self.require_removable_by(actor_role(authorization, self.fields.scope)?)?;
        match self.fields.status {
            MembershipStatus::Active => {
                self.fields.status = MembershipStatus::Archived;
                self.advance_version()?;
                Ok(MembershipCommandOutcome::Applied)
            }
            MembershipStatus::Archived => Ok(MembershipCommandOutcome::Unchanged),
            MembershipStatus::Revoked => Err(MembershipError::InvalidTransition),
        }
    }

    pub fn restore(
        &mut self,
        expected_version: AggregateVersion,
        authorization: &AuthorizationRequest<'_>,
    ) -> Result<MembershipCommandOutcome, MembershipError> {
        self.authorize_mutation(authorization)?;
        self.require_version(expected_version)?;
        require_distinct_actor(authorization, self.fields.principal_id)?;
        self.require_removable_by(actor_role(authorization, self.fields.scope)?)?;
        match self.fields.status {
            MembershipStatus::Archived => {
                self.fields.status = MembershipStatus::Active;
                self.advance_version()?;
                Ok(MembershipCommandOutcome::Applied)
            }
            MembershipStatus::Active => Ok(MembershipCommandOutcome::Unchanged),
            MembershipStatus::Revoked => Err(MembershipError::InvalidTransition),
        }
    }

    fn authorize_mutation(
        &self,
        authorization: &AuthorizationRequest<'_>,
    ) -> Result<(), MembershipError> {
        authorize_membership(authorization, self.fields.community_id, self.fields.scope)
    }

    fn require_version(&self, expected_version: AggregateVersion) -> Result<(), MembershipError> {
        if self.fields.version != expected_version {
            return Err(MembershipError::StaleVersion {
                expected: expected_version,
                actual: self.fields.version,
            });
        }
        Ok(())
    }

    fn require_active(&self) -> Result<(), MembershipError> {
        if self.fields.status != MembershipStatus::Active {
            return Err(MembershipError::InvalidTransition);
        }
        Ok(())
    }

    fn require_removable_by(&self, actor_role: MembershipRole) -> Result<(), MembershipError> {
        if self.fields.scope == MembershipScope::Community
            && self.fields.role == MembershipRole::Owner
        {
            return Err(MembershipError::ProtectedOwner);
        }
        let allowed = match actor_role {
            MembershipRole::Owner => true,
            MembershipRole::Admin => matches!(
                self.fields.role,
                MembershipRole::Member | MembershipRole::Guest | MembershipRole::Bot
            ),
            MembershipRole::Member | MembershipRole::Guest | MembershipRole::Bot => false,
        };
        if !allowed {
            return Err(MembershipError::RoleEscalation);
        }
        Ok(())
    }

    fn advance_version(&mut self) -> Result<(), MembershipError> {
        self.fields.version = self
            .fields
            .version
            .next()
            .ok_or(MembershipError::VersionExhausted)?;
        Ok(())
    }
}

fn authorize_membership(
    request: &AuthorizationRequest<'_>,
    community_id: CommunityId,
    scope: MembershipScope,
) -> Result<(), MembershipError> {
    if request.action != AuthorizationAction::Manage
        || request.resource.community_id != community_id
    {
        return Err(MembershipError::AuthorizationShape);
    }
    let resource_matches = match scope {
        MembershipScope::Community => {
            request.resource.kind == AuthorizationResourceKind::Community
                && request.resource.resource_id == AggregateId::from_uuid(community_id.as_uuid())
                && request.resource.channel_id.is_none()
        }
        MembershipScope::Channel(channel_id) => {
            request.resource.kind == AuthorizationResourceKind::Channel
                && request.resource.resource_id == channel_id
                && request.resource.channel_id == Some(channel_id)
        }
    };
    if !resource_matches {
        return Err(MembershipError::AuthorizationShape);
    }
    match authorize(request) {
        AuthorizationDecision::Allowed => Ok(()),
        AuthorizationDecision::Denied(denial) => Err(MembershipError::Unauthorized(denial)),
    }
}

fn actor_role(
    request: &AuthorizationRequest<'_>,
    scope: MembershipScope,
) -> Result<MembershipRole, MembershipError> {
    let role = match scope {
        MembershipScope::Community => request
            .community_membership
            .map(|membership| membership.role),
        MembershipScope::Channel(_) => request.channel_membership.map(|membership| membership.role),
    };
    role.ok_or(MembershipError::MissingActorMembership)
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

fn require_distinct_actor(
    request: &AuthorizationRequest<'_>,
    target_principal_id: PrincipalId,
) -> Result<(), MembershipError> {
    if authorization_subject(request) == target_principal_id {
        return Err(MembershipError::SelfMutation);
    }
    Ok(())
}

fn require_role_grant(
    actor_role: MembershipRole,
    target_role: MembershipRole,
) -> Result<(), MembershipError> {
    let allowed = match actor_role {
        MembershipRole::Owner => target_role != MembershipRole::Owner,
        MembershipRole::Admin => matches!(
            target_role,
            MembershipRole::Member | MembershipRole::Guest | MembershipRole::Bot
        ),
        MembershipRole::Member | MembershipRole::Guest | MembershipRole::Bot => false,
    };
    if !allowed {
        return Err(if target_role == MembershipRole::Owner {
            MembershipError::ProtectedOwner
        } else {
            MembershipError::RoleEscalation
        });
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MembershipError {
    AuthorizationShape,
    Unauthorized(AuthorizationDenial),
    MissingActorMembership,
    SelfMutation,
    ProtectedOwner,
    RoleEscalation,
    StaleVersion {
        expected: AggregateVersion,
        actual: AggregateVersion,
    },
    InvalidTransition,
    VersionExhausted,
}

impl fmt::Display for MembershipError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AuthorizationShape | Self::Unauthorized(_) | Self::MissingActorMembership => {
                formatter.write_str("membership command is not authorized")
            }
            Self::SelfMutation => formatter.write_str("membership self-mutation is forbidden"),
            Self::ProtectedOwner => formatter.write_str("owner membership is protected"),
            Self::RoleEscalation => formatter.write_str("membership role change is forbidden"),
            Self::StaleVersion { .. } => formatter.write_str("membership version is stale"),
            Self::InvalidTransition => formatter.write_str("membership transition is invalid"),
            Self::VersionExhausted => formatter.write_str("membership version is exhausted"),
        }
    }
}

impl Error for MembershipError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AuthorizationResource, AuthorizationScope, InviteAdmissionEvidence, InviteId,
        PrincipalScopes, ServiceAccountId, TenantContext, TrustedTenantRoute,
    };
    use uuid::Uuid;

    fn community(value: u128) -> CommunityId {
        CommunityId::from_uuid(Uuid::from_u128(value))
    }

    fn principal(value: u128) -> PrincipalId {
        PrincipalId::from_uuid(Uuid::from_u128(value))
    }

    fn aggregate(value: u128) -> AggregateId {
        AggregateId::from_uuid(Uuid::from_u128(value))
    }

    fn scope() -> AuthorizationScope {
        AuthorizationScope::new("communities:manage").expect("scope")
    }

    fn principal_scopes() -> PrincipalScopes {
        PrincipalScopes::new([scope()]).expect("scopes")
    }

    fn tenant(community_id: CommunityId) -> TenantContext {
        TenantContext::establish(
            Some(
                TrustedTenantRoute::from_listener(community_id, "membership-test").expect("route"),
            ),
            &[],
        )
        .expect("tenant")
    }

    fn authenticated(
        community_id: CommunityId,
        actor_principal_id: PrincipalId,
    ) -> crate::AuthenticatedPrincipal {
        crate::AuthenticatedPrincipal::zed_account(
            actor_principal_id,
            community_id,
            ServiceAccountId::new(1),
            principal_scopes(),
        )
    }

    fn policy_membership(
        community_id: CommunityId,
        actor_principal_id: PrincipalId,
        role: MembershipRole,
        version: AggregateVersion,
    ) -> CommunityMembership {
        CommunityMembership {
            community_id,
            principal_id: actor_principal_id,
            role,
            status: MembershipStatus::Active,
            version,
        }
    }

    fn request<'a>(
        tenant: &'a TenantContext,
        authenticated: &'a crate::AuthenticatedPrincipal,
        required_scope: &'a AuthorizationScope,
        membership: CommunityMembership,
        current_version: AggregateVersion,
    ) -> AuthorizationRequest<'a> {
        let community_id = tenant.community_id();
        AuthorizationRequest {
            tenant,
            principal: authenticated,
            required_scope,
            action: AuthorizationAction::Manage,
            resource: AuthorizationResource {
                community_id,
                kind: AuthorizationResourceKind::Community,
                resource_id: AggregateId::from_uuid(community_id.as_uuid()),
                owner_principal_id: None,
                channel_id: None,
            },
            current_membership_version: current_version,
            community_membership: Some(membership),
            current_channel_membership_version: None,
            channel_membership: None,
            delegation: None,
            now_millis: 100,
        }
    }

    fn active_member(
        community_id: CommunityId,
        target_principal_id: PrincipalId,
        role: MembershipRole,
    ) -> Membership {
        Membership::from_record(MembershipRecordFields {
            community_id,
            scope: MembershipScope::Community,
            principal_id: target_principal_id,
            role,
            status: MembershipStatus::Active,
            version: AggregateVersion::FIRST,
            added_by_principal_id: None,
        })
    }

    #[test]
    fn invite_projection_preserves_join_and_retry_semantics() {
        let community_id = community(1);
        let target = principal(2);
        let evidence = InviteAdmissionEvidence::new(
            InviteId::from_uuid(Uuid::from_u128(3)),
            community_id,
            Some(2),
            0,
            1_000,
            false,
            AggregateVersion::FIRST,
        )
        .expect("invite");
        let redemption = evidence
            .redeem(
                community_id,
                target,
                None,
                AggregateVersion::FIRST,
                AggregateVersion::FIRST,
                100,
            )
            .expect("redemption");
        let projection = Membership::from_invite_redemption(&redemption, None);
        assert!(projection.inserted());
        assert_eq!(
            projection.membership().policy_input(),
            MembershipPolicyInput::PersistentCommunity(redemption.membership())
        );

        let retry = redemption
            .evidence()
            .clone()
            .redeem(
                community_id,
                target,
                Some(redemption.membership()),
                redemption.evidence().version(),
                AggregateVersion::FIRST,
                100,
            )
            .expect("retry");
        assert!(!Membership::from_invite_redemption(&retry, None).inserted());
    }

    #[test]
    fn channel_membership_projects_an_exact_nip29_policy_input() {
        let community_id = community(1);
        let channel_id = aggregate(2);
        let member = Membership::from_record(MembershipRecordFields {
            community_id,
            scope: MembershipScope::Channel(channel_id),
            principal_id: principal(3),
            role: MembershipRole::Guest,
            status: MembershipStatus::Archived,
            version: AggregateVersion::new(4).expect("version"),
            added_by_principal_id: Some(principal(1)),
        });

        assert_eq!(
            member.policy_input(),
            MembershipPolicyInput::PersistentChannel(ChannelMembership {
                community_id,
                channel_id,
                principal_id: principal(3),
                role: MembershipRole::Guest,
                status: MembershipStatus::Archived,
                version: AggregateVersion::new(4).expect("version"),
            })
        );
    }

    #[test]
    fn owner_changes_roles_while_admin_cannot_escalate() {
        let community_id = community(1);
        let owner_id = principal(1);
        let target = principal(2);
        let tenant = tenant(community_id);
        let owner_principal = authenticated(community_id, owner_id);
        let required_scope = scope();
        let owner = policy_membership(
            community_id,
            owner_id,
            MembershipRole::Owner,
            AggregateVersion::FIRST,
        );
        let owner_request = request(
            &tenant,
            &owner_principal,
            &required_scope,
            owner,
            AggregateVersion::FIRST,
        );
        let mut membership = active_member(community_id, target, MembershipRole::Member);
        assert_eq!(
            membership
                .change_role(
                    AggregateVersion::FIRST,
                    MembershipRole::Admin,
                    &owner_request,
                )
                .expect("owner role change"),
            MembershipCommandOutcome::Applied
        );

        let admin_id = principal(3);
        let admin_principal = authenticated(community_id, admin_id);
        let admin = policy_membership(
            community_id,
            admin_id,
            MembershipRole::Admin,
            AggregateVersion::FIRST,
        );
        let admin_request = request(
            &tenant,
            &admin_principal,
            &required_scope,
            admin,
            AggregateVersion::FIRST,
        );
        let version = AggregateVersion::FIRST.next().expect("second");
        assert_eq!(
            membership.change_role(version, MembershipRole::Member, &admin_request),
            Err(MembershipError::RoleEscalation)
        );
    }

    #[test]
    fn removal_and_archive_preserve_owner_and_revocation_guards() {
        let community_id = community(1);
        let owner_id = principal(1);
        let target = principal(2);
        let tenant = tenant(community_id);
        let authenticated = authenticated(community_id, owner_id);
        let required_scope = scope();
        let owner = policy_membership(
            community_id,
            owner_id,
            MembershipRole::Owner,
            AggregateVersion::FIRST,
        );
        let owner_request = request(
            &tenant,
            &authenticated,
            &required_scope,
            owner,
            AggregateVersion::FIRST,
        );
        let mut membership = active_member(community_id, target, MembershipRole::Member);
        assert_eq!(
            membership
                .archive(AggregateVersion::FIRST, &owner_request)
                .expect("archive"),
            MembershipCommandOutcome::Applied
        );
        let second = AggregateVersion::FIRST.next().expect("second");
        assert_eq!(
            membership.restore(second, &owner_request).expect("restore"),
            MembershipCommandOutcome::Applied
        );
        let third = second.next().expect("third");
        membership
            .revoke(third, &owner_request)
            .expect("revoke membership");
        let fourth = third.next().expect("fourth");
        assert_eq!(
            membership.restore(fourth, &owner_request),
            Err(MembershipError::InvalidTransition)
        );

        let mut protected_owner = active_member(community_id, target, MembershipRole::Owner);
        assert_eq!(
            protected_owner.revoke(AggregateVersion::FIRST, &owner_request),
            Err(MembershipError::ProtectedOwner)
        );
    }

    #[test]
    fn changed_membership_invalidates_a_cached_authorization_snapshot() {
        let community_id = community(1);
        let actor_id = principal(1);
        let target = principal(2);
        let tenant = tenant(community_id);
        let authenticated = authenticated(community_id, actor_id);
        let required_scope = scope();
        let owner = policy_membership(
            community_id,
            actor_id,
            MembershipRole::Owner,
            AggregateVersion::FIRST,
        );
        let owner_request = request(
            &tenant,
            &authenticated,
            &required_scope,
            owner,
            AggregateVersion::FIRST,
        );
        let mut membership = active_member(community_id, target, MembershipRole::Member);
        let cached = membership
            .policy_input()
            .community_membership()
            .expect("community membership");
        membership
            .change_role(
                AggregateVersion::FIRST,
                MembershipRole::Guest,
                &owner_request,
            )
            .expect("role change");
        let current = membership.fields().version;
        let read_scope = AuthorizationScope::new("messages:read").expect("read scope");
        let target_scopes = PrincipalScopes::new([read_scope.clone()]).expect("target scopes");
        let target_principal = crate::AuthenticatedPrincipal::zed_account(
            target,
            community_id,
            ServiceAccountId::new(2),
            target_scopes,
        );
        let cached_request = AuthorizationRequest {
            tenant: &tenant,
            principal: &target_principal,
            required_scope: &read_scope,
            action: AuthorizationAction::Read,
            resource: AuthorizationResource {
                community_id,
                kind: AuthorizationResourceKind::Community,
                resource_id: aggregate(9),
                owner_principal_id: None,
                channel_id: None,
            },
            current_membership_version: current,
            community_membership: Some(cached),
            current_channel_membership_version: None,
            channel_membership: None,
            delegation: None,
            now_millis: 100,
        };
        assert_eq!(
            authorize(&cached_request),
            AuthorizationDecision::Denied(AuthorizationDenial::StaleMembership)
        );
    }
}
