use collaboration_domain::{
    AggregateId, AuthorizationAction, AuthorizationScope, NostrEventId, PrincipalId,
};
use sea_orm::{
    ConnectionTrait, DatabaseBackend, DatabaseConnection, DatabaseTransaction, QueryResult,
    Statement, TransactionTrait,
};

use crate::{
    messages::{
        channel_admission::{AuthorizedChannel, CHANNEL_READ_SCOPE},
        window_repository::{
            ChannelWindowCursor, ChannelWindowQuery, MessageWindowRepository, ThreadWindowQuery,
            WindowAccess, WindowSnapshot,
        },
    },
    pubsub::postgres::PostgresFanoutReplayStore,
};

const MAX_PAGE_SIZE: usize = 200;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HydratedReaction {
    pub value: String,
    pub actor_principal_id: PrincipalId,
    pub source_event_id: NostrEventId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HydratedMessage {
    pub community_id: collaboration_domain::CommunityId,
    pub channel_id: AggregateId,
    pub message_id: AggregateId,
    pub source_event_id: NostrEventId,
    pub current_event_id: NostrEventId,
    pub author_principal_id: PrincipalId,
    pub author_display_name: String,
    pub author_avatar_url: String,
    pub body: String,
    pub created_at: u64,
    pub version: u64,
    pub edited: bool,
    pub deleted: bool,
    pub reply_to_event_id: Option<NostrEventId>,
    pub reactions: Vec<HydratedReaction>,
    pub accepted_operation_id: Option<collaboration_domain::OperationId>,
    pub outbox_sequence: u64,
    pub reaction_version: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HydratedPage {
    pub messages: Vec<HydratedMessage>,
    pub next_cursor: Option<ChannelWindowCursor>,
    pub snapshot: WindowSnapshot,
    pub has_more: bool,
    pub authoritative_outbox_cursor: u64,
}

pub struct CanonicalMessageService {
    connection: DatabaseConnection,
    windows: MessageWindowRepository,
    replay: PostgresFanoutReplayStore,
}

impl CanonicalMessageService {
    pub fn new(connection: DatabaseConnection) -> anyhow::Result<Self> {
        if connection.get_database_backend() != DatabaseBackend::Postgres {
            return Err(anyhow::anyhow!(
                "canonical message service requires PostgreSQL"
            ));
        }
        let windows_connection =
            DatabaseConnection::from(connection.get_postgres_connection_pool().clone());
        let replay_connection =
            DatabaseConnection::from(connection.get_postgres_connection_pool().clone());
        Ok(Self {
            windows: MessageWindowRepository::new(windows_connection)?,
            replay: PostgresFanoutReplayStore::new(replay_connection)?,
            connection,
        })
    }

    pub const fn replay(&self) -> &PostgresFanoutReplayStore {
        &self.replay
    }

    pub async fn authorization_is_current(
        &self,
        authorization: &AuthorizedChannel,
    ) -> anyhow::Result<bool> {
        let transaction = self.connection.begin().await?;
        set_tenant(&transaction, authorization).await?;
        let row = transaction
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                r#"
SELECT
    community_membership.status AS community_status,
    community_membership.membership_version::text AS community_version,
    channel_membership.status AS channel_status,
    channel_membership.membership_version::text AS channel_version
FROM public.collaboration_community_memberships AS community_membership
JOIN public.collaboration_channel_memberships AS channel_membership
  ON channel_membership.community_id = community_membership.community_id
 AND channel_membership.principal_id = community_membership.principal_id
JOIN public.collaboration_zed_community_bindings AS community_binding
  ON community_binding.community_id = community_membership.community_id
JOIN public.collaboration_zed_principal_bindings AS principal_binding
  ON principal_binding.community_id = community_membership.community_id
 AND principal_binding.principal_id = community_membership.principal_id
JOIN public.channel_members AS legacy_membership
  ON legacy_membership.channel_id = community_binding.legacy_root_channel_id
 AND legacy_membership.user_id = principal_binding.legacy_user_id
 AND legacy_membership.accepted = true
 AND legacy_membership.role <> 'banned'
WHERE community_membership.community_id = $1
  AND community_membership.principal_id = $2
  AND channel_membership.channel_id = $3
"#,
                [
                    authorization.tenant.community_id().as_uuid().into(),
                    authorization.principal.principal_id().as_uuid().into(),
                    authorization.channel_id.as_uuid().into(),
                ],
            ))
            .await?;
        transaction.commit().await?;
        let Some(row) = row else {
            return Ok(false);
        };
        Ok(row.try_get::<String>("", "community_status")? == "active"
            && row.try_get::<String>("", "channel_status")? == "active"
            && row.try_get::<String>("", "community_version")?
                == authorization.community_membership.version.to_string()
            && row.try_get::<String>("", "channel_version")?
                == authorization.channel_membership.version.to_string())
    }

    pub async fn channel_page(
        &self,
        authorization: &AuthorizedChannel,
        requested_limit: usize,
        cursor: Option<(ChannelWindowCursor, WindowSnapshot)>,
    ) -> anyhow::Result<HydratedPage> {
        let scope = AuthorizationScope::new(CHANNEL_READ_SCOPE)?;
        let request =
            authorization.authorization_request(AuthorizationAction::Read, &scope, now_millis());
        let query = match cursor {
            Some((cursor, snapshot)) => ChannelWindowQuery::continuation(
                authorization.channel_id,
                requested_limit.min(MAX_PAGE_SIZE),
                cursor,
                snapshot,
            )?,
            None => ChannelWindowQuery::head(
                authorization.channel_id,
                requested_limit.min(MAX_PAGE_SIZE),
            )?,
        };
        let page = self
            .windows
            .channel_page(
                WindowAccess {
                    authorization: &request,
                },
                &query,
            )
            .await?;
        let messages = self
            .hydrate_rows(
                authorization,
                page.rows
                    .iter()
                    .map(|row| row.message_id)
                    .collect::<Vec<_>>(),
            )
            .await?;
        Ok(HydratedPage {
            messages,
            next_cursor: page.next_cursor,
            snapshot: page.snapshot,
            has_more: page.has_more,
            authoritative_outbox_cursor: self.latest_outbox_sequence(authorization).await?,
        })
    }

    pub async fn thread_page(
        &self,
        authorization: &AuthorizedChannel,
        root_event_id: NostrEventId,
        requested_limit: usize,
        cursor: Option<(collaboration_domain::ThreadCursor, WindowSnapshot)>,
    ) -> anyhow::Result<HydratedPage> {
        let scope = AuthorizationScope::new(CHANNEL_READ_SCOPE)?;
        let request =
            authorization.authorization_request(AuthorizationAction::Read, &scope, now_millis());
        let query = match cursor {
            Some((cursor, snapshot)) => ThreadWindowQuery::continuation(
                authorization.channel_id,
                root_event_id,
                requested_limit.min(MAX_PAGE_SIZE),
                None,
                cursor,
                snapshot,
            )?,
            None => ThreadWindowQuery::head(
                authorization.channel_id,
                root_event_id,
                requested_limit.min(MAX_PAGE_SIZE),
                None,
            )?,
        };
        let page = self
            .windows
            .thread_page(
                WindowAccess {
                    authorization: &request,
                },
                &query,
            )
            .await?;
        let messages = self
            .hydrate_rows(
                authorization,
                page.replies
                    .iter()
                    .map(|row| row.message.message_id)
                    .collect::<Vec<_>>(),
            )
            .await?;
        Ok(HydratedPage {
            messages,
            next_cursor: page.next_cursor.map(|cursor| ChannelWindowCursor {
                message_created_at: cursor.created_at,
                source_event_id: cursor.event_id,
            }),
            snapshot: page.snapshot,
            has_more: page.has_more,
            authoritative_outbox_cursor: self.latest_outbox_sequence(authorization).await?,
        })
    }

    pub async fn message(
        &self,
        authorization: &AuthorizedChannel,
        message_id: AggregateId,
    ) -> anyhow::Result<Option<HydratedMessage>> {
        Ok(self
            .hydrate_rows(authorization, vec![message_id])
            .await?
            .into_iter()
            .next())
    }

    async fn hydrate_rows(
        &self,
        authorization: &AuthorizedChannel,
        message_ids: Vec<AggregateId>,
    ) -> anyhow::Result<Vec<HydratedMessage>> {
        if message_ids.is_empty() {
            return Ok(Vec::new());
        }
        let transaction = self.connection.begin().await?;
        set_tenant(&transaction, authorization).await?;
        let mut messages = Vec::with_capacity(message_ids.len());
        for message_id in message_ids {
            if let Some(message) = hydrate_message(&transaction, authorization, message_id).await? {
                messages.push(message);
            }
        }
        transaction.commit().await?;
        Ok(messages)
    }

    async fn latest_outbox_sequence(
        &self,
        authorization: &AuthorizedChannel,
    ) -> anyhow::Result<u64> {
        let transaction = self.connection.begin().await?;
        set_tenant(&transaction, authorization).await?;
        let row = transaction
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT COALESCE(max(outbox_sequence), 0)::bigint AS sequence FROM public.collaboration_outbox WHERE community_id = $1 AND topic = $2",
                [
                    authorization.tenant.community_id().as_uuid().into(),
                    super::channel_mutation::channel_topic(authorization.channel_id).into(),
                ],
            ))
            .await?
            .ok_or_else(|| anyhow::anyhow!("missing outbox cursor"))?;
        let sequence = u64::try_from(row.try_get::<i64>("", "sequence")?)?;
        transaction.commit().await?;
        Ok(sequence)
    }
}

