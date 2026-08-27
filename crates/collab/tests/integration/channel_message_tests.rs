use collaboration_domain::{
    AggregateId, CommunityId, OperationId, TenantContext, TrustedTenantRoute,
    channel_id_for_legacy_channel, community_id_for_legacy_root_channel,
};
use gpui::{AppContext as _, BackgroundExecutor, TestAppContext};
use nostr_compat::{CanonicalEvent, EventSignature, PublicKey, SignedEvent};
use rpc::{RECEIVE_TIMEOUT, TypedEnvelope, proto};
use sea_orm::{ConnectionTrait as _, DatabaseConnection, Statement, TransactionTrait as _};
use secp256k1::{Keypair, Message, Secp256k1, SecretKey, XOnlyPublicKey};
use sqlx::migrate::MigrateDatabase as _;
use uuid::Uuid;

use crate::{TestServer, db_tests::run_database_migrations};
use collab::{
    pubsub::{
        postgres::PostgresFanoutReplayStore,
        redis::RedisFanoutTransport,
        subscription_bus::{FanoutReplayStore as _, FanoutTransport as _},
    },
    rpc::RECONNECT_TIMEOUT,
};

struct UpdateReceiver;

struct TestSigner {
    secret: SecretKey,
    public_key: [u8; 32],
}

impl TestSigner {
    fn new(seed: u8) -> Self {
        let mut bytes = [0; 32];
        bytes[31] = seed;
        let secret = SecretKey::from_slice(&bytes).expect("valid deterministic secret");
        let secp = Secp256k1::new();
        let keypair = Keypair::from_secret_key(&secp, &secret);
        let (public_key, _) = XOnlyPublicKey::from_keypair(&keypair);
        Self {
            secret,
            public_key: public_key.serialize(),
        }
    }

    fn event(
        &self,
        created_at: u64,
        kind: u16,
        tags: Vec<Vec<String>>,
        content: &str,
    ) -> proto::CollaborativeSignedEvent {
        let event = CanonicalEvent::new(
            PublicKey::from_bytes(self.public_key),
            created_at,
            kind,
            tags,
            content.to_owned(),
        );
        let claimed_id = event.event_id().expect("canonical event id");
        let secp = Secp256k1::new();
        let keypair = Keypair::from_secret_key(&secp, &self.secret);
        let signature =
            secp.sign_schnorr_no_aux_rand(&Message::from_digest(*claimed_id.as_bytes()), &keypair);
        let signed = SignedEvent {
            claimed_id,
            event,
            signature: EventSignature::from_hex(&signature.to_string())
                .expect("canonical signature"),
        };
        proto::CollaborativeSignedEvent {
            claimed_event_id: signed.claimed_id.as_bytes().to_vec(),
            public_key: signed.event.public_key.as_bytes().to_vec(),
            created_at: signed.event.created_at,
            kind: u32::from(signed.event.kind),
            tags: signed
                .event
                .tags
                .into_iter()
                .map(|values| proto::CollaborativeEventTag { values })
                .collect(),
            content: signed.event.content,
            signature: signed.signature.as_bytes().to_vec(),
        }
    }
}

