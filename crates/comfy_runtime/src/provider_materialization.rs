use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
    time::Instant,
};

use comfy_plugin_sdk::CanonicalTypeId;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    ProviderInvocationIdentity, ProviderResultNonce, ProviderResultReceipt,
    ProviderResultReceiptIssuer, ProviderResultReceiptVerifier,
};

pub const NATIVE_PROVIDER_TRANSPORT_SCHEMA: &str = "sim:comfy-provider-transport@1";
pub const NATIVE_PROVIDER_MATERIALIZER_SCHEMA: &str = "sim:comfy-provider-materializer@1";
pub const MAX_PROVIDER_MATERIALIZATION_RESPONSE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum ProviderMaterializationError {
    #[error("provider transport schema is not supported by the native materializer")]
    UnsupportedTransportSchema,
    #[error("provider materializer schema is not supported by the native materializer")]
    UnsupportedMaterializerSchema,
    #[error("provider result response exceeds the session bound")]
    ResponseTooLarge,
    #[error("provider result receipt session is already terminal")]
    ReceiptSessionFinished,
    #[error("provider result request ordinal is not the next host-owned ordinal")]
    RequestOrdinalOutOfOrder,
    #[error("provider result receipt is malformed, forged, expired, or belongs to another request")]
    ReceiptRejected,
    #[error("provider result receipt was not issued by this live session")]
    UnknownReceipt,
    #[error("provider result receipts must be resolved in host issuance order")]
    ReceiptOutOfOrder,
    #[error("provider result receipt session still has unresolved responses")]
    UnresolvedReceipts,
}

pub fn validate_native_provider_schemas(
    transport_schema: &CanonicalTypeId,
    materializer_schema: &CanonicalTypeId,
) -> Result<(), ProviderMaterializationError> {
    if transport_schema.to_string() != NATIVE_PROVIDER_TRANSPORT_SCHEMA {
        return Err(ProviderMaterializationError::UnsupportedTransportSchema);
    }
    if materializer_schema.to_string() != NATIVE_PROVIDER_MATERIALIZER_SCHEMA {
        return Err(ProviderMaterializationError::UnsupportedMaterializerSchema);
    }
    Ok(())
}

struct IssuedProviderResult {
    identity: ProviderInvocationIdentity,
    result_sha256: String,
    response: Vec<u8>,
}

pub struct ProviderResultReceiptSession {
    issuer: Arc<ProviderResultReceiptIssuer>,
    verifier: ProviderResultReceiptVerifier,
    maximum_response_bytes: usize,
    next_request_ordinal: u32,
    issued_order: VecDeque<ProviderResultNonce>,
    issued: BTreeMap<ProviderResultNonce, IssuedProviderResult>,
    terminal: bool,
}

impl ProviderResultReceiptSession {
    pub fn new(
        issuer: Arc<ProviderResultReceiptIssuer>,
        maximum_response_bytes: usize,
        first_request_ordinal: u32,
    ) -> Result<Self, ProviderMaterializationError> {
        if maximum_response_bytes == 0
            || maximum_response_bytes > MAX_PROVIDER_MATERIALIZATION_RESPONSE_BYTES
        {
            return Err(ProviderMaterializationError::ResponseTooLarge);
        }
        let verifier = issuer
            .verifier()
            .map_err(|_| ProviderMaterializationError::ReceiptRejected)?;
        Ok(Self {
            issuer,
            verifier,
            maximum_response_bytes,
            next_request_ordinal: first_request_ordinal,
            issued_order: VecDeque::new(),
            issued: BTreeMap::new(),
            terminal: false,
        })
    }

