use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
    time::{Duration, Instant},
};

use comfy_plugin_sdk::{CanonicalTypeId, ProviderResultReceiptSet, ValueFamily};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    MAX_PROVIDER_RESULT_RECEIPT_LIFETIME, ProviderInvocationIdentity, ProviderResultNonce,
    ProviderResultReceipt, ProviderResultReceiptIssuer, ProviderResultReceiptVerifier,
};

pub const NATIVE_PROVIDER_TRANSPORT_SCHEMA: &str = "sim:comfy-provider-transport@1";
pub const NATIVE_PROVIDER_MATERIALIZER_SCHEMA: &str = "sim:comfy-provider-materializer@1";
pub const MAX_PROVIDER_MATERIALIZATION_RESPONSE_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_PROVIDER_TRANSPORT_REQUEST_BYTES: usize = 16 * 1024 * 1024;
const PROVIDER_TRANSPORT_REQUEST_DOMAIN: &[u8] = b"sim.comfy.provider-transport-request\0";
const PROVIDER_TRANSPORT_RESPONSE_DOMAIN: &[u8] = b"sim.comfy.provider-transport-response\0";
const PROVIDER_TRANSPORT_VERSION: u16 = 1;
const MAX_PROVIDER_TRANSPORT_PORTS: usize = 1_024;
const MAX_PROVIDER_TRANSPORT_VALUES_PER_PORT: usize = 4_096;
const MAX_PROVIDER_TRANSPORT_IDENTITY_BYTES: usize = 1_024;

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
    #[error("provider result receipt authority is invalid")]
    InvalidReceiptAuthority,
    #[error("provider transport projection is invalid or exceeds its bound")]
    InvalidTransportProjection,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderTransportValue {
    type_id: String,
    family: ValueFamily,
    abi_bytes: Vec<u8>,
}

