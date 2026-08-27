use std::sync::Arc;

use async_trait::async_trait;
use collaboration_domain::{
    AggregateId, CommunityId, OperationId, TenantContext, TrustedTenantRoute,
};
use dashmap::DashMap;
use futures::{
    FutureExt as _,
    future::{AbortHandle, Abortable},
};
use rpc::{ConnectionId, Peer, proto};

use crate::{
    executor::Executor,
    messages::{
        channel_admission::AuthorizedChannel,
        channel_mutation::{MessageOutboxPayload, channel_topic},
        channel_service::{CanonicalMessageService, HydratedMessage, HydratedPage},
    },
    pubsub::{
        envelope::FanoutEnvelope,
        redis::RedisFanoutTransport,
        subscription_bus::{
            FanoutTransport, SubscriptionBus, SubscriptionBusError, SubscriptionEvent,
        },
    },
};

const DEDUPLICATION_CAPACITY: usize = 65_536;
const SUBSCRIPTION_QUEUE_CAPACITY: usize = 512;
const REPLAY_BATCH: usize = 512;

pub struct CanonicalMessageRuntime {
    database: Arc<crate::db::Database>,
    service: Arc<CanonicalMessageService>,
    buses: Arc<DashMap<CommunityId, SubscriptionBus>>,
    subscriptions: DashMap<(ConnectionId, AggregateId), AbortHandle>,
    redis: Option<RedisFanoutTransport>,
    executor: Executor,
}

impl CanonicalMessageRuntime {
    pub fn new(
        database: Arc<crate::db::Database>,
        service: CanonicalMessageService,
        redis_url: Option<&str>,
        executor: Executor,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            database,
            service: Arc::new(service),
            buses: Arc::new(DashMap::new()),
            subscriptions: DashMap::new(),
            redis: redis_url.map(RedisFanoutTransport::new).transpose()?,
            executor,
        })
    }

    pub fn service(&self) -> &CanonicalMessageService {
        &self.service
    }

    pub fn start_redis_receiver(&self) {
        let Some(redis) = self.redis.clone() else {
            return;
        };
        let buses = Arc::clone(&self.buses);
        let executor = self.executor.clone();
        self.executor.spawn_detached(async move {
            loop {
                let buses = Arc::clone(&buses);
                let result = redis
                    .subscribe(move |encoded| {
                        let Ok(envelope) = FanoutEnvelope::decode(&encoded) else {
                            log::warn!("discarding invalid Redis collaboration envelope");
                            return;
                        };
                        let community_id = envelope.community_id();
                        let Ok(route) = TrustedTenantRoute::from_deployment(
                            community_id,
                            "redis:zed.collaboration.message.v1",
                        ) else {
                            return;
                        };
                        let Ok(tenant) = TenantContext::establish(Some(route), &[]) else {
                            return;
                        };
                        let Ok(bus) = bus_for(&buses, community_id) else {
                            log::warn!("could not allocate collaboration subscription bus");
                            return;
                        };
                        if let Err(error) = bus.receive_remote(&tenant, &encoded) {
                            log::warn!("could not admit Redis collaboration envelope: {error}");
                        }
                    })
                    .await;
                if let Err(error) = result {
                    log::warn!("Redis collaboration notifications unavailable; PostgreSQL replay remains authoritative: {error}");
                }
                executor.sleep(std::time::Duration::from_secs(1)).await;
            }
        });
    }

    pub async fn publish_committed(
        &self,
        authorization: &AuthorizedChannel,
        operation_id: OperationId,
    ) -> Result<FanoutEnvelope, SubscriptionBusError> {
        let envelope = self
            .service
            .replay()
            .envelope_for_operation(&authorization.tenant, operation_id)
            .await?;
        let bus = bus_for(&self.buses, authorization.tenant.community_id())?;
        bus.publish_authoritative(&authorization.tenant, envelope.clone(), &LocalTransport)
            .await?;
        if let Some(redis) = &self.redis
            && let Err(error) = redis.publish(envelope.encode()?).await
        {
            log::warn!(
                "Redis collaboration notification failed; clients will recover from PostgreSQL: {error}"
            );
        }
        Ok(envelope)
    }

    pub async fn subscribe(
        &self,
        authorization: AuthorizedChannel,
        connection_id: ConnectionId,
        after_sequence: u64,
        peer: Arc<Peer>,
    ) -> Result<(), SubscriptionBusError> {
        self.close(connection_id, authorization.channel_id);
        let bus = bus_for(&self.buses, authorization.tenant.community_id())?;
        let subscription = bus
            .subscribe(
                &authorization.tenant,
                channel_topic(authorization.channel_id),
                after_sequence,
                SUBSCRIPTION_QUEUE_CAPACITY,
                REPLAY_BATCH,
                self.service.replay(),
            )
            .await?;
        let service = Arc::clone(&self.service);
        let database = Arc::clone(&self.database);
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        self.subscriptions
            .insert((connection_id, authorization.channel_id), abort_handle);
        self.executor.spawn_detached(
            Abortable::new(
                run_subscription(
                    subscription,
                    database,
                    service,
                    authorization,
                    connection_id,
                    peer,
                ),
                abort_registration,
            )
            .map(|_| ()),
        );
        Ok(())
    }

    pub fn close(&self, connection_id: ConnectionId, channel_id: AggregateId) {
        if let Some((_, handle)) = self.subscriptions.remove(&(connection_id, channel_id)) {
            handle.abort();
        }
    }

    pub fn close_connection(&self, connection_id: ConnectionId) {
        let channel_ids = self
            .subscriptions
            .iter()
            .filter_map(|entry| (entry.key().0 == connection_id).then_some(entry.key().1))
            .collect::<Vec<_>>();
        for channel_id in channel_ids {
            self.close(connection_id, channel_id);
        }
    }
}

