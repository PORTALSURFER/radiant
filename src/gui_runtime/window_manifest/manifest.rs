use super::WindowSpec;
use std::collections::HashSet;

/// Host-managed collection of application window descriptors.
///
/// This is a manifest and validation object, not a native event-loop runner.
/// Multi-window hosts can pair each spec with a separate runtime bridge or view
/// and let their platform adapter decide how to open, embed, or route windows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WindowManifest {
    specs: Vec<WindowSpec>,
}

impl WindowManifest {
    /// Build an empty window manifest.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a manifest from explicit specs.
    pub fn from_specs(specs: impl IntoIterator<Item = WindowSpec>) -> Result<Self, String> {
        let mut manifest = Self::new();
        for spec in specs {
            manifest.push(spec)?;
        }
        Ok(manifest)
    }

    /// Add one valid window spec, rejecting duplicate stable keys.
    pub fn push(&mut self, spec: WindowSpec) -> Result<(), String> {
        if self.specs.iter().any(|existing| existing.key == spec.key) {
            return Err(format!("duplicate window key '{}'", spec.key));
        }
        spec.validate()?;
        self.specs.push(spec);
        Ok(())
    }

    /// Return the number of window specs.
    pub fn len(&self) -> usize {
        self.specs.len()
    }

    /// Return whether this manifest has no window specs.
    pub fn is_empty(&self) -> bool {
        self.specs.is_empty()
    }

    /// Return all stable window keys in manifest order.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.specs.iter().map(|spec| spec.key.as_str())
    }

    /// Borrow one spec by stable key.
    pub fn get(&self, key: &str) -> Option<&WindowSpec> {
        self.specs.iter().find(|spec| spec.key == key)
    }

    /// Borrow all specs in manifest order.
    pub fn specs(&self) -> &[WindowSpec] {
        &self.specs
    }

    /// Consume the manifest into its ordered specs.
    pub fn into_specs(self) -> Vec<WindowSpec> {
        self.specs
    }

    /// Verify that all window keys are unique and all specs are valid.
    pub fn validate(&self) -> Result<(), String> {
        let mut seen = HashSet::with_capacity(self.specs.len());
        for spec in &self.specs {
            if !seen.insert(spec.key.as_str()) {
                return Err(format!("duplicate window key '{}'", spec.key));
            }
            spec.validate()?;
        }
        Ok(())
    }
}
