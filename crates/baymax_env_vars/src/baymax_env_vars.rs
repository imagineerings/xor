pub use env_var::{EnvVar, bool_env_var, env_var};
use std::sync::LazyLock;

/// Whether Baymax is running in stateless mode.
/// When true, Baymax will use in-memory databases instead of persistent storage.
pub static BAYMAX_STATELESS: LazyLock<bool> = bool_env_var!("BAYMAX_STATELESS");
