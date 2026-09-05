//! CLI artifact boundary: create before the event loop, write after it returns.

use std::{fs::File, io::Write};

pub(super) struct ArtifactOutput(File);

impl ArtifactOutput {
    pub(super) fn create(path: &str) -> Result<Self, String> {
        File::create_new(path)
            .map(Self)
            .map_err(|error| error.to_string())
    }

    pub(super) fn finish(mut self, rows: &[serde_json::Value]) -> Result<(), String> {
        for row in rows {
            writeln!(self.0, "{row}").map_err(|error| error.to_string())?;
        }
        self.0.flush().map_err(|error| error.to_string())
    }
}
