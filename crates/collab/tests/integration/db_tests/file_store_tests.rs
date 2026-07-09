use super::new_test_user;
use crate::test_both_dbs;
use collab::{
    Error,
    db::{
        Database, MessageId, channel_file,
        file_store::{FileStore, FileStoreConfig, FileStoreError, NewFileUpload},
        queries::channel_messages::NewChannelMessage,
    },
};
use sea_orm::{ColumnTrait as _, EntityTrait as _, QueryFilter as _};
use std::sync::Arc;

test_both_dbs!(
    test_file_store_validation,
    test_file_store_validation_postgres,
    test_file_store_validation_sqlite
);

async fn test_file_store_validation(db: &Arc<Database>) {
    let user_id = new_test_user(db).await;
    let channel_id = db.create_root_channel("files", user_id).await.unwrap();
    let file_store = test_file_store(db, 8, vec!["text/plain"]);

    let too_large = expect_file_store_error(
        file_store
            .generate_upload_url(new_file_upload(
                channel_id,
                user_id,
                "note.txt",
                9,
                "text/plain",
            ))
            .await,
    );
    assert_eq!(too_large, FileStoreError::FileTooLarge { max_file_size: 8 });

    let unsupported_type = expect_file_store_error(
        file_store
            .generate_upload_url(new_file_upload(
                channel_id,
                user_id,
                "note.png",
                8,
                "image/png",
            ))
            .await,
    );
    assert_eq!(unsupported_type, FileStoreError::UnsupportedFileType);

    let empty_filename = expect_file_store_error(
        file_store
            .generate_upload_url(new_file_upload(channel_id, user_id, "  ", 8, "text/plain"))
            .await,
    );
    assert_eq!(empty_filename, FileStoreError::EmptyFilename);

    let rows = db
        .transaction(|tx| async move {
            channel_file::Entity::find()
                .filter(channel_file::Column::ChannelId.eq(channel_id))
                .all(&*tx)
                .await
                .map_err(Into::into)
        })
        .await
        .unwrap();
    assert!(rows.is_empty());
}

test_both_dbs!(
    test_file_store_metadata_lifecycle,
    test_file_store_metadata_lifecycle_postgres,
    test_file_store_metadata_lifecycle_sqlite
);

async fn test_file_store_metadata_lifecycle(db: &Arc<Database>) {
    let user_id = new_test_user(db).await;
    let channel_id = db.create_root_channel("files", user_id).await.unwrap();
    let file_store = test_file_store(db, 1024, vec!["text/plain"]);

    let upload = file_store
        .generate_upload_url(NewFileUpload {
            image_width: Some(640),
            image_height: Some(480),
            duration_ms: Some(1200),
            ..new_file_upload(channel_id, user_id, "note.txt", 12, "text/plain")
        })
        .await
        .unwrap();
    assert!(upload.url.contains("file-store.test"));
    assert!(upload.headers.is_empty());

    let confirmed = file_store
        .confirm_upload(upload.file_id, user_id)
        .await
        .unwrap();
    assert_eq!(confirmed.id, upload.file_id);
    assert_eq!(confirmed.filename, "note.txt");
    assert_eq!(confirmed.file_size, 12);
    assert_eq!(confirmed.mime_type, "text/plain");
    assert_eq!(confirmed.uploader_id, user_id);
    assert_eq!(confirmed.image_width, Some(640));
    assert_eq!(confirmed.image_height, Some(480));
    assert_eq!(confirmed.duration_ms, Some(1200));
    assert!(confirmed.uploaded_at.is_some());
    assert!(confirmed.url.contains("file-store.test"));

    let metadata = file_store.get_file_metadata(upload.file_id).await.unwrap();
    assert_eq!(metadata, confirmed);

    let message = db
        .create_channel_message(NewChannelMessage {
            channel_id,
            sender_id: user_id,
            body: "with attachment".to_string(),
            nonce: 1.into(),
            mentions: Vec::new(),
            reply_to_message_id: None,
            scheduled_at: None,
        })
        .await
        .unwrap();
    let attachments = file_store
        .attach_files_to_message(
            channel_id,
            MessageId::from_proto(message.id),
            user_id,
            vec![upload.file_id],
        )
        .await
        .unwrap();
    assert_eq!(attachments, vec![confirmed]);

    let deleted = file_store
        .delete_message_files(channel_id, MessageId::from_proto(message.id))
        .await
        .unwrap();
    assert_eq!(deleted, 1);
    assert!(
        db.transaction(|tx| async move {
            channel_file::Entity::find_by_id(upload.file_id)
                .one(&*tx)
                .await
                .map_err(Into::into)
        })
        .await
        .unwrap()
        .is_none()
    );
}

fn test_file_store(
    db: &Arc<Database>,
    max_file_size: u64,
    allowed_mime_types: Vec<&str>,
) -> FileStore {
    FileStore::new_for_tests(
        db.clone(),
        FileStoreConfig::new(
            Some("test-bucket".to_string()),
            max_file_size,
            allowed_mime_types
                .into_iter()
                .map(ToOwned::to_owned)
                .collect(),
        ),
        "http://file-store.test",
    )
}

fn new_file_upload(
    channel_id: collab::db::ChannelId,
    uploader_id: collab::db::UserId,
    filename: &str,
    file_size: u64,
    mime_type: &str,
) -> NewFileUpload {
    NewFileUpload {
        channel_id,
        filename: filename.to_string(),
        file_size,
        mime_type: mime_type.to_string(),
        uploader_id,
        image_width: None,
        image_height: None,
        duration_ms: None,
    }
}

fn file_store_error(error: &Error) -> &FileStoreError {
    let Error::Internal(error) = error else {
        panic!("expected internal file store error, got {error}");
    };
    error
        .downcast_ref::<FileStoreError>()
        .expect("expected file store error")
}

fn expect_file_store_error<T>(result: Result<T, Error>) -> FileStoreError {
    match result {
        Ok(_) => panic!("expected file store error"),
        Err(error) => file_store_error(&error).clone(),
    }
}
