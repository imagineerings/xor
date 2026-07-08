use serde_json::json;

use crate::{
    ASSET_QUERY_INVALID_CURSOR_CODE, ASSET_QUERY_INVALID_HASH_CODE,
    ASSET_QUERY_INVALID_METADATA_FILTER_CODE, ASSET_QUERY_INVALID_OWNER_CODE,
    ASSET_QUERY_INVALID_SORT_CODE, ASSET_QUERY_INVALID_TAG_CODE, ComfyAssetCursor,
    ComfyAssetListQuery, ComfyAssetMetadataFilter, ComfyAssetMetadataNamespace, ComfyAssetOrder,
    ComfyAssetOwnerScope, ComfyAssetReferenceId, ComfyAssetSort, ComfyAssetValidatedHash,
    normalize_asset_tag,
};

#[test]
fn hash_validation_accepts_supported_canonical_hashes() {
    let hash =
        ComfyAssetValidatedHash::parse(&format!("sha256:{}", "A".repeat(64))).expect("valid hash");

    assert_eq!(hash.as_str(), &format!("sha256:{}", "a".repeat(64)));
}

#[test]
fn hash_validation_rejects_missing_algorithm_and_bad_digest() {
    let missing_algorithm =
        ComfyAssetValidatedHash::parse(&"a".repeat(64)).expect_err("missing algorithm should fail");
    let bad_digest =
        ComfyAssetValidatedHash::parse("sha256:not-hex").expect_err("bad digest should fail");

    assert_eq!(missing_algorithm.code, ASSET_QUERY_INVALID_HASH_CODE);
    assert_eq!(bad_digest.code, ASSET_QUERY_INVALID_HASH_CODE);
}

#[test]
fn cursor_round_trip_preserves_sort_value_and_reference_id() {
    let cursor = ComfyAssetCursor::new(
        "2026-07-08T17:51:03Z/name",
        ComfyAssetReferenceId::new("asset-reference-1"),
    );

    let decoded = ComfyAssetCursor::decode(&cursor.encode()).expect("cursor should decode");

    assert_eq!(decoded, cursor);
}

#[test]
fn cursor_validation_rejects_non_native_cursor_shape() {
    let error =
        ComfyAssetCursor::decode("comfy:cursor").expect_err("foreign cursor shape should fail");

    assert_eq!(error.code, ASSET_QUERY_INVALID_CURSOR_CODE);
}

#[test]
fn metadata_filters_parse_namespaces_and_json_values() {
    let user_filter = ComfyAssetMetadataFilter::parse("prompt=\"castle\"").expect("user filter");
    let system_filter =
        ComfyAssetMetadataFilter::parse("system.dimensions={\"w\":1024}").expect("system filter");

    assert_eq!(user_filter.namespace, ComfyAssetMetadataNamespace::User);
    assert_eq!(user_filter.key, "prompt");
    assert_eq!(user_filter.value, json!("castle"));
    assert_eq!(system_filter.namespace, ComfyAssetMetadataNamespace::System);
    assert_eq!(system_filter.key, "dimensions");
    assert_eq!(system_filter.value, json!({ "w": 1024 }));
}

#[test]
fn metadata_filter_validation_rejects_unknown_namespace() {
    let error = ComfyAssetMetadataFilter::parse("comfy.prompt=castle")
        .expect_err("unknown namespace should fail");

    assert_eq!(error.code, ASSET_QUERY_INVALID_METADATA_FILTER_CODE);
}

#[test]
fn tag_normalization_is_stable_and_rejects_unsafe_tags() {
    assert_eq!(
        normalize_asset_tag("  Generated Output  ").expect("tag"),
        "generated-output"
    );

    let error = normalize_asset_tag("bad tag!").expect_err("unsupported tag character should fail");

    assert_eq!(error.code, ASSET_QUERY_INVALID_TAG_CODE);
}

#[test]
fn sort_order_and_pagination_are_validated_into_native_types() {
    let owner_scope =
        ComfyAssetOwnerScope::resolve(Some("local"), None, false).expect("owner scope");
    let cursor = ComfyAssetCursor::new("42", ComfyAssetReferenceId::new("asset-reference-2"));
    let query = ComfyAssetListQuery::new(owner_scope)
        .with_include_tag("Model Files")
        .expect("include tag")
        .with_exclude_tag("hidden")
        .expect("exclude tag")
        .with_name_contains("castle")
        .with_metadata_filter("system.width=1024")
        .expect("metadata")
        .with_hash(&format!("blake3:{}", "b".repeat(64)))
        .expect("hash")
        .with_pagination(Some(1000), Some(20), Some(&cursor.encode()))
        .expect("pagination")
        .with_sort("updated-at")
        .expect("sort")
        .with_order("asc")
        .expect("order");

    assert_eq!(query.include_tags, vec!["model-files"]);
    assert_eq!(query.exclude_tags, vec!["hidden"]);
    assert_eq!(query.name_contains.as_deref(), Some("castle"));
    assert_eq!(query.metadata_filters.len(), 1);
    assert_eq!(query.pagination.limit, 500);
    assert_eq!(query.pagination.offset, 20);
    assert_eq!(query.pagination.cursor, Some(cursor));
    assert_eq!(query.sort, ComfyAssetSort::UpdatedAt);
    assert_eq!(query.order, ComfyAssetOrder::Ascending);
}

#[test]
fn sort_validation_rejects_unknown_fields() {
    let error = ComfyAssetSort::parse("comfy_sort").expect_err("unknown sort should fail");

    assert_eq!(error.code, ASSET_QUERY_INVALID_SORT_CODE);
}

#[test]
fn owner_scope_uses_authenticated_user_in_multi_user_mode() {
    let scope = ComfyAssetOwnerScope::resolve(Some("other"), Some("user-a"), true)
        .expect("authenticated owner");

    assert_eq!(scope.owner_id.as_str(), "user-a");
}

#[test]
fn owner_scope_rejects_internal_system_users() {
    let error = ComfyAssetOwnerScope::resolve(Some("system"), None, false)
        .expect_err("system owner should fail");

    assert_eq!(error.code, ASSET_QUERY_INVALID_OWNER_CODE);
}