async fn run_subscription(
    mut subscription: crate::pubsub::subscription_bus::FanoutSubscription,
    database: Arc<crate::db::Database>,
    service: Arc<CanonicalMessageService>,
    authorization: AuthorizedChannel,
    connection_id: ConnectionId,
    peer: Arc<Peer>,
) {
    while let Some(event) = subscription.recv().await {
        let SubscriptionEvent::Envelope(envelope) = event else {
            break;
        };
        match database
            .run_on_database_runtime(service.authorization_is_current(&authorization))
            .await
        {
            Ok(true) => {}
            Ok(false) => {
                log::debug!("collaboration message subscription authorization was revoked");
                break;
            }
            Err(error) => {
                log::warn!("could not revalidate collaboration message subscription: {error}");
                break;
            }
        }
        let payload = match database
            .run_on_database_runtime(
                service
                    .replay()
                    .payload(&authorization.tenant, envelope.outbox_sequence()),
            )
            .await
            .ok()
            .and_then(|payload| serde_json::from_slice::<MessageOutboxPayload>(&payload).ok())
        {
            Some(payload) => payload,
            None => {
                log::warn!("could not hydrate committed collaboration outbox record");
                continue;
            }
        };
        let mut message = match database
            .run_on_database_runtime(service.message(&authorization, payload.message_id))
            .await
        {
            Ok(message) => message,
            Err(error) => {
                log::warn!("could not hydrate collaboration message update: {error}");
                continue;
            }
        };
        if let Some(message) = &mut message {
            message.accepted_operation_id = Some(payload.operation_id);
            message.outbox_sequence = envelope.outbox_sequence();
        }
        if let Err(error) = peer.send(
            connection_id,
            proto::CollaborativeMessageStreamUpdate {
                contract_version: 1,
                community_id: authorization
                    .tenant
                    .community_id()
                    .as_uuid()
                    .as_bytes()
                    .to_vec(),
                channel_id: authorization.channel_id.as_uuid().as_bytes().to_vec(),
                outbox_sequence: envelope.outbox_sequence(),
                message: message.map(message_to_proto),
                operation_kind: operation_kind_to_proto(payload.kind).into(),
                actor_principal_id: payload
                    .actor_principal_id
                    .map_or_else(Vec::new, |principal_id| {
                        principal_id.as_uuid().as_bytes().to_vec()
                    }),
                acknowledged_outbox_sequence: payload
                    .acknowledged_outbox_sequence
                    .unwrap_or_default(),
            },
        ) {
            log::debug!("collaboration message subscriber disconnected: {error}");
            break;
        }
    }
    subscription.cancel();
}

