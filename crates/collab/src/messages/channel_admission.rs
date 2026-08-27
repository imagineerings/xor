use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context as _, Result, anyhow};
use collaboration_domain::{
    AggregateId, AggregateVersion, AuthenticatedPrincipal, AuthorizationAction,
    AuthorizationRequest, AuthorizationResource, AuthorizationResourceKind, AuthorizationScope,
    ChannelMembership, CommunityId, CommunityMembership, MembershipRole, MembershipStatus,
    PrincipalId, PrincipalScopes, ServiceAccountId, TenantContext, TrustedTenantRoute,
    UntrustedTenantClaim, UntrustedTenantClaimSource, channel_id_for_legacy_channel,
    community_id_for_legacy_root_channel, principal_id_for_legacy_user,
};
use sea_orm::{
    ConnectionTrait, DatabaseBackend, DatabaseConnection, QueryResult, Statement, TransactionTrait,
};

use crate::{
    db::{ChannelRole, Database},
    entities::User,
    tenant_admission::{AuthorizedRpcRequest, bind_rpc_tenant},
};

pub const CHANNEL_READ_SCOPE: &str = "collaboration:channel:read";
pub const CHANNEL_WRITE_SCOPE: &str = "collaboration:channel:write";

#[derive(Clone)]
pub struct AuthorizedChannel {
    pub tenant: TenantContext,
    pub principal: AuthenticatedPrincipal,
    pub community_membership: CommunityMembership,
    pub channel_membership: ChannelMembership,
    pub channel_id: AggregateId,
    pub signing_public_key: Option<[u8; 32]>,
}

impl AuthorizedChannel {
    pub fn authorization_request<'a>(
        &'a self,
        action: AuthorizationAction,
        required_scope: &'a AuthorizationScope,
        now_millis: u64,
    ) -> AuthorizationRequest<'a> {
        AuthorizationRequest {
            tenant: &self.tenant,
            principal: &self.principal,
            required_scope,
            action,
            resource: AuthorizationResource {
                community_id: self.tenant.community_id(),
                kind: AuthorizationResourceKind::Channel,
                resource_id: self.channel_id,
                owner_principal_id: None,
                channel_id: Some(self.channel_id),
            },
            current_membership_version: self.community_membership.version,
            community_membership: Some(self.community_membership),
            current_channel_membership_version: Some(self.channel_membership.version),
            channel_membership: Some(self.channel_membership),
            delegation: None,
            now_millis,
        }
    }

    pub fn message_authorization_request<'a>(
        &'a self,
        message_id: AggregateId,
        author_principal_id: PrincipalId,
        action: AuthorizationAction,
        required_scope: &'a AuthorizationScope,
        now_millis: u64,
    ) -> AuthorizationRequest<'a> {
        AuthorizationRequest {
            tenant: &self.tenant,
            principal: &self.principal,
            required_scope,
            action,
            resource: AuthorizationResource {
                community_id: self.tenant.community_id(),
                kind: AuthorizationResourceKind::Conversation,
                resource_id: message_id,
                owner_principal_id: Some(author_principal_id),
                channel_id: Some(self.channel_id),
            },
            current_membership_version: self.community_membership.version,
            community_membership: Some(self.community_membership),
            current_channel_membership_version: Some(self.channel_membership.version),
            channel_membership: Some(self.channel_membership),
            delegation: None,
            now_millis,
        }
    }

    pub fn authorize(
        &self,
        action: AuthorizationAction,
        now_millis: u64,
    ) -> Result<AuthorizedRpcRequest> {
        let scope = AuthorizationScope::new(match action {
            AuthorizationAction::Read => CHANNEL_READ_SCOPE,
            AuthorizationAction::Write
            | AuthorizationAction::Manage
            | AuthorizationAction::Delete => CHANNEL_WRITE_SCOPE,
        })?;
        AuthorizedRpcRequest::authorize(&self.authorization_request(action, &scope, now_millis))
            .map_err(|_| anyhow!("channel access denied"))
    }
}