    pub fn issue(
        &mut self,
        identity: ProviderInvocationIdentity,
        response: Vec<u8>,
        issued_at: Instant,
        expires_at: Instant,
    ) -> Result<Vec<u8>, ProviderMaterializationError> {
        self.check_active()?;
        if identity.request_ordinal() != self.next_request_ordinal {
            return Err(ProviderMaterializationError::RequestOrdinalOutOfOrder);
        }
        if response.len() > self.maximum_response_bytes {
            return Err(ProviderMaterializationError::ResponseTooLarge);
        }
        let next_request_ordinal = self
            .next_request_ordinal
            .checked_add(1)
            .ok_or(ProviderMaterializationError::RequestOrdinalOutOfOrder)?;
        let result_sha256 = format!("{:x}", Sha256::digest(&response));
        let nonce = ProviderResultNonce::generate()
            .map_err(|_| ProviderMaterializationError::ReceiptRejected)?;
        let receipt = self
            .issuer
            .issue(
                identity.clone(),
                result_sha256.clone(),
                issued_at,
                expires_at,
                nonce,
            )
            .map_err(|_| ProviderMaterializationError::ReceiptRejected)?;
        let receipt_bytes = receipt
            .to_bytes()
            .map_err(|_| ProviderMaterializationError::ReceiptRejected)?;
        self.issued.insert(
            nonce,
            IssuedProviderResult {
                identity,
                result_sha256,
                response,
            },
        );
        self.issued_order.push_back(nonce);
        self.next_request_ordinal = next_request_ordinal;
        Ok(receipt_bytes)
    }

    pub fn resolve(
        &mut self,
        receipt_bytes: &[u8],
        expected_identity: &ProviderInvocationIdentity,
        now: Instant,
    ) -> Result<Vec<u8>, ProviderMaterializationError> {
        self.check_active()?;
        let receipt = ProviderResultReceipt::from_bytes(receipt_bytes)
            .map_err(|_| ProviderMaterializationError::ReceiptRejected)?;
        let nonce = receipt.nonce();
        if self.issued_order.front().copied() != Some(nonce) {
            return if self.issued.contains_key(&nonce) {
                Err(ProviderMaterializationError::ReceiptOutOfOrder)
            } else {
                Err(ProviderMaterializationError::UnknownReceipt)
            };
        }
        let issued = self
            .issued
            .get(&nonce)
            .ok_or(ProviderMaterializationError::UnknownReceipt)?;
        if &issued.identity != expected_identity {
            return Err(ProviderMaterializationError::ReceiptRejected);
        }
        self.verifier
            .verify(&receipt, expected_identity, &issued.result_sha256, now)
            .map_err(|_| ProviderMaterializationError::ReceiptRejected)?;
        let issued = self
            .issued
            .remove(&nonce)
            .ok_or(ProviderMaterializationError::UnknownReceipt)?;
        self.issued_order.pop_front();
        Ok(issued.response)
    }

    pub fn finish(mut self) -> Result<(), ProviderMaterializationError> {
        self.check_active()?;
        if !self.issued.is_empty() || !self.issued_order.is_empty() {
            return Err(ProviderMaterializationError::UnresolvedReceipts);
        }
        self.terminal = true;
        Ok(())
    }

    pub fn abort(mut self) {
        self.issued.clear();
        self.issued_order.clear();
        self.terminal = true;
    }

