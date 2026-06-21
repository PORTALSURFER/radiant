//! Backend-neutral native file-open events.

use std::path::PathBuf;

/// Native document/file-open request supplied by the operating system.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeFileOpen {
    /// Files the operating system asked the application to open.
    pub paths: Vec<PathBuf>,
}

impl NativeFileOpen {
    /// Build a native file-open request from one or more paths.
    pub fn new(paths: Vec<PathBuf>) -> Self {
        Self { paths }
    }

    /// Build a native file-open request for one path.
    pub fn single(path: PathBuf) -> Self {
        Self { paths: vec![path] }
    }

    /// Return whether this request has no paths to open.
    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }
}
