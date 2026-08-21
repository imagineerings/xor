use std::{
    collections::BTreeSet,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use async_trait::async_trait;
use collab::{
    nostr::auth::{
        NIP42_AUTH_TIMEOUT_SECONDS, NostrAuthChallenge, NostrAuthProtocolDisposition,
        NostrAuthReplayStore, NostrAuthenticationError, NostrAuthenticationInfrastructureError,
        NostrAuthenticationState, NostrConnectionAuthenticator, NostrPrincipalResolutionError,
        NostrPrincipalResolver, ReplayClaim, VerifiedNip42Identity,
    },
    tenant_admission::bind_rpc_tenant,
};
use collaboration_domain::{
    AuthenticatedPrincipal, AuthorizationScope, CommunityId, NostrAuthenticationMethod,
    NostrPublicKey, PrincipalId, PrincipalScopes, TrustedTenantRoute,
};
use nostr_compat::{
    CanonicalEvent, EventSignature, PublicKey, SignedEvent, generated_kinds::KIND_AUTH,
};
use secp256k1::{Keypair, Message, Secp256k1, SecretKey, XOnlyPublicKey};
use uuid::Uuid;

const RELAY_URL: &str = "wss://relay.example.com";
const NOW: u64 = 1_900_000_000;

fn community(value: u128) -> CommunityId {
    CommunityId::from_uuid(Uuid::from_u128(value))
}

fn principal(value: u128) -> PrincipalId {
    PrincipalId::from_uuid(Uuid::from_u128(value))
}

fn tenant(community_id: CommunityId) -> collaboration_domain::TenantContext {
    bind_rpc_tenant(
        Some(
            TrustedTenantRoute::from_listener(community_id, "nostr-auth-vectors")
                .expect("trusted route"),
        ),
        &[],
    )
    .expect("tenant")
}

fn challenge() -> NostrAuthChallenge {
    NostrAuthChallenge::parse("ab".repeat(32)).expect("challenge")
}

fn signed_auth_event(
    secret_byte: u8,
    challenge: &NostrAuthChallenge,
    relay_url: &str,
    created_at: u64,
) -> SignedEvent {
    let secret = SecretKey::from_slice(&[secret_byte; 32]).expect("fixture secret");
    let secp = Secp256k1::new();
    let keypair = Keypair::from_secret_key(&secp, &secret);
    let (public_key, _) = XOnlyPublicKey::from_keypair(&keypair);
    let event = CanonicalEvent::new(
        PublicKey::from_bytes(public_key.serialize()),
        created_at,
        KIND_AUTH as u16,
        vec![
            vec!["relay".to_owned(), relay_url.to_owned()],
            vec!["challenge".to_owned(), challenge.as_str().to_owned()],
        ],
        String::new(),
    );
    let claimed_id = event.event_id().expect("event id");
    let signature =
        secp.sign_schnorr_no_aux_rand(&Message::from_digest(*claimed_id.as_bytes()), &keypair);
    SignedEvent {
        claimed_id,
        event,
        signature: EventSignature::from_hex(&signature.to_string()).expect("signature"),
    }
}

#[derive(Clone, Default)]
struct ReplayStore {
    state: Arc<ReplayState>,
}

#[derive(Default)]
struct ReplayState {
    claimed: Mutex<BTreeSet<(CommunityId, nostr_compat::EventId)>>,
    attempts: AtomicUsize,
}

#[async_trait]
impl NostrAuthReplayStore for ReplayStore {
    async fn claim(
        &self,
        community_id: CommunityId,
        event_id: nostr_compat::EventId,
        _expires_at_seconds: u64,
    ) -> Result<ReplayClaim, NostrAuthenticationInfrastructureError> {
        self.state.attempts.fetch_add(1, Ordering::SeqCst);
        let inserted = self
            .state
            .claimed
            .lock()
            .expect("replay store lock")
            .insert((community_id, event_id));
        Ok(if inserted {
            ReplayClaim::Claimed
        } else {
            ReplayClaim::Replay
        })
    }
}

#[derive(Clone, Copy)]
enum ResolutionMode {
    Allow,
    WrongTenant,
    Revoked,
}

#[derive(Clone, Copy)]
struct PrincipalResolver {
    mode: ResolutionMode,
}

#[async_trait]
impl NostrPrincipalResolver for PrincipalResolver {
    async fn resolve(
        &self,
        tenant: &collaboration_domain::TenantContext,
        identity: &VerifiedNip42Identity,
    ) -> Result<AuthenticatedPrincipal, NostrPrincipalResolutionError> {
        if matches!(self.mode, ResolutionMode::Revoked) {
            return Err(NostrPrincipalResolutionError::Denied);
        }
        let community_id = if matches!(self.mode, ResolutionMode::WrongTenant) {
            community(2)
        } else {
            tenant.community_id()
        };
        Ok(AuthenticatedPrincipal::nostr_identity(
            principal(3),
            community_id,
            NostrPublicKey::from_bytes(*identity.public_key().as_bytes()),
            NostrAuthenticationMethod::Nip42,
            PrincipalScopes::new([
                AuthorizationScope::new("events:read").expect("read scope"),
                AuthorizationScope::new("events:write").expect("write scope"),
            ])
            .expect("scopes"),
        ))
    }
}

fn authenticator(
    replay_store: ReplayStore,
    mode: ResolutionMode,
) -> NostrConnectionAuthenticator<ReplayStore, PrincipalResolver> {
    NostrConnectionAuthenticator::new(
        tenant(community(1)),
        RELAY_URL,
        challenge(),
        NOW,
        replay_store,
        PrincipalResolver { mode },
    )
    .expect("authenticator")
}

#[tokio::test]
async fn nostr_auth_vectors_accept_success_and_reject_reauthentication() {
    let replay_store = ReplayStore::default();
    let event = signed_auth_event(1, &challenge(), RELAY_URL, NOW);
    let mut authenticator = authenticator(replay_store.clone(), ResolutionMode::Allow);
    assert_eq!(
        authenticator.challenge_frame().as_deref(),
        Some(r#"["AUTH","abababababababababababababababababababababababababababababababab"]"#)
    );

    let authenticated = authenticator
        .authenticate(&event, NOW)
        .await
        .expect("valid NIP-42 event");
    assert_eq!(authenticated.community_id(), community(1));
    assert_eq!(authenticated.principal_id(), principal(3));
    assert!(matches!(
        authenticator.state(),
        NostrAuthenticationState::Authenticated(_)
    ));
    let error = authenticator
        .authenticate(&event, NOW)
        .await
        .expect_err("reauthentication is rejected");
    assert_eq!(error, NostrAuthenticationError::AlreadyAuthenticated);
    assert_eq!(
        error.protocol_disposition(),
        NostrAuthProtocolDisposition::Reject {
            reason: "auth-required: already authenticated",
            close_connection: false,
        }
    );
    assert_eq!(replay_store.state.attempts.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn nostr_auth_vectors_enforce_connection_timeout_before_crypto_or_replay() {
    let replay_store = ReplayStore::default();
    let event = signed_auth_event(1, &challenge(), RELAY_URL, NOW);
    let mut authenticator = authenticator(replay_store.clone(), ResolutionMode::Allow);

    let error = authenticator
        .authenticate(&event, NOW + NIP42_AUTH_TIMEOUT_SECONDS)
        .await
        .expect_err("deadline is terminal");
    assert_eq!(error, NostrAuthenticationError::TimedOut);
    assert_eq!(
        error.protocol_disposition(),
        NostrAuthProtocolDisposition::CloseWithoutResponse
    );
    assert_eq!(authenticator.state(), &NostrAuthenticationState::TimedOut);
    assert_eq!(replay_store.state.attempts.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn nostr_auth_vectors_reject_a_tenant_scoped_event_replay() {
    let replay_store = ReplayStore::default();
    let event = signed_auth_event(1, &challenge(), RELAY_URL, NOW);
    authenticator(replay_store.clone(), ResolutionMode::Allow)
        .authenticate(&event, NOW)
        .await
        .expect("first authentication");
    let mut replayed = authenticator(replay_store, ResolutionMode::Allow);

    assert_eq!(
        replayed.authenticate(&event, NOW).await,
        Err(NostrAuthenticationError::Replay)
    );
    assert_eq!(replayed.state(), &NostrAuthenticationState::Failed);
}

#[tokio::test]
async fn nostr_auth_vectors_reject_wrong_tenant_identity_resolution() {
    let event = signed_auth_event(1, &challenge(), RELAY_URL, NOW);
    let mut authenticator = authenticator(ReplayStore::default(), ResolutionMode::WrongTenant);

    assert_eq!(
        authenticator.authenticate(&event, NOW).await,
        Err(NostrAuthenticationError::IdentityDenied)
    );
    assert_eq!(authenticator.state(), &NostrAuthenticationState::Failed);
}

#[tokio::test]
async fn nostr_auth_vectors_reject_revoked_keys() {
    let event = signed_auth_event(1, &challenge(), RELAY_URL, NOW);
    let mut authenticator = authenticator(ReplayStore::default(), ResolutionMode::Revoked);

    assert_eq!(
        authenticator.authenticate(&event, NOW).await,
        Err(NostrAuthenticationError::IdentityDenied)
    );
    assert_eq!(authenticator.state(), &NostrAuthenticationState::Failed);
}

#[tokio::test]
async fn nostr_auth_vectors_reject_wrong_challenge_and_relay_without_replay_claims() {
    for event in [
        signed_auth_event(
            1,
            &NostrAuthChallenge::parse("cd".repeat(32)).expect("other challenge"),
            RELAY_URL,
            NOW,
        ),
        signed_auth_event(1, &challenge(), "wss://other.example.com", NOW),
    ] {
        let replay_store = ReplayStore::default();
        let mut authenticator = authenticator(replay_store.clone(), ResolutionMode::Allow);
        assert!(matches!(
            authenticator.authenticate(&event, NOW).await,
            Err(NostrAuthenticationError::ChallengeMismatch
                | NostrAuthenticationError::RelayMismatch)
        ));
        assert_eq!(replay_store.state.attempts.load(Ordering::SeqCst), 0);
    }
}
