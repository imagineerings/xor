use async_trait::async_trait;
use collab::{
    collaboration_command::{
        CommandAdapter, DomainCommand, DomainCommandDisposition, DomainCommandSink,
        DomainCommandSubmissionError,
    },
    db::collaboration::{
        outbox::{
            AppliedCommand, CommandFingerprint, OutboxOperation, TransactionalCommandMutation,
            TransactionalCommandOutbox,
        },
        rebuild::{
            ProjectionDriftState, ProjectionMaterialization, ProjectionRebuildAdapter,
            ProjectionRebuildError, ProjectionRebuilder, ProjectionRow, ProjectionRows,
            ProjectionSource,
        },
    },
    tenant_admission::bind_rpc_tenant,
};
use collaboration_domain::{
    AggregateVersion, AuthenticatedPrincipal, CommunityId, OperationId, PrincipalId,
    PrincipalScopes, Provenance, SourceRecordId, SourceSystem, TenantContext, TrustedTenantRoute,
};
use sea_orm::{
    ConnectionTrait, DatabaseBackend, DatabaseTransaction, QueryResult, Statement, Value,
};
use sqlx::{Executor, PgPool, postgres::PgPoolOptions};
use url::Url;
use uuid::Uuid;

const EVENTS_UP: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/migrations/20260820000100_collaboration_events.up.sql"
));
const HEADS_UP: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/migrations/20260820000200_collaboration_event_heads.up.sql"
));
const PROJECTIONS_UP: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/migrations/20260820000300_collaboration_projections.up.sql"
));
const PROJECTIONS_DOWN: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/migrations/20260820000300_collaboration_projections.down.sql"
));
const OUTBOX_UP: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/migrations/20260820000400_collaboration_outbox.up.sql"
));

fn tenant() -> TenantContext {
    let community_id = CommunityId::from_uuid(Uuid::from_u128(1));
    bind_rpc_tenant(
        Some(
            TrustedTenantRoute::from_listener(community_id, "storage-recovery")
                .expect("trusted tenant route"),
        ),
        &[],
    )
    .expect("tenant")
}

fn command(tenant: &TenantContext) -> DomainCommand<Vec<u8>> {
    let principal = AuthenticatedPrincipal::service(
        PrincipalId::from_uuid(Uuid::from_u128(2)),
        tenant.community_id(),
        "storage-recovery",
        PrincipalScopes::default(),
    )
    .expect("service principal");
    DomainCommand::new(
        OperationId::from_uuid(Uuid::from_u128(3)),
        tenant.clone(),
        principal,
        None,
        None,
        CommandAdapter::NostrInProcess,
        b"accepted-command".to_vec(),
    )
}

fn projection_source() -> ProjectionSource {
    ProjectionSource::new(
        "conversation_activity",
        Provenance::new(
            SourceSystem::Sim,
            SourceRecordId::new("conversation:42").expect("source ID"),
            1_900_000_000_001,
        )
        .with_source_version("2"),
    )
    .expect("projection source")
}

struct RecoveryMutation;

#[async_trait]
impl TransactionalCommandMutation<Vec<u8>> for RecoveryMutation {
    fn fingerprint(
        &self,
        command: &DomainCommand<Vec<u8>>,
    ) -> Result<CommandFingerprint, DomainCommandSubmissionError> {
        CommandFingerprint::new("recovery.accept", command.payload())
    }

    async fn apply(
        &self,
        transaction: &DatabaseTransaction,
        command: &DomainCommand<Vec<u8>>,
    ) -> Result<AppliedCommand, DomainCommandSubmissionError> {
        let result = transaction
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                r#"
INSERT INTO public.collaboration_recovery_commands (
    community_id, operation_id, authoritative_version, payload
) VALUES ($1, $2, 1, $3)
"#,
                [
                    Value::Uuid(Some(Box::new(command.tenant().community_id().as_uuid()))),
                    Value::Uuid(Some(Box::new(command.operation_id().as_uuid()))),
                    command.payload().clone().into(),
                ],
            ))
            .await
            .map_err(|_| DomainCommandSubmissionError::Unavailable)?;
        if result.rows_affected() != 1 {
            return Err(DomainCommandSubmissionError::Unavailable);
        }
        let operation = OutboxOperation::new(
            "recovery.accepted",
            Provenance::new(
                SourceSystem::Sim,
                SourceRecordId::new(command.operation_id().to_string()).expect("source ID"),
                1_900_000_000_000,
            )
            .with_source_version("1"),
            command.payload().clone(),
        )?;
        Ok(AppliedCommand::new(AggregateVersion::FIRST, operation))
    }
}