    fn check_active(&self) -> Result<(), ProviderMaterializationError> {
        if self.terminal {
            Err(ProviderMaterializationError::ReceiptSessionFinished)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn invocation_identity(
        node_id: &str,
        request_ordinal: u32,
        request_byte: char,
    ) -> Result<ProviderInvocationIdentity, Box<dyn std::error::Error>> {
        Ok(ProviderInvocationIdentity::new(
            "principal-a",
            "00000000-0000-0000-0000-000000000001",
            "00000000-0000-0000-0000-000000000002",
            "a".repeat(64),
            "00000000-0000-0000-0000-000000000003",
            node_id,
            request_ordinal,
            request_byte.to_string().repeat(64),
            "plugin.fixture",
            "c".repeat(64),
            "d".repeat(64),
            "fixture",
            "https://fixture.invalid/v1/generate",
        )?)
    }

    #[test]
    fn provider_schemas_are_exact_and_owned_by_the_native_materializer()
    -> Result<(), Box<dyn std::error::Error>> {
        let transport: CanonicalTypeId = NATIVE_PROVIDER_TRANSPORT_SCHEMA.parse()?;
        let materializer: CanonicalTypeId = NATIVE_PROVIDER_MATERIALIZER_SCHEMA.parse()?;
        validate_native_provider_schemas(&transport, &materializer)?;

        let wrong_transport: CanonicalTypeId = "sim:other-provider-transport@1".parse()?;
        assert_eq!(
            validate_native_provider_schemas(&wrong_transport, &materializer),
            Err(ProviderMaterializationError::UnsupportedTransportSchema)
        );
        let wrong_materializer: CanonicalTypeId = "sim:other-provider-materializer@1".parse()?;
        assert_eq!(
            validate_native_provider_schemas(&transport, &wrong_materializer),
            Err(ProviderMaterializationError::UnsupportedMaterializerSchema)
        );
        Ok(())
    }

    #[test]
    fn provider_result_session_resolves_exact_ordered_one_time_receipts()
    -> Result<(), Box<dyn std::error::Error>> {
        let origin = Instant::now();
        let issuer = Arc::new(ProviderResultReceiptIssuer::from_seed([23; 32], origin)?);
        let mut session = ProviderResultReceiptSession::new(issuer.clone(), 1_024, 4)?;
        let first_identity = invocation_identity("node.fixture", 4, 'e')?;
        let second_identity = invocation_identity("node.fixture", 5, 'f')?;
        let first_response = b"first-provider-response".to_vec();
        let second_response = b"second-provider-response".to_vec();
        let first_receipt = session.issue(
            first_identity.clone(),
            first_response.clone(),
            origin + Duration::from_secs(1),
            origin + Duration::from_secs(31),
        )?;
        let second_receipt = session.issue(
            second_identity.clone(),
            second_response.clone(),
            origin + Duration::from_secs(2),
            origin + Duration::from_secs(32),
        )?;
        assert!(
            !first_receipt
                .windows(first_response.len())
                .any(|window| window == first_response)
        );
        assert_eq!(
            session.resolve(
                &second_receipt,
                &second_identity,
                origin + Duration::from_secs(3),
            ),
            Err(ProviderMaterializationError::ReceiptOutOfOrder)
        );
        assert_eq!(
            session.resolve(
                &first_receipt,
                &invocation_identity("node.other", 4, 'e')?,
                origin + Duration::from_secs(3),
            ),
            Err(ProviderMaterializationError::ReceiptRejected)
        );
        assert_eq!(
            session.resolve(
                &first_receipt,
                &first_identity,
                origin + Duration::from_secs(3),
            )?,
            first_response
        );
        assert_eq!(
            session.resolve(
                &first_receipt,
                &first_identity,
                origin + Duration::from_secs(3),
            ),
            Err(ProviderMaterializationError::UnknownReceipt)
        );
        assert_eq!(
            session.resolve(
                &second_receipt,
                &second_identity,
                origin + Duration::from_secs(3),
            )?,
            second_response
        );
        session.finish()?;

        let mut unresolved = ProviderResultReceiptSession::new(issuer, 8, 0)?;
        assert_eq!(
            unresolved.issue(
                invocation_identity("node.fixture", 1, '1')?,
                vec![1],
                origin,
                origin + Duration::from_secs(1),
            ),
            Err(ProviderMaterializationError::RequestOrdinalOutOfOrder)
        );
        let receipt = unresolved.issue(
            invocation_identity("node.fixture", 0, '0')?,
            vec![1],
            origin,
            origin + Duration::from_secs(1),
        )?;
        assert!(!receipt.is_empty());
        assert_eq!(
            unresolved.finish(),
            Err(ProviderMaterializationError::UnresolvedReceipts)
        );
        Ok(())
    }
}
