use std::path::Path;

use serde_json::json;

use crate::{
    ComfyAssetApi, ComfyAssetListQuery, ComfyAssetOrder, ComfyAssetOwnerId, ComfyAssetOwnerScope,
    ComfyAssetTagListQuery, ComfyAssetTagService, ComfyAssetUploadRequest, ComfyUserDataStore,
    USER_DATA_FORBIDDEN_CODE, USER_DATA_NOT_FOUND_CODE,
};

fn upload_asset(
    api: &mut ComfyAssetApi,
    owner: &ComfyAssetOwnerId,
    name: &str,
) -> crate::ComfyAssetReferenceId {
    api.upload(
        owner.clone(),
        ComfyAssetUploadRequest::new(name, 10).expect("upload"),
    )
    .expect("upload")
    .reference
    .id
}

#[test]
fn tag_service_adds_removes_lists_and_refines_tags_by_owner() {
    let owner = ComfyAssetOwnerId::new("user-a");
    let other_owner = ComfyAssetOwnerId::new("user-b");
    let mut api = ComfyAssetApi::default();
    let first = upload_asset(&mut api, &owner, "first.png");
    let second = upload_asset(&mut api, &owner, "second.png");
    let other = upload_asset(&mut api, &other_owner, "other.png");

    {
        let mut tags = ComfyAssetTagService::new(&mut api);
        let report = tags
            .add_tag(&owner, &first, "Generated Output")
            .expect("add")
            .expect("visible");
        assert!(report.added);
        assert_eq!(report.total_tags, 1);
        assert!(
            tags.add_tag(&owner, &first, "generated-output")
                .expect("add again")
                .expect("visible")
                .already_present
        );
        tags.add_tag(&owner, &second, "generated-output")
            .expect("add second")
            .expect("visible");
        tags.add_tag(&other_owner, &other, "generated-output")
            .expect("add other")
            .expect("visible");
        assert!(
            tags.remove_tag(&owner, &first, "missing")
                .expect("remove missing")
                .expect("visible")
                .missing
        );
    }

    let tags = ComfyAssetTagService::new(&mut api);
    let listed = tags
        .list_tags(
            &ComfyAssetTagListQuery::new(owner.clone())
                .with_prefix("generated")
                .expect("prefix")
                .with_order(ComfyAssetOrder::Ascending),
        )
        .expect("list");
    let refined = tags
        .refine_tags(&ComfyAssetListQuery::new(ComfyAssetOwnerScope {
            owner_id: owner,
        }))
        .expect("refine");

    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].tag, "generated-output");
    assert_eq!(listed[0].count, 2);
    assert_eq!(refined[0].count, 2);
}

#[test]
fn user_data_store_confines_paths_and_keeps_owners_separate() {
    let owner = ComfyAssetOwnerId::new("user-a");
    let other_owner = ComfyAssetOwnerId::new("user-b");
    let mut store = ComfyUserDataStore::default();

    store
        .write_file(owner.clone(), Path::new("workflows/a.json"), b"{}".to_vec())
        .expect("write");
    store
        .write_file(
            other_owner.clone(),
            Path::new("workflows/a.json"),
            b"other".to_vec(),
        )
        .expect("write other");

    assert_eq!(
        store
            .read_file(&owner, Path::new("workflows/a.json"))
            .expect("read"),
        b"{}".to_vec()
    );
    assert_eq!(
        store
            .list_files(&owner, Path::new("workflows"), true)
            .expect("list")
            .len(),
        1
    );
    assert_eq!(
        store
            .read_file(&other_owner, Path::new("workflows/a.json"))
            .expect("read other"),
        b"other".to_vec()
    );

    let error = store
        .write_file(owner, Path::new("../escape.json"), Vec::new())
        .expect_err("escape should fail");
    assert_eq!(error.code, USER_DATA_FORBIDDEN_CODE);
}

#[test]
fn user_data_store_moves_deletes_splits_paths_and_persists_settings() {
    let owner = ComfyAssetOwnerId::new("user-a");
    let mut store = ComfyUserDataStore::default();
    store
        .write_file(
            owner.clone(),
            Path::new("settings/theme.json"),
            b"dark".to_vec(),
        )
        .expect("write");

    let moved = store
        .move_file(
            &owner,
            Path::new("settings/theme.json"),
            Path::new("settings/theme-v2.json"),
        )
        .expect("move");
    let parts = ComfyUserDataStore::path_parts(Path::new("settings/theme-v2.json")).expect("parts");

    assert_eq!(moved.file_name, "theme-v2.json");
    assert_eq!(parts.directory, Path::new("settings"));
    assert_eq!(parts.file_name, "theme-v2.json");
    assert_eq!(
        store
            .read_file(&owner, Path::new("settings/theme-v2.json"))
            .expect("read moved"),
        b"dark".to_vec()
    );
    assert!(
        store
            .delete_file(&owner, Path::new("settings/theme-v2.json"))
            .expect("delete")
    );
    assert_eq!(
        store
            .read_file(&owner, Path::new("settings/theme-v2.json"))
            .expect_err("deleted")
            .code,
        USER_DATA_NOT_FOUND_CODE
    );

    store.write_settings(owner.clone(), json!({ "theme": "dark" }));
    assert_eq!(store.read_settings(&owner), json!({ "theme": "dark" }));
}
