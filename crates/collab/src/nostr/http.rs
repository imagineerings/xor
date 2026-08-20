use std::{collections::BTreeMap, error::Error, fmt, str::FromStr, sync::Arc};

use async_trait::async_trait;
use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, Uri, header},
    response::{IntoResponse, Response},
    routing::get,
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use collaboration_domain::{
    AuthenticatedPrincipal, AuthenticatedPrincipalKind, NostrAuthenticationMethod, NostrPublicKey,
    TenantContext, TrustedTenantRouteSource,
};
use nostr_compat::{EventId, PublicKey, TimestampPolicy, verify_signed_event};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use url::Url;

use crate::nostr::{
    MAX_NOSTR_FRAME_BYTES,
    event_ingest::parse_wire_signed_event,
    subscriptions::{
        MAX_ACTIVE_SUBSCRIPTIONS, MAX_FILTER_LIMIT, MAX_FILTERS_PER_REQUEST,
        MAX_SUBSCRIPTION_ID_BYTES,
    },
};

const NIP98_KIND: u16 = 27_235;
const NIP98_FRESHNESS_SECONDS: u64 = 60;
const NIP98_REPLAY_SECONDS: u64 = 120;
const MAX_AUTHORIZATION_BYTES: usize = MAX_NOSTR_FRAME_BYTES * 2;
const MAX_NIP98_BODY_BYTES: usize = 1024 * 1024;
const MAX_NIP05_NAME_BYTES: usize = 64;
const MAX_METADATA_TEXT_BYTES: usize = 1_024;
const MAX_METADATA_URL_BYTES: usize = 2_048;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NostrHttpDirectoryError {
    Denied,
    Unavailable,
}

#[async_trait]
pub trait NostrHttpHostResolver: Send + Sync {
    async fn resolve_host(
        &self,
        canonical_host: &str,
    ) -> Result<TenantContext, NostrHttpDirectoryError>;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NostrTenantRelayMetadata {
    icon: Option<String>,
}

impl NostrTenantRelayMetadata {
    pub fn new(icon: Option<String>) -> Result<Self, NostrHttpConfigurationError> {
        if icon.as_ref().is_some_and(|value| {
            value.is_empty() || value.len() > MAX_METADATA_URL_BYTES || Url::parse(value).is_err()
        }) {
            return Err(NostrHttpConfigurationError::InvalidMetadata);
        }
        Ok(Self { icon })
    }

