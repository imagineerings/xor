use crate::{
    AppState, Result,
    db::{Database, join_request_store::JoinRequestStore},
};
use rpc::Notification;
use std::{env, sync::Arc, time::Duration};
use time::{OffsetDateTime, PrimitiveDateTime};
use util::ResultExt as _;

const DEFAULT_JOIN_REQUEST_TTL_SECS: i64 = 7 * 24 * 60 * 60;
const JOIN_REQUEST_EXPIRY_INTERVAL: Duration = Duration::from_secs(60 * 60);

pub fn expire_join_requests_periodically(app_state: Arc<AppState>) {
    let executor = app_state.executor.clone();
    executor.spawn_detached({
        let executor = executor.clone();
        async move {
            loop {
                expire_join_requests(app_state.db.clone()).await.log_err();
                executor.sleep(JOIN_REQUEST_EXPIRY_INTERVAL).await;
            }
        }
    });
}

pub async fn expire_join_requests(db: Arc<Database>) -> Result<()> {
    let ttl_secs = env::var("CHANNEL_JOIN_REQUEST_TTL_SECS")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|ttl_secs| *ttl_secs >= 0)
        .unwrap_or(DEFAULT_JOIN_REQUEST_TTL_SECS);
    expire_join_requests_with_ttl(db, time::Duration::seconds(ttl_secs)).await
}

pub async fn expire_join_requests_with_ttl(db: Arc<Database>, ttl: time::Duration) -> Result<()> {
    let now = OffsetDateTime::now_utc();
    let threshold = now - ttl;
    let threshold = PrimitiveDateTime::new(threshold.date(), threshold.time());
    let expired_requests = JoinRequestStore::new(db.clone())
        .expire_old_requests(threshold)
        .await?;

    db.transaction(|tx| {
        let db = db.clone();
        let expired_requests = expired_requests.clone();
        async move {
            for request in expired_requests {
                db.create_notification(
                    request.user_id,
                    Notification::JoinRequestDenied {
                        channel_id: request.channel_id.to_proto(),
                        channel_name: request.channel_name,
                        reason: Some("Your join request has expired.".to_string()),
                    },
                    false,
                    &tx,
                )
                .await?;
            }
            Ok(())
        }
    })
    .await
}