#[derive(Clone)]
struct LegacyChannel {
    id: u64,
    root_id: u64,
    name: String,
    visibility: String,
}

#[derive(Clone, Copy)]
struct LegacyMembership {
    user_id: u64,
    role: ChannelRole,
}

pub async fn bootstrap_canonical_channels(database: &Database) -> Result<()> {
    if database.pool.get_database_backend() != DatabaseBackend::Postgres {
        return Ok(());
    }
    let channels = load_channels(&database.pool).await?;
    let memberships = load_memberships(&database.pool).await?;
    let channels_by_root = channels.into_iter().fold(
        BTreeMap::<u64, Vec<LegacyChannel>>::new(),
        |mut grouped, channel| {
            grouped.entry(channel.root_id).or_default().push(channel);
            grouped
        },
    );
    let memberships_by_root = memberships.into_iter().fold(
        BTreeMap::<u64, Vec<LegacyMembership>>::new(),
        |mut grouped, (root_id, membership)| {
            grouped.entry(root_id).or_default().push(membership);
            grouped
        },
    );

    for (root_id, channels) in channels_by_root {
        let Some(memberships) = memberships_by_root.get(&root_id) else {
            continue;
        };
        if memberships.is_empty() {
            continue;
        }
        bootstrap_community(&database.pool, root_id, &channels, memberships).await?;
    }
    Ok(())
}

pub async fn admit_channel(
    connection: &DatabaseConnection,
    user: &User,
    claimed_community_id: CommunityId,
    claimed_channel_id: AggregateId,
) -> Result<AuthorizedChannel> {
    let route_row = connection
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"
SELECT
    channel_binding.community_id,
    channel_binding.channel_id,
    channel_binding.legacy_channel_id,
    community_binding.legacy_root_channel_id,
    principal_binding.principal_id,
    principal_binding.signing_public_key
FROM public.collaboration_zed_channel_bindings AS channel_binding
JOIN public.collaboration_zed_community_bindings AS community_binding
  ON community_binding.community_id = channel_binding.community_id
JOIN public.collaboration_zed_principal_bindings AS principal_binding
  ON principal_binding.community_id = channel_binding.community_id
 AND principal_binding.legacy_user_id = $3
WHERE channel_binding.community_id = $1
  AND channel_binding.channel_id = $2
"#,
            [
                claimed_community_id.as_uuid().into(),
                claimed_channel_id.as_uuid().into(),
                user.id.0.into(),
            ],
        ))
        .await
        .context("failed to resolve trusted channel route")?
        .ok_or_else(|| anyhow!("channel access denied"))?;

    let community_id = CommunityId::from_uuid(route_row.try_get("", "community_id")?);
    let channel_id = AggregateId::from_uuid(route_row.try_get("", "channel_id")?);
    if community_id != claimed_community_id || channel_id != claimed_channel_id {
        return Err(anyhow!("channel access denied"));
    }
    let legacy_root_channel_id =
        u64::try_from(route_row.try_get::<i64>("", "legacy_root_channel_id")?)?;
    let legacy_channel_id = u64::try_from(route_row.try_get::<i64>("", "legacy_channel_id")?)?;
    if community_id_for_legacy_root_channel(legacy_root_channel_id) != community_id
        || channel_id_for_legacy_channel(legacy_channel_id) != channel_id
    {
        return Err(anyhow!("channel access denied"));
    }

    let principal_id = PrincipalId::from_uuid(route_row.try_get("", "principal_id")?);
    let signing_public_key = route_row
        .try_get::<Option<Vec<u8>>>("", "signing_public_key")?
        .map(|bytes| {
            bytes
                .try_into()
                .map_err(|_| anyhow!("invalid signing identity"))
        })
        .transpose()?;
    let expected_principal = principal_id_for_legacy_user(community_id, user.id.0 as u64);
    if principal_id != expected_principal {
        return Err(anyhow!("channel access denied"));
    }
    let route = TrustedTenantRoute::from_deployment(
        community_id,
        format!("zed-channel:{legacy_root_channel_id}"),
    )?;
    let claim =
        UntrustedTenantClaim::new(claimed_community_id, UntrustedTenantClaimSource::BodyField);
    let tenant =
        bind_rpc_tenant(Some(route), &[claim]).map_err(|_| anyhow!("channel access denied"))?;
    let transaction = connection.begin().await?;
    transaction
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT set_config('app.community_id', $1, true)",
            [tenant.community_id().to_string().into()],
        ))
        .await?;
    let membership_row = transaction
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"
SELECT
    community_membership.role AS community_role,
    community_membership.status AS community_status,
    community_membership.membership_version::text AS community_version,
    channel_membership.role AS channel_role,
    channel_membership.status AS channel_status,
    channel_membership.membership_version::text AS channel_version
