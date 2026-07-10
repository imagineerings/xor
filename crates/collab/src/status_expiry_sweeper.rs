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
        for update in expired_status_updates(&user_ids) {
            for connection_id in connection_pool.connection_ids() {
                self.peer.send(connection_id, update.clone())?;
            }
        }
        Ok(())
    }
}

fn expired_status_updates(user_ids: &[UserId]) -> Vec<proto::UpdateUserStatus> {
    user_ids
        .iter()
        .map(|user_id| proto::UpdateUserStatus {
            user_id: user_id.to_proto(),
            status: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expired_status_updates_broadcast_one_clear_per_user() {
        let updates = expired_status_updates(&[UserId::from_proto(4), UserId::from_proto(8)]);
        assert_eq!(updates.len(), 2);
        assert_eq!(updates[0].user_id, 4);
        assert!(updates.iter().all(|update| update.status.is_none()));
    }

    #[test]
    fn expired_status_updates_are_empty_without_expired_users() {
        assert!(expired_status_updates(&[]).is_empty());
    }
}