fn operation_kind_to_proto(
    kind: crate::messages::channel_mutation::MessageOperationKind,
) -> proto::CollaborativeMessageOperationKind {
    use crate::messages::channel_mutation::MessageOperationKind;
    match kind {
        MessageOperationKind::Create => {
            proto::CollaborativeMessageOperationKind::CollaborativeMessageCreate
        }
        MessageOperationKind::Edit => {
            proto::CollaborativeMessageOperationKind::CollaborativeMessageEdit
        }
        MessageOperationKind::Delete => {
            proto::CollaborativeMessageOperationKind::CollaborativeMessageDelete
        }
        MessageOperationKind::ReactionAdd => {
            proto::CollaborativeMessageOperationKind::CollaborativeMessageReactionAdd
        }
        MessageOperationKind::ReactionRemove => {
            proto::CollaborativeMessageOperationKind::CollaborativeMessageReactionRemove
        }
        MessageOperationKind::Acknowledge => {
            proto::CollaborativeMessageOperationKind::CollaborativeMessageAcknowledge
        }
    }
}

pub fn page_to_proto(page: HydratedPage) -> proto::CollaborativeMessagePage {
    proto::CollaborativeMessagePage {
        messages: page.messages.into_iter().map(message_to_proto).collect(),
        next_cursor: page
            .next_cursor
            .map(|cursor| proto::CollaborativeMessageCursor {
                snapshot_micros: page.snapshot.as_micros(),
                message_created_at: cursor.message_created_at,
                source_event_id: cursor.source_event_id.as_bytes().to_vec(),
                outbox_sequence: page.authoritative_outbox_cursor,
            }),
        done: !page.has_more,
        authoritative_outbox_cursor: page.authoritative_outbox_cursor,
    }
}

pub fn message_to_proto(message: HydratedMessage) -> proto::CollaborativeMessageRecord {
    proto::CollaborativeMessageRecord {
        community_id: message.community_id.as_uuid().as_bytes().to_vec(),
        channel_id: message.channel_id.as_uuid().as_bytes().to_vec(),
        message_id: message.message_id.as_uuid().as_bytes().to_vec(),
        source_event_id: message.source_event_id.as_bytes().to_vec(),
        current_event_id: message.current_event_id.as_bytes().to_vec(),
        author_principal_id: message.author_principal_id.as_uuid().as_bytes().to_vec(),
        author_display_name: message.author_display_name,
        author_avatar_url: message.author_avatar_url,
        body: message.body,
        created_at: message.created_at,
        version: message.version,
        edited: message.edited,
        deleted: message.deleted,
        reply_to_event_id: message
            .reply_to_event_id
            .map_or_else(Vec::new, |event_id| event_id.as_bytes().to_vec()),
        reactions: message
            .reactions
            .into_iter()
            .map(|reaction| proto::CollaborativeMessageReaction {
                value: reaction.value,
                actor_principal_id: reaction.actor_principal_id.as_uuid().as_bytes().to_vec(),
                source_event_id: reaction.source_event_id.as_bytes().to_vec(),
            })
            .collect(),
        accepted_operation_id: message
            .accepted_operation_id
            .map_or_else(String::new, |operation_id| operation_id.to_string()),
        outbox_sequence: message.outbox_sequence,
        reaction_version: message.reaction_version,
    }
}

fn bus_for(
    buses: &DashMap<CommunityId, SubscriptionBus>,
    community_id: CommunityId,
) -> Result<SubscriptionBus, SubscriptionBusError> {
    if let Some(bus) = buses.get(&community_id) {
        return Ok(bus.clone());
    }
    let bus = SubscriptionBus::new(community_id, DEDUPLICATION_CAPACITY)?;
    Ok(buses.entry(community_id).or_insert(bus).clone())
}

struct LocalTransport;

#[async_trait]
impl FanoutTransport for LocalTransport {
    async fn publish(&self, _encoded_envelope: Vec<u8>) -> Result<(), SubscriptionBusError> {
        Ok(())
    }
}