FROM public.collaboration_community_memberships AS community_membership
JOIN public.collaboration_channel_memberships AS channel_membership
  ON channel_membership.community_id = community_membership.community_id
 AND channel_membership.principal_id = community_membership.principal_id
WHERE community_membership.community_id = $1
  AND community_membership.principal_id = $2
  AND channel_membership.channel_id = $3
"#,
            [
                community_id.as_uuid().into(),
                principal_id.as_uuid().into(),
                channel_id.as_uuid().into(),
            ],
        ))
        .await?
        .ok_or_else(|| anyhow!("channel access denied"))?;
    transaction.commit().await?;
    let scopes = PrincipalScopes::new([
        AuthorizationScope::new(CHANNEL_READ_SCOPE)?,
        AuthorizationScope::new(CHANNEL_WRITE_SCOPE)?,
    ])?;
    let service_account_id = u64::try_from(user.id.0)?;
    let principal = AuthenticatedPrincipal::zed_account(
        principal_id,
        community_id,
        ServiceAccountId::new(service_account_id),
        scopes,
    );
    let community_membership = membership_from_row(
        &membership_row,
        community_id,
        channel_id,
        principal_id,
        false,
    )?;
    let channel_membership = membership_from_row(
        &membership_row,
        community_id,
        channel_id,
        principal_id,
        true,
    )?;
    Ok(AuthorizedChannel {
        tenant,
        principal,
        community_membership: CommunityMembership {
            community_id,
            principal_id,
            role: community_membership.0,
            status: community_membership.1,
            version: community_membership.2,
        },
        channel_membership: ChannelMembership {
            community_id,
            channel_id,
            principal_id,
            role: channel_membership.0,
            status: channel_membership.1,
            version: channel_membership.2,
        },
        channel_id,
        signing_public_key,
    })
}

pub async fn update_principal_presentation(
    connection: &DatabaseConnection,
    authorized: &AuthorizedChannel,
    user: &User,
    signing_public_key: Option<&[u8]>,
) -> Result<()> {
    if signing_public_key.is_some_and(|key| key.len() != 32) {
        return Err(anyhow!("invalid signing public key"));
    }
    let transaction = connection.begin().await?;
    transaction
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT set_config('app.community_id', $1, true)",
            [authorized.tenant.community_id().to_string().into()],
        ))
        .await?;
    let result = transaction
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"
UPDATE public.collaboration_zed_principal_bindings
SET display_name = $3,
    avatar_url = $4,
    signing_public_key = CASE
        WHEN signing_public_key IS NULL THEN $5
        WHEN $5 IS NULL OR signing_public_key = $5 THEN signing_public_key
        ELSE signing_public_key
    END,
    updated_at = clock_timestamp()
WHERE community_id = $1 AND legacy_user_id = $2
  AND (signing_public_key IS NULL OR $5 IS NULL OR signing_public_key = $5)
