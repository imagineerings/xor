use serde_json::json;

use crate::{
    ASSET_API_HASH_NOT_FOUND_CODE, ComfyAssetApi, ComfyAssetListQuery, ComfyAssetOwnerId,
    ComfyAssetOwnerScope, ComfyAssetReferenceRequest, ComfyAssetUpdateRequest,
    ComfyAssetUploadRequest, ComfyAssetValidatedHash,
};

fn sha256(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

fn owner_scope(owner: &ComfyAssetOwnerId) -> ComfyAssetOwnerScope {
    ComfyAssetOwnerScope {
        owner_id: owner.clone(),
    }
}

#[test]
fn asset_api_uploads_and_lists_owner_scoped_assets() {
    let owner = ComfyAssetOwnerId::new("user-a");
    let other_owner = ComfyAssetOwnerId::new("user-b");
    let mut api = ComfyAssetApi::default();

    api.upload(
        owner.clone(),
        ComfyAssetUploadRequest::new("castle.png", 1024)
            .expect("upload")
            .with_known_hash(&sha256('a'))
            .expect("hash")
            .with_mime_type("image/png")
            .with_tag("Generated Output")
            .expect("tag")
            .with_user_metadata("prompt", json!("castle")),
    )
    .expect("first upload");
    api.upload(
        other_owner.clone(),
        ComfyAssetUploadRequest::new("forest.png", 1024)
            .expect("upload")
            .with_known_hash(&sha256('b'))
            .expect("hash")
            .with_tag("generated-output")
            .expect("tag"),
    )
    .expect("second upload");

    let query = ComfyAssetListQuery::new(owner_scope(&owner))
        .with_include_tag("generated output")
        .expect("tag")
        .with_metadata_filter("prompt=\"castle\"")
        .expect("metadata");
    let page = api.list(&query).expect("list");

    assert_eq!(page.total, 1);
    assert_eq!(page.items[0].reference.owner_id, owner);
    assert_eq!(page.items[0].reference.name, "castle.png");
    assert_eq!(
        page.items[0].content.mime_type.as_deref(),
        Some("image/png")
    );
    assert_eq!(
        api.list(&ComfyAssetListQuery::new(owner_scope(&other_owner)))
            .expect("other list")
            .total,
        1
    );
}

#[test]
fn asset_api_detail_update_and_delete_enforce_owner_access() {
    let owner = ComfyAssetOwnerId::new("user-a");
    let other_owner = ComfyAssetOwnerId::new("user-b");
    let mut api = ComfyAssetApi::default();
    let detail = api
        .upload(
            owner.clone(),
            ComfyAssetUploadRequest::new("castle.png", 128)
                .expect("upload")
                .with_known_hash(&sha256('c'))
                .expect("hash"),
        )
        .expect("upload");

    assert!(
        api.detail(&other_owner, &detail.reference.id)
            .expect("detail")
            .is_none()
    );
    assert!(
        api.update(
            &other_owner,
            &detail.reference.id,
            ComfyAssetUpdateRequest::default().with_name("stolen.png"),
        )
        .expect("update")
        .is_none()
    );

    let updated = api
        .update(
            &owner,
            &detail.reference.id,
            ComfyAssetUpdateRequest::default()
                .with_name("renamed.png")
                .with_tags(["favorite", "Generated Output"])
                .expect("tags"),
        )
        .expect("update")
        .expect("owner update");
    assert_eq!(updated.reference.name, "renamed.png");
    assert!(updated.reference.tags.contains("generated-output"));

    assert!(
        !api.delete(&other_owner, &detail.reference.id)
            .expect("wrong owner delete")
    );
    assert!(
        api.delete(&owner, &detail.reference.id)
            .expect("owner delete")
    );
    assert!(
        api.detail(&owner, &detail.reference.id)
            .expect("detail after delete")
            .is_none()
    );
}

#[test]
fn asset_api_create_from_hash_reuses_existing_content() {
    let owner = ComfyAssetOwnerId::new("user-a");
    let hash = ComfyAssetValidatedHash::parse(&sha256('d')).expect("hash");
    let mut api = ComfyAssetApi::default();
    let uploaded = api
        .upload(
            owner.clone(),
            ComfyAssetUploadRequest::new("source.png", 512)
                .expect("upload")
                .with_known_hash(hash.as_str())
                .expect("hash"),
        )
        .expect("upload");

    assert!(api.hash_exists(&hash));
    let linked = api
        .create_from_hash(
            owner,
            &hash,
            ComfyAssetReferenceRequest::new("linked.png", 512).with_tag("linked"),
        )
        .expect("create from hash");

    assert_eq!(uploaded.content.id, linked.content.id);
    assert_ne!(uploaded.reference.id, linked.reference.id);
    assert_eq!(api.repository().content_len(), 1);
    assert_eq!(api.repository().reference_len(), 2);
}

#[test]
fn asset_api_create_from_hash_requires_existing_content() {
    let owner = ComfyAssetOwnerId::new("user-a");
    let hash = ComfyAssetValidatedHash::parse(&sha256('e')).expect("hash");
    let mut api = ComfyAssetApi::default();

    let error = api
        .create_from_hash(
            owner,
            &hash,
            ComfyAssetReferenceRequest::new("missing.png", 12),
        )
        .expect_err("missing hash should fail");

    assert_eq!(error.code, ASSET_API_HASH_NOT_FOUND_CODE);
}

#[test]
fn asset_api_paginates_with_native_cursors() {
    let owner = ComfyAssetOwnerId::new("user-a");
    let mut api = ComfyAssetApi::default();
    for name in ["alpha.png", "beta.png", "gamma.png"] {
        api.upload(
            owner.clone(),
            ComfyAssetUploadRequest::new(name, 10).expect("upload"),
        )
        .expect("upload");
    }

    let first_query = ComfyAssetListQuery::new(owner_scope(&owner))
        .with_sort("name")
        .expect("sort")
        .with_order("asc")
        .expect("order")
        .with_pagination(Some(2), None, None)
        .expect("pagination");
    let first_page = api.list(&first_query).expect("first page");
    let second_query = ComfyAssetListQuery::new(owner_scope(&owner))
        .with_sort("name")
        .expect("sort")
        .with_order("asc")
        .expect("order")
        .with_pagination(Some(2), None, first_page.next_cursor.as_deref())
        .expect("pagination");
    let second_page = api.list(&second_query).expect("second page");

    assert_eq!(
        first_page
            .items
            .iter()
            .map(|item| item.reference.name.as_str())
            .collect::<Vec<_>>(),
        vec!["alpha.png", "beta.png"]
    );
    assert_eq!(second_page.items[0].reference.name, "gamma.png");
    assert!(second_page.next_cursor.is_none());
}
