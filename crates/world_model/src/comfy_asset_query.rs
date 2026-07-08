use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ComfyAssetOwnerId, ComfyAssetReferenceId};

pub const ASSET_QUERY_INVALID_CURSOR_CODE: &str = "world_model.comfy_assets.invalid_cursor";
pub const ASSET_QUERY_INVALID_HASH_CODE: &str = "world_model.comfy_assets.invalid_hash";
pub const ASSET_QUERY_INVALID_METADATA_FILTER_CODE: &str =
    "world_model.comfy_assets.invalid_metadata_filter";
pub const ASSET_QUERY_INVALID_OWNER_CODE: &str = "world_model.comfy_assets.invalid_owner";
pub const ASSET_QUERY_INVALID_SORT_CODE: &str = "world_model.comfy_assets.invalid_sort";
pub const ASSET_QUERY_INVALID_TAG_CODE: &str = "world_model.comfy_assets.invalid_tag";

const DEFAULT_LIMIT: usize = 50;
const MAX_LIMIT: usize = 500;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyAssetQueryDiagnostic {
    pub code: String,
    pub field: String,
    pub message: String,
}

impl ComfyAssetQueryDiagnostic {
    fn new(code: &str, field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            field: field.into(),
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyAssetValidatedHash(String);

impl ComfyAssetValidatedHash {
    pub fn parse(value: &str) -> Result<Self, ComfyAssetQueryDiagnostic> {
        let value = value.trim();
        if value.is_empty() {
            return Err(invalid_hash("hash", "asset hash cannot be empty"));
        }

        let Some((algorithm, digest)) = value.split_once(':') else {
            return Err(invalid_hash(
                "hash",
                "asset hash must include a hash algorithm prefix",
            ));
        };
        if !matches!(algorithm, "sha256" | "sha512" | "blake3") {
            return Err(invalid_hash(
                "hash",
                "asset hash algorithm must be sha256, sha512, or blake3",
            ));
        }

        let expected_len = match algorithm {
            "sha256" | "blake3" => 64,
            "sha512" => 128,
            _ => unreachable!("algorithm is validated above"),
        };
        if digest.len() != expected_len || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(invalid_hash(
                "hash",
                format!("{algorithm} asset hash must be {expected_len} hexadecimal characters"),
            ));
        }

        Ok(Self(format!(
            "{}:{}",
            algorithm,
            digest.to_ascii_lowercase()
        )))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComfyAssetSort {
    CreatedAt,
    UpdatedAt,
    Name,
    SizeBytes,
    Hash,
}

impl ComfyAssetSort {
    pub fn parse(value: &str) -> Result<Self, ComfyAssetQueryDiagnostic> {
        match normalize_token(value).as_str() {
            "created_at" | "created" | "createdat" => Ok(Self::CreatedAt),
            "updated_at" | "updated" | "updatedat" => Ok(Self::UpdatedAt),
            "name" => Ok(Self::Name),
            "size" | "size_bytes" | "sizebytes" => Ok(Self::SizeBytes),
            "hash" => Ok(Self::Hash),
            _ => Err(ComfyAssetQueryDiagnostic::new(
                ASSET_QUERY_INVALID_SORT_CODE,
                "sort",
                format!("unsupported asset sort `{value}`"),
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComfyAssetOrder {
    Ascending,
    Descending,
}

impl ComfyAssetOrder {
    pub fn parse(value: &str) -> Result<Self, ComfyAssetQueryDiagnostic> {
        match normalize_token(value).as_str() {
            "asc" | "ascending" => Ok(Self::Ascending),
            "desc" | "descending" => Ok(Self::Descending),
            _ => Err(ComfyAssetQueryDiagnostic::new(
                ASSET_QUERY_INVALID_SORT_CODE,
                "order",
                format!("unsupported asset order `{value}`"),
            )),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyAssetCursor {
    pub sort_value: String,
    pub reference_id: ComfyAssetReferenceId,
}

impl ComfyAssetCursor {
    pub fn new(sort_value: impl Into<String>, reference_id: ComfyAssetReferenceId) -> Self {
        Self {
            sort_value: sort_value.into(),
            reference_id,
        }
    }

    pub fn encode(&self) -> String {
        format!(
            "sim-asset-v1:{}:{}",
            encode_component(&self.sort_value),
            encode_component(self.reference_id.as_str())
        )
    }

    pub fn decode(value: &str) -> Result<Self, ComfyAssetQueryDiagnostic> {
        let Some(rest) = value.strip_prefix("sim-asset-v1:") else {
            return Err(invalid_cursor("cursor must use the sim-asset-v1 format"));
        };
        let mut parts = rest.split(':');
        let Some(sort_value) = parts.next() else {
            return Err(invalid_cursor("cursor is missing the sort value"));
        };
        let Some(reference_id) = parts.next() else {
            return Err(invalid_cursor("cursor is missing the reference id"));
        };
        if parts.next().is_some() {
            return Err(invalid_cursor("cursor has too many fields"));
        }

        Ok(Self {
            sort_value: decode_component(sort_value)?,
            reference_id: ComfyAssetReferenceId::new(decode_component(reference_id)?),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComfyAssetMetadataNamespace {
    User,
    System,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComfyAssetMetadataOperator {
    Equals,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComfyAssetMetadataFilter {
    pub namespace: ComfyAssetMetadataNamespace,
    pub key: String,
    pub operator: ComfyAssetMetadataOperator,
    pub value: Value,
}

impl ComfyAssetMetadataFilter {
    pub fn parse(value: &str) -> Result<Self, ComfyAssetQueryDiagnostic> {
        let Some((field, raw_value)) = value.split_once('=') else {
            return Err(invalid_metadata_filter(
                "metadata filters must use `field=value`",
            ));
        };
        let field = field.trim();
        if field.is_empty() {
            return Err(invalid_metadata_filter(
                "metadata filter field cannot be empty",
            ));
        }

        let (namespace, key) = match field.split_once('.') {
            Some(("user", key)) => (ComfyAssetMetadataNamespace::User, key),
            Some(("system", key)) => (ComfyAssetMetadataNamespace::System, key),
            Some((namespace, _)) => {
                return Err(invalid_metadata_filter(format!(
                    "unsupported metadata namespace `{namespace}`"
                )));
            }
            None => (ComfyAssetMetadataNamespace::User, field),
        };
        let key = key.trim();
        if key.is_empty() || key.split('.').any(str::is_empty) {
            return Err(invalid_metadata_filter("metadata filter key is invalid"));
        }

        Ok(Self {
            namespace,
            key: key.to_string(),
            operator: ComfyAssetMetadataOperator::Equals,
            value: parse_metadata_value(raw_value.trim()),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyAssetPagination {
    pub limit: usize,
    pub offset: usize,
    pub cursor: Option<ComfyAssetCursor>,
}

impl Default for ComfyAssetPagination {
    fn default() -> Self {
        Self {
            limit: DEFAULT_LIMIT,
            offset: 0,
            cursor: None,
        }
    }
}

impl ComfyAssetPagination {
    pub fn new(
        limit: Option<usize>,
        offset: Option<usize>,
        cursor: Option<&str>,
    ) -> Result<Self, ComfyAssetQueryDiagnostic> {
        let limit = limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
        let offset = offset.unwrap_or_default();
        let cursor = cursor.map(ComfyAssetCursor::decode).transpose()?;
        Ok(Self {
            limit,
            offset,
            cursor,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyAssetOwnerScope {
    pub owner_id: ComfyAssetOwnerId,
}

impl ComfyAssetOwnerScope {
    pub fn resolve(
        request_owner_id: Option<&str>,
        authenticated_user_id: Option<&str>,
        multi_user: bool,
    ) -> Result<Self, ComfyAssetQueryDiagnostic> {
        let owner = if multi_user {
            authenticated_user_id.ok_or_else(|| {
                ComfyAssetQueryDiagnostic::new(
                    ASSET_QUERY_INVALID_OWNER_CODE,
                    "owner",
                    "asset queries require an authenticated user in multi-user mode",
                )
            })?
        } else {
            request_owner_id
                .or(authenticated_user_id)
                .unwrap_or("local-user")
        };

        let owner = owner.trim();
        if owner.is_empty() || matches!(owner, "system" | "__system__" | "internal") {
            return Err(ComfyAssetQueryDiagnostic::new(
                ASSET_QUERY_INVALID_OWNER_CODE,
                "owner",
                "asset query owner is not allowed",
            ));
        }

        Ok(Self {
            owner_id: ComfyAssetOwnerId::new(owner),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComfyAssetListQuery {
    pub owner_scope: ComfyAssetOwnerScope,
    pub include_tags: Vec<String>,
    pub exclude_tags: Vec<String>,
    pub name_contains: Option<String>,
    pub metadata_filters: Vec<ComfyAssetMetadataFilter>,
    pub hash: Option<ComfyAssetValidatedHash>,
    pub pagination: ComfyAssetPagination,
    pub sort: ComfyAssetSort,
    pub order: ComfyAssetOrder,
}

impl ComfyAssetListQuery {
    pub fn new(owner_scope: ComfyAssetOwnerScope) -> Self {
        Self {
            owner_scope,
            include_tags: Vec::new(),
            exclude_tags: Vec::new(),
            name_contains: None,
            metadata_filters: Vec::new(),
            hash: None,
            pagination: ComfyAssetPagination::default(),
            sort: ComfyAssetSort::CreatedAt,
            order: ComfyAssetOrder::Descending,
        }
    }

    pub fn with_include_tag(mut self, tag: &str) -> Result<Self, ComfyAssetQueryDiagnostic> {
        self.include_tags.push(normalize_asset_tag(tag)?);
        self.include_tags.sort();
        self.include_tags.dedup();
        Ok(self)
    }

    pub fn with_exclude_tag(mut self, tag: &str) -> Result<Self, ComfyAssetQueryDiagnostic> {
        self.exclude_tags.push(normalize_asset_tag(tag)?);
        self.exclude_tags.sort();
        self.exclude_tags.dedup();
        Ok(self)
    }

    pub fn with_name_contains(mut self, name_contains: &str) -> Self {
        let name_contains = name_contains.trim();
        if !name_contains.is_empty() {
            self.name_contains = Some(name_contains.to_string());
        }
        self
    }

    pub fn with_metadata_filter(
        mut self,
        metadata_filter: &str,
    ) -> Result<Self, ComfyAssetQueryDiagnostic> {
        self.metadata_filters
            .push(ComfyAssetMetadataFilter::parse(metadata_filter)?);
        Ok(self)
    }

    pub fn with_hash(mut self, hash: &str) -> Result<Self, ComfyAssetQueryDiagnostic> {
        self.hash = Some(ComfyAssetValidatedHash::parse(hash)?);
        Ok(self)
    }

    pub fn with_pagination(
        mut self,
        limit: Option<usize>,
        offset: Option<usize>,
        cursor: Option<&str>,
    ) -> Result<Self, ComfyAssetQueryDiagnostic> {
        self.pagination = ComfyAssetPagination::new(limit, offset, cursor)?;
        Ok(self)
    }

    pub fn with_sort(mut self, sort: &str) -> Result<Self, ComfyAssetQueryDiagnostic> {
        self.sort = ComfyAssetSort::parse(sort)?;
        Ok(self)
    }

    pub fn with_order(mut self, order: &str) -> Result<Self, ComfyAssetQueryDiagnostic> {
        self.order = ComfyAssetOrder::parse(order)?;
        Ok(self)
    }
}

pub fn normalize_asset_tag(tag: &str) -> Result<String, ComfyAssetQueryDiagnostic> {
    let normalized = tag
        .trim()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .to_ascii_lowercase();
    if normalized.is_empty() {
        return Err(ComfyAssetQueryDiagnostic::new(
            ASSET_QUERY_INVALID_TAG_CODE,
            "tag",
            "asset tag cannot be empty",
        ));
    }
    if normalized
        .bytes()
        .any(|byte| !(byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'/')))
    {
        return Err(ComfyAssetQueryDiagnostic::new(
            ASSET_QUERY_INVALID_TAG_CODE,
            "tag",
            format!("asset tag `{tag}` contains unsupported characters"),
        ));
    }
    Ok(normalized)
}

fn invalid_hash(field: impl Into<String>, message: impl Into<String>) -> ComfyAssetQueryDiagnostic {
    ComfyAssetQueryDiagnostic::new(ASSET_QUERY_INVALID_HASH_CODE, field, message)
}

fn invalid_cursor(message: impl Into<String>) -> ComfyAssetQueryDiagnostic {
    ComfyAssetQueryDiagnostic::new(ASSET_QUERY_INVALID_CURSOR_CODE, "cursor", message)
}

fn invalid_metadata_filter(message: impl Into<String>) -> ComfyAssetQueryDiagnostic {
    ComfyAssetQueryDiagnostic::new(
        ASSET_QUERY_INVALID_METADATA_FILTER_CODE,
        "metadata",
        message,
    )
}

fn normalize_token(value: &str) -> String {
    value.trim().replace('-', "_").to_ascii_lowercase()
}

fn parse_metadata_value(value: &str) -> Value {
    serde_json::from_str(value).unwrap_or_else(|_| Value::String(value.to_string()))
}

fn encode_component(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn decode_component(value: &str) -> Result<String, ComfyAssetQueryDiagnostic> {
    let mut decoded = Vec::new();
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'%' {
            decoded.push(bytes[index]);
            index += 1;
            continue;
        }

        let high = bytes
            .get(index + 1)
            .and_then(|byte| hex_value(*byte))
            .ok_or_else(|| invalid_cursor("cursor percent escape is incomplete"))?;
        let low = bytes
            .get(index + 2)
            .and_then(|byte| hex_value(*byte))
            .ok_or_else(|| invalid_cursor("cursor percent escape is incomplete"))?;
        decoded.push((high << 4) | low);
        index += 3;
    }

    String::from_utf8(decoded).map_err(|_| invalid_cursor("cursor contains invalid utf-8"))
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