"#,
            [
                authorized.tenant.community_id().as_uuid().into(),
                user.id.0.into(),
                user.username.clone().into(),
                user.avatar_url.clone().into(),
                signing_public_key.map(ToOwned::to_owned).into(),
            ],
        ))
        .await?;
    if result.rows_affected() != 1 {
        transaction.rollback().await?;
        return Err(anyhow!(
            "signing identity does not match the authenticated account"
        ));
    }
    transaction.commit().await?;
    Ok(())
}

async fn load_channels(connection: &DatabaseConnection) -> Result<Vec<LegacyChannel>> {
    let rows = connection
        .query_all(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT id::bigint AS id, name, visibility, parent_path FROM public.channels ORDER BY id",
        ))
        .await?;
    rows.into_iter().map(channel_from_row).collect()
}

fn channel_from_row(row: QueryResult) -> Result<LegacyChannel> {
    let id = u64::try_from(row.try_get::<i64>("", "id")?)?;
    let parent_path: String = row.try_get("", "parent_path")?;
    let root_id = parent_path
        .split('/')
        .find(|part| !part.is_empty())
        .map(str::parse::<u64>)
        .transpose()?
        .unwrap_or(id);
    Ok(LegacyChannel {
        id,
        root_id,
        name: row.try_get("", "name")?,
        visibility: row.try_get("", "visibility")?,
    })
}

async fn load_memberships(connection: &DatabaseConnection) -> Result<Vec<(u64, LegacyMembership)>> {
    let rows = connection
        .query_all(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT channel_id::bigint AS channel_id, user_id::bigint AS user_id, role FROM public.channel_members WHERE accepted = true ORDER BY channel_id, user_id",
        ))
        .await?;
    rows.into_iter()
        .map(|row| {
            let root_id = u64::try_from(row.try_get::<i64>("", "channel_id")?)?;
            let user_id = u64::try_from(row.try_get::<i64>("", "user_id")?)?;
            let role = match row.try_get::<String>("", "role")?.as_str() {
                "admin" => ChannelRole::Admin,
                "member" => ChannelRole::Member,
                "talker" => ChannelRole::Talker,
                "guest" => ChannelRole::Guest,
                "banned" => ChannelRole::Banned,
                _ => return Err(anyhow!("invalid legacy channel role")),
            };
            Ok((root_id, LegacyMembership { user_id, role }))
        })
        .collect()
}

async fn bootstrap_community(
    connection: &DatabaseConnection,
    root_id: u64,
    channels: &[LegacyChannel],
    memberships: &[LegacyMembership],
) -> Result<()> {
    let community_id = community_id_for_legacy_root_channel(root_id);
    let transaction = connection.begin().await?;
    transaction
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT set_config('app.community_id', $1, true)",
            [community_id.to_string().into()],
        ))
        .await?;
    transaction
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"
INSERT INTO public.collaboration_communities (
    community_id, host, lifecycle_state, aggregate_version, source_system,
    source_record_id, source_version, source_observed_at, created_at, updated_at
) VALUES ($1, $2, 'active', 1, 'zed', $3, '1', clock_timestamp(), clock_timestamp(), clock_timestamp())
ON CONFLICT (community_id) DO UPDATE SET updated_at = EXCLUDED.updated_at
"#,
            [
                community_id.as_uuid().into(),
                format!("zed-channel-{root_id}").into(),
                format!("channel-root:{root_id}").into(),
            ],
        ))
        .await?;
    transaction
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"
INSERT INTO public.collaboration_zed_community_bindings (legacy_root_channel_id, community_id)
VALUES ($1, $2)
ON CONFLICT (legacy_root_channel_id) DO UPDATE SET community_id = EXCLUDED.community_id
"#,
            [
                i64::try_from(root_id)?.into(),
                community_id.as_uuid().into(),
            ],
        ))
        .await?;

    let mut active_principals = BTreeSet::new();
    let current_legacy_users = memberships
        .iter()
        .map(|membership| membership.user_id)
        .collect::<BTreeSet<_>>();
    let existing_bindings = transaction
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT legacy_user_id::bigint AS legacy_user_id, principal_id FROM public.collaboration_zed_principal_bindings WHERE community_id = $1",
            [community_id.as_uuid().into()],
        ))
        .await?;
    for binding in existing_bindings {
        let legacy_user_id = u64::try_from(binding.try_get::<i64>("", "legacy_user_id")?)?;
        if current_legacy_users.contains(&legacy_user_id) {
            continue;
        }
        let principal_id: uuid::Uuid = binding.try_get("", "principal_id")?;
        transaction
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                r#"
UPDATE public.collaboration_community_memberships
SET status = 'revoked', membership_version = membership_version + 1,
    updated_at = clock_timestamp()
WHERE community_id = $1 AND principal_id = $2 AND status <> 'revoked'
"#,
                [community_id.as_uuid().into(), principal_id.into()],
            ))
            .await?;
        transaction
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                r#"
UPDATE public.collaboration_channel_memberships
SET status = 'revoked', membership_version = membership_version + 1,
    updated_at = clock_timestamp()
