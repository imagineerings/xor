use crate::generated_native_image::{
    NativeImageDescriptor, NativeImageDescriptorError, parse_and_validate_native_descriptors,
};
use std::sync::OnceLock;

pub const NODE_DESCRIPTOR_IDS: &[&str] = &[
    "CheckpointLoaderSimple",
    "CLIPTextEncode",
    "EmptyLatentImage",
    "KSampler",
    "VAEDecode",
];
pub const NATIVE_DIFFUSION_DESCRIPTOR_SCHEMA_VERSION: u16 = 1;

const DESCRIPTORS: &str = include_str!("native_diffusion.descriptors.json");
static PARSED_DESCRIPTORS: OnceLock<Result<Vec<NativeImageDescriptor>, NativeImageDescriptorError>> =
    OnceLock::new();

pub fn native_diffusion_descriptors(
) -> Result<&'static [NativeImageDescriptor], NativeImageDescriptorError> {
    PARSED_DESCRIPTORS
        .get_or_init(|| {
            parse_and_validate_native_descriptors(
                DESCRIPTORS,
                NODE_DESCRIPTOR_IDS,
                NATIVE_DIFFUSION_DESCRIPTOR_SCHEMA_VERSION,
            )
        })
        .as_ref()
        .map(Vec::as_slice)
        .map_err(Clone::clone)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_has_exact_new_diffusion_adapters() -> Result<(), NativeImageDescriptorError> {
        let descriptors = native_diffusion_descriptors()?;
        assert_eq!(
            descriptors
                .iter()
                .map(|descriptor| descriptor.class_type.as_str())
                .collect::<Vec<_>>(),
            NODE_DESCRIPTOR_IDS
        );
        assert!(descriptors.iter().all(|descriptor| !descriptor.output_node));
        Ok(())
    }
}