    pub fn icon(&self) -> Option<&str> {
        self.icon.as_deref()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Nip05Identity {
    public_key: PublicKey,
}

impl Nip05Identity {
    pub const fn new(public_key: PublicKey) -> Self {
        Self { public_key }
    }

    pub const fn public_key(self) -> PublicKey {
        self.public_key
    }
}

#[async_trait]
pub trait NostrHttpDirectory: Send + Sync {
    async fn relay_metadata(
        &self,
        tenant: &TenantContext,
    ) -> Result<NostrTenantRelayMetadata, NostrHttpDirectoryError>;

    async fn resolve_nip05(
        &self,
        tenant: &TenantContext,
        canonical_name: &str,
    ) -> Result<Option<Nip05Identity>, NostrHttpDirectoryError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NostrHttpConfiguration {
    name: String,
    description: String,
    software: String,
    version: String,
    relay_self: Option<PublicKey>,
    supported_nips: Vec<u32>,
    supported_extensions: Vec<String>,
    public_tls: bool,
}

impl NostrHttpConfiguration {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        software: impl Into<String>,
        version: impl Into<String>,
        relay_self: Option<PublicKey>,
        supported_nips: Vec<u32>,
        supported_extensions: Vec<String>,
        public_tls: bool,
    ) -> Result<Self, NostrHttpConfigurationError> {
        let name = name.into();
        let description = description.into();
        let software = software.into();
        let version = version.into();
        if !valid_metadata_text(&name)
            || !valid_metadata_text(&description)
            || !valid_metadata_text(&version)
            || software.len() > MAX_METADATA_URL_BYTES
            || Url::parse(&software).is_err()
            || supported_nips.len() > 128
            || supported_extensions.len() > 128
            || supported_extensions.iter().any(|value| {
                value.is_empty() || value.len() > 128 || value.chars().any(char::is_control)
            })
        {
            return Err(NostrHttpConfigurationError::InvalidMetadata);
        }
        let mut supported_nips = supported_nips;
        supported_nips.sort_unstable();
        supported_nips.dedup();
        let mut supported_extensions = supported_extensions;
        supported_extensions.sort();
        supported_extensions.dedup();
        Ok(Self {
            name,
            description,
            software,
            version,
            relay_self,
            supported_nips,
            supported_extensions,
            public_tls,
        })
    }

    pub fn buzz_compatible(
        version: impl Into<String>,
        relay_self: Option<PublicKey>,
        public_tls: bool,
    ) -> Result<Self, NostrHttpConfigurationError> {
        Self::new(
            "Sim Collaborative Relay",
            "Sim collaborative workspace relay",
            "https://github.com/simtropolis/sim",
            version,
            relay_self,
            vec![1, 2, 10, 11, 16, 17, 23, 25, 29, 33, 38, 42, 50, 56],
            vec!["nip-er".into()],
            public_tls,
        )
    }
}

fn valid_metadata_text(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_METADATA_TEXT_BYTES
        && !value.chars().any(char::is_control)
}

#[derive(Clone)]
pub struct NostrPublicHttpState {
    configuration: NostrHttpConfiguration,
    host_resolver: Arc<dyn NostrHttpHostResolver>,
    directory: Arc<dyn NostrHttpDirectory>,
}

impl NostrPublicHttpState {
    pub fn new(
        configuration: NostrHttpConfiguration,
        host_resolver: Arc<dyn NostrHttpHostResolver>,
        directory: Arc<dyn NostrHttpDirectory>,
    ) -> Self {
        Self {
            configuration,
            host_resolver,
            directory,
        }
    }
}

pub fn router(state: Arc<NostrPublicHttpState>) -> Router {
    Router::new()
        .route("/", get(nip11_root))
        .route("/info", get(nip11_info))
        .route("/.well-known/nostr.json", get(nip05))
        .with_state(state)
}

async fn nip11_root(
    State(state): State<Arc<NostrPublicHttpState>>,
    headers: HeaderMap,
) -> Response {
    let accepts_nostr = headers
        .get(header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(',')
                .any(|part| part.trim() == "application/nostr+json")
        });
    if !accepts_nostr {
        return StatusCode::NOT_FOUND.into_response();
    }
    nip11_response(&state, &headers).await
}

async fn nip11_info(
    State(state): State<Arc<NostrPublicHttpState>>,
    headers: HeaderMap,
) -> Response {
    nip11_response(&state, &headers).await
}

async fn nip11_response(state: &NostrPublicHttpState, headers: &HeaderMap) -> Response {
    let tenant = resolve_request_tenant(state.host_resolver.as_ref(), headers).await;
    let tenant_metadata = match tenant.as_ref() {
        Some(tenant) => state.directory.relay_metadata(tenant).await.ok(),
        None => None,
    };
    Json(Nip11Document::from_configuration(
        &state.configuration,
        tenant_metadata.as_ref(),
    ))
    .into_response()
}

#[derive(Deserialize)]
struct Nip05Query {
    name: Option<String>,
}

async fn nip05(
    State(state): State<Arc<NostrPublicHttpState>>,
    headers: HeaderMap,
    Query(query): Query<Nip05Query>,
) -> Response {
    let mut document = Nip05Document::default();
    if let (Some(name), Some(tenant)) = (
        query.name.and_then(|value| canonical_nip05_name(&value)),
        resolve_request_tenant(state.host_resolver.as_ref(), &headers).await,
    ) && let Ok(Some(identity)) = state.directory.resolve_nip05(&tenant, &name).await
    {
        let public_key = identity.public_key().to_hex();
        document.names.insert(name, public_key.clone());
        document.relays.insert(
            public_key,
            vec![format!(
                "{}://{}",
                if state.configuration.public_tls {
                    "wss"
                } else {
                    "ws"
                },
                tenant.route_reference()
            )],
        );
    }
    let mut response = Json(document).into_response();
    response.headers_mut().insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        header::HeaderValue::from_static("*"),
    );
    response
}

async fn resolve_request_tenant(
    resolver: &dyn NostrHttpHostResolver,
    headers: &HeaderMap,
) -> Option<TenantContext> {
    let host = canonical_host(headers)?;
    let tenant = resolver.resolve_host(&host).await.ok()?;
    matches!(
        tenant.route_source(),
        TrustedTenantRouteSource::DirectHost | TrustedTenantRouteSource::TrustedForwardedHost
    )
    .then_some(tenant)
}

fn canonical_host(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get(header::HOST)?.to_str().ok()?;
    if raw.is_empty() || raw.len() > 1_024 || raw.chars().any(char::is_control) {
        return None;
    }
    let authority = axum::http::uri::Authority::from_str(raw).ok()?;
    let host = authority.host().to_ascii_lowercase();
    if host.is_empty() {
        return None;
    }
    Some(match authority.port_u16() {
        Some(port) => format!("{host}:{port}"),
        None => host,
    })
}

fn canonical_nip05_name(value: &str) -> Option<String> {
    let value = value.trim().to_ascii_lowercase();
    (!value.is_empty()
        && value.len() <= MAX_NIP05_NAME_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte)))
    .then_some(value)
}

