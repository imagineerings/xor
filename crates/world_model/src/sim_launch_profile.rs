use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{DeviceBackend, MemoryMode, PrecisionPolicy, RuntimePolicyRequest};

pub const LAUNCH_PROFILE_INVALID_OPTION_CODE: &str = "world_model.launch_profile.invalid_option";
pub const LAUNCH_PROFILE_UNSUPPORTED_OPTION_CODE: &str =
    "world_model.launch_profile.unsupported_option";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimLaunchDiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimLaunchDiagnostic {
    pub code: String,
    pub severity: SimLaunchDiagnosticSeverity,
    pub option: String,
    pub message: String,
    pub nearest_sim_equivalent: Option<String>,
}

impl SimLaunchDiagnostic {
    fn invalid(option: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: LAUNCH_PROFILE_INVALID_OPTION_CODE.to_string(),
            severity: SimLaunchDiagnosticSeverity::Error,
            option: option.into(),
            message: message.into(),
            nearest_sim_equivalent: None,
        }
    }

    fn unsupported(
        option: impl Into<String>,
        reason: impl Into<String>,
        nearest_sim_equivalent: Option<&'static str>,
    ) -> Self {
        Self {
            code: LAUNCH_PROFILE_UNSUPPORTED_OPTION_CODE.to_string(),
            severity: SimLaunchDiagnosticSeverity::Warning,
            option: option.into(),
            message: reason.into(),
            nearest_sim_equivalent: nearest_sim_equivalent.map(str::to_string),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimLaunchProfile {
    pub network: SimNetworkLaunchOptions,
    pub directories: SimDirectoryLaunchOptions,
    pub upload_limit_bytes: Option<u64>,
    pub auto_launch: bool,
    pub logging: SimLoggingLaunchOptions,
    pub assets: SimAssetLaunchOptions,
    pub database_url: Option<String>,
    pub api_nodes: SimApiNodeLaunchOptions,
    pub custom_nodes: SimCustomNodeLaunchOptions,
    pub manager: SimManagerLaunchOptions,
    pub feature_flags: BTreeMap<String, String>,
    pub compression: SimCompressionLaunchOptions,
    pub runtime_policy: RuntimePolicyRequest,
    pub cache: SimCacheLaunchOptions,
    pub performance: SimPerformanceLaunchOptions,
    pub diagnostics: Vec<SimLaunchDiagnostic>,
}

impl Default for SimLaunchProfile {
    fn default() -> Self {
        Self {
            network: SimNetworkLaunchOptions::default(),
            directories: SimDirectoryLaunchOptions::default(),
            upload_limit_bytes: None,
            auto_launch: false,
            logging: SimLoggingLaunchOptions::default(),
            assets: SimAssetLaunchOptions::default(),
            database_url: None,
            api_nodes: SimApiNodeLaunchOptions::default(),
            custom_nodes: SimCustomNodeLaunchOptions::default(),
            manager: SimManagerLaunchOptions::default(),
            feature_flags: BTreeMap::new(),
            compression: SimCompressionLaunchOptions::default(),
            runtime_policy: RuntimePolicyRequest::new(
                PrecisionPolicy::Fp32,
                DeviceBackend::Cpu,
                MemoryMode::DynamicVram,
            ),
            cache: SimCacheLaunchOptions::default(),
            performance: SimPerformanceLaunchOptions::default(),
            diagnostics: Vec::new(),
        }
    }
}

impl SimLaunchProfile {
    pub fn is_valid(&self) -> bool {
        self.diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity != SimLaunchDiagnosticSeverity::Error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimNetworkLaunchOptions {
    pub listen_address: String,
    pub port: u16,
    pub tls_keyfile: Option<PathBuf>,
    pub tls_certfile: Option<PathBuf>,
    pub cors_origin: Option<String>,
}

impl Default for SimNetworkLaunchOptions {
    fn default() -> Self {
        Self {
            listen_address: "127.0.0.1".to_string(),
            port: 8188,
            tls_keyfile: None,
            tls_certfile: None,
            cors_origin: None,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimDirectoryLaunchOptions {
    pub base_directory: Option<PathBuf>,
    pub input_directory: Option<PathBuf>,
    pub output_directory: Option<PathBuf>,
    pub temp_directory: Option<PathBuf>,
    pub user_directory: Option<PathBuf>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimLoggingLaunchOptions {
    pub level: String,
    pub verbose: bool,
    pub stdout: bool,
}

impl Default for SimLoggingLaunchOptions {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            verbose: false,
            stdout: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimAssetLaunchOptions {
    pub enabled: bool,
}

impl Default for SimAssetLaunchOptions {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimApiNodeLaunchOptions {
    pub enabled: bool,
}

impl Default for SimApiNodeLaunchOptions {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimCustomNodeLaunchOptions {
    pub enabled: bool,
    pub directory: Option<PathBuf>,
}

impl Default for SimCustomNodeLaunchOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            directory: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimManagerLaunchOptions {
    pub enabled: bool,
    pub mode: String,
}

impl Default for SimManagerLaunchOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: "disabled".to_string(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimCompressionLaunchOptions {
    pub response_compression: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimCacheLaunchOptions {
    pub mode: String,
}

impl Default for SimCacheLaunchOptions {
    fn default() -> Self {
        Self {
            mode: "classic".to_string(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimPerformanceLaunchOptions {
    pub attention_backend: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct SimLaunchProfileParser;

impl SimLaunchProfileParser {
    pub fn new() -> Self {
        Self
    }

    pub fn parse_args<I, S>(&self, args: I) -> SimLaunchProfile
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut profile = SimLaunchProfile::default();
        let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
        let mut index = 0;

        while index < args.len() {
            let option = &args[index];
            match option.as_str() {
                "--listen" => {
                    profile.network.listen_address =
                        optional_value(&args, &mut index).unwrap_or("0.0.0.0".to_string());
                }
                "--port" => {
                    if let Some(value) = required_value(&args, &mut index, &mut profile) {
                        match value.parse::<u16>() {
                            Ok(port) => profile.network.port = port,
                            Err(_) => profile.diagnostics.push(SimLaunchDiagnostic::invalid(
                                option,
                                "port must be an integer between 0 and 65535",
                            )),
                        }
                    }
                }
                "--tls-keyfile" => {
                    if let Some(value) = required_value(&args, &mut index, &mut profile) {
                        profile.network.tls_keyfile = Some(PathBuf::from(value));
                    }
                }
                "--tls-certfile" => {
                    if let Some(value) = required_value(&args, &mut index, &mut profile) {
                        profile.network.tls_certfile = Some(PathBuf::from(value));
                    }
                }
                "--enable-cors-header" => {
                    profile.network.cors_origin =
                        optional_value(&args, &mut index).or(Some("*".to_string()));
                }
                "--max-upload-size" => {
                    if let Some(value) = required_value(&args, &mut index, &mut profile) {
                        match value.parse::<u64>() {
                            Ok(megabytes) => {
                                profile.upload_limit_bytes =
                                    megabytes.checked_mul(1024 * 1024).or_else(|| {
                                        profile.diagnostics.push(SimLaunchDiagnostic::invalid(
                                            option,
                                            "upload size is too large",
                                        ));
                                        None
                                    });
                            }
                            Err(_) => profile.diagnostics.push(SimLaunchDiagnostic::invalid(
                                option,
                                "upload size must be an integer number of MiB",
                            )),
                        }
                    }
                }
                "--base-directory" => {
                    set_path(option, &args, &mut index, &mut profile, |p, path| {
                        p.directories.base_directory = Some(path)
                    })
                }
                "--input-directory" => {
                    set_path(option, &args, &mut index, &mut profile, |p, path| {
                        p.directories.input_directory = Some(path)
                    })
                }
                "--output-directory" => {
                    set_path(option, &args, &mut index, &mut profile, |p, path| {
                        p.directories.output_directory = Some(path)
                    })
                }
                "--temp-directory" => {
                    set_path(option, &args, &mut index, &mut profile, |p, path| {
                        p.directories.temp_directory = Some(path)
                    })
                }
                "--user-directory" => {
                    set_path(option, &args, &mut index, &mut profile, |p, path| {
                        p.directories.user_directory = Some(path)
                    })
                }
                "--auto-launch" => profile.auto_launch = true,
                "--verbose" => profile.logging.verbose = true,
                "--log-stdout" => profile.logging.stdout = true,
                "--log-level" => {
                    if let Some(value) = required_value(&args, &mut index, &mut profile) {
                        profile.logging.level = value;
                    }
                }
                "--enable-assets" => profile.assets.enabled = true,
                "--disable-assets" => profile.assets.enabled = false,
                "--database-url" => {
                    if let Some(value) = required_value(&args, &mut index, &mut profile) {
                        profile.database_url = Some(value);
                    }
                }
                "--enable-api-nodes" => profile.api_nodes.enabled = true,
                "--disable-api-nodes" => profile.api_nodes.enabled = false,
                "--disable-custom-nodes" => profile.custom_nodes.enabled = false,
                "--custom-node-directory" => {
                    set_path(option, &args, &mut index, &mut profile, |p, path| {
                        p.custom_nodes.directory = Some(path)
                    })
                }
                "--manager-mode" => {
                    if let Some(value) = required_value(&args, &mut index, &mut profile) {
                        profile.manager.enabled = value != "disabled";
                        profile.manager.mode = value;
                    }
                }
                "--feature" => {
                    if let Some(value) = required_value(&args, &mut index, &mut profile) {
                        match value.split_once('=') {
                            Some((name, flag_value)) if !name.trim().is_empty() => {
                                profile
                                    .feature_flags
                                    .insert(name.trim().to_string(), flag_value.to_string());
                            }
                            _ => profile.diagnostics.push(SimLaunchDiagnostic::invalid(
                                option,
                                "feature flags must use name=value syntax",
                            )),
                        }
                    }
                }
                "--enable-compress-response" => profile.compression.response_compression = true,
                "--cpu" => profile.runtime_policy.device = DeviceBackend::Cpu,
                "--cuda" => profile.runtime_policy.device = DeviceBackend::Cuda,
                "--directml" => profile.runtime_policy.device = DeviceBackend::DirectMl,
                "--fp32" => profile.runtime_policy.precision = PrecisionPolicy::Fp32,
                "--fp16" => profile.runtime_policy.precision = PrecisionPolicy::Fp16,
                "--bf16" => profile.runtime_policy.precision = PrecisionPolicy::Bf16,
                "--fp8" => profile.runtime_policy.precision = PrecisionPolicy::Fp8,
                "--quantized" => profile.runtime_policy.precision = PrecisionPolicy::Quantized,
                "--gpu-only" => profile.runtime_policy.memory = MemoryMode::GpuOnly,
                "--highvram" => profile.runtime_policy.memory = MemoryMode::HighVram,
                "--lowvram" => profile.runtime_policy.memory = MemoryMode::LowVram,
                "--novram" => profile.runtime_policy.memory = MemoryMode::NoVram,
                "--multi-gpu" => profile.runtime_policy.multi_gpu = true,
                "--async-offload" => profile.runtime_policy.async_offload = true,
                "--pin-shared-memory" => profile.runtime_policy.pinned_memory = true,
                "--mmap-weights" => profile.runtime_policy.mmap_weights = true,
                "--dont-mmap-weights" => profile.runtime_policy.mmap_weights = false,
                "--free-memory-before-load" => {
                    profile.runtime_policy.release_cache_before_load = true;
                }
                "--allow-downloads" => profile.runtime_policy.allow_downloads = true,
                "--dependency-reviewed" => profile.runtime_policy.dependency_reviewed = true,
                "--cache-classic" => profile.cache.mode = "classic".to_string(),
                "--cache-lru" => profile.cache.mode = "lru".to_string(),
                "--cache-none" => profile.cache.mode = "none".to_string(),
                "--attention-backend" => {
                    if let Some(value) = required_value(&args, &mut index, &mut profile) {
                        profile.performance.attention_backend = Some(value);
                    }
                }
                "--windows-standalone-build" => {
                    profile.diagnostics.push(SimLaunchDiagnostic::unsupported(
                        option,
                        "standalone packaging is selected through Sim packaging profiles",
                        Some("Sim packaging profile catalog"),
                    ))
                }
                "--preview-method" => {
                    let _ = optional_value(&args, &mut index);
                    profile.diagnostics.push(SimLaunchDiagnostic::unsupported(
                        option,
                        "preview rendering is owned by Sim media preview routing",
                        Some("Sim media preview service"),
                    ));
                }
                "--dont-print-server" => {
                    profile.diagnostics.push(SimLaunchDiagnostic::unsupported(
                        option,
                        "server log visibility is controlled by Sim diagnostics settings",
                        Some("Sim diagnostics logging profile"),
                    ))
                }
                _ if option.starts_with("--") => {
                    profile.diagnostics.push(SimLaunchDiagnostic::unsupported(
                        option,
                        "no native Sim launch behavior is registered for this option",
                        None,
                    ))
                }
                _ => profile.diagnostics.push(SimLaunchDiagnostic::invalid(
                    option,
                    "launch arguments must be named options",
                )),
            }

            index += 1;
        }

        profile
    }
}

fn required_value(
    args: &[String],
    index: &mut usize,
    profile: &mut SimLaunchProfile,
) -> Option<String> {
    let option = args[*index].clone();
    let next_index = *index + 1;
    match args.get(next_index) {
        Some(value) if !value.starts_with("--") => {
            *index = next_index;
            Some(value.clone())
        }
        _ => {
            profile.diagnostics.push(SimLaunchDiagnostic::invalid(
                option,
                "missing required value",
            ));
            None
        }
    }
}

fn optional_value(args: &[String], index: &mut usize) -> Option<String> {
    let next_index = *index + 1;
    let value = args.get(next_index)?;
    if value.starts_with("--") {
        None
    } else {
        *index = next_index;
        Some(value.clone())
    }
}

fn set_path(
    option: &str,
    args: &[String],
    index: &mut usize,
    profile: &mut SimLaunchProfile,
    set: impl FnOnce(&mut SimLaunchProfile, PathBuf),
) {
    if let Some(value) = required_value(args, index, profile) {
        if value.trim().is_empty() {
            profile
                .diagnostics
                .push(SimLaunchDiagnostic::invalid(option, "path cannot be empty"));
        } else {
            set(profile, PathBuf::from(value));
        }
    }
}