struct RecoveryProjectionAdapter;

#[async_trait]
impl ProjectionRebuildAdapter for RecoveryProjectionAdapter {
    async fn load_authority(
        &self,
        transaction: &DatabaseTransaction,
        tenant: &TenantContext,
        source: &ProjectionSource,
    ) -> Result<ProjectionRows, ProjectionRebuildError> {
        let rows = transaction
            .query_all(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                r#"
SELECT record_key, payload
FROM public.collaboration_recovery_authority
WHERE community_id = $1 AND source_record_id = $2
ORDER BY record_key
"#,
                [
                    Value::Uuid(Some(Box::new(tenant.community_id().as_uuid()))),
                    source.provenance().source_record_id.as_str().into(),
                ],
            ))
            .await
            .map_err(ProjectionRebuildError::Unavailable)?;
        projection_rows(rows)
    }

    async fn replace_projection(
        &self,
        transaction: &DatabaseTransaction,
        tenant: &TenantContext,
        source: &ProjectionSource,
        rows: &ProjectionRows,
    ) -> Result<(), ProjectionRebuildError> {
        transaction
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                r#"
DELETE FROM public.collaboration_recovery_projection
WHERE community_id = $1 AND source_record_id = $2
"#,
                [
                    Value::Uuid(Some(Box::new(tenant.community_id().as_uuid()))),
                    source.provenance().source_record_id.as_str().into(),
                ],
            ))
            .await
            .map_err(ProjectionRebuildError::Unavailable)?;
        for row in rows.as_slice() {
            transaction
                .execute(Statement::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    r#"
INSERT INTO public.collaboration_recovery_projection (
    community_id, source_record_id, source_version, record_key, payload
) VALUES ($1, $2, $3, $4, $5)
"#,
                    [
                        Value::Uuid(Some(Box::new(tenant.community_id().as_uuid()))),
                        source.provenance().source_record_id.as_str().into(),
                        source.provenance().source_version.as_deref().into(),
                        row.key().into(),
                        row.payload().to_vec().into(),
                    ],
                ))
                .await
                .map_err(ProjectionRebuildError::Unavailable)?;
        }
        Ok(())
    }

    async fn load_projection(
        &self,
        transaction: &DatabaseTransaction,
        tenant: &TenantContext,
        source: &ProjectionSource,
    ) -> Result<ProjectionMaterialization, ProjectionRebuildError> {
        let rows = transaction
            .query_all(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                r#"
SELECT source_version, record_key, payload
FROM public.collaboration_recovery_projection
WHERE community_id = $1 AND source_record_id = $2
ORDER BY record_key
"#,
                [
                    Value::Uuid(Some(Box::new(tenant.community_id().as_uuid()))),
                    source.provenance().source_record_id.as_str().into(),
                ],
            ))
            .await
            .map_err(ProjectionRebuildError::Unavailable)?;
        let source_version = rows
            .first()
            .map(|row| row.try_get("", "source_version"))
            .transpose()
            .map_err(ProjectionRebuildError::Unavailable)?;
        ProjectionMaterialization::new(source_version, projection_rows(rows)?)
    }
}

fn projection_rows(rows: Vec<QueryResult>) -> Result<ProjectionRows, ProjectionRebuildError> {
    let rows = rows
        .into_iter()
        .map(|row| {
            let key: String = row
                .try_get("", "record_key")
                .map_err(ProjectionRebuildError::Unavailable)?;
            let payload: Vec<u8> = row
                .try_get("", "payload")
                .map_err(ProjectionRebuildError::Unavailable)?;
            ProjectionRow::new(key, payload)
        })
        .collect::<Result<Vec<_>, _>>()?;
    ProjectionRows::new(rows)
}