#[derive(Serialize)]
struct Nip11Document {
    name: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon: Option<String>,
    pubkey: Option<String>,
    contact: Option<String>,
    supported_nips: Vec<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    supported_extensions: Vec<String>,
    software: String,
    version: String,
    limitation: Nip11Limitation,
    #[serde(rename = "self", skip_serializing_if = "Option::is_none")]
    relay_self: Option<String>,
}

impl Nip11Document {
    fn from_configuration(
        configuration: &NostrHttpConfiguration,
        tenant_metadata: Option<&NostrTenantRelayMetadata>,
    ) -> Self {
        Self {
            name: configuration.name.clone(),
            description: configuration.description.clone(),
            icon: tenant_metadata.and_then(|metadata| metadata.icon().map(str::to_owned)),
            pubkey: None,
            contact: None,
            supported_nips: configuration.supported_nips.clone(),
            supported_extensions: configuration.supported_extensions.clone(),
            software: configuration.software.clone(),
            version: configuration.version.clone(),
            limitation: Nip11Limitation {
                max_message_length: MAX_NOSTR_FRAME_BYTES as u64,
                max_subscriptions: MAX_ACTIVE_SUBSCRIPTIONS as u32,
                max_filters: MAX_FILTERS_PER_REQUEST as u32,
                max_limit: MAX_FILTER_LIMIT,
                max_subid_length: MAX_SUBSCRIPTION_ID_BYTES as u32,
                auth_required: true,
                payment_required: false,
                restricted_writes: true,
            },
            relay_self: configuration.relay_self.map(PublicKey::to_hex),
        }
    }
}

#[derive(Serialize)]
struct Nip11Limitation {
    max_message_length: u64,
    max_subscriptions: u32,
    max_filters: u32,
    max_limit: u32,
    max_subid_length: u32,
    auth_required: bool,
    payment_required: bool,
    restricted_writes: bool,
}

#[derive(Default, Serialize)]
struct Nip05Document {
    names: BTreeMap<String, String>,
    relays: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Nip98ReplayClaim {
    Claimed,
    Replay,
}

#[async_trait]
pub trait Nip98ReplayStore: Send + Sync {
    async fn claim(
        &self,
        tenant: &TenantContext,
        event_id: EventId,
        expires_at_seconds: u64,
    ) -> Result<Nip98ReplayClaim, NostrHttpDirectoryError>;
}

#[async_trait]
pub trait Nip98PrincipalResolver: Send + Sync {
    async fn resolve(
        &self,
        tenant: &TenantContext,
        public_key: NostrPublicKey,
    ) -> Result<AuthenticatedPrincipal, NostrHttpDirectoryError>;
}

pub struct Nip98Authenticator {
    public_tls: bool,
    replay_store: Arc<dyn Nip98ReplayStore>,
    principal_resolver: Arc<dyn Nip98PrincipalResolver>,
}

impl Nip98Authenticator {
    pub fn new(
        public_tls: bool,
        replay_store: Arc<dyn Nip98ReplayStore>,
        principal_resolver: Arc<dyn Nip98PrincipalResolver>,
    ) -> Self {
        Self {
            public_tls,
            replay_store,
            principal_resolver,
        }
    }

