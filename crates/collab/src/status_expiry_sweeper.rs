use crate::{
    Result,
    db::{Database, UserId, user_status_store::UserStatusStore},
    executor::Executor,
    rpc::ConnectionPool,
};
use rpc::{Peer, proto};
use std::{sync::Arc, time::Duration};

const STATUS_EXPIRY_INTERVAL: Duration = Duration::from_secs(30);

pub struct StatusExpirySweeper {
    db: Arc<Database>,
    executor: Executor,
    peer: Arc<Peer>,
    connection_pool: Arc<parking_lot::Mutex<ConnectionPool>>,
}

impl StatusExpirySweeper {
    pub fn new(
        db: Arc<Database>,
        executor: Executor,
        peer: Arc<Peer>,
        connection_pool: Arc<parking_lot::Mutex<ConnectionPool>>,
    ) -> Self {
        Self {
            db,
            executor,
            peer,
            connection_pool,
        }
    }

    pub fn start(self) {
        let executor = self.executor.clone();
        executor.spawn_detached(async move {
            loop {
                if let Err(error) = self.sweep_and_broadcast().await {
                    log::error!("expiring custom statuses: {error}");
                }
                self.executor.sleep(STATUS_EXPIRY_INTERVAL).await;
            }
        });
    }

    pub async fn sweep(&self) -> Result<Vec<UserId>> {
        let now = time::OffsetDateTime::now_utc();
        UserStatusStore::new(self.db.clone())
            .delete_expired_custom_statuses(time::PrimitiveDateTime::new(now.date(), now.time()))
            .await
    }

    async fn sweep_and_broadcast(&self) -> Result<()> {
        let user_ids = self.sweep().await?;
        let connection_pool = self.connection_pool.lock();
        for user_id in user_ids {
            let update = proto::UpdateUserStatus {
                user_id: user_id.to_proto(),
                status: None,
            };
            for connection_id in connection_pool.connection_ids() {
                self.peer.send(connection_id, update.clone())?;
            }
        }
        Ok(())
    }
}
