use comfy_plugin_sdk::CanonicalTypeId;
use thiserror::Error;

pub const NATIVE_PROVIDER_TRANSPORT_SCHEMA: &str = "sim:comfy-provider-transport@1";
pub const NATIVE_PROVIDER_MATERIALIZER_SCHEMA: &str = "sim:comfy-provider-materializer@1";

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum ProviderMaterializationError {
    #[error("provider transport schema is not supported by the native materializer")]
    UnsupportedTransportSchema,
    #[error("provider materializer schema is not supported by the native materializer")]
    UnsupportedMaterializerSchema,
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