    pub async fn authenticate(
        &self,
        tenant: &TenantContext,
        authorization: &str,
        method: &str,
        path_and_query: &str,
        body: &[u8],
        require_payload: bool,
        now: u64,
    ) -> Result<AuthenticatedPrincipal, Nip98AuthenticationError> {
        validate_http_tenant(tenant)?;
        if authorization.len() > MAX_AUTHORIZATION_BYTES || body.len() > MAX_NIP98_BODY_BYTES {
            return Err(Nip98AuthenticationError::Malformed);
        }
        let encoded = authorization
            .strip_prefix("Nostr ")
            .ok_or(Nip98AuthenticationError::Missing)?;
        let decoded = BASE64
            .decode(encoded)
            .map_err(|_| Nip98AuthenticationError::Malformed)?;
        if decoded.len() > MAX_NOSTR_FRAME_BYTES {
            return Err(Nip98AuthenticationError::Malformed);
        }
        let wire_event: serde_json::Value =
            serde_json::from_slice(&decoded).map_err(|_| Nip98AuthenticationError::Malformed)?;
        let (signed_event, _) =
            parse_wire_signed_event(wire_event).map_err(|_| Nip98AuthenticationError::Malformed)?;
        if signed_event.event.kind != NIP98_KIND {
            return Err(Nip98AuthenticationError::Invalid);
        }
        let verification_event = Arc::new(signed_event);
        let blocking_event = Arc::clone(&verification_event);
        let verification = tokio::task::spawn_blocking(move || {
            verify_signed_event(
                &blocking_event,
                TimestampPolicy::Bounded {
                    now,
                    max_past_seconds: NIP98_FRESHNESS_SECONDS,
                    max_future_seconds: NIP98_FRESHNESS_SECONDS,
                },
            )
        })
        .await
        .map_err(|_| Nip98AuthenticationError::Unavailable)?;
        verification.map_err(|_| Nip98AuthenticationError::Invalid)?;
        let signed_event =
            Arc::try_unwrap(verification_event).unwrap_or_else(|event| (*event).clone());

        let expected_url = expected_http_url(self.public_tls, tenant, path_and_query)?;
        validate_nip98_tags(
            &signed_event.event.tags,
            &expected_url,
            method,
            body,
            require_payload,
        )?;

        match self
            .replay_store
            .claim(
                tenant,
                signed_event.claimed_id,
                now.saturating_add(NIP98_REPLAY_SECONDS),
            )
            .await
        {
            Ok(Nip98ReplayClaim::Claimed) => {}
            Ok(Nip98ReplayClaim::Replay) => return Err(Nip98AuthenticationError::Replay),
            Err(_) => return Err(Nip98AuthenticationError::Unavailable),
        }

        let public_key = NostrPublicKey::from_bytes(*signed_event.event.public_key.as_bytes());
        let principal = self
            .principal_resolver
            .resolve(tenant, public_key)
            .await
            .map_err(|error| match error {
                NostrHttpDirectoryError::Denied => Nip98AuthenticationError::Denied,
                NostrHttpDirectoryError::Unavailable => Nip98AuthenticationError::Unavailable,
            })?;
        validate_nip98_principal(tenant, &principal, public_key)?;
        Ok(principal)
    }
}

fn validate_http_tenant(tenant: &TenantContext) -> Result<(), Nip98AuthenticationError> {
    if !matches!(
        tenant.route_source(),
        TrustedTenantRouteSource::DirectHost | TrustedTenantRouteSource::TrustedForwardedHost
    ) || canonical_authority(tenant.route_reference()).as_deref()
        != Some(tenant.route_reference())
    {
        return Err(Nip98AuthenticationError::TenantBinding);
    }
    Ok(())
}

fn canonical_authority(raw: &str) -> Option<String> {
    let authority = axum::http::uri::Authority::from_str(raw).ok()?;
    let host = authority.host().to_ascii_lowercase();
    if host.is_empty() {
        return None;
    }
    Some(match authority.port_u16() {
        Some(port) => format!("{host}:{port}"),
        None => host,
    })
}

fn expected_http_url(
    public_tls: bool,
    tenant: &TenantContext,
    path_and_query: &str,
) -> Result<String, Nip98AuthenticationError> {
    if !path_and_query.starts_with('/')
        || path_and_query.len() > 8_192
        || path_and_query.chars().any(char::is_control)
    {
        return Err(Nip98AuthenticationError::Invalid);
    }
    let value = format!(
        "{}://{}{}",
        if public_tls { "https" } else { "http" },
        tenant.route_reference(),
        path_and_query
    );
    normalize_http_url(&value).ok_or(Nip98AuthenticationError::Invalid)
}

fn validate_nip98_tags(
    tags: &[Vec<String>],
    expected_url: &str,
    method: &str,
    body: &[u8],
    require_payload: bool,
) -> Result<(), Nip98AuthenticationError> {
    if method.is_empty()
        || method.len() > 32
        || !method.bytes().all(|byte| byte.is_ascii_alphabetic())
    {
        return Err(Nip98AuthenticationError::Invalid);
    }
    let url = unique_tag_value(tags, "u")?;
    let signed_url = normalize_http_url(url).ok_or(Nip98AuthenticationError::Invalid)?;
    if signed_url != expected_url {
        return Err(Nip98AuthenticationError::TenantBinding);
    }
    let signed_method = unique_tag_value(tags, "method")?;
    if !signed_method.eq_ignore_ascii_case(method) {
        return Err(Nip98AuthenticationError::Invalid);
    }
    let payloads = tag_values(tags, "payload")?;
    if payloads.len() > 1 || (require_payload && payloads.is_empty()) {
        return Err(Nip98AuthenticationError::Invalid);
    }
    if let Some(payload) = payloads.first() {
        let computed = hex::encode(Sha256::digest(body));
        if payload.len() != 64
            || !payload
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            || *payload != computed
        {
            return Err(Nip98AuthenticationError::Invalid);
        }
    }
    Ok(())
}

fn unique_tag_value<'a>(
    tags: &'a [Vec<String>],
    name: &str,
) -> Result<&'a str, Nip98AuthenticationError> {
    let values = tag_values(tags, name)?;
    if values.len() != 1 {
        return Err(Nip98AuthenticationError::Invalid);
    }
    Ok(values[0])
}