fn admin_database_url() -> String {
    std::env::var("COLLAB_TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_owned())
}

fn isolated_database_url(admin_url: &str, database_name: &str) -> anyhow::Result<String> {
    let mut url = Url::parse(admin_url)?;
    url.set_path(&format!("/{database_name}"));
    Ok(url.to_string())
}

async fn apply_recovery_migrations(pool: &PgPool) -> anyhow::Result<()> {
    for migration in [EVENTS_UP, HEADS_UP, PROJECTIONS_UP, OUTBOX_UP] {
        sqlx::raw_sql(migration).execute(pool).await?;
    }
    Ok(())
}

async fn prepare_recovery_tables(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::raw_sql(
        r#"
CREATE TABLE public.collaboration_recovery_commands (
    community_id uuid NOT NULL,
    operation_id uuid NOT NULL,
    authoritative_version bigint NOT NULL,
    payload bytea NOT NULL,
    PRIMARY KEY (community_id, operation_id)
);
CREATE TABLE public.collaboration_recovery_authority (
    community_id uuid NOT NULL,
    source_record_id text NOT NULL,
    record_key text NOT NULL,
    payload bytea NOT NULL,
    PRIMARY KEY (community_id, source_record_id, record_key)
);
CREATE TABLE public.collaboration_recovery_projection (
    community_id uuid NOT NULL,
    source_record_id text NOT NULL,
    source_version text NOT NULL,
    record_key text NOT NULL,
    payload bytea NOT NULL,
    PRIMARY KEY (community_id, source_record_id, record_key)
);
CREATE FUNCTION public.interrupt_collaboration_outbox() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'seeded outbox interruption';
END;
$$ LANGUAGE plpgsql;
CREATE TRIGGER interrupt_collaboration_outbox
    BEFORE INSERT ON public.collaboration_outbox
    FOR EACH ROW EXECUTE FUNCTION public.interrupt_collaboration_outbox();
"#,
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn exercise_storage_recovery(database_url: &str) -> anyhow::Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect(database_url)
        .await?;
    apply_recovery_migrations(&pool).await?;
    prepare_recovery_tables(&pool).await?;
    let tenant = tenant();

    {
        let connection = sea_orm::Database::connect(database_url).await?;
        let store = TransactionalCommandOutbox::new(connection, RecoveryMutation)
            .map_err(|backend| anyhow::anyhow!("unexpected database backend: {backend:?}"))?;
        let interrupted = store.submit(command(&tenant)).await;
        anyhow::ensure!(
            interrupted == Err(DomainCommandSubmissionError::Unavailable),
            "outbox interruption must fail the command"
        );
        let command_count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM public.collaboration_recovery_commands")
                .fetch_one(&pool)
                .await?;
        let receipt_count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM public.collaboration_command_receipts")
                .fetch_one(&pool)
                .await?;
        let outbox_count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM public.collaboration_outbox")
                .fetch_one(&pool)
                .await?;
        anyhow::ensure!(
            (command_count, receipt_count, outbox_count) == (0, 0, 0),
            "interrupted transaction leaked authority, receipt or outbox state"
        );

        sqlx::raw_sql(
            "DROP TRIGGER interrupt_collaboration_outbox ON public.collaboration_outbox;\
             DROP FUNCTION public.interrupt_collaboration_outbox();",
        )
        .execute(&pool)
        .await?;
        let applied = store.submit(command(&tenant)).await?;
        let duplicate = store.submit(command(&tenant)).await?;
        anyhow::ensure!(applied.disposition() == DomainCommandDisposition::Applied);
        anyhow::ensure!(duplicate.disposition() == DomainCommandDisposition::Duplicate);
        let command_count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM public.collaboration_recovery_commands")
                .fetch_one(&pool)
                .await?;
        let receipt_count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM public.collaboration_command_receipts")
                .fetch_one(&pool)
                .await?;
        let outbox_count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM public.collaboration_outbox")
                .fetch_one(&pool)
                .await?;
        anyhow::ensure!((command_count, receipt_count, outbox_count) == (1, 1, 1));
    }

    sqlx::query(
        r#"
INSERT INTO public.collaboration_events (
    community_id, event_id, author_public_key, event_created_at, kind, tags,
    content, canonical_event_bytes, signature, signature_state, verified_at,
    persistence_class, discriminator
) VALUES ($1, $2, $3, 1900000000, 1, '[]'::jsonb, 'authority survives',
          $4, $5, 'verified_historical', clock_timestamp(), 'regular', NULL)
"#,
    )
    .bind(tenant.community_id().as_uuid())
    .bind(vec![21_u8; 32])
    .bind(vec![22_u8; 32])
    .bind(b"canonical-authority".as_slice())
    .bind(vec![23_u8; 64])
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
INSERT INTO public.collaboration_recovery_authority (
    community_id, source_record_id, record_key, payload
) VALUES ($1, 'conversation:42', 'activity:1', 'v1')
"#,
    )
    .bind(tenant.community_id().as_uuid())
    .execute(&pool)
    .await?;
    sqlx::query(
        r#"
INSERT INTO public.collaboration_recovery_projection (
    community_id, source_record_id, source_version, record_key, payload
) VALUES ($1, 'conversation:42', '1', 'activity:1', 'v1')
"#,
    )
    .bind(tenant.community_id().as_uuid())
    .execute(&pool)
    .await?;

    let mut stale_reader = pool.begin().await?;
    stale_reader
        .execute("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .await?;
    let stale_before: String = sqlx::query_scalar(
        "SELECT source_version FROM public.collaboration_recovery_projection WHERE community_id = $1 AND source_record_id = 'conversation:42'",
    )
    .bind(tenant.community_id().as_uuid())
    .fetch_one(&mut *stale_reader)
    .await?;
    anyhow::ensure!(stale_before == "1");

    sqlx::query(
        "UPDATE public.collaboration_recovery_authority SET payload = 'v2' WHERE community_id = $1 AND source_record_id = 'conversation:42'",
    )
    .bind(tenant.community_id().as_uuid())
    .execute(&pool)
    .await?;
    {
        let connection = sea_orm::Database::connect(database_url).await?;
        let rebuilder = ProjectionRebuilder::new(connection, RecoveryProjectionAdapter)?;
        let diagnostic = rebuilder.rebuild(&tenant, &projection_source()).await?;
        anyhow::ensure!(diagnostic.state() == ProjectionDriftState::Clean);
    }
    let stale_after: String = sqlx::query_scalar(
        "SELECT source_version FROM public.collaboration_recovery_projection WHERE community_id = $1 AND source_record_id = 'conversation:42'",
    )
    .bind(tenant.community_id().as_uuid())
    .fetch_one(&mut *stale_reader)
    .await?;
    anyhow::ensure!(stale_after == "1", "repeatable-read lag fixture advanced");
    stale_reader.rollback().await?;
    let fresh_version: String = sqlx::query_scalar(
        "SELECT source_version FROM public.collaboration_recovery_projection WHERE community_id = $1 AND source_record_id = 'conversation:42'",
    )
    .bind(tenant.community_id().as_uuid())
    .fetch_one(&pool)
    .await?;
    let authoritative_payload: Vec<u8> = sqlx::query_scalar(
        "SELECT payload FROM public.collaboration_recovery_authority WHERE community_id = $1 AND source_record_id = 'conversation:42'",
    )
    .bind(tenant.community_id().as_uuid())
    .fetch_one(&pool)
    .await?;
    anyhow::ensure!(fresh_version == "2");
    anyhow::ensure!(authoritative_payload == b"v2");

    sqlx::raw_sql(PROJECTIONS_DOWN).execute(&pool).await?;
    let event_count: i64 = sqlx::query_scalar("SELECT count(*) FROM public.collaboration_events")
        .fetch_one(&pool)
        .await?;
    let command_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM public.collaboration_recovery_commands")
            .fetch_one(&pool)
            .await?;
    let outbox_count: i64 = sqlx::query_scalar("SELECT count(*) FROM public.collaboration_outbox")
        .fetch_one(&pool)
        .await?;
    anyhow::ensure!((event_count, command_count, outbox_count) == (1, 1, 1));
    sqlx::raw_sql(PROJECTIONS_UP).execute(&pool).await?;
    {
        let connection = sea_orm::Database::connect(database_url).await?;
        let rebuilder = ProjectionRebuilder::new(connection, RecoveryProjectionAdapter)?;
        let diagnostic = rebuilder.rebuild(&tenant, &projection_source()).await?;
        anyhow::ensure!(diagnostic.state() == ProjectionDriftState::Clean);
    }
    let recovered_checkpoint_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM public.collaboration_projection_checkpoints")
            .fetch_one(&pool)
            .await?;
    anyhow::ensure!(recovered_checkpoint_count == 1);

    pool.close().await;
    Ok(())
}

#[tokio::test]
async fn collaboration_storage_recovery_preserves_authority_across_failures() {
    let admin_url = admin_database_url();
    let database_name = format!("sim_collaboration_recovery_{}", Uuid::new_v4().simple());
    let database_url = isolated_database_url(&admin_url, &database_name)
        .expect("isolated database URL must be valid");
    let admin_pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&admin_url)
        .await
        .expect("connect to COLLAB_TEST_DATABASE_URL admin database");
    admin_pool
        .execute(format!("CREATE DATABASE {database_name}").as_str())
        .await
        .expect("create isolated collaboration recovery database");

    let result = exercise_storage_recovery(&database_url).await;
    admin_pool
        .execute(
            format!(
                "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{database_name}' AND pid <> pg_backend_pid()"
            )
            .as_str(),
        )
        .await
        .expect("terminate isolated database sessions");
    admin_pool
        .execute(format!("DROP DATABASE {database_name}").as_str())
        .await
        .expect("drop isolated collaboration recovery database");
    admin_pool.close().await;

    result.expect("isolated PostgreSQL recovery scenarios pass");
}
