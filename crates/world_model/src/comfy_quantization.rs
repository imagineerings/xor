use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::SafetensorsHeaderMetadata;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum QuantizationFormat {
    Fp8E4M3,
    Fp8E5M2,
    Int8,
    Int4,
    Nf4,
    Gguf,
    Unknown(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct QuantizedLayerMetadata {
    pub layer_name: String,
    pub format: QuantizationFormat,
    pub scale: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyQuantizationMetadata {
    pub global_format: Option<QuantizationFormat>,
    pub layers: Vec<QuantizedLayerMetadata>,
}

impl ComfyQuantizationMetadata {
    pub fn from_safetensors(metadata: &SafetensorsHeaderMetadata) -> Option<Self> {
        let global_format = metadata
            .metadata
            .get("quantization.format")
            .or_else(|| metadata.metadata.get("quant_method"))
            .or_else(|| metadata.metadata.get("format"))
            .and_then(|value| parse_quantization_format(value));

        let layer_values = metadata
            .metadata
            .iter()
            .filter_map(|(key, value)| {
                parse_layer_key(key).map(|(layer, field)| (layer, field, value))
            })
            .fold(
                BTreeMap::<String, BTreeMap<String, String>>::new(),
                |mut layers, (layer, field, value)| {
                    layers
                        .entry(layer)
                        .or_default()
                        .insert(field, value.to_string());
                    layers
                },
            );

        let mut layers = Vec::new();
        for (layer_name, values) in layer_values {
            let format = values
                .get("format")
                .and_then(|value| parse_quantization_format(value))
                .or_else(|| global_format.clone());
            if let Some(format) = format {
                layers.push(QuantizedLayerMetadata {
                    layer_name,
                    format,
                    scale: values.get("scale").cloned(),
                });
            }
        }

        if global_format.is_none() && layers.is_empty() {
            return None;
        }

        Some(Self {
            global_format,
            layers,
        })
    }

    pub fn has_quantized_weights(&self) -> bool {
        self.global_format.is_some() || !self.layers.is_empty()
    }
}

fn parse_layer_key(key: &str) -> Option<(String, String)> {
    let remainder = key.strip_prefix("quantization.layers.")?;
    let (layer, field) = remainder.rsplit_once('.')?;
    if layer.is_empty() || field.is_empty() {
        return None;
    }
    Some((layer.to_string(), field.to_string()))
}

fn parse_quantization_format(value: &str) -> Option<QuantizationFormat> {
    let normalized = value.trim().replace(['-', ' '], "_").to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }

    Some(match normalized.as_str() {
        "fp8" | "fp8_e4m3" | "fp8_e4m3fn" => QuantizationFormat::Fp8E4M3,
        "fp8_e5m2" => QuantizationFormat::Fp8E5M2,
        "int8" | "i8" | "q8" | "q8_0" => QuantizationFormat::Int8,
        "int4" | "i4" | "q4" | "q4_0" | "q4_k" => QuantizationFormat::Int4,
        "nf4" => QuantizationFormat::Nf4,
        "gguf" => QuantizationFormat::Gguf,
        other if other.contains("fp8") => QuantizationFormat::Fp8E4M3,
        other if other.contains("int8") || other.contains("q8") => QuantizationFormat::Int8,
        other if other.contains("int4") || other.contains("q4") => QuantizationFormat::Int4,
        other => QuantizationFormat::Unknown(other.to_string()),
    })
}