#[gpui::test]
async fn canonical_channel_messages_round_trip_between_two_clients(
    executor: BackgroundExecutor,
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
    cx_denied: &mut TestAppContext,
) {
    if std::env::var("CI").is_ok() && !cfg!(target_os = "linux") {
        return;
    }

    let mut server = TestServer::start_postgres(executor.clone()).await;
    let client_a = server.create_client(cx_a, "message_author").await;
    let client_b = server.create_client(cx_b, "message_reader").await;
    let denied_client = server.create_client(cx_denied, "message_outsider").await;
    let channel_id = server
        .make_channel(
            "canonical-messages",
            None,
            (&client_a, cx_a),
            &mut [(&client_b, cx_b)],
        )
        .await;
    executor.run_until_parked();

    let community_id = community_id_for_legacy_root_channel(channel_id.0);
    let canonical_channel_id = channel_id_for_legacy_channel(channel_id.0);
    let signer_a = TestSigner::new(1);
    let signer_b = TestSigner::new(2);

    let (updates_tx, updates_rx) = async_channel::bounded(32);
    let update_receiver = cx_b.new(|_| UpdateReceiver);
    let _update_subscription = client_b.add_message_handler(
        update_receiver.downgrade(),
        move |_, envelope: TypedEnvelope<proto::CollaborativeMessageStreamUpdate>, _| {
            let updates_tx = updates_tx.clone();
            async move {
                updates_tx.send(envelope.payload).await?;
                anyhow::Ok(())
            }
        },
    );

    let open_a = client_a
        .request(open_request(
            community_id,
            canonical_channel_id,
            signer_a.public_key,
            0,
        ))
        .await
        .expect("author opens channel");
    assert_eq!(
        open_a.error_code,
        proto::CollaborativeMessageErrorCode::CollaborativeMessageErrorNone as i32
    );
    let open_b = client_b
        .request(open_request(
            community_id,
            canonical_channel_id,
            signer_b.public_key,
            0,
        ))
        .await
        .expect("reader opens channel");
    assert_eq!(
        open_b.error_code,
        proto::CollaborativeMessageErrorCode::CollaborativeMessageErrorNone as i32
    );

    let denied = denied_client
        .request(open_request(
            community_id,
            canonical_channel_id,
            TestSigner::new(3).public_key,
            0,
        ))
        .await
        .expect("denied open has typed response");
    assert_eq!(
        denied.error_code,
        proto::CollaborativeMessageErrorCode::CollaborativeMessageErrorDenied as i32
    );
    let cross_tenant = client_a
        .request(open_request(
            CommunityId::new(),
            canonical_channel_id,
            signer_a.public_key,
            0,
        ))
        .await
        .expect("cross-tenant open has typed response");
    assert_eq!(
        cross_tenant.error_code,
        proto::CollaborativeMessageErrorCode::CollaborativeMessageErrorDenied as i32
    );

    let now = unix_time_seconds();
    let message_id = AggregateId::new();
    let create_operation = OperationId::new();
    let create_event = signer_a.event(
        now,
        40_002,
        vec![vec!["h".into(), canonical_channel_id.to_string()]],
        "hello from client A",
    );
    let create_request = operation_request(
        community_id,
        canonical_channel_id,
        message_id,
        create_operation,
        proto::CollaborativeMessageOperationKind::CollaborativeMessageCreate,
        0,
        "hello from client A",
        "",
        create_event.clone(),
    );
    let created = client_a
        .request(create_request.clone())
        .await
        .expect("create request");
    assert!(
        created.accepted,
        "create rejected with error code {} and authoritative version {}",
        created.error_code, created.authoritative_version
    );
    assert!(!created.duplicate);
    let created_record = created.message.expect("created message record");
    assert_eq!(created_record.body, "hello from client A");

    let duplicate = client_a
        .request(create_request)
        .await
        .expect("idempotent retry");
    assert!(duplicate.accepted);
    assert!(duplicate.duplicate);
    assert_eq!(
        duplicate.message.expect("duplicate record").message_id,
        created_record.message_id
    );

    let delivered = updates_rx.recv().await.expect("reader receives create");
    assert_eq!(delivered.outbox_sequence, created_record.outbox_sequence);
    assert_eq!(
        delivered.message.expect("delivered record").body,
        "hello from client A"
    );

    server.forbid_connections();
    server.disconnect_client(client_b.peer_id().expect("reader peer id"));
    executor.advance_clock(RECEIVE_TIMEOUT + RECONNECT_TIMEOUT);
    assert!(!client_b.status().borrow().is_connected());

    let edited = client_a
        .request(operation_request(
            community_id,
            canonical_channel_id,
            message_id,
            OperationId::new(),
            proto::CollaborativeMessageOperationKind::CollaborativeMessageEdit,
            created_record.version,
            "edited by client A",
            "",
            signer_a.event(
                now + 1,
                40_003,
                vec![vec![
                    "e".into(),
                    hex::encode(&created_record.source_event_id),
                ]],
                "edited by client A",
            ),
        ))
        .await
        .expect("edit request");
    assert!(edited.accepted);
    let edited_record = edited.message.expect("edited message record");
    assert_eq!(edited_record.body, "edited by client A");
    assert_eq!(edited_record.version, created_record.version + 1);

    server.allow_connections();
    executor.advance_clock(RECEIVE_TIMEOUT + RECONNECT_TIMEOUT);
    assert!(client_b.status().borrow().is_connected());
    let recovered_after_disconnect = client_b
        .request(open_request(
            community_id,
            canonical_channel_id,
            signer_b.public_key,
            created_record.outbox_sequence,
        ))
        .await
        .expect("reader reopens channel from authoritative cursor");
    let recovered_page = recovered_after_disconnect.page.expect("recovered page");
    assert_eq!(recovered_page.messages.len(), 1);
    assert_eq!(recovered_page.messages[0].body, "edited by client A");
    assert_eq!(
        recovered_page.messages[0].message_id,
        created_record.message_id
    );

    let stale = client_a
        .request(operation_request(
            community_id,
            canonical_channel_id,
            message_id,
            OperationId::new(),
            proto::CollaborativeMessageOperationKind::CollaborativeMessageEdit,
            created_record.version,
            "stale edit",
            "",
            signer_a.event(
                now + 2,
                40_003,
                vec![vec![
                    "e".into(),
                    hex::encode(&created_record.source_event_id),
                ]],
                "stale edit",
            ),
        ))
        .await
        .expect("stale edit response");
    assert!(!stale.accepted);
    assert_eq!(
        stale.error_code,
        proto::CollaborativeMessageErrorCode::CollaborativeMessageErrorStaleVersion as i32
    );

    let reacted = client_b
        .request(operation_request(
            community_id,
            canonical_channel_id,
            message_id,
            OperationId::new(),
            proto::CollaborativeMessageOperationKind::CollaborativeMessageReactionAdd,
            edited_record.reaction_version,
            "👍",
            "👍",
            signer_b.event(
                now + 3,
                7,
                vec![vec![
                    "e".into(),
                    hex::encode(&created_record.source_event_id),
                ]],
                "👍",
            ),
        ))
        .await
        .expect("reaction request");
    assert!(reacted.accepted);
    let reacted_record = reacted.message.expect("reacted message record");
    assert_eq!(reacted_record.reactions.len(), 1);

    let acknowledged = client_b
        .request(proto::ApplyCollaborativeMessageOperation {
            contract_version: 1,
            community_id: community_id.as_uuid().as_bytes().to_vec(),
            channel_id: canonical_channel_id.as_uuid().as_bytes().to_vec(),
            message_id: AggregateId::new().as_uuid().as_bytes().to_vec(),
            operation_id: OperationId::new().to_string(),
            kind: proto::CollaborativeMessageOperationKind::CollaborativeMessageAcknowledge.into(),
            expected_version: 0,
            body: String::new(),
            reply_to_event_id: Vec::new(),
            reaction: String::new(),
            related_reaction_event_id: Vec::new(),
            signed_event: None,
            acknowledged_outbox_sequence: reacted_record.outbox_sequence,
        })
        .await
        .expect("acknowledge request");
    assert!(acknowledged.accepted);

    let history = client_b
        .request(open_request(
            community_id,
            canonical_channel_id,
            signer_b.public_key,
            reacted_record.outbox_sequence,
        ))
        .await
        .expect("history after reconnect");
    let page = history.page.expect("history page");
    assert_eq!(page.messages.len(), 1);
    assert_eq!(page.messages[0].body, "edited by client A");
    assert_eq!(page.messages[0].reactions.len(), 1);

    let deleted = client_a
        .request(operation_request(
            community_id,
            canonical_channel_id,
            message_id,
            OperationId::new(),
            proto::CollaborativeMessageOperationKind::CollaborativeMessageDelete,
            edited_record.version,
            "",
            "",
            signer_a.event(
                now + 4,
                5,
                vec![vec![
                    "e".into(),
                    hex::encode(&created_record.source_event_id),
                ]],
                "",
            ),
        ))
        .await
        .expect("delete request");
    assert!(deleted.accepted);
    assert!(deleted.message.expect("deleted record").deleted);

    client_a
        .channel_store()
        .update(cx_a, |store, cx| {
            store.remove_member(channel_id, client_b.user_id().expect("reader user id"), cx)
        })
        .await
        .expect("remove channel member");
    executor.run_until_parked();
    while updates_rx.try_recv().is_ok() {}

    let after_revocation = client_a
        .request(operation_request(
            community_id,
            canonical_channel_id,
            AggregateId::new(),
            OperationId::new(),
            proto::CollaborativeMessageOperationKind::CollaborativeMessageCreate,
            0,
            "not visible after revocation",
            "",
            signer_a.event(
                now + 5,
                40_002,
                vec![vec!["h".into(), canonical_channel_id.to_string()]],
                "not visible after revocation",
            ),
        ))
        .await
        .expect("authorized member sends after revocation");
    assert!(after_revocation.accepted);
    executor.run_until_parked();
    assert!(
        updates_rx.try_recv().is_err(),
        "removed member subscription must close before message hydration"
    );

    let removed_member = client_b
        .request(open_request(
            community_id,
            canonical_channel_id,
            signer_b.public_key,
            0,
        ))
        .await
        .expect("removed member receives typed denial");
    assert_eq!(
        removed_member.error_code,
        proto::CollaborativeMessageErrorCode::CollaborativeMessageErrorDenied as i32
    );
}

