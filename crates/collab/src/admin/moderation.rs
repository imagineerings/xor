use std::fmt;

use async_trait::async_trait;
use collaboration_domain::{
    AggregateId, AggregateVersion, AuditOutcome, AuthorizationAction, AuthorizationResourceKind,
    AuthorizationScope, CommunityId, CommunityMembership, MembershipRole,
    ModerationAuthorizationDecision, ModerationAuthorizationDenial, ModerationAuthorizationRequest,
    ModerationCommandSource, ModerationError, ModerationReport, ModerationReportContext,
    ModerationReportReason, ModerationReportTarget, ModerationResolution, ModerationRestriction,
    NostrPublicKey, OperationId, PrincipalId, authorize_with_moderation,
};

use crate::audit::events::{AuditEventContext, ModerationAuditOperation, SecurityAuditEvent};

const MODERATION_REPORT_SCOPE: &str = "moderation:report";
const MODERATION_MANAGE_SCOPE: &str = "moderation:manage";
const COMMUNITY_MANAGE_SCOPE: &str = "communities:manage";
pub const MAX_OPERATOR_REPORTS: u16 = 200;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArchiveVersionFence {
    expected: Option<AggregateVersion>,
    current: Option<AggregateVersion>,
}

impl ArchiveVersionFence {
    pub const fn new(
        expected: Option<AggregateVersion>,
        current: Option<AggregateVersion>,
    ) -> Self {
        Self { expected, current }
    }