impl ProviderTransportValue {
    pub fn checked(
        type_id: impl Into<String>,
        family: ValueFamily,
        abi_bytes: Vec<u8>,
    ) -> Result<Self, ProviderMaterializationError> {
        let value = Self {
            type_id: type_id.into(),
            family,
            abi_bytes,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn type_id(&self) -> &str {
        &self.type_id
    }

    pub const fn family(&self) -> ValueFamily {
        self.family
    }

    pub fn abi_bytes(&self) -> &[u8] {
        &self.abi_bytes
    }

    fn validate(&self) -> Result<(), ProviderMaterializationError> {
        if !valid_transport_identity(&self.type_id)
            || self.abi_bytes.is_empty()
            || self.abi_bytes.len() > MAX_PROVIDER_TRANSPORT_REQUEST_BYTES
        {
            return Err(ProviderMaterializationError::InvalidTransportProjection);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderTransportPort {
    port_id: String,
    present: bool,
    values: Vec<ProviderTransportValue>,
}

impl ProviderTransportPort {
    pub fn checked(
        port_id: impl Into<String>,
        present: bool,
        values: Vec<ProviderTransportValue>,
    ) -> Result<Self, ProviderMaterializationError> {
        let port = Self {
            port_id: port_id.into(),
            present,
            values,
        };
        port.validate()?;
        Ok(port)
    }

    pub fn port_id(&self) -> &str {
        &self.port_id
    }

    pub const fn present(&self) -> bool {
        self.present
    }

    pub fn values(&self) -> &[ProviderTransportValue] {
        &self.values
    }

    fn validate(&self) -> Result<(), ProviderMaterializationError> {
        if !valid_transport_identity(&self.port_id)
            || self.values.len() > MAX_PROVIDER_TRANSPORT_VALUES_PER_PORT
            || (!self.present && !self.values.is_empty())
        {
            return Err(ProviderMaterializationError::InvalidTransportProjection);
        }
        self.values
            .iter()
            .try_for_each(ProviderTransportValue::validate)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ProviderTransportProjection {
    class_type: String,
    ports: Vec<ProviderTransportPort>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderTransportRequest(ProviderTransportProjection);

impl ProviderTransportRequest {
    pub fn checked(
        class_type: impl Into<String>,
        ports: Vec<ProviderTransportPort>,
    ) -> Result<Self, ProviderMaterializationError> {
        let projection = ProviderTransportProjection {
            class_type: class_type.into(),
            ports,
        };
        validate_transport_projection(&projection)?;
        Ok(Self(projection))
    }

    pub fn class_type(&self) -> &str {
        &self.0.class_type
    }

    pub fn ports(&self) -> &[ProviderTransportPort] {
        &self.0.ports
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, ProviderMaterializationError> {
        encode_transport_projection(
            PROVIDER_TRANSPORT_REQUEST_DOMAIN,
            &self.0,
            MAX_PROVIDER_TRANSPORT_REQUEST_BYTES,
        )
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProviderMaterializationError> {
        decode_transport_projection(
            PROVIDER_TRANSPORT_REQUEST_DOMAIN,
            bytes,
            MAX_PROVIDER_TRANSPORT_REQUEST_BYTES,
        )
        .map(Self)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderTransportResponse(ProviderTransportProjection);

impl ProviderTransportResponse {
    pub fn checked(
        class_type: impl Into<String>,
        ports: Vec<ProviderTransportPort>,
    ) -> Result<Self, ProviderMaterializationError> {
        let projection = ProviderTransportProjection {
            class_type: class_type.into(),
            ports,
        };
        validate_transport_projection(&projection)?;
        Ok(Self(projection))
    }

    pub fn class_type(&self) -> &str {
        &self.0.class_type
    }

    pub fn ports(&self) -> &[ProviderTransportPort] {
        &self.0.ports
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, ProviderMaterializationError> {
        encode_transport_projection(
            PROVIDER_TRANSPORT_RESPONSE_DOMAIN,
            &self.0,
            MAX_PROVIDER_MATERIALIZATION_RESPONSE_BYTES,
        )
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProviderMaterializationError> {
        decode_transport_projection(
            PROVIDER_TRANSPORT_RESPONSE_DOMAIN,
            bytes,
            MAX_PROVIDER_MATERIALIZATION_RESPONSE_BYTES,
        )
        .map(Self)
    }
}

fn validate_transport_projection(
    projection: &ProviderTransportProjection,
) -> Result<(), ProviderMaterializationError> {
    if !valid_transport_identity(&projection.class_type)
        || projection.ports.len() > MAX_PROVIDER_TRANSPORT_PORTS
    {
        return Err(ProviderMaterializationError::InvalidTransportProjection);
    }
    let mut previous_port = None;
    for port in &projection.ports {
        port.validate()?;
        if previous_port.is_some_and(|previous| previous >= port.port_id()) {
            return Err(ProviderMaterializationError::InvalidTransportProjection);
        }
        previous_port = Some(port.port_id());
    }
    Ok(())
}

fn encode_transport_projection(
    domain: &[u8],
    projection: &ProviderTransportProjection,
    maximum_bytes: usize,
) -> Result<Vec<u8>, ProviderMaterializationError> {
    validate_transport_projection(projection)?;
    let payload = postcard::to_stdvec(projection)
        .map_err(|_| ProviderMaterializationError::InvalidTransportProjection)?;
    let total = domain
        .len()
        .checked_add(2)
        .and_then(|length| length.checked_add(payload.len()))
        .ok_or(ProviderMaterializationError::InvalidTransportProjection)?;
    if total > maximum_bytes {
        return Err(ProviderMaterializationError::InvalidTransportProjection);
    }
    let mut bytes = Vec::with_capacity(total);
    bytes.extend_from_slice(domain);
    bytes.extend_from_slice(&PROVIDER_TRANSPORT_VERSION.to_le_bytes());
    bytes.extend_from_slice(&payload);
    Ok(bytes)
}

fn decode_transport_projection(
    domain: &[u8],
    bytes: &[u8],
    maximum_bytes: usize,
) -> Result<ProviderTransportProjection, ProviderMaterializationError> {
    if bytes.len() > maximum_bytes || !bytes.starts_with(domain) {
        return Err(ProviderMaterializationError::InvalidTransportProjection);
    }
    let version_start = domain.len();
    let version_end = version_start
        .checked_add(2)
        .ok_or(ProviderMaterializationError::InvalidTransportProjection)?;
    let version_bytes = bytes
        .get(version_start..version_end)
        .ok_or(ProviderMaterializationError::InvalidTransportProjection)?;
    let version = u16::from_le_bytes(
        version_bytes
            .try_into()
            .map_err(|_| ProviderMaterializationError::InvalidTransportProjection)?,
    );
    if version != PROVIDER_TRANSPORT_VERSION {
        return Err(ProviderMaterializationError::InvalidTransportProjection);
    }
    let projection: ProviderTransportProjection = postcard::from_bytes(
        bytes
            .get(version_end..)
            .ok_or(ProviderMaterializationError::InvalidTransportProjection)?,
    )
    .map_err(|_| ProviderMaterializationError::InvalidTransportProjection)?;
    validate_transport_projection(&projection)?;
    Ok(projection)
}

fn valid_transport_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_PROVIDER_TRANSPORT_IDENTITY_BYTES
        && value == value.trim()
        && !value.chars().any(char::is_control)
}

#[derive(Clone)]
pub struct ProviderResultReceiptAuthority {
    principal_id: String,
    prompt_sha256: String,
    provider_binding_sha256: String,
    issuer: Arc<ProviderResultReceiptIssuer>,
    receipt_lifetime: Duration,
}

impl ProviderResultReceiptAuthority {
    pub fn new(
        principal_id: impl Into<String>,
        prompt_sha256: impl Into<String>,
        provider_binding_sha256: impl Into<String>,
        issuer: Arc<ProviderResultReceiptIssuer>,
        receipt_lifetime: Duration,
    ) -> Result<Self, ProviderMaterializationError> {
        let principal_id = principal_id.into();
        let prompt_sha256 = prompt_sha256.into();
        let provider_binding_sha256 = provider_binding_sha256.into();
        if principal_id.is_empty()
            || principal_id.len() > 1_024
            || principal_id != principal_id.trim()
            || principal_id.chars().any(char::is_control)
            || !is_sha256(&prompt_sha256)
            || !is_sha256(&provider_binding_sha256)
            || receipt_lifetime.is_zero()
            || receipt_lifetime > MAX_PROVIDER_RESULT_RECEIPT_LIFETIME
        {
            return Err(ProviderMaterializationError::InvalidReceiptAuthority);
        }
        Ok(Self {
            principal_id,
            prompt_sha256,
            provider_binding_sha256,
            issuer,
            receipt_lifetime,
        })
    }

    pub fn principal_id(&self) -> &str {
        &self.principal_id
    }

    pub fn prompt_sha256(&self) -> &str {
        &self.prompt_sha256
    }

    pub fn provider_binding_sha256(&self) -> &str {
        &self.provider_binding_sha256
    }

    pub fn receipt_lifetime(&self) -> Duration {
        self.receipt_lifetime
    }

    pub fn begin_session(
        &self,
        maximum_response_bytes: usize,
    ) -> Result<ProviderResultReceiptSession, ProviderMaterializationError> {
        ProviderResultReceiptSession::new(self.issuer.clone(), maximum_response_bytes, 0)
    }
}

impl std::fmt::Debug for ProviderResultReceiptAuthority {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ProviderResultReceiptAuthority([REDACTED])")
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedProviderResult {
    identity: ProviderInvocationIdentity,
    response: Vec<u8>,
}

impl ResolvedProviderResult {
    pub fn identity(&self) -> &ProviderInvocationIdentity {
        &self.identity
    }

    pub fn response(&self) -> &[u8] {
        &self.response
    }

    pub fn into_response(self) -> Vec<u8> {
        self.response
    }
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

    pub fn next_request_ordinal(&self) -> Result<u32, ProviderMaterializationError> {
        self.check_active()?;
        Ok(self.next_request_ordinal)
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

    pub fn resolve_receipt_set(
        &mut self,
        receipt_set: &ProviderResultReceiptSet,
        now: Instant,
    ) -> Result<Vec<ResolvedProviderResult>, ProviderMaterializationError> {
        self.check_active()?;
        if receipt_set.receipts().len() != self.issued_order.len() {
            return Err(ProviderMaterializationError::UnresolvedReceipts);
        }
        let mut validated_nonces = Vec::with_capacity(receipt_set.receipts().len());
        for (receipt_bytes, expected_nonce) in receipt_set
            .receipts()
            .iter()
            .zip(self.issued_order.iter().copied())
        {
            let receipt = ProviderResultReceipt::from_bytes(receipt_bytes)
                .map_err(|_| ProviderMaterializationError::ReceiptRejected)?;
            if receipt.nonce() != expected_nonce {
                return if self.issued.contains_key(&receipt.nonce()) {
                    Err(ProviderMaterializationError::ReceiptOutOfOrder)
                } else {
                    Err(ProviderMaterializationError::UnknownReceipt)
                };
            }
            let issued = self
                .issued
                .get(&expected_nonce)
                .ok_or(ProviderMaterializationError::UnknownReceipt)?;
            self.verifier
                .verify(&receipt, &issued.identity, &issued.result_sha256, now)
                .map_err(|_| ProviderMaterializationError::ReceiptRejected)?;
            validated_nonces.push(expected_nonce);
        }

        let mut resolved = Vec::with_capacity(validated_nonces.len());
        for nonce in validated_nonces {
            if self.issued_order.pop_front() != Some(nonce) {
                return Err(ProviderMaterializationError::ReceiptOutOfOrder);
            }
            let issued = self
                .issued
                .remove(&nonce)
                .ok_or(ProviderMaterializationError::UnknownReceipt)?;
            resolved.push(ResolvedProviderResult {
                identity: issued.identity,
                response: issued.response,
            });
        }
        Ok(resolved)
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
    use comfy_plugin_sdk::{PluginValue, ScalarValue, TypeRegistry};
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
    fn provider_transport_is_domain_separated_bounded_and_canonical()
    -> Result<(), Box<dyn std::error::Error>> {
        let registry = TypeRegistry::built_in()?;
        let type_id: CanonicalTypeId = "comfy:string@1".parse()?;
        let value = PluginValue::scalar(
            type_id.clone(),
            ScalarValue::String("result".to_owned()),
            &registry,
        )?;
        let port = ProviderTransportPort::checked(
            "result",
            true,
            vec![ProviderTransportValue::checked(
                type_id.to_string(),
                value.family(),
                value.abi_bytes()?,
            )?],
        )?;
        let request = ProviderTransportRequest::checked("provider.echo", vec![port.clone()])?;
        let response = ProviderTransportResponse::checked("provider.echo", vec![port])?;
        let request_bytes = request.to_bytes()?;
        let response_bytes = response.to_bytes()?;
        assert_ne!(request_bytes, response_bytes);
        assert_eq!(
            ProviderTransportRequest::from_bytes(&request_bytes)?,
            request
        );
        assert_eq!(
            ProviderTransportResponse::from_bytes(&response_bytes)?,
            response
        );
        assert!(ProviderTransportRequest::from_bytes(&response_bytes).is_err());
        assert!(ProviderTransportResponse::from_bytes(&request_bytes).is_err());

        assert!(
            ProviderTransportRequest::checked(
                "provider.echo",
                vec![
                    ProviderTransportPort::checked("z", false, Vec::new())?,
                    ProviderTransportPort::checked("a", false, Vec::new())?,
                ],
            )
            .is_err()
        );
        assert!(
            ProviderTransportPort::checked(
                "result",
                false,
                vec![ProviderTransportValue::checked(
                    type_id.to_string(),
                    ValueFamily::Scalar,
                    value.abi_bytes()?,
                )?,]
            )
            .is_err()
        );
        Ok(())
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

    #[test]
    fn provider_result_session_resolves_a_complete_receipt_set_atomically()
    -> Result<(), Box<dyn std::error::Error>> {
        let origin = Instant::now();
        let issuer = Arc::new(ProviderResultReceiptIssuer::from_seed([29; 32], origin)?);
        let mut session = ProviderResultReceiptSession::new(issuer, 1_024, 7)?;
        let first_identity = invocation_identity("node.fixture", 7, 'a')?;
        let second_identity = invocation_identity("node.fixture", 8, 'b')?;
        let first_receipt = session.issue(
            first_identity.clone(),
            b"first".to_vec(),
            origin + Duration::from_secs(1),
            origin + Duration::from_secs(31),
        )?;
        let second_receipt = session.issue(
            second_identity.clone(),
            b"second".to_vec(),
            origin + Duration::from_secs(2),
            origin + Duration::from_secs(32),
        )?;
        let reversed =
            ProviderResultReceiptSet::new(vec![second_receipt.clone(), first_receipt.clone()])?;
        assert_eq!(
            session.resolve_receipt_set(&reversed, origin + Duration::from_secs(3)),
            Err(ProviderMaterializationError::ReceiptOutOfOrder)
        );

        let receipt_set = ProviderResultReceiptSet::new(vec![first_receipt, second_receipt])?;
        let resolved =
            session.resolve_receipt_set(&receipt_set, origin + Duration::from_secs(3))?;
        assert_eq!(resolved.len(), 2);
        assert_eq!(resolved[0].identity(), &first_identity);
        assert_eq!(resolved[0].response(), b"first");
        assert_eq!(resolved[1].identity(), &second_identity);
        assert_eq!(resolved[1].response(), b"second");
        session.finish()?;
        Ok(())
    }
}
