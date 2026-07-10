//! Per-OS sandbox integrations for terminal commands run on behalf of the
//! agent.
//!
//! Each supported operating system has its own module here, gated behind
//! its `target_os` cfg so callers reach for the right one explicitly and
//! non-host targets don't carry dead code.
//!
//! macOS uses [`macos_seatbelt`], while Windows routes commands through WSL
//! and runs them under Bubblewrap there (see [`windows_wsl`]).

#[cfg(target_os = "macos")]
pub mod macos_seatbelt;

#[cfg(target_os = "linux")]
pub mod linux_bubblewrap;

#[cfg(target_os = "windows")]
pub mod windows_wsl;

/// Marker prefix for [`windows_wsl`] errors that mean the sandboxing
/// environment is unavailable rather than a command-specific request failed.
pub const WSL_SANDBOX_UNAVAILABLE_PREFIX: &str = "Windows sandboxing via WSL is unavailable";

/// Per-command relaxations for the Bubblewrap sandbox used inside WSL.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SandboxPermissions {
    pub allow_network: bool,
    pub allow_fs_write: bool,
}
