pub mod device_certification;
pub mod native_diffusion_fixture;
pub mod oracle;

pub use device_certification::{
    CertificationArtifact, CertificationArtifactError, CertificationEnvironment, CertificationFact,
    CertificationMatrixRow, CertificationMemoryFact, CertificationPackageEvidence,
    CertificationPayload, CertificationProvenance, CertificationSignature, CertificationStatus,
    ContractEvidence, DeviceEvidence, PackageEvidence, SignedDeviceCertification,
};

pub use native_diffusion_fixture::{NativeDiffusionFixture, NativeDiffusionFixtureError};
pub use oracle::{
    FixtureBundle, OracleError, OracleFixture, OracleRecorder, ReleaseBoundaryPolicy,
    load_embedded_fixtures, load_release_boundary_policy, load_tensor_signature_resolution_fixture,
};

pub fn rust_source_before_test_module(source: &str) -> &str {
    let mut offset = 0;
    let mut lines = source.split_inclusive('\n').peekable();
    while let Some(line) = lines.next() {
        if line.trim() == "#[cfg(test)]"
            && lines
                .peek()
                .is_some_and(|next| next.trim_start().starts_with("mod tests"))
        {
            return source.get(..offset).unwrap_or(source);
        }
        offset += line.len();
    }
    source
}

#[cfg(test)]
mod tests {
    use super::rust_source_before_test_module;

    #[test]
    fn production_source_projection_handles_indented_test_modules() {
        let source = "fn production() {}\n    #[cfg(test)]\n    mod tests {\n        fn helper() {}\n    }\n";
        assert_eq!(
            rust_source_before_test_module(source),
            "fn production() {}\n"
        );
        assert_eq!(
            rust_source_before_test_module("#[cfg(test)]\nfn helper() {}\nfn production() {}\n"),
            "#[cfg(test)]\nfn helper() {}\nfn production() {}\n"
        );
    }
}