async fn hydrate_message(
    transaction: &DatabaseTransaction,
    authorization: &AuthorizedChannel,
    message_id: AggregateId,
) -> anyhow::Result<Option<HydratedMessage>> {
    let row = transaction
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"
SELECT
    message.message_id,
    message.source_event_id,
    message.current_event_id,
    message.author_principal_id,
    message.message_created_at::text AS created_at,
    message.lifecycle_state,
    message.message_version::text AS message_version,
    current_event.content,
    source_event.tags,
    principal.display_name,
    principal.avatar_url,
    latest_outbox.operation_id,
    COALESCE(latest_outbox.outbox_sequence, 0)::bigint AS outbox_sequence,
    (
        SELECT (COUNT(*) + 1)::bigint
        FROM public.collaboration_message_auxiliary_events AS reaction_event
        WHERE reaction_event.community_id = message.community_id
          AND reaction_event.channel_id = message.channel_id
          AND reaction_event.target_message_event_id = message.source_event_id
          AND reaction_event.auxiliary_kind IN ('reaction_add', 'reaction_remove')
    ) AS reaction_version
FROM public.collaboration_messages AS message
JOIN public.collaboration_events AS current_event
  ON current_event.community_id = message.community_id
 AND current_event.event_id = message.current_event_id
JOIN public.collaboration_events AS source_event
  ON source_event.community_id = message.community_id
 AND source_event.event_id = message.source_event_id
JOIN public.collaboration_zed_principal_bindings AS principal
  ON principal.community_id = message.community_id
 AND principal.principal_id = message.author_principal_id
LEFT JOIN LATERAL (
    SELECT outbox.operation_id, outbox.outbox_sequence
    FROM public.collaboration_outbox AS outbox
    WHERE outbox.community_id = message.community_id
      AND outbox.topic = $4
      AND (
          outbox.source_record_id IN (
              encode(message.source_event_id, 'hex'),
              encode(message.current_event_id, 'hex')
          )
          OR outbox.source_record_id IN (
              SELECT encode(auxiliary.auxiliary_event_id, 'hex')
              FROM public.collaboration_message_auxiliary_events AS auxiliary
              WHERE auxiliary.community_id = message.community_id
                AND auxiliary.channel_id = message.channel_id
                AND auxiliary.target_message_event_id = message.source_event_id
          )
      )
    ORDER BY outbox.outbox_sequence DESC
    LIMIT 1
) AS latest_outbox ON true
WHERE message.community_id = $1 AND message.channel_id = $2 AND message.message_id = $3
"#,
            [
                authorization.tenant.community_id().as_uuid().into(),
                authorization.channel_id.as_uuid().into(),
                message_id.as_uuid().into(),
                super::channel_mutation::channel_topic(authorization.channel_id).into(),
            ],
        ))
        .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let source_event_id = event_id(&row, "source_event_id")?;
    let reaction_rows = transaction
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"
SELECT added.emoji, added.actor_principal_id, added.auxiliary_event_id
FROM public.collaboration_message_auxiliary_events AS added
WHERE added.community_id = $1 AND added.channel_id = $2
  AND added.target_message_event_id = $3
  AND added.auxiliary_kind = 'reaction_add'
  AND NOT EXISTS (
      SELECT 1
      FROM public.collaboration_message_auxiliary_events AS removed
      WHERE removed.community_id = added.community_id
        AND removed.channel_id = added.channel_id
        AND removed.auxiliary_kind = 'reaction_remove'
        AND removed.related_event_id = added.auxiliary_event_id
  )