#[gpui::test]
async fn canonical_channel_history_paginates_dense_timestamps_without_duplicates(
    executor: BackgroundExecutor,
    cx: &mut TestAppContext,
) {
    if std::env::var("CI").is_ok() && !cfg!(target_os = "linux") {
        return;
    }

    let mut server = TestServer::start_postgres(executor.clone()).await;
    let client = server.create_client(cx, "dense_history_author").await;
    let channel_id = server
        .make_channel("dense-history", None, (&client, cx), &mut [])
        .await;
    executor.run_until_parked();
    let community_id = community_id_for_legacy_root_channel(channel_id.0);
    let canonical_channel_id = channel_id_for_legacy_channel(channel_id.0);
    let signer = TestSigner::new(4);
    let opened = client
        .request(open_request(
            community_id,
            canonical_channel_id,
            signer.public_key,
            0,
        ))
        .await
        .expect("author opens dense history channel");
    assert_eq!(
        opened.error_code,
        proto::CollaborativeMessageErrorCode::CollaborativeMessageErrorNone as i32
    );

    let created_at = unix_time_seconds();
    for index in 0..3 {
        let message_id = AggregateId::new();
        let body = format!("dense message {index}");
        let response = client
            .request(operation_request(
                community_id,
                canonical_channel_id,
                message_id,
                OperationId::new(),
                proto::CollaborativeMessageOperationKind::CollaborativeMessageCreate,
                0,
                &body,
                "",
                signer.event(
                    created_at,
                    40_002,
                    vec![vec!["h".into(), canonical_channel_id.to_string()]],
                    &body,
                ),
            ))
            .await
            .expect("create dense timestamp message");
        assert!(response.accepted);
    }

    let first = client
        .request(proto::GetCollaborativeMessageWindow {
            contract_version: 1,
            community_id: community_id.as_uuid().as_bytes().to_vec(),
            channel_id: canonical_channel_id.as_uuid().as_bytes().to_vec(),
            thread_root_event_id: Vec::new(),
            page_size: 2,
            cursor: None,
        })
        .await
        .expect("first dense history page")
        .page
        .expect("first page payload");
    assert_eq!(first.messages.len(), 2);
    assert!(!first.done);
    let cursor = first.next_cursor.expect("dense history cursor");
    let second = client
        .request(proto::GetCollaborativeMessageWindow {
            contract_version: 1,
            community_id: community_id.as_uuid().as_bytes().to_vec(),
            channel_id: canonical_channel_id.as_uuid().as_bytes().to_vec(),
            thread_root_event_id: Vec::new(),
            page_size: 2,
            cursor: Some(cursor),
        })
        .await
        .expect("second dense history page")
        .page
        .expect("second page payload");
    assert_eq!(second.messages.len(), 1);
    assert!(second.done);
    let unique_message_ids = first
        .messages
        .iter()
        .chain(&second.messages)
        .map(|message| message.message_id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(unique_message_ids.len(), 3);
}

#[tokio::test(flavor = "multi_thread")]
async fn postgres_replay_recovers_when_redis_notification_is_unavailable() -> anyhow::Result<()> {
    let Ok(base_url) = std::env::var("COLLAB_TEST_DATABASE_URL") else {
        return Ok(());
    };
    let mut database_url = url::Url::parse(&base_url)?;
    database_url.set_path(&format!("zed-test-{}", Uuid::new_v4()));
    let database_url = database_url.to_string();
    sqlx::Postgres::create_database(&database_url).await?;

    let test_result = run_redis_loss_replay_test(&database_url).await;
    let drop_result = sqlx::Postgres::drop_database(&database_url).await;
    test_result?;
    drop_result?;
    Ok(())
}

async fn run_redis_loss_replay_test(database_url: &str) -> anyhow::Result<()> {
    sqlx::any::install_default_drivers();
    let options = sea_orm::ConnectOptions::new(database_url.to_owned());
    run_database_migrations(&options, concat!(env!("CARGO_MANIFEST_DIR"), "/migrations")).await?;
    let connection = sea_orm::Database::connect(options).await?;
    let community_id = CommunityId::new();
    let operation_id = OperationId::new();
    let topic = format!("community.{}.channel.test", community_id);
    let transaction = connection.begin().await?;
    transaction
        .execute(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT set_config('app.community_id', $1, true)",
            [community_id.to_string().into()],
        ))
        .await?;
    transaction
        .execute(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            r#"
INSERT INTO public.collaboration_command_receipts (
    community_id, operation_id, contract_version, principal_id, originating_adapter,
    command_kind, command_fingerprint, authoritative_version, accepted_at
) VALUES ($1, $2, 1, $3, 'zed_rpc', 'test.redis_loss', $4, 1, clock_timestamp())
"#,
            [
                community_id.as_uuid().into(),
                operation_id.as_uuid().into(),
                Uuid::new_v4().into(),
                vec![7_u8; 32].into(),
            ],
        ))
        .await?;
    transaction
        .execute(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            r#"
INSERT INTO public.collaboration_outbox (
    community_id, operation_id, authoritative_version, topic, source_system,
    source_record_id, source_version, source_observed_at, payload
) VALUES ($1, $2, 1, $3, 'zed', $4, '1', clock_timestamp(), $5)
"#,
            [
                community_id.as_uuid().into(),
                operation_id.as_uuid().into(),
                topic.clone().into(),
                operation_id.to_string().into(),
                b"committed-message".to_vec().into(),
            ],
        ))
        .await?;
    transaction.commit().await?;

    let route = TrustedTenantRoute::from_deployment(community_id, "test:redis-loss")?;
    let tenant = TenantContext::establish(Some(route), &[])?;
    let replay = PostgresFanoutReplayStore::new(DatabaseConnection::from(
        connection.get_postgres_connection_pool().clone(),
    ))?;
    let envelope = replay.envelope_for_operation(&tenant, operation_id).await?;
    let encoded = envelope.encode()?;

    if let Ok(redis_url) = std::env::var("COLLAB_TEST_REDIS_URL") {
        let redis = RedisFanoutTransport::new(&redis_url)?;
        let subscriber = redis.clone();
        let expected = encoded.clone();
        let (delivered_tx, delivered_rx) = tokio::sync::oneshot::channel();
        let mut delivered_tx = Some(delivered_tx);
        let subscription = tokio::spawn(async move {
            subscriber
                .subscribe(move |payload| {
                    if payload == expected
                        && let Some(delivered_tx) = delivered_tx.take()
                    {
                        delivered_tx
                            .send(())
                            .expect("Redis delivery receiver remains active");
                    }
                })
                .await
        });
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        redis.publish(encoded.clone()).await?;
        tokio::time::timeout(std::time::Duration::from_secs(5), delivered_rx).await??;
        subscription.abort();
    }

    let unavailable_redis = RedisFanoutTransport::new("redis://127.0.0.1:1")?;
    assert!(unavailable_redis.publish(encoded).await.is_err());
    let recovered = replay.load_after(&tenant, &topic, 0, 16).await?;
    assert_eq!(recovered, vec![envelope]);
    assert_eq!(
        replay
            .payload(&tenant, recovered[0].outbox_sequence())
            .await?,
        b"committed-message"
    );
    connection.close().await?;
    Ok(())
}