WHERE community_id = $1 AND principal_id = $2 AND status <> 'revoked'
"#,
                [community_id.as_uuid().into(), principal_id.into()],
            ))
            .await?;
    }
    for membership in memberships {
        let principal_id = principal_id_for_legacy_user(community_id, membership.user_id);
        let (role, status) = canonical_membership(membership.role);
        transaction
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                r#"
INSERT INTO public.collaboration_community_memberships (
    community_id, principal_id, role, status, membership_version, joined_at, updated_at,
    source_system, source_record_id, source_version, source_observed_at
) VALUES ($1, $2, $3, $4, 1, clock_timestamp(), clock_timestamp(), 'zed', $5, '1', clock_timestamp())
ON CONFLICT (community_id, principal_id) DO UPDATE SET
    role = EXCLUDED.role,
    status = EXCLUDED.status,
    membership_version = CASE
        WHEN collaboration_community_memberships.role IS DISTINCT FROM EXCLUDED.role
          OR collaboration_community_memberships.status IS DISTINCT FROM EXCLUDED.status
        THEN collaboration_community_memberships.membership_version + 1
        ELSE collaboration_community_memberships.membership_version
    END,
    updated_at = EXCLUDED.updated_at
"#,
                [
                    community_id.as_uuid().into(),
                    principal_id.as_uuid().into(),
                    role.into(),
                    status.into(),
                    format!("channel-member:{}:{}", root_id, membership.user_id).into(),
                ],
            ))
            .await?;
        transaction
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                r#"
INSERT INTO public.collaboration_zed_principal_bindings (
    legacy_user_id, community_id, principal_id, display_name
) VALUES ($1, $2, $3, $4)
ON CONFLICT (legacy_user_id, community_id) DO NOTHING
"#,
                [
                    i64::try_from(membership.user_id)?.into(),
                    community_id.as_uuid().into(),
                    principal_id.as_uuid().into(),
                    format!("user-{}", membership.user_id).into(),
                ],
            ))
            .await?;
        if status == "active" {
            active_principals.insert(principal_id);
        }
    }
    let creator = active_principals
        .first()
        .copied()
        .ok_or_else(|| anyhow!("channel community has no active member"))?;

    for channel in channels {
        let channel_id = channel_id_for_legacy_channel(channel.id);
        transaction
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                r#"
INSERT INTO public.collaboration_channels (
    community_id, channel_id, name, channel_type, visibility, lifecycle_state,
    creator_principal_id, channel_version, source_system, source_record_id,
    source_version, source_observed_at, created_at, updated_at
) VALUES ($1, $2, $3, 'stream', $4, 'active', $5, 1, 'zed', $6, '1', clock_timestamp(), clock_timestamp(), clock_timestamp())
ON CONFLICT (community_id, channel_id) DO UPDATE SET
    name = EXCLUDED.name, visibility = EXCLUDED.visibility, updated_at = EXCLUDED.updated_at
"#,
                [
                    community_id.as_uuid().into(),
                    channel_id.as_uuid().into(),
                    channel.name.clone().into(),
                    (if channel.visibility == "public" { "open" } else { "private" }).into(),
                    creator.as_uuid().into(),
                    format!("channel:{}", channel.id).into(),
                ],
            ))
            .await?;
        transaction
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                r#"
INSERT INTO public.collaboration_zed_channel_bindings (legacy_channel_id, community_id, channel_id)
VALUES ($1, $2, $3)
ON CONFLICT (legacy_channel_id) DO UPDATE SET
    community_id = EXCLUDED.community_id, channel_id = EXCLUDED.channel_id