ORDER BY added.emoji, added.actor_principal_id, added.auxiliary_event_id
"#,
            [
                authorization.tenant.community_id().as_uuid().into(),
                authorization.channel_id.as_uuid().into(),
                source_event_id.as_bytes().to_vec().into(),
            ],
        ))
        .await?;
    let reactions = reaction_rows
        .into_iter()
        .map(|row| {
            Ok(HydratedReaction {
                value: row.try_get("", "emoji")?,
                actor_principal_id: PrincipalId::from_uuid(row.try_get("", "actor_principal_id")?),
                source_event_id: event_id(&row, "auxiliary_event_id")?,
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let tags: serde_json::Value = row.try_get("", "tags")?;
    let reply_to_event_id = tags
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_array)
        .find_map(|tag| {
            (tag.first().and_then(serde_json::Value::as_str) == Some("e")
                && tag.get(3).and_then(serde_json::Value::as_str) == Some("reply"))
            .then(|| tag.get(1).and_then(serde_json::Value::as_str))
            .flatten()
        })
        .and_then(|value| hex::decode(value).ok())
        .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
        .map(NostrEventId::from_bytes);
    let lifecycle: String = row.try_get("", "lifecycle_state")?;
    Ok(Some(HydratedMessage {
        community_id: authorization.tenant.community_id(),
        channel_id: authorization.channel_id,
        message_id,
        source_event_id,
        current_event_id: event_id(&row, "current_event_id")?,
        author_principal_id: PrincipalId::from_uuid(row.try_get("", "author_principal_id")?),
        author_display_name: row.try_get("", "display_name")?,
        author_avatar_url: row.try_get("", "avatar_url")?,
        body: if lifecycle == "deleted" {
            String::new()
        } else {
            row.try_get("", "content")?
        },
        created_at: row.try_get::<String>("", "created_at")?.parse()?,
        version: row.try_get::<String>("", "message_version")?.parse()?,
        edited: lifecycle == "edited",
        deleted: lifecycle == "deleted",
        reply_to_event_id,
        reactions,
        accepted_operation_id: row
            .try_get::<Option<uuid::Uuid>>("", "operation_id")?
            .map(collaboration_domain::OperationId::from_uuid),
        outbox_sequence: u64::try_from(row.try_get::<i64>("", "outbox_sequence")?)?,
        reaction_version: u64::try_from(row.try_get::<i64>("", "reaction_version")?)?,
    }))
}

async fn set_tenant(
    transaction: &DatabaseTransaction,
    authorization: &AuthorizedChannel,
) -> anyhow::Result<()> {
    transaction
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT set_config('app.community_id', $1, true)",
            [authorization.tenant.community_id().to_string().into()],
        ))
        .await?;
    Ok(())
}

fn event_id(row: &QueryResult, column: &str) -> anyhow::Result<NostrEventId> {
    let bytes: Vec<u8> = row.try_get("", column)?;
    Ok(NostrEventId::from_bytes(bytes.try_into().map_err(
        |_| anyhow::anyhow!("invalid canonical event id"),
    )?))
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis() as u64)
}
