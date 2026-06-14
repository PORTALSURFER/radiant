use std::fmt;

/// Stable host-defined key for one loadable resource.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceKey(String);

impl ResourceKey {
    /// Build a resource key from host-owned text.
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    /// Build a resource key from a stable scope and host-owned identity.
    ///
    /// Use this when an application has several resource classes, such as
    /// documents, cache entries, folders, devices, or viewports, and wants
    /// resource-task diagnostics to preserve that class boundary.
    pub fn scoped(scope: impl AsRef<str>, identity: impl AsRef<str>) -> Self {
        Self(format!("{}:{}", scope.as_ref(), identity.as_ref()))
    }

    /// Build a resource key for a path-like host resource.
    ///
    /// Radiant treats the path as opaque identity text; the host still owns any
    /// path normalization, privacy policy, and filesystem behavior.
    pub fn path(scope: impl AsRef<str>, path: impl AsRef<std::path::Path>) -> Self {
        Self::scoped(scope, path.as_ref().display().to_string())
    }

    /// Return the key text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::ResourceKey;

    #[test]
    fn scoped_resource_key_preserves_scope_and_identity() {
        let key = ResourceKey::scoped("document", "draft.md");

        assert_eq!(key.as_str(), "document:draft.md");
    }

    #[test]
    fn path_resource_key_preserves_scope_and_display_path() {
        let key = ResourceKey::path("cache", std::path::Path::new("samples/kick.wav"));

        assert_eq!(key.as_str(), "cache:samples/kick.wav");
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
