//! Generic focus-surface primitives.

/// Logical focus bucket used for contextual input routing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FocusSurface {
    /// No UI surface currently owns keyboard focus.
    #[default]
    None,
    /// Timeline or signal-editor surface.
    Timeline,
    /// Main content list or table surface.
    ContentList,
    /// Navigation tree surface.
    NavigationTree,
    /// Navigation list surface.
    NavigationList,
}

#[cfg(test)]
mod tests {
    use super::FocusSurface;

    #[test]
    fn focus_surface_defaults_to_none() {
        assert_eq!(FocusSurface::default(), FocusSurface::None);
    }
}
