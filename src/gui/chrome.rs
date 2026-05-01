//! Generic chrome and status-surface primitives.

/// Structured status content for left/center/right chrome segments.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct StatusSegments {
    /// Left-aligned status segment.
    pub left: String,
    /// Center-aligned status segment.
    pub center: String,
    /// Right-aligned status segment.
    pub right: String,
}

#[cfg(test)]
mod tests {
    use super::StatusSegments;

    #[test]
    fn status_segments_default_to_empty_text() {
        assert_eq!(StatusSegments::default().left, "");
        assert_eq!(StatusSegments::default().center, "");
        assert_eq!(StatusSegments::default().right, "");
    }
}
