pub use env_var::{EnvVar, bool_env_var, env_var};
use std::sync::LazyLock;

/// Whether Sim is running in stateless mode.
/// When true, Sim will use in-memory databases instead of persistent storage.
pub static SIM_STATELESS: LazyLock<bool> = bool_env_var!("SIM_STATELESS");