fn open_request(
    community_id: collaboration_domain::CommunityId,
    channel_id: AggregateId,
    public_key: [u8; 32],
    after_outbox_sequence: u64,
) -> proto::OpenCollaborativeChannel {
    proto::OpenCollaborativeChannel {
        contract_version: 1,
        community_id: community_id.as_uuid().as_bytes().to_vec(),
        channel_id: channel_id.as_uuid().as_bytes().to_vec(),
        page_size: 100,
        after_outbox_sequence,
        signing_public_key: public_key.to_vec(),
    }
}

fn operation_request(
    community_id: collaboration_domain::CommunityId,
    channel_id: AggregateId,
    message_id: AggregateId,
    operation_id: OperationId,
    kind: proto::CollaborativeMessageOperationKind,
    expected_version: u64,
    body: &str,
    reaction: &str,
    signed_event: proto::CollaborativeSignedEvent,
) -> proto::ApplyCollaborativeMessageOperation {
    proto::ApplyCollaborativeMessageOperation {
        contract_version: 1,
        community_id: community_id.as_uuid().as_bytes().to_vec(),
        channel_id: channel_id.as_uuid().as_bytes().to_vec(),
        message_id: message_id.as_uuid().as_bytes().to_vec(),
        operation_id: operation_id.to_string(),
        kind: kind.into(),
        expected_version,
        body: body.to_owned(),
        reply_to_event_id: Vec::new(),
        reaction: reaction.to_owned(),
        related_reaction_event_id: Vec::new(),
        signed_event: Some(signed_event),
        acknowledged_outbox_sequence: 0,
    }
}

fn unix_time_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