"#,
                [
                    i64::try_from(channel.id)?.into(),
                    community_id.as_uuid().into(),
                    channel_id.as_uuid().into(),
                ],
            ))
            .await?;
        for membership in memberships {
            let principal_id = principal_id_for_legacy_user(community_id, membership.user_id);
            let (role, status) = canonical_membership(membership.role);
            transaction
                .execute(Statement::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    r#"
INSERT INTO public.collaboration_channel_memberships (
    community_id, channel_id, principal_id, role, status, membership_version,
    joined_at, updated_at, source_system, source_record_id, source_version, source_observed_at
) VALUES ($1, $2, $3, $4, $5, 1, clock_timestamp(), clock_timestamp(), 'zed', $6, '1', clock_timestamp())
ON CONFLICT (community_id, channel_id, principal_id) DO UPDATE SET
    role = EXCLUDED.role,
    status = EXCLUDED.status,
    membership_version = CASE
        WHEN collaboration_channel_memberships.role IS DISTINCT FROM EXCLUDED.role
          OR collaboration_channel_memberships.status IS DISTINCT FROM EXCLUDED.status
        THEN collaboration_channel_memberships.membership_version + 1
        ELSE collaboration_channel_memberships.membership_version
    END,
    updated_at = EXCLUDED.updated_at
"#,
                    [
                        community_id.as_uuid().into(),
                        channel_id.as_uuid().into(),
                        principal_id.as_uuid().into(),
                        role.into(),
                        status.into(),
                        format!("channel-member:{}:{}", channel.id, membership.user_id).into(),
                    ],
                ))
                .await?;
        }
    }
    transaction.commit().await?;
    Ok(())
}

fn canonical_membership(role: ChannelRole) -> (&'static str, &'static str) {
    match role {
        ChannelRole::Admin => ("admin", "active"),
        ChannelRole::Member | ChannelRole::Talker => ("member", "active"),
        ChannelRole::Guest => ("guest", "active"),
        ChannelRole::Banned => ("guest", "revoked"),
    }
}

fn membership_from_row(
    row: &QueryResult,
    _community_id: CommunityId,
    _channel_id: AggregateId,
    _principal_id: PrincipalId,
    channel: bool,
) -> Result<(MembershipRole, MembershipStatus, AggregateVersion)> {
    let prefix = if channel { "channel" } else { "community" };
    let role = match row
        .try_get::<String>("", &format!("{prefix}_role"))?
        .as_str()
    {
        "owner" => MembershipRole::Owner,
        "admin" => MembershipRole::Admin,
        "member" => MembershipRole::Member,
        "guest" => MembershipRole::Guest,
        "bot" => MembershipRole::Bot,
        _ => return Err(anyhow!("invalid membership role")),
    };
    let status = match row
        .try_get::<String>("", &format!("{prefix}_status"))?
        .as_str()
    {
        "active" => MembershipStatus::Active,
        "revoked" => MembershipStatus::Revoked,
        "archived" => MembershipStatus::Archived,
        _ => return Err(anyhow!("invalid membership status")),
    };
    let version = row
        .try_get::<String>("", &format!("{prefix}_version"))?
        .parse::<u64>()?;
    let version =
        AggregateVersion::new(version).ok_or_else(|| anyhow!("invalid membership version"))?;
    Ok((role, status, version))
}