    fn next_version(self) -> Result<AggregateVersion, ModerationOperatorError> {
        match (self.expected, self.current) {
            (None, None) => Ok(AggregateVersion::FIRST),
            (Some(expected), Some(current)) if expected == current => current
                .next()
                .ok_or(ModerationOperatorError::InvalidRequest),
            (None, Some(_)) | (Some(_), None) | (Some(_), Some(_)) => {
                Err(ModerationOperatorError::StaleAction)
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModerationOperatorCommand {
    ListReports {
        limit: u16,
        source: ModerationCommandSource,
    },
    FileReport {
        report_id: AggregateId,
        target: ModerationReportTarget,
        reason: ModerationReportReason,
        private_context: Option<ModerationReportContext>,
        source: ModerationCommandSource,
    },
    ResolveReport {
        report: ModerationReport,
        expected_version: AggregateVersion,
        resolution: ModerationResolution,
        source: ModerationCommandSource,
    },
    ApplyBan {
        restriction: ModerationRestriction,
        expected_version: AggregateVersion,
        expires_at_millis: Option<u64>,
        target_membership: CommunityMembership,
        current_target_membership_version: AggregateVersion,
        source: ModerationCommandSource,
    },
    ApplyTimeout {
        restriction: ModerationRestriction,
        expected_version: AggregateVersion,
        expires_at_millis: u64,
        target_membership: CommunityMembership,
        current_target_membership_version: AggregateVersion,
        source: ModerationCommandSource,
    },
    ArchiveIdentity {
        target_membership: CommunityMembership,
        current_target_membership_version: AggregateVersion,
        identity_public_key: NostrPublicKey,
        version_fence: ArchiveVersionFence,
        source: ModerationCommandSource,
    },
    ArchiveCommunity {
        version_fence: ArchiveVersionFence,
        source: ModerationCommandSource,
    },
}

impl ModerationOperatorCommand {
    const fn source(&self) -> ModerationCommandSource {
        match self {
            Self::ListReports { source, .. }
            | Self::FileReport { source, .. }
            | Self::ResolveReport { source, .. }
            | Self::ApplyBan { source, .. }
            | Self::ApplyTimeout { source, .. }
            | Self::ArchiveIdentity { source, .. }
            | Self::ArchiveCommunity { source, .. } => *source,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityArchiveCommand {
    pub community_id: CommunityId,
    pub target_principal_id: PrincipalId,
    pub identity_public_key: NostrPublicKey,
    pub archive_version: AggregateVersion,
    pub actor_principal_id: PrincipalId,
    pub source: ModerationCommandSource,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommunityArchiveCommand {
    pub community_id: CommunityId,
    pub archive_version: AggregateVersion,
    pub actor_principal_id: PrincipalId,
    pub source: ModerationCommandSource,
}

#[derive(Clone, Eq, PartialEq)]
pub enum ModerationOperatorWrite {
    Report(ModerationReport),
    Restriction(ModerationRestriction),
    IdentityArchive(IdentityArchiveCommand),
    CommunityArchive(CommunityArchiveCommand),
}

impl fmt::Debug for ModerationOperatorWrite {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Report(report) => formatter.debug_tuple("Report").field(report).finish(),
            Self::Restriction(restriction) => formatter
                .debug_tuple("Restriction")
                .field(restriction)
                .finish(),
            Self::IdentityArchive(command) => formatter
                .debug_struct("IdentityArchive")
                .field("community_id", &command.community_id)
                .field("archive_version", &command.archive_version)
                .field("identity", &"[REDACTED]")
                .finish(),
            Self::CommunityArchive(command) => formatter
                .debug_tuple("CommunityArchive")
                .field(command)
                .finish(),
        }
    }
}

#[derive(Clone, Eq, PartialEq)]
pub enum ModerationBackendCommand {
    ListReports { limit: u16 },
    Write(ModerationOperatorWrite),
}

impl fmt::Debug for ModerationBackendCommand {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ListReports { limit } => formatter
                .debug_struct("ListReports")
                .field("limit", limit)
                .finish(),
            Self::Write(write) => formatter.debug_tuple("Write").field(write).finish(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModerationWriteReceipt {
    pub operation_id: OperationId,
    pub version: AggregateVersion,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModerationBackendResponse {
    Reports(Vec<ModerationReport>),
    Written(ModerationWriteReceipt),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorizedModerationOperation {
    command: ModerationBackendCommand,
    audit_event: SecurityAuditEvent,
    expected_receipt: Option<ModerationWriteReceipt>,
}

impl AuthorizedModerationOperation {
    pub const fn command(&self) -> &ModerationBackendCommand {
        &self.command
    }

    pub const fn audit_event(&self) -> &SecurityAuditEvent {
        &self.audit_event
    }

    pub const fn expected_receipt(&self) -> Option<ModerationWriteReceipt> {
        self.expected_receipt
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModerationBackendError {
    Conflict,
    Unavailable,
    PartialFailure,
    InvalidData,
}

#[async_trait]
pub trait ModerationOperatorBackend: Send + Sync {
    async fn execute(
        &self,
        community_id: CommunityId,
        operation: &AuthorizedModerationOperation,
    ) -> Result<ModerationBackendResponse, ModerationBackendError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModerationOperatorOutcome {
    Reports(Vec<ModerationReport>),
    Applied(ModerationWriteReceipt),
}

pub struct ModerationOperatorApi<B> {
    backend: B,
}

impl<B> ModerationOperatorApi<B>
where
    B: ModerationOperatorBackend,
{
    pub const fn new(backend: B) -> Self {
        Self { backend }
    }

    pub fn into_backend(self) -> B {
        self.backend
    }

    pub async fn execute(
        &self,
        authorization: &ModerationAuthorizationRequest<'_, '_>,
        command: ModerationOperatorCommand,
    ) -> Result<ModerationOperatorOutcome, ModerationOperatorError> {
        let community_id = authorization.authorization.tenant.community_id();
        let limit = match &command {
            ModerationOperatorCommand::ListReports { limit, .. } => Some(*limit),
            _ => None,
        };
        let operation = prepare_operation(authorization, command)?;
        let response = self
            .backend
            .execute(community_id, &operation)
            .await
            .map_err(ModerationOperatorError::from_backend)?;
        validate_response(community_id, limit, operation.expected_receipt, response)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModerationOperatorError {
    AuthorizationDenied,
    TenantMismatch,
    InvalidRequest,
    StaleAction,
    Unavailable,
    PartialFailure,
    InvalidBackendResponse,
}

impl ModerationOperatorError {
    pub const fn diagnostic_code(self) -> &'static str {
        match self {
            Self::AuthorizationDenied => "moderation_operator_denied",
            Self::TenantMismatch => "moderation_operator_tenant_mismatch",
            Self::InvalidRequest => "moderation_operator_invalid_request",
            Self::StaleAction => "moderation_operator_stale_action",
            Self::Unavailable => "moderation_operator_unavailable",
            Self::PartialFailure => "moderation_operator_partial_failure",
            Self::InvalidBackendResponse => "moderation_operator_invalid_backend_response",
        }
    }

    const fn from_backend(error: ModerationBackendError) -> Self {
        match error {
            ModerationBackendError::Conflict => Self::StaleAction,
            ModerationBackendError::Unavailable => Self::Unavailable,
            ModerationBackendError::PartialFailure => Self::PartialFailure,
            ModerationBackendError::InvalidData => Self::InvalidBackendResponse,
        }
    }
}

impl fmt::Display for ModerationOperatorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "moderation operator request failed ({})",
            self.diagnostic_code()
        )
    }
}

impl std::error::Error for ModerationOperatorError {}

fn prepare_operation(
    authorization: &ModerationAuthorizationRequest<'_, '_>,
    command: ModerationOperatorCommand,
) -> Result<AuthorizedModerationOperation, ModerationOperatorError> {
    validate_authorization_shape(authorization.authorization, &command)?;
    match authorize_with_moderation(authorization) {
        ModerationAuthorizationDecision::Allowed(_) => {}
        ModerationAuthorizationDecision::Denied(denial) => {
            return Err(map_policy_denial(denial));
        }
    }

    let request = authorization.authorization;
    let community_id = request.tenant.community_id();
    let actor_principal_id = effective_principal_id(request);
    let source = command.source();
    let context = AuditEventContext::new(
        request.tenant,
        source.operation_id,
        Some(request.principal),
        AuditOutcome::Succeeded,
        None,
        source.occurred_at_millis,
    )
    .map_err(|_| ModerationOperatorError::InvalidRequest)?;

    let (backend_command, audit_operation, subject_principal_id, record_id, expected_receipt) =
        match command {
            ModerationOperatorCommand::ListReports { limit, .. } => {
                require_operator(request)?;
                if !(1..=MAX_OPERATOR_REPORTS).contains(&limit) {
                    return Err(ModerationOperatorError::InvalidRequest);
                }
                (
                    ModerationBackendCommand::ListReports { limit },
                    ModerationAuditOperation::ViewQueue,
                    None,
                    community_id.as_uuid(),
                    None,
                )
            }
            ModerationOperatorCommand::FileReport {
                report_id,
                target,
                reason,
                private_context,
                source,
            } => {
                let subject_principal_id = match target {
                    ModerationReportTarget::Principal(principal_id) => Some(principal_id),
                    ModerationReportTarget::Event(_) | ModerationReportTarget::BlobSha256(_) => {
                        None
                    }
                };
                let report = ModerationReport::file(
                    report_id,
                    community_id,
                    target,
                    reason,
                    private_context,
                    source,
                    request,
                )
                .map_err(map_domain_error)?;
                let receipt = ModerationWriteReceipt {
                    operation_id: source.operation_id,
                    version: report.fields().version,
                };
                (
                    ModerationBackendCommand::Write(ModerationOperatorWrite::Report(report)),
                    ModerationAuditOperation::Report,
                    subject_principal_id,
                    report_id.as_uuid(),
                    Some(receipt),
                )
            }
            ModerationOperatorCommand::ResolveReport {
                mut report,
                expected_version,
                resolution,
                source,
            } => {
                require_operator(request)?;
                report
                    .resolve(expected_version, resolution, source, request)
                    .map_err(map_domain_error)?;
                let report_id = report.fields().report_id;
                let subject_principal_id = match report.fields().target {
                    ModerationReportTarget::Principal(principal_id) => Some(principal_id),
                    ModerationReportTarget::Event(_) | ModerationReportTarget::BlobSha256(_) => {
                        None
                    }
                };
                let receipt = ModerationWriteReceipt {
                    operation_id: source.operation_id,
                    version: report.fields().version,
                };
                (
                    ModerationBackendCommand::Write(ModerationOperatorWrite::Report(report)),
                    ModerationAuditOperation::ResolveReport,
                    subject_principal_id,
                    report_id.as_uuid(),
                    Some(receipt),
                )
            }
            ModerationOperatorCommand::ApplyBan {
                mut restriction,
                expected_version,
                expires_at_millis,
                target_membership,
                current_target_membership_version,
                source,
            } => {
                require_operator(request)?;
                restriction
                    .apply_ban(
                        expected_version,
                        expires_at_millis,
                        source,
                        target_membership,
                        current_target_membership_version,
                        request,
                    )
                    .map_err(map_domain_error)?;
                let target_principal_id = restriction.fields().target_principal_id;
                let receipt = ModerationWriteReceipt {
                    operation_id: source.operation_id,
                    version: restriction.fields().version,
                };
                (
                    ModerationBackendCommand::Write(ModerationOperatorWrite::Restriction(
                        restriction,
                    )),
                    ModerationAuditOperation::ApplyRestriction,
                    Some(target_principal_id),
                    target_principal_id.as_uuid(),
                    Some(receipt),
                )
            }
            ModerationOperatorCommand::ApplyTimeout {
                mut restriction,
                expected_version,
                expires_at_millis,
                target_membership,
                current_target_membership_version,
                source,
            } => {
                require_operator(request)?;
                restriction
                    .apply_timeout(
                        expected_version,
                        expires_at_millis,
                        source,
                        target_membership,
                        current_target_membership_version,
                        request,
                    )
                    .map_err(map_domain_error)?;
                let target_principal_id = restriction.fields().target_principal_id;
                let receipt = ModerationWriteReceipt {
                    operation_id: source.operation_id,
                    version: restriction.fields().version,
                };
                (
                    ModerationBackendCommand::Write(ModerationOperatorWrite::Restriction(
                        restriction,
                    )),
                    ModerationAuditOperation::ApplyRestriction,
                    Some(target_principal_id),
                    target_principal_id.as_uuid(),
                    Some(receipt),
                )
            }
            ModerationOperatorCommand::ArchiveIdentity {
                target_membership,
                current_target_membership_version,
                identity_public_key,
                version_fence,
                source,
            } => {
                require_operator_target(
                    request,
                    target_membership,
                    current_target_membership_version,
                )?;
                let archive_version = version_fence.next_version()?;
                let command = IdentityArchiveCommand {
                    community_id,
                    target_principal_id: target_membership.principal_id,
                    identity_public_key,
                    archive_version,
                    actor_principal_id,
                    source,
                };
                let receipt = ModerationWriteReceipt {
                    operation_id: source.operation_id,
                    version: archive_version,
                };
                (
                    ModerationBackendCommand::Write(ModerationOperatorWrite::IdentityArchive(
                        command,
                    )),
                    ModerationAuditOperation::ArchiveIdentity,
                    Some(target_membership.principal_id),
                    target_membership.principal_id.as_uuid(),
                    Some(receipt),
                )
            }
            ModerationOperatorCommand::ArchiveCommunity {
                version_fence,
                source,
            } => {
                require_owner(request)?;
                let archive_version = version_fence.next_version()?;
                let command = CommunityArchiveCommand {
                    community_id,
                    archive_version,
                    actor_principal_id,
                    source,
                };
                let receipt = ModerationWriteReceipt {
                    operation_id: source.operation_id,
                    version: archive_version,
                };
                (
                    ModerationBackendCommand::Write(ModerationOperatorWrite::CommunityArchive(
                        command,
                    )),
                    ModerationAuditOperation::ArchiveCommunity,
                    None,
                    community_id.as_uuid(),
                    Some(receipt),
                )
            }
        };

    let audit_event = SecurityAuditEvent::Moderation {
        context,
        operation: audit_operation,
        subject_principal_id,
        record_id,
    };
    audit_event
        .clone()
        .into_record()
        .map_err(|_| ModerationOperatorError::InvalidRequest)?;
    Ok(AuthorizedModerationOperation {
        command: backend_command,
        audit_event,
        expected_receipt,
    })
}

fn validate_authorization_shape(
    request: &collaboration_domain::AuthorizationRequest<'_>,
    command: &ModerationOperatorCommand,
) -> Result<(), ModerationOperatorError> {
    let (scope, action, resource_kind) = match command {
        ModerationOperatorCommand::ListReports { .. } => (
            MODERATION_MANAGE_SCOPE,
            AuthorizationAction::Read,
            AuthorizationResourceKind::Administration,
        ),
        ModerationOperatorCommand::FileReport { .. } => (
            MODERATION_REPORT_SCOPE,
            AuthorizationAction::Write,
            AuthorizationResourceKind::Community,
        ),
        ModerationOperatorCommand::ResolveReport { .. }
        | ModerationOperatorCommand::ApplyBan { .. }
        | ModerationOperatorCommand::ApplyTimeout { .. }
        | ModerationOperatorCommand::ArchiveIdentity { .. } => (
            MODERATION_MANAGE_SCOPE,
            AuthorizationAction::Manage,
            AuthorizationResourceKind::Administration,
        ),
        ModerationOperatorCommand::ArchiveCommunity { .. } => (
            COMMUNITY_MANAGE_SCOPE,
            AuthorizationAction::Delete,
            AuthorizationResourceKind::Community,
        ),
    };
    let expected_scope =
        AuthorizationScope::new(scope).map_err(|_| ModerationOperatorError::InvalidRequest)?;
    if request.required_scope != &expected_scope
        || request.action != action
        || request.resource.kind != resource_kind
        || request.resource.resource_id.as_uuid() != request.tenant.community_id().as_uuid()
    {
        return Err(ModerationOperatorError::AuthorizationDenied);
    }
    Ok(())
}

fn require_operator(
    request: &collaboration_domain::AuthorizationRequest<'_>,
) -> Result<MembershipRole, ModerationOperatorError> {
    let role = request
        .community_membership
        .map(|membership| membership.role)
        .ok_or(ModerationOperatorError::AuthorizationDenied)?;
    match role {
        MembershipRole::Owner | MembershipRole::Admin => Ok(role),
        MembershipRole::Member | MembershipRole::Guest | MembershipRole::Bot => {
            Err(ModerationOperatorError::AuthorizationDenied)
        }
    }
}

fn require_owner(
    request: &collaboration_domain::AuthorizationRequest<'_>,
) -> Result<(), ModerationOperatorError> {
    match require_operator(request)? {
        MembershipRole::Owner => Ok(()),
        MembershipRole::Admin => Err(ModerationOperatorError::AuthorizationDenied),
        MembershipRole::Member | MembershipRole::Guest | MembershipRole::Bot => {
            Err(ModerationOperatorError::AuthorizationDenied)
        }
    }
}

fn require_operator_target(
    request: &collaboration_domain::AuthorizationRequest<'_>,
    target_membership: CommunityMembership,
    current_target_membership_version: AggregateVersion,
) -> Result<(), ModerationOperatorError> {
    let actor_role = require_operator(request)?;
    let actor_principal_id = effective_principal_id(request);
    if target_membership.community_id != request.tenant.community_id() {
        return Err(ModerationOperatorError::TenantMismatch);
    }
    if target_membership.version != current_target_membership_version {
        return Err(ModerationOperatorError::StaleAction);
    }
    if target_membership.principal_id == actor_principal_id
        || target_membership.role == MembershipRole::Owner
        || (actor_role == MembershipRole::Admin && target_membership.role == MembershipRole::Admin)
    {
        return Err(ModerationOperatorError::AuthorizationDenied);
    }
    Ok(())
}

fn effective_principal_id(request: &collaboration_domain::AuthorizationRequest<'_>) -> PrincipalId {
    match request.principal.kind() {
        collaboration_domain::AuthenticatedPrincipalKind::ScopedToken {
            subject_principal_id,
            ..
        } => *subject_principal_id,
        _ => request.principal.principal_id(),
    }
}

fn map_policy_denial(denial: ModerationAuthorizationDenial) -> ModerationOperatorError {
    match denial {
        ModerationAuthorizationDenial::TenantMismatch => ModerationOperatorError::TenantMismatch,
        ModerationAuthorizationDenial::PolicyUnavailable
        | ModerationAuthorizationDenial::AmbiguousPolicyState => {
            ModerationOperatorError::Unavailable
        }
        ModerationAuthorizationDenial::Authorization(_)
        | ModerationAuthorizationDenial::InvalidPolicyInput
        | ModerationAuthorizationDenial::Banned
        | ModerationAuthorizationDenial::TimedOut
        | ModerationAuthorizationDenial::IdentityArchived
        | ModerationAuthorizationDenial::CommunityArchived => {
            ModerationOperatorError::AuthorizationDenied
        }
    }
}

fn map_domain_error(error: ModerationError) -> ModerationOperatorError {
    match error {
        ModerationError::TenantMismatch => ModerationOperatorError::TenantMismatch,
        ModerationError::StaleTarget
        | ModerationError::StaleVersion { .. }
        | ModerationError::ConflictingOperation => ModerationOperatorError::StaleAction,
        ModerationError::AuthorizationShape
        | ModerationError::Unauthorized(_)
        | ModerationError::ProtectedTarget
        | ModerationError::SelfRestriction
        | ModerationError::PersonalMuteOwnerMismatch => {
            ModerationOperatorError::AuthorizationDenied
        }
        ModerationError::InvalidIdentity
        | ModerationError::InvalidOperationId
        | ModerationError::InvalidReportContext
        | ModerationError::InvalidTimestamp
        | ModerationError::InvalidExpiry
        | ModerationError::InvalidTransition
        | ModerationError::SelfMute
        | ModerationError::TooManyTransitions
        | ModerationError::VersionExhausted
        | ModerationError::InvalidRecord => ModerationOperatorError::InvalidRequest,
    }
}

fn validate_response(
    community_id: CommunityId,
    limit: Option<u16>,
    expected_receipt: Option<ModerationWriteReceipt>,
    response: ModerationBackendResponse,
) -> Result<ModerationOperatorOutcome, ModerationOperatorError> {
    match (limit, expected_receipt, response) {
        (Some(limit), None, ModerationBackendResponse::Reports(reports))
            if reports.len() <= usize::from(limit)
                && reports
                    .iter()
                    .all(|report| report.fields().community_id == community_id) =>
        {
            Ok(ModerationOperatorOutcome::Reports(reports))
        }
        (None, Some(expected), ModerationBackendResponse::Written(actual))
            if actual == expected =>
        {
            Ok(ModerationOperatorOutcome::Applied(actual))
        }
        _ => Err(ModerationOperatorError::InvalidBackendResponse),
    }
}
