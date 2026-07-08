use serde_json::json;

use crate::{
    ASSET_QUERY_INVALID_CURSOR_CODE, ASSET_QUERY_INVALID_HASH_CODE,
    ASSET_QUERY_INVALID_METADATA_FILTER_CODE, ASSET_QUERY_INVALID_OWNER_CODE,
    ASSET_QUERY_INVALID_SORT_CODE, ASSET_QUERY_INVALID_TAG_CODE, SimAssetCursor, SimAssetListQuery,
    SimAssetMetadataFilter, SimAssetMetadataNamespace, SimAssetOrder, SimAssetOwnerScope,
    SimAssetReferenceId, SimAssetSort, SimAssetValidatedHash, normalize_asset_tag,
};

#[test]
fn hash_validation_accepts_supported_canonical_hashes() {
    let hash =
        SimAssetValidatedHash::parse(&format!("sha256:{}", "A".repeat(64))).expect("valid hash");

    assert_eq!(hash.as_str(), &format!("sha256:{}", "a".repeat(64)));
}

#[test]
fn hash_validation_rejects_missing_algorithm_and_bad_digest() {
    let missing_algorithm =
        SimAssetValidatedHash::parse(&"a".repeat(64)).expect_err("missing algorithm should fail");
    let bad_digest =
        SimAssetValidatedHash::parse("sha256:not-hex").expect_err("bad digest should fail");

    assert_eq!(missing_algorithm.code, ASSET_QUERY_INVALID_HASH_CODE);
    assert_eq!(bad_digest.code, ASSET_QUERY_INVALID_HASH_CODE);
}

#[test]
fn cursor_round_trip_preserves_sort_value_and_reference_id() {
    let cursor = SimAssetCursor::new(
        "2026-07-08T17:51:03Z/name",
        SimAssetReferenceId::new("asset-reference-1"),
    );

    let decoded = SimAssetCursor::decode(&cursor.encode()).expect("cursor should decode");

    assert_eq!(decoded, cursor);
}

#[test]
fn cursor_validation_rejects_non_native_cursor_shape() {
    let error =
        SimAssetCursor::decode("comfy:cursor").expect_err("foreign cursor shape should fail");

    assert_eq!(error.code, ASSET_QUERY_INVALID_CURSOR_CODE);
}

#[test]
fn metadata_filters_parse_namespaces_and_json_values() {
    let user_filter = SimAssetMetadataFilter::parse("prompt=\"castle\"").expect("user filter");
    let system_filter =
        SimAssetMetadataFilter::parse("system.dimensions={\"w\":1024}").expect("system filter");

    assert_eq!(user_filter.namespace, SimAssetMetadataNamespace::User);
    assert_eq!(user_filter.key, "prompt");
    assert_eq!(user_filter.value, json!("castle"));
    assert_eq!(system_filter.namespace, SimAssetMetadataNamespace::System);
    assert_eq!(system_filter.key, "dimensions");
    assert_eq!(system_filter.value, json!({ "w": 1024 }));
}

#[test]
fn metadata_filter_validation_rejects_unknown_namespace() {
    let error = SimAssetMetadataFilter::parse("comfy.prompt=castle")
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
    let owner_scope = SimAssetOwnerScope::resolve(Some("local"), None, false).expect("owner scope");
    let cursor = SimAssetCursor::new("42", SimAssetReferenceId::new("asset-reference-2"));
    let query = SimAssetListQuery::new(owner_scope)
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
    assert_eq!(query.sort, SimAssetSort::UpdatedAt);
    assert_eq!(query.order, SimAssetOrder::Ascending);
}

#[test]
fn sort_validation_rejects_unknown_fields() {
    let error = SimAssetSort::parse("comfy_sort").expect_err("unknown sort should fail");

    assert_eq!(error.code, ASSET_QUERY_INVALID_SORT_CODE);
}

#[test]
fn owner_scope_uses_authenticated_user_in_multi_user_mode() {
    let scope = SimAssetOwnerScope::resolve(Some("other"), Some("user-a"), true)
        .expect("authenticated owner");

    assert_eq!(scope.owner_id.as_str(), "user-a");
}

#[test]
fn owner_scope_rejects_internal_system_users() {
    let error = SimAssetOwnerScope::resolve(Some("system"), None, false)
        .expect_err("system owner should fail");

    assert_eq!(error.code, ASSET_QUERY_INVALID_OWNER_CODE);
}