fn tag_values<'a>(
    tags: &'a [Vec<String>],
    name: &str,
) -> Result<Vec<&'a str>, Nip98AuthenticationError> {
    tags.iter()
        .filter(|tag| tag.first().is_some_and(|value| value == name))
        .map(|tag| {
            if tag.len() != 2 || tag[1].is_empty() {
                Err(Nip98AuthenticationError::Invalid)
            } else {
                Ok(tag[1].as_str())
            }
        })
        .collect()
}

fn normalize_http_url(raw: &str) -> Option<String> {
    let mut url = Url::parse(raw).ok()?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.host_str().is_none()
    {
        return None;
    }
    let path = url.path().trim_end_matches('/').to_owned();
    url.set_path(&path);
    Some(url.to_string())
}

fn validate_nip98_principal(
    tenant: &TenantContext,
    principal: &AuthenticatedPrincipal,
    public_key: NostrPublicKey,
) -> Result<(), Nip98AuthenticationError> {
    if principal.community_id() != tenant.community_id() {
        return Err(Nip98AuthenticationError::TenantBinding);
    }
    match principal.kind() {
        AuthenticatedPrincipalKind::NostrIdentity {
            public_key: resolved,
            authentication_method: NostrAuthenticationMethod::Nip98,
            ..
        } if *resolved == public_key => Ok(()),
        AuthenticatedPrincipalKind::OwnerAttestedAgent {
            agent_public_key,
            authentication_method: NostrAuthenticationMethod::Nip98,
            ..
        } if *agent_public_key == public_key => Ok(()),
        _ => Err(Nip98AuthenticationError::Denied),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Nip98AuthenticationError {
    Missing,
    Malformed,
    Invalid,
    TenantBinding,
    Replay,
    Denied,
    Unavailable,
}

impl fmt::Display for Nip98AuthenticationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Missing => "NIP-98 authorization is missing",
            Self::Malformed => "NIP-98 authorization is malformed",
            Self::Invalid => "NIP-98 authorization is invalid",
            Self::TenantBinding => "NIP-98 authorization does not match the request tenant",
            Self::Replay => "NIP-98 authorization was already used",
            Self::Denied => "NIP-98 principal is not authorized",
            Self::Unavailable => "NIP-98 authorization is unavailable",
        })
    }
}

impl Error for Nip98AuthenticationError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NostrHttpConfigurationError {
    InvalidMetadata,
}

impl fmt::Display for NostrHttpConfigurationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Nostr HTTP metadata configuration is invalid")
    }
}

impl Error for NostrHttpConfigurationError {}

pub fn request_path_and_query(uri: &Uri) -> &str {
    uri.path_and_query()
        .map(axum::http::uri::PathAndQuery::as_str)
        .unwrap_or("/")
}
