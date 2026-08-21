use std::{error::Error, fmt};

use async_trait::async_trait;
use collaboration_domain::{
    AuthenticatedPrincipal, AuthenticatedPrincipalKind, CommunityId, NostrAuthenticationMethod,
    NostrPublicKey, TenantContext,
};
use nostr_compat::{
    EventId, SignedEvent, TimestampPolicy, buzz_nips::identity::AgentAuthentication,
    generated_kinds::KIND_AUTH, verify_signed_event,
};
use rand::random;
use url::Url;

pub const NIP42_AUTH_TIMEOUT_SECONDS: u64 = 5;
pub const NIP42_EVENT_FRESHNESS_SECONDS: u64 = 60;

#[derive(Clone, Eq, PartialEq)]
pub struct NostrAuthChallenge(String);

impl NostrAuthChallenge {
    pub fn generate() -> Self {
        Self(hex::encode(random::<[u8; 32]>()))
    }

    pub fn parse(value: impl Into<String>) -> Result<Self, NostrAuthenticationError> {
        let value = value.into();
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(NostrAuthenticationError::InvalidChallenge);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for NostrAuthChallenge {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("NostrAuthChallenge([redacted])")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedNip42OwnerAttestation {
    owner_public_key: NostrPublicKey,
    exact_conditions: String,
}

impl VerifiedNip42OwnerAttestation {
    pub const fn owner_public_key(&self) -> NostrPublicKey {
        self.owner_public_key
    }

    pub fn exact_conditions(&self) -> &str {
        &self.exact_conditions
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedNip42Identity {
    public_key: NostrPublicKey,
    event_id: EventId,
    owner_attestation: Option<VerifiedNip42OwnerAttestation>,
}

impl VerifiedNip42Identity {
    pub const fn public_key(&self) -> NostrPublicKey {
        self.public_key
    }

    pub const fn event_id(&self) -> EventId {
        self.event_id
    }

    pub const fn owner_attestation(&self) -> Option<&VerifiedNip42OwnerAttestation> {
        self.owner_attestation.as_ref()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplayClaim {
    Claimed,
    Replay,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NostrAuthenticationInfrastructureError {
    Unavailable,
}

#[async_trait]
pub trait NostrAuthReplayStore: Send + Sync {
    async fn claim(
        &self,
        community_id: CommunityId,
        event_id: EventId,
        expires_at_seconds: u64,
    ) -> Result<ReplayClaim, NostrAuthenticationInfrastructureError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NostrPrincipalResolutionError {
    Denied,
    Unavailable,
}

#[async_trait]
pub trait NostrPrincipalResolver: Send + Sync {
    async fn resolve(
        &self,
        tenant: &TenantContext,
        identity: &VerifiedNip42Identity,
    ) -> Result<AuthenticatedPrincipal, NostrPrincipalResolutionError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NostrAuthenticationState {
    Pending {
        challenge: NostrAuthChallenge,
        deadline_seconds: u64,
    },
    Authenticated(AuthenticatedPrincipal),
    Failed,
    TimedOut,
}

pub struct NostrConnectionAuthenticator<R, P> {
    tenant: TenantContext,
    relay_url: String,
    replay_store: R,
    principal_resolver: P,
    state: NostrAuthenticationState,
}

impl<R, P> NostrConnectionAuthenticator<R, P>
where
    R: NostrAuthReplayStore,
    P: NostrPrincipalResolver,
{
    pub fn new(
        tenant: TenantContext,
        relay_url: impl Into<String>,
        challenge: NostrAuthChallenge,
        issued_at_seconds: u64,
        replay_store: R,
        principal_resolver: P,
    ) -> Result<Self, NostrAuthenticationError> {
        let relay_url = relay_url.into();
        if normalize_relay_url(&relay_url).is_none() {
            return Err(NostrAuthenticationError::InvalidRelayConfiguration);
        }
        Ok(Self {
            tenant,
            relay_url,
            replay_store,
            principal_resolver,
            state: NostrAuthenticationState::Pending {
                challenge,
                deadline_seconds: issued_at_seconds.saturating_add(NIP42_AUTH_TIMEOUT_SECONDS),
            },
        })
    }

    pub const fn state(&self) -> &NostrAuthenticationState {
        &self.state
    }

    pub fn challenge(&self) -> Option<&NostrAuthChallenge> {
        let NostrAuthenticationState::Pending { challenge, .. } = &self.state else {
            return None;
        };
        Some(challenge)
    }

    pub fn challenge_frame(&self) -> Option<String> {
        self.challenge()
            .map(|challenge| serde_json::json!(["AUTH", challenge.as_str()]).to_string())
    }

    pub fn expire(&mut self, now_seconds: u64) -> bool {
        let NostrAuthenticationState::Pending {
            deadline_seconds, ..
        } = &self.state
        else {
            return false;
        };
        if now_seconds < *deadline_seconds {
            return false;
        }
        self.state = NostrAuthenticationState::TimedOut;
        true
    }

    pub async fn authenticate(
        &mut self,
        signed_event: &SignedEvent,
        now_seconds: u64,
    ) -> Result<&AuthenticatedPrincipal, NostrAuthenticationError> {
        let challenge = match &self.state {
            NostrAuthenticationState::Pending { challenge, .. } => challenge.clone(),
            NostrAuthenticationState::Authenticated(_) => {
                return Err(NostrAuthenticationError::AlreadyAuthenticated);
            }
            NostrAuthenticationState::Failed => {
                return Err(NostrAuthenticationError::AuthenticationAlreadyFailed);
            }
            NostrAuthenticationState::TimedOut => {
                return Err(NostrAuthenticationError::TimedOut);
            }
        };
        if self.expire(now_seconds) {
            return Err(NostrAuthenticationError::TimedOut);
        }

        let result = self
            .verify_and_resolve(signed_event, &challenge, now_seconds)
            .await;
        match result {
            Ok(principal) => {
                self.state = NostrAuthenticationState::Authenticated(principal);
                match &self.state {
                    NostrAuthenticationState::Authenticated(principal) => Ok(principal),
                    _ => Err(NostrAuthenticationError::InfrastructureUnavailable),
                }
            }
            Err(error) => {
                self.state = NostrAuthenticationState::Failed;
                Err(error)
            }
        }
    }

    async fn verify_and_resolve(
        &self,
        signed_event: &SignedEvent,
        challenge: &NostrAuthChallenge,
        now_seconds: u64,
    ) -> Result<AuthenticatedPrincipal, NostrAuthenticationError> {
        if u32::from(signed_event.event.kind) != KIND_AUTH {
            return Err(NostrAuthenticationError::InvalidEvent);
        }
        verify_signed_event(
            signed_event,
            TimestampPolicy::Bounded {
                now: now_seconds,
                max_past_seconds: NIP42_EVENT_FRESHNESS_SECONDS,
                max_future_seconds: NIP42_EVENT_FRESHNESS_SECONDS,
            },
        )
        .map_err(|_| NostrAuthenticationError::InvalidEvent)?;

        let presented_challenge = single_text_tag(&signed_event.event.tags, "challenge")?;
        if presented_challenge != challenge.as_str() {
            return Err(NostrAuthenticationError::ChallengeMismatch);
        }
        let presented_relay = single_text_tag(&signed_event.event.tags, "relay")?;
        if normalize_relay_url(presented_relay) != normalize_relay_url(&self.relay_url) {
            return Err(NostrAuthenticationError::RelayMismatch);
        }

        let public_key = NostrPublicKey::from_bytes(*signed_event.event.public_key.as_bytes());
        let owner_attestation = if signed_event
            .event
            .tags
            .iter()
            .any(|tag| tag.first().map(String::as_str) == Some("auth"))
        {
            let authentication = AgentAuthentication::parse_event(&signed_event.event)
                .map_err(|_| NostrAuthenticationError::InvalidOwnerAttestation)?;
            authentication
                .verify_attestation(&signed_event.event)
                .map_err(|_| NostrAuthenticationError::InvalidOwnerAttestation)?;
            Some(VerifiedNip42OwnerAttestation {
                owner_public_key: NostrPublicKey::from_bytes(
                    *authentication.attestation.owner.as_bytes(),
                ),
                exact_conditions: authentication.attestation.conditions.as_str().to_owned(),
            })
        } else {
            None
        };
        let identity = VerifiedNip42Identity {
            public_key,
            event_id: signed_event.claimed_id,
            owner_attestation,
        };

        match self
            .replay_store
            .claim(
                self.tenant.community_id(),
                signed_event.claimed_id,
                now_seconds.saturating_add(NIP42_EVENT_FRESHNESS_SECONDS),
            )
            .await
            .map_err(|_| NostrAuthenticationError::InfrastructureUnavailable)?
        {
            ReplayClaim::Claimed => {}
            ReplayClaim::Replay => return Err(NostrAuthenticationError::Replay),
        }

        let principal = self
            .principal_resolver
            .resolve(&self.tenant, &identity)
            .await
            .map_err(|error| match error {
                NostrPrincipalResolutionError::Denied => NostrAuthenticationError::IdentityDenied,
                NostrPrincipalResolutionError::Unavailable => {
                    NostrAuthenticationError::InfrastructureUnavailable
                }
            })?;
        if principal.community_id() != self.tenant.community_id() {
            return Err(NostrAuthenticationError::IdentityDenied);
        }
        if !principal_matches_identity(&principal, &identity) {
            return Err(NostrAuthenticationError::IdentityDenied);
        }
        Ok(principal)
    }
}

fn single_text_tag<'a>(
    tags: &'a [Vec<String>],
    name: &'static str,
) -> Result<&'a str, NostrAuthenticationError> {
    let mut matches = tags
        .iter()
        .filter(|tag| tag.first().map(String::as_str) == Some(name));
    let Some(tag) = matches.next() else {
        return Err(NostrAuthenticationError::InvalidEvent);
    };
    if matches.next().is_some() || tag.len() != 2 || tag[1].is_empty() {
        return Err(NostrAuthenticationError::InvalidEvent);
    }
    Ok(&tag[1])
}

fn normalize_relay_url(raw: &str) -> Option<String> {
    let mut parsed = Url::parse(raw).ok()?;
    if !matches!(parsed.scheme(), "ws" | "wss")
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.fragment().is_some()
    {
        return None;
    }
    if matches!(parsed.host_str(), Some("localhost" | "::1"))
        && parsed.set_host(Some("127.0.0.1")).is_err()
    {
        return None;
    }
    let path = parsed.path().trim_end_matches('/').to_owned();
    parsed.set_path(&path);
    Some(parsed.to_string())
}

fn principal_matches_identity(
    principal: &AuthenticatedPrincipal,
    identity: &VerifiedNip42Identity,
) -> bool {
    match (principal.kind(), identity.owner_attestation()) {
        (
            AuthenticatedPrincipalKind::NostrIdentity {
                public_key,
                authentication_method,
                ..
            },
            None,
        ) => {
            *public_key == identity.public_key()
                && *authentication_method == NostrAuthenticationMethod::Nip42
        }
        (
            AuthenticatedPrincipalKind::OwnerAttestedAgent {
                agent_public_key,
                owner_public_key,
                authentication_method,
                ..
            },
            Some(attestation),
        ) => {
            *agent_public_key == identity.public_key()
                && *owner_public_key == attestation.owner_public_key()
                && *authentication_method == NostrAuthenticationMethod::Nip42
        }
        _ => false,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NostrAuthenticationError {
    InvalidChallenge,
    InvalidRelayConfiguration,
    InvalidEvent,
    ChallengeMismatch,
    RelayMismatch,
    InvalidOwnerAttestation,
    Replay,
    IdentityDenied,
    InfrastructureUnavailable,
    TimedOut,
    AlreadyAuthenticated,
    AuthenticationAlreadyFailed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NostrAuthProtocolDisposition {
    Reject {
        reason: &'static str,
        close_connection: bool,
    },
    CloseWithoutResponse,
}

impl NostrAuthenticationError {
    pub const fn protocol_disposition(self) -> NostrAuthProtocolDisposition {
        match self {
            Self::TimedOut => NostrAuthProtocolDisposition::CloseWithoutResponse,
            Self::AlreadyAuthenticated => NostrAuthProtocolDisposition::Reject {
                reason: "auth-required: already authenticated",
                close_connection: false,
            },
            Self::AuthenticationAlreadyFailed => NostrAuthProtocolDisposition::Reject {
                reason: "auth-required: authentication already failed",
                close_connection: false,
            },
            Self::IdentityDenied | Self::InvalidOwnerAttestation => {
                NostrAuthProtocolDisposition::Reject {
                    reason: "restricted: not a relay member",
                    close_connection: false,
                }
            }
            Self::InfrastructureUnavailable | Self::InvalidRelayConfiguration => {
                NostrAuthProtocolDisposition::Reject {
                    reason: "error: internal",
                    close_connection: true,
                }
            }
            Self::InvalidChallenge
            | Self::InvalidEvent
            | Self::ChallengeMismatch
            | Self::RelayMismatch
            | Self::Replay => NostrAuthProtocolDisposition::Reject {
                reason: "auth-required: verification failed",
                close_connection: false,
            },
        }
    }
}

impl fmt::Display for NostrAuthenticationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidChallenge => "NIP-42 challenge is invalid",
            Self::InvalidRelayConfiguration => "NIP-42 relay configuration is invalid",
            Self::InvalidEvent => "NIP-42 authentication event is invalid",
            Self::ChallengeMismatch => "NIP-42 challenge does not match",
            Self::RelayMismatch => "NIP-42 relay does not match",
            Self::InvalidOwnerAttestation => "NIP-42 owner attestation is invalid",
            Self::Replay => "NIP-42 authentication event was already used",
            Self::IdentityDenied => "NIP-42 identity is not authorized",
            Self::InfrastructureUnavailable => "NIP-42 authentication is unavailable",
            Self::TimedOut => "NIP-42 authentication timed out",
            Self::AlreadyAuthenticated => "NIP-42 connection is already authenticated",
            Self::AuthenticationAlreadyFailed => "NIP-42 authentication already failed",
        })
    }
}

impl Error for NostrAuthenticationError {}
