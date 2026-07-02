mod loader;

pub use loader::*;

use std::path::PathBuf;

/// A hint loaded from a `.baymaxhints` file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Hint {
    /// The source of the hint (global or project path).
    pub source: HintSource,
    /// The text content of the hint.
    pub content: String,
    /// Priority — higher values take precedence when merging.
    /// Project hints use a higher priority than global hints.
    pub priority: u8,
}

/// Where a hint was loaded from.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HintSource {
    /// A file in the global hints directory
    /// (`~/.config/baymax/hints/`).
    Global { path: PathBuf },
    /// A `.baymaxhints` file inside a project worktree.
    Project {
        worktree_root: String,
        path: PathBuf,
    },
}

/// Errors that can occur when loading hints.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HintLoadError {
    pub path: PathBuf,
    pub message: String,
}

/// Directories (relative to a worktree root) scanned for
/// `.baymaxhints` files.
pub(crate) const BAYMAX_HINTS_FILE_NAMES: &[&str] = &[".baymaxhints"];

/// Name of the global hints directory inside the baymax config dir.
pub(crate) const GLOBAL_HINTS_DIR_NAME: &str = "hints";
