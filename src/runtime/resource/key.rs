use std::fmt;

/// Stable host-defined key for one loadable resource.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceKey(String);

impl ResourceKey {
    /// Build a resource key from host-owned text.
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    /// Return the key text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&'static str> for ResourceKey {
    fn from(value: &'static str) -> Self {
        Self::new(value)
    }
}

impl From<String> for ResourceKey {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for ResourceKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}
